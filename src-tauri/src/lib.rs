mod app_config;
mod storage;
mod tray;
mod watcher;

use notify_debouncer_full::notify::RecommendedWatcher;
use notify_debouncer_full::{Debouncer, RecommendedCache};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Manager, RunEvent, WindowEvent};

pub(crate) struct AppState {
    pub(crate) is_quitting: AtomicBool,
    // Canonical, validated storage folder. Set by `set_storage_folder` once
    // the frontend has resolved (and created, if needed) the user's chosen
    // folder. All storage commands read this — the renderer is no longer
    // trusted to pass the folder on every call.
    pub(crate) storage_folder: Mutex<Option<PathBuf>>,
    // Debounced FS watcher for the current storage folder. Replaced when the
    // user changes the folder via Settings. The `Drop` impl on `Debouncer`
    // stops the underlying watcher thread, so simply overwriting this field
    // tears down the previous watcher.
    pub(crate) watcher: Mutex<Option<Debouncer<RecommendedWatcher, RecommendedCache>>>,
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppState {
            is_quitting: AtomicBool::new(false),
            storage_folder: Mutex::new(None),
            watcher: Mutex::new(None),
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            app_config::get_default_storage_folder,
            storage::path_exists,
            storage::create_dir,
            storage::set_storage_folder,
            storage::list_md_files,
            storage::read_note,
            storage::write_note,
            storage::delete_note,
            storage::get_mtime,
            storage::read_meta,
            storage::read_meta_bak,
            storage::write_meta,
            storage::delete_meta,
            app_config::read_app_config,
            app_config::write_app_config,
        ])
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if !state.is_quitting.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            WindowEvent::ThemeChanged(theme) => {
                let app = window.app_handle();
                if let Some(tray_icon) = app.tray_by_id("notd-tray") {
                    tray::apply_tray_theme(&tray_icon, *theme);
                }
            }
            _ => {}
        })
        .setup(|app| {
            tray::build_tray(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { .. } => {
            let state = app_handle.state::<AppState>();
            state.is_quitting.store(true, Ordering::Relaxed);
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => {
            show_main_window(app_handle);
        }
        _ => {}
    });
}
