import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'

const MAX_ZOOM = 12

function clampCenter(c, z) {
  const half = 0.5 / z
  return Math.min(1 - half, Math.max(half, c))
}

/**
 * Side-by-side SVG tiles of the same area, zoomed and panned together.
 * ⌘/Ctrl + wheel or pinch zooms, drag pans, double-click zooms in.
 * The SVG is re-laid-out (not CSS-scaled), so lines stay sharp at any zoom.
 */
export default function ZoomFigure({ w, h, tiles, columns, compact = false, open }) {
  const [view, setView] = useState({ z: 1, cx: 0.5, cy: 0.5 })
  const [tileW, setTileW] = useState(0)
  const gridRef = useRef(null)
  const drag = useRef(null)

  useLayoutEffect(() => {
    const el = gridRef.current?.querySelector('.zf-frame')
    if (!el) return undefined
    const ro = new ResizeObserver(() => setTileW(el.clientWidth))
    ro.observe(el)
    setTileW(el.clientWidth)
    return () => ro.disconnect()
  }, [])

  const zoomAt = useCallback((factor, fx = 0.5, fy = 0.5) => {
    setView((v) => {
      const z = Math.min(MAX_ZOOM, Math.max(1, v.z * factor))
      // Keep the point under the cursor (fx, fy in frame fractions) fixed.
      const px = v.cx + (fx - 0.5) / v.z
      const py = v.cy + (fy - 0.5) / v.z
      return { z, cx: clampCenter(px - (fx - 0.5) / z, z), cy: clampCenter(py - (fy - 0.5) / z, z) }
    })
  }, [])

  useEffect(() => {
    const grid = gridRef.current
    if (!grid) return undefined
    const onWheel = (e) => {
      if (!(e.ctrlKey || e.metaKey)) return
      const frame = e.target.closest('.zf-frame')
      if (!frame) return
      e.preventDefault()
      const r = frame.getBoundingClientRect()
      zoomAt(Math.exp(-e.deltaY * 0.002), (e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height)
    }
    grid.addEventListener('wheel', onWheel, { passive: false })
    return () => grid.removeEventListener('wheel', onWheel)
  }, [zoomAt])

  const onPointerDown = (e) => {
    if (view.z <= 1) return
    e.currentTarget.setPointerCapture(e.pointerId)
    const r = e.currentTarget.getBoundingClientRect()
    drag.current = { x: e.clientX, y: e.clientY, cx: view.cx, cy: view.cy, fw: r.width, fh: r.height }
  }
  const onPointerMove = (e) => {
    const d = drag.current
    if (!d) return
    setView((v) => ({
      ...v,
      cx: clampCenter(d.cx - (e.clientX - d.x) / (d.fw * v.z), v.z),
      cy: clampCenter(d.cy - (e.clientY - d.y) / (d.fh * v.z), v.z),
    }))
  }
  const onPointerUp = () => {
    drag.current = null
  }
  const onDoubleClick = (e) => {
    const r = e.currentTarget.getBoundingClientRect()
    zoomAt(2, (e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height)
  }

  const frameH = tileW * (h / w)
  const imgW = tileW * view.z
  const imgH = frameH * view.z
  const left = tileW / 2 - view.cx * imgW
  const top = frameH / 2 - view.cy * imgH
  const native = tileW > 0 ? w / tileW : 1
  const cols = columns ?? tiles.length

  const controls = (
    <div className="zf-controls">
      <button type="button" size-="small" onClick={() => zoomAt(1 / 1.6)} disabled={view.z <= 1} aria-label="zoom out">
        −
      </button>
      <button type="button" size-="small" onClick={() => zoomAt(1.6)} disabled={view.z >= MAX_ZOOM} aria-label="zoom in">
        +
      </button>
      <button
        type="button"
        size-="small"
        onClick={() => setView({ z: Math.min(MAX_ZOOM, Math.max(1, native)), cx: view.cx, cy: view.cy })}
      >
        1:1
      </button>
      <button type="button" size-="small" onClick={() => setView({ z: 1, cx: 0.5, cy: 0.5 })} disabled={view.z === 1}>
        fit
      </button>
      <span className="zf-zoom">{Math.round(view.z * 100)}%</span>
      {compact ? null : (
        <span className="zf-hint">⌘/Ctrl + scroll or pinch to zoom · drag to pan · double-click to zoom in</span>
      )}
      {open ? (
        <a className="zf-open" href={open} target="_blank" rel="noreferrer">
          open SVG ↗
        </a>
      ) : null}
    </div>
  )

  return (
    <figure className={`zf${compact ? ' zf-compact' : ''}`}>
      {controls}
      <div className="zf-grid" ref={gridRef} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {tiles.map((t) => (
          <div key={t.src} className="zf-tile">
            <div
              className={`zf-frame${view.z > 1 ? ' zf-pannable' : ''}`}
              style={{ aspectRatio: `${w} / ${h}` }}
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={onPointerUp}
              onDoubleClick={onDoubleClick}
            >
              {tileW > 0 ? (
                <img
                  src={t.src}
                  alt={t.label ?? ''}
                  draggable={false}
                  loading="lazy"
                  style={{ width: imgW, height: imgH, left, top }}
                />
              ) : null}
            </div>
            {t.label || t.code ? (
              <figcaption>
                {t.label ? <span className="zf-label">{t.label}</span> : null}
                {t.code ? <code className="zf-code">{t.code}</code> : null}
              </figcaption>
            ) : null}
          </div>
        ))}
      </div>
    </figure>
  )
}
