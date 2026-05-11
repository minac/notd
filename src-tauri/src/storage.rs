use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::State;

pub(crate) const META_FILENAME: &str = ".notd-meta.json";
pub(crate) const META_BAK_FILENAME: &str = ".notd-meta.json.bak";

#[derive(Serialize, Deserialize, Clone)]
pub struct MdFileInfo {
    filename: String,
    mtime_ms: i64,
}

pub(crate) fn ensure_md_filename(filename: &str) -> Result<(), String> {
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

pub(crate) fn mtime_ms(path: &Path) -> Result<i64, String> {
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
pub(crate) fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
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
pub fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
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
pub fn set_storage_folder(
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
    crate::watcher::install_watcher(&app, &state, &canonical);
    Ok(())
}

#[tauri::command]
pub fn list_md_files(state: State<'_, AppState>) -> Result<Vec<MdFileInfo>, String> {
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
pub fn read_note(state: State<'_, AppState>, filename: String) -> Result<String, String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_note(
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
pub fn delete_note(state: State<'_, AppState>, filename: String) -> Result<(), String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_mtime(state: State<'_, AppState>, filename: String) -> Result<i64, String> {
    ensure_md_filename(&filename)?;
    let canonical = current_folder(&state)?;
    let path = canonical.join(&filename);
    mtime_ms(&path)
}

#[tauri::command]
pub fn read_meta(state: State<'_, AppState>) -> Result<Option<String>, String> {
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
pub fn read_meta_bak(state: State<'_, AppState>) -> Result<Option<String>, String> {
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
pub fn write_meta(state: State<'_, AppState>, json: String) -> Result<(), String> {
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
pub fn delete_meta(state: State<'_, AppState>) -> Result<(), String> {
    let canonical = current_folder(&state)?;
    let path = canonical.join(META_FILENAME);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
