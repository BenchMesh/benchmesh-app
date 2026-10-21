/** An instrument as listed by `GET /instruments`. */
export type Instrument = {
  id: string
  name?: string
  IDN?: string
  classes: { class: string; channels: string[]; ui_component?: string }[]
  health?: {
    is_alive?: boolean; status?: string; consecutive_failures?: number; last_error?: string
    /** Parked: attached / reachable (found without talking to it); where, or why not. */
    present?: boolean | null; presence?: string | null
  }
  /** Parked: no link, nothing sent to it, only its presence is checked (`POST|DELETE /devices/{id}/park`). */
  parked?: boolean
  /** Port / path / host of the current connection, and its transport (serial, usbtmc, tcpip, vxi11). */
  port?: string
  transport?: string
  /** Model from the definition, once known (identify reply or config). */
  model?: string
  /** How far that model is known to work (ai_context/migration/coverage/README.md). */
  support?: Support
}

/** `tested` maintainers' bench · `confirmed` by a user's check · `awaiting` from the manual only. */
export type Support = 'tested' | 'confirmed' | 'awaiting'
