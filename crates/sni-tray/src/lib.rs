//! Linux tray icon over StatusNotifierItem (what Ubuntu's AppIndicator support and Xfce's panel
//! speak): left click (or middle click) → `open`, right click → menu. Framework-free; both tray
//! builds (Tauri 2 and Tauri 1 for Ubuntu 20.04) wire their own actions in. Also the X11 focus fix
//! both use when showing the panel (`present_with_server_time`). Empty on other platforms.

#[cfg(target_os = "linux")]
mod focus;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use focus::present_with_server_time;
#[cfg(target_os = "linux")]
pub use linux::*;
