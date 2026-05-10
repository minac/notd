#!/usr/bin/env bash
# notd dev/build helper. One subcommand at a time.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

APP_NAME="notd"
APP_BUNDLE="src-tauri/target/release/bundle/macos/${APP_NAME}.app"
APPLICATIONS="/Applications"

usage() {
  cat <<EOF
Usage: ./run.sh <command>

Commands:
  dev      Start the Tauri dev server (hot reload).
  build    Production build, then copy ${APP_NAME}.app into ${APPLICATIONS}.
  cli      Build and install notd-cli to \$CLI_PREFIX/bin (default ~/.local).
  doctor   Run svelte-check and cargo check.
  audit    Run cargo audit (src-tauri, cli) and npm audit for known CVEs.
  clean    Remove build artifacts (build/, .svelte-kit/, src-tauri/target, cli/target).
  help     Show this message.
EOF
}

cmd_dev() {
  npm run tauri dev
}

cmd_build() {
  npm run tauri build

  if [[ ! -d "$APP_BUNDLE" ]]; then
    echo "error: expected ${APP_BUNDLE} to exist after build" >&2
    exit 1
  fi

  local target="${APPLICATIONS}/${APP_NAME}.app"

  if [[ -d "$target" ]]; then
    echo "Removing previous ${target}"
    rm -rf "$target"
  fi

  echo "Copying ${APP_BUNDLE} → ${target}"
  cp -R "$APP_BUNDLE" "$target"

  # Strip the macOS quarantine attribute so first launch doesn't get blocked.
  xattr -dr com.apple.quarantine "$target" 2>/dev/null || true

  echo
  echo "Installed at ${target}"
  ls -1 src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null || true
}

cmd_doctor() {
  npx svelte-check --tsconfig ./tsconfig.json
  ( cd src-tauri && cargo check )
  ( cd cli && cargo check )
}

cmd_cli() {
  local prefix="${CLI_PREFIX:-$HOME/.local}"
  cargo install --path cli --root "$prefix" --force --quiet
  local bin="$prefix/bin/notd-cli"
  echo
  echo "Installed ${bin}"
  case ":$PATH:" in
    *":$prefix/bin:"*) ;;
    *) echo "note: $prefix/bin is not on \$PATH — add it to your shell rc to use \`notd-cli\` directly." ;;
  esac
}

cmd_audit() {
  echo "→ cargo audit (src-tauri)"
  ( cd src-tauri && cargo audit )
  echo
  echo "→ cargo audit (cli)"
  ( cd cli && cargo audit )
  echo
  echo "→ npm audit (frontend)"
  npm audit --omit=dev || true
}

cmd_clean() {
  rm -rf build .svelte-kit
  ( cd src-tauri && cargo clean )
  ( cd cli && cargo clean )
  echo "Cleaned."
}

main() {
  local sub="${1:-help}"
  shift || true
  case "$sub" in
    dev)    cmd_dev "$@" ;;
    build)  cmd_build "$@" ;;
    cli)    cmd_cli "$@" ;;
    doctor) cmd_doctor "$@" ;;
    audit)  cmd_audit "$@" ;;
    clean)  cmd_clean "$@" ;;
    help|-h|--help) usage ;;
    *)      echo "Unknown command: $sub" >&2; usage; exit 1 ;;
  esac
}

main "$@"
