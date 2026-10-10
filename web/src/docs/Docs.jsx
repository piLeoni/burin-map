import { NavLink, Navigate, Route, Routes, Link, useLocation } from 'react-router-dom'
import { useEffect, useRef } from 'react'
import MarkdownBody from './MarkdownBody.jsx'
import Overview from './Overview.jsx'
import Examples from './Examples.jsx'
import { GUIDE_SECTIONS } from './guideSections.js'

function SectionPage({ section }) {
  return (
    <article className="docs-article">
      <MarkdownBody>{section.body}</MarkdownBody>
    </article>
  )
}

const NAV = [
  { group: 'Start', items: [{ slug: 'overview', title: 'Overview' }, { slug: 'examples', title: 'Examples' }] },
  ...['Model', 'Land', 'Water'].map((group) => ({
    group,
    items: GUIDE_SECTIONS.filter((s) => s.group === group),
  })),
]

function navClass({ isActive }) {
  return isActive ? 'docs-nav-active' : undefined
}

export default function Docs() {
  const main = useRef(null)
  const { pathname, hash } = useLocation()
  useEffect(() => {
    if (hash) document.getElementById(hash.slice(1))?.scrollIntoView()
    else main.current?.scrollTo(0, 0)
  }, [pathname, hash])

  return (
    <div className="docs-shell">
      <header className="docs-header" box-="">
        <Link to="/">← map</Link>
        <span className="docs-header-title">Burin Map docs</span>
        <a href="https://github.com/piLeoni/burin-map/blob/main/SYNTAX.md" target="_blank" rel="noreferrer">
          SYNTAX.md
        </a>
      </header>
      <div className="docs-body">
        <nav className="docs-nav" aria-label="Guide topics">
          {NAV.map(({ group, items }) => (
            <div key={group} className="docs-nav-group">
              <div className="docs-nav-heading">{group}</div>
              <ul>
                {items.map((s) => (
                  <li key={s.slug}>
                    <NavLink to={`/docs/${s.slug}`} className={navClass}>
                      {s.title}
                    </NavLink>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
        <main className="docs-main" ref={main}>
          <Routes>
            <Route index element={<Navigate to="overview" replace />} />
            <Route path="overview" element={<Overview />} />
            <Route path="examples" element={<Examples />} />
            {GUIDE_SECTIONS.map((s) => (
              <Route key={s.slug} path={s.slug} element={<SectionPage section={s} />} />
            ))}
          </Routes>
        </main>
      </div>
    </div>
  )
}
