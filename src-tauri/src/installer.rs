use crate::model::Build;
use anyhow::{anyhow, Result};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

pub fn edition_dir(base: &Path, edition: &str) -> PathBuf {
    base.join(sanitize(edition))
}

pub fn version_dir(base: &Path, edition: &str, version: &str) -> PathBuf {
    edition_dir(base, edition).join(sanitize(version))
}

fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}

pub fn log(app: &AppHandle, msg: &str) {
    let _ = app.emit("log", msg.to_string());
}

/// Find the game binary inside a directory (recursive). Prefers a name containing the version,
/// otherwise the single matching binary; otherwise the first match.
pub fn find_binary(dir: &Path) -> Option<PathBuf> {
    if !dir.exists() {
        return None;
    }
    let is_win = cfg!(target_os = "windows");
    let entries = list_binaries(dir, is_win);
    if entries.is_empty() {
        return None;
    }
    // Prefer a file whose name matches the containing folder version.
    let folder = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if let Some(p) = entries.iter().find(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.contains(folder))
            .unwrap_or(false)
    }) {
        return Some(p.clone());
    }
    if entries.len() == 1 {
        return Some(entries[0].clone());
    }
    Some(entries[0].clone())
}

fn list_binaries(dir: &Path, is_win: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(list_binaries(&p, is_win));
            } else {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    let ok = if is_win {
                        name.to_ascii_lowercase().ends_with(".exe")
                    } else {
                        name.ends_with(".AppImage")
                    };
                    if ok {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

pub async fn download_and_apply(app: &AppHandle, base: &Path, edition: &str, build: &Build) -> Result<()> {
    let client = Client::builder().build()?;
    let vdir = version_dir(base, edition, &build.version);

    if vdir.exists() && build.version == "PTE" {
        log(app, "Removing old build...");
        fs::remove_dir_all(&vdir)?;
    }
    fs::create_dir_all(&vdir)?;

    let tmp = std::env::temp_dir()
        .join(format!("tq-{}-{}", edition, build.version))
        .join(&build.file_name);
    if let Some(parent) = tmp.parent() {
        fs::create_dir_all(parent)?;
    }

    log(app, &format!("Downloading {}", build.file_name));
    set_progress(app, 0.0);

    let mut resp = client.get(&build.download_url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {} while downloading {}", resp.status(), build.download_url));
    }
    let total = build.size.or(resp.content_length());
    let mut hasher = Sha256::new();
    let mut file = fs::File::create(&tmp)?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk)?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if let Some(t) = total {
            set_progress(app, (downloaded as f64) / (t as f64).max(1.0));
        }
    }
    file.flush()?;

    if let Some(expected) = &build.sha256 {
        log(app, "Verifying file...");
        let actual = hex(&hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&tmp);
            return Err(anyhow!("Hash mismatch; download corrupted"));
        }
    }

    let suffix = Path::new(&build.file_name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if suffix == "zip" {
        log(app, "Extracting archive...");
        let f = fs::File::open(&tmp)?;
        let mut archive = zip::ZipArchive::new(f)?;
        archive.extract(&vdir)?;
        let bin = find_binary(&vdir).ok_or_else(|| anyhow!("No matching binary found after extraction"))?;
        make_exec(&bin)?;
        let _ = fs::remove_file(&tmp);
    } else {
        let final_path = vdir.join(&build.file_name);
        fs::rename(&tmp, &final_path)?;
        make_exec(&final_path)?;
    }

    set_progress(app, 1.0);
    log(app, "Update applied");
    Ok(())
}

pub fn uninstall(base: &Path, edition: &str, version: &str) -> Result<()> {
    let vdir = version_dir(base, edition, version);
    if vdir.exists() {
        fs::remove_dir_all(&vdir)?;
        Ok(())
    } else {
        Err(anyhow!("No installed build found for {version}"))
    }
}

pub fn set_progress(app: &AppHandle, fraction: f64) {
    let _ = app.emit("download-progress", fraction.clamp(0.0, 1.0));
}

pub fn make_exec(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(perms.mode() | 0o755);
        fs::set_permissions(path, perms)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", b);
    }
    out
}
