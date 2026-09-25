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
 *   setStyle: (s: Record<string, number>) => void,
 *   status: { phase: string, detail?: string },
 *   drawing: { svg: string, lines: number, dashes: number, ms: number } | null,
 * }} props
 */
export default function Sidebar({ setStyle, status, drawing }) {
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

  const Slider = ({ p }) => {
    const isInt = p.name === 'seed'
    const [lo, hi] = RANGES[p.name] || [0, Math.max(1, p.default * 2)]
    const step = isInt ? 1 : (hi - lo) / 200
    const v = draft[p.name] ?? p.default
    return (
      <div className="w-full min-w-0">
        <div className="flex w-full min-w-0 items-baseline justify-between gap-2">
          <span className="min-w-0 truncate font-mono text-xs">{p.name}</span>
          <span className="shrink-0 font-mono text-xs opacity-60">{fmt(v)}</span>
        </div>
        <input
          type="range"
          className="range range-xs w-full"
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
        <p className="mt-0.5 w-full text-xs leading-snug wrap-break-word opacity-50">{p.help}</p>
      </div>
    )
  }

  return (
    <aside className="flex h-full min-h-0 w-full min-w-0 shrink-0 flex-col overflow-x-hidden overflow-y-auto border-t border-base-content lg:w-72 lg:border-t-0 lg:border-l">
      <div className="flex min-w-0 flex-col gap-5 p-4">
        <header className="min-w-0">
          <h1 className="font-mono text-xl font-bold tracking-tight">burin</h1>
          <p className="text-sm opacity-60">pen-and-ink terrain from elevation</p>
        </header>

        <p className="min-w-0 text-sm leading-snug">
          <span className="font-mono text-xs uppercase">{status.phase}</span>
          {status.detail ? (
            <>
              <br />
              <span className="opacity-70">{status.detail}</span>
            </>
          ) : null}
        </p>

        <p className="min-w-0 text-sm leading-snug opacity-70">
          Explore on the basemap. When you stop, elevation for that view is drawn on top. Move again
          and the drawing fades so you can navigate.
        </p>

        <section className="flex min-w-0 flex-col gap-4">
          <div className="flex items-baseline justify-between gap-2">
            <h2 className="font-mono text-xs font-bold tracking-wide uppercase">Style</h2>
            <button type="button" className="btn btn-xs btn-ghost" onClick={reset}>
              reset
            </button>
          </div>
          {primary.map((p) => (
            <Slider key={p.name} p={p} />
          ))}
          {rest.length > 0 && (
            <details className="min-w-0">
              <summary className="cursor-pointer font-mono text-xs opacity-60">
                more parameters
              </summary>
              <div className="mt-3 flex min-w-0 flex-col gap-4">
                {rest.map((p) => (
                  <Slider key={p.name} p={p} />
                ))}
              </div>
            </details>
          )}
        </section>

        <div className="mt-2 flex min-w-0 flex-col gap-2">
          <button
            type="button"
            className="btn w-full"
            disabled={!drawing?.svg}
            onClick={download}
          >
            Download SVG
          </button>
          <p className="min-w-0 text-xs leading-snug wrap-break-word opacity-50">
            Elevation from AWS Terrain Tiles. Basemap: Mapbox.
          </p>
        </div>
      </div>
    </aside>
  )
}
