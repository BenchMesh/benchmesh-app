import '@testing-library/jest-dom'
import '@testing-library/jest-dom/vitest'
import { configure } from '@testing-library/react'

// findBy*/waitFor: 1 s is too tight when the whole suite runs in parallel
configure({ asyncUtilTimeout: 5000 })

// jsdom has no canvas: kit components draw nothing when getContext returns null.
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext
