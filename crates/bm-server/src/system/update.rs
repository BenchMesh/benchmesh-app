//! Updates of a headless BenchMesh (contract `ai_context/HEADLESS.md`): the desktop app's updater
//! needs its window runtime, so the server reads the same signed `latest.json` of the latest
//! published GitHub release, downloads this installation's file, verifies its minisign signature
//! against the same public key (`tauri.conf.json`), and installs it when the process has stopped:
//!
//! - Linux AppImage: the AppImage file is swapped (the old one kept as `<file>.previous`) and started.
//! - Windows: the signed installer runs silently after BenchMesh exited, then starts it again.
//! - macOS: the `.app` bundle is swapped (old one kept as `<bundle>.previous`) and started.
//! - Linux `.deb`: the package manager owns the files - the verified package is downloaded and the
//!   UI shows the `sudo apt install` command.
//! - Built from source (`benchmesh-server`, `cargo run`): check only.
//!
//! Nothing from a request chooses what is downloaded: the key is fixed here, the manifest's address
//! too (`BENCHMESH_UPDATE_URL` replaces it for staging; downloads must still carry the key's signature).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use bm_store::now_ms;
use serde::Deserialize;
use serde_json::{json, Value};

/// The same manifest the desktop app's updater reads (`tauri.conf.json` `plugins.updater.endpoints`).
pub const LATEST_JSON: &str = "https://github.com/MarkoVcode/BenchMesh/releases/latest/download/latest.json";
/// The updater's public key (`tauri.conf.json` `plugins.updater.pubkey`; a test keeps them equal).
pub const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEIwNUM5RjY1NjRDMDIwQTUKUldTbElNQmtaWjljc0dZY2diRXNlZkVXYmJ4R1dSRCtPK2hTZ2hOQjJsZ2dXTlR0cTFFT1o2aEUK";

/// How this BenchMesh was installed, which decides how it is updated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Install {
    /// Linux AppImage: this file is replaced.
    AppImage(PathBuf),
    /// Windows, installed by the NSIS installer: the installed executable.
    Windows(PathBuf),
    /// macOS: the `.app` bundle.
    MacApp(PathBuf),
    /// Linux `.deb`: the package manager owns the files.
    Deb,
    /// Built from source: nothing to install.
    Source,
}

impl Install {
    /// The installed desktop app's executable, run headless.
    pub fn of_app() -> Install {
        let exe = std::env::current_exe().unwrap_or_default();
        if cfg!(target_os = "linux") {
            if let Some(a) = std::env::var_os("APPIMAGE") {
                return Install::AppImage(PathBuf::from(a));
            }
            return if exe.starts_with("/usr") { Install::Deb } else { Install::Source };
        }
        if cfg!(windows) {
            return if exe.components().any(|c| c.as_os_str() == "target") { Install::Source } else { Install::Windows(exe) };
        }
        if cfg!(target_os = "macos") {
            // …/BenchMesh.app/Contents/MacOS/benchmesh
            if let Some(bundle) = exe.ancestors().nth(3).filter(|b| b.extension().is_some_and(|e| e == "app")) {
                return Install::MacApp(bundle.to_path_buf());
            }
        }
        Install::Source
    }

    /// The `latest.json` platform suffix of this installation's file.
    fn kind(&self) -> Option<&'static str> {
        match self {
            Install::AppImage(_) => Some("appimage"),
            Install::Windows(_) => Some("nsis"),
            Install::MacApp(_) => Some("app"),
            Install::Deb => Some("deb"),
            Install::Source => None,
        }
    }

    /// Replaces itself (the others: a command to run, or nothing).
    fn replaces_itself(&self) -> bool {
        matches!(self, Install::AppImage(_) | Install::Windows(_) | Install::MacApp(_))
    }

    fn describe(&self) -> &'static str {
        match self {
            Install::AppImage(_) => "AppImage",
            Install::Windows(_) => "Windows installer",
            Install::MacApp(_) => "macOS app",
            Install::Deb => ".deb package",
            Install::Source => "built from source",
        }
    }
}

/// `latest.json` key of a file, e.g. `linux-aarch64-appimage`.
pub fn platform_key(kind: &str) -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        o => o,
    };
    format!("{os}-{}-{kind}", std::env::consts::ARCH)
}

/// Numeric x.y.z comparison: `a` newer than `b`.
pub fn newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> { v.trim_start_matches('v').split(['-', '+']).next().unwrap_or("").split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let (l, r) = (parse(a), parse(b));
    for i in 0..l.len().max(r.len()) {
        let (x, y) = (l.get(i).copied().unwrap_or(0), r.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// `file` signed by the updater key: `signature` and `public_key` as in `latest.json` and
/// `tauri.conf.json` (base64 of the minisign text).
pub fn verify(file: &Path, signature: &str, public_key: &str) -> Result<(), String> {
    let text = |b64: &str, what: &str| -> Result<String, String> {
        let raw = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| format!("{what}: {e}"))?;
        String::from_utf8(raw).map_err(|e| format!("{what}: {e}"))
    };
    let pk = minisign_verify::PublicKey::decode(&text(public_key, "public key")?).map_err(|e| format!("public key: {e}"))?;
    let sig = minisign_verify::Signature::decode(&text(signature, "signature")?).map_err(|e| format!("signature: {e}"))?;
    let mut f = std::fs::File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let bad = |e: minisign_verify::Error| format!("the download is not signed by BenchMesh ({e}): not installed");
    match pk.verify_stream(&sig) {
        Ok(mut v) => {
            let mut buf = vec![0u8; 1 << 16];
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                v.update(&buf[..n]);
            }
            v.finalize().map_err(bad)
        }
        // a legacy (not prehashed) signature: the whole file at once
        Err(_) => {
            let mut data = Vec::new();
            f.read_to_end(&mut data).map_err(|e| e.to_string())?;
            pk.verify(&data, &sig, false).map_err(bad)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    platforms: std::collections::HashMap<String, Platform>,
}

#[derive(Debug, Clone, Deserialize)]
struct Platform {
    signature: String,
    url: String,
}

/// A downloaded, verified update waiting for the process to stop.
#[derive(Debug, Clone)]
pub struct Staged {
    pub install: Install,
    pub file: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Checking,
    Downloading,
    Installing,
    /// Downloaded and verified; the user runs the command (`.deb`).
    Manual,
    Failed,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Checking => "checking",
            Phase::Downloading => "downloading",
            Phase::Installing => "installing",
            Phase::Manual => "manual",
            Phase::Failed => "failed",
        }
    }
}

struct State {
    phase: Phase,
    checked_ms: Option<i64>,
    latest: Option<Manifest>,
    error: Option<String>,
    downloaded: u64,
    total: Option<u64>,
    command: Option<String>,
    staged: Option<Staged>,
}

pub struct Updater {
    /// `None` in the desktop app (its own updater offers new versions at start).
    install: Option<Install>,
    /// Where downloads go (`<data dir>/updates`).
    dir: Option<PathBuf>,
    endpoint: String,
    public_key: String,
    state: Mutex<State>,
}

impl Updater {
    /// `BENCHMESH_UPDATE_URL` replaces the manifest's address (staging); the key never changes.
    pub fn new(install: Option<Install>, data_dir: Option<&Path>) -> Updater {
        let endpoint = std::env::var("BENCHMESH_UPDATE_URL").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| LATEST_JSON.into());
        Updater::with_source(install, data_dir.map(|d| d.join("updates")), &endpoint, PUBLIC_KEY)
    }

    /// Another manifest and key (tests).
    pub fn with_source(install: Option<Install>, dir: Option<PathBuf>, endpoint: &str, public_key: &str) -> Updater {
        let state = State { phase: Phase::Idle, checked_ms: None, latest: None, error: None, downloaded: 0, total: None, command: None, staged: None };
        Updater { install, dir, endpoint: endpoint.into(), public_key: public_key.into(), state: Mutex::new(state) }
    }

    /// Why this installation cannot install updates from the UI, if it cannot.
    fn refusal(&self) -> Option<&'static str> {
        match &self.install {
            None => Some("the desktop app offers new versions when it starts"),
            Some(Install::Source) => Some("built from source: update the sources and build again"),
            Some(_) if self.dir.is_none() => Some("a simulation does not install updates"),
            Some(_) => None,
        }
    }

    fn available(&self, s: &State) -> bool {
        s.latest.as_ref().is_some_and(|m| newer(&m.version, env!("CARGO_PKG_VERSION")))
    }

    pub fn json(&self) -> Value {
        let s = self.state.lock().unwrap();
        json!({
            "current": env!("CARGO_PKG_VERSION"),
            "installed_as": self.install.as_ref().map(Install::describe).unwrap_or("desktop app"),
            "can_install": self.refusal().is_none(),
            "replaces_itself": self.install.as_ref().is_some_and(Install::replaces_itself),
            "reason": self.refusal(),
            "checked_ms": s.checked_ms,
            "latest": s.latest.as_ref().map(|m| json!({"version": m.version, "notes": m.notes, "pub_date": m.pub_date})),
            "available": self.available(&s),
            "phase": s.phase.name(),
            "downloaded": s.downloaded,
            "total": s.total,
            "error": s.error,
            "command": s.command,
        })
    }

    fn client() -> Result<reqwest::Client, String> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .user_agent(concat!("BenchMesh/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())
    }

    /// Read the latest release's manifest.
    pub async fn check(&self) -> Value {
        {
            let mut s = self.state.lock().unwrap();
            if matches!(s.phase, Phase::Downloading | Phase::Installing) {
                drop(s);
                return self.json();
            }
            s.phase = Phase::Checking;
        }
        let r: Result<Manifest, String> = async {
            let res = Self::client()?.get(&self.endpoint).timeout(Duration::from_secs(30)).send().await.map_err(|e| format!("cannot reach GitHub: {e}"))?;
            if !res.status().is_success() {
                return Err(format!("GitHub answered {}", res.status()));
            }
            res.json::<Manifest>().await.map_err(|e| format!("the release manifest: {e}"))
        }
        .await;
        let mut s = self.state.lock().unwrap();
        s.checked_ms = Some(now_ms());
        match r {
            Ok(m) => {
                s.latest = Some(m);
                s.error = None;
                s.phase = if s.command.is_some() { Phase::Manual } else { Phase::Idle };
            }
            Err(e) => {
                eprintln!("update check: {e}");
                s.error = Some(e);
                s.phase = Phase::Failed;
            }
        }
        drop(s);
        self.json()
    }

    /// Start downloading the newer version; `ready` is called once it is verified and staged
    /// for an installation that replaces itself.
    pub fn start_install(self: &Arc<Self>, ready: impl FnOnce(Staged) + Send + 'static) -> Result<(), String> {
        if let Some(why) = self.refusal() {
            return Err(why.into());
        }
        let install = self.install.clone().unwrap_or(Install::Source);
        let kind = install.kind().ok_or("nothing to install")?;
        let (version, platform) = {
            let mut s = self.state.lock().unwrap();
            if matches!(s.phase, Phase::Downloading | Phase::Installing | Phase::Checking) {
                return Err(format!("already {}", s.phase.name()));
            }
            if !self.available(&s) {
                return Err("no newer version: check for updates first".into());
            }
            let m = s.latest.clone().unwrap();
            let key = platform_key(kind);
            let p = m.platforms.get(&key).cloned().ok_or_else(|| format!("the release has no {key} file"))?;
            s.phase = Phase::Downloading;
            s.downloaded = 0;
            s.total = None;
            s.error = None;
            s.command = None;
            (m.version, p)
        };
        let me = self.clone();
        tokio::spawn(async move {
            match me.download(&install, &version, &platform).await {
                Ok(staged) if install.replaces_itself() => {
                    eprintln!("update: {version} downloaded and verified, installing");
                    let mut s = me.state.lock().unwrap();
                    s.phase = Phase::Installing;
                    s.staged = Some(staged.clone());
                    drop(s);
                    ready(staged);
                }
                Ok(staged) => {
                    let mut s = me.state.lock().unwrap();
                    s.command = Some(format!("sudo apt install {}", shell_quote(&staged.file.display().to_string())));
                    s.phase = Phase::Manual;
                }
                Err(e) => {
                    eprintln!("update: {e}");
                    let mut s = me.state.lock().unwrap();
                    s.error = Some(e);
                    s.phase = Phase::Failed;
                }
            }
        });
        Ok(())
    }

    async fn download(&self, install: &Install, version: &str, p: &Platform) -> Result<Staged, String> {
        let dir = self.dir.clone().ok_or("no data directory")?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = p.url.rsplit('/').next().filter(|n| !n.is_empty() && !n.contains(['\\', '/'])).unwrap_or("update").replace("..", "");
        let part = dir.join(format!("{name}.part"));
        let mut res = Self::client()?.get(&p.url).send().await.map_err(|e| format!("download: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("download: GitHub answered {}", res.status()));
        }
        self.state.lock().unwrap().total = res.content_length();
        let result = async {
            let mut f = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
            while let Some(chunk) = res.chunk().await.map_err(|e| format!("download: {e}"))? {
                f.write_all(&chunk).map_err(|e| format!("{}: {e}", part.display()))?;
                self.state.lock().unwrap().downloaded += chunk.len() as u64;
            }
            f.sync_all().map_err(|e| e.to_string())?;
            drop(f);
            let (part2, sig, key) = (part.clone(), p.signature.clone(), self.public_key.clone());
            tokio::task::spawn_blocking(move || verify(&part2, &sig, &key)).await.map_err(|e| e.to_string())??;
            let file = dir.join(&name);
            std::fs::rename(&part, &file).map_err(|e| format!("{}: {e}", file.display()))?;
            Ok::<_, String>(Staged { install: install.clone(), file, version: version.to_string() })
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_file(&part);
        }
        result
    }

    /// The verified update waiting for the process to stop, if any.
    pub fn staged(&self) -> Option<Staged> {
        self.state.lock().unwrap().staged.clone()
    }
}

fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Install `staged` once BenchMesh has stopped. Returns the program to start now (with the same
/// arguments), or `None` when a helper starts it (Windows: after the installer).
pub fn apply(staged: &Staged) -> Result<Option<PathBuf>, String> {
    match &staged.install {
        Install::AppImage(target) => swap_appimage(&staged.file, target).map(Some),
        Install::MacApp(bundle) => swap_mac_app(&staged.file, bundle).map(Some),
        Install::Windows(exe) => run_windows_installer(&staged.file, exe).map(|_| None),
        Install::Deb | Install::Source => Err("this installation does not replace itself".into()),
    }
}

/// The new AppImage in place of the old one (kept as `<file>.previous`), same name: a start
/// entry pointing at it stays valid.
pub fn swap_appimage(new: &Path, target: &Path) -> Result<PathBuf, String> {
    let dir = target.parent().ok_or("the AppImage has no folder")?;
    let tmp = dir.join(".benchmesh-update.AppImage");
    std::fs::copy(new, &tmp).map_err(|e| format!("cannot write next to {}: {e}", target.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let previous = PathBuf::from(format!("{}.previous", target.display()));
    let _ = std::fs::remove_file(&previous);
    std::fs::rename(target, &previous).map_err(|e| format!("{}: {e}", target.display()))?;
    if let Err(e) = std::fs::rename(&tmp, target) {
        let _ = std::fs::rename(&previous, target);
        return Err(format!("{}: {e}", target.display()));
    }
    let _ = std::fs::remove_file(new);
    Ok(target.to_path_buf())
}

/// The new `.app` from the update archive in place of the old one (kept as `<bundle>.previous`).
fn swap_mac_app(archive: &Path, bundle: &Path) -> Result<PathBuf, String> {
    let parent = bundle.parent().ok_or("the app has no folder")?;
    let unpack = parent.join(".benchmesh-update");
    let _ = std::fs::remove_dir_all(&unpack);
    std::fs::create_dir_all(&unpack).map_err(|e| format!("cannot write next to {}: {e}", bundle.display()))?;
    let out = std::process::Command::new("tar").arg("-xzf").arg(archive).arg("-C").arg(&unpack).output().map_err(|e| format!("tar: {e}"))?;
    if !out.status.success() {
        return Err(format!("tar: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let new = std::fs::read_dir(&unpack)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|x| x == "app"))
        .ok_or("the update archive holds no .app")?;
    let previous = PathBuf::from(format!("{}.previous", bundle.display()));
    let _ = std::fs::remove_dir_all(&previous);
    std::fs::rename(bundle, &previous).map_err(|e| format!("{}: {e}", bundle.display()))?;
    if let Err(e) = std::fs::rename(&new, bundle) {
        let _ = std::fs::rename(&previous, bundle);
        return Err(format!("{}: {e}", bundle.display()));
    }
    let _ = std::fs::remove_dir_all(&unpack);
    let _ = std::fs::remove_file(archive);
    let exe = std::env::current_exe().ok().and_then(|e| e.file_name().map(|n| n.to_owned())).unwrap_or_else(|| "benchmesh".into());
    Ok(bundle.join("Contents/MacOS").join(exe))
}

/// Windows cannot replace a running executable: a detached helper waits for BenchMesh to exit,
/// runs the installer silently and starts BenchMesh again with the same arguments.
pub fn windows_helper_line(installer: &Path, exe: &Path, args: &[String]) -> String {
    use super::autostart::windows_word;
    let again: Vec<String> = std::iter::once(exe.display().to_string()).chain(args.iter().cloned()).map(|a| windows_word(&a)).collect();
    // ping: a delay that works without a console (timeout does not)
    format!("/C ping -n 4 127.0.0.1 >NUL & {} /S & start \"\" {}", windows_word(&installer.display().to_string()), again.join(" "))
}

#[cfg(windows)]
fn run_windows_installer(installer: &Path, exe: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::Command::new("cmd")
        .raw_arg(windows_helper_line(installer, exe, &args))
        .creation_flags(0x0000_0008 | 0x0800_0000) // DETACHED_PROCESS | CREATE_NO_WINDOW
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot start the installer: {e}"))
}

#[cfg(not(windows))]
fn run_windows_installer(_: &Path, _: &Path) -> Result<(), String> {
    Err("the Windows installer runs on Windows only".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/update/fixture.bin");

    fn fixture_sig() -> String {
        std::fs::read_to_string(format!("{FIXTURE}.sig")).unwrap()
    }

    #[test]
    fn the_key_is_the_desktop_app_s() {
        let conf: Value = serde_json::from_str(include_str!("../../../benchmesh-app/tauri.conf.json")).unwrap();
        assert_eq!(conf["plugins"]["updater"]["pubkey"], PUBLIC_KEY);
        assert_eq!(conf["plugins"]["updater"]["endpoints"][0], LATEST_JSON);
    }

    #[test]
    fn only_files_signed_with_the_updater_key_pass() {
        assert_eq!(verify(Path::new(FIXTURE), &fixture_sig(), PUBLIC_KEY), Ok(()));
        let dir = std::env::temp_dir().join(format!("bm-update-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tampered = dir.join("tampered.bin");
        std::fs::write(&tampered, "BenchMesh update signature fixture!\n").unwrap();
        let e = verify(&tampered, &fixture_sig(), PUBLIC_KEY).unwrap_err();
        assert!(e.contains("not signed by BenchMesh"), "{e}");
        assert!(verify(Path::new(FIXTURE), "bm90IGEgc2lnbmF0dXJl", PUBLIC_KEY).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(newer("0.1.15", "0.1.14"));
        assert!(newer("0.1.10", "0.1.9"));
        assert!(newer("v1.0.0", "0.9.99"));
        assert!(!newer("0.1.14", "0.1.14"));
        assert!(!newer("0.1.13", "0.1.14"));
    }

    #[test]
    fn the_platform_key_names_os_architecture_and_file() {
        let k = platform_key("appimage");
        assert!(k.ends_with(&format!("-{}-appimage", std::env::consts::ARCH)), "{k}");
        if cfg!(target_os = "linux") {
            assert!(k.starts_with("linux-"));
        }
    }

    #[test]
    fn the_appimage_is_swapped_and_the_old_one_kept() {
        let dir = std::env::temp_dir().join(format!("bm-update-swap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("updates")).unwrap();
        let target = dir.join("BenchMesh_0.1.14_amd64.AppImage");
        std::fs::write(&target, "old").unwrap();
        let new = dir.join("updates/BenchMesh_0.1.15_amd64.AppImage");
        std::fs::write(&new, "new").unwrap();
        assert_eq!(swap_appimage(&new, &target).unwrap(), target);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(dir.join("BenchMesh_0.1.14_amd64.AppImage.previous")).unwrap(), "old");
        assert!(!new.exists() && !dir.join(".benchmesh-update.AppImage").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_windows_helper_waits_installs_silently_and_starts_again() {
        let line = windows_helper_line(Path::new(r"C:\Users\lab\.benchmesh\updates\BenchMesh_0.1.15_x64-setup.exe"), Path::new(r"C:\Users\lab\AppData\Local\BenchMesh\benchmesh.exe"), &["--headless".into(), "--data-dir".into(), r"D:\bench data".into()]);
        assert_eq!(line, r#"/C ping -n 4 127.0.0.1 >NUL & C:\Users\lab\.benchmesh\updates\BenchMesh_0.1.15_x64-setup.exe /S & start "" C:\Users\lab\AppData\Local\BenchMesh\benchmesh.exe --headless --data-dir "D:\bench data""#);
    }

    /// A release served locally: check, download, verify, stage, then install over an AppImage.
    async fn serve_release(file: Vec<u8>, signature: String) -> (String, tokio::task::JoinHandle<()>) {
        use axum::routing::get;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let manifest = json!({
            "version": "99.0.0", "notes": "Better things", "pub_date": "2026-10-22T00:00:00Z",
            "platforms": {platform_key("appimage"): {"signature": signature, "url": format!("{base}/download/BenchMesh_99.0.0.AppImage")}},
        });
        let app = axum::Router::new()
            .route("/latest.json", get(move || async move { axum::Json(manifest) }))
            .route("/download/BenchMesh_99.0.0.AppImage", get(move || async move { file }));
        (base, tokio::spawn(async move { axum::serve(listener, app).await.unwrap() }))
    }

    async fn settle(u: &Updater) -> Value {
        for _ in 0..200 {
            let j = u.json();
            if !matches!(j["phase"].as_str(), Some("checking" | "downloading")) {
                return j;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("the download did not finish: {}", u.json());
    }

    #[tokio::test]
    async fn a_signed_release_is_downloaded_verified_and_installed() {
        let dir = std::env::temp_dir().join(format!("bm-update-flow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("BenchMesh.AppImage");
        std::fs::write(&target, "old").unwrap();
        let (base, server) = serve_release(std::fs::read(FIXTURE).unwrap(), fixture_sig()).await;
        let u = Arc::new(Updater::with_source(Some(Install::AppImage(target.clone())), Some(dir.join("updates")), &format!("{base}/latest.json"), PUBLIC_KEY));

        assert!(u.start_install(|_| {}).unwrap_err().contains("check for updates first"));
        let j = u.check().await;
        assert_eq!((j["available"].as_bool(), j["latest"]["version"].as_str(), j["phase"].as_str()), (Some(true), Some("99.0.0"), Some("idle")), "{j}");

        let (tx, rx) = tokio::sync::oneshot::channel();
        u.start_install(move |s| tx.send(s).unwrap()).unwrap();
        let staged = tokio::time::timeout(Duration::from_secs(10), rx).await.unwrap().unwrap();
        let j = settle(&u).await;
        assert_eq!(j["phase"], "installing", "{j}");
        assert_eq!(j["downloaded"], std::fs::metadata(FIXTURE).unwrap().len());
        assert_eq!(staged.version, "99.0.0");
        assert_eq!(apply(&staged).unwrap(), Some(target.clone()));
        assert_eq!(std::fs::read(&target).unwrap(), std::fs::read(FIXTURE).unwrap());
        server.abort();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_download_with_a_wrong_signature_is_refused_and_removed() {
        let dir = std::env::temp_dir().join(format!("bm-update-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (base, server) = serve_release(b"something else".to_vec(), fixture_sig()).await;
        let u = Arc::new(Updater::with_source(Some(Install::Deb), Some(dir.clone()), &format!("{base}/latest.json"), PUBLIC_KEY));
        u.check().await;
        let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let r = ran.clone();
        // a .deb is not on this platform's key in the served manifest
        assert!(u.start_install(move |_| r.store(true, std::sync::atomic::Ordering::SeqCst)).unwrap_err().contains("-deb file"));
        let u = Arc::new(Updater::with_source(Some(Install::AppImage(dir.join("x.AppImage"))), Some(dir.clone()), &format!("{base}/latest.json"), PUBLIC_KEY));
        u.check().await;
        let r = ran.clone();
        u.start_install(move |_| r.store(true, std::sync::atomic::Ordering::SeqCst)).unwrap();
        let j = settle(&u).await;
        assert_eq!(j["phase"], "failed");
        assert!(j["error"].as_str().unwrap().contains("not signed by BenchMesh"), "{j}");
        assert!(!ran.load(std::sync::atomic::Ordering::SeqCst), "never installed");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "the download is deleted");
        assert!(u.staged().is_none());
        server.abort();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_deb_is_downloaded_and_the_command_shown() {
        let dir = std::env::temp_dir().join(format!("bm-update-deb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let manifest = json!({"version": "99.0.0", "platforms": {platform_key("deb"): {"signature": fixture_sig(), "url": format!("{base}/BenchMesh_99.0.0_arm64.deb")}}});
        let file = std::fs::read(FIXTURE).unwrap();
        let app = axum::Router::new()
            .route("/latest.json", axum::routing::get(move || async move { axum::Json(manifest) }))
            .route("/BenchMesh_99.0.0_arm64.deb", axum::routing::get(move || async move { file }));
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let u = Arc::new(Updater::with_source(Some(Install::Deb), Some(dir.clone()), &format!("{base}/latest.json"), PUBLIC_KEY));
        u.check().await;
        u.start_install(|_| panic!("a .deb never installs itself")).unwrap();
        let j = settle(&u).await;
        assert_eq!(j["phase"], "manual", "{j}");
        assert_eq!(j["command"], format!("sudo apt install {}", dir.join("BenchMesh_99.0.0_arm64.deb").display()));
        assert!(dir.join("BenchMesh_99.0.0_arm64.deb").exists());
        server.abort();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_each_installation_can_do() {
        let u = |i: Option<Install>, dir: Option<PathBuf>| Updater::with_source(i, dir, "http://127.0.0.1:9/latest.json", PUBLIC_KEY).json();
        let d = Some(PathBuf::from("/tmp/x"));
        assert_eq!(u(None, d.clone())["reason"], "the desktop app offers new versions when it starts");
        assert_eq!(u(Some(Install::Source), d.clone())["can_install"], false);
        let deb = u(Some(Install::Deb), d.clone());
        assert_eq!((deb["can_install"].as_bool(), deb["replaces_itself"].as_bool()), (Some(true), Some(false)));
        assert_eq!(u(Some(Install::AppImage("/a".into())), d)["replaces_itself"], true);
        assert_eq!(u(Some(Install::AppImage("/a".into())), None)["reason"], "a simulation does not install updates");
    }
}
