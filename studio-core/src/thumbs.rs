use crate::{StudioError, StudioResult};

pub const THUMB_MAX_EDGE: u32 = 360;

/// Rasterize an SVG into a PNG thumbnail whose long edge is `max_edge`.
/// The grid shows only these PNGs; a live SVG is mounted solely in the
/// inspector, one at a time.
pub fn render_thumbnail(svg_text: &str, max_edge: u32) -> StudioResult<Vec<u8>> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_text, &options)
        .map_err(|e| StudioError::Convert(format!("SVG 解析失败：{e}")))?;

    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    if w <= 0.0 || h <= 0.0 {
        return Err(StudioError::Convert("SVG 尺寸为空".into()));
    }
    let scale = (max_edge as f32 / w.max(h)).min(1.0);
    let pw = ((w * scale).round() as u32).max(1);
    let ph = ((h * scale).round() as u32).max(1);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(pw, ph)
        .ok_or_else(|| StudioError::Convert("无法分配缩略图画布".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| StudioError::Convert(format!("缩略图编码失败：{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoadedImage, Rect, StudioConfig};

    #[test]
    fn thumbnail_of_converted_svg_is_not_blank() {
        let img = LoadedImage::from_rgba(crate::image_io::test_image(96, 64));
        let roi = img.crop_roi(Rect { x: 0, y: 0, w: 96, h: 64 }).unwrap();
        let svg = vtracer::convert(roi, (&StudioConfig::default()).into()).unwrap();
        let png = render_thumbnail(&svg.to_string(), 48).unwrap();

        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.width().max(decoded.height()), 48);
        let distinct: std::collections::HashSet<_> = decoded.pixels().map(|p| p.0).collect();
        assert!(distinct.len() > 1, "缩略图不能是纯色/空白");
    }

    #[test]
    fn no_upscale_beyond_source() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M0 0 H10 V10 H0 Z" fill="#123456"/></svg>"##;
        let png = render_thumbnail(svg, 360).unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.width(), 10, "小图不放大");
    }
}
