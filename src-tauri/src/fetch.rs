use crate::catalog::platform;
use crate::model::{Build, Catalog, Edition};
use anyhow::{anyhow, Result};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD};
use regex::Regex;
use reqwest::Client;

pub async fn fetch_edition(client: &Client, catalog: &Catalog, edition_key: &str) -> Result<Vec<Build>> {
    let edition = catalog
        .editions
        .get(edition_key)
        .ok_or_else(|| anyhow!("unknown edition: {edition_key}"))?;
    match edition.source.as_str() {
        "github" => fetch_github(client, edition_key, edition).await,
        "tqo" => fetch_tqo(client, edition_key, edition).await,
        other => Err(anyhow!("unsupported edition source: {other}")),
    }
}

async fn fetch_github(client: &Client, edition_key: &str, edition: &Edition) -> Result<Vec<Build>> {
    let cfg = edition.github.as_ref().ok_or_else(|| anyhow!("github config missing"))?;
    let matcher = cfg
        .assets
        .get(platform())
        .ok_or_else(|| anyhow!("no asset matcher for platform {} in {}", platform(), edition_key))?;
    let re = Regex::new(matcher)?;

    let url = format!("https://api.github.com/repos/{}/releases?per_page=100", cfg.repo);
    let resp = client
        .get(&url)
        .header("User-Agent", "tq-launcher")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("GitHub API {}: {}", resp.status(), url));
    }
    let releases: Vec<serde_json::Value> = resp.json().await?;

    let mut builds = Vec::new();
    for rel in releases {
        let tag = rel.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if tag.is_empty() {
            continue;
        }
        let notes = rel.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let assets = rel.get("assets").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        if let Some(asset) = assets.into_iter().find(|a| {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            re.is_match(name)
        }) {
            let name = asset.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let url = asset.get("browser_download_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let size = asset.get("size").and_then(|v| v.as_u64());
            let sha256 = asset
                .get("digest")
                .and_then(|v| v.as_str())
                .and_then(digest_to_hex);
            builds.push(Build {
                edition: edition_key.to_string(),
                version: tag.clone(),
                label: tag.clone(),
                file_name: name,
                download_url: url,
                sha256,
                size,
                release_notes: notes,
                patch: None,
            });
        }
    }
    builds.sort_by(|a, b| crate::model::version_key(&b.version).cmp(&crate::model::version_key(&a.version)));
    Ok(builds)
}

async fn fetch_tqo(client: &Client, edition_key: &str, edition: &Edition) -> Result<Vec<Build>> {
    let cfg = edition.tqo.as_ref().ok_or_else(|| anyhow!("tqo config missing"))?;
    let url = format!("{}/{}/latest.json", cfg.base_url.trim_end_matches('/'), cfg.channel);
    let resp = client
        .get(&url)
        .header("User-Agent", "tq-launcher")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("TQO manifest {}: {}", resp.status(), url));
    }
    let v: serde_json::Value = resp.json().await?;
    let patch = v.get("patch").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let file_name = v
        .get("file_name")
        .and_then(|x| x.as_str())
        .unwrap_or(&cfg.file_name)
        .to_string();
    let download_url = v.get("download_url").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let sha256 = v.get("sha256").and_then(|x| x.as_str()).map(|s| s.to_string());
    let size = v.get("size").and_then(|x| x.as_u64());
    let notes = v.get("release_notes").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let version = v.get("version").and_then(|x| x.as_str()).unwrap_or(&cfg.channel).to_string();

    Ok(vec![Build {
        edition: edition_key.to_string(),
        version,
        label: cfg.channel.clone(),
        file_name,
        download_url,
        sha256,
        size,
        release_notes: notes,
        patch: Some(patch),
    }])
}

/// Convert a GitHub asset digest (base64, possibly `sha256:`-prefixed, possibly url-safe)
/// into a lowercase hex string. Falls back to treating the value as raw hex if present.
fn digest_to_hex(value: &str) -> Option<String> {
    let s = value.trim().strip_prefix("sha256:").unwrap_or(value.trim());
    for engine in [STANDARD, URL_SAFE_NO_PAD, URL_SAFE] {
        if let Ok(bytes) = engine.decode(s) {
            if bytes.len() == 32 {
                return Some(hex(&bytes));
            }
        }
    }
    if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(s.to_ascii_lowercase());
    }
    None
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", b);
    }
    out
}
