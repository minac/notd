use crate::AppState;
use notify_debouncer_full::new_debouncer;
use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::DebounceEventResult;
use std::path::Path;
use std::time::Duration;
use tauri::{Emitter, State};

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
pub(crate) fn install_watcher(app: &tauri::AppHandle, state: &State<'_, AppState>, folder: &Path) {
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
