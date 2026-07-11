use crate::{Rect, StudioConfig};

/// Cache identity for one candidate result: decoded-image hash + ROI + every
/// effective config field (canonical JSON). Phase 1 keeps results in memory
/// per session; Phase 2 will persist to disk under this same key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    pub fn new(image_hash: &str, roi: Rect, config: &StudioConfig) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(image_hash.as_bytes());
        hasher.update(&roi.x.to_le_bytes());
        hasher.update(&roi.y.to_le_bytes());
        hasher.update(&roi.w.to_le_bytes());
        hasher.update(&roi.h.to_le_bytes());
        hasher.update(config.canonical_json().as_bytes());
        Self(hasher.finalize().to_hex().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_changes_with_any_input() {
        let roi = Rect { x: 0, y: 0, w: 10, h: 10 };
        let cfg = StudioConfig::default();
        let base = CacheKey::new("img1", roi, &cfg);

        assert_eq!(base, CacheKey::new("img1", roi, &cfg), "同输入同键");
        assert_ne!(base, CacheKey::new("img2", roi, &cfg));
        assert_ne!(base, CacheKey::new("img1", Rect { x: 1, ..roi }, &cfg));

        let mut cfg2 = cfg.clone();
        cfg2.corner_threshold = 61; // 非轴参数也必须参与键
        assert_ne!(base, CacheKey::new("img1", roi, &cfg2));
    }
}
