mod catalog;
mod commands;
mod fetch;
mod installer;
mod launch;
mod model;
mod scan;

use std::sync::Mutex;
use tauri::Manager;

// Minimum supported Windows build: Windows 10 1809. Older builds cannot run
// WebView2, so we fail fast with a message instead of crashing on window init.
#[cfg(windows)]
mod os_check {
    use std::ffi::c_void;
    use std::mem::{size_of, zeroed};
    use std::ptr;

    #[repr(C)]
    struct OsVersionInfoW {
        dw_os_version_info_size: u32,
        dw_major_version: u32,
        dw_minor_version: u32,
        dw_build_number: u32,
        dw_platform_id: u32,
        sz_csd_version: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(lp_version_information: *mut OsVersionInfoW) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            h_wnd: *mut c_void,
            lp_text: *const u16,
            lp_caption: *const u16,
            u_type: u32,
        ) -> i32;
    }

    const MB_OK: u32 = 0x0000;
    const MB_ICONERROR: u32 = 0x0010;
    const MB_SETFOREGROUND: u32 = 0x00010000;
    const MIN_BUILD: u32 = 17763; // Windows 10 1809

    fn build_number() -> u32 {
        let mut info = unsafe {
            let mut info: OsVersionInfoW = zeroed();
            info.dw_os_version_info_size = size_of::<OsVersionInfoW>() as u32;
            info
        };
        unsafe {
            RtlGetVersion(&mut info);
        }
        info.dw_build_number
    }

    pub fn ensure_supported() {
        let build = build_number();
        if build < MIN_BUILD {
            let text = format!(
                "This version of Windows (build {build}) is not supported.\n\n\
                 TQ Launcher requires Windows 10 1809 (build 17763) or newer.\n\n\
                 WebView2 is not available on older Windows versions."
            );
            let title = "Unsupported Windows version";
            let text_w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
            let title_w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            unsafe {
                MessageBoxW(
                    ptr::null_mut(),
                    text_w.as_ptr(),
                    title_w.as_ptr(),
                    MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
                );
            }
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
mod os_check {
    pub fn ensure_supported() {}
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    os_check::ensure_supported();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
