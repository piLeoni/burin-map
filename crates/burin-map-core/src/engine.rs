//! Which device builds the terrain fields.
//!
//! The choice is ours, not the caller's: the drawing is the product, and the
//! fastest device that can do it is simply the right one. So the default is
//! `Auto` — use the GPU when the machine has one, fall back to the CPU when it
//! does not.
//!
//! Two escapes exist, for people who need them (tests that compare byte for
//! byte, share links, a driver that misbehaves):
//!
//! - [`force_cpu`] — never touch the GPU, ever.
//! - [`force_gpu`] — insist on the GPU, and fail loudly if there is none.
//! - `BURIN_ENGINE=cpu|gpu|auto` in the environment does the same, so a CLI,
//!   a Node process or a Python script can settle it without a code path.
//!
//! The decision is taken once per process and cached, including the negative
//! answer: "this machine has no GPU" is a real result, not an absence of one,
//! and asking wgpu again on every draw would be both slow and racy.
//!
//! Native only. In the browser WebGPU is reached from JavaScript, asynchronously
//! and without `Sync`, so `burin-map-wasm` decides there instead.

#![cfg(all(feature = "gpu", not(target_arch = "wasm32")))]

use std::sync::OnceLock;

use crate::gpu::Gpu;

/// What the caller asked for, before the hardware has a say.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Pick {
    /// GPU when there is one, CPU otherwise. The default.
    #[default]
    Auto,
    /// Never the GPU.
    ForceCpu,
    /// The GPU, and an error if there is none.
    ForceGpu,
}

static MODE: OnceLock<Pick> = OnceLock::new();
/// Resolved adapter, or `None` when the machine has none. Cached either way.
static DEVICE: OnceLock<Option<Gpu>> = OnceLock::new();

/// Settle how fields get built. Call before the first draw; ignored afterwards,
/// since the choice is cached on first use.
///
/// [Gpu::new_blocking] is not cheap — it spins up a device — so this does not
/// resolve yet, it only records the wish.
pub fn set(pick: Pick) {
    let _ = MODE.set(pick);
}

/// Never build fields on the GPU. For tests, golden files, share links.
pub fn force_cpu() {
    set(Pick::ForceCpu);
}

/// Insist on the GPU. [`device`] then returns `None` when there is none, and
/// callers with an error channel (the CLI) report it.
pub fn force_gpu() {
    set(Pick::ForceGpu);
}

/// Read `BURIN_ENGINE` (`cpu`, `gpu`, `auto`), if set. Unknown values are
/// ignored, so a typo cannot silently change what runs.
pub fn from_env() {
    match std::env::var("BURIN_ENGINE").as_deref().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Ok("cpu") => force_cpu(),
        Ok("gpu") => force_gpu(),
        Ok("auto") => set(Pick::Auto),
        _ => {}
    }
}

/// The current wish, read from the environment on first access when nothing
/// was set explicitly.
fn pick() -> Pick {
    *MODE.get_or_init(|| {
        match std::env::var("BURIN_ENGINE").as_deref().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Ok("cpu") => Pick::ForceCpu,
            Ok("gpu") => Pick::ForceGpu,
            _ => Pick::Auto,
        }
    })
}

/// The adapter to use, resolving it on first call. `None` on the CPU, whether
/// because it was asked for or because the machine has none.
///
/// On `wasm32` there is no blocking device: use [`device_async`] from the
/// browser bindings instead.
#[cfg(not(target_arch = "wasm32"))]
pub fn device() -> Option<&'static Gpu> {
    DEVICE
        .get_or_init(|| {
            if pick() == Pick::ForceCpu {
                return None;
            }
            Gpu::new_blocking()
        })
        .as_ref()
}

/// Adapter name for logs, or a reason the GPU is not in use.
pub fn describe() -> String {
    match pick() {
        Pick::ForceCpu => "CPU (forced)".to_string(),
        Pick::ForceGpu => match device() {
            Some(g) => format!("GPU: {}", g.name),
            None => "GPU (forced, but none is available)".to_string(),
        },
        Pick::Auto => match device() {
            Some(g) => format!("GPU: {}", g.name),
            None => "CPU (no usable GPU)".to_string(),
        },
    }
}

/// True when a forced GPU was asked for and could not be honoured: the CLI
/// turns this into an error rather than quietly drawing on the CPU.
pub fn gpu_unavailable() -> bool {
    pick() == Pick::ForceGpu && device().is_none()
}
