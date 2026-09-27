import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { fileURLToPath } from 'node:url'

// Minimal config for the SSR render smoke test: stub the WASM module and
// mapbox-gl, which need a browser.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      'mapbox-gl': fileURLToPath(new URL('./scripts/stub-mapbox.js', import.meta.url)),
      'mapbox-gl/dist/mapbox-gl.css': fileURLToPath(new URL('./scripts/stub-empty.js', import.meta.url)),
      '../lib/burin': fileURLToPath(new URL('./scripts/stub-burin.js', import.meta.url)),
    },
  },
  ssr: {
    noExternal: true,
    external: ['jsdom'],
  },
})
