import { useEffect, useRef, useState } from 'react'

const WIDTH = 20
const SPIN = '⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'

/** A bar as [filled, empty] runs of `width` cells, `frac` of them filled. */
const bar = (frac, width = WIDTH) => {
  const n = Math.round(Math.max(0, Math.min(1, frac)) * width)
  return ['█'.repeat(n), '░'.repeat(width - n)]
}

/** A block sliding to and fro, for work with no count and no estimate. */
const bounce = (ms, width = WIDTH) => {
  const w = Math.max(2, Math.round(width / 4))
  const span = width - w
  const step = Math.floor(ms / 70) % (2 * span)
  const at = step < span ? step : 2 * span - step
  return ['░'.repeat(at), '█'.repeat(w), '░'.repeat(span - at)]
}

const secs = (ms) => `${(ms / 1000).toFixed(1)} s`

function row(t, now, width) {
  // A task begun since the last frame starts after `now`.
  const ms = t.state === 'run' ? Math.max(0, now - t.t0) : t.ms
  if (t.state === 'wait') return { meter: ['', '░'.repeat(width)], detail: '' }
  if (t.state !== 'run') {
    const meter = bar(t.state === 'ok' ? 1 : t.total ? t.done / t.total : 0, width)
    const count = t.total && !t.child ? `${t.done}/${t.total} ${t.unit} · ` : ''
    return { meter, detail: `${count}${secs(ms)}${t.note ? ` · ${t.note}` : ''}` }
  }
  if (t.total) {
    const what = t.note || `${t.done}/${t.total} ${t.unit}`
    return { meter: bar(t.done / t.total, width), detail: t.child ? `${what} · ${secs(ms)}` : what }
  }
  // Past the estimate it creeps on rather than sitting full.
  if (t.eta) return { meter: bar(Math.min(0.97, ms / t.eta), width), detail: `${secs(ms)} / ~${secs(t.eta)}` }
  return { meter: bounce(ms, width), detail: secs(ms) }
}

const MARK = { ok: '✓', fail: '✗', wait: '·' }
const LOG_MARK = { info: ' ', warn: '!', error: '✗' }

/**
 * The draw's progress as a small terminal: a bar per task (tiles counted,
 * the drawing timed against the last one, each of its steps beneath it), and
 * below them the log. Black on white like the rest of the app.
 * @param {{ title: string, load: { tasks: import('../lib/progress.js').Task[], log: import('../lib/progress.js').LogLine[] } }} props
 */
export default function LoadPanel({ title, load }) {
  const [now, setNow] = useState(() => performance.now())
  const running = load.tasks.some((t) => t.state === 'run')
  useEffect(() => {
    if (!running) return undefined
    let id = requestAnimationFrame(function tick() {
      setNow(performance.now())
      id = requestAnimationFrame(tick)
    })
    return () => cancelAnimationFrame(id)
  }, [running])
  const logRef = useRef(null)
  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight
  }, [load.log.length])

  const label = Math.max(9, ...load.tasks.map((t) => t.label.length + (t.child ? 2 : 0)))
  const spin = SPIN[Math.floor(now / 80) % SPIN.length]
  return (
    <div className="w-[34rem] max-w-[92%] border border-base-content bg-base-100 p-3 font-mono text-[11px] leading-snug text-base-content">
      <div className="mb-2 truncate">
        <span className="font-bold">please wait</span>
        <span className="opacity-50"> ▸ {title}</span>
      </div>
      <div className="max-h-[70vh] overflow-hidden">
        {load.tasks.map((t, i) => {
          const last = t.child && !load.tasks[i + 1]?.child
          const width = t.child ? WIDTH - 6 : WIDTH
          const { meter, detail } = row(t, now, width)
          const [lead, full, rest = ''] = meter.length === 3 ? meter : ['', ...meter]
          return (
            <div
              key={t.id}
              className={`flex gap-2 whitespace-pre ${t.state === 'wait' ? 'opacity-35' : ''} ${t.state === 'fail' ? 'font-bold' : ''}`}
            >
              <span className="w-3 shrink-0">{t.state === 'run' ? spin : MARK[t.state]}</span>
              <span>{(t.child ? `${last ? '└' : '├'} ${t.label}` : t.label).padEnd(label)}</span>
              <span>
                <span className="opacity-25">{lead}</span>
                <span>{full}</span>
                <span className="opacity-25">{rest}</span>
                {' '.repeat(WIDTH - width)}
              </span>
              <span className={`truncate ${t.state === 'ok' ? 'opacity-60' : ''}`}>{detail}</span>
            </div>
          )
        })}
      </div>
      {load.log.length > 0 && (
        <div ref={logRef} className="mt-2 max-h-24 overflow-hidden border-t border-base-content/20 pt-1">
          {load.log.map((l, i) => (
            <div
              key={i}
              className={`truncate ${l.level === 'info' ? 'opacity-60' : ''} ${l.level === 'error' ? 'font-bold' : ''}`}
            >
              {LOG_MARK[l.level]} {l.text}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
