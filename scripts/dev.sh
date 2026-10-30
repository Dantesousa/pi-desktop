#!/usr/bin/env bash
# Dev launcher that removes leftovers from a previous (crashed) session,
# which otherwise leave an orphan vite holding port 1420.
set -euo pipefail
cd "$(dirname "$0")/.."
PORT="${PORT:-1420}"

if ss -ltn 2>/dev/null | grep -q ":$PORT "; then
  echo "porta $PORT ocupada — limpando processo órfão…"
  if command -v fuser >/dev/null; then
    fuser -k "${PORT}/tcp" 2>/dev/null || true
  else
    pkill -f "vite dev" 2>/dev/null || true
  fi
  sleep 1
fi

pkill -f "target/debug/pi-desktop" 2>/dev/null || true
sleep 0.3
exec npm run tauri dev -- --config src-tauri/tauri.dev.conf.json
