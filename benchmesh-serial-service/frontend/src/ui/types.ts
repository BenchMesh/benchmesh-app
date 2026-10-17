/** An instrument as listed by `GET /instruments`. */
export type Instrument = {
  id: string
  name?: string
  IDN?: string
  classes: { class: string; channels: string[]; ui_component?: string }[]
  health?: { is_alive?: boolean; status?: string; consecutive_failures?: number }
}
