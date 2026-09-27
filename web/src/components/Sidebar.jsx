import { useEffect, useState, useCallback, useMemo } from 'react'
import { useAtom } from 'jotai'
import Editor from '@monaco-editor/react'
import { loadBurin } from '../lib/burin'
import { styleAtom } from '../state'

/* ---------------------------------------------------------------- */
/*  Ranges, int params, group metadata                               */
/* ---------------------------------------------------------------- */

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

/* ---------------------------------------------------------------- */
/*  Helpers                                                          */
/* ---------------------------------------------------------------- */

function isIntParam(name) { return INT_PARAMS.has(name) }
function rangeOf(name, falback) { return RANGES[name] || [0, Math.max(1, (falback ?? 1) * 2), 0.1] }

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
  const q = Number(v.toFixed(decimals))
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
  return raw.map((entry, i) => {
    const style = { ...defaults }
    if (entry.style && typeof entry.style === 'object') {
      for (const [k, v] of Object.entries(entry.style)) {
        if (k in defaults && typeof v === 'number' && Number.isFinite(v)) {
          style[k] = quantize(k, v) ?? defaults[k]
        }
      }
    }
    return { angle: quantize('batch_angle', entry.angle ?? 0) ?? 0, style }
  })
}

/* ---------------------------------------------------------------- */
/*  Slider (compact — no help label, just a (?) tooltip trigger)     */
/* ---------------------------------------------------------------- */

function StyleSlider({ name, help, value, onDrag, onCommit }) {
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
    <div className="w-full min-w-0">
      <div className="flex w-full min-w-0 items-center justify-between gap-1">
        <span className="min-w-0 truncate font-mono text-[10px]">{name}</span>
        <input
          type="text"
          inputMode={int ? 'numeric' : 'decimal'}
          className="input input-xs w-12 shrink-0 text-right font-mono text-[10px]"
          value={text}
          onFocus={() => setEditing(true)}
          onChange={(e) => setText(e.target.value)}
          onBlur={commitText}
          onKeyDown={(e) => {
            if (e.key === 'Enter') e.currentTarget.blur()
            else if (e.key === 'Escape') { setText(fmt(name, value)); setEditing(false); e.currentTarget.blur() }
          }}
        />
        <span className="tooltip shrink-0 text-[10px] cursor-help opacity-40 font-mono" data-tip={help}>
          (?)
        </span>
      </div>
      <input
        type="range"
        className="range range-xs w-full"
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
    </div>
  )
}

/* ---------------------------------------------------------------- */
/*  BatchCard — one pass: angle + collapsible grouped sliders        */
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

  const drag = useCallback((name, value) => {
    onUpdate(index, { ...batch, style: { ...batch.style, [name]: value } })
  }, [index, batch, onUpdate])

  const commit = useCallback((name, value) => {
    onUpdate(index, { ...batch, style: { ...batch.style, [name]: value } })
  }, [index, batch, onUpdate])

  return (
    <div className="border border-base-content/30 rounded-lg overflow-hidden">
      {/* Header */}
      <button
        type="button"
        className="flex w-full items-center justify-between px-3 py-2 bg-base-200/50 hover:bg-base-200"
        onClick={() => setOpen((o) => !o)}
      >
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs font-bold">Batch {index + 1}</span>
          <span className="text-[10px] opacity-50">{open ? '▾' : '▸'}</span>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1 text-[10px]">
            <span className="opacity-40">∠</span>
            <input
              type="number"
              className="input input-xs w-12 font-mono text-[10px]"
              value={batch.angle}
              min={-180}
              max={180}
              step={1}
              onChange={(e) => {
                const v = Number(e.target.value)
                if (Number.isFinite(v)) onUpdate(index, { ...batch, angle: Math.round(v) })
              }}
              onClick={(e) => e.stopPropagation()}
            />
            <span className="opacity-40">°</span>
          </label>
          {canRemove && (
            <button
              type="button"
              className="btn btn-xs btn-ghost text-error"
              onClick={(e) => { e.stopPropagation(); onRemove(index) }}
            >
              ×
            </button>
          )}
        </div>
      </button>

      {/* Collapsible body */}
      {open && (
        <div className="flex flex-col gap-2 p-3 pt-2">
          {grouped.map((g) => (
            <details key={g.id} className="group" open>
              <summary className="cursor-pointer font-mono text-[10px] font-bold uppercase tracking-wide opacity-50">
                {g.title}
              </summary>
              <div className="mt-1 flex flex-col gap-1">
                {g.params.map((p) => (
                  <StyleSlider
                    key={p.name}
                    name={p.name}
                    help={p.help}
                    value={batch.style[p.name] ?? p.default}
                    onDrag={drag}
                    onCommit={commit}
                  />
                ))}
              </div>
            </details>
          ))}
          {group.length === 0 && (
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
  const [style, setStyle] = useAtom(styleAtom)
  const [defaults, setDefaults] = useState({})
  const [batches, setBatches] = useState([])
  const [view, setView] = useState('visual')   // 'visual' | 'json'
  const [jsonText, setJsonText] = useState('')
  const [jsonError, setJsonError] = useState('')

  // ---------- init ----------
  useEffect(() => {
    loadBurin().then((m) => {
      const list = m.style_params()
      const g = typeof m.style_groups === 'function' ? m.style_groups() : []
      setParams(list)
      setGroups(g)
      const defs = defaultsFrom(list)
      setDefaults(defs)
      // Existing style from localStorage — migrate to batch list
      if (style && Object.keys(style).length > 0) {
        const restored = { ...defs }
        for (const [k, v] of Object.entries(style)) {
          if (k in defs && typeof v === 'number' && Number.isFinite(v)) {
            restored[k] = quantize(k, v) ?? defs[k]
          }
        }
        setBatches([{ angle: 0, style: restored }])
      } else {
        setBatches([{ angle: 0, style: { ...defs } }])
      }
    })
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // ---------- synch JSON text when batches change ----------
  useEffect(() => {
    if (batches.length === 0) return
    setJsonText(batchesToJson(batches))
  }, [batches])

  // ---------- synch to localStorage ----------
  useEffect(() => {
    if (batches.length === 0) return
    const merged = {}
    for (const b of batches) Object.assign(merged, b.style)
    setStyle(merged)
  }, [batches, setStyle])

  // ---------- actions ----------
  const addBatch = () => {
    setBatches((prev) => [...prev, { angle: 0, style: { ...defaults } }])
  }
  const removeBatch = (i) => {
    setBatches((prev) => prev.length > 1 ? prev.filter((_, j) => j !== i) : prev)
  }
  const updateBatch = (i, b) => {
    setBatches((prev) => prev.map((orig, j) => (j === i ? b : orig)))
  }
  const reset = () => {
    setBatches([{ angle: 0, style: { ...defaults } }])
  }

  // ---------- JSON import ----------
  const importJson = () => {
    const raw = prompt('Paste batch JSON:')
    if (!raw) return
    try {
      const parsed = jsonToBatches(raw, defaults)
      if (parsed.length === 0) throw new Error('Need at least one batch')
      setBatches(parsed)
      setJsonError('')
    } catch (e) {
      setJsonError(e.message || 'Invalid JSON')
    }
  }
  const downloadJson = () => {
    const blob = new Blob([jsonText], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url; a.download = 'burin-style.json'; a.click()
    URL.revokeObjectURL(url)
  }
  const monacoApply = () => {
    try {
      const parsed = jsonToBatches(jsonText, defaults)
      if (parsed.length === 0) throw new Error('Need at least one batch')
      setBatches(parsed)
      setJsonError('')
    } catch (e) {
      setJsonError(e.message || 'Invalid JSON')
    }
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

        {/* Toggle + actions */}
        <div className="flex items-center justify-between gap-2 border-t border-base-content pt-4">
          <h2 className="font-mono text-xs font-bold tracking-wide uppercase">Style</h2>
          <div className="flex items-center gap-1">
            <button type="button" className="btn btn-xs border border-base-content" onClick={reset}>Reset</button>
            <button type="button" className="btn btn-xs border border-base-content" onClick={importJson}>Import</button>
            <button type="button" className="btn btn-xs border border-base-content" onClick={downloadJson}>Export</button>
          </div>
        </div>

        {/* View toggle */}
        <div className="flex gap-1">
          <button type="button" className={`btn btn-xs ${view === 'visual' ? 'btn-neutral' : 'btn-ghost'}`} onClick={() => setView('visual')}>Sliders</button>
          <button type="button" className={`btn btn-xs ${view === 'json' ? 'btn-neutral' : 'btn-ghost'}`} onClick={() => setView('json')}>JSON</button>
        </div>

        {/* Visual mode: batch cards */}
        {view === 'visual' && (
          <div className="flex flex-col gap-3">
            {batches.map((b, i) => (
              <BatchCard
                key={i}
                index={i}
                batch={b}
                params={params}
                groups={groups}
                onUpdate={updateBatch}
                onRemove={removeBatch}
                canRemove={batches.length > 1}
              />
            ))}
            <button type="button" className="btn btn-xs btn-outline self-start" onClick={addBatch}>
              + Batch
            </button>
          </div>
        )}

        {/* JSON mode: Monaco editor */}
        {view === 'json' && (
          <div className="flex flex-col gap-2 min-h-[300px]">
            <Editor
              height="40vh"
              defaultLanguage="json"
              theme="vs-dark"
              value={jsonText}
              onChange={(v) => { setJsonText(v ?? ''); setJsonError('') }}
              options={{ minimap: { enabled: false }, fontSize: 11, lineNumbers: 'off', scrollBeyondLastLine: false }}
            />
            <div className="flex items-center gap-2">
              <button type="button" className="btn btn-xs btn-neutral" onClick={monacoApply}>Apply</button>
              {jsonError && <span className="text-[10px] text-error font-mono">{jsonError}</span>}
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
