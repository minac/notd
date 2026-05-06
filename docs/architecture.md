# Architecture

A single-window macOS app. The frontend is Svelte 4 (SPA, prerendered) talking to a thin Rust backend over Tauri's IPC.

## Layout

```
+--------------------------------------------+
| dot row (40px)                             |
+--------------------------------------------+
| app bar (32px)  [Copy] [Clear] [Preview]   |
+--------------------------------------------+
| (banner, when present)                     |
+--------------------------------------------+
| editor textarea OR markdown preview        |
|                                            |
+--------------------------------------------+
```

No sidebar, no tabs, no panes.

## State

All UI state lives in Svelte stores (`src/lib/stores.ts`):

| Store              | Type                       | Purpose                                                     |
| ------------------ | -------------------------- | ----------------------------------------------------------- |
| `storageFolder`    | `string \| null`           | Absolute path to the notes folder on disk.                  |
| `meta`             | `Meta`                     | Parsed `.notd-meta.json`. Owns dot order and color indices. |
| `activeFilename`   | `string \| null`           | Which note is currently open. Persisted to app config.      |
| `activeBody`       | `string`                   | The textarea's current value.                               |
| `lastSavedBody`    | `string`                   | The body as it currently exists on disk.                    |
| `lastKnownMtime`   | `number`                   | mtime of the active note as we last saw it.                 |
| `theme`            | `'light' \| 'dark'`        | Reflects macOS appearance.                                  |
| `mode`             | `'edit' \| 'preview'`      | Edit/preview toggle.                                        |
| `banner`           | `BannerState \| null`      | Save-error or external-conflict banner.                     |
| `settingsOpen`     | `boolean`                  | Whether the settings modal is shown.                        |

`dirty` and `sortedNotes` are derived stores.

## Save flow

1. User types → `Editor.svelte` fires `onInput`.
2. Parent calls `scheduleSave()`, which debounces 500 ms.
3. On fire, `persistOrDelete(filename, value)` runs:
   - **If `value === ''` and `lastSavedBody !== ''`** → call `handleDelete(filename)`. The note's file and its meta entry are removed; the dot disappears.
   - Else → `writeNote()`, then refresh `lastSavedBody` and `lastKnownMtime`.
4. On save error, a non-modal banner appears with a Retry button.

`flushPendingSave()` runs the same path eagerly when switching notes so we never lose pending edits.

## External-change handling

On every window focus event:

1. Re-scan the folder via `loadMeta()` (handles new files, deleted files, missing meta).
2. For the active note, compare disk `mtime` to `lastKnownMtime`:
   - mtime advanced + body is dirty → conflict banner with **Keep mine** / **Use theirs**.
   - mtime advanced + body is clean → silently reload contents.

## Backend (`src-tauri/src/lib.rs`)

A small set of `#[tauri::command]` functions, each enforcing that filenames end in `.md` and contain no path separators or `..`:

- `get_default_storage_folder` → `~/Dropbox/Apps/notd`
- `path_exists`, `create_dir`
- `list_md_files`, `read_note`, `write_note`, `delete_note`, `get_mtime`
- `read_meta`, `write_meta`, `delete_meta`
- `read_app_config`, `write_app_config` (stores `{ storageFolder, activeFilename }` under `app_config_dir`)

The frontend never touches the file system directly; all I/O goes through these.

## Plugins

| Plugin                | Why                                                  |
| --------------------- | ---------------------------------------------------- |
| `tauri-plugin-fs`     | Required by other plugins; not used directly.        |
| `tauri-plugin-dialog` | Folder picker (Setup / Settings) and confirm prompts. |
| `tauri-plugin-shell`  | Opening external links from the Markdown preview.    |
| `tauri-plugin-window-state` | Persists window size and position across launches. |
