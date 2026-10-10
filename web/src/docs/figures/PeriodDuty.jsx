/** One dash period: duty controls ink vs gap along the line. */
export default function PeriodDuty({ duty = 0.5, period = 120, label }) {
  const ink = period * duty
  const gap = period - ink
  const h = 48
  const y = 28
  return (
    <svg viewBox={`0 0 ${period + 40} ${h}`} className="docs-figure" role="img" aria-label={label ?? `Period with ${Math.round(duty * 100)}% ink`}>
      <text x="0" y="14" className="docs-figure-label">along the line →</text>
      <rect x="20" y={y} width={ink} height="6" fill="currentColor" />
      <rect x={20 + ink} y={y} width={gap} height="6" fill="var(--background2)" stroke="var(--foreground2)" strokeWidth="0.5" />
      <text x="20" y={y + 22} className="docs-figure-caption">
        period {period}px · duty {Math.round(duty * 100)}%
      </text>
    </svg>
  )
}
