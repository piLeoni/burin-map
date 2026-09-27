import { useState } from 'react'
import { useAtomValue, useSetAtom } from 'jotai'
import MapView from './components/MapView'
import Sidebar from './components/Sidebar'
import PasswordGate from './components/PasswordGate'
import { drawingAtom, firstStyle, statusAtom, styleAtom } from './state'

export default function App() {
  const batches = useAtomValue(styleAtom)
  const status = useAtomValue(statusAtom)
  const drawing = useAtomValue(drawingAtom)
  const setStatus = useSetAtom(statusAtom)
  const setDrawing = useSetAtom(drawingAtom)
  const [menuOpen, setMenuOpen] = useState(false)

  return (
    <PasswordGate>
      <div className="relative flex h-full min-h-0 w-full flex-col lg:flex-row">
        <main className="relative min-h-0 min-w-0 flex-1">
          <MapView batches={batches} onStatus={setStatus} onDrawing={setDrawing} drawing={drawing} />

          <button
            type="button"
            className="btn btn-sm safe-fab absolute right-4 z-20 border border-base-content bg-base-100 lg:hidden"
            onClick={() => setMenuOpen(true)}
          >
            Controls
          </button>
        </main>

        {menuOpen ? (
          <button
            type="button"
            aria-label="Close controls"
            className="absolute inset-0 z-30 bg-base-content/25 lg:hidden"
            onClick={() => setMenuOpen(false)}
          />
        ) : null}

        <Sidebar
          open={menuOpen}
          onClose={() => setMenuOpen(false)}
          status={status}
          drawing={drawing}
        />
      </div>
    </PasswordGate>
  )
}
