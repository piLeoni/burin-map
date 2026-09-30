import { useEffect, useRef, useState } from 'react'
import { useAtom, useSetAtom } from 'jotai'
import Editor from '@monaco-editor/react'
import { EDITOR_THEME, defineEditorTheme, editorFont, registerCliLanguage } from '../lib/editorTheme'
import { loadBurin } from '../lib/burin'
import { clearServiceWorkers } from '../lib/devCache'
import { PRESETS } from '../lib/presets'
import { docFromHash, docToHash } from '../lib/doc'
import { areaFromFrame, frameFromArea } from '../lib/frame'
import { VIEWS, detectView, parseView, renderView } from '../lib/views'
import {
  BATCHES_V1_KEY, bathymetryAtom, cameraRequestAtom, frameAtom, mapViewAtom, notesAtom, styleAtom, viewKindAtom, waterAtom,
} from '../state'

const PHASE_LABEL = {
  loading: 'Starting…',
  working: 'Drawing…',
  ready: 'Up to date',
  exploring: 'Redraws when you stop',
  error: 'Something went wrong',
}

function fmtCount(n) { return Number(n).toLocaleString() }

/** Read a document from the URL fragment, then clear it: the state lives in the app, not the address bar. */
function takeDocFromHash() {
  const loc = globalThis.location
  if (!loc?.hash) return null
  let doc = null
  try {
    doc = docFromHash(loc.hash)
  } catch (e) {
    console.warn('Ignoring unreadable link:', e)
  }
  if (doc) globalThis.history?.replaceState(null, '', loc.pathname + loc.search)
  return doc
}

/** Older builds saved every parameter of a batch; keep only those that differ from the default. */
function onlyChanged(list, params) {
  const defaults = Object.fromEntries(params.map((p) => [p.name, p.default]))
  return list.map((b) => ({
    ...(typeof b?.name === 'string' && b.name.trim() ? { name: b.name } : {}),
    angle: Number(b?.angle) || 0,
    style: Object.fromEntries(Object.entries(b?.style ?? {}).filter(
      ([k, v]) => k in defaults && typeof v === 'number' && Math.abs(v - defaults[k]) > 1e-6,
    )),
  }))
}

function isFullDump(list, params) {
  return list.every((b) => params.every((p) => p.name in (b?.style ?? {})))
}

function batchesFromV1(params) {
  try {
    const list = JSON.parse(globalThis.localStorage?.getItem(BATCHES_V1_KEY) ?? 'null')
    return Array.isArray(list) && list.length > 0 ? onlyChanged(list, params) : null
  } catch {
    return null
  }
}

const FRESH = [{ angle: 0, style: {} }]

/* ---------------------------------------------------------------- */
/*  Sidebar — the document as YAML, JSON5 or a command               */
/* ---------------------------------------------------------------- */

export default function Sidebar({ open, onClose, status, drawing }) {
  const [schema, setSchema] = useState(null)   // { params, groups } from the engine
  const [batches, setBatches] = useAtom(styleAtom)
  const [water, setWater] = useAtom(waterAtom)
  const [bathymetry, setBathymetry] = useAtom(bathymetryAtom)
  const [notes, setNotes] = useAtom(notesAtom)
  const [kind, setKind] = useAtom(viewKindAtom)
  const [mapView] = useAtom(mapViewAtom)
  const [frame, setFrame] = useAtom(frameAtom)
  const setCameraRequest = useSetAtom(cameraRequestAtom)
  const [draft, setDraft] = useState(null)   // text typed but not applied yet
  const [error, setError] = useState('')
  const [notice, setNotice] = useState(null)   // { text, error? }
  const fileRef = useRef(null)
  const presetMenuRef = useRef(null)
  const noticeTimer = useRef(0)
  const applyDraftRef = useRef(() => {})

  const view = VIEWS[kind] ? kind : 'yaml'
  const list = Array.isArray(batches) ? batches : []

  const flash = (text, isError = false) => {
    clearTimeout(noticeTimer.current)
    setNotice({ text, error: isError })
    noticeTimer.current = setTimeout(() => setNotice(null), isError ? 8000 : 3000)
  }

  /** Point the map and the frame at a document's area block. Throws when it can't be drawn. */
  const applyArea = (area) => {
    if (!area || Object.keys(area).length === 0) return
    const { center, frame: next } = frameFromArea(area, frame)
    setFrame(next)
    if (center) setCameraRequest({ center })
  }

  /** Make a parsed model the current state. Throws when its area can't be drawn. */
  const load = (m) => {
    applyArea(m.area)
    setBatches(m.batches)
    setWater(m.water ?? null)
    setBathymetry(m.bathymetry ?? null)
    setNotes(m.notes ?? {})
  }

  // ---------- init ----------
  useEffect(() => {
    // A #d=… fragment wins over whatever is in localStorage.
    const fromHash = takeDocFromHash()
    if (fromHash) {
      try {
        applyArea(fromHash.area)
      } catch (e) {
        flash(`Can't open this URL: ${e.message}`, true)
      }
    }

    loadBurin().then((m) => {
      const params = m.style_params()
      const groups = typeof m.style_groups === 'function' ? m.style_groups() : []
      setSchema({ params, groups })

      if (fromHash && Array.isArray(fromHash.batches) && fromHash.batches.length > 0) {
        setBatches(fromHash.batches)
        setWater(fromHash.water ?? null)
        setNotes({})
        return
      }
      setBatches((prev) => {
        const cur = Array.isArray(prev) ? prev : []
        if (cur.length === 0) return batchesFromV1(params) ?? FRESH
        return isFullDump(cur, params) ? onlyChanged(cur, params) : cur
      })
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setBatches, setCameraRequest])

  // ---------- the text ----------
  const model = { area: areaFromFrame(frame, mapView?.center), batches: list, water, bathymetry, notes: notes ?? {} }
  const liveText = schema && list.length > 0 ? renderView(view, model, schema) : ''
  const shown = draft ?? liveText
  const dirty = draft != null && draft !== liveText

  const applyDraft = () => {
    if (!dirty) return
    try {
      load(parseView(view, draft, schema))
      setDraft(null)
      setError('')
    } catch (e) {
      setError(e.message || 'Can\'t read this')
    }
  }
  applyDraftRef.current = applyDraft

  const revertDraft = () => {
    setDraft(null)
    setError('')
  }

  /** Switch view; an unapplied edit comes along, rewritten in the new view. */
  const switchView = (next) => {
    if (next === view) return
    if (dirty) {
      try {
        setDraft(renderView(next, parseView(view, draft, schema), schema))
      } catch (e) {
        setError(`Fix this before switching: ${e.message}`)
        return
      }
    } else {
      setDraft(null)
    }
    setError('')
    setKind(next)
  }

  const loadPreset = (p) => {
    presetMenuRef.current?.removeAttribute('open')
    const doc = structuredClone(p.doc)
    try {
      load({ area: doc.area, batches: doc.batches, water: doc.water ?? null, bathymetry: doc.bathymetry ?? null, notes: {} })
    } catch (e) {
      flash(`Can't load ${p.label}: ${e.message}`, true)
      return
    }
    setDraft(null)
    setError('')
    flash(`Loaded ${p.label}`)
  }

  // ---------- files, clipboard, link ----------
  const importFile = async (file) => {
    if (!file || !schema) return
    try {
      const text = await file.text()
      load(parseView(detectView(file.name, text), text, schema))
      setDraft(null)
      setError('')
      flash(`Loaded ${file.name}`)
    } catch (e) {
      flash(`Can't open ${file.name}: ${e.message || 'not a BurinMap document'}`, true)
    }
  }

  const download = () => {
    const url = URL.createObjectURL(new Blob([shown], { type: VIEWS[view].mime }))
    const a = document.createElement('a')
    a.href = url
    a.download = `burin-map.${VIEWS[view].ext}`
    a.click()
    URL.revokeObjectURL(url)
  }

  const copy = async (text, what) => {
    try {
      await navigator.clipboard.writeText(text)
      flash(`${what} copied`)
    } catch {
      flash('The browser blocked the clipboard', true)
    }
  }

  const copyLink = () => {
    const loc = globalThis.location
    copy(`${loc.origin}${loc.pathname}${docToHash({ area: model.area, batches: list, water })}`, 'Link')
  }

  // A link pasted into an open tab only changes the hash, which doesn't reload.
  useEffect(() => {
    const win = globalThis.window
    if (!schema || !win?.addEventListener) return
    const onHash = () => {
      const doc = takeDocFromHash()
      if (!doc) return
      try {
        load({ area: doc.area, batches: doc.batches, water: doc.water, notes: {} })
        setDraft(null)
        flash('Opened URL')
      } catch (e) {
        flash(`Can't open this URL: ${e.message || 'unreadable'}`, true)
      }
    }
    win.addEventListener('hashchange', onHash)
    return () => win.removeEventListener('hashchange', onHash)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [schema])

  useEffect(() => {
    const close = (e) => {
      const menu = presetMenuRef.current
      if (menu?.open && !menu.contains(e.target)) menu.removeAttribute('open')
    }
    document.addEventListener('pointerdown', close)
    return () => document.removeEventListener('pointerdown', close)
  }, [])

  const tool = 'btn btn-xs border-base-content/25 whitespace-nowrap'

  return (
    <aside
      className={`fixed inset-x-0 bottom-0 z-40 flex h-[85vh] min-h-0 w-full min-w-0 flex-col border-t border-base-content bg-base-100 transition-transform duration-200 lg:static lg:z-auto lg:h-full lg:w-[28rem] lg:shrink-0 lg:translate-y-0 lg:pointer-events-auto lg:border-t-0 lg:border-l ${
        open ? 'translate-y-0' : 'pointer-events-none translate-y-full'
      }`}
    >
      {/* Mobile header */}
      <div className="flex shrink-0 items-center justify-between border-b border-base-content px-4 py-2 lg:hidden">
        <span className="font-mono text-sm font-bold">BurinMap</span>
        <button type="button" aria-label="Close" className="btn btn-sm btn-square border border-base-content" onClick={onClose}>✕</button>
      </div>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {/* Fixed top: title, stats, toolbar */}
        <div className="flex shrink-0 flex-col gap-3.5 px-5 pt-5 pb-4">
          <header className="flex flex-col gap-2">
            <div className="flex items-center justify-between gap-2">
              <h1 className="hidden font-mono text-2xl font-bold tracking-tight lg:block">BurinMap</h1>
              {import.meta.env.DEV && (
                <button
                  type="button"
                  className="btn btn-ghost btn-xs ml-auto whitespace-nowrap opacity-40 hover:opacity-100"
                  title="Dev only: remove service workers and their caches, then reload, so the latest WASM is used. The document is kept."
                  onClick={() => clearServiceWorkers()}
                >
                  Clear cache
                </button>
              )}
            </div>
            <p className="text-[11px] leading-relaxed opacity-50">
              Terrain drawn as engraved lines. Pan or zoom the map; the view is
              redrawn when you stop. An online demo of{' '}
              <a className="link whitespace-nowrap" href="https://github.com/piLeoni/burin-map" target="_blank" rel="noreferrer">burin-map</a>,
              a terrain engine written in Rust, also available as a command-line
              tool and as Python and Node libraries. Here the same engine runs in
              your browser, compiled to WebAssembly.
            </p>
          </header>

          <div className="flex items-baseline gap-3 font-mono text-[10px]">
            <span className="shrink-0 tabular-nums opacity-60">
              {drawing
                ? `${fmtCount(drawing.lines)} lines · ${fmtCount(drawing.dashes)} dashes · ${drawing.ms} ms`
                  + (drawing.threads > 1 ? ` · ${drawing.threads} threads` : '')
                  + (typeof drawing.gpu === 'string' ? ` · ${drawing.gpu}` : '')
                : '-'}
            </span>
            <span className={`ml-auto truncate tracking-wider uppercase ${status.phase === 'error' ? 'text-error' : 'opacity-40'}`}>
              {status.detail || PHASE_LABEL[status.phase] || status.phase}
            </span>
          </div>

          <div className="flex flex-col gap-2 border-t border-base-content/15 pt-4">
            <div className="flex items-center gap-2">
              <div role="tablist" className="join">
                {Object.entries(VIEWS).map(([id, v]) => (
                  <button
                    key={id}
                    type="button"
                    role="tab"
                    aria-selected={view === id}
                    className={`btn btn-xs join-item px-3 whitespace-nowrap ${view === id ? 'border-base-content bg-base-content text-base-100' : 'border-base-content/25 bg-base-100'}`}
                    onClick={() => switchView(id)}
                  >
                    {v.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="grid grid-cols-4 gap-1.5">
              <button type="button" className={tool} title="Open a YAML, JSON, JSON5 or .sh file" disabled={!schema} onClick={() => fileRef.current?.click()}>
                Import
              </button>
              <button type="button" className={tool} title={`Save as burin-map.${VIEWS[view].ext}`} disabled={!shown} onClick={download}>
                Download
              </button>
              <details ref={presetMenuRef} className="dropdown dropdown-end flex">
                <summary className={`${tool} w-full list-none ${schema ? '' : 'btn-disabled'}`} title="Replace the style and the area with a ready-made one">
                  Load ▾
                </summary>
                <ul className="menu dropdown-content z-20 mt-1 w-64 rounded-box border border-base-content/25 bg-base-100 p-1 shadow">
                  {PRESETS.map((p) => (
                    <li key={p.id}>
                      <button type="button" className="flex flex-col items-start gap-0" onClick={() => loadPreset(p)}>
                        <span className="font-mono text-[11px] font-bold">{p.label}</span>
                        <span className="text-[10px] opacity-50">{p.blurb}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              </details>
              <button type="button" className={tool} title="Copy a link that opens this map and these batches (without comments)" disabled={!schema} onClick={copyLink}>
                Link
              </button>
              <input
                ref={fileRef}
                type="file"
                accept=".yaml,.yml,.json,.json5,.sh,.txt"
                className="hidden"
                onChange={(e) => { importFile(e.target.files?.[0]); e.target.value = '' }}
              />
            </div>
            {notice && (
              <p className={`font-mono text-[10px] leading-snug break-words ${notice.error ? 'text-error' : 'opacity-60'}`}>
                {notice.text}
              </p>
            )}
          </div>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-2 border-t border-base-content/15 px-5 py-4">
          <div className="flex shrink-0 flex-col gap-1.5">
            <div className="flex items-center gap-1.5">
              <span className={`mr-auto truncate font-mono text-[11px] ${dirty ? 'font-bold' : 'opacity-50'}`}>
                {dirty ? '● Edited, not applied' : 'In step with the map'}
              </span>
              <button type="button" className="btn btn-xs border-base-content/25" disabled={!dirty} onClick={revertDraft}>
                Revert
              </button>
              <button type="button" className={`btn btn-xs ${dirty ? 'border-base-content bg-base-content text-base-100' : 'border-base-content/25'}`} disabled={!dirty} title="⌘/Ctrl + Enter" onClick={applyDraft}>
                Apply
              </button>
            </div>
            {error && <p className="font-mono text-[10px] leading-snug break-words text-error">{error}</p>}
          </div>
          <div className={`relative min-h-0 flex-1 overflow-hidden rounded-lg border ${dirty ? 'border-base-content' : 'border-base-content/25'}`}>
            {shown && (
              <button
                type="button"
                aria-label={`Copy ${VIEWS[view].label}`}
                title={`Copy the ${VIEWS[view].label}`}
                className="btn btn-square btn-xs absolute top-1.5 right-4 z-10 border-base-content/25 bg-base-100/90 opacity-60 hover:opacity-100"
                onClick={() => copy(shown, VIEWS[view].label)}
              >
                <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <rect x="9" y="9" width="12" height="12" rx="2" />
                  <path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" />
                </svg>
              </button>
            )}
            {schema ? (
              <Editor
                height="100%"
                language={VIEWS[view].language}
                theme={EDITOR_THEME}
                value={shown}
                onChange={(v) => { setDraft(v ?? ''); setError('') }}
                beforeMount={(monaco) => {
                  monaco.languages.json?.jsonDefaults?.setDiagnosticsOptions({ validate: false })
                  registerCliLanguage(monaco)
                  defineEditorTheme(monaco)
                }}
                onMount={(editor, monaco) => {
                  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => applyDraftRef.current())
                  // Monaco measures glyphs once; a web font that lands later
                  // would leave the cursor out of step with the text.
                  document.fonts.ready.then(() => monaco.editor.remeasureFonts())
                }}
                options={{
                  minimap: { enabled: false },
                  fontFamily: editorFont(),
                  fontSize: 11,
                  bracketPairColorization: { enabled: false },
                  lineNumbersMinChars: 3,
                  scrollBeyondLastLine: false,
                  wordWrap: 'on',
                  wrappingIndent: 'deepIndent',
                  tabSize: 2,
                  insertSpaces: true,
                  detectIndentation: false,
                }}
              />
            ) : (
              <p className="p-3 text-[11px] opacity-40">Loading parameters…</p>
            )}
          </div>
        </div>

        <p className="shrink-0 border-t border-base-content/15 px-5 py-2 text-[10px] opacity-40">
          Elevation: AWS Terrain Tiles. Basemap: Mapbox.
        </p>
      </div>
    </aside>
  )
}
