//! Headless core of VTracer Studio.
//!
//! Everything in this crate must build and test without a display or any GUI
//! dependency; the Tauri shell in `desktop/src-tauri` is a thin IPC mapping
//! over these types.

mod cache;
mod config;
mod error;
mod image_io;
mod matrix;
mod metrics;
mod recipe;
mod runner;
mod thumbs;

pub use cache::*;
pub use config::*;
pub use error::*;
pub use image_io::*;
pub use matrix::*;
pub use metrics::*;
pub use recipe::*;
pub use runner::*;
pub use thumbs::*;
