//! Headless core of VTracer Studio.
//!
//! Everything in this crate must build and test without a display or any GUI
//! dependency; the Tauri shell in `desktop/src-tauri` is a thin IPC mapping
//! over these types.

mod config;
mod error;
mod image_io;
mod matrix;
mod recipe;

pub use config::*;
pub use error::*;
pub use image_io::*;
pub use matrix::*;
pub use recipe::*;
