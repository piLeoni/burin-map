import { useEffect, useRef, useState, useCallback } from 'react'
import { useAtom } from 'jotai'
import mapboxgl from 'mapbox-gl'
import 'mapbox-gl/dist/mapbox-gl.css'
import { bathymetryNeedsNetworkFetch, fetchBathymetryGrid, waterSpecNeedsDepth } from '../lib/bathymetry'
import { buildDem } from '../lib/dem'
import { batchesJson, drawBatches, loadBurin } from '../lib/burin'
import { waterTiles } from '../lib/water'
import { FIT_PAD, MAX_EDGE, groundKm, groundWidthAt, sig3 } from '../lib/frame'
import { cropGeojson, cropSvg, strokeOf } from '../lib/crop'
import { watchBoard } from '../lib/board'
import { createProgress } from '../lib/progress'
import { logTimings, timingsOn } from '../lib/timings'
import LoadPanel from './LoadPanel'
import { mapViewAtom, cameraRequestAtom, frameAtom, forceCpu } from '../state'

const MAPBOX_TOKEN = import.meta.env.VITE_MAPBOX_TOKEN || ''
const SETTLE_MS = 90

/* ---------------------------------------------------------------- */
/*  MapView                                                          */
/* ---------------------------------------------------------------- */

function fmtCount(n) { return Number(n).toLocaleString() }

/** wgpu names its backends like BrowserWebGpu. */
function gpuName(g) { return /webgpu/i.test(g) ? 'WebGPU' : g }

function isUserGesture(e) {
  return !!(e && e.originalEvent)
}

/** How much to zoom, in zoom levels, for the frame to fill the map at latitude `lat`; 0 without a frame. */
function zoomToFit(map, frame, lat) {
  const g = groundKm(frame)
  if (!g) return 0
  const el = map.getContainer()
  const kmPerPx = groundWidthAt(lat, map.getZoom(), 1)
  const k = Math.min(
    (el.clientWidth - 2 * FIT_PAD) / (g[0] / kmPerPx),
    (el.clientHeight - 2 * FIT_PAD) / (g[1] / kmPerPx),
  )
  return k > 0 ? Math.log2(k) : 0
}

export default function MapView({ batches, water, bathymetry, onStatus, onDrawing, drawing }) {
  const containerRef = useRef(null)
  const mapRef = useRef(null)
  const genRef = useRef(0)
  const movingRef = useRef(false)
  const readyRef = useRef(false)
  const skipStyleOnceRef = useRef(true)
  const settleTimer = useRef(0)
  const batchesRef = useRef(batches)
  batchesRef.current = batches
  const waterRef = useRef(water)
  waterRef.current = water
  const bathymetryRef = useRef(bathymetry)
  bathymetryRef.current = bathymetry
  const convertRef = useRef(async () => {})

  const [, setMapView] = useAtom(mapViewAtom)
  const [cameraRequest, setCameraRequest] = useAtom(cameraRequestAtom)
  const cameraRequestRef = useRef(cameraRequest)
  cameraRequestRef.current = cameraRequest
  const [frame, setFrame] = useAtom(frameAtom)

  const mapViewRef = useRef(null)
  if (!mapViewRef.current) {
    try {
      const raw =
        localStorage.getItem('burin-map.map') ?? localStorage.getItem('burin.map')
      mapViewRef.current = raw
        ? JSON.parse(raw)
        : { center: [-122.45, 37.75], zoom: 12 }
    } catch {
      mapViewRef.current = { center: [-122.45, 37.75], zoom: 12 }
    }
  }

  const [busy, setBusy] = useState(false)
  const [load, setLoad] = useState(null)
  const [loadFailed, setLoadFailed] = useState(false)
  const progressRef = useRef(null)
  const [downloadOpen, setDownloadOpen] = useState(false)
  const frameRef = useRef(frame)
  frameRef.current = frame

  const convert = useCallback(async () => {
    const map = mapRef.current
    if (!map || movingRef.current) return
    if (map.getContainer().clientWidth < 2 || map.getContainer().clientHeight < 2) {
      clearTimeout(settleTimer.current)
      settleTimer.current = setTimeout(() => convertRef.current(), 100)
      return
    }

    const my = ++genRef.current
    const list = Array.isArray(batchesRef.current) ? batchesRef.current : []
    if (list.length === 0) {
      onDrawing(null)
      return
    }
    const b = map.getBounds()
    const bounds = {
      west: b.getWest(),
      south: b.getSouth(),
      east: b.getEast(),
      north: b.getNorth(),
    }

    const el = map.getContainer()
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    const cssW = el.clientWidth
    const cssH = el.clientHeight
    const scale = Math.min(1, MAX_EDGE / Math.max(cssW * dpr, cssH * dpr))
    const outW = Math.max(64, Math.round(cssW * dpr * scale))
    const outH = Math.max(64, Math.round(cssH * dpr * scale))

    const wet = waterRef.current
    const bathy = bathymetryRef.current
    const waterSpec = wet && {
      classes: wet.classes,
      shoreline: wet.shoreline,
      batches: wet.batches,
    }
    setBusy(true)
    setLoadFailed(false)
    setLoad(null)
    onStatus({ phase: 'working' })
    progressRef.current?.close()
    let progress = null
    let stopBoard = () => {}

    try {
      const t0 = performance.now()
      const m = await loadBurin()
      if (my !== genRef.current) return
      const title = `${outW}×${outH} px · ${list.length} batch${list.length > 1 ? 'es' : ''}`
      const panel = new m.Panel(title, batchesJson(list), waterSpec ? JSON.stringify(waterSpec) : undefined)
      progress = createProgress(panel, (view) => {
        if (my === genRef.current) setLoad(view)
      })
      progressRef.current = progress
      progress.area(bounds)
      const needsDepth = waterSpec && waterSpecNeedsDepth(waterSpec)
      const [dem, tiles, depthGrid] = await Promise.all([
        buildDem(bounds, outW, outH, progress),
        wet ? waterTiles(bounds, outW, wet.source, progress) : null,
        needsDepth && bathymetryNeedsNetworkFetch(bathy)
          ? fetchBathymetryGrid(bounds, outW, outH, bathy, progress)
          : null,
      ])
      if (my !== genRef.current || movingRef.current) return
      const { width: demW, height: demH } = dem
      const tDem = performance.now()

      const result = await drawBatches(dem.elev, dem.width, dem.height, dem.pxM, batchesRef.current, {
        nw_lon: bounds.west,
        nw_lat: bounds.north,
        deg_per_px_lon: (bounds.east - bounds.west) / outW,
        deg_per_px_lat: (bounds.north - bounds.south) / outH,
      }, null, {
        gpu: !forceCpu,
        water: wet && {
          spec: JSON.stringify(waterSpec),
          bounds: [bounds.west, bounds.south, bounds.east, bounds.north],
          tiles,
          ...(depthGrid ? { depth: depthGrid } : {}),
        },
        onBoard: (board, draws) => {
          if (my !== genRef.current) return
          stopBoard = watchBoard(board, draws, progress)
        },
      })
      stopBoard()
      const tBack = performance.now()
      const received = performance.timeOrigin + tBack
      if (my !== genRef.current || movingRef.current) return
      // Whatever the board showed last, or all of it where it isn't shared.
      for (const slot of progress.slots()) progress.board(slot, result.board[slot] ?? 0)
      progress.dashes(result.dashes)
      progress.finish(true)
      if (timingsOn) {
        // Two frames later the new SVG has been painted.
        requestAnimationFrame(() => requestAnimationFrame(() => {
          const tShown = performance.now()
          const device = typeof result.gpu === 'string' ? `GPU ${result.gpu}`
            : result.gpu === false ? 'CPU (no WebGPU)' : 'CPU (forced)'
          logTimings(
            `burin-map · ${demW}×${demH} px · ${list.length} batch${list.length > 1 ? 'es' : ''}`
              + ` · ${result.threads} thread${result.threads > 1 ? 's' : ''} · ${device}`
              + ` · ${result.dashes} paths · svg ${(result.svg.length / 1e6).toFixed(1)} MB`,
            [
              [wet ? 'page: elevation and water tiles' : 'page: elevation tiles', tDem - t0],
              ...result.timings,
              ['worker → page', received - result.sent],
              ['page: shown on screen', tShown - tBack],
              ['total', tShown - t0],
            ],
            list.length > 1 ? ['batches run in parallel, so their rows overlap in time'] : [],
          )
        }))
      }

      onDrawing({
        svg: result.svg,
        geojson: result.geojson,
        lines: result.lines,
        dashes: result.dashes,
        ms: Math.round(result.ms),
        threads: result.threads,
        gpu: result.gpu,
        view: { cssW, cssH, outW, outH, bounds },
      })
      onStatus({ phase: 'ready' })
    } catch (e) {
      console.error(e)
      onStatus({ phase: 'error', detail: String(e.message || e) })
      onDrawing(null)
      stopBoard()
      progress?.log(String(e.message || e), 'error')
      progress?.finish(false)
      if (my === genRef.current) setLoadFailed(true)
    } finally {
      if (my !== genRef.current) progress?.close()
      if (my === genRef.current) setBusy(false)
    }
  }, [onDrawing, onStatus])

  convertRef.current = convert

  const persistView = useCallback(() => {
    const map = mapRef.current
    if (!map) return
    const c = map.getCenter()
    setMapView({ center: [c.lng, c.lat], zoom: map.getZoom() })
  }, [setMapView])

  // One jump: centred first at a low zoom, the map would clamp the centre to its edges.
  const applyCamera = useCallback((cam) => {
    const map = mapRef.current
    if (!map || !cam?.center) return
    map.jumpTo({ center: cam.center, zoom: map.getZoom() + zoomToFit(map, frameRef.current, cam.center[1]) })
  }, [])

  const scheduleConvert = useCallback(
    (ms = SETTLE_MS) => {
      clearTimeout(settleTimer.current)
      settleTimer.current = setTimeout(() => {
        if (!movingRef.current) convertRef.current()
      }, ms)
    },
    [],
  )

  useEffect(() => {
    if (!MAPBOX_TOKEN) {
      onStatus({
        phase: 'error',
        detail: 'Missing Mapbox token: set VITE_MAPBOX_TOKEN in web/.env.local',
      })
      return
    }

    const saved = mapViewRef.current
    mapboxgl.accessToken = MAPBOX_TOKEN
    readyRef.current = false
    skipStyleOnceRef.current = true
    movingRef.current = false

    const map = new mapboxgl.Map({
      container: containerRef.current,
      style: 'mapbox://styles/mapbox/light-v11',
      center: saved.center,
      zoom: saved.zoom,
      minZoom: 2,
      projection: 'mercator',
      pitch: 0,
      maxPitch: 0,
      dragRotate: false,
      pitchWithRotate: false,
      renderWorldCopies: false,
      attributionControl: false,
    })
    mapRef.current = map

    const onMoveStart = (e) => {
      if (!readyRef.current || !isUserGesture(e)) return
      movingRef.current = true
      clearTimeout(settleTimer.current)
      genRef.current += 1
      setBusy(false)
      setLoadFailed(false)
      onDrawing(null)
      onStatus({ phase: 'exploring' })
    }

    const onMoveEnd = (e) => {
      if (!readyRef.current) return
      if (!isUserGesture(e) && !movingRef.current) return
      movingRef.current = false
      persistView()
      scheduleConvert()
    }

    map.on('error', (e) => {
      const err = e.error ?? {}
      const url = String(err.url ?? '').replace(/access_token=[^&]+/, 'access_token=…')
      console.error('mapbox:', err.status ?? '', err.message ?? String(err), url)
    })

    map.on('load', () => {
      map.resize()
      if (cameraRequestRef.current) {
        applyCamera(cameraRequestRef.current)
        setCameraRequest(null)
      }
      readyRef.current = true
      movingRef.current = false
      skipStyleOnceRef.current = false
      persistView()
      onStatus({ phase: 'working' })
      convertRef.current()
    })

    map.on('movestart', onMoveStart)
    map.on('moveend', onMoveEnd)
    map.on('zoomstart', onMoveStart)
    map.on('zoomend', onMoveEnd)

    return () => {
      clearTimeout(settleTimer.current)
      map.remove()
      mapRef.current = null
      readyRef.current = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    if (!readyRef.current || !mapRef.current || movingRef.current) return
    if (skipStyleOnceRef.current) {
      skipStyleOnceRef.current = false
      batchesRef.current = batches
      return
    }
    batchesRef.current = batches
    scheduleConvert(120)
  }, [batches, scheduleConvert])

  const shownWaterRef = useRef(water)
  useEffect(() => {
    if (shownWaterRef.current === water) return
    shownWaterRef.current = water
    if (!readyRef.current || !mapRef.current || movingRef.current) return
    onDrawing(null)
    scheduleConvert(120)
  }, [water, scheduleConvert, onDrawing])

  useEffect(() => {
    if (!cameraRequest || !readyRef.current) return
    genRef.current += 1
    applyCamera(cameraRequest)
    setCameraRequest(null)
    persistView()
    onDrawing(null)
    scheduleConvert(0)
  }, [cameraRequest, applyCamera, setCameraRequest, persistView, onDrawing, scheduleConvert])

  /** Zoom so the frame fits the map: out when it spills over, and also in when `grow`. */
  const fitFrame = useCallback((grow) => {
    const map = mapRef.current
    if (!map || !readyRef.current) return
    const dz = zoomToFit(map, frameRef.current, map.getCenter().lat)
    if ((dz >= 0 && !grow) || Math.abs(dz) < 0.02) return
    genRef.current += 1
    onDrawing(null)
    map.jumpTo({ zoom: map.getZoom() + dz })
    persistView()
    scheduleConvert()
  }, [onDrawing, persistView, scheduleConvert])

  // Opening Download gives the frame a size if it has none, and brings it into view.
  const groundKey = JSON.stringify(groundKm(frame))
  const wasOpenRef = useRef(false)
  useEffect(() => {
    const justOpened = downloadOpen && !wasOpenRef.current
    wasOpenRef.current = downloadOpen
    if (!downloadOpen) return
    const map = mapRef.current
    if (justOpened && !groundKm(frameRef.current) && map) {
      const el = map.getContainer()
      const kmPerPx = groundWidthAt(map.getCenter().lat, map.getZoom(), 1)
      const w = sig3(el.clientWidth * 0.7 * kmPerPx)
      const h = sig3(el.clientHeight * 0.7 * kmPerPx)
      setFrame({ ...frameRef.current, mode: 'land', size_km: [w, h] })
      return
    }
    fitFrame(!justOpened)
  }, [downloadOpen, groundKey, fitFrame, setFrame])

  /** The frame on the drawing: `rect` in drawing px, `box` in degrees, `paper` in mm. */
  const frameCut = () => {
    const map = mapRef.current
    const v = drawing?.view
    const ground = groundKm(frame)
    if (!map || !v || !ground || !(frame.scale > 0)) return null
    const kmPerPx = groundWidthAt(map.getCenter().lat, map.getZoom(), 1)
    const kx = v.outW / v.cssW
    const ky = v.outH / v.cssH
    const bw = ground[0] / kmPerPx
    const bh = ground[1] / kmPerPx
    const rect = [
      (v.cssW - bw) / 2 * kx, (v.cssH - bh) / 2 * ky,
      (v.cssW + bw) / 2 * kx, (v.cssH + bh) / 2 * ky,
    ]
    const { west, south, east, north } = v.bounds
    const lon = (x) => west + (x / v.outW) * (east - west)
    const lat = (y) => north - (y / v.outH) * (north - south)
    return {
      rect,
      box: [lon(rect[0]), lat(rect[3]), lon(rect[2]), lat(rect[1])],
      paper: ground.map((km) => (km * 1e6) / frame.scale),
    }
  }

  const exportSvg = () => {
    const cut = frameCut()
    return cut ? cropSvg(drawing.svg, cut.rect, cut.paper) : null
  }
  /** How thick the drawing's lines come out on paper at the frame's scale, mm. */
  const lineMm = () => {
    const cut = downloadOpen && drawing?.svg ? frameCut() : null
    const stroke = cut ? strokeOf(drawing.svg) : null
    return stroke ? (stroke * cut.paper[0]) / (cut.rect[2] - cut.rect[0]) : null
  }
  const exportGeojson = () => {
    const cut = frameCut()
    return cut && drawing.geojson ? cropGeojson(drawing.geojson, cut.box) : null
  }

  if (!MAPBOX_TOKEN) {
    return (
      <div className="flex h-full min-w-0 items-center justify-center border-r border-black p-8">
        <div box-="square" className="max-w-md space-y-[1lh]">
          <p className="font-bold">Mapbox token required</p>
          <p className="opacity-70">
            Create a token at{' '}
            <a className="underline" href="https://account.mapbox.com/access-tokens/" target="_blank" rel="noreferrer">
              account.mapbox.com
            </a>
            , then put it in <code>web/.env.local</code>:
          </p>
          <pre className="bg-[var(--background1)] px-[1ch]">
            VITE_MAPBOX_TOKEN=pk.…
          </pre>
          <p className="opacity-70">Restart <code>npm run dev</code> after saving.</p>
        </div>
      </div>
    )
  }

  return (
    <div className="relative h-full w-full min-w-0">
      <div ref={containerRef} className="h-full w-full" />

      {drawing?.svg && !busy && (
        <div
          className="burin-svg-overlay pointer-events-none absolute inset-0 z-[1] bg-white"
          dangerouslySetInnerHTML={{ __html: drawing.svg }}
        />
      )}

      {(busy || loadFailed) && load && (
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <LoadPanel view={load} />
        </div>
      )}

      {downloadOpen && <FrameOverlay frame={frame} map={mapRef.current} />}

      {drawing && !busy && (
        <p className="pointer-events-none absolute top-2 left-2 z-20 bg-white px-[1ch] text-black/60 tabular-nums">
          {fmtCount(drawing.lines)} lines · {fmtCount(drawing.dashes)} dashes · {drawing.ms} ms
          {drawing.threads > 1 && ` · ${drawing.threads} threads`}
          {typeof drawing.gpu === 'string' && ` · ${gpuName(drawing.gpu)}`}
        </p>
      )}

      <div className="safe-fab safe-fab-lg absolute left-4 z-20">
        <DownloadMenu
          open={downloadOpen}
          onOpenChange={setDownloadOpen}
          drawing={drawing}
          frame={frame}
          setFrame={setFrame}
          exportSvg={exportSvg}
          exportGeojson={exportGeojson}
          lineMm={lineMm()}
        />
      </div>
    </div>
  )
}

/* ---------------------------------------------------------------- */
/*  FrameOverlay — the export frame, centred on the map              */
/* ---------------------------------------------------------------- */

function FrameOverlay({ frame, map }) {
  const [, redraw] = useState(0)

  useEffect(() => {
    if (!map) return
    const on = () => redraw((n) => n + 1)
    map.on('move', on)
    map.on('resize', on)
    return () => {
      map.off('move', on)
      map.off('resize', on)
    }
  }, [map])

  const ground = groundKm(frame)
  if (!map || !ground) return null
  const kmPerPx = groundWidthAt(map.getCenter().lat, map.getZoom(), 1)
  const paper = frame.scale > 0 ? ground.map((km) => (km * 1e6) / frame.scale) : null

  return (
    <div className="pointer-events-none absolute inset-0 z-[5] overflow-hidden">
      <div
        className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 border-[1.5px] border-dashed border-black"
        style={{ width: ground[0] / kmPerPx, height: ground[1] / kmPerPx }}
      >
        <span className="absolute bottom-full left-0 mb-1 bg-white px-1 font-mono text-[10px] whitespace-nowrap text-black">
          {sig3(ground[0])} × {sig3(ground[1])} km
          {paper && ` · ${sig3(paper[0])} × ${sig3(paper[1])} mm at 1:${frame.scale}`}
        </span>
      </div>
    </div>
  )
}

/* ---------------------------------------------------------------- */
/*  DownloadMenu — the export frame, with the CLI's names            */
/* ---------------------------------------------------------------- */

/** A number box that commits on Enter or blur; anything not above 0 is refused. */
function NumInput({ value, onCommit, label, className = 'w-[7ch]' }) {
  const [text, setText] = useState(String(value ?? ''))
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    if (!editing) setText(String(value ?? ''))
  }, [value, editing])

  const commit = () => {
    setEditing(false)
    const n = Number(String(text).replace(',', '.'))
    if (Number.isFinite(n) && n > 0) onCommit(n)
    else setText(String(value ?? ''))
  }

  return (
    <input
      type="text"
      inputMode="decimal"
      aria-label={label}
      className={`min-w-0 shrink-0 px-[0.5ch] text-right tabular-nums ${className}`}
      value={text}
      onFocus={() => setEditing(true)}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit()
        else if (e.key === 'Escape') { setText(String(value ?? '')); setEditing(false); e.currentTarget.blur() }
      }}
    />
  )
}

/** One branch of the frame's tree, like the steps under a task in LoadPanel. */
function FlagRow({ flag, last = false, children }) {
  return (
    <div className="flex items-center gap-[1ch]">
      <span className="mr-auto whitespace-pre">{last ? '└' : '├'} {flag}</span>
      {children}
    </div>
  )
}

function DownloadMenu({ open, onOpenChange, drawing, frame, setFrame, exportSvg, exportGeojson, lineMm }) {
  useEffect(() => {
    if (!open) return
    const on = (e) => { if (e.key === 'Escape') onOpenChange(false) }
    globalThis.addEventListener('keydown', on)
    return () => globalThis.removeEventListener('keydown', on)
  }, [open, onOpenChange])

  const save = (text, type, name) => {
    if (!text) return
    const a = document.createElement('a')
    a.href = URL.createObjectURL(new Blob([text], { type }))
    a.download = name
    a.click()
    URL.revokeObjectURL(a.href)
  }

  const ground = groundKm(frame)
  const paper = ground && frame.scale > 0 ? ground.map((km) => (km * 1e6) / frame.scale) : null

  /** Switching keeps the same ground, so the frame on the map doesn't move. */
  const setMode = (mode) => {
    if (mode === frame.mode) return
    if (mode === 'paper' && paper) setFrame({ ...frame, mode, size_mm: paper.map(sig3) })
    else if (mode === 'land' && ground) setFrame({ ...frame, mode, size_km: ground.map(sig3) })
    else setFrame({ ...frame, mode })
  }
  const key = frame.mode === 'land' ? 'size_km' : 'size_mm'
  const size = frame.mode === 'land' ? ground : frame.size_mm
  const setSize = (i, v) => {
    const next = [...(size ?? [v, v])]
    next[i] = v
    setFrame({ ...frame, [key]: next })
  }

  if (!open) {
    return (
      <button type="button" aria-expanded={false} size-="small" onClick={() => onOpenChange(true)}>
        download ▴
      </button>
    )
  }

  return (
    <div className="flex w-[44ch] max-w-[calc(100vw-2rem)] flex-col gap-1 border border-black bg-white p-3 font-mono text-[11px] leading-snug text-black [--font-size:11px] [--line-height:1.375]">
      <button
        type="button"
        aria-expanded
        size-="small"
        variant-="background0"
        className="mb-1 w-full justify-start px-0 hover:no-underline"
        title="Close (Esc)"
        onClick={() => onOpenChange(false)}
      >
        <span className="font-bold">download</span>
        <span className="truncate opacity-50">&nbsp;▸ the frame on the map</span>
        <span className="ml-auto pl-[1ch]">▾</span>
      </button>

      <div role="tablist" className="flex gap-[2ch]">
        {['land', 'paper'].map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={frame.mode === id}
            size-="small"
            variant-="background0"
            className={`px-0 ${frame.mode === id ? '' : 'opacity-50 hover:opacity-100'}`}
            onClick={() => setMode(id)}
          >
            {frame.mode === id ? '●' : '○'} {id === 'land' ? 'ground' : 'paper'}
          </button>
        ))}
      </div>

      <FlagRow flag={frame.mode === 'land' ? '--terrain-size' : '--frame-size'}>
        <NumInput label="Width" value={size?.[0]} onCommit={(v) => setSize(0, v)} />
        <span className="opacity-40">×</span>
        <NumInput label="Height" value={size?.[1]} onCommit={(v) => setSize(1, v)} />
        <span className="w-5 opacity-50">{frame.mode === 'land' ? 'km' : 'mm'}</span>
      </FlagRow>
      <FlagRow flag="--export-scale">
        <span className="opacity-50">1:</span>
        <NumInput label="Scale" className="w-[10ch]" value={frame.scale} onCommit={(v) => setFrame({ ...frame, scale: v })} />
        <span className="w-5" />
      </FlagRow>
      <FlagRow flag="--pen-width" last>
        <NumInput label="Pen width" value={frame.pen_width} onCommit={(v) => setFrame({ ...frame, pen_width: v })} />
        <span className="w-5 opacity-50">mm</span>
      </FlagRow>

      {ground && paper && (
        <p className="pl-[2ch] tabular-nums opacity-50">
          {frame.mode === 'land'
            ? `paper ${sig3(paper[0])} × ${sig3(paper[1])} mm`
            : `ground ${sig3(ground[0])} × ${sig3(ground[1])} km`}
          {lineMm > 0 && ` · lines ${sig3(lineMm)} mm`}
        </p>
      )}

      <div className="mt-1 grid grid-cols-2 gap-[1ch] border-t border-black pt-2">
        <button
          type="button"
          size-="small"
          disabled={!drawing?.svg}
          onClick={() => save(exportSvg(), 'image/svg+xml', 'burin-map.svg')}
        >
          svg
        </button>
        <button
          type="button"
          size-="small"
          variant-="background1"
          disabled={!drawing?.geojson}
          onClick={() => save(exportGeojson(), 'application/geo+json', 'burin-map.geojson')}
        >
          geojson
        </button>
      </div>
      {!drawing?.svg && <p className="opacity-50">waiting for the drawing…</p>}
    </div>
  )
}
