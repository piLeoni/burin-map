import { useEffect, useRef, useState } from 'react'
import { useAtom, useSetAtom } from 'jotai'
import Editor from '@monaco-editor/react'
import { EDITOR_THEME, defineEditorTheme, editorFont, registerCliLanguage, registerDocLanguage } from '../lib/editorTheme'
import BURIN_TYPES from '../lib/burin.d.ts?raw'
import { loadBurin } from '../lib/burin'
import { PRESETS } from '../lib/presets'
import { docFromHash, docToHash } from '../lib/doc'
import { areaFromFrame, frameFromArea } from '../lib/frame'
import { VIEWS, atlasToModel, detectView, parseView, renderView, runJs } from '../lib/views'
import Panel from './Panel'
import {
  BATCHES_V1_KEY, bathymetryAtom, cameraRequestAtom, frameAtom, mapViewAtom, notesAtom, styleAtom, viewKindAtom, waterAtom,
} from '../state'

const PHASE_LABEL = {
  loading: 'Starting…',
  working: 'Drawing…',
  ready: 'Up to date!',
  exploring: 'Redraws when you stop',
  error: 'Something went wrong',
}

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

/** WebTUI button looks: a grey key. */
const KEY = { 'size-': 'small', 'variant-': 'background1' }

/* ---------------------------------------------------------------- */
/*  Sidebar — the document as YAML, JSON5 or a command               */
/* ---------------------------------------------------------------- */

export default function Sidebar({ open, onClose, status }) {
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

  /**
   * A view's text into a model. The JS view runs through the interpreter, so
   * this is async; the others parse straight away.
   */
  const toModel = async (kind, text, area) => {
    if (kind === 'js') return atlasToModel(await runJs(text), schema, area ?? {})
    return parseView(kind, text, schema)
  }

  const applyDraft = async () => {
    if (!dirty) return
    try {
      load(await toModel(view, draft, model.area))
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
  const switchView = async (next) => {
    if (next === view) return
    if (dirty) {
      try {
        const m = await toModel(view, draft, model.area)
        // The JS view is written from the model; the rest render it.
        setDraft(next === 'js' ? null : renderView(next, m, schema))
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

  const loadPreset = async (p) => {
    presetMenuRef.current?.removeAttribute('open')
    // A JS preset is a script: open the JS view and run it.
    if (p.script) {
      setKind('js')
      setDraft(p.script)
      try {
        load(await toModel('js', p.script, model.area))
        setError('')
        flash(`Loaded ${p.label} — a JavaScript document`)
      } catch (e) {
        setError(e.message || 'Can\'t run this')
      }
      return
    }
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
      const kind = detectView(file.name, text)
      load(await toModel(kind, text, model.area))
      if (kind === 'js') setKind('js')
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

  const statusText = status.detail || PHASE_LABEL[status.phase] || status.phase
  const failed = status.phase === 'error'

  return (
    <aside
      className={`fixed inset-x-0 bottom-0 z-40 flex h-[85vh] min-h-0 w-full min-w-0 flex-col border-t border-black bg-white px-[2ch] transition-transform duration-200 lg:static lg:z-auto lg:h-full lg:w-[28rem] lg:shrink-0 lg:translate-y-0 lg:pointer-events-auto lg:border-t-0 lg:border-l ${
        open ? 'translate-y-0' : 'pointer-events-none translate-y-full'
      }`}
    >
      <Panel
        title="BurinMap"
        className="shrink-0"
        aside={(
          <>
            <a is-="button" {...KEY} href="https://github.com/piLeoni/burin-map" target="_blank" rel="noreferrer" title="burin-map on GitHub">
              github ↗
            </a>
            <button type="button" aria-label="Close" {...KEY} className="lg:hidden" onClick={onClose}>✕</button>
          </>
        )}
      >
        <p className="opacity-60">
          Terrain drawn as engraved lines. Pan or zoom the map; the view is
          redrawn when you stop. An online demo of burin-map, a terrain
          engine written in Rust, also available as a command-line tool and
          as Python and Node libraries. Here the same engine runs in your
          browser, compiled to WebAssembly.
        </p>
      </Panel>

      <Panel
        title={dirty ? 'Document ●' : 'Document'}
        className="flex min-h-0 flex-1 flex-col"
        aside={(
          <span role="tablist" className="flex">
            {Object.entries(VIEWS).map(([id, v]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={view === id}
                size-="small"
                variant-={view === id ? 'foreground0' : 'background0'}
                onClick={() => switchView(id)}
              >
                {v.label}
              </button>
            ))}
          </span>
        )}
      >
        <div className="relative flex shrink-0 flex-wrap gap-[1ch]">
          <button type="button" {...KEY} title="Open a YAML, JSON, JSON5 or .sh file" disabled={!schema} onClick={() => fileRef.current?.click()}>
            import
          </button>
          <button type="button" {...KEY} title={`Save as burin-map.${VIEWS[view].ext}`} disabled={!shown} onClick={download}>
            download
          </button>
          <button type="button" {...KEY} title="Copy a link that opens this map and these batches (without comments)" disabled={!schema} onClick={copyLink}>
            share
          </button>
          {/* Static, so the menu opens under the whole row rather than this button. */}
          <details ref={presetMenuRef} is-="popover" className="static">
            <summary is-="button" {...KEY} aria-disabled={!schema} title="Replace the style and the area with a ready-made one">
              presets ▾
            </summary>
            <div className="z-30 w-full border border-black bg-white">
              <ul className="max-h-[60vh] divide-y divide-black overflow-y-auto">
                {PRESETS.map((p) => (
                  <li key={p.id}>
                    <button
                      type="button"
                      size-="small"
                      variant-="background0"
                      className="h-auto w-full flex-col items-start justify-start px-[1ch] py-[0.5lh] text-left hover:bg-black hover:text-white hover:no-underline"
                      onClick={() => loadPreset(p)}
                    >
                      <span className="font-bold">{p.label}</span>
                      <span className="opacity-60">{p.blurb}</span>
                      <span className="opacity-60">{p.screen} on screen · {p.plot} to plot</span>
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          </details>
          <input
            ref={fileRef}
            type="file"
            accept=".yaml,.yml,.json,.json5,.sh,.txt"
            className="hidden"
            onChange={(e) => { importFile(e.target.files?.[0]); e.target.value = '' }}
          />
        </div>
        {notice && (
          <p className={`mt-[0.5lh] shrink-0 break-words ${notice.error ? 'font-bold' : 'opacity-60'}`}>
            {notice.error ? `✗ ${notice.text}` : notice.text}
          </p>
        )}

        <div className="mt-[1lh] flex shrink-0 items-center gap-[1ch]">
          <span
            className={`mr-auto truncate ${dirty || failed ? 'font-bold' : 'opacity-60'}`}
            title={dirty ? undefined : statusText}
          >
            {dirty ? 'Edited, not applied' : failed ? `✗ ${statusText}` : statusText}
          </span>
          <button type="button" {...KEY} disabled={!dirty} onClick={revertDraft}>
            revert
          </button>
          <button type="button" size-="small" variant-={dirty ? 'foreground0' : 'background1'} disabled={!dirty} title="⌘/Ctrl + Enter" onClick={applyDraft}>
            apply
          </button>
        </div>
        {error && <p className="mt-[0.5lh] shrink-0 font-bold break-words">✗ {error}</p>}

        <div className="relative mt-[0.5lh] min-h-0 flex-1 overflow-hidden border-t border-black pt-[0.5lh]">
          {shown && (
            <button
              type="button"
              {...KEY}
              title={`Copy the ${VIEWS[view].label}`}
              className="absolute top-[0.5lh] right-[2ch] z-10"
              onClick={() => copy(shown, VIEWS[view].label)}
            >
              copy
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
                registerDocLanguage(monaco, BURIN_TYPES)
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
            <p className="opacity-40">loading parameters…</p>
          )}
        </div>
      </Panel>

      <p className="shrink-0 border-t border-black py-[0.5lh]">
        <span className="opacity-40">Elevation: AWS Terrain Tiles. Basemap: Mapbox.</span>
      </p>
    </aside>
  )
}
