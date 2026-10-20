import React from 'react'
import SwaggerUI from 'swagger-ui-react'
import 'swagger-ui-react/swagger-ui.css'

interface DocsViewerProps {
  onClose: () => void
  apiBase: string
}

/** The running server's API reference (its OpenAPI document). Guides live on the website. */
export function DocsViewer({ onClose, apiBase }: DocsViewerProps) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="docs-modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="docs-header">
          <h2>API reference</h2>
          <button className="modal-close" onClick={onClose} aria-label="Close">×</button>
        </div>
        <div className="docs-content">
          <div className="docs-swagger-wrapper">
            <p className="docs-api-intro">
              Every endpoint of this BenchMesh server. Guides for flows, the assistant and MCP: <b>benchmesh.online/docs</b>
            </p>
            <div className="docs-swagger-container">
              <SwaggerUI url={`${apiBase}/openapi.json`} />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
