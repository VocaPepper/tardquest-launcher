mod catalog;
mod commands;
mod fetch;
mod installer;
mod launch;
mod model;
mod scan;

use std::sync::Mutex;
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = commands::load_state(&app.handle());
            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_catalog,
            commands::fetch_channel,
            commands::scan_install,
            commands::download_and_apply,
            commands::uninstall,
            commands::launch_game,
            commands::open_privacy,
            commands::set_install_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
