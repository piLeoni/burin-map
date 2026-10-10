/** Three quantized tone bands with different dash duty. */
export default function LevelsBar() {
  const zones = [
    { label: '10%', w: 90, duty: 0.1 },
    { label: '50%', w: 90, duty: 0.5 },
    { label: '100%', w: 90, duty: 1 },
  ]
  let x = 0
  return (
    <svg viewBox="0 0 280 72" className="docs-figure" role="img" aria-label="Quantized ink zones">
      {zones.map((z) => {
        const bx = x
        x += z.w
        const ink = 50 * z.duty
        return (
          <g key={z.label}>
            <rect x={bx} y="8" width={z.w - 4} height="20" fill="var(--background1)" stroke="var(--foreground0)" strokeWidth="0.5" />
            <text x={bx + (z.w - 4) / 2} y="22" textAnchor="middle" className="docs-figure-caption">{z.label}</text>
            <rect x={bx + 8} y="38" width={ink} height="5" fill="currentColor" />
            <rect x={bx + 8 + ink} y="38" width={50 - ink} height="5" fill="var(--background2)" />
          </g>
        )
      })}
      <text x="0" y="68" className="docs-figure-caption">dash_levels: 3 — flat zones, sharp borders</text>
    </svg>
  )
}
