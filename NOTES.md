# Session notes — 27/28 September 2026

What changed, and why, between `976c08d` and `8f27d47`. Nine commits, all on
`main`, all pushed; this file was committed on top of them. The CI is green:
[run 36387792910](https://github.com/piLeoni/burin-map/actions/runs/36387792910).

This is a working note, not documentation. The user-facing explanation of the
engine lives in the README, under "GPU fields"; this file records the decisions
and the mistakes behind it.

## The shape of it

The session began with a working GPU implementation of `build_fields` that
exposed a CPU/GPU toggle to the user, and ended with no toggle at all: the GPU
is used when the machine has one, and `--force-cpu` is the only flag left.

> "Ma all'utente cosa interessa se usiamo la CPU, la GPU? Una calcolatrice?
> L'utente vuole le linee."

That is the whole design principle. The drawing is the product; the device that
made it is an implementation detail. A machine without a GPU is not a failure
case, it is a machine.

## Commits

| Commit | What |
| --- | --- |
| `788ed12` | GPU fields on wgpu/WebGPU (predates this session's decisions) |
| `f5c9fc5` | the persisted CPU/GPU choice was silently dropped |
| `a13c684` | auto device selection, everywhere |
| `c21f10a` | **remove the engine switch; keep only `--force-cpu`** |
| `46bbeb0` | **`force_cpu()` must fail loudly when it arrives too late** |
| `96076ea` | **CI: test every surface on push** |
| `828fef0` | pin the toolchain; stop comparing wasm bytes across machines |
| `e4ffbda` | fix the relative path in the wasm job |
| `8f27d47` | stop pretending a committed wasm can be diffed against a fresh build |

## The final surface

One request, "use the CPU", and a way to make it from every surface.

| Where | How to ask for the CPU |
| --- | --- |
| CLI | `--force-cpu` |
| Anywhere | `BURIN_ENGINE=cpu` |
| Browser | `?force-cpu` in the URL |
| Python | `force_cpu()` |
| Node | `forceCpu()` |

Removed for good: `--force-gpu` and its `--gpu` alias, `BURIN_ENGINE=gpu|auto`,
`force_gpu()`/`forceGpu()`, `useGpu()`, and the Auto/CPU buttons in the sidebar.
`burin_map_core::engine` collapsed from a three-state `Pick` enum to a bool:
forced-CPU or not, resolved once per process and cached either way.

`engine()` / `describe()` still report what is in use, which is useful in a bug
report even though it is not a choice:

```
GPU: Apple M4 (Metal)
CPU (forced)
CPU (no usable GPU)
```

### Why `--force-cpu` survives

The GPU works in f32, the CPU path in f64 for the blurs. The tracer amplifies
the last-bit differences, so every dash lands slightly differently, and
differently per GPU vendor. A golden-file test, a share link or a bug report
needs the reproducible one. That is the only reason the flag exists.

## Bug found: a silent no-op

```python
>>> burin_map.engine()
'GPU: Apple M4 (Metal)'
>>> burin_map.force_cpu()      # did nothing at all
>>> burin_map.engine()
'GPU: Apple M4 (Metal)'        # caller keeps the GPU drawing
```

`let _ = FORCE_CPU.set(true)` threw the request away once the device was
resolved, with no warning. The doc comment said "call before the first draw",
but a documented trap is still a trap. It now panics and names the fix.

Asking for the CPU when it is already the CPU stays a no-op — that is
repetition, not contradiction, and `BURIN_ENGINE=cpu` plus `force_cpu()` is a
reasonable thing to write.

This was found by **running** the bindings, not compiling them: Python and Node
both checked clean with the call silently useless.

## CI

Five jobs, one per way in; `rust` runs twice, with and without the `gpu`
feature, hence six rows. Every one was run by hand before being committed; a
CI that has never executed is a CI that does not work.

| Job | Covers |
| --- | --- |
| `rust (with-gpu)` | build, clippy, test, `--force-cpu` guard |
| `rust (no-gpu)` | the same without the `gpu` feature |
| `python` | maturin build, `engine()`, surface, pytest |
| `node` | napi build, surface, `node --test` |
| `wasm` | the crate builds; committed exports complete |
| `web` | oxlint, vite build |

Currently green: 40 Rust tests, 9 Python, 3 Node.

### The `--force-cpu` guard test

`crates/burin-map-core/tests/force_cpu_order.rs`. It took three attempts and the
first two are worth remembering: **both passed while the bug was still in
place.** A panicking `#[test]` has its output swallowed by the harness, so the
parent process could not tell "failed as intended" from "failed by accident".
The children now write their verdict to a file named by
`BURIN_FORCE_CPU_REPORT`.

Removing the guard makes the test fail, which is the only proof a regression
test is doing anything. Verify that whenever you touch it.

### What CI found that manual work did not

Four faults, none of which I would have caught by hand:

1. **clippy 1.95 ≠ 1.98.** Clean locally, failed on the runner, which knows
   `chunks_exact_to_as_chunks`. Six call sites now use `as_chunks`, checked to
   discard the tail exactly as `chunks_exact` did: two in `burin-map-py`, one
   each in the wasm crate, `raster.rs`, `dem.rs` and `examples/draw.rs`. The
   message of `828fef0` also claims "five more sites in `burin-map-py`"; the
   diff has none, so ignore that line.

2. **`maturin` from `python/` fails** — there is no `Cargo.toml` there.
   `pyproject.toml` in the root names the manifest and the source, so it has to
   run from the root.

3. **`maturin` ignores `.venv/bin/maturin`.** It decides where to install from
   `VIRTUAL_ENV`, so a bare path installs into the wrong interpreter and the
   import then fails with "No module named burin_map". The venv is activated per
   step now.

4. **A committed wasm cannot be diffed against a fresh build.** This one is the
   most instructive, because I got it wrong twice:
   - comparing the whole package: the `.wasm` embeds absolute registry paths
     (`/Users/pleoni/.cargo/...` against `/home/runner/...`), so it can never
     match;
   - comparing the `.js` glue instead, on the reasoning that it carries no
     paths. Wrong: the glue is wasm-bindgen symbol hashes, and those differ
     between toolchains — `13dab8c06ff4562a` locally, `339db9943090086e` on the
     runner.

   Checking a premise on one machine only proves it holds on that machine. The
   job now checks what a runner honestly can: the crate builds, the committed
   `.d.ts` still exports everything the app calls, and the size has not drifted
   wildly.

### `rust-toolchain.toml`

Added, pinned to 1.98.1 with `clippy`, `rustfmt` and `wasm32-unknown-unknown`.
Without it the CI and a developer sit on different stables and a new lint lands
in one and not the other. The workflow installs `stable` without a version,
but that does not matter: as soon as cargo runs inside the repo, rustup reads
this file and installs 1.98.1. A bump here is all it takes to move the CI.

## Not covered, and worth knowing

- **The real GPU path is untested.** GitHub runners have no GPU. The
  `with-gpu` job compiles wgpu but at runtime finds no adapter and falls back to
  the CPU — which does test the no-GPU machine's path, but nothing verifies that
  Metal/Vulkan produce correct output. Only your own machine does that.
- **The flag test is the only engine test.** `describe()` and the fallback
  logic beyond the ordering have no coverage.
- **Stale wasm packages rely on discipline.** Three are committed: `web/pkg`,
  its copy `web/public/pkg`, and the threaded `web/public/pkg-mt`. CI checks
  only the exports and size of `web/public/pkg`, and nothing at all of
  `pkg-mt`. Run `web/build-wasm.sh` before committing a change to
  `crates/burin-map-wasm`; `pkg-mt` needs the nightly named in the script, and
  without it the script skips `pkg-mt` and still exits 0.

## Afterwards: striped tracing, and joined dashes

Tracing was the bottleneck, about 60% of a draw, and ran on one core: every
line depends on the lines already placed. The striped tracer that sat in
`stash@{0}` is now on `main` (`8c36ae1`). It cuts the plane into 512 px
vertical stripes and traces them in parallel with the same rule.

- Tracing alone is roughly 4× faster (446 → 112 ms on the 3072×2304 test
  island). The whole draw gains much less: San Francisco from the CLI goes
  from 0.7 to 0.5 s, because loading, dashes, ordering and writing do not
  change. With several batches the gain shrinks too, since batches already
  run on separate cores.
- The drawing is equivalent, not identical: 5% more ink, the same density in
  the barest cells, and a column about 4 px wide left of each seam with
  10–20% less ink. Nobody could see a seam, in crops or in the app.
- Narrower stripes do not help much (384 px: 95 ms, 256 px: 99 ms): the
  stripe with the most terrain sets the time, and every stripe adds a seam.
  The GPU is the wrong tool: tracing is sequential decisions, not the same
  sum over many pixels.

Dashes that touch are now one stroke. In shadow a dash fills its whole period
and ends where the next begins, and in mid-tones it can overrun it, so a line
that looks solid used to be many paths, some drawn twice. `dash_lines` now
extends the open dash instead; only a real gap ends it. The random sequence is
unchanged, so the ink is where it was: San Francisco goes from 94,091 to
76,403 paths, and the rasters differ only in edge antialiasing where overlaps
used to double up (zero pixels differ at 25% tolerance).

`stripes-wip` and `stripes-tracing` are empty despite their names (they point
at commits already on `main`), and `stash@{0}` is now redundant.

## Verifying any of this

```bash
cargo test --workspace --features gpu          # 42 tests
cargo test --workspace --no-default-features   # the CPU-only build
cargo clippy --workspace --features gpu --all-targets -- -D warnings

cargo build --release -p burin-map             # the commands below use it
./target/release/burin-map --force-cpu --tp san-francisco --out-file sf.svg
BURIN_ENGINE=cpu ./target/release/burin-map --tp san-francisco --out-file sf.svg
./target/release/burin-map --gpu --tp x        # error: unexpected argument

gh run list -L1                                # the CI verdict
```
