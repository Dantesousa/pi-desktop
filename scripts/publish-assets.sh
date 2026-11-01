#!/usr/bin/env bash
# Publishes the release assets with a safe naming scheme:
#   Hina_<v>_amd64.AppImage            -> the self-contained build (runs anywhere)
#   Hina_<v>_amd64_selfcontained.AppImage -> same file, kept for updaters that
#                                            look for the old suffix
#   Hina_<v>_amd64_ubuntu.AppImage     -> the CI build (Ubuntu/Debian only)
#   Hina_<v>_amd64.deb                 -> Debian package (from CI)
#
# An updater that simply picks "the AppImage from the release" then gets a build
# that starts on any distro instead of the Ubuntu-only one.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:?usage: scripts/publish-assets.sh 0.1.10}"
TAG="v$VERSION"
REPO="${REPO:-Dantesousa/pi-desktop}"
BUNDLE="src-tauri/target/release/bundle/appimage"
BASE="Hina_${VERSION}_amd64"

[ -f "$BUNDLE/$BASE.AppImage" ] || {
  echo "missing $BUNDLE/$BASE.AppImage — run 'npm run dist' first" >&2
  exit 1
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$BUNDLE/$BASE.AppImage" "$TMP/$BASE.AppImage"                    # local, self-contained
mkdir -p "$TMP/ci"
gh release download "$TAG" -R "$REPO" -p "$BASE.AppImage" -D "$TMP/ci" --clobber
cp "$TMP/ci/$BASE.AppImage" "$TMP/${BASE}_ubuntu.AppImage"           # CI build, renamed
cp "$TMP/$BASE.AppImage" "$TMP/${BASE}_selfcontained.AppImage"       # compat copy

gh release upload "$TAG" \
  "$TMP/$BASE.AppImage" \
  "$TMP/${BASE}_ubuntu.AppImage" \
  "$TMP/${BASE}_selfcontained.AppImage" \
  -R "$REPO" --clobber

echo "published:"
gh release view "$TAG" -R "$REPO" --json assets \
  --jq '.assets[] | "  • " + .name + " — " + ((.size/1048576*10|floor)/10|tostring) + " MB"'
