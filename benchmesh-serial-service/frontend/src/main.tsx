import React from 'react'
import { createRoot } from 'react-dom/client'
import App from './ui/App'
import { DocsPage } from './ui/DocsPage'
import { MetricsPage } from './ui/MetricsPage'
import { PanelWindow } from './ui/bench/PanelWindow'
import { SkinProvider } from './ui/kit/skin'
import { RequestLogProvider } from './ui/RequestLogContext'

// Simple routing: check URL path to determine which component to render
const path = window.location.pathname

const root = createRoot(document.getElementById('root')!)

// One instrument panel in its own window: /ui/?panel=<instrumentId:CLASS>
const panel = new URLSearchParams(window.location.search).get('panel')

// Render appropriate page based on URL path
if (panel) {
  root.render(<RequestLogProvider><SkinProvider><PanelWindow panelKey={panel} /></SkinProvider></RequestLogProvider>)
} else if (path === '/ui/metrics' || path === '/ui/metrics/') {
  root.render(<MetricsPage />)
} else if (path === '/ui/docs' || path === '/ui/docs/') {
  root.render(<DocsPage />)
} else {
  root.render(<App />)
}
//root.render(<React.StrictMode><App /></React.StrictMode>)