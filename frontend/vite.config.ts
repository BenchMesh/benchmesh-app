import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The UI talks to the API on its own origin (src/api/base.ts). In development the Vite
// server forwards the API paths to the backend (BENCHMESH_API, default localhost:57666).
const apiTarget = process.env.BENCHMESH_API ?? 'http://localhost:57666'
const apiPaths = [
  '/instruments', '/drivers', '/config', '/devices', '/serial-ports', '/usbtmc-devices',
  '/status', '/version', '/metrics', '/recordings', '/automation', '/mcp', '/remote', '/openapi.json', '/openapi.yaml', '/ai', '/ws',
]
const proxy = Object.fromEntries(apiPaths.map((p) => [p, { target: apiTarget, changeOrigin: true, ws: true }]))

// Use a dev server port unlikely to clash and allow any host (for container use)
export default defineConfig({
  plugins: [react()],
  base: '/ui/',
  server: {
    host: true,
    port: 52893,
    strictPort: true,
    cors: true,
    hmr: {
      clientPort: 52893
    },
    proxy
  },
  preview: {
    port: 52893,
    strictPort: true
  },
  build: {
    rollupOptions: {
      output: {
        // The recordings chart (echarts, tree-shaken) is not listed: it stays in the lazily
        // loaded chunk of `recording/RecordingChart.tsx`.
        manualChunks: {
          // Core React libraries
          'vendor-react': ['react', 'react-dom'],
        }
      }
    },
    // Slightly increase threshold since vendor chunks may be 500-800KB
    chunkSizeWarningLimit: 600
  }
})
