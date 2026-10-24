//! Linux/X11: BenchMesh windows do not wait for the compositor between frames.
//!
//! GTK 3 on X11 draws a frame, then waits for the window manager's `_NET_WM_FRAME_DRAWN`
//! before it draws the next one. GNOME 42 (mutter) under XWayland sometimes never sends it
//! (journal: "Frame has assigned frame counter but no frame drawn time"): the window's
//! picture then stays as it is for good while the page keeps running and takes clicks - the
//! frozen window. Without frame synchronisation GTK draws whenever it has a frame, so a lost
//! message cannot stop it. Applied to every window (main and detached panels) as it is
//! created; nothing changes under native Wayland.

use gtk::prelude::*;
use tauri::{plugin::TauriPlugin, Runtime};

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("frame-sync")
        .on_window_ready(|window| {
            let target = window.clone();
            let _ = window.run_on_main_thread(move || {
                if let Ok(gtk_window) = target.gtk_window() {
                    without_frame_sync(&gtk_window);
                }
            });
        })
        .build()
}

/// Switch frame synchronisation off now, or when the window gets its X11 window.
pub fn without_frame_sync<W: IsA<gtk::Widget>>(window: &W) {
    if window.is_realized() {
        disable(window.upcast_ref());
    } else {
        window.connect_realize(|w| disable(w.upcast_ref()));
    }
}

fn disable(widget: &gtk::Widget) {
    // only an X11 window has frame synchronisation (not a native Wayland one)
    if let Some(x11) = widget.window().and_then(|w| w.downcast::<gdkx11::X11Window>().ok()) {
        x11.set_frame_sync_enabled(false);
    }
}
