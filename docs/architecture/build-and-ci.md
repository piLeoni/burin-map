# Build and CI

## Rust toolchain

`rust-toolchain.toml` pins stable (e.g. 1.98.1) with `clippy`, `rustfmt`, `wasm32-unknown-unknown`. CI uses `dtolnay/rust-toolchain@stable`; cargo in-repo reads the pin.

## Workspace builds

```bash
cargo build -p burin-map          # CLI
cargo test --workspace --features gpu
cargo test --workspace --no-default-features   # no GPU crate path
```

Default members: `burin-map-core`, `burin-map` only; py/wasm/node built by their toolchains.

## WASM for web (`web/build-wasm.sh`)

| Output | Crate | Notes |
|--------|-------|--------|
| `web/public/pkg` | `burin-map-wasm` | stable, `--features gpu` |
| `web/public/pkg-mt` | `burin-map-wasm` | `threads` + nightly `build-std`; skipped if nightly missing |
| `web/public/pkg-script` | `burin-map-script-wasm` | profile `wasm-script` |

Committed artifacts: CI does **not** require byte-identical wasm rebuilds; wasm job checks build + export surface + size sanity for `pkg`.

After changing `burin-map-wasm` or `burin-map-script-wasm`, run `web/build-wasm.sh` before commit.

## GitHub Actions (`.github/workflows/ci.yml`)

| Job | Covers |
|-----|--------|
| `rust` (matrix) | fmt, clippy, test with/without `gpu`; `--force-cpu` smoke + `force_cpu_order` test |
| `python` | maturin from root, pytest |
| `node` | napi build, node tests |
| `wasm` | wasm crate builds + committed pkg checks |
| `web` | oxlint, vite build |

Concurrency cancels in-progress runs per ref.

## Local web dev

`web/serve.sh` / `yarn dev` (Vite). Python `web/serve.py` optional for COOP headers if testing `pkg-mt`.

## Related

- Engine GPU notes: [core-engine.md](core-engine.md), [decisions.md](decisions.md)
- Operational detail: [NOTES.md](../../NOTES.md) (CI pitfalls, maturin paths)
