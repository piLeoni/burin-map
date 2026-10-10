import { BrowserRouter, Route, Routes } from 'react-router-dom'
import App from './App.jsx'
import Docs from './docs/Docs.jsx'

export default function Root() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<App />} />
        <Route path="/docs/*" element={<Docs />} />
      </Routes>
    </BrowserRouter>
  )
}
