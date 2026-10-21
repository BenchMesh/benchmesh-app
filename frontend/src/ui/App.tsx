import React, { useState } from 'react'
import { RequestLogProvider } from './RequestLogContext'
import { BenchLayout } from './bench/BenchLayout'
import { DisclaimerModal } from './DisclaimerModal'
import { useApiBase, useInstruments, useRegistrySocket } from './data'
import { RecordingsProvider } from './recording/RecordingsProvider'

export default function App() {
  const apiBase = useApiBase()
  const [refreshTrigger, setRefreshTrigger] = useState(0)
  const { data: instruments, loading, error } = useInstruments(apiBase, refreshTrigger)
  const { registry, wsDiag } = useRegistrySocket(apiBase)

  // Disclaimer modal state - shows on first app load if not previously accepted
  const [disclaimerOpen, setDisclaimerOpen] = useState(() => {
    try {
      return localStorage.getItem('benchmesh:disclaimer-accepted') !== 'true'
    } catch {
      return true
    }
  })

  function handleConfigUpdated() {
    setRefreshTrigger(prev => prev + 1)
  }

  function handleDisclaimerAccept(dontShowAgain: boolean) {
    setDisclaimerOpen(false)
    if (dontShowAgain) {
      try { localStorage.setItem('benchmesh:disclaimer-accepted', 'true') } catch { /* storage unavailable */ }
    }
  }

  return (
    <RequestLogProvider>
      <RecordingsProvider>
        <BenchLayout
          instruments={instruments || []}
          registry={registry}
          apiBase={apiBase}
          loading={loading}
          error={error}
          wsDiag={wsDiag}
          onConfigUpdated={handleConfigUpdated}
        />

        {/* Disclaimer modal - highest priority, shown on first app load */}
        <DisclaimerModal isOpen={disclaimerOpen} onAccept={handleDisclaimerAccept} />
      </RecordingsProvider>
    </RequestLogProvider>
  )
}
