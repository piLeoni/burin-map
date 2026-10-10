import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Link } from 'react-router-dom'
import ZoomFigure from './ZoomFigure.jsx'
import manifest from './figures.json'

const REPO = 'https://github.com/piLeoni/burin-map/blob/main'

/** Guide page files → site routes. */
const PAGE_SLUGS = { '00-model': 'model', README: 'overview' }

/** `img/foo.png` (relative, as on GitHub) → the figure id `foo`. */
function figureId(src) {
  return src?.replace(/^\.?\/?img\//, '').replace(/\.png$/, '')
}

function siteImageSrc(src) {
  if (!src || /^https?:\/\//.test(src)) return src
  return `/docs/guide/img/${src.replace(/^\.?\/?img\//, '')}`
}

function siteLink(href) {
  if (!href) return { href }
  if (/^https?:\/\//.test(href) || href.startsWith('#')) return { href }
  const repo = href.match(/^(?:\.\.\/)+(.+)$/)
  if (repo) return { href: `${REPO}/${repo[1]}`, external: true }
  const page = href.match(/^([\w-]+)\.md(#.*)?$/)
  if (page) return { to: `/docs/${PAGE_SLUGS[page[1]] ?? page[1]}${page[2] ?? ''}` }
  return { href }
}

function firstHeader(node) {
  const th = node?.children?.find((c) => c.tagName === 'thead')?.children?.find((c) => c.tagName === 'tr')
    ?.children?.find((c) => c.tagName === 'th')
  return th?.children?.map((c) => c.value ?? '').join('').trim()
}

function isFigure(node) {
  const kids = node?.children?.filter((c) => !(c.type === 'text' && !c.value.trim())) ?? []
  return kids.length === 1 && kids[0].tagName === 'img' && manifest.figures[figureId(kids[0].properties?.src)]
}

/** Render guide markdown; fix repo-relative links/images for the site. */
export default function MarkdownBody({ children }) {
  return (
    <div className="docs-prose">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          p: ({ node, children: kids, ...props }) =>
            isFigure(node) ? <>{kids}</> : <p {...props}>{kids}</p>,
          a: ({ href, children: kids, node: _node, ...props }) => {
            const l = siteLink(href)
            if (l.to) return <Link to={l.to} {...props}>{kids}</Link>
            return (
              <a href={l.href} {...props} {...(l.external ? { target: '_blank', rel: 'noreferrer' } : {})}>
                {kids}
              </a>
            )
          },
          img: ({ src, alt, node: _node, ...props }) => {
            const fig = manifest.figures[figureId(src)]
            if (fig) return <ZoomFigure {...fig} />
            // eslint-disable-next-line jsx-a11y/alt-text
            return <img src={siteImageSrc(src)} alt={alt} loading="lazy" {...props} />
          },
          table: ({ node, ...props }) => (
            <table className={firstHeader(node) === 'Group' ? 'docs-params' : undefined} {...props} />
          ),
          pre: ({ children: kids }) => <pre className="docs-code">{kids}</pre>,
          code: ({ className, children: kids, node: _node, ...props }) => {
            const block = className?.includes('language-') || String(kids).includes('\n')
            return (
              <code className={block ? undefined : 'docs-inline-code'} {...props}>
                {kids}
              </code>
            )
          },
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  )
}
