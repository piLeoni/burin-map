import { useEffect, useState, useCallback } from 'react'
import { useAtom } from 'jotai'
import { loadBurin } from '../lib/burin'
import { styleAtom } from '../state'

const RANGES = {
  spacing: [1, 12],
  spacing_steep: [0.2, 1],
  spacing_dark: [0.2, 1],
  spacing_gamma: [0.3, 3],
  test_ratio: [0.2, 1],
  fill_ratio: [0.5, 1],
  step: [0.5, 2],
  min_len: [1, 20],
  flow_blur: [1, 30],
  shade_blur: [0.5, 8],
  broad_blur: [2, 40],
  broad_weight: [0, 1],
  wobble: [0, 60],
  wobble_size: [2, 40],
  azimuth: [0, 360],
  altitude: [5, 85],
  z_exag: [1, 12],
  flat_level: [0, 0.6],
  shade_gain: [0.5, 8],
  dark_gamma: [0.3, 2],
  lit_gain: [0.5, 4],
  lit_floor: [0, 0.3],
  period: [1, 5],
  duty_min: [0, 0.5],
  duty_max: [0.5, 1],
  cutoff: [0, 0.15],
  jitter: [0, 1],
  breaks: [0, 0.5],
  stroke: [0.4, 3],
  seed: [0, 100],
}

/** Integer-only parameters. */
const INT_PARAMS = new Set([
  'seed',
  'azimuth',
  'altitude',
  'wobble',
  'wobble_size',
  'min_len',
  'flow_blur',
  'broad_blur',
])

function isIntParam(name) {
  return INT_PARAMS.has(name)
}

function clamp(name, n) {
  const [lo, hi] = RANGES[name] || [n, n]
  return Math.min(hi, Math.max(lo, n))
}

/** Max one decimal for floats; whole numbers for int params. */
function quantize(name, raw) {
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return null
  const c = clamp(name, n)
  if (isIntParam(name)) return Math.round(c)
  return Math.round(c * 10) / 10
}

function fmt(name, v) {
  if (isIntParam(name)) return String(Math.round(v))
  const q = Math.round(v * 10) / 10
  return Number.isInteger(q) ? String(q) : q.toFixed(1)
}

function fmtCount(n) {
  return Number(n).toLocaleString()
}

/** Stable slider — must live outside Sidebar or drag remounts mid-gesture. */
function StyleSlider({ p, value, onDrag, onCommit }) {
  const int = isIntParam(p.name)
  const [lo, hi] = RANGES[p.name] || [0, Math.max(1, p.default * 2)]
  const step = int ? 1 : 0.1
  const [text, setText] = useState(() => fmt(p.name, value))
  const [editing, setEditing] = useState(false)

  // Keep the input in sync unless the user is typing.
  useEffect(() => {
    if (!editing) setText(fmt(p.name, value))
  }, [value, p.name, editing])

  const commitText = () => {
    setEditing(false)
    const q = quantize(p.name, text)
    if (q == null) {
      setText(fmt(p.name, value))
      return
    }
    setText(fmt(p.name, q))
    onCommit(p.name, q)
  }

  return (
    <div className="w-full min-w-0">
      <div className="flex w-full min-w-0 items-center justify-between gap-2">
        <span className="min-w-0 truncate font-mono text-xs">{p.name}</span>
        <input
          type="text"
          inputMode={int ? 'numeric' : 'decimal'}
          className="input input-xs w-14 shrink-0 text-right font-mono"
          value={text}
          onFocus={() => setEditing(true)}
          onChange={(e) => setText(e.target.value)}
          onBlur={commitText}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.currentTarget.blur()
            } else if (e.key === 'Escape') {
              setText(fmt(p.name, value))
              setEditing(false)
              e.currentTarget.blur()
            }
          }}
        />
      </div>
      <input
        type="range"
        className="range range-xs w-full"
        min={lo}
        max={hi}
        step={step}
        value={value}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId)
        }}
        onChange={(e) => {
          const q = quantize(p.name, e.target.value)
          if (q != null) onDrag(p.name, q)
        }}
        onPointerUp={(e) => {
          const q = quantize(p.name, e.currentTarget.value)
          if (q != null) onCommit(p.name, q)
        }}
        onKeyUp={(e) => {
          if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'Home' || e.key === 'End') {
            const q = quantize(p.name, e.currentTarget.value)
            if (q != null) onCommit(p.name, q)
          }
        }}
      />
      <p className="mt-0.5 w-full text-[10px] leading-snug wrap-break-word opacity-45">{p.help}</p>
    </div>
  )
}

/**
 * @param {{
 *   open: boolean,
 *   onClose: () => void,
 *   status: { phase: string, detail?: string },
 *   drawing: { lines: number, dashes: number, ms: number } | null,
 * }} props
 */
export default function Sidebar({ open, onClose, status, drawing }) {
  const [params, setParams] = useState([])
  const [style, setStyle] = useAtom(styleAtom)
  /** Local while dragging — style atom updates only on release. */
  const [draft, setDraft] = useState({})

  useEffect(() => {
    loadBurin().then((m) => {
      const list = m.style_params()
      setParams(list)
      const defaults = {}
      for (const p of list) defaults[p.name] = quantize(p.name, p.default) ?? p.default
      const merged = { ...defaults }
      for (const [k, v] of Object.entries(style)) {
        if (k in defaults && typeof v === 'number' && Number.isFinite(v)) {
          merged[k] = quantize(k, v) ?? defaults[k]
        }
      }
      setDraft(merged)
      setStyle(merged)
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const onDrag = useCallback((name, value) => {
    setDraft((d) => ({ ...d, [name]: value }))
  }, [])

  const onCommit = useCallback(
    (name, value) => {
      const q = quantize(name, value)
      if (q == null) return
      setDraft((d) => {
        const next = { ...d, [name]: q }
        setStyle(next)
        return next
      })
    },
    [setStyle],
  )

  const reset = () => {
    const init = {}
    for (const p of params) init[p.name] = quantize(p.name, p.default) ?? p.default
    setDraft(init)
    setStyle(init)
  }

  return (
    <aside
      className={`fixed inset-x-0 bottom-0 z-40 flex max-h-[85vh] min-h-0 w-full min-w-0 flex-col border-t border-base-content bg-base-100 transition-transform duration-200 lg:static lg:z-auto lg:h-full lg:max-h-none lg:w-80 lg:translate-y-0 lg:pointer-events-auto lg:border-t-0 lg:border-l ${
        open ? 'translate-y-0' : 'pointer-events-none translate-y-full'
      }`}
    >
      <div className="flex shrink-0 items-center justify-between border-b border-base-content px-4 py-2 lg:hidden">
        <span className="font-mono text-sm font-bold">BurinMap</span>
        <button
          type="button"
          aria-label="Close"
          className="btn btn-sm btn-square border border-base-content"
          onClick={onClose}
        >
          ✕
        </button>
      </div>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-5 overflow-x-hidden overflow-y-auto p-4">
        <header className="flex min-w-0 flex-col gap-3">
          <div>
            <h1 className="font-mono text-3xl font-bold tracking-tight">BurinMap</h1>
            <p className="mt-1 min-w-0 text-sm leading-snug opacity-70">
              Pen-and-ink terrain from elevation. Explore on the basemap; when you stop, the view is
              drawn on top. Move again and the drawing fades so you can navigate.
            </p>
          </div>

          <dl className="grid grid-cols-3 gap-2 font-mono">
            <div>
              <dt className="text-[10px] tracking-wide uppercase opacity-45">lines</dt>
              <dd className="text-sm tabular-nums">{drawing ? fmtCount(drawing.lines) : '—'}</dd>
            </div>
            <div>
              <dt className="text-[10px] tracking-wide uppercase opacity-45">dashes</dt>
              <dd className="text-sm tabular-nums">{drawing ? fmtCount(drawing.dashes) : '—'}</dd>
            </div>
            <div>
              <dt className="text-[10px] tracking-wide uppercase opacity-45">time</dt>
              <dd className="text-sm tabular-nums">{drawing ? `${drawing.ms} ms` : '—'}</dd>
            </div>
          </dl>
          <p className="truncate font-mono text-[10px] uppercase opacity-45">
            {status.phase}
            {status.detail ? ` · ${status.detail}` : ''}
          </p>
        </header>

        <section className="flex min-w-0 flex-col gap-4 border-t border-base-content pt-5">
          <div className="flex items-center justify-between gap-2">
            <h2 className="font-mono text-xs font-bold tracking-wide uppercase">Style</h2>
            <button type="button" className="btn btn-sm border border-base-content" onClick={reset}>
              Reset
            </button>
          </div>
          {params.map((p) => (
            <StyleSlider
              key={p.name}
              p={p}
              value={draft[p.name] ?? p.default}
              onDrag={onDrag}
              onCommit={onCommit}
            />
          ))}
          <p className="min-w-0 text-[10px] leading-snug wrap-break-word opacity-45">
            Elevation from AWS Terrain Tiles. Basemap: Mapbox.
          </p>
        </section>
      </div>
    </aside>
  )
}
