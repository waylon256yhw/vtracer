use std::io::Cursor;
use std::path::Path;

use image::{imageops, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use vtracer::ColorImage;

use crate::{StudioError, StudioResult};

pub const PREVIEW_MAX_EDGE: u32 = 1600;

/// A rectangle in original-image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// The decoded source image. Decoded exactly once per load; pixels live in
/// Rust and never cross IPC — the frontend only ever sees PNG files on disk.
pub struct LoadedImage {
    pub rgba: RgbaImage,
    /// blake3 over decoded RGBA bytes + dimensions (cache identity — two
    /// files with identical pixels share cache entries).
    pub hash: String,
}

impl LoadedImage {
    pub fn open(path: &Path) -> StudioResult<Self> {
        let rgba = image::open(path)?.to_rgba8();
        Ok(Self::from_rgba(rgba))
    }

    pub fn from_rgba(rgba: RgbaImage) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(rgba.as_raw());
        hasher.update(&rgba.width().to_le_bytes());
        hasher.update(&rgba.height().to_le_bytes());
        let hash = hasher.finalize().to_hex().to_string();
        Self { rgba, hash }
    }

    pub fn width(&self) -> u32 {
        self.rgba.width()
    }

    pub fn height(&self) -> u32 {
        self.rgba.height()
    }

    pub fn check_roi(&self, roi: Rect) -> StudioResult<()> {
        let (w, h) = (self.width(), self.height());
        let fits = roi.w > 0
            && roi.h > 0
            && roi.x.checked_add(roi.w).is_some_and(|r| r <= w)
            && roi.y.checked_add(roi.h).is_some_and(|b| b <= h);
        if fits {
            Ok(())
        } else {
            Err(StudioError::InvalidRoi {
                width: w,
                height: h,
                x: roi.x,
                y: roi.y,
                w: roi.w,
                h: roi.h,
            })
        }
    }

    /// Crop the ROI into a fresh `vtracer::ColorImage`. Called once per run;
    /// the runner clones the returned buffer per candidate.
    pub fn crop_roi(&self, roi: Rect) -> StudioResult<ColorImage> {
        self.check_roi(roi)?;
        let view = imageops::crop_imm(&self.rgba, roi.x, roi.y, roi.w, roi.h).to_image();
        Ok(ColorImage {
            pixels: view.into_raw(),
            width: roi.w as usize,
            height: roi.h as usize,
        })
    }

    /// The full image as a `ColorImage` (one explicit ~4·w·h byte clone —
    /// used only for the final full-resolution render).
    pub fn to_color_image(&self) -> ColorImage {
        ColorImage {
            pixels: self.rgba.as_raw().clone(),
            width: self.width() as usize,
            height: self.height() as usize,
        }
    }

    /// Whole-image preview PNG, downscaled to `PREVIEW_MAX_EDGE`. Display-only
    /// (ROI picking, layout) — parameter judgments must use full-resolution
    /// assets instead.
    pub fn preview_png(&self) -> StudioResult<Vec<u8>> {
        let (w, h) = (self.width(), self.height());
        let long = w.max(h);
        if long <= PREVIEW_MAX_EDGE {
            return encode_png(&self.rgba);
        }
        let scale = PREVIEW_MAX_EDGE as f64 / long as f64;
        let nw = ((w as f64 * scale).round() as u32).max(1);
        let nh = ((h as f64 * scale).round() as u32).max(1);
        let small = imageops::resize(&self.rgba, nw, nh, imageops::FilterType::Triangle);
        encode_png(&small)
    }

    /// Full-resolution ROI crop PNG — the faithful "original" side of the
    /// candidate inspector.
    pub fn roi_reference_png(&self, roi: Rect) -> StudioResult<Vec<u8>> {
        self.check_roi(roi)?;
        let view = imageops::crop_imm(&self.rgba, roi.x, roi.y, roi.w, roi.h).to_image();
        encode_png(&view)
    }
}

fn encode_png(img: &RgbaImage) -> StudioResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}

#[cfg(test)]
pub(crate) fn test_image(w: u32, h: u32) -> RgbaImage {
    // Deterministic test pattern: red background, blue square, green stripe.
    RgbaImage::from_fn(w, h, |x, y| {
        if y < h / 8 {
            image::Rgba([40, 200, 60, 255])
        } else if x > w / 4 && x < 3 * w / 4 && y > h / 4 && y < 3 * h / 4 {
            image::Rgba([30, 60, 220, 255])
        } else {
            image::Rgba([220, 40, 40, 255])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_dimension_sensitive() {
        let a = LoadedImage::from_rgba(test_image(64, 64));
        let b = LoadedImage::from_rgba(test_image(64, 64));
        assert_eq!(a.hash, b.hash);
        let c = LoadedImage::from_rgba(test_image(64, 32));
        assert_ne!(a.hash, c.hash);
    }

    #[test]
    fn crop_roi_bounds() {
        let img = LoadedImage::from_rgba(test_image(64, 48));
        let ok = img.crop_roi(Rect { x: 8, y: 8, w: 16, h: 16 }).unwrap();
        assert_eq!((ok.width, ok.height), (16, 16));
        assert_eq!(ok.pixels.len(), 16 * 16 * 4);
        assert!(img.crop_roi(Rect { x: 60, y: 0, w: 8, h: 8 }).is_err());
        assert!(img.crop_roi(Rect { x: 0, y: 0, w: 0, h: 8 }).is_err());
        assert!(img
            .crop_roi(Rect { x: u32::MAX, y: 0, w: 2, h: 2 })
            .is_err());
    }

    #[test]
    fn preview_respects_max_edge() {
        let img = LoadedImage::from_rgba(test_image(64, 32));
        let png = img.preview_png().unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 32), "小图不缩放");
    }

    #[test]
    fn golden_roi_converts_to_svg() {
        let img = LoadedImage::from_rgba(test_image(64, 64));
        let roi = img.crop_roi(Rect { x: 0, y: 0, w: 64, h: 64 }).unwrap();
        let cfg: vtracer::Config = (&crate::StudioConfig::default()).into();
        let svg = vtracer::convert(roi, cfg).unwrap();
        assert!(!svg.paths.is_empty());
        let text = svg.to_string();
        assert!(text.contains("<path"));
        assert!(text.contains("<svg"));
    }
}
