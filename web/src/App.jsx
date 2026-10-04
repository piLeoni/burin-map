import { useMemo, useState } from 'react'
import { useAtomValue, useSetAtom } from 'jotai'
import MapView from './components/MapView'
import Sidebar from './components/Sidebar'
import PasswordGate from './components/PasswordGate'
import { atlasBatches, atlasWater } from './lib/document.js'
import { atlasAtom, drawingAtom, statusAtom } from './state'

export default function App() {
  const atlas = useAtomValue(atlasAtom)
  // Names are labels only; renaming a batch must not trigger a redraw.
  const batches = useMemo(() => atlasBatches(atlas), [atlas])
  const water = useMemo(() => atlasWater(atlas), [atlas])
  const status = useAtomValue(statusAtom)
  const drawing = useAtomValue(drawingAtom)
  const setStatus = useSetAtom(statusAtom)
  const setDrawing = useSetAtom(drawingAtom)
  const [menuOpen, setMenuOpen] = useState(false)

  return (
    <PasswordGate>
      <div className="relative flex h-full min-h-0 w-full flex-col lg:flex-row">
        <main className="relative min-h-0 min-w-0 flex-1">
          <MapView batches={batches} water={water} bathymetry={null} onStatus={setStatus} onDrawing={setDrawing} drawing={drawing} />

          <button
            type="button"
            size-="small"
            className="safe-fab absolute right-4 z-20 lg:hidden"
            onClick={() => setMenuOpen(true)}
          >
            controls
          </button>
        </main>

        {menuOpen ? (
          <button
            type="button"
            aria-label="Close controls"
            className="absolute inset-0 z-30 h-full w-full bg-black/25 bg-none lg:hidden"
            onClick={() => setMenuOpen(false)}
          />
        ) : null}

        <Sidebar
          open={menuOpen}
          onClose={() => setMenuOpen(false)}
          status={status}
        />
      </div>
    </PasswordGate>
  )
}
