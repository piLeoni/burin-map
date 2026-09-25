import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 8765,
    proxy: {
      // AWS S3 answers CORS preflights but not GETs — proxy so we can read pixels.
      '/terrarium': {
        target: 'https://s3.amazonaws.com/elevation-tiles-prod',
        changeOrigin: true,
        rewrite: (p) => p.replace(/^\/terrarium/, '/terrarium'),
      },
    },
  },
  // WASM lives in /pkg (built by wasm-pack), not processed by Vite.
  publicDir: 'public',
})
