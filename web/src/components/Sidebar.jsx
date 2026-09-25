import { useEffect, useMemo, useState } from 'react'
import { loadBurin } from '../lib/burin'

const PRIMARY = new Set([
  'spacing',
  'spacing_steep',
  'shade_gain',
  'flat_level',
  'lit_floor',
  'broad_weight',
  'flow_blur',
  'jitter',
  'wobble',
  'breaks',
  'azimuth',
  'stroke',
])

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

function fmt(v) {
  if (Number.isInteger(v)) return String(v)
  return String(Math.round(v * 1000) / 1000)
}

/**
 * @param {{
 *   style: Record<string, number>,
 *   setStyle: (s: Record<string, number>) => void,
 *   status: { phase: string, detail?: string },
 *   drawing: { svg: string, lines: number, dashes: number, ms: number } | null,
 * }} props
 */
export default function Sidebar({ style, setStyle, status, drawing }) {
  const [params, setParams] = useState([])
  const [draft, setDraft] = useState({})

  useEffect(() => {
    loadBurin().then((m) => {
      const list = m.style_params()
      setParams(list)
      const init = {}
      for (const p of list) init[p.name] = p.default
      setDraft(init)
      setStyle(init)
    })
  }, [setStyle])

  const primary = useMemo(() => params.filter((p) => PRIMARY.has(p.name)), [params])
  const rest = useMemo(() => params.filter((p) => !PRIMARY.has(p.name)), [params])

  const commit = (name, value) => {
    const next = { ...draft, [name]: value }
    setDraft(next)
    setStyle(next)
  }

  const reset = () => {
    const init = {}
    for (const p of params) init[p.name] = p.default
    setDraft(init)
    setStyle(init)
  }

  const download = () => {
    if (!drawing?.svg) return
    const a = document.createElement('a')
    a.href = URL.createObjectURL(new Blob([drawing.svg], { type: 'image/svg+xml' }))
    a.download = 'burin.svg'
    a.click()
    URL.revokeObjectURL(a.href)
  }

  const phaseBadge = {
    exploring: 'badge-ghost',
    working: 'badge-warning',
    ready: 'badge-success',
    error: 'badge-error',
  }[status.phase] || 'badge-ghost'

  const Slider = ({ p }) => {
    const isInt = p.name === 'seed'
    const [lo, hi] = RANGES[p.name] || [0, Math.max(1, p.default * 2)]
    const step = isInt ? 1 : (hi - lo) / 200
    const v = draft[p.name] ?? p.default
    return (
      <fieldset className="fieldset py-1">
        <div className="flex items-baseline justify-between gap-2">
          <legend className="fieldset-legend py-0 text-xs font-mono opacity-70">{p.name}</legend>
          <span className="font-mono text-xs">{fmt(v)}</span>
        </div>
        <input
          type="range"
          className="range range-xs range-primary"
          min={lo}
          max={hi}
          step={step}
          value={v}
          onChange={(e) => {
            const n = isInt ? Math.round(+e.target.value) : +e.target.value
            setDraft((d) => ({ ...d, [p.name]: n }))
          }}
          onMouseUp={(e) => commit(p.name, isInt ? Math.round(+e.target.value) : +e.target.value)}
          onTouchEnd={(e) => {
            const el = e.target
            commit(p.name, isInt ? Math.round(+el.value) : +el.value)
          }}
        />
        <p className="label text-[0.7rem] leading-snug opacity-60">{p.help}</p>
      </fieldset>
    )
  }

  return (
    <aside className="flex h-full w-full flex-col gap-4 overflow-y-auto bg-base-200 p-4 lg:w-80 lg:shrink-0 lg:border-l lg:border-base-300">
      <header>
        <h1 className="font-mono text-2xl font-semibold tracking-tight">burin</h1>
        <p className="text-sm opacity-60">pen-and-ink terrain from elevation</p>
      </header>

      <div className="rounded-box bg-base-100 p-3 text-sm shadow-sm">
        <div className="mb-1 flex items-center gap-2">
          <span className={`badge badge-sm ${phaseBadge}`}>{status.phase}</span>
        </div>
        <p className="text-xs leading-relaxed opacity-80">{status.detail || '—'}</p>
      </div>

      <div className="alert alert-info text-xs shadow-sm">
        <span>
          Explore freely on the map. When you stop, elevation is fetched for that view and drawn.
          Moving again fades the overlay.
        </span>
      </div>

      <section>
        <div className="mb-2 flex items-center justify-between">
          <h2 className="text-xs font-semibold tracking-wide uppercase opacity-60">Style</h2>
          <button type="button" className="btn btn-ghost btn-xs" onClick={reset}>
            reset
          </button>
        </div>
        <div className="flex flex-col gap-1">
          {primary.map((p) => (
            <Slider key={p.name} p={p} />
          ))}
        </div>
        {rest.length > 0 && (
          <details className="mt-2">
            <summary className="cursor-pointer font-mono text-xs opacity-60">more parameters</summary>
            <div className="mt-2 flex flex-col gap-1">
              {rest.map((p) => (
                <Slider key={p.name} p={p} />
              ))}
            </div>
          </details>
        )}
      </section>

      <div className="mt-auto flex flex-col gap-2 pt-2">
        <button
          type="button"
          className="btn btn-primary"
          disabled={!drawing?.svg}
          onClick={download}
        >
          Download SVG
        </button>
        <p className="text-[0.7rem] leading-snug opacity-50">
          Elevation:{' '}
          <a
            className="link"
            href="https://registry.opendata.aws/terrain-tiles/"
            target="_blank"
            rel="noreferrer"
          >
            AWS Terrain Tiles
          </a>
          .           Map:{' '}
          {import.meta.env.VITE_MAPBOX_TOKEN ? (
            <a className="link" href="https://www.mapbox.com/" target="_blank" rel="noreferrer">
              Mapbox
            </a>
          ) : (
            <>
              <a className="link" href="https://openfreemap.org/" target="_blank" rel="noreferrer">
                OpenFreeMap
              </a>{' '}
              via MapLibre (set <code className="text-[0.65rem]">VITE_MAPBOX_TOKEN</code> for Mapbox
              outdoors + globe).
            </>
          )}
        </p>
      </div>
    </aside>
  )
}
