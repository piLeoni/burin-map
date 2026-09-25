import { useEffect, useRef, useState, useCallback } from 'react'
import mapboxgl from 'mapbox-gl'
import 'mapbox-gl/dist/mapbox-gl.css'
import { buildDem } from '../lib/dem'
import { loadBurin, paintDrawing } from '../lib/burin'

const MAPBOX_TOKEN = import.meta.env.VITE_MAPBOX_TOKEN || ''
const SOURCE = 'burin-overlay'
const LAYER = 'burin-overlay-layer'
const SETTLE_MS = 450
const MAX_EDGE = 1200

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

  /** Hide burin so the Mapbox basemap is fully visible. */
  const showBasemap = useCallback(() => {
    const map = mapRef.current
    if (!map?.getLayer(LAYER)) return
    map.setPaintProperty(LAYER, 'raster-opacity', 0)
    map.setLayoutProperty(LAYER, 'visibility', 'none')
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
        layout: { visibility: 'none' },
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

    showBasemap()
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
        if (!map.getLayer(LAYER)) return
        map.setLayoutProperty(LAYER, 'visibility', 'visible')
        map.setPaintProperty(LAYER, 'raster-opacity', 1)
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
      showBasemap()
    } finally {
      if (my === genRef.current) {
        setBusy(false)
        setBusyMsg('')
      }
    }
  }, [ensureLayer, onDrawing, onStatus, showBasemap])

  const onMoveStart = useCallback(() => {
    movingRef.current = true
    clearTimeout(settleTimer.current)
    genRef.current += 1
    setBusy(false)
    setBusyMsg('')
    showBasemap()
    onStatus({ phase: 'exploring', detail: 'Mapbox · move freely' })
    onDrawing(null)
  }, [showBasemap, onDrawing, onStatus])

  const onMoveEnd = useCallback(() => {
    movingRef.current = false
    clearTimeout(settleTimer.current)
    settleTimer.current = setTimeout(() => {
      if (!movingRef.current) convert()
    }, SETTLE_MS)
  }, [convert])

  useEffect(() => {
    if (!MAPBOX_TOKEN) {
      onStatus({
        phase: 'error',
        detail: 'Set VITE_MAPBOX_TOKEN in web/.env.local (from account.mapbox.com)',
      })
      return
    }

    mapboxgl.accessToken = MAPBOX_TOKEN
    const map = new mapboxgl.Map({
      container: containerRef.current,
      style: 'mapbox://styles/mapbox/light-v11',
      center: [-122.45, 37.75],
      zoom: 12,
      projection: 'globe',
      attributionControl: true,
    })
    map.addControl(new mapboxgl.NavigationControl({ visualizePitch: true }), 'top-left')
    map.addControl(new mapboxgl.ScaleControl({ maxWidth: 120 }), 'bottom-left')
    mapRef.current = map

    map.on('load', () => {
      onStatus({ phase: 'exploring', detail: 'Mapbox ready' })
      onMoveEnd()
    })
    map.on('movestart', onMoveStart)
    map.on('moveend', onMoveEnd)
    map.on('pitchstart', onMoveStart)
    map.on('rotatestart', onMoveStart)

    return () => {
      clearTimeout(settleTimer.current)
      map.remove()
      mapRef.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    if (!mapRef.current || movingRef.current) return
    clearTimeout(settleTimer.current)
    showBasemap()
    settleTimer.current = setTimeout(() => convert(), 200)
  }, [style, convert, showBasemap])

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
      <canvas ref={canvasRef} className="hidden" />

      {busy && (
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <div className="flex flex-col items-center gap-3 border border-base-content bg-base-100 px-6 py-5">
            <span className="loading loading-spinner loading-lg" />
            <p className="font-mono text-sm">{busyMsg || 'Converting…'}</p>
          </div>
        </div>
      )}
    </div>
  )
}
