# Remote access (control the bench from another computer on the network)

Status: contract for the implementation in `crates/bm-server/src/remote/` and
`frontend/src/ui/remote/`.

## What it is

Off by default. When it is switched on in the app (Settings → Remote access) with a password,
the server opens a second listener, **HTTPS on every interface, port 57667**. Any browser on the
local network then opens `https://<computer>:57667/`, signs in with the password and gets
the same bench as the app window: panels, recordings, automation and instrument configuration.
The app window and remote browsers work at the same time; every change made in one shows in
the others, as with two app windows today. A detached panel opens as a browser popup.

An optional second password, the **view-only password**, signs in browsers that see the bench
(panels, live values, recordings, flows, the instrument console) and change nothing: a
supervisor, a student at the next bench, a screen on the wall.

The local listener (`127.0.0.1:57666`, what the app window uses) stays as it is: loopback
only, no password. **Nothing listens on the network without the password any more**:
`BENCHMESH_LISTEN`, `BENCHMESH_ALLOWED_HOSTS` and `benchmesh-server --host` are removed.

## Who may do what: principals and scopes

Every request carries a principal, which is decided by the listener it arrived on and its
session, never by anything the client says:

| Principal | How | Scopes |
|---|---|---|
| `local` | the loopback listener (the app window, scripts and MCP clients on this computer) | all |
| `remote` (role `control`) | the network listener with a session signed in with the remote access password | all except `admin` |
| `remote` (role `view`) | the network listener with a session signed in with the view-only password | reads only: `GET`/`HEAD` (which includes WebSockets) of every scope except `admin`, plus sign-out |
| (future) `online` | a BenchMesh online account (see below) | granted per account, e.g. `instruments` for chosen instruments |

The scopes of the routes (`remote/scope.rs`; routes not listed there are `admin`, so a new route
is local-only until it is classified):

- `instruments`: `/`, `/status`, `/version`, `/instruments/**`, `/ws/registry`, `/catalog`,
  `/drivers/**`, `/devices/{id}/wire`, `/metrics/**`, `/ws/metrics`, `/ai/context`,
  `/ai/system-prompt`, `/openapi.*`, `/docs`, `/redoc`, `/ui/**`, `/remote/session`, `/remote/logout`
- `configuration`: `/config`, `/devices` (add, update, delete, detect), `/serial-ports`,
  `GET /usbtmc-devices`
- `recordings`: `/recordings/**`, `/ws/recordings`
- `automation`: `/automation/**` except the parts below
- `system`: `/system/**` - the computer's resources; for a headless BenchMesh restart, shut down
  and start with the computer (`ai_context/HEADLESS.md`: headless, the browser is the only way to
  look after it). A view-only session sees the resources only.
- `admin` (local only): remote access settings (`/remote`, `/remote/password`,
  `/remote/sessions/**`), `/mcp` and `/automation/mcp` (AI clients get control only from this
  computer), `PUT /automation/assistant` (API keys), `POST /usbtmc-devices/driver` (an
  administrator prompt on this computer)

A remote request for an `admin` route gets `403 {"detail": "... only from the BenchMesh app on
this computer"}`. The UI hides those controls for a remote principal. A view-only session's
request with any other method gets `403 {"detail": "this session is view only: ..."}`, checked
in `remote::gate` before any handler runs; no `GET` changes the bench (queries only read, and the
WebSockets take nothing from the client). The UI shows **Remote - view only** in the top bar,
makes the panel bodies `inert`, and offers no park control or new recording.

## Threats and what stops them

| Risk | Prevention |
|---|---|
| Someone on the same Wi-Fi reads the password or the session | HTTPS only (rustls). The certificate is self-signed, made once per data dir (`remote-tls/`), and its SHA-256 fingerprint is shown in the app so the first browser warning can be checked against it |
| Guessing the password | At least 10 characters; PBKDF2-HMAC-SHA256 (600 000 rounds, random salt, `ring`), constant-time compare; per address: after 5 failures each further try is refused for 1 s, doubling to 15 min; at most 30 failed sign-ins per minute in total, at most two checks at a time (no CPU exhaustion); every failure is logged |
| The port forwarded to the internet, or a hostile public network | Connections from addresses outside loopback, private (10/8, 172.16/12, 192.168/16, 100.64/10), link-local and IPv6 ULA / link-local ranges are dropped before TLS. The app shows who is connected |
| A web page the remote user visits drives the bench (CSRF, WebSocket hijack, clickjacking) | Session cookie `__Host-bm_session; HttpOnly; Secure; SameSite=Strict`; on the network listener every request with `Origin` must be same-origin exactly; `Host` must be an IP literal, single-label or `.local` name (DNS rebinding); `X-Frame-Options: DENY` and `frame-ancestors 'none'` (no framing of the controls); the sign-in page runs no scripts |
| A website rebinds its name to the loopback listener | The loopback listener accepts only loopback `Host` names (it no longer serves the network at all) |
| A stolen or forgotten session | Random 256-bit token; only its SHA-256 is kept (memory only: a restart signs everyone out); idle timeout 12 h, at most 7 days; logout; the app lists sessions and disconnects one or all; changing the password or switching remote access off ends every session, including open WebSockets |
| Remote switches outputs on while someone works at the bench | The top bar of the app shows **Remote: N** while anyone is connected (with addresses) and a one-click **Disconnect all and switch off**. Energising controls still need two presses, flows need `arm` |
| A remote user widens their own access | Remote access, the password, MCP, assistant keys and the Windows driver install are `admin`: only from the app on this computer |
| A remote user stops or re-registers BenchMesh (headless) | Restart / Shut down need a control session, two presses, and are refused while a flow runs; start with the computer only ever runs BenchMesh's own executable with the options it was started with (no command from the request), as the user BenchMesh runs as, without administrator rights |
| A remote user reads secrets | Assistant API keys are never returned (only hints); the password hash and TLS key are never served |
| Local malware or other users of this computer | Out of scope: the loopback listener is as before (another program running as you can already drive the instruments) |
| Simulation | Works the same with in-memory settings, certificate and sessions |

Firewalls: the first time the network listener opens, Windows asks whether to allow it on
private networks.

## Settings and storage

`<data dir>/remote.json` (0600): `{"enabled": bool, "port": 57667, "password": {"salt",
"iterations", "hash"} (hex), "view_password": same or absent}`. A view-only password needs the
remote access password, and the two must differ (a sign-in tries the remote access password
first, then the view-only one). `<data dir>/remote-tls/cert.der` and `key.der` (0600): made
once (it names `localhost` and the addresses the computer had then) and kept, so its fingerprint
stays the one users accepted; delete the folder for a new one. When `enabled` is saved with a
password, the network listener starts at once and again at every start. Headless:
`benchmesh-server remote-password` reads the password from stdin and switches remote access on.

## API

Admin (local only):

- `GET /remote` → `{"enabled", "port" (the bound one while listening), "password_set", "view_password_set", "listening", "error", "urls":
  ["https://192.168.1.20:57667/"], "fingerprint": "AB:CD:...", "sessions": [{"id", "role" ("control" | "view"), "address",
  "agent", "since", "last_seen"}]}` (times in ms since epoch; `id` is not the token)
- `PUT /remote` `{"enabled": bool, "port"?: n}` → as `GET`. Enabling needs a password (409);
  switching off ends every session.
- `PUT /remote/password` `{"password"}` → 204; 400 when shorter than 10 characters; 409 when it
  is the view-only password; ends every session.
- `PUT /remote/view-password` `{"password"}` → 204; 400 when shorter than 10 characters; 409
  without a remote access password or when it is that password; ends every view-only session.
- `DELETE /remote/view-password` → 204; ends every view-only session.
- `DELETE /remote/sessions/{id}`, `DELETE /remote/sessions` → 204

Everyone (both listeners):

- `GET /remote/session` → `{"principal": "local"}` or `{"principal": "remote", "role", "address",
  "since"}`; on the network listener without a session: 401.
- `POST /remote/logout` → 303 to `/remote/login` (ends this session).

Network listener only:

- `GET /remote/login` → the sign-in page (plain HTML, no scripts).
- `POST /remote/login` (form `password`: the remote access password signs in a `control`
  session, the view-only password a `view` one) → 303 to `/ui/` with the session cookie, or 303 back to
  the login page with an error (wrong password, or wait N seconds).
- Without a session, a page navigation goes to `/remote/login` (303); anything else is 401.

## Later: BenchMesh online accounts

When the app gains sign-in with a BenchMesh online account, that account carries over:

- The sign-in page offers "Sign in with BenchMesh online" beside the password. The app checks the
  account's token with the online service; it creates the same `remote` session as the password
  (the owner of the app, all scopes except `admin`). The password stays as a way in that works
  offline.
- Instrument access from the web application is a different principal, `online`, with only the
  `instruments` scope and an allow-list of instrument ids (no recordings, automation or
  configuration). It is enforced by the same scope check: a route must be classified and the
  principal must hold its scope, plus the instrument filter for `instruments` routes.
