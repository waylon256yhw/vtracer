use thiserror::Error;

/// Errors surfaced to the UI. Messages are zh-CN because they are shown to the
/// user verbatim.
#[derive(Debug, Error)]
pub enum StudioError {
    #[error("无法读取文件：{0}")]
    Io(#[from] std::io::Error),

    #[error("无法解码图片：{0}")]
    Image(#[from] image::ImageError),

    #[error("参数无效：{0}")]
    InvalidParam(String),

    #[error("ROI 超出图片范围（图片 {width}×{height}，ROI x={x} y={y} w={w} h={h}）")]
    InvalidRoi {
        width: u32,
        height: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    },

    #[error("不支持的文件版本 {found}（本版本只支持 {supported}），请用新版 VTracer Studio 打开")]
    UnsupportedVersion { found: u32, supported: u32 },

    #[error("转换失败：{0}")]
    Convert(String),

    #[error("JSON 解析失败：{0}")]
    Json(#[from] serde_json::Error),
}

pub type StudioResult<T> = Result<T, StudioError>;
