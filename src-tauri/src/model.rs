use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub struct Catalog {
    pub editions: HashMap<String, Edition>,
}

#[derive(Deserialize, Clone)]
pub struct Edition {
    pub source: String,
    #[serde(default)]
    pub github: Option<GithubConfig>,
    #[serde(default)]
    pub tqo: Option<TqoConfig>,
    #[serde(default)]
    pub display: Option<Display>,
}

#[derive(Deserialize, Clone)]
pub struct GithubConfig {
    pub repo: String,
    pub assets: HashMap<String, String>,
}

#[derive(Deserialize, Clone)]
pub struct TqoConfig {
    pub base_url: String,
    pub channel: String,
    pub file_name: String,
}

#[derive(Deserialize, Clone)]
pub struct Display {
    pub title: String,
    pub subtitle: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Build {
    pub edition: String,
    pub version: String,
    pub label: String,
    pub file_name: String,
    pub download_url: String,
    pub sha256: Option<String>,
    pub size: Option<u64>,
    pub release_notes: String,
    pub patch: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EditionInfo {
    pub key: String,
    pub title: String,
    pub subtitle: String,
    pub source: String,
}

#[derive(Serialize, Clone)]
pub struct ScanResult {
    pub install_dir: String,
    pub installed_versions: Vec<String>,
    pub installed_patch: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppState {
    pub install_dir: Option<String>,
    pub local_patches: HashMap<String, String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            install_dir: None,
            local_patches: HashMap::new(),
        }
    }
}

/// Sortable key for semver-ish versions like 1.19.2 or 1.19.2-251213 or 0.0.0-ERA.
pub fn version_key(version: &str) -> (i64, i64, i64, String) {
    let v = version.trim();
    let m = v.find(|c| c == '+' || c == '-');
    let (core, suffix) = match m {
        Some(i) => (&v[..i], v[i..].to_string()),
        None => (v, String::new()),
    };
    let parts: Vec<&str> = core.split('.').collect();
    let major = parts.get(0).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    (major, minor, patch, suffix)
}
