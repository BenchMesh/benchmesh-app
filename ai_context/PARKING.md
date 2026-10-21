# Parking instruments

A parked instrument stays configured but is left alone: BenchMesh closes its link, sends it
nothing, and only checks now and then whether it is still attached or reachable. Its panels
leave the bench for the parking strip on the left, and go back to their place when it is
unparked.

Use it when an instrument should not be polled for a while. For example, it is used from its
front panel, it is unplugged and should stop the reconnect attempts, or it is on the bench but
not needed and its traffic should not slow the others down.

## Contract

| | |
|---|---|
| State | `parked: true` in the instrument's `config.yaml` entry. It survives restarts, and the app window, detached windows and remote browsers all see the same state. |
| Park | `POST /devices/{id}/park` → `{id, parked: true}`. The device thread finishes its current exchange, then runs the definition's `on_disconnect` steps (e.g. back to local) and releases the port. After that, nothing is sent to the instrument. |
| Unpark | `DELETE /devices/{id}/park` → `{id, parked: false}`. The device thread connects again: identify, `on_connect`, polling. |
| Both | Idempotent (parking twice changes nothing). `404` for an unknown id. `500` when `config.yaml` cannot be saved (the state is not changed then). Scope `Instruments` (remote sessions may park). |
| At start | A parked instrument's port is never opened. |
| Presence | Checked every 5 s (`ActorConfig::presence_every`) without a byte to the instrument (`bm_transport::presence`). A USB instrument with `match` is found by port enumeration (its identity); a fixed `port`/`device` by the system's port list or the device node. A LAN instrument (`tcpip`, `vxi11`) by opening and closing a TCP connection to its port (the portmapper for VXI-11). In a simulation it is always present. Changes are written to the instrument console as info lines. |
| Requests | `GET`/`POST /instruments/{CLS}/{id}/...` of a parked instrument answer `409` "`<id>` is parked: unpark it to use it". |
| `GET /instruments` | Lists the instrument (classes, model) with `parked: true` and `health: {status: "parked", is_alive: false, present: bool \| null, presence: "<where / why not>"}`. |
| `/ws/registry` | `{<id>: {parked: {present, detail}}}`: no `IDN`, no status. |
| `/status` | `parked` count. A parked instrument is not counted as `disconnected`. |
| Config edits | `PUT /devices/{id}` without `parked` keeps the current state (the Instruments form does not know the field). `POST /config` takes the entries as given. |

## Automation

- Parking is refused (`409` "`<id>` is used by the running flow ...") while the active run's
  flow addresses the instrument. That covers set and read steps, and series of a `record`
  step. The run status lists them in `devices`.
- A run whose flow uses a parked instrument is refused at start: `409 {detail, parked: [ids]}`.
  When a flow is validated, a parked instrument is a warning (a flow may be written while an
  instrument is parked).
- The run check and the park are one step under the runner's lock (`Runner::unless_used`), and
  a run start checks for parked instruments under the same lock. A run cannot start between the
  check and the park.

## Edge cases

- **Output on:** parking does not switch anything off. An output that is on stays on, but it is
  no longer watched. The UI asks for a second press when a status `OUTPUT`, `INPUT` or `BIAS_ON`
  reads ON.
- **Long exchange in progress** (e.g. a scope screenshot): the park request waits in the device
  thread's queue. The API answers once it is done, or after the request timeout. A queued park
  is still carried out.
- **Parked while reconnecting:** the reconnect attempts stop at once. This is the way to silence
  an unplugged instrument.
- **Recordings:** a parked instrument's series get no samples (a gap). The recording itself goes on.
- **Detect:** a parked instrument's port is not probed. Detect recognises the instrument by its
  identity wherever it is plugged in, and reports "parked: not probed".
- **Detached panel windows:** a window whose instrument is parked closes. The bench drops it
  from "In own windows".
- **Several panels:** parking is per instrument, so all of its class panels go to the strip
  together. Their order, width, fold state and channel layout are kept.
- **Removing a parked instrument** (`DELETE /devices/{id}`): it is removed without traffic.
- **Changing the port while parked:** the edit keeps it parked. Unparking connects on the new port.

## UI

- **Panel header:** a park icon sits before Fold (bench and own window; not in Arrange mode). It
  is disabled with a reason while the running flow uses the instrument. With an output on, the
  first press shows a warning line and a second press within 5 s parks. A refusal is shown in
  the same line.
- **Parking strip** (`bench/ParkingStrip.tsx`, shown only while something is parked): a 34 px
  column left of the rack, sticky below the top bar. It has a toggle with the count, then one
  tab per instrument with a presence dot and its name written vertically. Opening it shows a
  sheet with name, model, classes, presence ("Attached at ..." / "Not found: ..." / "Checking
  ...") and **Unpark**. Escape closes the sheet.
- **State:** taken from the live registry (pushed every second) once it lists the instrument,
  from `GET /instruments` until then (`bench/parking.ts`).
