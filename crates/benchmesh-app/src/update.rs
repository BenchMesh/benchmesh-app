//! Update check at start (pattern of the LTBR FM receiver): the UI asks once per app start
//! (`update_check`); the updater fetches the signed `latest.json` of the latest published GitHub
//! release (endpoint in `tauri.conf.json`; later proxied through the website). When the user
//! accepts (`update_install`) the new version is downloaded, verified against the public key in
//! `tauri.conf.json`, the devices are stopped (their on_disconnect steps run), then it is
//! installed and the app restarts. Failures to check are silent: offline means no prompt.
//!
//! Where the package manager owns the files (Linux `.deb`) the app cannot replace itself; the
//! prompt then offers the download page (`update_open_page`).

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Download progress for the UI (`downloaded` bytes of `total`, absent when unknown).
const PROGRESS_EVENT: &str = "update://progress";

/// The update found by the check of this app start (`checked` once it ran).
#[derive(Default)]
pub struct Pending {
    checked: bool,
    update: Option<Update>,
}

pub type PendingState = Mutex<Pending>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    version: String,
    current: String,
    notes: Option<String>,
    /// The installation can replace itself (otherwise: download page).
    can_install: bool,
}

#[derive(Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// True where the installation can replace itself: Windows, macOS, a Linux AppImage (its
/// runtime sets `APPIMAGE`); not a `.deb` install.
fn can_install() -> bool {
    !cfg!(target_os = "linux") || std::env::var_os("APPIMAGE").is_some()
}

fn available(u: &Update) -> Available {
    Available { version: u.version.clone(), current: u.current_version.clone(), notes: u.body.clone(), can_install: can_install() }
}

/// The newer version, if any. Checks the network once per app start (a reloaded UI gets the
/// same answer); any failure counts as "no update".
#[tauri::command]
pub async fn update_check(app: AppHandle, pending: State<'_, PendingState>) -> Result<Option<Available>, ()> {
    {
        let p = pending.lock().unwrap();
        if p.checked {
            return Ok(p.update.as_ref().map(available));
        }
    }
    let update = match app.updater() {
        Ok(u) => u.check().await.ok().flatten(),
        Err(_) => None,
    };
    let mut p = pending.lock().unwrap();
    p.checked = true;
    p.update = update;
    Ok(p.update.as_ref().map(available))
}

/// Download, verify and install the update found at start, then restart. Returns only on failure
/// (before installing nothing has changed; the app keeps running).
#[tauri::command]
pub async fn update_install(app: AppHandle, pending: State<'_, PendingState>) -> Result<(), String> {
    if !can_install() {
        return Err("this installation cannot update itself; download the new version".into());
    }
    let update = pending.lock().unwrap().update.take().ok_or("no update available")?;
    let mut downloaded = 0u64;
    let bytes = update
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| {
            pending.lock().unwrap().update = Some(update.clone());
            format!("download failed: {e}")
        })?;
    // the installer replaces the running files (Windows: and exits this process), so the devices
    // are stopped first
    let server = app.state::<crate::Server>().0.lock().unwrap().take();
    if let Some(server) = server {
        let _ = tauri::async_runtime::spawn_blocking(move || server.stop()).await;
    }
    if let Err(e) = update.install(&bytes) {
        // the devices are stopped: start over with the current version rather than run without them
        eprintln!("update install failed: {e}");
    }
    app.restart();
}

/// Open the releases page in the system browser.
#[tauri::command]
pub fn update_open_page() {
    let _ = crate::links::open(crate::links::RELEASES);
}

#[cfg(test)]
mod tests {
    use super::can_install;

    #[test]
    fn deb_installs_cannot_replace_themselves() {
        // the test environment is never an AppImage
        assert_eq!(can_install(), !cfg!(target_os = "linux"));
    }
}
