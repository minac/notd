use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, RunEvent, State, Theme, WindowEvent};

const TRAY_ICON_LIGHT: &[u8] = include_bytes!("../icons/tray-light.png");
const TRAY_ICON_DARK: &[u8] = include_bytes!("../icons/tray-dark.png");

const META_FILENAME: &str = ".notd-meta.json";
const META_BAK_FILENAME: &str = ".notd-meta.json.bak";

fn tray_icon_for(theme: Theme) -> Result<Image<'static>, tauri::Error> {
    let bytes = if matches!(theme, Theme::Dark) {
        TRAY_ICON_DARK
    } else {
        TRAY_ICON_LIGHT
    };
    Image::from_bytes(bytes)
}

fn apply_tray_theme(tray: &TrayIcon, theme: Theme) {
    if let Ok(icon) = tray_icon_for(theme) {
        let _ = tray.set_icon(Some(icon));
    }
}

struct AppState {
    is_quitting: AtomicBool,
    // Canonical, validated storage folder. Set by `set_storage_folder` once
    // the frontend has resolved (and created, if needed) the user's chosen
    // folder. All storage commands read this — the renderer is no longer
    // trusted to pass the folder on every call.
    storage_folder: Mutex<Option<PathBuf>>,
    // Debounced FS watcher for the current storage folder. Replaced when the
    // user changes the folder via Settings. The `Drop` impl on `Debouncer`
    // stops the underlying watcher thread, so simply overwriting this field
    // tears down the previous watcher.
    watcher: Mutex<Option<Debouncer<RecommendedWatcher, RecommendedCache>>>,
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

// Write to a hidden sibling `.<name>.tmp` file, then rename into place, then
// fsync both the file and its parent directory. Guards against:
//   - Truncated reads when Dropbox (or another syncer) observes the file
//     mid-write — a torn `.notd-meta.json` is especially nasty because it
//     triggers a full meta rebuild from index 0, breaking the monotonic
//     `createdIndex` invariant.
//   - A power loss between `rename` and the FS journal flush: `rename` is
//     atomic across processes but not across crashes. fsyncing the file and
//     the directory makes the rename durable.
//
// The tmp file is dot-prefixed so it doesn't show up in `read_dir` listings,
// Dropbox indexers, or shell globs while a write is in flight.
fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent".to_string())?;
    let filename = path
        .file_name()
        .ok_or("path has no file name".to_string())?
        .to_string_lossy();
    let tmp = parent.join(format!(".{}.tmp", filename));
    fs::write(&tmp, contents).map_err(|e| format!("write tmp: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename: {e}"))?;

    // fsync the file and its directory so the rename survives a crash.
    let f = fs::File::open(path).map_err(|e| format!("open: {e}"))?;
    f.sync_all().map_err(|e| format!("fsync file: {e}"))?;
    let d = fs::File::open(parent).map_err(|e| format!("open parent: {e}"))?;
    d.sync_all().map_err(|e| format!("fsync parent: {e}"))?;
    Ok(())
}

// Validates a folder path supplied by the renderer. The ONLY caller is
// `set_storage_folder` — every other storage command reads the canonical
// path from `AppState.storage_folder` instead.
//
// NOTE: this calls `fs::canonicalize`, which fails if the path doesn't
// exist. The frontend creates the folder (via `create_dir`) before calling
// `set_storage_folder`, so this is fine.
fn validate_folder(folder: &str) -> Result<PathBuf, String> {
    let p = Path::new(folder);
    if !p.is_absolute() {
        return Err("storage folder must be an absolute path".into());
    }
    let canonical = fs::canonicalize(p).map_err(|e| format!("canonicalize: {e}"))?;
    if !canonical.is_dir() {
        return Err("storage folder is not a directory".into());
    }
    Ok(canonical)
}

// Returns the currently configured (canonical) storage folder, or an error
// if `set_storage_folder` hasn't been called yet. Callers join filenames
// onto the returned PathBuf.
fn current_folder(state: &State<AppState>) -> Result<PathBuf, String> {
    let guard = state
        .storage_folder
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    guard
        .clone()
        .ok_or_else(|| "storage folder not set".to_string())
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

// One-time-per-session (or per-folder-change) handshake: the frontend
// resolves and creates the user's chosen folder, then calls this. Every
// subsequent storage command reads from AppState — the renderer never
// passes a folder again.
//
// Idempotent: calling this twice with the same folder is a no-op; calling
// it with a different folder simply swaps the canonical path. The lock is
// released before any IO so we don't hold it across an `await` on the JS
// side.
//
// As a side effect, this (re)installs a debounced filesystem watcher
// rooted at the new folder. The watcher emits a `fs-changed` Tauri event
// whenever a non-hidden `.md` file changes, so the frontend can refresh
// in near-real-time instead of waiting for a window-focus event.
// Watcher creation failure is intentionally non-fatal: the focus-based
// refresh still works, and we'd rather not refuse the folder switch over
// a missing watcher.
#[tauri::command]
fn set_storage_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    folder: String,
) -> Result<(), String> {
    let canonical = validate_folder(&folder)?;
    {
        let mut guard = state
            .storage_folder
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        *guard = Some(canonical.clone());
    }
    install_watcher(&app, &state, &canonical);
    Ok(())
}

// Build a fresh debounced watcher for `folder` and stash it in AppState,
// dropping any previous watcher. Best-effort: errors are logged and
// swallowed.
//
// Filtering happens callback-side: notify-debouncer-full's API doesn't
// expose a path predicate, so we examine each batch and only emit the
// `fs-changed` event if at least one event touches a non-hidden `.md`
// file. This naturally ignores our own `.<name>.tmp`, `.notd-meta.json`,
// `.notd-meta.json.bak`, and macOS `.DS_Store` traffic — exactly the
// noise that would otherwise feedback-loop into refreshFromDisk.
fn install_watcher(app: &tauri::AppHandle, state: &State<'_, AppState>, folder: &Path) {
    let app_for_cb = app.clone();
    let result = new_debouncer(
        Duration::from_millis(500),
        None,
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                let touches_md = events.iter().any(|ev| {
                    ev.event.paths.iter().any(|p| {
                        let is_md = p.extension().and_then(|s| s.to_str()) == Some("md");
                        let is_hidden = p
                            .file_name()
                            .and_then(|s| s.to_str())
                            .map(|n| n.starts_with('.'))
                            .unwrap_or(true);
                        is_md && !is_hidden
                    })
                });
                if touches_md {
                    let _ = app_for_cb.emit("fs-changed", ());
                }
            }
            Err(errors) => {
                for e in errors {
                    eprintln!("fs watcher error: {e:?}");
                }
            }
        },
    );

    let mut debouncer = match result {
        Ok(d) => d,
        Err(e) => {
            eprintln!("fs watcher: failed to create debouncer: {e:?}");
            // Still clear any previous watcher so we don't keep stale
            // notifications flowing from the old folder.
            if let Ok(mut guard) = state.watcher.lock() {
                *guard = None;
            }
            return;
        }
    };

    // The storage folder is flat — no `.md` files live in subdirectories.
    if let Err(e) = debouncer.watch(folder, RecursiveMode::NonRecursive) {
        eprintln!("fs watcher: failed to watch {folder:?}: {e:?}");
        if let Ok(mut guard) = state.watcher.lock() {
            *guard = None;
        }
        return;
    }

    if let Ok(mut guard) = state.watcher.lock() {
        // Assigning here drops the previous debouncer, which stops its
        // event thread. Order matters: we replace _after_ the new one is
        // wired up so there's no observable gap.
        *guard = Some(debouncer);
    }
}

#[tauri::command]
fn list_md_files(state: State<'_, AppState>) -> Result<Vec<MdFileInfo>, String> {
    let canonical = current_folder(&state)?;
    // Defensive: the folder was canonicalized at set-time, but it could
    // have been deleted out from under us since then (Dropbox unmount,
    // user moved it in Finder). Treat that as an empty listing rather
    // than a hard error — the frontend already handles this in the
    // refresh-on-focus path.
    if !canonical.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&canonical).map_err(|e| e.to_string())?;
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
fn read_note(state: State<'_, AppState>, filename: String) -> Result<String, String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_note(
    state: State<'_, AppState>,
    filename: String,
    contents: String,
) -> Result<(), String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    atomic_write(&path, contents.as_bytes())
}

#[tauri::command]
fn delete_note(state: State<'_, AppState>, filename: String) -> Result<(), String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_mtime(state: State<'_, AppState>, filename: String) -> Result<i64, String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    mtime_ms(&path)
}

#[tauri::command]
fn read_meta(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let canonical = current_folder(&state)?;
    let path = canonical.join(META_FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| e.to_string())
}

// Returns the contents of `.notd-meta.json.bak` if it exists. The frontend
// calls this when the primary meta is missing or invalid, before falling
// through to a full rebuild — the bak survives a torn write of the
// primary, preserving the monotonic `createdIndex` invariant.
#[tauri::command]
fn read_meta_bak(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let canonical = current_folder(&state)?;
    let path = canonical.join(META_BAK_FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn write_meta(state: State<'_, AppState>, json: String) -> Result<(), String> {
    let canonical = current_folder(&state)?;
    let path = canonical.join(META_FILENAME);

    // Snapshot the existing meta to `.notd-meta.json.bak` before
    // overwriting. Best-effort: if the copy fails we still proceed with
    // the write — the bak is a safety net, not a hard requirement, and
    // failing the write because the safety net wobbled would be worse
    // than no safety net at all.
    if path.exists() {
        let bak = canonical.join(META_BAK_FILENAME);
        let _ = fs::copy(&path, &bak);
    }

    atomic_write(&path, json.as_bytes())
}

#[tauri::command]
fn delete_meta(state: State<'_, AppState>) -> Result<(), String> {
    let canonical = current_folder(&state)?;
    let path = canonical.join(META_FILENAME);
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
    atomic_write(&path, json.as_bytes())
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
            get_default_storage_folder,
            path_exists,
            create_dir,
            set_storage_folder,
            list_md_files,
            read_note,
            write_note,
            delete_note,
            get_mtime,
            read_meta,
            read_meta_bak,
            write_meta,
            delete_meta,
            read_app_config,
            write_app_config,
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
                if let Some(tray) = app.tray_by_id("notd-tray") {
                    apply_tray_theme(&tray, *theme);
                }
            }
            _ => {}
        })
        .setup(|app| {
            // A right-click "Quit" entry is the only way to exit the app
            // when the window is hidden, since Cmd+Q on a hidden window
            // doesn't reach the menu bar.
            let quit_item = MenuItem::with_id(app, "quit", "Quit notd", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            let initial_theme = app
                .get_webview_window("main")
                .and_then(|w| w.theme().ok())
                .unwrap_or(Theme::Light);

            let tray = TrayIconBuilder::with_id("notd-tray")
                .icon(tray_icon_for(initial_theme)?)
                .icon_as_template(false)
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

            // Theme swap is wired via WindowEvent::ThemeChanged on the
            // builder-level handler above, which finds the tray by id.
            let _ = tray;

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
