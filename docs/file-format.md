# File format

Everything is plain files in a folder. Sync is handled by your Dropbox client (or whatever else watches the folder); notd does no network I/O.

## Note files

- One `.md` file per note.
- File body is raw Markdown. **No YAML front matter, no metadata in the file.**
- Filename: `YYYY-MM-DD-HHmmss.md` (24-hour, zero-padded, local time).
- On filename collision (two notes within the same second), append `-2`, `-3`, etc. before `.md`.

Examples:

```
2026-05-06-143012.md
2026-05-06-143012-2.md
2026-05-06-153444.md
```

The user never sees these names. They exist purely so files are sortable and unique.

## Meta file

A hidden `.notd-meta.json` lives in the same folder. It owns the dot order and color assignment so they stay consistent across devices.

```json
{
  "version": 1,
  "notes": [
    { "filename": "2026-05-06-143012.md", "createdIndex": 0 },
    { "filename": "2026-05-06-153444.md", "createdIndex": 1 }
  ],
  "nextIndex": 2
}
```

### Rules

- `createdIndex` is monotonically increasing per note. **It is never reused, and never shifts** when other notes are deleted. Gaps in the color sequence are intentional.
- `nextIndex` is the index that will be assigned to the next created note.
- The dot color is `palette[createdIndex % 12]`, where the palette switches between light and dark mode.
- Dot order is `createdIndex` ascending — oldest leftmost, newest rightmost.

## Reconciliation

On load, the meta file is reconciled against what's actually on disk:

| Disk state                            | Action                                                            |
| ------------------------------------- | ----------------------------------------------------------------- |
| `.notd-meta.json` missing or invalid  | Rebuild from scratch. List `.md` files, sort by mtime, assign 0..N-1, set `nextIndex = N`. |
| `.md` file exists but not in meta     | Append to meta with the next available `createdIndex`.            |
| Meta references a missing `.md`       | Drop that entry from meta.                                        |

Reconciliation runs on app start and on every window focus event.

## Settings

The user's storage folder choice is stored in macOS app config dir (`~/Library/Application Support/eu.migueldavid.notd/config.json`):

```json
{
  "storageFolder": "/Users/you/Dropbox/Apps/notd",
  "activeFilename": "2026-05-06-143012.md"
}
```

`activeFilename` is what restores on relaunch.
