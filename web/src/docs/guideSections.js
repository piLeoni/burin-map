import model from '../../../docs/guide/00-model.md?raw'
import landLines from '../../../docs/guide/land-lines.md?raw'
import landDots from '../../../docs/guide/land-dots.md?raw'
import waterLines from '../../../docs/guide/water-lines.md?raw'
import waterRipples from '../../../docs/guide/water-ripples.md?raw'
import waterDots from '../../../docs/guide/water-dots.md?raw'
import shoreline from '../../../docs/guide/shoreline.md?raw'
import fills from '../../../docs/guide/fills.md?raw'

/** Sidebar order and markdown body (repo source of truth: docs/guide/). */
export const GUIDE_SECTIONS = [
  { slug: 'model', title: 'The shared model', body: model },
  { slug: 'land-lines', title: 'Land — lines', body: landLines, presetPreviews: true },
  { slug: 'land-dots', title: 'Land — dots', body: landDots },
  { slug: 'water-lines', title: 'Water — lines', body: waterLines },
  { slug: 'water-ripples', title: 'Water — ripples', body: waterRipples },
  { slug: 'water-dots', title: 'Water — dots', body: waterDots },
  { slug: 'shoreline', title: 'Shoreline', body: shoreline },
  { slug: 'fills', title: 'Fills', body: fills },
]

export const INK_PRESET_PREVIEWS = [
  { id: 'ink-classic', label: 'classic' },
  { id: 'ink-solid', label: 'solid' },
  { id: 'ink-half', label: 'half' },
  { id: 'ink-jitter', label: 'jitter' },
  { id: 'ink-levels', label: 'levels' },
  { id: 'ink-flat', label: 'flat' },
  { id: 'ink-thresholds', label: 'thresholds' },
]
