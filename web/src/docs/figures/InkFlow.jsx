/** Order of rules applied each dash period. */
export default function InkFlow() {
  const steps = ['cutoff', 'solid', 'mod_flat', 'levels', 'duty', 'jitter', 'breaks']
  return (
    <svg viewBox="0 0 320 56" className="docs-figure" role="img" aria-label="Ink decision order">
      {steps.map((s, i) => (
        <g key={s}>
          <rect x={i * 44} y="12" width="40" height="28" fill="var(--background1)" stroke="currentColor" strokeWidth="0.8" />
          <text x={i * 44 + 20} y="30" textAnchor="middle" className="docs-figure-micro">{s}</text>
          {i < steps.length - 1 ? (
            <text x={i * 44 + 44} y="30" textAnchor="middle" className="docs-figure-micro">→</text>
          ) : null}
        </g>
      ))}
    </svg>
  )
}
