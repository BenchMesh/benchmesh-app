import React, { useState } from 'react'
import { RequestLogProvider } from './RequestLogContext'
import { BenchLayout } from './bench/BenchLayout'
import { DisclaimerModal } from './DisclaimerModal'
import { disclaimerHidden, setDisclaimerHidden } from './disclaimer'
import { useApiBase, useInstruments, useRegistrySocket } from './data'
import { RecordingsProvider } from './recording/RecordingsProvider'

export default function App() {
  const apiBase = useApiBase()
  const [refreshTrigger, setRefreshTrigger] = useState(0)
  const { data: instruments, loading, error } = useInstruments(apiBase, refreshTrigger)
  const { registry, wsDiag } = useRegistrySocket(apiBase)

  // Start screen (welcome and safety information) unless hidden with "don't show again"
  const [disclaimerOpen, setDisclaimerOpen] = useState(() => !disclaimerHidden())

  function handleConfigUpdated() {
    setRefreshTrigger(prev => prev + 1)
  }

  function handleDisclaimerAccept(dontShowAgain: boolean) {
    setDisclaimerOpen(false)
    if (dontShowAgain) setDisclaimerHidden(true)
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
