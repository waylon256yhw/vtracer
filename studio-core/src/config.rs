use serde::{Deserialize, Serialize};

use crate::{StudioError, StudioResult};

/// Serde mirror of `vtracer::Config` using the UI-facing parameter names
/// (`gradient_step` instead of `layer_difference`, `segment_length` instead of
/// `length_threshold`). This is the canonical config shape for IPC, recipes
/// and cache keys; it converts into `vtracer::Config` right before calling
/// `vtracer::convert`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StudioConfig {
    pub color_mode: ColorMode,
    pub hierarchical: Hierarchical,
    pub mode: CurveMode,
    /// Linear speckle size in px (squared into an area internally by vtracer).
    /// Valid range [0,128] — the CLI's [0,16] is a legacy limit; the upstream
    /// web UI exposes up to 128.
    pub filter_speckle: u32,
    pub color_precision: i32,
    /// UI name for `vtracer::Config::layer_difference`. 0 switches the
    /// clustering runner into diagonal mode, which is a different behavior,
    /// not just a tiny step — the UI shows a hint at 0.
    pub gradient_step: i32,
    pub corner_threshold: i32,
    /// UI name for `vtracer::Config::length_threshold`.
    pub segment_length: f64,
    pub max_iterations: usize,
    pub splice_threshold: i32,
    pub path_precision: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Color,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hierarchical {
    Stacked,
    Cutout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurveMode {
    Spline,
    Polygon,
    Pixel,
}

pub const FILTER_SPECKLE_MAX: u32 = 128;

impl Default for StudioConfig {
    fn default() -> Self {
        Self {
            color_mode: ColorMode::Color,
            hierarchical: Hierarchical::Stacked,
            mode: CurveMode::Spline,
            filter_speckle: 4,
            color_precision: 6,
            gradient_step: 16,
            corner_threshold: 60,
            segment_length: 4.0,
            max_iterations: 10,
            splice_threshold: 45,
            path_precision: Some(2),
        }
    }
}

impl StudioConfig {
    pub fn validate(&self) -> StudioResult<()> {
        if self.filter_speckle > FILTER_SPECKLE_MAX {
            return Err(StudioError::InvalidParam(format!(
                "斑点过滤 filter_speckle 必须在 [0,{FILTER_SPECKLE_MAX}] 内，当前 {}",
                self.filter_speckle
            )));
        }
        if !(1..=8).contains(&self.color_precision) {
            return Err(StudioError::InvalidParam(format!(
                "颜色精度 color_precision 必须在 [1,8] 内，当前 {}",
                self.color_precision
            )));
        }
        if !(0..=255).contains(&self.gradient_step) {
            return Err(StudioError::InvalidParam(format!(
                "渐变步长 gradient_step 必须在 [0,255] 内，当前 {}",
                self.gradient_step
            )));
        }
        if !(0..=180).contains(&self.corner_threshold) {
            return Err(StudioError::InvalidParam(format!(
                "拐角阈值 corner_threshold 必须在 [0,180] 内，当前 {}",
                self.corner_threshold
            )));
        }
        if !(3.5..=10.0).contains(&self.segment_length) {
            return Err(StudioError::InvalidParam(format!(
                "线段长度 segment_length 必须在 [3.5,10] 内，当前 {}",
                self.segment_length
            )));
        }
        if !(0..=180).contains(&self.splice_threshold) {
            return Err(StudioError::InvalidParam(format!(
                "拼接阈值 splice_threshold 必须在 [0,180] 内，当前 {}",
                self.splice_threshold
            )));
        }
        // visioncortex asserts max_iterations > 0 — a zero from a hand-edited
        // recipe would panic inside the converter.
        if self.max_iterations == 0 {
            return Err(StudioError::InvalidParam(
                "最大迭代次数 max_iterations 至少为 1".into(),
            ));
        }
        if let Some(p) = self.path_precision {
            if p > 8 {
                return Err(StudioError::InvalidParam(format!(
                    "路径小数位 path_precision 必须在 [0,8] 内，当前 {p}"
                )));
            }
        }
        Ok(())
    }

    /// Mirror of the cmdapp presets (cmdapp/src/config.rs `Config::from_preset`).
    pub fn from_preset(preset: &str) -> Option<Self> {
        let base = Self::default();
        match preset {
            "bw" => Some(Self {
                color_mode: ColorMode::Binary,
                ..base
            }),
            "poster" => Some(Self {
                color_precision: 8,
                ..base
            }),
            "photo" => Some(Self {
                filter_speckle: 10,
                color_precision: 8,
                gradient_step: 48,
                corner_threshold: 180,
                ..base
            }),
            _ => None,
        }
    }

    /// Canonical JSON used inside cache keys. Field order is struct order,
    /// which serde_json preserves, so equal configs always produce equal keys.
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("StudioConfig serialization cannot fail")
    }
}

impl From<&StudioConfig> for vtracer::Config {
    fn from(c: &StudioConfig) -> Self {
        vtracer::Config {
            color_mode: match c.color_mode {
                ColorMode::Color => vtracer::ColorMode::Color,
                ColorMode::Binary => vtracer::ColorMode::Binary,
            },
            hierarchical: match c.hierarchical {
                Hierarchical::Stacked => vtracer::Hierarchical::Stacked,
                Hierarchical::Cutout => vtracer::Hierarchical::Cutout,
            },
            mode: match c.mode {
                CurveMode::Spline => visioncortex::PathSimplifyMode::Spline,
                CurveMode::Polygon => visioncortex::PathSimplifyMode::Polygon,
                CurveMode::Pixel => visioncortex::PathSimplifyMode::None,
            },
            filter_speckle: c.filter_speckle as usize,
            color_precision: c.color_precision,
            layer_difference: c.gradient_step,
            corner_threshold: c.corner_threshold,
            length_threshold: c.segment_length,
            max_iterations: c.max_iterations,
            splice_threshold: c.splice_threshold,
            path_precision: c.path_precision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_uses_ui_names() {
        let json = StudioConfig::default().canonical_json();
        assert!(json.contains("\"gradient_step\":16"), "{json}");
        assert!(json.contains("\"segment_length\":4.0"), "{json}");
        assert!(json.contains("\"color_mode\":\"color\""), "{json}");
        assert!(!json.contains("layer_difference"));
        assert!(!json.contains("length_threshold"));
    }

    #[test]
    fn roundtrip() {
        let c = StudioConfig::from_preset("photo").unwrap();
        let back: StudioConfig = serde_json::from_str(&c.canonical_json()).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn maps_to_vtracer_names() {
        let mut c = StudioConfig::default();
        c.gradient_step = 42;
        c.segment_length = 5.5;
        let v: vtracer::Config = (&c).into();
        assert_eq!(v.layer_difference, 42);
        assert_eq!(v.length_threshold, 5.5);
        assert_eq!(v.filter_speckle, 4);
    }

    #[test]
    fn validation_ranges() {
        let mut c = StudioConfig::default();
        c.filter_speckle = 128;
        assert!(c.validate().is_ok(), "128 是合法上限（web UI 范围）");
        c.filter_speckle = 129;
        assert!(c.validate().is_err());
        c = StudioConfig::default();
        c.color_precision = 9;
        assert!(c.validate().is_err());
        c = StudioConfig::default();
        c.gradient_step = 0;
        assert!(c.validate().is_ok(), "0 合法（diagonal 模式）");
        c = StudioConfig::default();
        c.max_iterations = 0;
        assert!(c.validate().is_err(), "0 迭代会让 visioncortex panic");
        c = StudioConfig::default();
        c.path_precision = Some(9);
        assert!(c.validate().is_err());
    }
}
