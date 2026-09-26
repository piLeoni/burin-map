import { useEffect, useState, useCallback, useMemo } from 'react'
import { useAtom } from 'jotai'
import { loadBurin } from '../lib/burin'
import { styleAtom } from '../state'

/** [min, max, step] — step drives the slider grain. */
const RANGES = {
  space: [1, 12, 0.1],
  space_light: [0.5, 3, 0.05],
  space_dark: [0.2, 1.5, 0.05],
  space_steep: [0, 1, 0.05],
  space_steep_gamma: [0.3, 3, 0.1],
  dash_light: [0, 1, 0.05],
  dash_dark: [0, 1, 0.05],
  dash_period: [1, 8, 0.1],
  dash_cutoff: [0, 0.2, 0.01],
  dash_drop: [0, 1, 0.05],
  mod_jitter: [0, 1, 0.05],
  mod_breaks: [0, 1, 0.05],
  mod_wobble: [0, 60, 1],
  mod_wobble_size: [2, 40, 1],
  mod_smooth: [1, 30, 0.5],
  input_flat: [0, 0.6, 0.01],
  input_gain: [0.5, 8, 0.1],
  input_dark_gamma: [0.3, 2, 0.05],
  input_light_gain: [0.5, 4, 0.1],
  input_light_floor: [0, 0.3, 0.01],
  input_blur: [0.5, 8, 0.1],
  input_broad_blur: [2, 40, 0.5],
  input_broad: [0, 1, 0.05],
  sun_azimuth: [0, 360, 1],
  sun_altitude: [5, 85, 1],
  sun_exag: [1, 12, 0.1],
  engine_step: [0.5, 2, 0.1],
  engine_min: [1, 20, 1],
  engine_stop: [0.2, 1, 0.05],
  engine_fill: [0.5, 1, 0.05],
  batch_floor: [0, 1, 0.05],
  batch_stroke: [0.4, 3, 0.1],
  batch_seed: [0, 100, 1],
}

const INT_PARAMS = new Set([
  'batch_seed',
  'sun_azimuth',
  'sun_altitude',
  'mod_wobble',
  'mod_wobble_size',
  'engine_min',
])

const GROUP_TITLE = {
  space: 'Space',
  dash: 'Dash',
  mod: 'Mod',
  input: 'Input',
  sun: 'Sun',
  engine: 'Engine',
  batch: 'Batch',
}

function isIntParam(name) {
  return INT_PARAMS.has(name)
}

function rangeOf(name, fallbackDefault) {
  return RANGES[name] || [0, Math.max(1, (fallbackDefault ?? 1) * 2), 0.1]
}

function clamp(name, n) {
  const [lo, hi] = rangeOf(name, n)
  return Math.min(hi, Math.max(lo, n))
}

function quantize(name, raw) {
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return null
  const c = clamp(name, n)
  if (isIntParam(name)) return Math.round(c)
  const step = rangeOf(name, c)[2]
  const decimals = step < 0.05 ? 2 : 1
  const f = 10 ** decimals
  return Math.round(c * f) / f
}

function fmt(name, v) {
  if (isIntParam(name)) return String(Math.round(v))
  const step = rangeOf(name, v)[2]
  const decimals = step < 0.05 ? 2 : 1
  const q = Number(v.toFixed(decimals))
  return Number.isInteger(q) ? String(q) : q.toFixed(decimals)
}

function fmtCount(n) {
  return Number(n).toLocaleString()
}

/** Stable slider — must live outside Sidebar or drag remounts mid-gesture. */
function StyleSlider({ p, value, onDrag, onCommit }) {
  const int = isIntParam(p.name)
  const [lo, hi, step] = rangeOf(p.name, p.default)
  const [text, setText] = useState(() => fmt(p.name, value))
  const [editing, setEditing] = useState(false)

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
        step={int ? 1 : step}
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
  const [groups, setGroups] = useState([])
  const [style, setStyle] = useAtom(styleAtom)
  const [draft, setDraft] = useState({})

  useEffect(() => {
    loadBurin().then((m) => {
      const list = m.style_params()
      const g = typeof m.style_groups === 'function' ? m.style_groups() : []
      setParams(list)
      setGroups(g)
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

  const grouped = useMemo(() => {
    const blurbs = Object.fromEntries(groups.map((g) => [g.id, g.blurb]))
    const order = groups.length ? groups.map((g) => g.id) : [...new Set(params.map((p) => p.group))]
    return order
      .map((id) => ({
        id,
        title: GROUP_TITLE[id] || id,
        blurb: blurbs[id] || '',
        params: params.filter((p) => p.group === id),
      }))
      .filter((g) => g.params.length > 0)
  }, [params, groups])

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

        <div className="flex items-center justify-between gap-2 border-t border-base-content pt-5">
          <h2 className="font-mono text-xs font-bold tracking-wide uppercase">Style</h2>
          <button type="button" className="btn btn-sm border border-base-content" onClick={reset}>
            Reset
          </button>
        </div>

        {grouped.map((g) => (
          <section key={g.id} className="flex min-w-0 flex-col gap-3 border-t border-base-content/40 pt-4">
            <div>
              <h3 className="font-mono text-xs font-bold tracking-wide uppercase">{g.title}</h3>
              {g.blurb ? (
                <p className="mt-1 text-[11px] leading-snug opacity-60">{g.blurb}</p>
              ) : null}
            </div>
            {g.params.map((p) => (
              <StyleSlider
                key={p.name}
                p={p}
                value={draft[p.name] ?? p.default}
                onDrag={onDrag}
                onCommit={onCommit}
              />
            ))}
          </section>
        ))}

        <p className="min-w-0 text-[10px] leading-snug wrap-break-word opacity-45">
          Elevation from AWS Terrain Tiles. Basemap: Mapbox.
        </p>
      </div>
    </aside>
  )
}
