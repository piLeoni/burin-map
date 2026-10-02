/**
 * The engine's progress panel, as the command line shows it on a terminal:
 * the tree on top (download, then the drawing and its steps), growing as
 * work starts, and the messages below. Everything in it comes from the
 * engine (see lib/progress.js); this only lays it out, black on white.
 * @param {{ view: import('../lib/progress.js').View }} props
 */
export default function LoadPanel({ view }) {
  const rule = view.earlier ? ` messages · ${view.earlier} earlier ` : ' messages '
  return (
    <div className="w-[40rem] max-w-[92%] border border-black bg-white p-3 font-mono text-[11px] leading-snug text-black">
      <div className="mb-2 truncate">
        <span className="font-bold">{view.head}</span>
        <span className="opacity-50"> ▸ {view.title}</span>
      </div>
      <div className="max-h-[70vh] overflow-hidden">
        {view.rows.map((r) => {
          const done = r.state === 'ok'
          return (
            <div key={r.name} className={`flex whitespace-pre ${r.state === 'fail' ? 'font-bold' : ''}`}>
              <span>{r.mark} {r.name.padEnd(view.name_width)}  </span>
              {r.bar && (
                <span>
                  <span className="opacity-25">{r.bar[0]}</span>
                  <span>{r.bar[1]}</span>
                  <span className="opacity-25">{r.bar[2]}</span>
                  {'  '}
                </span>
              )}
              <span className={`truncate ${done ? 'opacity-60' : ''}`}>{r.detail}</span>
            </div>
          )
        })}
      </div>
      {view.messages.length > 0 && (
        <div className="mt-2">
          <div className="truncate whitespace-pre opacity-60">──{rule}{'─'.repeat(80)}</div>
          {view.messages.map((m, i) => (
            <div key={view.earlier + i} className={`truncate whitespace-pre ${m.warn ? 'font-bold' : 'opacity-60'}`}>
              {m.at.toFixed(1).padStart(6)} s  {m.text}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
