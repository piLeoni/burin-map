import { useEffect, useRef, useState } from 'react'
import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import Editor from '@monaco-editor/react'
import { EDITOR_THEME, defineEditorTheme, editorFont, registerDocLanguage } from '../lib/editorTheme'
import BURIN_TYPES from '../lib/burin.d.ts?raw'
import { PRESETS } from '../lib/presets'
import { atlasToDoc, atlasToScript, docToAtlas, runJs } from '../lib/document'
import { docFromHash, docToHash } from '../lib/doc'
import { areaFromFrame, frameFromArea } from '../lib/frame'
import Panel from './Panel'
import {
  appliedScriptAtom, areaAtom, atlasAtom, cameraRequestAtom, editorAtom, frameAtom,
  loadPersistedDocument, mapViewAtom,
} from '../state'

const PHASE_LABEL = {
  loading: 'Starting…',
  working: 'Drawing…',
  ready: 'Up to date!',
  exploring: 'Redraws when you stop',
  error: 'Something went wrong',
}

/** A link's drawing as a script, so the editor shows something it can run. */
function scriptFromLink(doc) {
  return atlasToScript(docToAtlas(doc), doc.area ?? null)
}

/** Read a document from the URL fragment, then clear it. */
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

/* ---------------------------------------------------------------- */
/*  Sidebar — the document, as a JavaScript script                   */
/* ---------------------------------------------------------------- */

let documentBooted = false

export default function Sidebar({ open, onClose, status }) {
  const [editor, setEditor] = useAtom(editorAtom)
  const [applied, setApplied] = useAtom(appliedScriptAtom)
  const atlas = useAtomValue(atlasAtom)
  const [, setAtlas] = useAtom(atlasAtom)
  const [, setArea] = useAtom(areaAtom)
  const [mapView] = useAtom(mapViewAtom)
  const [frame, setFrame] = useAtom(frameAtom)
  const setCameraRequest = useSetAtom(cameraRequestAtom)

  const [error, setError] = useState('')
  const [notice, setNotice] = useState(null)   // { text, error? }
  const fileRef = useRef(null)
  const presetMenuRef = useRef(null)
  const noticeTimer = useRef(0)
  const applyDraftRef = useRef(() => {})
  const dirty = editor !== applied

  const flash = (text, isError = false) => {
    clearTimeout(noticeTimer.current)
    setNotice({ text, error: isError })
    noticeTimer.current = setTimeout(() => setNotice(null), isError ? 8000 : 3000)
  }

  /** Point the map and the frame at a script's area. Throws when it can't be drawn. */
  const applyArea = (area) => {
    setArea(area ?? null)
    if (!area || Object.keys(area).length === 0) return
    const { center, frame: next } = frameFromArea(area, frame)
    setFrame(next)
    if (center) setCameraRequest({ center })
  }

  /** Run a script and make its atlas the current drawing. Throws on a bad script. */
  const loadScript = async (text, { precomputed = null } = {}) => {
    if (precomputed) {
      setAtlas(precomputed.atlas)
      applyArea(precomputed.area)
      return
    }
    const { atlas, area } = await runJs(text)
    setAtlas(atlas)
    applyArea(area)
  }

  // ---------- init ----------
  useEffect(() => {
    if (documentBooted) return
    documentBooted = true

    // A #d=… fragment wins over whatever is in localStorage; otherwise read
    // storage synchronously so we never overwrite a saved document with the preset.
    const fromHash = takeDocFromHash()
    if (fromHash) {
      const source = scriptFromLink(fromHash)
      setEditor(source)
      setApplied(source)
      setAtlas(docToAtlas(fromHash))
      applyArea(fromHash.area ?? null)
      return
    }

    const stored = loadPersistedDocument()
    if (!stored.editor.trim()) {
      const p = PRESETS[0]
      setEditor(p.doc)
      setApplied(p.doc)
      if (p.atlas) {
        setAtlas(p.atlas)
        applyArea(null)
      }
      return
    }

    setEditor(stored.editor)
    setApplied(stored.applied)
    const draw = stored.applied.trim() ? stored.applied : stored.editor
    const preset = PRESETS.find((p) => p.doc === draw)
    const precomputed = preset?.atlas ? { atlas: preset.atlas, area: null } : null
    loadScript(draw, precomputed ? { precomputed } : {})
      .catch((e) => setError(e.message || 'Can\'t read this'))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const applyDraft = async () => {
    if (!dirty) return
    try {
      await loadScript(editor)
      setApplied(editor)
      setError('')
    } catch (e) {
      setError(e.message || 'Can\'t read this')
    }
  }
  applyDraftRef.current = applyDraft

  const revertDraft = () => {
    setEditor(applied)
    setError('')
  }

  const loadPreset = async (p) => {
    presetMenuRef.current?.removeAttribute('open')
    setError('')
    setEditor(p.doc)
    setApplied(p.doc)
    try {
      await loadScript(p.doc, p.atlas ? { precomputed: { atlas: p.atlas, area: null } } : {})
      flash(`Loaded ${p.label}`)
    } catch (e) {
      setError(e.message || 'Can\'t run this')
    }
  }

  // ---------- files, clipboard, link ----------
  const importFile = async (file) => {
    if (!file) return
    try {
      const text = await file.text()
      const looksLikeLink = /(?:^|[#&])(d|doc)=/.test(text.trim())
      const source = looksLikeLink ? scriptFromLink(docFromHash(text.trim().replace(/^[^#]*/, ''))) : text
      await loadScript(source)
      setEditor(source)
      setApplied(source)
      setError('')
      flash(`Loaded ${file.name}`)
    } catch (e) {
      flash(`Can't open ${file.name}: ${e.message || 'not a burin-map document'}`, true)
    }
  }

  const download = () => {
    const url = URL.createObjectURL(new Blob([editor], { type: 'text/javascript' }))
    const a = document.createElement('a')
    a.href = url
    a.download = 'burin-map.js'
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
    if (!atlas) {
      flash('Apply the script first', true)
      return
    }
    const area = areaFromFrame(frame, mapView?.center)
    copy(`${loc.origin}${loc.pathname}${docToHash(atlasToDoc(atlas, area))}`, 'Link')
  }

  // A link pasted into an open tab only changes the hash, which doesn't reload.
  useEffect(() => {
    const win = globalThis.window
    if (!win?.addEventListener) return
    const onHash = () => {
      const doc = takeDocFromHash()
      if (!doc) return
      const source = scriptFromLink(doc)
      setEditor(source)
      setApplied(source)
      loadScript(source, { precomputed: { atlas: docToAtlas(doc), area: doc.area ?? null } })
        .then(() => flash('Opened URL'))
        .catch((e) => flash(`Can't open this URL: ${e.message || 'unreadable'}`, true))
    }
    win.addEventListener('hashchange', onHash)
    return () => win.removeEventListener('hashchange', onHash)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

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
            <a is-="button" {...KEY} href="/docs" title="Style guide: spacing, ink, presets">
              docs
            </a>
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
      >
        <div className="relative flex shrink-0 flex-wrap gap-[1ch]">
          <button type="button" {...KEY} title="Open a .js document" onClick={() => fileRef.current?.click()}>
            import
          </button>
          <button type="button" {...KEY} title="Save as burin-map.js" disabled={!editor} onClick={download}>
            download
          </button>
          <button type="button" {...KEY} title="Copy a link that opens this map and these batches" onClick={copyLink}>
            share
          </button>
          {/* Static, so the menu opens under the whole row rather than this button. */}
          <details ref={presetMenuRef} is-="popover" className="static">
            <summary is-="button" {...KEY} title="Replace the document with a ready-made one">
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
            accept=".js,.mjs,.txt"
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
          {editor && (
            <button
              type="button"
              {...KEY}
              title="Copy the document"
              className="absolute top-[0.5lh] right-[2ch] z-10"
              onClick={() => copy(editor, 'Document')}
            >
              copy
            </button>
          )}
          <Editor
            height="100%"
            language="burin-doc"
            theme={EDITOR_THEME}
            value={editor}
            onChange={(v) => { setEditor(v ?? ''); setError('') }}
            beforeMount={(monaco) => {
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
        </div>
      </Panel>

      <p className="shrink-0 border-t border-black py-[0.5lh]">
        <span className="opacity-40">Elevation: AWS Terrain Tiles. Basemap: Mapbox.</span>
      </p>
    </aside>
  )
}

/** WebTUI button looks: a grey key. */
const KEY = { 'size-': 'small', 'variant-': 'background1' }
