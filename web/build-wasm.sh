#!/usr/bin/env bash
# Build the two WASM packages the web app serves from public/:
#   pkg/     single-threaded, stable Rust; works on any page
#   pkg-mt/  rayon threads (wasm-bindgen-rayon); needs a cross-origin isolated
#            page (COOP/COEP headers) and a nightly toolchain for build-std
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NIGHTLY="${BURIN_NIGHTLY:-nightly-2026-04-14}"
cd "$ROOT/crates/burin-map-wasm"

wasm-pack build --target web --release --out-dir ../../web/pkg
rm -rf ../../web/public/pkg
cp -R ../../web/pkg ../../web/public/pkg
printf '' > ../../web/public/pkg/.gitignore

if ! rustup run "$NIGHTLY" rustc --version >/dev/null 2>&1; then
  echo "skipping pkg-mt: install the toolchain with" >&2
  echo "  rustup toolchain install $NIGHTLY --profile minimal --component rust-src --target wasm32-unknown-unknown" >&2
  exit 0
fi
# Own target dir so the std rebuilt with atomics doesn't evict the stable build.
CARGO_TARGET_DIR="$ROOT/target/wasm-mt" \
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory \
  -C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--max-memory=4294967296 \
  -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size \
  -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base" \
  rustup run "$NIGHTLY" wasm-pack build --target web --release --out-dir ../../web/public/pkg-mt \
  -- --features threads -Z build-std=panic_abort,std
printf '' > ../../web/public/pkg-mt/.gitignore
