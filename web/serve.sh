#!/usr/bin/env bash
# Build the WASM package into web/pkg and serve the site (with a tile proxy).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/burin-wasm"
wasm-pack build --target web --release --out-dir ../../web/pkg
rm -f ../../web/pkg/.gitignore
printf '' > ../../web/pkg/.gitignore
cd "$ROOT"
exec python3 web/serve.py
