//! Giving the panel keyboard focus when it's shown from the tray on X11.
//!
//! GNOME's window manager (mutter) only focuses a newly shown window if it carries a recent user
//! timestamp (`_NET_WM_USER_TIME`). A panel shown in response to a tray click has none — the click
//! happened in GNOME Shell, not in our app — so mutter left it unfocused ("demands attention"), and
//! the panel's "hide when focus is lost" rule never fired: clicking elsewhere didn't close it.
//! Presenting the window with the X server's current time counts as fresh user interaction.
//!
//! Plain C calls into GTK/GDK 3 (which every Tauri app on Linux links), so both tray builds share
//! this whatever their gtk-rs versions.

use std::ffi::c_void;

#[link(name = "gtk-3")]
unsafe extern "C" {
    fn gtk_widget_get_window(widget: *mut c_void) -> *mut c_void;
    fn gtk_window_present_with_time(window: *mut c_void, timestamp: u32);
}

#[link(name = "gdk-3")]
unsafe extern "C" {
    fn gdk_window_get_display(window: *mut c_void) -> *mut c_void;
    fn gdk_x11_display_get_type() -> usize;
    fn gdk_x11_get_server_time(window: *mut c_void) -> u32;
}

#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_type_check_instance_is_a(instance: *mut c_void, iface_type: usize) -> i32;
}

/// Shows, raises and focuses `gtk_window` (a `GtkWindow*`) stamped with the X server's current time.
/// Returns false, doing nothing, when the window isn't on X11 (then the caller's own focus request
/// is all there is).
///
/// # Safety
/// `gtk_window` must be a live `GtkWindow*`, and this must run on the GTK main thread.
pub unsafe fn present_with_server_time(gtk_window: *mut c_void) -> bool {
    unsafe {
        let gdk_window = gtk_widget_get_window(gtk_window);
        if gdk_window.is_null() {
            return false;
        }
        let display = gdk_window_get_display(gdk_window);
        if display.is_null() || g_type_check_instance_is_a(display, gdk_x11_display_get_type()) == 0 {
            return false;
        }
        gtk_window_present_with_time(gtk_window, gdk_x11_get_server_time(gdk_window));
        true
    }
}
