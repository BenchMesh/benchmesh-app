//! Linux AppImage: the desktop entry and icon a .deb would install, written for the user at start.
//!
//! A dock or taskbar shows an application's icon by matching its window to a desktop entry: the
//! X11 window class or the Wayland app id (`benchmesh`, the program name) against the entry's
//! name or `StartupWMClass`. The .deb installs one; an AppImage installs nothing, so newer
//! desktops (GNOME on Wayland has no window icons) show a generic icon. Writing
//! `~/.local/share/applications/benchmesh.desktop` with our icon fixes the dock and puts
//! BenchMesh in the applications menu; it is rewritten only when it changes (e.g. the AppImage
//! was moved). Removing both files undoes it.

use std::io;
use std::path::{Path, PathBuf};

/// The window's class / app id (GTK's program name: the binary name).
pub const APP_ID: &str = "benchmesh";
/// The app icon (256 px) the entry points at.
pub const ICON_PNG: &[u8] = include_bytes!("../icons/128x128@2x.png");

/// The desktop entry for the AppImage at `appimage` with the icon file at `icon`.
pub fn entry(appimage: &Path, icon: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=BenchMesh\nComment=Lab instruments in one window\n\
         Exec={} %U\nIcon={}\nStartupWMClass={APP_ID}\nCategories=Science;Electronics;\nTerminal=false\n\
         X-BenchMesh-AppImage=true\n",
        exec_arg(&appimage.to_string_lossy()),
        icon.to_string_lossy()
    )
}

/// A path as one `Exec` argument (desktop entry spec: quoted, `"` `` ` `` `$` `\` escaped, `%` doubled).
fn exec_arg(path: &str) -> String {
    let mut out = String::from("\"");
    for ch in path.chars() {
        match ch {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '%' => out.push_str("%%"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Write the icon and the entry under `data_home` (`~/.local/share`) unless they are already
/// there as they would be written. Returns whether anything was written.
pub fn install(data_home: &Path, appimage: &Path) -> io::Result<bool> {
    let icon = data_home.join("icons/hicolor/256x256/apps").join(format!("{APP_ID}.png"));
    let desktop = data_home.join("applications").join(format!("{APP_ID}.desktop"));
    let mut wrote = write_if_changed(&icon, ICON_PNG)?;
    wrote |= write_if_changed(&desktop, entry(appimage, &icon).as_bytes())?;
    Ok(wrote)
}

fn write_if_changed(path: &Path, content: &[u8]) -> io::Result<bool> {
    if std::fs::read(path).is_ok_and(|c| c == content) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)?;
    Ok(true)
}

/// `$XDG_DATA_HOME`, else `~/.local/share`.
pub fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

/// When running as an AppImage (`$APPIMAGE`), make sure its desktop entry is installed.
pub fn install_for_appimage() {
    let (Some(appimage), Some(home)) = (std::env::var_os("APPIMAGE").map(PathBuf::from), data_home()) else {
        return;
    };
    if let Err(e) = install(&home, &appimage) {
        eprintln!("warning: could not write the desktop entry: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bm-desktop-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn the_entry_matches_the_window_and_starts_the_appimage() {
        let e = entry(Path::new("/home/u/Apps/BenchMesh_0.1.13_amd64.AppImage"), Path::new("/home/u/.local/share/icons/hicolor/256x256/apps/benchmesh.png"));
        assert!(e.starts_with("[Desktop Entry]\n"));
        assert!(e.contains("\nExec=\"/home/u/Apps/BenchMesh_0.1.13_amd64.AppImage\" %U\n"), "{e}");
        assert!(e.contains("\nStartupWMClass=benchmesh\n"));
        assert!(e.contains("\nIcon=/home/u/.local/share/icons/hicolor/256x256/apps/benchmesh.png\n"));
    }

    #[test]
    fn odd_paths_stay_one_argument() {
        assert_eq!(exec_arg("/a b/100% \"x\" $y\\z`"), "\"/a b/100%% \\\"x\\\" \\$y\\\\z\\`\"");
    }

    #[test]
    fn installs_once_and_follows_a_moved_appimage() {
        let home = temp_home("install");
        assert!(install(&home, Path::new("/opt/BenchMesh.AppImage")).unwrap());
        let desktop = home.join("applications/benchmesh.desktop");
        let icon = home.join("icons/hicolor/256x256/apps/benchmesh.png");
        assert_eq!(std::fs::read(&icon).unwrap(), ICON_PNG);
        assert!(std::fs::read_to_string(&desktop).unwrap().contains(&format!("Icon={}\n", icon.display())));
        assert!(!install(&home, Path::new("/opt/BenchMesh.AppImage")).unwrap(), "nothing changed: nothing written");
        assert!(install(&home, Path::new("/home/u/BenchMesh.AppImage")).unwrap());
        assert!(std::fs::read_to_string(&desktop).unwrap().contains("Exec=\"/home/u/BenchMesh.AppImage\""));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_icon_is_the_large_one() {
        // PNG IHDR: width and height at bytes 16..24
        assert_eq!(&ICON_PNG[16..24], &[0, 0, 1, 0, 0, 0, 1, 0], "256 x 256");
    }
}
