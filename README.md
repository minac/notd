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

## CLI

`notd-cli` is a small Rust companion CLI in `cli/` for scripting and Claude Code workflows. The binary is named `notd-cli` to keep it distinct from the GUI app. Run it via the `./notd-cli` wrapper at the repo root (rebuilds on source change, otherwise execs the binary directly), or install it system-wide with `./run.sh cli` (puts it at `~/.local/bin/notd-cli`).

```sh
./notd-cli help              # commands and positioning
./notd-cli ls                # list notes, oldest → newest
./notd-cli show last         # print the most recent note
./notd-cli new "quick idea"  # create a note (or pipe stdin)
./notd-cli append 7 "more"   # append to note #7
./notd-cli grep "pattern"    # case-insensitive search; hits show note index
./notd-cli rm 7              # delete note #7
```

The CLI reads the same `config.json` the app writes, so it operates on whichever folder you've chosen in the app.

## License

MIT
