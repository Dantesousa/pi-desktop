#!/usr/bin/env bash
# Bump the app version everywhere it is declared.
set -euo pipefail
cd "$(dirname "$0")/.."
V="${1:?uso: scripts/bump.sh 0.1.1}"

python3 - "$V" <<'PY'
import json, re, sys, pathlib
v = sys.argv[1]
pkg = pathlib.Path("package.json")
d = json.loads(pkg.read_text())
d["version"] = v
pkg.write_text(json.dumps(d, indent=2) + "\n")

cargo = pathlib.Path("src-tauri/Cargo.toml")
cargo.write_text(re.sub(r'^version = ".*"$', f'version = "{v}"', cargo.read_text(), count=1, flags=re.M))

conf = pathlib.Path("src-tauri/tauri.conf.json")
c = json.loads(conf.read_text())
c["version"] = v
conf.write_text(json.dumps(c, indent=2) + "\n")
print(f"versao -> {v}")
PY

# refresh Cargo.lock so `cargo check --locked` (CI) keeps working
(cd src-tauri && cargo check -q) || true
echo "ok"
