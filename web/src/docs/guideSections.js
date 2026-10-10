import { createElement } from 'react'
import overview from '../../../docs/guide/01-overview.md?raw'
import spacing from '../../../docs/guide/02-spacing.md?raw'
import ink from '../../../docs/guide/03-ink.md?raw'
import modifiers from '../../../docs/guide/04-modifiers.md?raw'
import levels from '../../../docs/guide/05-levels.md?raw'
import presets from '../../../docs/guide/06-ink-presets.md?raw'
import water from '../../../docs/guide/07-water.md?raw'

import InkFlow from './figures/InkFlow.jsx'
import LevelsBar from './figures/LevelsBar.jsx'
import PeriodDuty from './figures/PeriodDuty.jsx'
import SpacingTone from './figures/SpacingTone.jsx'

/** Sidebar order and markdown body (repo source of truth: docs/guide/). */
export const GUIDE_SECTIONS = [
  {
    slug: 'overview',
    title: 'Overview',
    body: overview,
    figures: null,
  },
  {
    slug: 'spacing',
    title: 'Spacing',
    body: spacing,
    figures: () => createElement(SpacingTone),
  },
  {
    slug: 'ink',
    title: 'Ink along the line',
    body: ink,
    figures: () =>
      createElement(
        'div',
        { className: 'docs-figure-stack' },
        createElement(PeriodDuty, { duty: 1, period: 100, label: 'Full duty' }),
        createElement(PeriodDuty, { duty: 0.5, period: 100, label: 'Half duty' }),
        createElement(InkFlow),
      ),
  },
  {
    slug: 'modifiers',
    title: 'Modifiers',
    body: modifiers,
    figures: null,
  },
  {
    slug: 'levels',
    title: 'Quantization',
    body: levels,
    figures: () => createElement(LevelsBar),
  },
  {
    slug: 'ink-presets',
    title: 'Ink presets',
    body: presets,
    figures: null,
    presetPreviews: true,
  },
  {
    slug: 'water',
    title: 'Water & shoreline',
    body: water,
    figures: null,
  },
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
