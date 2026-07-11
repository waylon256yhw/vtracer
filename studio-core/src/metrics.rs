use serde::Serialize;
use vtracer::SvgFile;

/// Per-candidate objective numbers shown in the grid and the metrics table.
#[derive(Debug, Clone, Serialize)]
pub struct Metrics {
    /// Number of `<path>` elements (stacked layers), NOT Bézier segment
    /// count — the UI labels this 「路径元素数」.
    pub paths: usize,
    /// Distinct fill colors.
    pub colors: usize,
    /// Byte length of the serialized SVG.
    pub svg_bytes: usize,
    pub elapsed_ms: u64,
}

impl Metrics {
    pub fn from_svg(svg: &SvgFile, svg_text: &str, elapsed_ms: u64) -> Self {
        let mut colors: Vec<[u8; 3]> = svg
            .paths
            .iter()
            .map(|p| [p.color.r, p.color.g, p.color.b])
            .collect();
        colors.sort_unstable();
        colors.dedup();
        Self {
            paths: svg.paths.len(),
            colors: colors.len(),
            svg_bytes: svg_text.len(),
            elapsed_ms,
        }
    }
}
