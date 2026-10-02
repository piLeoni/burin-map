import { useMemo, useState } from 'react'
import { useAtomValue, useSetAtom } from 'jotai'
import MapView from './components/MapView'
import Sidebar from './components/Sidebar'
import PasswordGate from './components/PasswordGate'
import { bathymetryAtom, drawingAtom, statusAtom, styleAtom, waterAtom } from './state'

export default function App() {
  const batches = useAtomValue(styleAtom)
  // Names are labels only; renaming a batch must not trigger a redraw.
  const drawKey = JSON.stringify(
    batches.map(({ name: _name, ...drawn }) => drawn),
  )
  const drawn = useMemo(() => JSON.parse(drawKey), [drawKey])
  const water = useAtomValue(waterAtom)
  const waterKey = JSON.stringify(water && {
    ...water,
    batches: water.batches.map(({ name: _name, ...drawn }) => drawn),
  })
  const wet = useMemo(() => JSON.parse(waterKey), [waterKey])
  const bathymetry = useAtomValue(bathymetryAtom)
  const status = useAtomValue(statusAtom)
  const drawing = useAtomValue(drawingAtom)
  const setStatus = useSetAtom(statusAtom)
  const setDrawing = useSetAtom(drawingAtom)
  const [menuOpen, setMenuOpen] = useState(false)

  return (
    <PasswordGate>
      <div className="relative flex h-full min-h-0 w-full flex-col lg:flex-row">
        <main className="relative min-h-0 min-w-0 flex-1">
          <MapView batches={drawn} water={wet} bathymetry={bathymetry} onStatus={setStatus} onDrawing={setDrawing} drawing={drawing} />

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
