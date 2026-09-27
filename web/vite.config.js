import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['icons/apple-touch-icon.png', 'icons/icon-192.png', 'icons/icon-512.png'],
      manifest: {
        name: 'BurinMap',
        short_name: 'BurinMap',
        description: 'Pen-and-ink terrain from elevation',
        theme_color: '#ffffff',
        background_color: '#ffffff',
        display: 'standalone',
        orientation: 'any',
        start_url: '/',
        icons: [
          { src: '/icons/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png' },
          {
            src: '/icons/icon-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
      workbox: {
        maximumFileSizeToCacheInBytes: 4 * 1024 * 1024,  // WASM can be large
        navigateFallback: '/index.html',
        globPatterns: ['**/*.{js,css,html,ico,png,svg,wasm}'],
        runtimeCaching: [
          {
            urlPattern: /\/terrarium\/.*/i,
            handler: 'CacheFirst',
            options: {
              cacheName: 'terrarium-tiles',
              expiration: { maxEntries: 256, maxAgeSeconds: 60 * 60 * 24 * 30 },
            },
          },
        ],
      },
    }),
  ],
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
