//! Which device builds the terrain fields.
//!
//! The GPU when the machine has one, the CPU when it does not. There is no
//! third option and nothing to configure: the drawing is the product, and the
//! fastest device that can make it is simply the right one.
//!
//! [`force_cpu`] exists for the cases that need reproducible bytes — a
//! golden-file test, a share link, a bug report — since the GPU works in f32
//! and lands every dash in a slightly different place. `BURIN_ENGINE=cpu` does
//! the same from the environment, so the CLI, Node and Python all honour it
//! without a code path.
//!
//! The choice is resolved once per process and cached, including the negative
//! answer: "this machine has no GPU" is a result, not an absence of one, and
//! asking wgpu again on every draw would be both slow and racy.
//!
//! Native only. In the browser WebGPU is reached from JavaScript, asynchronously
//! and without `Sync`, so `burin-map-wasm` decides there instead.

#![cfg(all(feature = "gpu", not(target_arch = "wasm32")))]

use std::sync::OnceLock;

use crate::gpu::Gpu;

/// Set only by [`force_cpu`], and only meaningful before [`device`] first runs.
static FORCE_CPU: OnceLock<bool> = OnceLock::new();
/// Resolved adapter, or `None` when the machine has none. Cached either way.
static DEVICE: OnceLock<Option<Gpu>> = OnceLock::new();

/// Never build fields on the GPU.
///
/// Must be called before the first draw; once [`device`] has resolved, the
/// choice is cached and this cannot take it back. Calling it that late is a
/// bug, so it says so rather than quietly doing nothing — silently ignoring
/// "use the CPU" would hand back a drawing that is not the reproducible one
/// the caller asked for.
///
/// Asking for the CPU when it is already the CPU is not a contradiction, only
/// a repetition, so `BURIN_ENGINE=cpu` followed by this call is fine.
///
/// # Panics
/// If the device has already resolved to a GPU.
pub fn force_cpu() {
    if let Some(Some(_)) = DEVICE.get() {
        panic!(
            "force_cpu() was called after the first draw, when a GPU is already \
             resolved and cached. Call it first — at the top of main(), before any \
             draw — or use `BURIN_ENGINE=cpu`, which is read before the first draw \
             and cannot arrive too late."
        );
    }
    let _ = FORCE_CPU.set(true);
}

/// True when the CPU was asked for, here or through `BURIN_ENGINE=cpu`.
fn forced() -> bool {
    *FORCE_CPU.get_or_init(|| {
        std::env::var("BURIN_ENGINE").is_ok_and(|v| v.trim().eq_ignore_ascii_case("cpu"))
    })
}

/// The adapter to use, resolving it on first call. `None` on the CPU, whether
/// because it was asked for or because the machine has none.
#[cfg(not(target_arch = "wasm32"))]
pub fn device() -> Option<&'static Gpu> {
    DEVICE
        .get_or_init(|| if forced() { None } else { Gpu::new_blocking() })
        .as_ref()
}

/// Where the fields are being built, for logs: "GPU: Apple M4 (Metal)",
/// "CPU (forced)", "CPU (no usable GPU)".
pub fn describe() -> String {
    match device() {
        Some(g) => format!("GPU: {}", g.name),
        None if forced() => "CPU (forced)".to_string(),
        None => "CPU (no usable GPU)".to_string(),
    }
}
