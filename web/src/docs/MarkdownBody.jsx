import ReactMarkdown from 'react-markdown'

const REPO_SYNTAX = 'https://github.com/piLeoni/burin-map/blob/main/SYNTAX.md'

/** Render guide markdown; fix repo-only links for the site. */
export default function MarkdownBody({ children }) {
  return (
    <div className="docs-prose">
    <ReactMarkdown
      components={{
        a: ({ href, ...props }) => {
          let h = href
          if (h?.includes('SYNTAX.md')) h = REPO_SYNTAX
          if (h?.startsWith('../../SYNTAX.md')) h = REPO_SYNTAX
          if (h === '/docs') h = '/docs'
          return <a href={h} {...props} />
        },
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
