//! The extension point of a BenchMesh distribution (contract `ai_context/OPEN_CORE.md`).
//!
//! The open-source build has no extensions: `benchmesh-server` and the desktop app built from this
//! repository pass `Extensions::default()`. The official distribution adds its own (app updates,
//! online services) at build time through the `benchmesh-extensions` crate, which the desktop app
//! asks for them. An extension adds routes to the same API: they are served behind the same
//! cross-site guard and remote gate, and their remote scope follows their path like every other
//! route (`remote::scope`; an unclassified path is local-only).

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;

use crate::S;

/// The distribution name of a build without extensions (`GET /version` `distribution`).
pub const COMMUNITY: &str = "community";

/// What an extension is told about the BenchMesh it runs in.
#[derive(Debug, Clone)]
pub struct Host {
    /// The user data directory (`None`: a simulation, everything in memory).
    pub data_dir: Option<PathBuf>,
    /// Running without an app window (`--headless`): the process may stop and start itself
    /// (`system::System::hand_over`).
    pub headless: bool,
}

/// One route of an extension as `/openapi.json` describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiRoute {
    pub method: &'static str,
    pub path: &'static str,
    /// One of the API's tags (`openapi::TAGS`).
    pub tag: &'static str,
    pub summary: &'static str,
    /// It takes a JSON body.
    pub body: bool,
    /// Why the API reference must not send it (`x-no-try`).
    pub no_try: Option<&'static str>,
}

pub trait Extension: Send + Sync {
    /// Short name for logs.
    fn name(&self) -> &'static str;
    /// Routes merged into the API.
    fn routes(&self) -> Router<S>;
    /// Every route of `routes()`, for `/openapi.json`.
    fn api(&self) -> Vec<ApiRoute>;
}

impl fmt::Debug for dyn Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Extension({})", self.name())
    }
}

/// The extensions of this build and the distribution they make.
#[derive(Debug, Clone)]
pub struct Extensions {
    /// `community` for a build of the open-source repository; the official build names itself.
    pub distribution: &'static str,
    pub list: Vec<Arc<dyn Extension>>,
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions { distribution: COMMUNITY, list: Vec::new() }
    }
}

impl Extensions {
    /// The documented routes of every extension.
    pub fn api(&self) -> Vec<ApiRoute> {
        self.list.iter().flat_map(|e| e.api()).collect()
    }
}
