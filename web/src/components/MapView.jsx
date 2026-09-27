import { useEffect, useRef, useState, useCallback } from 'react'
import { useAtom } from 'jotai'
import mapboxgl from 'mapbox-gl'
import 'mapbox-gl/dist/mapbox-gl.css'
import { buildDem } from '../lib/dem'
import { loadBurin, drawBatches } from '../lib/burin'
import { mapViewAtom } from '../state'

const MAPBOX_TOKEN = import.meta.env.VITE_MAPBOX_TOKEN || ''
const SETTLE_MS = 90
const MAX_EDGE = 3072

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

/** User gesture only — Mapbox also fires move* while loading tiles. */
function isUserGesture(e) {
  return !!(e && e.originalEvent)
}

/**
 * @param {{
 *   batches: Array<{ angle: number, style: Record<string, number> }>,
 *   onStatus: (s: { phase: string, detail?: string }) => void,
 *   onDrawing: (d: { svg: string, lines: number, dashes: number, ms: number } | null) => void,
 *   drawing: { svg: string } | null,
 * }} props
 */
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
  const mapViewRef = useRef(null)
  // Read once for initial camera; later updates go through setMapView.
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

  const convert = useCallback(async () => {
    const map = mapRef.current
    if (!map || movingRef.current) return
    if (map.getContainer().clientWidth < 2 || map.getContainer().clientHeight < 2) {
      // Layout not ready yet — try again shortly.
      clearTimeout(settleTimer.current)
      settleTimer.current = setTimeout(() => convertRef.current(), 100)
      return
    }

    const my = ++genRef.current
    const list = Array.isArray(batchesRef.current) ? batchesRef.current : []
    if (list.length === 0) {
      // Nothing to draw until the sidebar has initialised its batches.
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
    setBusyMsg('Computing…')
    onStatus({ phase: 'working', detail: 'Computing…' })
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

      progress('Computing…')
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))
      if (my !== genRef.current || movingRef.current) return

      const result = drawBatches(burin, dem.elev, dem.width, dem.height, dem.pxM, batchesRef.current)
      if (my !== genRef.current || movingRef.current) return

      onDrawing({
        svg: result.svg,
        lines: result.lines,
        dashes: result.dashes,
        ms: Math.round(result.ms),
      })
      onStatus({
        phase: 'ready',
        detail: dem.fetched ? `Downloaded ${dem.fetched} tiles` : undefined,
      })
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
    setMapView({
      center: [c.lng, c.lat],
      zoom: map.getZoom(),
    })
  }, [setMapView])

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
        detail: 'Set VITE_MAPBOX_TOKEN in web/.env.local (from account.mapbox.com)',
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
      onStatus({ phase: 'exploring', detail: 'Mapbox · move freely' })
    }

    const onMoveEnd = (e) => {
      if (!readyRef.current) return
      // After a user gesture, always settle; also accept moveend without originalEvent
      // that follows a user movestart (movingRef still true).
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
      readyRef.current = true
      movingRef.current = false
      onStatus({ phase: 'working', detail: 'Computing…' })
      // Draw immediately — don't wait for 'idle' (tiles can keep it deferred).
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
    // Skip the Sidebar's initial style hydrate so it doesn't cancel the first draw.
    if (skipStyleOnceRef.current) {
      skipStyleOnceRef.current = false
      batchesRef.current = batches
      return
    }
    batchesRef.current = batches
    onDrawing(null)
    scheduleConvert(120)
  }, [batches, scheduleConvert, onDrawing])

  const download = () => {
    if (!drawing?.svg) return
    const a = document.createElement('a')
    a.href = URL.createObjectURL(new Blob([drawing.svg], { type: 'image/svg+xml' }))
    a.download = 'BurinMap.svg'
    a.click()
    URL.revokeObjectURL(a.href)
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
          className="burin-svg-overlay pointer-events-none absolute inset-0 z-[1] bg-white"
          dangerouslySetInnerHTML={{ __html: drawing.svg }}
        />
      )}

      {busy && (
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <div className="flex w-56 flex-col items-center gap-3 border border-base-content bg-base-100 px-6 py-5">
            <span className="loading loading-spinner loading-lg" />
            <p className="w-full truncate text-center font-mono text-sm">
              {busyMsg || 'Computing…'}
            </p>
          </div>
        </div>
      )}

      <button
        type="button"
        className="btn btn-sm safe-fab safe-fab-lg absolute left-4 z-20 border border-base-content bg-base-100"
        disabled={!drawing?.svg}
        onClick={download}
      >
        Download SVG
      </button>
    </div>
  )
}
