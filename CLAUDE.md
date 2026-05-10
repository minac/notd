# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A single-window macOS Tauri 2 app for capturing short Markdown drafts. Frontend: SvelteKit 2 + Svelte 5 (SPA, prerendered, SSR off). Backend: ~300 lines of Rust exposing `#[tauri::command]` file I/O. No tests — verify changes by running the app.

## Commands

Use `./run.sh` rather than calling npm/cargo directly:

| Command           | What it does                                                         |
| ----------------- | -------------------------------------------------------------------- |
| `./run.sh dev`    | Tauri dev (Vite on `:1420`, Rust hot rebuild). Loads the app window. |
| `./run.sh build`  | Production build → installs `notd.app` to `/Applications/`, strips quarantine. Produces a `.dmg` under `src-tauri/target/release/bundle/dmg/`. |
| `./run.sh doctor` | `svelte-check` + `cargo check`. The closest thing to CI.             |
| `./run.sh clean`  | Remove `build/`, `.svelte-kit/`, and `cargo clean`.                  |

Frontend-only checks: `npm run check`. Vite needs port 1420 free (`strictPort: true`).

## Architecture

### Frontend ↔ Rust split

The frontend never touches the filesystem. Every read/write/list goes through a `#[tauri::command]` in `src-tauri/src/lib.rs`, wrapped on the JS side by `src/lib/fs.ts` (notes), `src/lib/meta.ts` (`.notd-meta.json`), or inline `invoke()` calls. **Adding a new command requires two steps**: define `#[tauri::command] fn` and add it to the `tauri::generate_handler![...]` list in `run()`. If it touches a new capability (dialog, shell, etc.), also update `src-tauri/capabilities/default.json`.

All filename-accepting commands run through `ensure_md_filename()` — rejects empty strings, `/`, `\`, `..`, and anything not ending in `.md`. Mirror this whenever you add a filename-aware command; the frontend trusts it.

### Single source of truth for "what notes exist and what color"

The on-disk folder is the data; `.notd-meta.json` is the index. Crucial invariant: **`createdIndex` is monotonically increasing and never reused, even when notes are deleted**. Dot color is `palette[createdIndex % 12]`, so deleting a note leaves a "gap" in the color rotation — that's intentional. `nextIndex` only ever grows.

`loadMeta()` (in `src/lib/meta.ts`) reconciles meta against disk on every load:
- Missing/invalid meta → `rebuildMeta()` (sort `.md` files by mtime, assign 0..N-1).
- Files on disk not in meta → appended with new `createdIndex` values.
- Meta entries with no file → dropped.

Only persists if anything actually changed (avoids needless Dropbox churn).

### Save / conflict flow (`src/routes/+page.svelte`)

1. `Editor.svelte` typing → `scheduleSave()` debounces 500ms.
2. `persistOrDelete(filename, value)`:
   - `value === ''` && `lastSavedBody !== ''` → **delete the note** (and its meta entry, and pick a neighbor as active). Empty body == no note.
   - Otherwise → `writeNote()` and refresh `lastSavedBody` / `lastKnownMtime`.
3. `flushPendingSave()` runs eagerly when switching notes or changing folder so a queued save can't land on the wrong file (`savingFilename` tracks this).
4. Save errors render a non-modal `Banner` with Retry; they don't block input.

External edits are detected on **window focus only** — there is no FS watcher. On focus, `refreshFromDisk()` rescans the folder and compares disk mtime to `lastKnownMtime` for the active note. If mtime advanced and the body is dirty → conflict banner with Keep mine / Use theirs. If clean → silent reload.

### Window / tray lifecycle

- Closing the window does **not** quit (`WindowEvent::CloseRequested` calls `api.prevent_close()` and hides). The only exits are: tray right-click → "Quit notd", `Cmd+Q` while focused, or `RunEvent::ExitRequested`. The `AppState.is_quitting` flag gates this — flip it before `app.exit(0)`.
- Tray icon: two PNGs baked into the binary (`include_bytes!` on `tray-{light,dark}.png`). `WindowEvent::ThemeChanged` swaps them at runtime via `apply_tray_theme()`. The tray must be looked up by its `id` (`"notd-tray"`) — it's not in scope from the event handler.
- Left-click tray → `show_main_window()` (also called on macOS `RunEvent::Reopen` for dock-icon clicks).

### Generated assets

Tray icons are generated, not hand-edited. Regenerate after touching `src-tauri/icons/icon.icns`:

```sh
python3 scripts/generate-tray-icon.py   # needs `magick` (imagemagick) + iconutil
python3 scripts/generate-icons.py       # placeholder app icons (rarely needed)
```

## Conventions specific to this repo

- **No tests.** `./run.sh doctor` is the only static check; manual app testing is the verification step. Don't fabricate a test framework — if behavior needs verification, run the app.
- **No global hotkey, no menubar, no shortcut config.** By design — see `docs/keyboard.md`. Don't add one without explicit ask.
- **Default storage folder is `~/Dropbox/Apps/notd`** (`get_default_storage_folder` in Rust). The user can move it via Settings; don't hardcode the Dropbox path elsewhere.
- **App config lives at `~/Library/Application Support/eu.migueldavid.notd/config.json`** (storageFolder + activeFilename). Bundle identifier `eu.migueldavid.notd` is referenced in CSP and config paths — keep it in sync if it ever changes.
- **Filenames are never shown in the UI.** The dot row is the entire navigation surface. Don't surface `2026-05-06-143012.md` to the user.

## Project docs

- `docs/architecture.md` — same overview, rendered for the GitHub Pages site.
- `docs/file-format.md` — authoritative spec for `.md` naming and `.notd-meta.json` structure. Update this if you change either.
- `docs/keyboard.md` — keyboard shortcut surface. Keep in sync with `src/lib/shortcuts.ts`.

The `docs/` folder is the GitHub Pages site (root + `.nojekyll`); it is not bundled into the app.
