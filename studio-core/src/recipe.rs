use serde::{Deserialize, Serialize};

use crate::{MatrixAxes, Rect, StudioConfig, StudioError, StudioResult, FILTER_SPECKLE_MAX};

pub const RECIPE_VERSION: u32 = 1;
pub const SESSION_VERSION: u32 = 1;

/// A reusable conversion recipe. Deliberately NOT tied to any particular
/// image — source/ROI/axes/selection are experiment provenance and live in
/// [`ExperimentSession`] instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub version: u32,
    pub app: String,
    pub created_at: String,
    pub config: StudioConfig,
    pub scaling: Scaling,
}

/// How `filter_speckle` adapts when the recipe is applied to images of a
/// different size than it was tuned on (Phase 2 batch mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scaling {
    pub filter_speckle_mode: SpeckleScaling,
    /// Long edge (px) of the image the recipe was tuned on.
    pub reference_long_edge: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeckleScaling {
    /// Use `config.filter_speckle` as-is.
    Fixed,
    /// Scale linearly with the long-edge ratio. `filter_speckle` is a linear
    /// px quantity (vtracer squares it into an area internally), so the
    /// correct factor is `r`, NOT `r²`.
    Scaled,
}

impl Recipe {
    pub fn new(config: StudioConfig, created_at: String, reference_long_edge: u32) -> Self {
        Self {
            version: RECIPE_VERSION,
            app: "vtracer-studio".into(),
            created_at,
            config,
            scaling: Scaling {
                filter_speckle_mode: SpeckleScaling::Fixed,
                reference_long_edge,
            },
        }
    }

    pub fn from_json(json: &str) -> StudioResult<Self> {
        let r: Recipe = serde_json::from_str(json)?;
        if r.version != RECIPE_VERSION {
            return Err(StudioError::UnsupportedVersion {
                found: r.version,
                supported: RECIPE_VERSION,
            });
        }
        r.config.validate()?;
        Ok(r)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Recipe serialization cannot fail")
    }

    /// The effective config for an image with the given long edge, applying
    /// the speckle scaling policy.
    pub fn effective_config(&self, target_long_edge: u32) -> StudioConfig {
        let mut config = self.config.clone();
        if let SpeckleScaling::Scaled = self.scaling.filter_speckle_mode {
            let r = target_long_edge as f64 / self.scaling.reference_long_edge.max(1) as f64;
            let scaled = (self.config.filter_speckle as f64 * r).round() as u32;
            config.filter_speckle = scaled.min(FILTER_SPECKLE_MAX);
        }
        config
    }
}

/// Everything needed to restore an experiment: which image, which ROI, which
/// axis values, what was chosen, and where the user was looking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentSession {
    pub version: u32,
    pub source: SourceInfo,
    pub base_config: StudioConfig,
    pub roi: Rect,
    pub axes: MatrixAxes,
    pub selected_candidate: Option<String>,
    pub viewport: Option<Viewport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub file: String,
    pub width: u32,
    pub height: u32,
    /// blake3 of decoded RGBA + dims (matches `LoadedImage::hash`).
    pub blake3: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

impl ExperimentSession {
    pub fn from_json(json: &str) -> StudioResult<Self> {
        let s: ExperimentSession = serde_json::from_str(json)?;
        if s.version != SESSION_VERSION {
            return Err(StudioError::UnsupportedVersion {
                found: s.version,
                supported: SESSION_VERSION,
            });
        }
        // A hand-edited session flows straight into runs — validate everything.
        s.base_config.validate()?;
        s.axes.normalized()?;
        let roi_ok = s.roi.w > 0
            && s.roi.h > 0
            && s.roi.x.checked_add(s.roi.w).is_some_and(|r| r <= s.source.width)
            && s.roi.y.checked_add(s.roi.h).is_some_and(|b| b <= s.source.height);
        if !roi_ok {
            return Err(StudioError::InvalidParam(
                "会话中的 ROI 超出图片范围或为空".into(),
            ));
        }
        if let Some(v) = &s.viewport {
            if !v.scale.is_finite() || v.scale <= 0.0 || !v.x.is_finite() || !v.y.is_finite() {
                return Err(StudioError::InvalidParam("会话中的视口数值无效".into()));
            }
        }
        Ok(s)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Session serialization cannot fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_roundtrip_and_version_gate() {
        let r = Recipe::new(StudioConfig::default(), "2026-07-11T00:00:00Z".into(), 4000);
        let back = Recipe::from_json(&r.to_json()).unwrap();
        assert_eq!(r, back);

        let mut bad = r.clone();
        bad.version = 2;
        let err = Recipe::from_json(&bad.to_json()).unwrap_err();
        assert!(err.to_string().contains("不支持的文件版本"));
    }

    #[test]
    fn speckle_scaling_is_linear_not_squared() {
        let mut r = Recipe::new(StudioConfig::default(), "t".into(), 1000);
        r.config.filter_speckle = 4;
        r.scaling.filter_speckle_mode = SpeckleScaling::Scaled;

        // 2x long edge → 2x speckle (linear), NOT 4x (squared).
        assert_eq!(r.effective_config(2000).filter_speckle, 8);
        assert_eq!(r.effective_config(500).filter_speckle, 2);
        assert_eq!(r.effective_config(1000).filter_speckle, 4);

        // Clamped to the max.
        r.config.filter_speckle = 100;
        assert_eq!(r.effective_config(4000).filter_speckle, FILTER_SPECKLE_MAX);

        // Fixed mode ignores size.
        r.scaling.filter_speckle_mode = SpeckleScaling::Fixed;
        assert_eq!(r.effective_config(4000).filter_speckle, 100);
    }

    #[test]
    fn session_roundtrip() {
        let s = ExperimentSession {
            version: SESSION_VERSION,
            source: SourceInfo {
                file: "C:\\pics\\a.png".into(),
                width: 4000,
                height: 3000,
                blake3: "abc".into(),
            },
            base_config: StudioConfig::default(),
            roi: Rect { x: 10, y: 20, w: 300, h: 400 },
            axes: MatrixAxes {
                gradient_step: vec![8, 16, 32],
                filter_speckle: vec![2, 4, 8],
                color_precision: vec![4, 6, 8],
            },
            selected_candidate: Some("g16_f4_p6".into()),
            viewport: Some(Viewport { scale: 2.0, x: -120.0, y: -80.0 }),
        };
        let back = ExperimentSession::from_json(&s.to_json()).unwrap();
        assert_eq!(s, back);

        // Hand-edited sessions with unusable content are rejected up front.
        let mut bad = s.clone();
        bad.roi = Rect { x: 3900, y: 0, w: 300, h: 300 };
        assert!(ExperimentSession::from_json(&bad.to_json()).is_err(), "ROI 越界");
        let mut bad = s.clone();
        bad.base_config.max_iterations = 0;
        assert!(ExperimentSession::from_json(&bad.to_json()).is_err(), "非法 config");
        let mut bad = s.clone();
        bad.axes.gradient_step = vec![999];
        assert!(ExperimentSession::from_json(&bad.to_json()).is_err(), "非法轴取值");
        let mut bad = s;
        bad.viewport = Some(Viewport { scale: f64::NAN, x: 0.0, y: 0.0 });
        assert!(ExperimentSession::from_json(&bad.to_json()).is_err(), "非法视口");
    }
}
