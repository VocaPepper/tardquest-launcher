use crate::installer::{edition_dir, find_binary};
use crate::model::{version_key, ScanResult};
use std::collections::HashMap;
use std::path::Path;

pub fn scan(base: &Path, edition: &str, patch_key: Option<&str>, local_patches: &HashMap<String, String>) -> ScanResult {
    let dir = edition_dir(base, edition);
    let mut installed_versions: Vec<String> = Vec::new();

    if dir.exists() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && find_binary(&p).is_some() {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        installed_versions.push(name.to_string());
                    }
                }
            }
        }
    }
    installed_versions.sort_by(|a, b| version_key(b).cmp(&version_key(a)));

    let installed_patch = patch_key.and_then(|k| local_patches.get(k).cloned());

    ScanResult {
        install_dir: dir.to_string_lossy().to_string(),
        installed_versions,
        installed_patch,
    }
}
