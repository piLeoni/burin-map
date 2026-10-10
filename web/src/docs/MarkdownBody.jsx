import ReactMarkdown from 'react-markdown'

const REPO = 'https://github.com/piLeoni/burin-map/blob/main'
const REPO_SYNTAX = `${REPO}/SYNTAX.md`

/** `img/foo.png` (relative, as on GitHub) → `/docs/guide/img/foo.png` (site). */
function siteImageSrc(src) {
  if (!src) return src
  if (/^https?:\/\//.test(src)) return src
  const name = src.replace(/^\.?\/?img\//, '')
  return `/docs/guide/img/${name}`
}

/** Render guide markdown; fix repo-relative links/images for the site. */
export default function MarkdownBody({ children }) {
  return (
    <div className="docs-prose">
      <ReactMarkdown
        components={{
          a: ({ href, ...props }) => {
            let h = href
            if (h?.includes('SYNTAX.md')) h = REPO_SYNTAX
            else if (h === '/docs') h = '/docs'
            else if (h?.startsWith('../../')) h = `${REPO}/${h.replace('../../', '')}`
            return <a href={h} {...props} />
          },
          img: ({ src, alt, ...props }) => (
            // eslint-disable-next-line jsx-a11y/alt-text
            <img src={siteImageSrc(src)} alt={alt} loading="lazy" {...props} />
          ),
          code: ({ className, children, ...props }) => {
            const block = className?.includes('language-')
            if (block) {
              return (
                <pre className="docs-code">
                  <code {...props}>{children}</code>
                </pre>
              )
            }
            return <code className="docs-inline-code" {...props}>{children}</code>
          },
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  )
}
