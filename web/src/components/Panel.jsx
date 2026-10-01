/**
 * A terminal box with its title, and optionally controls, set into the top
 * border: ┌ title ──── aside ┐.
 */
export default function Panel({ title, aside, className = '', children }) {
  return (
    <section box-="square" className={`relative ${className}`}>
      {(title || aside) && (
        <div className="pointer-events-none absolute inset-x-[1ch] top-0 flex h-[1lh] items-center gap-[1ch]">
          {title && <span className="pointer-events-auto truncate bg-white px-[1ch] font-bold">{title}</span>}
          {aside && <span className="pointer-events-auto ml-auto flex items-center bg-white px-[1ch]">{aside}</span>}
        </div>
      )}
      {children}
    </section>
  )
}
