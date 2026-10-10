import { INK_PRESET_PREVIEWS } from './guideSections.js'

export default function PresetPreviewGrid() {
  return (
    <div className="docs-preview-grid">
      {INK_PRESET_PREVIEWS.map(({ id, label }) => (
        <figure key={id} className="docs-preview-card">
          <img
            src={`/docs/previews/${id}.png`}
            alt={`${label} ink preset sample`}
            width={320}
            height={222}
            loading="lazy"
          />
          <figcaption>
            <code>{id}</code>
          </figcaption>
        </figure>
      ))}
    </div>
  )
}
