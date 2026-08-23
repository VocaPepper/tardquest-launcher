use crate::model::Catalog;
use anyhow::{anyhow, Result};
use std::path::Path;

/// Bundled at build time — the "thin bootstrap" catalog.
const BUNDLED: &str = include_str!("../../catalog.json");

/// Load the catalog, preferring a runtime-override copy in the config dir if present.
pub fn load_catalog(config_dir: &Path) -> Result<Catalog> {
    let override_path = config_dir.join("catalog.json");
    if override_path.exists() {
        let text = std::fs::read_to_string(&override_path)?;
        return serde_json::from_str(&text).map_err(|e| anyhow!("catalog.json override invalid: {e}"));
    }
    serde_json::from_str(BUNDLED).map_err(|e| anyhow!("bundled catalog.json invalid: {e}"))
}

pub fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}
