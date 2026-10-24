//! Files the UI saves: a recording's CSV, a flow's JSON, the instrument console's log. The webview
//! would put a download into the Downloads folder without a word, so the app asks where with the
//! system's Save dialog and writes the file itself. The UI hands over the text; the user picks
//! the place, so a page cannot write anywhere on its own.

use tauri::{Manager, Runtime, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// Larger than any CSV the UI exports in one piece (a day of 8 series at 1 s is about 50 MiB).
const MAX_BYTES: usize = 256 << 20;

/// The name the dialog suggests: the last path part only, without characters file systems refuse.
pub fn suggested_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = base
        .chars()
        .map(|c| if c.is_control() || matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    let clean = clean.trim().trim_start_matches('.').to_string();
    if clean.is_empty() { "benchmesh.txt".into() } else { clean }
}

/// The extension the dialog filters on (`csv`, `json`, `txt`), if the name has a short one.
pub fn extension(name: &str) -> Option<&str> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && (1..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric())).then_some(ext)
}

/// Ask where to save `contents` as `name` and write it there. `None` when the user cancels.
#[tauri::command]
pub async fn save_file<R: Runtime>(window: WebviewWindow<R>, name: String, contents: String) -> Result<Option<String>, String> {
    if contents.len() > MAX_BYTES {
        return Err(format!("the file is too large to save ({} MiB)", contents.len() >> 20));
    }
    let name = suggested_name(&name);
    let mut dialog = window.dialog().file().set_parent(&window).set_file_name(&name);
    if let Some(ext) = extension(&name) {
        dialog = dialog.add_filter(ext.to_uppercase(), &[ext]);
    }
    if let Ok(dir) = window.path().download_dir() {
        dialog = dialog.set_directory(dir);
    }
    let Some(chosen) = dialog.blocking_save_file() else { return Ok(None) };
    let path = chosen.into_path().map_err(|e| format!("not a file path: {e}"))?;
    std::fs::write(&path, contents).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::{extension, suggested_name};

    #[test]
    fn suggests_a_plain_file_name() {
        assert_eq!(suggested_name("Load test.csv"), "Load test.csv");
        assert_eq!(suggested_name("../../etc/passwd"), "passwd");
        assert_eq!(suggested_name("C:\\Users\\x\\run.json"), "run.json");
        assert_eq!(suggested_name("a:b*c?.csv"), "a_b_c_.csv");
        assert_eq!(suggested_name(".hidden"), "hidden");
        assert_eq!(suggested_name(""), "benchmesh.txt");
        assert_eq!(suggested_name("dir/"), "benchmesh.txt");
    }

    #[test]
    fn filters_on_a_short_extension() {
        assert_eq!(extension("Load test.csv"), Some("csv"));
        assert_eq!(extension("flow.json"), Some("json"));
        assert_eq!(extension("console.2026-10-24T15-00-00.txt"), Some("txt"));
        assert_eq!(extension("README"), None);
        assert_eq!(extension(".csv"), None);
        assert_eq!(extension("v0.1 test"), None);
    }
}
