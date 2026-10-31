#!/usr/bin/env bash
# Dev launcher that removes leftovers from a previous (crashed) session,
# which otherwise leave an orphan vite holding port 1420.
set -euo pipefail
cd "$(dirname "$0")/.."
PORT="${PORT:-1420}"

# If a packaged build (AppImage) was launched from this shell, its AppRun exports
# GTK_*/LD_LIBRARY_PATH/APPDIR pointing at the mount — the dev app then looks for
# WebKit helpers inside that mount and dies. Drop that pollution.
unset APPDIR GTK_DATA_PREFIX GTK_EXE_PREFIX GTK_PATH GTK_IM_MODULE_FILE \
      GDK_PIXBUF_MODULE_FILE WEBKIT_EXEC_PATH WEBKIT_INJECTED_BUNDLE_PATH \
      PYTHONHOME PYTHONPATH 2>/dev/null || true
if [ -n "${LD_LIBRARY_PATH:-}" ]; then
  LD_LIBRARY_PATH="$(printf '%s' "$LD_LIBRARY_PATH" | tr ':' '\n' | { grep -v '/tmp/.mount_' || true; } | paste -sd: -)"
  export LD_LIBRARY_PATH
fi

if ss -ltn 2>/dev/null | grep -q ":$PORT "; then
  echo "port $PORT is busy — clearing the orphan process…"
  if command -v fuser >/dev/null; then
    fuser -k "${PORT}/tcp" 2>/dev/null || true
  else
    pkill -f "vite dev" 2>/dev/null || true
  fi
  sleep 1
fi

pkill -f "target/debug/pi-desktop" 2>/dev/null || true
sleep 0.3
exec node_modules/.bin/tauri dev --config src-tauri/tauri.dev.conf.json
