import { useEffect, useRef, useState, useCallback } from 'react'
import { useAtom } from 'jotai'
import mapboxgl from 'mapbox-gl'
import 'mapbox-gl/dist/mapbox-gl.css'
import { buildDem } from '../lib/dem'
import { loadBurin, drawBatches } from '../lib/burin'
import { FIT_PAD, MAX_EDGE, groundKm, groundWidthAt, sig3 } from '../lib/frame'
import { cropGeojson, cropSvg } from '../lib/crop'
import { mapViewAtom, cameraRequestAtom, frameAtom } from '../state'

const MAPBOX_TOKEN = import.meta.env.VITE_MAPBOX_TOKEN || ''
const SETTLE_MS = 90

/* ---------------------------------------------------------------- */
/*  MapView                                                          */
/* ---------------------------------------------------------------- */

function setSymbolVisibility(map, visible) {
  if (!map?.isStyleLoaded()) return
  const style = map.getStyle()
  if (!style?.layers) return
  for (const layer of style.layers) {
    if (layer.type === 'symbol') {
      try {
        map.setLayoutProperty(layer.id, 'visibility', visible ? 'visible' : 'none')
      } catch {
        /* layer may be gone after a style swap */
      }
    }
  }
}

function isUserGesture(e) {
  return !!(e && e.originalEvent)
}

export default function MapView({ batches, onStatus, onDrawing, drawing }) {
  const containerRef = useRef(null)
  const mapRef = useRef(null)
  const genRef = useRef(0)
  const movingRef = useRef(false)
  const readyRef = useRef(false)
  const skipStyleOnceRef = useRef(true)
  const settleTimer = useRef(0)
  const batchesRef = useRef(batches)
  batchesRef.current = batches
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
  const [busyMsg, setBusyMsg] = useState('')
  const [downloadOpen, setDownloadOpen] = useState(false)
  const svgOverlayRef = useRef(null)
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

    setBusy(true)
    setBusyMsg('Drawing…')
    onStatus({ phase: 'working', detail: 'Drawing…' })
    const progress = (msg) => {
      setBusyMsg(msg)
      onStatus({ phase: 'working', detail: msg })
    }

    try {
      progress('Loading engine…')
      const burin = await loadBurin()
      if (my !== genRef.current || movingRef.current) return

      const dem = await buildDem(bounds, outW, outH, progress)
      if (my !== genRef.current || movingRef.current) return

      progress('Drawing…')
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))
      if (my !== genRef.current || movingRef.current) return

      const result = drawBatches(burin, dem.elev, dem.width, dem.height, dem.pxM, batchesRef.current, {
        nw_lon: bounds.west,
        nw_lat: bounds.north,
        deg_per_px_lon: (bounds.east - bounds.west) / outW,
        deg_per_px_lat: (bounds.north - bounds.south) / outH,
      })
      if (my !== genRef.current || movingRef.current) return

      onDrawing({
        svg: result.svg,
        geojson: result.geojson,
        lines: result.lines,
        dashes: result.dashes,
        ms: Math.round(result.ms),
        view: { cssW, cssH, outW, outH, bounds },
      })
      onStatus({ phase: 'ready' })
    } catch (e) {
      console.error(e)
      onStatus({ phase: 'error', detail: String(e.message || e) })
      onDrawing(null)
    } finally {
      if (my === genRef.current) {
        setBusy(false)
        setBusyMsg('')
      }
    }
  }, [onDrawing, onStatus])

  convertRef.current = convert

  const persistView = useCallback(() => {
    const map = mapRef.current
    if (!map) return
    const c = map.getCenter()
    setMapView({ center: [c.lng, c.lat], zoom: map.getZoom() })
  }, [setMapView])

  const applyCamera = useCallback((cam) => {
    const map = mapRef.current
    if (!map || !cam?.center) return
    map.jumpTo({ center: cam.center })
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
      setBusyMsg('')
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

    map.on('style.load', () => {
      setSymbolVisibility(map, false)
    })

    map.on('load', () => {
      setSymbolVisibility(map, false)
      map.resize()
      if (cameraRequestRef.current) {
        applyCamera(cameraRequestRef.current)
        setCameraRequest(null)
      }
      readyRef.current = true
      movingRef.current = false
      persistView()
      onStatus({ phase: 'working', detail: 'Drawing…' })
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
    onDrawing(null)
    scheduleConvert(120)
  }, [batches, scheduleConvert, onDrawing])

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
    const g = groundKm(frameRef.current)
    if (!map || !g || !readyRef.current) return
    const el = map.getContainer()
    const kmPerPx = groundWidthAt(map.getCenter().lat, map.getZoom(), 1)
    const k = Math.min(
      (el.clientWidth - 2 * FIT_PAD) / (g[0] / kmPerPx),
      (el.clientHeight - 2 * FIT_PAD) / (g[1] / kmPerPx),
    )
    if (!(k > 0) || (k >= 1 && !grow) || Math.abs(Math.log2(k)) < 0.02) return
    genRef.current += 1
    onDrawing(null)
    map.jumpTo({ zoom: map.getZoom() + Math.log2(k) })
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

  // While Download is open the lines on screen take the pen width they will have on paper.
  useEffect(() => {
    const g = svgOverlayRef.current?.querySelector('svg > g[stroke-width]')
    const map = mapRef.current
    if (!g || !map || !drawing?.view) return
    if (!g.dataset.stroke) g.dataset.stroke = g.getAttribute('stroke-width')
    const ground = groundKm(frame)
    if (!downloadOpen || !ground || !(frame.scale > 0) || !(frame.pen_width > 0)) {
      g.setAttribute('stroke-width', g.dataset.stroke)
      return
    }
    const kmPerCssPx = groundWidthAt(map.getCenter().lat, map.getZoom(), 1)
    const pxPerKm = drawing.view.outW / drawing.view.cssW / kmPerCssPx
    const mmPerPx = 1e6 / frame.scale / pxPerKm
    g.setAttribute('stroke-width', (frame.pen_width / mmPerPx).toPrecision(3))
  }, [downloadOpen, frame, drawing])

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
    return cut ? cropSvg(drawing.svg, cut.rect, cut.paper, frame.pen_width) : null
  }
  const exportGeojson = () => {
    const cut = frameCut()
    return cut && drawing.geojson ? cropGeojson(drawing.geojson, cut.box) : null
  }

  if (!MAPBOX_TOKEN) {
    return (
      <div className="flex h-full min-w-0 items-center justify-center border-r border-base-content p-8">
        <div className="max-w-md space-y-3 font-mono text-sm">
          <p className="font-bold">Mapbox token required</p>
          <p className="opacity-70">
            Create a token at{' '}
            <a className="link" href="https://account.mapbox.com/access-tokens/" target="_blank" rel="noreferrer">
              account.mapbox.com
            </a>
            , then put it in <code>web/.env.local</code>:
          </p>
          <pre className="border border-base-content bg-base-200 p-3 text-xs">
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
          ref={svgOverlayRef}
          className="burin-svg-overlay pointer-events-none absolute inset-0 z-[1] bg-white"
          dangerouslySetInnerHTML={{ __html: drawing.svg }}
        />
      )}

      {busy && (
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <div className="flex w-56 flex-col items-center gap-3 border border-base-content bg-base-100 px-6 py-5">
            <span className="loading loading-spinner loading-lg" />
            <p className="w-full truncate text-center font-mono text-sm">
              {busyMsg || 'Drawing…'}
            </p>
          </div>
        </div>
      )}

      {downloadOpen && <FrameOverlay frame={frame} map={mapRef.current} />}

      <div className="absolute bottom-4 left-4 z-20">
        <DownloadMenu
          open={downloadOpen}
          onOpenChange={setDownloadOpen}
          drawing={drawing}
          frame={frame}
          setFrame={setFrame}
          exportSvg={exportSvg}
          exportGeojson={exportGeojson}
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
function NumInput({ value, onCommit, label, className = 'w-14' }) {
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
      className={`input input-xs h-6 shrink-0 px-1.5 text-right font-mono text-[11px] tabular-nums ${className}`}
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

function FlagRow({ flag, children }) {
  return (
    <div className="flex items-center gap-1 font-mono text-[11px]">
      <span className="mr-auto">{flag}</span>
      {children}
    </div>
  )
}

function DownloadMenu({ open, onOpenChange, drawing, frame, setFrame, exportSvg, exportGeojson }) {
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

  return (
    <div className="flex flex-col-reverse items-start gap-2">
      <button
        type="button"
        aria-expanded={open}
        className="btn btn-sm border-base-content bg-base-content text-base-100 shadow-md hover:bg-base-content/85"
        onClick={() => onOpenChange(!open)}
      >
        Download
        <svg
          viewBox="0 0 20 20"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
          className={`ml-0.5 h-3.5 w-3.5 transition-transform duration-150 ${open ? '' : 'rotate-180'}`}
        >
          <path d="M5 7.5l5 5 5-5" />
        </svg>
      </button>

      {open && (
        <div className="flex w-72 flex-col gap-2 rounded-lg border border-base-content/25 bg-base-100 p-3 shadow-md">
          <div role="tablist" className="join w-full">
            {[['land', '--terrain-size'], ['paper', '--frame-size']].map(([id, flag]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={frame.mode === id}
                className={`btn btn-xs join-item flex-1 font-mono text-[10px] normal-case ${frame.mode === id ? 'border-base-content bg-base-content text-base-100' : 'border-base-content/25 bg-base-100'}`}
                onClick={() => setMode(id)}
              >
                {flag}
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
            <NumInput label="Scale" className="w-20" value={frame.scale} onCommit={(v) => setFrame({ ...frame, scale: v })} />
            <span className="w-5" />
          </FlagRow>
          <FlagRow flag="--pen-width">
            <NumInput label="Pen width" value={frame.pen_width} onCommit={(v) => setFrame({ ...frame, pen_width: v })} />
            <span className="w-5 opacity-50">mm</span>
          </FlagRow>

          {ground && paper && (
            <p className="font-mono text-[10px] tabular-nums opacity-50">
              {frame.mode === 'land'
                ? `paper ${sig3(paper[0])} × ${sig3(paper[1])} mm`
                : `ground ${sig3(ground[0])} × ${sig3(ground[1])} km`}
            </p>
          )}

          <div className="grid grid-cols-2 gap-1.5 border-t border-base-content/15 pt-2">
            <button
              type="button"
              className="btn btn-xs btn-neutral"
              disabled={!drawing?.svg}
              onClick={() => save(exportSvg(), 'image/svg+xml', 'burin-map.svg')}
            >
              SVG
            </button>
            <button
              type="button"
              className="btn btn-xs border-base-content/25"
              disabled={!drawing?.geojson}
              onClick={() => save(exportGeojson(), 'application/geo+json', 'burin-map.geojson')}
            >
              GeoJSON
            </button>
          </div>
          {!drawing?.svg && <p className="font-mono text-[10px] opacity-50">Waiting for the drawing…</p>}
        </div>
      )}
    </div>
  )
}
