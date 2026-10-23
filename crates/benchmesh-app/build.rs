fn main() {
    // The app's own commands (links.rs) get permissions, so a capability can grant them to the UI,
    // which is served by the embedded server (a remote origin to Tauri).
    let manifest = tauri_build::AppManifest::new().commands(&["open_page"]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest)).expect("failed to run tauri-build");
}
