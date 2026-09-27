import { useEffect, useState, useCallback, useMemo } from 'react'
import { useAtom } from 'jotai'
import Editor from '@monaco-editor/react'
import { loadBurin } from '../lib/burin'
import { styleAtom } from '../state'

/* ---------------------------------------------------------------- */
/*  Ranges, int params, group metadata                               */
/* ---------------------------------------------------------------- */

const RANGES = {
  angle: [-90, 90, 1],
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
  'angle',
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

/* ---------------------------------------------------------------- */
/*  Helpers                                                          */
/* ---------------------------------------------------------------- */

function isIntParam(name) { return INT_PARAMS.has(name) }
function rangeOf(name, fallback) { return RANGES[name] || [0, Math.max(1, (fallback ?? 1) * 2), 0.1] }

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
  return Math.round(c * 10 ** decimals) / 10 ** decimals
}

function fmt(name, v) {
  if (isIntParam(name)) return String(Math.round(v))
  const step = rangeOf(name, v)[2]
  const decimals = step < 0.05 ? 2 : 1
  const q = Number(Number(v).toFixed(decimals))
  return Number.isInteger(q) ? String(q) : q.toFixed(decimals)
}

function fmtCount(n) { return Number(n).toLocaleString() }

function defaultsFrom(params) {
  const d = {}
  for (const p of params) d[p.name] = quantize(p.name, p.default) ?? p.default
  return d
}

/** JSON representation of batches — matches --style-json schema. */
function batchesToJson(batches) {
  return JSON.stringify(
    batches.map((b) => ({
      angle: b.angle,
      style: Object.fromEntries(
        Object.entries(b.style).filter(([, v]) => v != null && typeof v === 'number' && Number.isFinite(v)),
      ),
    })),
    null,
    2,
  )
}

function jsonToBatches(json, defaults) {
  const raw = JSON.parse(json)
  if (!Array.isArray(raw)) throw new Error('Expected a JSON array of batches')
  return raw.map((entry) => {
    const style = { ...defaults }
    if (entry?.style && typeof entry.style === 'object') {
      for (const [k, v] of Object.entries(entry.style)) {
        if (k in defaults && typeof v === 'number' && Number.isFinite(v)) {
          style[k] = quantize(k, v) ?? defaults[k]
        }
      }
    }
    return { angle: quantize('angle', entry?.angle ?? 0) ?? 0, style }
  })
}

/* ---------------------------------------------------------------- */
/*  HelpTip — a real (?) button with a hover/focus tooltip           */
/* ---------------------------------------------------------------- */

function HelpTip({ text }) {
  return (
    <span className="group/help relative inline-flex shrink-0 items-center">
      <button
        type="button"
        aria-label="Help"
        className="flex h-4 w-4 cursor-help items-center justify-center rounded-full border border-current/40 font-mono text-[9px] leading-none opacity-45 hover:opacity-100 focus:opacity-100"
      >
        ?
      </button>
      <span
        role="tooltip"
        className="pointer-events-none absolute bottom-full left-1/2 z-50 mb-1 w-52 -translate-x-1/2 rounded border border-base-content bg-base-100 px-2 py-1.5 text-[10px] leading-snug shadow-lg opacity-0 transition-opacity group-hover/help:opacity-100 group-focus-within/help:opacity-100"
      >
        {text}
      </span>
    </span>
  )
}

/* ---------------------------------------------------------------- */
/*  ParamRow — label (?)  /  [slider]  [value]                       */
/* ---------------------------------------------------------------- */

function ParamRow({ name, label, help, value, onDrag, onCommit }) {
  const int = isIntParam(name)
  const [lo, hi, step] = rangeOf(name, value)
  const [text, setText] = useState(() => fmt(name, value))
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    if (!editing) setText(fmt(name, value))
  }, [value, name, editing])

  const commitText = () => {
    setEditing(false)
    const q = quantize(name, text)
    if (q == null) { setText(fmt(name, value)); return }
    setText(fmt(name, q))
    onCommit(name, q)
  }

  return (
    <div className="flex w-full min-w-0 flex-col gap-0.5">
      <div className="flex w-full min-w-0 items-center gap-1">
        <span className="min-w-0 truncate font-mono text-[10px] leading-none">{label}</span>
        <HelpTip text={help} />
      </div>
      <div className="flex w-full min-w-0 items-center gap-2">
        <input
          type="range"
          className="range range-xs min-w-0 flex-1"
          min={lo} max={hi} step={int ? 1 : step}
          value={value}
          onPointerDown={(e) => e.currentTarget.setPointerCapture(e.pointerId)}
          onChange={(e) => { const q = quantize(name, e.target.value); if (q != null) onDrag(name, q) }}
          onPointerUp={(e) => { const q = quantize(name, e.currentTarget.value); if (q != null) onCommit(name, q) }}
          onKeyUp={(e) => {
            if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) {
              const q = quantize(name, e.currentTarget.value); if (q != null) onCommit(name, q)
            }
          }}
        />
        <input
          type="text"
          inputMode={int ? 'numeric' : 'decimal'}
          className="input input-xs w-11 shrink-0 text-right font-mono text-[10px] tabular-nums"
          value={text}
          onFocus={() => setEditing(true)}
          onChange={(e) => setText(e.target.value)}
          onBlur={commitText}
          onKeyDown={(e) => {
            if (e.key === 'Enter') e.currentTarget.blur()
            else if (e.key === 'Escape') { setText(fmt(name, value)); setEditing(false); e.currentTarget.blur() }
          }}
        />
      </div>
    </div>
  )
}

/* ---------------------------------------------------------------- */
/*  BatchCard — one pass: angle row + collapsible grouped params     */
/* ---------------------------------------------------------------- */

function BatchCard({ index, batch, params, groups, onUpdate, onRemove, canRemove }) {
  const [open, setOpen] = useState(true)

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

  const setParam = useCallback((name, value) => {
    onUpdate(index, { ...batch, style: { ...batch.style, [name]: value } })
  }, [index, batch, onUpdate])

  const setAngle = useCallback((_name, value) => {
    onUpdate(index, { ...batch, angle: Math.round(value) })
  }, [index, batch, onUpdate])

  const label = (name) => name.replace(/_/g, '-')

  return (
    <div className="overflow-hidden rounded-lg border border-base-content/30">
      {/* Header */}
      <div className="flex w-full items-center justify-between bg-base-200/50 px-3 py-2">
        <button
          type="button"
          className="flex flex-1 items-center gap-2 text-left hover:opacity-80"
          onClick={() => setOpen((o) => !o)}
        >
          <span className="font-mono text-xs font-bold">Batch {index + 1}</span>
          <span className="text-[10px] opacity-50">{open ? '▾' : '▸'}</span>
        </button>
        {canRemove && (
          <button
            type="button"
            aria-label={`Delete batch ${index + 1}`}
            className="btn btn-xs btn-ghost text-error"
            onClick={() => onRemove(index)}
          >
            ×
          </button>
        )}
      </div>

      {/* Collapsible body */}
      {open && (
        <div className="flex flex-col gap-2 p-3 pt-2">
          {/* Angle — a first-class parameter row */}
          <ParamRow
            name="angle"
            label="angle"
            help="degrees from the contour tangent: 0 = along level curves, 90 = downslope"
            value={batch.angle}
            onDrag={setAngle}
            onCommit={setAngle}
          />

          {grouped.map((g) => (
            <details key={g.id} className="group" open>
              <summary className="cursor-pointer font-mono text-[10px] font-bold uppercase tracking-wide opacity-50">
                {g.title}
              </summary>
              <div className="mt-1.5 flex flex-col gap-1.5">
                {g.params.map((p) => (
                  <ParamRow
                    key={p.name}
                    name={p.name}
                    label={label(p.name)}
                    help={p.help}
                    value={batch.style[p.name] ?? p.default}
                    onDrag={setParam}
                    onCommit={setParam}
                  />
                ))}
              </div>
            </details>
          ))}
          {grouped.length === 0 && (
            <p className="text-[10px] opacity-40">No groups loaded — WASM may still be initialising.</p>
          )}
        </div>
      )}
    </div>
  )
}

/* ---------------------------------------------------------------- */
/*  Sidebar — batch list, JSON tools, Monaco toggle                  */
/* ---------------------------------------------------------------- */

export default function Sidebar({ open, onClose, status, drawing }) {
  const [params, setParams] = useState([])
  const [groups, setGroups] = useState([])
  const [defaults, setDefaults] = useState({})
  const [batches, setBatches] = useAtom(styleAtom)
  const [view, setView] = useState('style')   // 'style' | 'json'
  const [jsonText, setJsonText] = useState('')
  const [jsonError, setJsonError] = useState('')
  const [ready, setReady] = useState(false)

  const list = Array.isArray(batches) ? batches : []

  // ---------- init ----------
  useEffect(() => {
    loadBurin().then((m) => {
      const p = m.style_params()
      const g = typeof m.style_groups === 'function' ? m.style_groups() : []
      setParams(p)
      setGroups(g)
      const defs = defaultsFrom(p)
      setDefaults(defs)
      setReady(true)
      // If nothing is persisted yet, start with one default batch.
      setBatches((prev) => {
        const cur = Array.isArray(prev) ? prev : []
        if (cur.length > 0) {
          // Backfill any missing keys so newer params get their defaults.
          return cur.map((b) => ({ ...b, style: { ...defs, ...b.style } }))
        }
        return [{ angle: 0, style: { ...defs } }]
      })
    })
  }, [setBatches])

  // ---------- keep JSON text in step with batches ----------
  useEffect(() => {
    if (list.length === 0) return
    setJsonText(batchesToJson(list))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [batches])

  // ---------- actions ----------
  const addBatch = () => setBatches([...list, { angle: 0, style: { ...defaults } }])
  const removeBatch = (i) => {
    if (list.length <= 1) return
    setBatches(list.filter((_, j) => j !== i))
  }
  const updateBatch = (i, b) => setBatches(list.map((orig, j) => (j === i ? b : orig)))
  const reset = () => setBatches([{ angle: 0, style: { ...defaults } }])

  const applyJson = (raw) => {
    try {
      const parsed = jsonToBatches(raw, defaults)
      if (parsed.length === 0) throw new Error('Need at least one batch')
      setBatches(parsed)
      setJsonError('')
    } catch (e) {
      setJsonError(e.message || 'Invalid JSON')
    }
  }

  const importJson = () => {
    const raw = prompt('Paste batch JSON:')
    if (raw) applyJson(raw)
  }

  const downloadJson = () => {
    const blob = new Blob([jsonText], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url; a.download = 'burin-batches.json'; a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <aside
      className={`fixed inset-x-0 bottom-0 z-40 flex max-h-[85vh] min-h-0 w-full min-w-0 flex-col border-t border-base-content bg-base-100 transition-transform duration-200 lg:static lg:z-auto lg:h-full lg:max-h-none lg:w-80 lg:translate-y-0 lg:pointer-events-auto lg:border-t-0 lg:border-l ${
        open ? 'translate-y-0' : 'pointer-events-none translate-y-full'
      }`}
    >
      {/* Mobile header */}
      <div className="flex shrink-0 items-center justify-between border-b border-base-content px-4 py-2 lg:hidden">
        <span className="font-mono text-sm font-bold">BurinMap</span>
        <button type="button" aria-label="Close" className="btn btn-sm btn-square border border-base-content" onClick={onClose}>✕</button>
      </div>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-x-hidden overflow-y-auto p-4">
        {/* Header */}
        <header className="flex min-w-0 flex-col gap-2">
          <h1 className="font-mono text-3xl font-bold tracking-tight">BurinMap</h1>
          <p className="min-w-0 text-sm leading-snug opacity-70">
            Pen-and-ink terrain from elevation. Pan the map — it draws when you stop.
          </p>
          <dl className="grid grid-cols-3 gap-2 font-mono">
            <div>
              <dt className="text-[10px] uppercase opacity-45">lines</dt>
              <dd className="text-sm tabular-nums">{drawing ? fmtCount(drawing.lines) : '—'}</dd>
            </div>
            <div>
              <dt className="text-[10px] uppercase opacity-45">dashes</dt>
              <dd className="text-sm tabular-nums">{drawing ? fmtCount(drawing.dashes) : '—'}</dd>
            </div>
            <div>
              <dt className="text-[10px] uppercase opacity-45">time</dt>
              <dd className="text-sm tabular-nums">{drawing ? `${drawing.ms} ms` : '—'}</dd>
            </div>
          </dl>
          <p className="truncate font-mono text-[10px] uppercase opacity-45">
            {status.phase}{status.detail ? ` · ${status.detail}` : ''}
          </p>
        </header>

        {/* Section header + io buttons */}
        <div className="flex items-center justify-between gap-2 border-t border-base-content pt-4">
          <h2 className="font-mono text-xs font-bold uppercase tracking-wide">Style</h2>
          <div className="flex items-center gap-1">
            <button type="button" className="btn btn-xs border border-base-content" onClick={reset}>Reset</button>
            <button type="button" className="btn btn-xs border border-base-content" onClick={importJson}>Import</button>
            <button type="button" className="btn btn-xs border border-base-content" onClick={downloadJson}>Export</button>
          </div>
        </div>

        {/* View toggle */}
        <div className="flex gap-1">
          <button
            type="button"
            className={`btn btn-xs ${view === 'style' ? 'btn-neutral' : 'btn-ghost'}`}
            onClick={() => setView('style')}
          >
            Parameters
          </button>
          <button
            type="button"
            className={`btn btn-xs ${view === 'json' ? 'btn-neutral' : 'btn-ghost'}`}
            onClick={() => setView('json')}
          >
            JSON
          </button>
        </div>

        {/* Parameters view */}
        {view === 'style' && (
          <div className="flex flex-col gap-3">
            {!ready && <p className="text-[10px] opacity-40">Loading parameters…</p>}
            {ready && list.map((b, i) => (
              <BatchCard
                key={i}
                index={i}
                batch={b}
                params={params}
                groups={groups}
                onUpdate={updateBatch}
                onRemove={removeBatch}
                canRemove={list.length > 1}
              />
            ))}
            {ready && (
              <button type="button" className="btn btn-xs btn-outline self-start" onClick={addBatch}>
                + Batch
              </button>
            )}
          </div>
        )}

        {/* JSON view */}
        {view === 'json' && (
          <div className="flex min-h-[300px] flex-col gap-2">
            <div className="overflow-hidden rounded border border-base-content/30">
              <Editor
                height="40vh"
                defaultLanguage="json"
                theme="vs"
                value={jsonText}
                onChange={(v) => { setJsonText(v ?? ''); setJsonError('') }}
                options={{
                  minimap: { enabled: false },
                  fontSize: 11,
                  lineNumbers: 'off',
                  scrollBeyondLastLine: false,
                  wordWrap: 'on',
                }}
              />
            </div>
            <div className="flex items-center gap-2">
              <button type="button" className="btn btn-xs btn-neutral" onClick={() => applyJson(jsonText)}>Apply</button>
              {jsonError && <span className="font-mono text-[10px] text-error">{jsonError}</span>}
            </div>
          </div>
        )}

        <p className="min-w-0 text-[10px] leading-snug opacity-45">
          Elevation from AWS Terrain Tiles. Basemap: Mapbox.
        </p>
      </div>
    </aside>
  )
}
