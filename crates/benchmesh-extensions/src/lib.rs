//! What a BenchMesh distribution adds to the open-source app (contract `ai_context/OPEN_CORE.md`).
//!
//! This is the open-source build: it adds nothing. The official distribution replaces this crate,
//! at build time, with its own crate of the same name and the same three items below (app updates,
//! online services); its code is not part of this repository. Everything a build of this
//! repository does works without it.

use std::sync::Arc;

use bm_server::extension::{Extension, Host, COMMUNITY};

/// The distribution name (`GET /version` `distribution`).
pub const DISTRIBUTION: &str = COMMUNITY;

/// Stops the embedded server and its devices (their on_disconnect steps run); the app gives it.
pub type StopServer = Arc<dyn Fn(&tauri::AppHandle) + Send + Sync>;

/// The server's extensions (routes in the API).
pub fn server(_host: &Host) -> Vec<Arc<dyn Extension>> {
    Vec::new()
}

/// Adds to the desktop app's window runtime (plugins, their commands).
pub fn desktop(builder: tauri::Builder<tauri::Wry>, _stop_server: StopServer) -> tauri::Builder<tauri::Wry> {
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_open_source_build_adds_nothing() {
        assert_eq!(DISTRIBUTION, "community");
        assert!(server(&Host { data_dir: None, headless: true }).is_empty());
    }
}
