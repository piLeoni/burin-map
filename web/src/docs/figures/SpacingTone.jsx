/** Spacing widens in light and tightens in shadow along tone t. */
export default function SpacingTone() {
  const w = 280
  const h = 100
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="docs-figure" role="img" aria-label="Spacing vs tone">
      <defs>
        <linearGradient id="toneGrad" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="var(--background1)" />
          <stop offset="100%" stopColor="var(--foreground0)" />
        </linearGradient>
      </defs>
      <text x="0" y="12" className="docs-figure-label">tone t →</text>
      <rect x="40" y="18" width="220" height="10" fill="url(#toneGrad)" />
      <text x="40" y="34" className="docs-figure-caption">0 light</text>
      <text x="240" y="34" textAnchor="end" className="docs-figure-caption">1 shadow</text>
      {[0, 0.5, 1].map((t, i) => {
        const x = 40 + t * 220
        const gap = 28 - t * 10
        return (
          <g key={i}>
            <line x1={x - gap / 2} y1={55} x2={x - gap / 2} y2={88} stroke="currentColor" strokeWidth="1.2" />
            <line x1={x + gap / 2} y1={55} x2={x + gap / 2} y2={88} stroke="currentColor" strokeWidth="1.2" />
          </g>
        )
      })}
      <text x="0" y="96" className="docs-figure-caption">space_light (wider) → space_dark (tighter)</text>
    </svg>
  )
}
