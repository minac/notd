# notd

A boring, fast, native-feeling macOS app for capturing short text drafts. Each draft is one `.md` file in a folder you choose (default `~/Dropbox/Apps/notd/`). Notes are represented by a row of colored dots at the top of the window — filenames are never shown. Sync is whatever your Dropbox client does; the app itself does no network I/O.

notd lives in the menu bar. Closing the window hides it; clicking the tray icon brings it back. Right-click the tray icon → **Quit notd** (or `⌘Q`) to actually exit.

## Local development

Prerequisites:
- Node 20+
- Rust (stable, 1.77+) — `brew install rust` or `rustup`
- Xcode Command Line Tools — `xcode-select --install`

Install and run:

```sh
npm install
npm run tauri dev
```

## Build

```sh
npm run tauri build
```

This produces a `.dmg` under `src-tauri/target/release/bundle/dmg/`.

## File format

Notes are plain Markdown files in your chosen folder, one per note, named `YYYY-MM-DD-HHmmss.md`. A hidden `.notd-meta.json` in the same folder tracks dot order and color assignment so they stay consistent across devices when synced via Dropbox.

If `.notd-meta.json` is missing or invalid, notd rebuilds it from the file modification times the next time it loads the folder.

## License

MIT
