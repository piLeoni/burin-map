#!/usr/bin/env bash
# Build WASM into web/pkg (+ public/pkg for Vite) and start the React app.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/burin-wasm"
wasm-pack build --target web --release --out-dir ../../web/pkg
rm -f ../../web/pkg/.gitignore
printf '' > ../../web/pkg/.gitignore
rm -rf ../../web/public/pkg
cp -R ../../web/pkg ../../web/public/pkg
cd "$ROOT/web"
exec npm run dev
