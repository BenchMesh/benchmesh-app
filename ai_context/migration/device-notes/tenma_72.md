# Device notes: TENMA 72-series PSU (`tenma_72`, class PSU)

Input for the Phase 2 device definition. Bench unit: **72-2540, firmware V2.1** (device id `tenma-72-2540`).

**Sources**
- *protocol*: `BenchMeshDOC/tenma_72/doc/Series+Protocol+V2.0+of+Remote+Control.pdf` (2 pp.)
- *manual*: `BenchMeshDOC/tenma_72/doc/TENMA_72-2540_User_Manual.pdf` (10 pp., V1.0 19/05/16)
- *driver*: `benchmesh-serial-service/src/benchmesh_service/drivers/tenma_72/{driver.py,manifest.json}`
- *RO capture*: `captures/20261016-201609` (read-only) · *SET capture*: `captures/20261016-203514` (setters)
- *probe*: the separate timing/format probes recorded in the plan Log (not stored as a session)

Precedence: hardware > protocol > manual > driver.

---

## 1. Identity

| | |
|---|---|
| Query | `*IDN?` (protocol p.2 #10) |
| Observed reply | `TENMA 72-2540 V2.1` (18 B, no terminator, no serial number) (RO + SET capture, 5/5) |
| Protocol example | `TENMA 72-2535 V2.0` (protocol p.2). Its description says "Returns the KA3005P identification", which shows the family is a Korad KA3005P OEM. |
| Manual example | `72-10480 SN:xxxxxxxx Vx.xx` (manual p.8). It includes a serial number and no vendor. The 72-10480 has no remote interface (manual p.3), so this example is unreliable. |
| Parse | `^TENMA (72-\d{4,5}) V(\d+\.\d+)$` → model, firmware |

Manifest `id_patterns` are prefix/substring matches: `TENMA 72-2535`, `72-2540`, `72-2545`, `72-2550`, `72-2705`, `72-2710`, `72-2930`, `72-2940`, `72-13320`, `72-13330`, `72-13360`. Only 72-2540 has been verified. The 5-digit models (72-13320/13330 are 3-channel) are unverified in both reply format and channel addressing.
Model limits in the manifest (V/A/W): 2535 30/3/90 · 2540 30/5/150 · 2545 60/2/120 · 2550 60/3/180 · 2705 30/3/90 · 2710 30/5/150 · 2930 30/10/300 · 2940 60/5/300 · 13320 30/3/195 · 13330 30/5/315 · 13360 60/15/300. The manual confirms only 2535/2540/2545/2550 (manual p.9).

## 2. Link

| | Manual / protocol | Observed (72-2540 V2.1) |
|---|---|---|
| Physical | USB (CDC virtual COM) / RS232, rear (manual p.3, p.8) | `/dev/serial/by-id/usb-Nuvoton_USB_Virtual_COM_…` |
| Serial | 9600 8N1, no flow control (manual p.8) | same. DTR/RTS forced low by Python, ok=true (SET capture). Effect not tested. |
| TX terminator | not specified | **none**. Commands are sent as bare ASCII. |
| RX terminator | not specified | **none**. Replies have a fixed length per command (see §3). |
| Reply latency | — | first byte ≈30 ms; whole exchange 30–45 ms (probe) |
| Query→query gap | — | 0 ms is fine (0/30 failures), provided the full fixed-length reply has been read first (probe) |
| Setter→next cmd | — | **≥50 ms**. At 20 ms the setter was lost 10/10 times (probe). |
| Setter reply | not specified | **none** (VSET/ISET/OUT: `no_reply`, SET capture) |
| Remote mode | Entered automatically on USB connect: beep, then panel keys locked. Exit by closing the port or unplugging; a beep follows (manual p.8). | No explicit command. Panel locked during capture (assumed, consistent with STATUS bit5=0). |

DSL framing: `tx: {terminator: "", min_gap_after_set_ms: 50}`. For `rx: {mode: fixed, bytes: N}` per command, trim the trailing space. Use an idle-gap fallback (≈10 ms; 1 byte = 1.04 ms at 9600) for models whose reply widths are unverified, e.g. 72-2930 at 10 A or the 60 V models.

## 3. Commands

`X` = channel (1 on single-output models). All replies are ASCII without terminator unless noted. Numeric replies are zero-padded fixed width.

| Command | Params | Reply (format, real example) | Side effects / preconditions | Ref |
|---|---|---|---|---|
| `*IDN?` | — | 18 B `TENMA 72-2540 V2.1` | — | protocol p.2 #10 |
| `VOUT<X>?` | X int | 5 B `NN.NN` V, actual terminal voltage: `00.00` (output off), `05.00` (on, VSET 5) | Reads 0 while output is off | protocol p.1 #6; RO/SET capture |
| `IOUT<X>?` | X int | 5 B `N.NNN` A: `0.000` | — | protocol p.1 #5; capture |
| `VSET<X>?` | X int | 5 B `NN.NN` V: `04.94`, `05.00` | — | protocol p.1 #4; capture |
| `ISET<X>?` | X int | **6 B** `N.NNN␠`, trailing space: `0.988 `, `0.500 ` | — | protocol p.1 #2; capture |
| `VSET<X>:<v>` | v float, 0…V_max (30 V on 2540), **≤2 decimals**, 10 mV resolution (manual p.9) | none | Accepted: `5`, `5.0`, `05.10`, `4.94`. **Silently ignored:** `12.345` (3 dp), `1.5e1`. Wait ≥50 ms before the next command. Setting V above the OVP level turns the output off (manual p.7). | protocol p.1 #3; SET capture; probe |
| `ISET<X>:<i>` | i float, 0…I_max (5 A on 2540), **≤3 decimals**, 1 mA resolution (manual p.9) | none | Accepted: `0.5`, `0.988`. Use `%.3f`. 4+ dp presumably ignored (not tested). ≥50 ms gap. | protocol p.1 #1; SET capture |
| `OUT<b>` | b ∈ {0,1} | none | Global, no channel. `OUT1` → VOUT follows VSET; STATUS 0x10→0x51. `OUT0` → 0x11. ≥50 ms gap. | protocol p.2 #8; SET capture |
| `STATUS?` | — | **1 B binary** bitfield (§4): `0x10`, `0x11`, `0x51` (=`'Q'`) | — | protocol p.2 #9; capture |
| `OCP<b>` | b ∈ {0,1} | none (presumed) | Enables over-current protection. Trip level = ISET (manual p.9 FAQ Q4: OCP trips immediately if ISET is too small). The OCP LED lights (manual p.4). **Never sent on the wire** (Python `is` bug). The protocol text "Stores the panel setting" is a copy-paste error. | protocol p.2 #13; manual p.4, p.9 |
| `OVP<b>` | b ∈ {0,1} | none (presumed) | Enables over-voltage protection. On trip the output cuts off and the OVP LED flashes (manual p.4). No command sets an OVP level; the level is presumably VSET or a panel value (open Q). Never sent. | protocol p.2 #14; manual p.4, p.7 |
| `BEEP<b>` | b ∈ {0,1} | none (presumed) | Panel equivalent: hold OCP for 2 s (manual p.7). Reflected in STATUS bit4. Driver comment says "doesnt work". | protocol p.2 #7; manual p.5, p.7 |
| `SAV<n>` | n int 1…5 | none (presumed) | Stores VSET/ISET/beep/knob mode. Output and lock are always stored as off (manual p.7). Driver comment: "doesnt work". | protocol p.2 #12; manual p.7 |
| `RCL<n>` | n int 1…5 | none (presumed) | **Turns the output off** (manual p.7, p.8). The panel has M1–M4 buttons; M5 is reached via the knob (manual p.8). | protocol p.2 #11; manual p.7–8 |
| LOCK / remote | — | — | **No documented command.** The panel locks automatically in remote mode (manual p.8). The panel LOCK key exists; unlock by holding it 2 s (manual p.7). | manual p.7–8 |
| `*RST` | — | — | **Not supported** by the protocol, but DriverBase exposes `set_reset`. Exclude it from the definition. | protocol (absent) |

Formatting rule for the definition: send `VSET1:%.2f` and `ISET1:%.3f`, clamped to the model limits. Reject values that would round to a different value, and never use exponent notation.

## 4. STATUS? bitfield

Per the manual (protocol p.2 #9):

| Bit | Mask | Meaning (manual) |
|---|---|---|
| 0 | 0x01 | CH1: 0=CC, 1=CV |
| 1 | 0x02 | CH2: 0=CC, 1=CV |
| 2–3 | 0x0C | Tracking: 00 independent, 01 series, 11 parallel |
| 4 | 0x10 | Beep: 1=on |
| 5 | 0x20 | Lock: **0=Lock, 1=Unlock** |
| 6 | 0x40 | Output: 1=on |
| 7 | 0x80 | N/A |

Observed (72-2540 V2.1, no load, VSET 5.00, ISET 0.500):

| Byte | When | Decoding |
|---|---|---|
| `0x10` | Power-up idle, output never enabled (RO: 96/96 replies; SET: before `OUT1`) | beep on, bit5=0 (locked, as expected in remote mode), output off, bit0=0 |
| `0x51` (`'Q'`) | After `OUT1` (4/4) | output on + CV (no load ⇒ CV is correct) + beep |
| `0x11` | After `OUT0`, until the session ended (66/66) | output off. **bit0 stays latched** at the last regulation mode. |

Resolution:
- **OUTPUT = bit6.** Confirmed: it toggles with OUT1/OUT0, and VOUT follows.
- **CV = bit0 (1=CV), CC = !bit0, valid only while bit6=1.** Confirmed CV=1 with no load. CC=1 under load is not yet observed (open Q1). While the output is off, bit0 is stale: 0 after power-up and latched after use. Derive `SBY = !bit6`, `CV = bit6 && bit0`, `CC = bit6 && !bit0`.
- **bit5:** the manual's polarity (0=locked) matches the remote-mode panel lock, but this has not been toggled on hardware (open Q3). The Korad-family community driver (sigrok `korad-kaxxxxp`, unverified recollection) treats bit5 as OCP-enabled and bit7 as OVP-enabled on some firmwares. Do not map bit5 to `REMOTE` or `lock` until it is verified.
- Bits 1–3 are meaningless on single-channel models (always 0).
- The RO capture has 1 `STATUS?` with no reply. It is the last line of the session (service stopped mid-read), so it is an artifact, not a device fault.

## 5. Recommended poll strategy

Class status keys (CLASS_CONTRACT.md §PSU): `VOUT`, `IOUT`, `POUT` (SI); `OUTPUT`, `REMOTE`; plus `CV`, `CC`, `SBY`.

Minimum per cycle: **`VOUT1?` (5 B) → `IOUT1?` (5 B) → `STATUS?` (1 B)**, back to back with no gap, each reply read to its fixed length.
- `VOUT`, `IOUT` come from the replies. `POUT = VOUT·IOUT` is computed numerically before SI formatting.
- `OUTPUT = bit6 ? "ON" : "OFF"`, `SBY = !bit6`, `CV = bit6 && bit0`, `CC = bit6 && !bit0`.
- `REMOTE`: constant `"ON"` while the port is open (manual p.8: connecting USB enters remote). Do not expose `set_remote`, because no command exists. Drop it from this definition's operations, or reject it with a clear 4xx.
- Optional extras: `BEEP = bit4`.
- **Expected cycle ≈ 3 × 30–45 ms ≈ 100–130 ms** (Python today: 5 queries × ≈1 s ≈ 5 s). A 250–500 ms interval is comfortably feasible. Keep it ≥ ~150 ms so API setters can interleave with their 50 ms post-gap.
- Setpoints (`VSET1?`, `ISET1?`) are not status keys. Query them on demand, or cache them and invalidate on `set_voltage`/`set_current`/`RCL`.

## 6. Discrepancies

**Manual/protocol vs hardware**
1. IDN format: the manual shows `72-10480 SN:… Vx.xx`; the hardware returns `TENMA 72-2540 V2.1` with no serial number.
2. `ISET?` reply is 6 B with a trailing space. It is undocumented, and VSET/VOUT/IOUT are all 5 B.
3. Terminators, reply framing, latency, the ≥50 ms setter gap and the "setters never reply" behaviour are all undocumented.
4. Decimal limits are undocumented. `VSET` with 3 dp or exponent notation is **silently ignored**, with no error indication.
5. STATUS bit0 latches while the output is off. The manual implies it is always a live mode.
6. Protocol text errors: `OCP` described as "Stores the panel setting"; `*IDN?` "Returns the KA3005P identification".
7. Memories: protocol says 1–5 (p.2); the manual has 4 buttons, with M5 via the knob (p.7–8). Manifest `memory_banks: 5` is consistent with the protocol.

**Python driver bugs (do not replicate)**
1. `query_output_voltage/current`: inverted cache check (`is not None`). On a miss it returns `si_to_value(None)`, and the API crashes with `could not convert string to float: 'None'`. On a hit it re-queries. As a result `poll_status` sends VOUT, IOUT, VOUT, IOUT, STATUS = 5 queries per cycle (RO/SET capture: 188 VOUT vs 97 STATUS).
2. Every read waits for the 1 s pyserial timeout (no terminator, `read_until_reol`), so each exchange is ≈1005 ms instead of ≈35 ms.
3. Setters read after writing: +1 s each and no reply. The ≥50 ms gap is satisfied only by accident.
4. `set_ocp`/`set_ovp` use `value is "ON"`, so they never send (confirmed on the wire: no OCP in the SET capture).
5. Setpoint formatting is `str(value)` after API float coercion (`"5.00"` → `VSET1:5.0`). `12.345` or values that render in exponent form (e.g. `1e-05`) are sent and silently ignored, with no clamping to `absolute_limits`.
6. `POUT` = `_parse_numeric` on an SI-dict repr. It is wrong when an SI prefix is present (e.g. `0.988` A → `988 m`).
7. `lockEnabled = bit5` ignores the manual polarity (0=Lock). `REMOTE` is hardcoded to `"ON"` and there is no `set_remote` (UI POST → 400).
8. `query_status` reads 8 B for a 1 B reply, which again forces the timeout.
9. `set_memory_read` returns `None`, and `RCL` turning the output off is not reflected or invalidated.
10. Inherited `set_reset` sends `*RST`, which the device does not support.
11. Manifest polling `interval: 3` (72-2540) and 6 (default) only fit the 5 s Python cycle. They are irrelevant once framing is correct.

## 7. Open questions (verify on hardware)

| # | Question | Kind |
|---|---|---|
| Q1 | Does bit0 = 0 under load in CC (output on, small ISET, resistor load)? This confirms the CC mapping. | **state-changing** (output on + load) |
| Q2 | `STATUS?` bit changes after `OCP1`/`OCP0` and `OVP1`/`OVP0`: which bits (5? 7?) and do the panel LEDs follow? Is a reply ever sent? | **state-changing** |
| Q3 | Does bit5 change when the panel LOCK key is held 2 s while connected? Does the panel unlock at all in remote mode? | state-changing (panel only, no output change) |
| Q4 | Are undocumented `LOCK0/LOCK1` or similar accepted? | **state-changing** |
| Q5 | `BEEP0`/`BEEP1`: does bit4 follow? (Driver says "doesnt work".) | **state-changing** (harmless) |
| Q6 | `SAV1`…`SAV5` / `RCL1`…`RCL5`: do they work, does RCL turn the output off, is `RCL5` valid, and does `SAV` need a gap before the next command? | **state-changing** (overwrites memory; record M1–M5 first) |
| Q7 | Minimum gap after `OUT`/`OCP`/`OVP`/`RCL` (50 ms was measured only for VSET/ISET). | **state-changing** |
| Q8 | `ISET1:0.1234` (4 dp) and `VSET1:30.01` (over range): ignored, clamped or accepted? | **state-changing** |
| Q9 | Reply widths at full scale: `VOUT1?` at 30 V (`30.00`?), and `IOUT1?`/`ISET1?` near 5 A. | read-only (needs setpoint/load to reach the values) |
| Q10 | Behaviour of `VOUT2?` / a channel ≠ 1 on single-channel units: no reply, or garbage? This matters for multi-channel 72-13320/13330 definitions. | read-only |
| Q11 | Does a malformed or unknown query (`FOO?`) leave stray bytes or desync the stream? This sets the recovery policy (flush input before each query?). | read-only |
| Q12 | Do any replies get dropped or truncated under sustained fast polling (≥1000 cycles at Rust speed)? The only drop seen so far was a shutdown artifact. | read-only |
| Q13 | Effect of DTR/RTS levels: does the device still answer with DTR/RTS left high (they are best-effort in Rust)? | read-only |
| Q14 | OVP trip level: VSET-relative, or a separate panel-set value? | **state-changing** |
