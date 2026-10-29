#!/usr/bin/env bash
# Registers a desktop entry + icon so Wayland/KDE shows the app icon in the
# taskbar and launcher (dev builds have no installed .desktop otherwise).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
APP_DIR="$HOME/.local/share/applications"
BIN="$ROOT/src-tauri/target/release/pi-desktop"
[ -x "$BIN" ] || BIN="$ROOT/src-tauri/target/debug/pi-desktop"

mkdir -p "$ICON_DIR" "$APP_DIR"
cp "$ROOT/src-tauri/icons/128x128@2x.png" "$ICON_DIR/hina.png"

write_entry() {
  local id="$1"
  cat > "$APP_DIR/$id.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Hina
Comment=Desktop client for the Pi coding agent
Exec=$BIN
Icon=$ICON_DIR/hina.png
Terminal=false
Categories=Development;
StartupWMClass=$id
Keywords=pi;agent;ai;coding;
DESKTOP
  chmod +x "$APP_DIR/$id.desktop"
}

write_entry "pi-desktop"
write_entry "Hina"
write_entry "com.dante.hina"

command -v update-desktop-database >/dev/null && update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
echo "ícone registrado para: pi-desktop, Hina, com.dante.hina"
