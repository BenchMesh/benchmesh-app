# Virtual SCPI instruments (VISA programs: MATLAB, LabVIEW, PyVISA, TestStand)

Status: contract for the implementation in `crates/bm-server/src/scpi/` and
`frontend/src/ui/connectors/`.

## What it is

Every configured instrument can answer SCPI on its own raw TCP socket on this computer, the
way LAN instruments do on port 5025. A VISA program opens it as
`TCPIP0::127.0.0.1::<port>::SOCKET`, whatever the instrument's own link (serial, USBTMC, LAN)
and dialect: a Tenma or OWON serial supply that speaks no SCPI gains SCPI through its definition.
Commands run through the same device thread as the REST API, so the bench, recordings and the
instrument console keep showing what happens, and the definition's value specs refuse bad
values before anything is sent.

Off by default. Instruments → Connectors (or `PUT /scpi`) sets the access:

| access | |
|---|---|
| `off` (default) | no port is open |
| `read` | queries, `FETCh` and the common commands; setters answer `-203` |
| `control` | also setters, including those that switch an output on (as the REST API) |

Ports: `base_port` (default 5025) for the first instrument, the next free port for each further
one. A port stays with its instrument (`<data dir>/scpi.json`: `{access, base_port, ports: {id:
port}}`) while the instrument is configured; a new base renumbers all. A simulation keeps the
settings in memory.

## Safety

- **This computer only**: the sockets bind `127.0.0.1`; a peer that is not loopback is dropped.
  Other computers use remote access (HTTPS, password) or the Python package through it.
- **Cross-protocol requests**: a web page can make the browser send an HTTP request to any local
  port, with a body of its choice. A connection whose line is an HTTP request line (`POST /
  HTTP/1.1`) or a `Host:` / `Origin:` header is closed before anything runs.
- **Flows**: a setter for an instrument the running flow uses answers `-221` (as parking does).
- **Limits**: at most 8 connections per instrument; a program message over 64 KiB closes the
  connection.
- `GET|PUT /scpi` are `admin` (this computer only, `remote/scope.rs`).

## Dialect

Lines end with LF (CR LF accepted); replies end with LF. Case does not matter. A program message
holds several commands separated by `;`; the answers of its queries come back in one line,
separated by `;`. Every command after a `;` is complete (no SCPI path inheritance).

| Command | Answer |
|---|---|
| `*IDN?` | `BenchMesh,<model>,<device id>,<app version>` |
| `*CLS` | clears the error queue |
| `*OPC?` | `1` (commands run in order) |
| `*OPC`, `*WAI` | nothing to wait for |
| `SYSTem:ERRor[:NEXT]?` | `0,"No error"` or the oldest error, e.g. `-222,"Data out of range; ..."` |
| `SYSTem:VERSion?` | `1999.0` |
| `SYSTem:HELP:HEADers?` | every header of this instrument, one line, comma separated |
| `[<CLS>:][CH<n>:]<op>?` | runs the definition's query operation |
| `[<CLS>:][CH<n>:]<op> <value>` | runs its setter (`VOLTAGE 5`, `OUTPUT ON`, `MODE "VOLT DC"`) |
| `[<CLS>:][CH<n>:]FETCh:<field>?` | the latest polled status field (`FETCH:VOUT?`), nothing sent to the instrument |

- `<op>` and `<field>` are the definition's operation and status names (`/methods`, `/status`);
  no SCPI short forms. `CH<n>` (also `CHAN<n>`, `CHANNEL<n>`) defaults to channel 1.
- `<CLS>` may be left out when the instrument has one class, or when only one of its classes
  knows the name; otherwise `-113` names the classes to choose from.
- Values: SI values and numbers as numbers in base units (`4.94`, `1E-6`), booleans `1`/`0`,
  text as the instrument reports it. Binary data (screenshots) is an IEEE 488.2 definite-length
  block `#<digits><length><bytes>`.
- **A query that fails answers an empty field** (an empty line) instead of leaving the program
  waiting for a timeout; `SYST:ERR?` then says why.
- Error queue per connection, 20 entries (the last becomes `-350,"Queue overflow"`).

| Error | When |
|---|---|
| `-102` Syntax error | empty or malformed header |
| `-108` Parameter not allowed | a value after a query |
| `-109` Missing parameter | a setter without a value |
| `-113` Undefined header | unknown command, operation or field; no query / no setter; class ambiguous |
| `-114` Header suffix out of range | channel above the class's channels |
| `-203` Command protected | a setter while access is `read` |
| `-221` Settings conflict | refused by the definition (e.g. a mode precondition), parked, used by the running flow |
| `-222` / `-224` | value outside the setter's range / not a valid value |
| `-230` Data corrupt or stale | no fresh status for `FETCh` (offline), malformed reply |
| `-240` / `-241` | instrument timeout / instrument not reachable |
| `-200` Execution error | anything else |

## API

- `GET /scpi` → `{access, base_port, host, instruments: [{id, name, port, address, listening, error}]}`
- `PUT /scpi` `{access?: off|read|control, base_port?: 1024-65000}` → as GET; 422 on bad values.
- `GET /instruments/{CLS}/{id}/status` (used by `FETCh` clients and the Python package):
  the latest polled status per channel with its update time.
