import { useEffect, useRef, useState, useCallback } from 'react'
import * as MapLibre from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import { buildDem } from '../lib/dem'
import { loadBurin, paintDrawing } from '../lib/burin'

// MapLibre is the default (no token). Set VITE_MAPBOX_TOKEN for Mapbox outdoors + globe.
const MAPBOX_TOKEN = import.meta.env.VITE_MAPBOX_TOKEN || ''
const LIBERTY = 'https://tiles.openfreemap.org/styles/liberty'
const SOURCE = 'burin-overlay'
const LAYER = 'burin-overlay-layer'
const SETTLE_MS = 450
const MAX_EDGE = 1200

const PRESETS = [
  { label: 'Twin Peaks', center: [-122.45, 37.75], zoom: 12 },
  { label: 'SF Bay', center: [-122.35, 37.7], zoom: 10 },
  { label: 'Zermatt', center: [7.75, 46.02], zoom: 12 },
  { label: 'Bern', center: [7.45, 46.95], zoom: 11 },
  { label: 'Mont Blanc', center: [6.86, 45.83], zoom: 11 },
]

async function createMap(container) {
  if (MAPBOX_TOKEN) {
    const mapboxgl = (await import('mapbox-gl')).default
    await import('mapbox-gl/dist/mapbox-gl.css')
    mapboxgl.accessToken = MAPBOX_TOKEN
    const map = new mapboxgl.Map({
      container,
      style: 'mapbox://styles/mapbox/outdoors-v12',
      center: [-122.45, 37.75],
      zoom: 12,
      projection: 'globe',
      attributionControl: true,
    })
    map.addControl(new mapboxgl.NavigationControl({ visualizePitch: true }), 'top-left')
    map.addControl(new mapboxgl.ScaleControl({ maxWidth: 120 }), 'bottom-left')
    return { map, NavigationControl: mapboxgl.NavigationControl, kind: 'mapbox' }
  }

  const map = new MapLibre.Map({
    container,
    style: LIBERTY,
    center: [-122.45, 37.75],
    zoom: 12,
    attributionControl: true,
  })
  try {
    map.setProjection({ type: 'globe' })
  } catch {
    /* older builds */
  }
  map.addControl(new MapLibre.NavigationControl({ visualizePitch: true }), 'top-left')
  map.addControl(new MapLibre.ScaleControl({ maxWidth: 120 }), 'bottom-left')
  return { map, kind: 'maplibre' }
}

/**
 * @param {{
 *   style: Record<string, number>,
 *   onStatus: (s: { phase: string, detail?: string }) => void,
 *   onDrawing: (d: { svg: string, lines: number, dashes: number, ms: number } | null) => void,
 * }} props
 */
export default function MapView({ style, onStatus, onDrawing }) {
  const containerRef = useRef(null)
  const mapRef = useRef(null)
  const canvasRef = useRef(null)
  const genRef = useRef(0)
  const movingRef = useRef(false)
  const settleTimer = useRef(0)
  const styleRef = useRef(style)
  styleRef.current = style

  const [busy, setBusy] = useState(false)
  const [busyMsg, setBusyMsg] = useState('')
  const [overlayOn, setOverlayOn] = useState(false)
  const [engine, setEngine] = useState(MAPBOX_TOKEN ? 'mapbox' : 'maplibre')

  const fadeOut = useCallback(() => {
    const map = mapRef.current
    if (!map?.getLayer(LAYER)) return
    map.setPaintProperty(LAYER, 'raster-opacity', 0)
    setOverlayOn(false)
  }, [])

  const ensureLayer = useCallback((map, url, coordinates) => {
    if (map.getSource(SOURCE)) {
      map.getSource(SOURCE).updateImage({ url, coordinates })
    } else {
      map.addSource(SOURCE, { type: 'image', url, coordinates })
      map.addLayer({
        id: LAYER,
        type: 'raster',
        source: SOURCE,
        paint: { 'raster-opacity': 0, 'raster-fade-duration': 0 },
      })
    }
  }, [])

  const convert = useCallback(async () => {
    const map = mapRef.current
    if (!map || movingRef.current) return
    const my = ++genRef.current
    const canvas = canvasRef.current
    const b = map.getBounds()
    const bounds = {
      west: b.getWest(),
      south: b.getSouth(),
      east: b.getEast(),
      north: b.getNorth(),
    }
    const coordinates = [
      [bounds.west, bounds.north],
      [bounds.east, bounds.north],
      [bounds.east, bounds.south],
      [bounds.west, bounds.south],
    ]

    const size = map.getCanvas()
    const scale = Math.min(1, MAX_EDGE / Math.max(size.width, size.height))
    const outW = Math.max(64, Math.round(size.width * scale))
    const outH = Math.max(64, Math.round(size.height * scale))

    setBusy(true)
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

      progress('Drawing lines…')
      const result = burin.draw(dem.elev, dem.width, dem.height, dem.pxM, styleRef.current)
      if (my !== genRef.current || movingRef.current) return

      paintDrawing(canvas, result, styleRef.current)
      const url = canvas.toDataURL('image/png')
      ensureLayer(map, url, coordinates)

      requestAnimationFrame(() => {
        if (my !== genRef.current || movingRef.current) return
        if (map.getLayer(LAYER)) {
          map.setPaintProperty(LAYER, 'raster-opacity', 1)
          setOverlayOn(true)
        }
      })

      onDrawing({
        svg: result.svg,
        lines: result.lines,
        dashes: result.dashes,
        ms: Math.round(result.ms),
      })
      onStatus({
        phase: 'ready',
        detail: `${result.lines.toLocaleString()} lines · ${result.dashes.toLocaleString()} dashes · ${Math.round(result.ms)} ms`,
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
  }, [ensureLayer, onDrawing, onStatus])

  const onMoveStart = useCallback(() => {
    movingRef.current = true
    clearTimeout(settleTimer.current)
    genRef.current += 1
    setBusy(false)
    setBusyMsg('')
    fadeOut()
    onStatus({ phase: 'exploring', detail: 'Move the map · drawing pauses' })
    onDrawing(null)
  }, [fadeOut, onDrawing, onStatus])

  const onMoveEnd = useCallback(() => {
    movingRef.current = false
    clearTimeout(settleTimer.current)
    settleTimer.current = setTimeout(() => {
      if (!movingRef.current) convert()
    }, SETTLE_MS)
  }, [convert])

  useEffect(() => {
    let cancelled = false
    let map

    createMap(containerRef.current).then((created) => {
      if (cancelled) {
        created.map.remove()
        return
      }
      map = created.map
      mapRef.current = map
      setEngine(created.kind)
      map.on('load', () => {
        onStatus({ phase: 'exploring', detail: 'Ready' })
        onMoveEnd()
      })
      map.on('movestart', onMoveStart)
      map.on('moveend', onMoveEnd)
      map.on('pitchstart', onMoveStart)
      map.on('rotatestart', onMoveStart)
    })

    return () => {
      cancelled = true
      clearTimeout(settleTimer.current)
      map?.remove()
      mapRef.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    if (!mapRef.current || movingRef.current) return
    clearTimeout(settleTimer.current)
    fadeOut()
    settleTimer.current = setTimeout(() => convert(), 200)
  }, [style, convert, fadeOut])

  const flyTo = (p) => {
    mapRef.current?.flyTo({ center: p.center, zoom: p.zoom, essential: true })
  }

  return (
    <div className="relative h-full w-full">
      <div ref={containerRef} className="h-full w-full" />
      <canvas ref={canvasRef} className="hidden" />

      {busy && (
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <div className="flex flex-col items-center gap-3 rounded-box bg-base-100/90 px-6 py-5 shadow-xl backdrop-blur-sm">
            <span className="loading loading-spinner loading-lg text-primary" />
            <p className="text-sm font-medium">{busyMsg || 'Converting…'}</p>
          </div>
        </div>
      )}

      <div className="absolute bottom-6 left-1/2 z-10 flex max-w-[min(100%-2rem,40rem)] -translate-x-1/2 flex-wrap justify-center gap-2">
        {PRESETS.map((p) => (
          <button
            key={p.label}
            type="button"
            className="btn btn-sm border-0 bg-base-100/90 shadow backdrop-blur-sm"
            onClick={() => flyTo(p)}
          >
            {p.label}
          </button>
        ))}
      </div>

      {overlayOn && !busy && (
        <div className="pointer-events-none absolute right-4 top-4 z-10">
          <span className="badge badge-primary badge-outline bg-base-100/80 backdrop-blur-sm">
            burin overlay · {engine}
          </span>
        </div>
      )}
    </div>
  )
}
