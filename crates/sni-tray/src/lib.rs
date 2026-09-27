//! Linux tray icon over StatusNotifierItem (what Ubuntu's AppIndicator support and Xfce's panel
//! speak): left click → `on_open`, right click → menu. Framework-free; both tray builds (Tauri 2 and
//! Tauri 1 for Ubuntu 20.04) wire their own actions in. Empty on other platforms.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
