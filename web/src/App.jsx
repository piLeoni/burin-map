import { useCallback, useState } from 'react'
import MapView from './components/MapView'
import Sidebar from './components/Sidebar'

export default function App() {
  const [style, setStyle] = useState({})
  const [status, setStatus] = useState({ phase: 'loading', detail: 'Starting…' })
  const [drawing, setDrawing] = useState(null)
  const onStatus = useCallback((s) => setStatus(s), [])
  const onDrawing = useCallback((d) => setDrawing(d), [])

  return (
    <div className="flex h-full min-h-0 flex-col lg:flex-row">
      <main className="min-h-0 min-w-0 flex-1">
        <MapView style={style} onStatus={onStatus} onDrawing={onDrawing} />
      </main>
      <Sidebar style={style} setStyle={setStyle} status={status} drawing={drawing} />
    </div>
  )
}
