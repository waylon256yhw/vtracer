//! One-shot generator for the app icons in desktop/src-tauri/icons/.
//! Run from the repo root: `cargo run -p studio-core --example gen_icons`

use image::{Rgba, RgbaImage};

fn main() {
    let out = std::path::Path::new("desktop/src-tauri/icons");
    std::fs::create_dir_all(out).unwrap();

    for (name, size) in [("32x32.png", 32u32), ("128x128.png", 128), ("128x128@2x.png", 256)] {
        icon(size).save(out.join(name)).unwrap();
    }
    icon(256)
        .save_with_format(out.join("icon.ico"), image::ImageFormat::Ico)
        .unwrap();
    icon(512).save(out.join("icon.png")).unwrap();
    println!("icons written to {}", out.display());
}

/// Rounded dark tile with a light "V" of two strokes — drawn analytically so
/// no font or external asset is needed.
fn icon(size: u32) -> RgbaImage {
    let s = size as f64;
    let radius = s * 0.22;
    let bg_a = (38u8, 70u8, 83u8); // #264653
    let bg_b = (42u8, 157u8, 143u8); // #2a9d8f
    let fg = (233u8, 196u8, 106u8); // #e9c46a

    // The V: two thick line segments from top corners to bottom center.
    let top_y = s * 0.26;
    let bot_y = s * 0.78;
    let left_x = s * 0.28;
    let right_x = s * 0.72;
    let mid_x = s * 0.50;
    let stroke = s * 0.11;

    RgbaImage::from_fn(size, size, |x, y| {
        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);

        // Rounded-rect mask
        let dx = (px - s / 2.0).abs() - (s / 2.0 - radius);
        let dy = (py - s / 2.0).abs() - (s / 2.0 - radius);
        let outside = if dx > 0.0 && dy > 0.0 {
            (dx * dx + dy * dy).sqrt() > radius
        } else {
            dx > s / 2.0 || dy > s / 2.0
        };
        if outside {
            return Rgba([0, 0, 0, 0]);
        }

        // Diagonal background gradient
        let t = ((px + py) / (2.0 * s)).clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t) as u8;
        let mut pixel = [lerp(bg_a.0, bg_b.0), lerp(bg_a.1, bg_b.1), lerp(bg_a.2, bg_b.2), 255];

        let d1 = dist_to_segment(px, py, left_x, top_y, mid_x, bot_y);
        let d2 = dist_to_segment(px, py, right_x, top_y, mid_x, bot_y);
        if d1.min(d2) < stroke / 2.0 {
            pixel = [fg.0, fg.1, fg.2, 255];
        }
        Rgba(pixel)
    })
}

fn dist_to_segment(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let (vx, vy) = (x2 - x1, y2 - y1);
    let len2 = vx * vx + vy * vy;
    let t = (((px - x1) * vx + (py - y1) * vy) / len2).clamp(0.0, 1.0);
    let (cx, cy) = (x1 + t * vx, y1 + t * vy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}
