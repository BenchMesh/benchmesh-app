# RIGOL DHO800 / DHO900 series oscilloscope — device notes

Definition: `devices/rigol_dho800.yaml` (class `OSC`, contract `devices/classes/OSC.yaml`).
Source: *DHO800/DHO900 Programming Guide* (`BenchMeshDOC/rigol_dho800/doc/DHO800-Series_programmingguide_EN.pdf`),
one guide for both series. Verified on a **DHO804** (FW 00.01.03, USB `1ab1:044d`, USBTMC) on 2026-10-17.

## Family

| Model | Channels | Bandwidth | V/div at 1X | Verified |
|---|---|---|---|---|
| DHO802 | 2 | 70 MHz | 500 µV–10 V | no |
| DHO804 | 4 | 70 MHz | 500 µV–10 V | **yes** |
| DHO812 | 2 | 100 MHz | 500 µV–10 V | no |
| DHO814 | 4 | 100 MHz | 500 µV–10 V | no |
| DHO914(S) | 4 | 125 MHz | 200 µV–10 V | no |
| DHO924(S) | 4 | 250 MHz | 200 µV–10 V | no |

Model differences are only channel count and feature values (`models[].channels/features`).

## Link and framing
- USBTMC, kernel driver `/dev/usbtmcN`. Commands end `\n`; replies are one USBTMC message ending `\n`.
- Every query round trip ≈ 23–29 ms (measured), so polling is split: 250 ms traces
  (`TRIGger:STATus?`, `WAVeform:PREamble?`, `WAVeform:DATA?`), 1 s display + measurements,
  3 s settings. A set re-polls every group at once (engine), so changes show immediately.
- Setters never reply (`reply: none`).

## Waveforms
- Session `on_connect`: `:WAVeform:MODE NORMal` (screen data, ≤ 1000 points) and `:WAVeform:FORMat BYTE`.
- Per channel: `:WAVeform:SOURce CHANnel<n>` (a `before:` step) then `:WAVeform:PREamble?` and
  `:WAVeform:DATA?` → IEEE block `#9000001000` + 1000 bytes + `\n` (≈ 55 ms).
- Preamble (p413): `format,type,points,count,xinc,xorig,xref,yinc,yorig,yref`.
  Volts = (code − yorig − yref) · yinc. Measured: yorig 0, yref 128, yinc 1.7067 mV at 50 mV/div
  (= 50 mV · 8 div / 234 codes), xinc 20 ns at 2 µs/div (1000 points over 10 div), xorig −10 µs.
- Hidden channels: the data query is skipped (`requires: disp_q == 'ON'`) → `WAVE: null`.

## Measurements
`:MEASure:ITEM? <item>,CHANnel<n>` (VPP, VAVG, VRMS, FREQuency). **9.9E37** means "no valid
measurement" (e.g. FREQ on a flat line) → status null.

## Verified live (2026-10-17)
- `bm-probe verify`: all readable operations, 4 channels, status complete.
- `scripts/bench_scope_setters.sh`: 14/14 setters change state and are restored (display, scale,
  offset, coupling, probe, BW limit, invert, timebase, timebase offset, trigger level/slope/source/
  sweep, acquisition type). `run` STOP → `TRIGger:STATus?` STOP; RUN → AUTO.

## Open issues
- **Screenshot** `:DISPlay:DATA? PNG` (p. 3.9.7, IEEE block): over the kernel usbtmc driver the
  transfer stalled after 24 564 bytes (timeout), the next `*IDN?` got an empty reply, a USBTMC
  device clear did not help, and the scope **dropped off USB and came back ≈ 50 s later**
  (re-enumerated; settings intact). Operation disabled in the definition. Next step: the userspace
  USBTMC transport (`nusb`, same item as the DGE screenshot) with proper multi-transfer reads.
- Edge trigger only (other trigger types not modelled). Digital channels (D0–D15), math, decoders,
  memory depth and RAW/MAXimum waveform reads are not modelled.
