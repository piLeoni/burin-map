#!/usr/bin/env bash
# Build WASM (single-threaded + threaded, see build-wasm.sh) and start the React app.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
"$ROOT/web/build-wasm.sh"
cd "$ROOT/web"
exec npm run dev
