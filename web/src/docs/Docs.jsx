import { NavLink, Navigate, Route, Routes, Link } from 'react-router-dom'
import MarkdownBody from './MarkdownBody.jsx'
import PresetPreviewGrid from './PresetPreviewGrid.jsx'
import { GUIDE_SECTIONS } from './guideSections.js'

function SectionPage({ section }) {
  return (
    <article className="docs-article">
      <MarkdownBody>{section.body.replace(/^# .+\n+/, '')}</MarkdownBody>
      {section.presetPreviews ? <PresetPreviewGrid /> : null}
    </article>
  )
}

export default function Docs() {
  const first = GUIDE_SECTIONS[0]?.slug ?? 'model'
  return (
    <div className="docs-shell">
      <header className="docs-header" box-="">
        <Link to="/">← map</Link>
        <span className="docs-header-title">Docs</span>
        <a href="https://github.com/piLeoni/burin-map/blob/main/SYNTAX.md" target="_blank" rel="noreferrer">
          SYNTAX.md
        </a>
      </header>
      <div className="docs-body">
        <nav className="docs-nav" aria-label="Guide topics">
          <ul>
            {GUIDE_SECTIONS.map((s) => (
              <li key={s.slug}>
                <NavLink to={`/docs/${s.slug}`} className={({ isActive }) => (isActive ? 'docs-nav-active' : undefined)}>
                  {s.title}
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>
        <main className="docs-main">
          <Routes>
            <Route index element={<Navigate to={first} replace />} />
            {GUIDE_SECTIONS.map((s) => (
              <Route key={s.slug} path={s.slug} element={<SectionPage section={s} />} />
            ))}
          </Routes>
        </main>
      </div>
    </div>
  )
}
