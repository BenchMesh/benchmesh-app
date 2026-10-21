//! Web pages the UI opens in the system browser (the webview does not follow external links).
//! A fixed list: the UI is served over HTTP, so it names a page and never passes a URL.

pub const RELEASES: &str = "https://github.com/MarkoVcode/BenchMesh/releases/latest";
const PAGES: [(&str, &str); 5] = [
    ("issues", "https://github.com/MarkoVcode/BenchMesh/issues"),
    ("new-issue", "https://github.com/MarkoVcode/BenchMesh/issues/new"),
    ("releases", RELEASES),
    ("docs", "https://benchmesh.online/docs"),
    ("email", "mailto:support@benchmesh.online"),
];

pub fn url_of(page: &str) -> Option<&'static str> {
    PAGES.iter().find(|(name, _)| *name == page).map(|(_, url)| *url)
}

/// Open `url` with the system's default browser.
pub fn open(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    let child = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let child = std::process::Command::new("open").arg(url).spawn();
    #[cfg(windows)]
    let child = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    child.map(|_| ())
}

/// Open one of the known pages (`issues`, `new-issue`, `releases`, `docs`, `email`: the mail app).
#[tauri::command]
pub fn open_page(page: String) -> Result<(), String> {
    let url = url_of(&page).ok_or_else(|| format!("unknown page {page:?}"))?;
    open(url).map_err(|e| format!("could not start the browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::url_of;

    #[test]
    fn only_known_pages_open() {
        assert_eq!(url_of("new-issue"), Some("https://github.com/MarkoVcode/BenchMesh/issues/new"));
        assert!(url_of("issues").unwrap().ends_with("/issues"));
        assert_eq!(url_of("docs"), Some("https://benchmesh.online/docs"));
        assert_eq!(url_of("email"), Some("mailto:support@benchmesh.online"));
        assert_eq!(url_of("https://example.com"), None);
        assert_eq!(url_of(""), None);
    }
}
