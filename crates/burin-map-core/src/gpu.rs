//! `build_fields` on the GPU (wgpu: Metal / Vulkan / DX12 natively, WebGPU in
//! the browser). Same pipeline as fields.rs, but f32 maths on another device
//! rounds differently, and the streamline tracer turns those last-bit
//! differences into a different (equally valid) drawing. Output is therefore
//! not identical to the CPU path, nor across GPUs.

use std::collections::HashMap;
use std::sync::Mutex;

use wgpu::util::DeviceExt;

use crate::fields::Fields;
use crate::grid::{box_radii, gaussian_kernel};
use crate::rng::Rng;
use crate::style::Style;

const ENTRIES: &[&str] = &[
    "conv_h", "conv_v", "box_h", "box_v", "map1", "gradient", "direction", "minmax", "histogram", "hist_scan", "rank",
    "sum_partial", "sum_final", "wobble", "shade", "shade_norm", "input_space", "finish",
];
const BINS: usize = 65536;

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct P {
    w: u32,
    h: u32,
    r: u32,
    mode: u32,
    f: [f32; 12],
}

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipes: HashMap<&'static str, wgpu::ComputePipeline>,
    pool: Mutex<Vec<wgpu::Buffer>>,
    staging: Mutex<Vec<wgpu::Buffer>>,
    /// Adapter name and backend, for logs.
    pub name: String,
}

impl Gpu {
    /// `None` when there is no usable adapter (no WebGPU in this browser, …).
    pub async fn new() -> Option<Gpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok()?;
        let info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .ok()?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fields"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fields.wgsl").into()),
        });
        let pipes = ENTRIES
            .iter()
            .map(|&e| {
                let pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(e),
                    layout: None,
                    module: &module,
                    entry_point: Some(e),
                    compilation_options: Default::default(),
                    cache: None,
                });
                (e, pipe)
            })
            .collect();
        Some(Gpu { device, queue, pipes, pool: Default::default(), staging: Default::default(), name: format!("{} ({:?})", info.name, info.backend) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_blocking() -> Option<Gpu> {
        pollster::block_on(Self::new())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_fields_blocking(&self, elev: &[f32], w: usize, h: usize, px_m: f32, st: &Style) -> Fields {
        pollster::block_on(self.build_fields(elev, w, h, px_m, st))
    }

    pub async fn build_fields(&self, elev: &[f32], w: usize, h: usize, px_m: f32, st: &Style) -> Fields {
        let n = w * h;
        let mut r = Rec::new(self, w, h);

        let elev_b = r.upload_grid(elev);
        let land = r.grid_buf();
        r.grid("map1", r.p(1, &[0.0, 0.5]), &[(1, &elev_b), (2, &land)]);

        // Direction: gradient of a heavily smoothed surface, turned 90°.
        let (a, t) = (r.grid_buf(), r.grid_buf());
        r.blur(&elev_b, &a, &t, st.mod_smooth);
        let (gy, gx) = (r.grid_buf(), r.grid_buf());
        r.grid("gradient", r.p(0, &[px_m]), &[(1, &a), (2, &gy), (3, &gx)]);
        let (mag, tx, ty) = (r.grid_buf(), r.grid_buf(), r.grid_buf());
        r.grid("direction", r.p(0, &[]), &[(1, &gx), (2, &gy), (3, &mag), (4, &tx), (5, &ty)]);

        let (stats, hist) = r.histogram(&mag, &land, false);
        let table = r.buf(BINS);
        r.run("hist_scan", r.p(0, &[]), &[(3, &table), (6, &stats), (7, &hist)], (1, 1));
        let steep = r.grid_buf();
        r.grid("rank", r.p(0, &[]), &[(1, &mag), (2, &land), (3, &table), (4, &steep), (6, &stats)]);

        let elev_px = r.grid_buf();
        r.grid("map1", r.p(0, &[px_m]), &[(1, &elev_b), (2, &elev_px)]);
        let s = r.grid_buf();
        r.relative_shade(&elev_px, &s, None, st.input_blur, &land, st, [&a, &t, &gy, &gx]);
        if st.input_broad > 0.0 {
            let b = r.grid_buf();
            r.relative_shade(&elev_px, &b, Some((&s, st.input_broad)), st.input_broad_blur, &land, st, [&a, &t, &gy, &gx]);
        }

        r.blur(&land, &a, &t, 2.0);
        let input = r.grid_buf();
        let f = [
            st.input_gain,
            st.input_dark_gamma,
            st.input_light_gain,
            st.input_flat,
            st.input_light_floor,
            st.space_steep,
            st.space_steep_gamma,
            st.space_light,
            st.space_dark,
        ];
        r.grid("input_space", r.p(0, &f), &[(1, &s), (2, &a), (3, &steep), (4, &input), (5, &gx)]);
        r.blur(&gx, &a, &t, 3.0);
        let (space, walk) = (gy, elev_px);
        r.grid("finish", r.p(0, &[st.space]), &[(1, &a), (2, &space), (3, &land), (4, &mag), (5, &walk)]);

        // On near-flat ground a smooth random rotation, fading to none on
        // slopes. Last, so the GPU runs the rest while the CPU makes noise.
        if st.mod_wobble > 0.0 {
            r.flush();
            let mut noise = vec![0.0f32; n];
            Rng::new(st.batch_seed).fill_normal(&mut noise);
            let noise = r.upload_grid(&noise);
            r.blur(&noise, &a, &t, st.mod_wobble_size);
            let moments = r.buf(2);
            r.mean_std(&a, &t, &moments);
            let amp = st.mod_wobble.to_radians();
            r.grid("wobble", r.p(0, &[amp]), &[(1, &tx), (2, &ty), (3, &a), (4, &steep), (5, &moments)]);
        }

        let mut out = r.read(&[&tx, &ty, &input, &space, &walk]).await.into_iter();
        let mut next = || out.next().unwrap();
        let (tx, ty, input, space) = (next(), next(), next(), next());
        let walkable = next().into_iter().map(|v| v > 0.5).collect();
        Fields { w, h, tx, ty, input, walkable, space }
    }
}

/// Commands for one `build_fields`, recorded into a single submission.
struct Rec<'a> {
    g: &'a Gpu,
    enc: wgpu::CommandEncoder,
    w: u32,
    h: u32,
    /// Pool buffers in use, returned by `read`.
    taken: std::cell::RefCell<Vec<wgpu::Buffer>>,
}

impl<'a> Rec<'a> {
    fn new(g: &'a Gpu, w: usize, h: usize) -> Self {
        let enc = g.device.create_command_encoder(&Default::default());
        Rec { g, enc, w: w as u32, h: h as u32, taken: Default::default() }
    }

    /// Start executing what is recorded so far.
    fn flush(&mut self) {
        let enc = std::mem::replace(&mut self.enc, self.g.device.create_command_encoder(&Default::default()));
        self.g.queue.submit([enc.finish()]);
    }

    fn p(&self, mode: u32, f: &[f32]) -> P {
        let mut p = P { w: self.w, h: self.h, mode, ..Default::default() };
        p.f[..f.len()].copy_from_slice(f);
        p
    }

    fn buf(&self, len: usize) -> wgpu::Buffer {
        self.g.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (len.max(1) * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// A w × h buffer from the pool: fresh GPU allocations cost more than
    /// the kernels themselves (first-touch mapping and zero-fill).
    fn grid_buf(&self) -> wgpu::Buffer {
        let size = (self.w * self.h).max(1) as u64 * 4;
        let mut pool = self.g.pool.lock().unwrap();
        let b = match pool.iter().position(|b| b.size() == size) {
            Some(i) => pool.swap_remove(i),
            None => self.buf((self.w * self.h) as usize),
        };
        self.taken.borrow_mut().push(b.clone());
        b
    }

    fn upload_grid(&self, data: &[f32]) -> wgpu::Buffer {
        let b = self.grid_buf();
        self.g.queue.write_buffer(&b, 0, bytemuck::cast_slice(data));
        b
    }

    fn upload<T: bytemuck::Pod>(&self, data: &[T]) -> wgpu::Buffer {
        self.g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn run(&mut self, entry: &str, p: P, binds: &[(u32, &wgpu::Buffer)], groups: (u32, u32)) {
        let dev = &self.g.device;
        let pipe = &self.g.pipes[entry];
        let ub = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&p),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let mut entries = vec![wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }];
        entries.extend(binds.iter().map(|&(binding, b)| wgpu::BindGroupEntry { binding, resource: b.as_entire_binding() }));
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipe.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut pass = self.enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(pipe);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(groups.0, groups.1, 1);
    }

    /// One invocation per pixel.
    fn grid(&mut self, entry: &str, p: P, binds: &[(u32, &wgpu::Buffer)]) {
        let groups = (self.w.div_ceil(16), self.h.div_ceil(16));
        self.run(entry, p, binds, groups);
    }

    /// `grid::blur` into `dst`; `tmp` is scratch. `src` must differ from both.
    fn blur(&mut self, src: &wgpu::Buffer, dst: &wgpu::Buffer, tmp: &wgpu::Buffer, sigma: f32) {
        if sigma <= 0.0 {
            self.enc.copy_buffer_to_buffer(src, 0, dst, 0, src.size());
            return;
        }
        let mut p = self.p(0, &[]);
        if sigma < 3.0 {
            let k = gaussian_kernel(sigma);
            let kb = self.upload(&k);
            p.r = (k.len() / 2) as u32;
            self.run("conv_h", p, &[(1, src), (2, tmp), (3, &kb)], (self.w.div_ceil(256), self.h));
            self.run("conv_v", p, &[(1, tmp), (2, dst), (3, &kb)], (self.w.div_ceil(64), self.h.div_ceil(4)));
            return;
        }
        const RUN: u32 = 16;
        for (i, r) in box_radii(sigma, 3).into_iter().enumerate() {
            p.r = r as u32;
            let from = if i == 0 { src } else { dst };
            let rows = (self.w.div_ceil(RUN).div_ceil(64), self.h.div_ceil(4));
            self.run("box_h", p, &[(1, from), (2, tmp)], rows);
            let cols = (self.w.div_ceil(64), self.h.div_ceil(RUN).div_ceil(4));
            self.run("box_v", p, &[(1, tmp), (2, dst)], cols);
        }
    }

    /// Min / max / count and a histogram of `v` (or |v|) over `mask`.
    fn histogram(&mut self, v: &wgpu::Buffer, mask: &wgpu::Buffer, abs: bool) -> (wgpu::Buffer, wgpu::Buffer) {
        let stats = self.upload(&[u32::MAX, 0, 0, 0]);
        let hist = self.buf(BINS);
        let p = self.p(abs as u32, &[]);
        self.grid("minmax", p, &[(1, v), (2, mask), (6, &stats)]);
        self.grid("histogram", p, &[(1, v), (2, mask), (6, &stats), (7, &hist)]);
        (stats, hist)
    }

    /// Mean into `out[0]`, 1 / standard deviation into `out[1]`.
    fn mean_std(&mut self, v: &wgpu::Buffer, partial: &wgpu::Buffer, out: &wgpu::Buffer) {
        let groups = (self.w * self.h).div_ceil(256);
        let gx = groups.min(65535);
        let gy = groups.div_ceil(gx);
        for mode in 0..2 {
            self.run("sum_partial", self.p(mode, &[]), &[(1, v), (2, partial), (3, out)], (gx, gy));
            let mut p = self.p(mode, &[]);
            p.r = gx * gy;
            self.run("sum_final", p, &[(2, partial), (3, out)], (1, 1));
        }
    }

    /// `fields::relative_shade` into `dst`, or mixed into `mix.0` by `mix.1`.
    #[allow(clippy::too_many_arguments)]
    fn relative_shade(
        &mut self,
        elev_px: &wgpu::Buffer,
        dst: &wgpu::Buffer,
        mix: Option<(&wgpu::Buffer, f32)>,
        sigma: f32,
        land: &wgpu::Buffer,
        st: &Style,
        [a, t, gy, gx]: [&wgpu::Buffer; 4],
    ) {
        self.blur(elev_px, a, t, sigma);
        self.grid("gradient", self.p(0, &[1.0]), &[(1, a), (2, gy), (3, gx)]);
        let (alt, az) = (st.sun_altitude.to_radians(), st.sun_azimuth.to_radians());
        let light = [alt.cos() * az.sin(), -alt.cos() * az.cos(), alt.sin(), st.sun_exag];
        self.grid("shade", self.p(0, &light), &[(1, gy), (2, gx), (3, dst)]);
        let (stats, hist) = self.histogram(dst, land, true);
        self.run("hist_scan", self.p(1, &[98.0]), &[(3, t), (6, &stats), (7, &hist)], (1, 1));
        match mix {
            None => self.grid("shade_norm", self.p(0, &[]), &[(1, dst), (2, a), (6, &stats)]),
            Some((into, k)) => self.grid("shade_norm", self.p(1, &[k]), &[(1, dst), (2, into), (6, &stats)]),
        }
    }

    async fn read(self, bufs: &[&wgpu::Buffer]) -> Vec<Vec<f32>> {
        let Rec { g, mut enc, taken, .. } = self;
        let staging: Vec<wgpu::Buffer> = bufs
            .iter()
            .map(|b| {
                let mut pool = g.staging.lock().unwrap();
                let s = match pool.iter().position(|s| s.size() == b.size()) {
                    Some(i) => pool.swap_remove(i),
                    None => g.device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: b.size(),
                        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                };
                enc.copy_buffer_to_buffer(b, 0, &s, 0, b.size());
                s
            })
            .collect();
        g.queue.submit([enc.finish()]);
        let waits: Vec<_> = staging
            .iter()
            .map(|s| {
                let (tx, rx) = futures_channel::oneshot::channel();
                s.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                rx
            })
            .collect();
        #[cfg(not(target_arch = "wasm32"))]
        g.device.poll(wgpu::PollType::wait_indefinitely()).expect("gpu poll");
        let mut out = Vec::with_capacity(bufs.len());
        for (s, rx) in staging.iter().zip(waits) {
            rx.await.expect("map callback").expect("map buffer");
            let view = s.slice(..).get_mapped_range().expect("mapped range");
            out.push(bytemuck::cast_slice::<u8, f32>(&view).to_vec());
            drop(view);
            s.unmap();
        }
        g.staging.lock().unwrap().extend(staging);
        g.pool.lock().unwrap().extend(taken.into_inner());
        out
    }
}
