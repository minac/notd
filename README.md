# notd

A boring, fast, native-feeling macOS app for capturing short text drafts. Each draft is one `.md` file in a folder you choose (default `~/Dropbox/Apps/notd/`). Notes are represented by a row of colored dots at the top of the window — filenames are never shown. Sync is whatever your Dropbox client does; the app itself does no network I/O.

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

A small Rust CLI lives in `cli/` for scripting and Claude Code workflows. Run via the `./notd` wrapper at the repo root — it builds on first use and execs the binary directly afterwards.

```sh
./notd help              # commands
./notd ls                # list notes, oldest → newest
./notd show last         # print the most recent note
./notd new "quick idea"  # create a note (or pipe stdin)
./notd append 7 "more"   # append to note #7
./notd grep "pattern"    # ripgrep across notes
```

The CLI reads the same `config.json` the app writes, so it operates on whichever folder you've chosen in the app.

## License

MIT
