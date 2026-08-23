use crate::catalog::load_catalog;
use crate::fetch::fetch_edition;
use crate::installer::{download_and_apply as install, log, uninstall as remove};
use crate::launch::launch as do_launch;
use crate::model::{AppState, Build, Catalog, EditionInfo, ScanResult};
use crate::scan::scan as do_scan;
use anyhow::Result;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

fn state_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("launcher.config")
}

pub fn load_state(app: &AppHandle) -> AppState {
    let p = state_path(app);
    if let Ok(text) = std::fs::read_to_string(&p) {
        if let Ok(s) = serde_json::from_str::<AppState>(&text) {
            return s;
        }
    }
    AppState::default()
}

pub fn save_state(app: &AppHandle, state: &AppState) {
    let p = state_path(app);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&p, text);
    }
}

fn base_dir(app: &AppHandle, state: &Mutex<AppState>) -> PathBuf {
    let st = state.lock().unwrap();
    if let Some(d) = &st.install_dir {
        return PathBuf::from(d);
    }
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn catalog_for(app: &AppHandle) -> Result<Catalog> {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    load_catalog(&config_dir)
}

fn tqo_channel(catalog: &Catalog, edition: &str) -> Option<String> {
    catalog
        .editions
        .get(edition)
        .and_then(|e| e.tqo.as_ref())
        .map(|c| c.channel.clone())
}

fn patch_key(catalog: &Catalog, edition: &str) -> Option<String> {
    tqo_channel(catalog, edition).map(|ch| format!("{edition}/{ch}"))
}

fn cache_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("catalog_cache.json")
}

fn save_cache(app: &AppHandle, edition: &str, builds: &[Build]) {
    let p = cache_path(app);
    let mut map: std::collections::HashMap<String, Vec<Build>> = std::fs::read_to_string(&p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    map.insert(edition.to_string(), builds.to_vec());
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&p, t);
    }
}

fn load_cache(app: &AppHandle, edition: &str) -> Option<Vec<Build>> {
    let p = cache_path(app);
    let map: std::collections::HashMap<String, Vec<Build>> = std::fs::read_to_string(&p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())?;
    map.get(edition).cloned()
}

#[tauri::command]
pub async fn list_catalog(app: AppHandle) -> Result<Vec<EditionInfo>, String> {
    let catalog = catalog_for(&app).map_err(|e| e.to_string())?;
    let mut v = Vec::new();
    for (key, ed) in &catalog.editions {
        let d = ed.display.clone();
        v.push(EditionInfo {
            key: key.clone(),
            title: d.as_ref().map(|x| x.title.clone()).unwrap_or_default(),
            subtitle: d.as_ref().map(|x| x.subtitle.clone()).unwrap_or_default(),
            source: ed.source.clone(),
        });
    }
    v.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(v)
}

#[tauri::command]
pub async fn fetch_channel(app: AppHandle, edition: String) -> Result<Vec<Build>, String> {
    let catalog = catalog_for(&app).map_err(|e| e.to_string())?;
    let client = Client::builder().build().map_err(|e| e.to_string())?;
    match fetch_edition(&client, &catalog, &edition).await {
        Ok(builds) => {
            save_cache(&app, &edition, &builds);
            Ok(builds)
        }
        Err(e) => {
            if let Some(builds) = load_cache(&app, &edition) {
                let _ = app.emit("log", "Using cached catalog (offline).");
                Ok(builds)
            } else {
                Err(e.to_string())
            }
        }
    }
}

#[tauri::command]
pub async fn scan_install(app: AppHandle, state: State<'_, Mutex<AppState>>, edition: String) -> Result<ScanResult, String> {
    let catalog = catalog_for(&app).map_err(|e| e.to_string())?;
    let base = base_dir(&app, &state);
    let pkey = patch_key(&catalog, &edition);
    let lp = state.lock().unwrap().local_patches.clone();
    Ok(do_scan(&base, &edition, pkey.as_deref(), &lp))
}

#[tauri::command]
pub async fn download_and_apply(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    edition: String,
    build: Build,
) -> Result<(), String> {
    let catalog = catalog_for(&app).map_err(|e| e.to_string())?;
    let base = base_dir(&app, &state);
    log(&app, &format!("Installing {} {}", edition, build.version));
    install(&app, &base, &edition, &build)
        .await
        .map_err(|e| {
            log(&app, &format!("Download failed: {e}"));
            e.to_string()
        })?;

    if build.patch.is_some() {
        let mut st = state.lock().unwrap();
        let key = patch_key(&catalog, &edition)
            .or_else(|| Some(format!("{}/{}", edition, build.version)))
            .unwrap_or_default();
        if let Some(patch) = &build.patch {
            st.local_patches.insert(key, patch.clone());
        }
        save_state(&app, &st);
    }
    Ok(())
}

#[tauri::command]
pub async fn uninstall(app: AppHandle, state: State<'_, Mutex<AppState>>, edition: String, version: String) -> Result<(), String> {
    let base = base_dir(&app, &state);
    remove(&base, &edition, &version).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, state: State<'_, Mutex<AppState>>, edition: String, version: String) -> Result<(), String> {
    let base = base_dir(&app, &state);
    do_launch(&app, &base, &edition, &version).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_privacy(app: AppHandle) -> Result<(), String> {
    // Reuse an existing window, or open a dedicated one.
    if let Some(window) = app.get_webview_window("privacy") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, "privacy", WebviewUrl::App("src/privacy/index.html".into()))
        .title("Privacy Policy")
        .inner_size(980.0, 720.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_install_dir(app: AppHandle, state: State<'_, Mutex<AppState>>, dir: String) -> Result<(), String> {
    let mut st = state.lock().unwrap();
    st.install_dir = Some(dir);
    save_state(&app, &st);
    let _ = app.emit("install-dir-changed", ());
    Ok(())
}
