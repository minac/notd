use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent, WindowEvent};

struct AppState {
    is_quitting: AtomicBool,
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct MdFileInfo {
    filename: String,
    mtime_ms: i64,
}

fn ensure_md_filename(filename: &str) -> Result<(), String> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return Err("Invalid filename".into());
    }
    if !filename.ends_with(".md") {
        return Err("Filename must end with .md".into());
    }
    Ok(())
}

fn mtime_ms(path: &Path) -> Result<i64, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified = metadata.modified().map_err(|e| e.to_string())?;
    let dur = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(dur.as_millis() as i64)
}

#[tauri::command]
fn get_default_storage_folder() -> String {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    home.join("Dropbox")
        .join("Apps")
        .join("notd")
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[tauri::command]
fn create_dir(path: String) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_md_files(folder: String) -> Result<Vec<MdFileInfo>, String> {
    let folder_path = Path::new(&folder);
    if !folder_path.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(folder_path).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        // Skip hidden files (e.g. .notd-meta.json never has .md ext, but be safe)
        if name.starts_with('.') {
            continue;
        }
        let mtime = mtime_ms(&path).unwrap_or(0);
        files.push(MdFileInfo {
            filename: name,
            mtime_ms: mtime,
        });
    }
    Ok(files)
}

#[tauri::command]
fn read_note(folder: String, filename: String) -> Result<String, String> {
    ensure_md_filename(&filename)?;
    let path = Path::new(&folder).join(&filename);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_note(folder: String, filename: String, contents: String) -> Result<(), String> {
    ensure_md_filename(&filename)?;
    let folder_path = Path::new(&folder);
    if !folder_path.exists() {
        fs::create_dir_all(folder_path).map_err(|e| e.to_string())?;
    }
    let path = folder_path.join(&filename);
    fs::write(&path, contents.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_note(folder: String, filename: String) -> Result<(), String> {
    ensure_md_filename(&filename)?;
    let path = Path::new(&folder).join(&filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_mtime(folder: String, filename: String) -> Result<i64, String> {
    ensure_md_filename(&filename)?;
    let path = Path::new(&folder).join(&filename);
    mtime_ms(&path)
}

#[tauri::command]
fn read_meta(folder: String) -> Result<Option<String>, String> {
    let path = Path::new(&folder).join(".notd-meta.json");
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn write_meta(folder: String, json: String) -> Result<(), String> {
    let folder_path = Path::new(&folder);
    if !folder_path.exists() {
        fs::create_dir_all(folder_path).map_err(|e| e.to_string())?;
    }
    let path = folder_path.join(".notd-meta.json");
    fs::write(&path, json.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_meta(folder: String) -> Result<(), String> {
    let path = Path::new(&folder).join(".notd-meta.json");
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn config_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

#[tauri::command]
fn read_app_config(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let path = config_file_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn write_app_config(app: tauri::AppHandle, json: String) -> Result<(), String> {
    let path = config_file_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, json.as_bytes()).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppState {
            is_quitting: AtomicBool::new(false),
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_default_storage_folder,
            path_exists,
            create_dir,
            list_md_files,
            read_note,
            write_note,
            delete_note,
            get_mtime,
            read_meta,
            write_meta,
            delete_meta,
            read_app_config,
            write_app_config,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if !state.is_quitting.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            // A right-click "Quit" entry is the only way to exit the app
            // when the window is hidden, since Cmd+Q on a hidden window
            // doesn't reach the menu bar.
            let quit_item = MenuItem::with_id(app, "quit", "Quit notd", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id("notd-tray")
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "quit" {
                        let state = app.state::<AppState>();
                        state.is_quitting.store(true, Ordering::Relaxed);
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;
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
