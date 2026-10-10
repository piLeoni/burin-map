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
  { slug: 'model', group: 'Model', title: 'Tone, spacing, ink', body: model },
  { slug: 'land-lines', group: 'Land', title: 'Lines', body: landLines },
  { slug: 'land-dots', group: 'Land', title: 'Dots', body: landDots },
  { slug: 'fills', group: 'Land', title: 'Fills (land and sea)', body: fills },
  { slug: 'water-lines', group: 'Water', title: 'Lines', body: waterLines },
  { slug: 'water-ripples', group: 'Water', title: 'Ripples', body: waterRipples },
  { slug: 'water-dots', group: 'Water', title: 'Dots', body: waterDots },
  { slug: 'shoreline', group: 'Water', title: 'Shoreline', body: shoreline },
]
