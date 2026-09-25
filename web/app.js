/**
 * burin map: terrarium elevation → WASM draw → canvas.
 *
 * View is centre lon/lat + width in km. Terrarium PNGs (Web Mercator) are
 * mosaicked and resampled to the canvas size; burin-core draws the lines.
 */

import init, { draw, style_params } from "./pkg/burin_wasm.js";

const TERRARIUM = "/terrarium";
const TILE = 256;
const R = 6378137; // Web Mercator equatorial radius, metres
// Style knobs shown up front; the rest sit under "more".
const PRIMARY = new Set([
  "spacing", "spacing_steep", "shade_gain", "flat_level", "lit_floor",
  "broad_weight", "flow_blur", "jitter", "wobble", "breaks", "azimuth", "stroke",
]);

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d", { alpha: false });
const statusEl = document.getElementById("status");
const placeEl = document.getElementById("place");
const sizeEl = document.getElementById("size");
const sizeVal = document.getElementById("size-val");
const styleBox = document.getElementById("style-controls");
const exportBtn = document.getElementById("export");
const mapEl = document.getElementById("map");

/** @type {{ lon: number, lat: number, km: number }} */
let view = { lon: -122.45, lat: 37.75, km: 20 };
/** @type {Record<string, number>} */
let style = {};
/** @type {string | null} */
let lastSvg = null;
let drawing = false;
let redrawTimer = 0;
let drag = null;
let gen = 0;

const tileCache = new Map(); // "z/x/y" → Float32Array(256*256)

function setStatus(msg, busy = false) {
  statusEl.textContent = msg;
  statusEl.parentElement.classList.toggle("busy", busy);
}

function lonLatToMerc(lon, lat) {
  const x = (R * lon * Math.PI) / 180;
  const y = R * Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360));
  return [x, y];
}

function mercToLonLat(x, y) {
  const lon = (x / R) * (180 / Math.PI);
  const lat = (2 * Math.atan(Math.exp(y / R)) - Math.PI / 2) * (180 / Math.PI);
  return [lon, lat];
}

function lonLatToTile(lon, lat, z) {
  const n = 2 ** z;
  const x = ((lon + 180) / 360) * n;
  const s = Math.sin((lat * Math.PI) / 180);
  const y = (0.5 - Math.log((1 + s) / (1 - s)) / (4 * Math.PI)) * n;
  return [x, y];
}

function tilePxM(lat, z) {
  return (Math.cos((lat * Math.PI) / 180) * 2 * Math.PI * R) / (TILE * 2 ** z);
}

function chooseZ(lat, metresPerPx) {
  // Prefer tiles a little finer than the output so resampling has something to work with.
  let z = Math.round(Math.log2((Math.cos((lat * Math.PI) / 180) * 2 * Math.PI * R) / (TILE * metresPerPx)));
  return Math.max(0, Math.min(15, z));
}

function parseLatLon(q) {
  const m = q.trim().match(/^(-?\d+(?:\.\d+)?)\s*[, ]\s*(-?\d+(?:\.\d+)?)$/);
  if (!m) return null;
  const lat = +m[1], lon = +m[2];
  if (lat < -90 || lat > 90 || lon < -180 || lon > 180) return null;
  return { lat, lon, label: `${lat.toFixed(5)}, ${lon.toFixed(5)}` };
}

async function locate(query) {
  const coords = parseLatLon(query);
  if (coords) return coords;
  const url = "https://nominatim.openstreetmap.org/search?" +
    new URLSearchParams({ q: query, format: "json", limit: "1" });
  const r = await fetch(url, { headers: { Accept: "application/json" } });
  if (!r.ok) throw new Error(`geocoding failed (${r.status})`);
  const hits = await r.json();
  if (!hits.length) throw new Error(`nothing found for “${query}”`);
  return { lat: +hits[0].lat, lon: +hits[0].lon, label: hits[0].display_name };
}

async function tileElev(z, x, y) {
  const key = `${z}/${x}/${y}`;
  if (tileCache.has(key)) return tileCache.get(key);
  if (tileCache.size > 256) {
    const first = tileCache.keys().next().value;
    tileCache.delete(first);
  }
  const url = `${TERRARIUM}/${z}/${x}/${y}.png`;
  const r = await fetch(url);
  if (!r.ok) {
    // Missing over deep ocean — return zeros.
    const empty = new Float32Array(TILE * TILE);
    tileCache.set(key, empty);
    return empty;
  }
  const bmp = await createImageBitmap(await r.blob());
  const c = new OffscreenCanvas(TILE, TILE);
  const g = c.getContext("2d", { willReadFrequently: true });
  g.drawImage(bmp, 0, 0);
  const { data } = g.getImageData(0, 0, TILE, TILE);
  const elev = new Float32Array(TILE * TILE);
  for (let i = 0, j = 0; i < data.length; i += 4, j++) {
    elev[j] = data[i] * 256 + data[i + 1] + data[i + 2] / 256 - 32768;
  }
  tileCache.set(key, elev);
  return elev;
}

/** Sample elevation at global tile-pixel coordinates (z-space). */
function sampleMosaic(tiles, x0, y0, nx, ny, px, py) {
  const x = px - x0 * TILE;
  const y = py - y0 * TILE;
  if (x < 0 || y < 0 || x >= nx * TILE || y >= ny * TILE) return 0;
  const ix = Math.min(Math.floor(x), nx * TILE - 1);
  const iy = Math.min(Math.floor(y), ny * TILE - 1);
  const tx = Math.floor(ix / TILE);
  const ty = Math.floor(iy / TILE);
  const elev = tiles[ty * nx + tx];
  if (!elev) return 0;
  return elev[(iy % TILE) * TILE + (ix % TILE)];
}

async function buildDem(outW, outH) {
  const aspect = outH / outW;
  const widthM = view.km * 1000;
  const heightM = widthM * aspect;
  const [cx, cy] = lonLatToMerc(view.lon, view.lat);
  const west = cx - widthM / 2;
  const east = cx + widthM / 2;
  const south = cy - heightM / 2;
  const north = cy + heightM / 2;
  const [lonW, latS] = mercToLonLat(west, south);
  const [lonE, latN] = mercToLonLat(east, north);

  const pxM = widthM / outW;
  const z = chooseZ(view.lat, pxM);

  const [tx0f, ty0f] = lonLatToTile(lonW, latN, z); // NW
  const [tx1f, ty1f] = lonLatToTile(lonE, latS, z); // SE
  const x0 = Math.floor(tx0f);
  const y0 = Math.floor(ty0f);
  const x1 = Math.floor(tx1f);
  const y1 = Math.floor(ty1f);
  const nx = x1 - x0 + 1;
  const ny = y1 - y0 + 1;
  if (nx * ny > 64) {
    throw new Error(`too many tiles (${nx}×${ny}); zoom in`);
  }

  setStatus(`fetching ${nx * ny} elevation tiles (z${z})…`, true);
  const jobs = [];
  for (let ty = y0; ty <= y1; ty++) {
    for (let tx = x0; tx <= x1; tx++) {
      jobs.push(tileElev(z, tx, ty));
    }
  }
  const tiles = await Promise.all(jobs);

  // For each output pixel: mercator → lon/lat → fractional tile → nearest sample.
  const elev = new Float32Array(outW * outH);
  for (let row = 0; row < outH; row++) {
    const my = north - (row + 0.5) * (heightM / outH);
    for (let col = 0; col < outW; col++) {
      const mx = west + (col + 0.5) * (widthM / outW);
      const [lon, lat] = mercToLonLat(mx, my);
      const [fx, fy] = lonLatToTile(lon, lat, z);
      // Pixel centre in global tile-pixel space.
      const px = fx * TILE;
      const py = fy * TILE;
      elev[row * outW + col] = sampleMosaic(tiles, x0, y0, nx, ny, px, py);
    }
  }
  return { elev, width: outW, height: outH, pxM, z, tiles: nx * ny };
}

function paint(drawing) {
  const dpr = window.devicePixelRatio || 1;
  const cssW = mapEl.clientWidth;
  const cssH = mapEl.clientHeight;
  canvas.width = Math.round(cssW * dpr);
  canvas.height = Math.round(cssH * dpr);
  canvas.style.width = cssW + "px";
  canvas.style.height = cssH + "px";
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, cssW, cssH);

  const sx = cssW / drawing.width;
  const sy = cssH / drawing.height;
  const paths = drawing.paths;
  const ends = drawing.ends;
  const stroke = (style.stroke ?? 1.2) * sx;

  ctx.strokeStyle = "#000000";
  ctx.lineWidth = stroke;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  let start = 0;
  for (let i = 0; i < ends.length; i++) {
    const end = ends[i];
    if (end - start < 1) { start = end; continue; }
    ctx.beginPath();
    ctx.moveTo(paths[start * 2] * sx, paths[start * 2 + 1] * sy);
    for (let p = start + 1; p < end; p++) {
      ctx.lineTo(paths[p * 2] * sx, paths[p * 2 + 1] * sy);
    }
    if (end - start === 1) {
      // Single-point dash: tiny mark.
      const x = paths[start * 2] * sx;
      const y = paths[start * 2 + 1] * sy;
      ctx.lineTo(x + 0.01, y);
    }
    ctx.stroke();
    start = end;
  }
}

async function redraw() {
  if (drawing) return;
  drawing = true;
  exportBtn.disabled = true;
  const my = ++gen;
  // Clear to white while we wait.
  {
    const dpr = window.devicePixelRatio || 1;
    const cssW = mapEl.clientWidth;
    const cssH = mapEl.clientHeight;
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
    canvas.style.width = cssW + "px";
    canvas.style.height = cssH + "px";
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, cssW, cssH);
  }
  try {
    const cssW = mapEl.clientWidth;
    const cssH = mapEl.clientHeight;
    // Cap DEM size for speed; canvas still fills the screen via scale.
    const maxEdge = 1400;
    const scale = Math.min(1, maxEdge / Math.max(cssW, cssH));
    const outW = Math.max(64, Math.round(cssW * scale));
    const outH = Math.max(64, Math.round(cssH * scale));

    const dem = await buildDem(outW, outH);
    if (my !== gen) return;
    setStatus(`drawing ${outW}×${outH}…`, true);
    const t0 = performance.now();
    const result = draw(dem.elev, dem.width, dem.height, dem.pxM, style);
    if (my !== gen) return;
    paint(result);
    lastSvg = result.svg;
    const wall = performance.now() - t0;
    setStatus(
      `${view.lat.toFixed(3)}°, ${view.lon.toFixed(3)}° · ${view.km} km · ` +
      `z${dem.z} · ${result.lines.toLocaleString()} lines · ` +
      `${result.dashes.toLocaleString()} dashes · ${Math.round(result.ms)} ms draw` +
      (wall > result.ms + 30 ? ` (${Math.round(wall)} ms with tiles)` : ""),
    );
  } catch (e) {
    console.error(e);
    setStatus(String(e.message || e));
  } finally {
    drawing = false;
    exportBtn.disabled = !lastSvg;
  }
}

function scheduleRedraw(ms = 180) {
  clearTimeout(redrawTimer);
  redrawTimer = setTimeout(() => redraw(), ms);
}

function sizeLabel(km) {
  return km >= 10 ? `${km} km` : `${km} km`;
}

const RANGES = {
  spacing: [1, 12], spacing_steep: [0.2, 1], spacing_dark: [0.2, 1], spacing_gamma: [0.3, 3],
  test_ratio: [0.2, 1], fill_ratio: [0.5, 1], step: [0.5, 2], min_len: [1, 20],
  flow_blur: [1, 30], shade_blur: [0.5, 8], broad_blur: [2, 40], broad_weight: [0, 1],
  wobble: [0, 60], wobble_size: [2, 40],
  azimuth: [0, 360], altitude: [5, 85], z_exag: [1, 12],
  flat_level: [0, 0.6], shade_gain: [0.5, 8], dark_gamma: [0.3, 2], lit_gain: [0.5, 4], lit_floor: [0, 0.3],
  period: [1, 5], duty_min: [0, 0.5], duty_max: [0.5, 1], cutoff: [0, 0.15],
  jitter: [0, 1], breaks: [0, 0.5], stroke: [0.4, 3], seed: [0, 100],
};

function styleControl(p) {
  style[p.name] = p.default;
  const wrap = document.createElement("div");
  wrap.className = "ctrl";
  const isInt = p.name === "seed";
  const [lo, hi] = RANGES[p.name] || [0, Math.max(1, p.default * 2)];
  const step = isInt ? 1 : (hi - lo) / 200;
  wrap.innerHTML = `
    <label><span class="name">${p.name}</span><span class="val">${fmt(p.default)}</span></label>
    <input type="range" min="${lo}" max="${hi}" step="${step}" value="${p.default}" />
    <p class="help">${p.help}</p>`;
  const input = wrap.querySelector("input");
  const val = wrap.querySelector(".val");
  input.addEventListener("input", () => {
    const v = isInt ? Math.round(+input.value) : +input.value;
    style[p.name] = v;
    val.textContent = fmt(v);
  });
  input.addEventListener("change", () => scheduleRedraw(60));
  return wrap;
}

function buildStyleControls(params) {
  styleBox.innerHTML = "";
  const primary = params.filter((p) => PRIMARY.has(p.name));
  const rest = params.filter((p) => !PRIMARY.has(p.name));
  primary.forEach((p) => styleBox.appendChild(styleControl(p)));
  if (!rest.length) return;
  const det = document.createElement("details");
  det.innerHTML = `<summary>more parameters</summary>`;
  const inner = document.createElement("div");
  inner.className = "more";
  rest.forEach((p) => inner.appendChild(styleControl(p)));
  det.appendChild(inner);
  styleBox.appendChild(det);
}

function fmt(v) {
  if (Number.isInteger(v)) return String(v);
  const t = Math.round(v * 1000) / 1000;
  return String(t);
}

function resetStyle(params) {
  for (const p of params) style[p.name] = p.default;
  buildStyleControls(params);
  scheduleRedraw(60);
}

async function goPlace() {
  const q = placeEl.value.trim();
  if (!q) return;
  setStatus(`looking up “${q}”…`, true);
  try {
    const hit = await locate(q);
    view.lon = hit.lon;
    view.lat = hit.lat;
    placeEl.value = hit.label;
    scheduleRedraw(0);
  } catch (e) {
    setStatus(String(e.message || e));
  }
}

function onPointerDown(e) {
  if (e.button !== 0) return;
  drag = { x: e.clientX, y: e.clientY, lon: view.lon, lat: view.lat };
  mapEl.classList.add("dragging");
  mapEl.setPointerCapture?.(e.pointerId);
}

function onPointerMove(e) {
  if (!drag) return;
  const cssW = mapEl.clientWidth;
  const metresPerPx = (view.km * 1000) / cssW;
  const dx = (e.clientX - drag.x) * metresPerPx;
  const dy = (e.clientY - drag.y) * metresPerPx;
  const [cx, cy] = lonLatToMerc(drag.lon, drag.lat);
  const [lon, lat] = mercToLonLat(cx - dx, cy + dy);
  view.lon = lon;
  view.lat = lat;
}

function onPointerUp() {
  if (!drag) return;
  drag = null;
  mapEl.classList.remove("dragging");
  scheduleRedraw(80);
}

function onWheel(e) {
  e.preventDefault();
  const factor = e.deltaY > 0 ? 1.12 : 1 / 1.12;
  // Zoom toward cursor.
  const rect = mapEl.getBoundingClientRect();
  const cssW = rect.width;
  const cssH = rect.height;
  const fx = (e.clientX - rect.left) / cssW - 0.5;
  const fy = (e.clientY - rect.top) / cssH - 0.5;
  const widthM = view.km * 1000;
  const heightM = widthM * (cssH / cssW);
  const [cx, cy] = lonLatToMerc(view.lon, view.lat);
  const mx = cx + fx * widthM;
  const my = cy - fy * heightM;
  view.km = Math.min(200, Math.max(1.5, view.km * factor));
  const widthM2 = view.km * 1000;
  const heightM2 = widthM2 * (cssH / cssW);
  const [lon, lat] = mercToLonLat(mx - fx * widthM2, my + fy * heightM2);
  view.lon = lon;
  view.lat = lat;
  sizeEl.value = String(Math.round(view.km));
  sizeVal.textContent = sizeLabel(+sizeEl.value);
  scheduleRedraw(120);
}

async function main() {
  setStatus("loading engine…", true);
  await init();
  const params = style_params();
  buildStyleControls(params);

  document.getElementById("go").addEventListener("click", goPlace);
  placeEl.addEventListener("keydown", (e) => { if (e.key === "Enter") goPlace(); });
  sizeEl.addEventListener("input", () => {
    view.km = +sizeEl.value;
    sizeVal.textContent = sizeLabel(view.km);
  });
  sizeEl.addEventListener("change", () => scheduleRedraw(60));
  document.getElementById("reset-style").addEventListener("click", () => resetStyle(params));
  exportBtn.addEventListener("click", () => {
    if (!lastSvg) return;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([lastSvg], { type: "image/svg+xml" }));
    a.download = `burin_${view.lat.toFixed(3)}_${view.lon.toFixed(3)}.svg`;
    a.click();
    URL.revokeObjectURL(a.href);
  });
  document.querySelectorAll(".presets button").forEach((b) => {
    b.addEventListener("click", () => {
      const [lat, lon] = b.dataset.at.split(",").map(Number);
      view = { lat, lon, km: +b.dataset.km };
      sizeEl.value = String(view.km);
      sizeVal.textContent = sizeLabel(view.km);
      placeEl.value = b.textContent;
      scheduleRedraw(0);
    });
  });

  mapEl.addEventListener("pointerdown", onPointerDown);
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  mapEl.addEventListener("wheel", onWheel, { passive: false });
  window.addEventListener("resize", () => scheduleRedraw(200));

  placeEl.value = "Twin Peaks, San Francisco";
  await redraw();
}

main().catch((e) => {
  console.error(e);
  setStatus("failed to start: " + (e.message || e));
});
