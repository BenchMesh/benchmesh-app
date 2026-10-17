import '@testing-library/jest-dom'
import '@testing-library/jest-dom/vitest'

// jsdom has no canvas: kit components draw nothing when getContext returns null.
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext
