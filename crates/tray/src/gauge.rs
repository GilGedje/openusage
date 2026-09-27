//! The tray icon image (pixels come from `usage_core::gauge`). On macOS it's a template image the
//! system tints for light/dark menu bars.

use tauri::image::Image;
use usage_core::gauge::{SIZE, ring_rgba};

pub fn icon(fraction: Option<f64>) -> Image<'static> {
    Image::new_owned(ring_rgba(fraction, cfg!(target_os = "macos")), SIZE, SIZE)
}
