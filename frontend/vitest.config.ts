import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./vitest.setup.ts'],
    // the full suite runs many jsdom files in parallel: interaction-heavy tests need headroom
    testTimeout: 20000,
    // Playwright specs live in e2e/ and run with `npm run test:e2e`, not vitest
    exclude: ['e2e/**', 'node_modules/**', 'dist/**'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      include: ['src/**'],
      exclude: ['e2e/**', 'dist/**']
    }
  }
})
