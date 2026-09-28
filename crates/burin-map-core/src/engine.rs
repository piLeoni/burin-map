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

static FORCE_CPU: OnceLock<bool> = OnceLock::new();
/// Resolved adapter, or `None` when the machine has none. Cached either way.
static DEVICE: OnceLock<Option<Gpu>> = OnceLock::new();

/// Never build fields on the GPU. Call before the first draw; ignored
/// afterwards, since the choice is cached on first use.
pub fn force_cpu() {
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
