# Independent instrument libraries: inventory, overlap and reliability

Step 1 of validating our device definitions (`devices/*.yaml`) against open-source instrument
libraries and user evidence. Date: 2026-10-18. Scope: the 95 families present at the start of this
work (the Riden `riden_rd60xx` families added the same day are not covered here; Baldanos/rd6006
and sigrok `rdtech-dps` are their sources).

Libraries are cloned (shallow) outside the repo at `/home/marek/project/BenchMeshDOC/_libs/`.
**No library code goes into this repo.** GPL/AGPL sources (InstrumentKit, libsigrok, scopehal,
tenma-serial, fygen, …) are used for facts only (command spellings, quirks), never copied.

The command-level comparison for the big-library families is already done:
[`crosscheck/README.md`](crosscheck/README.md) (`cargo run -p bm-driver --example crosscheck`,
14 definitions, pilot review 2026-10-18). This document adds the hobby/single-instrument libraries,
user reports of what breaks, and a second wave of pilots (§5).

Vector counts: pymeasure = `(command, reply)` tuples in the `expected_protocol` lists of the test
file (regex, so parametrised tests count once per literal; "≈"); Qcodes = `q:` entries in the
pyvisa-sim YAML; InstrumentKit = items in the sent/reply lists of `expected_protocol(...)`; hobby
libraries = distinct command literals or captured reply lines (method stated per row).

---

## 1. Libraries

### 1.1 Big libraries

| Library | Licence | Last commit | Scope | How tests encode the protocol | Usable vectors for our families |
|---|---|---|---|---|---|
| **pymeasure** (`_libs/pymeasure`) | MIT | 2026-10-16 | ~80 vendors, lab-oriented; few hobby brands (Rigol DG800/DHO, Siglent SPD/SDS-HD, Teledyne OEMs of Siglent) | `pymeasure.test.expected_protocol(Cls, [(sent, reply), …])`: exact strings, `None` = no reply; replies are **author-written** (from manuals, sometimes from a session). A `test_<x>_with_device.py` file means the driver was run against real hardware (no captured vectors, just live asserts) | ≈ 1 250 tuples over 16 drivers (§2) |
| **Qcodes** (`_libs/Qcodes`) | MIT | 2026-10-18 | Quantum/physics labs: Keysight, Keithley, Rigol DG/DP/DS, Tektronix high end | pyvisa-sim YAML (`src/qcodes/instrument/sims/*.yaml`): `dialogues: [{q, r}]` and `properties: {name: {default, getter: {q, r}, setter: {q}}}`; replies are **defaults, not captures** (IDN "QCoDeS, …"). Driver tests in `tests/drivers/` run against the sims | ≈ 144 `q:` entries (DS1074Z 13, 33xxx 18, 34465A 104, E4980A 9); driver code (no sim) for DG1062, DG4000, DP821/831/832, 34401A, Keithley 2000/6500 |
| **Qcodes_contrib_drivers** (`_libs/Qcodes_contrib_drivers`, partial) | MIT | 2026-10-18 | Community add-ons: Siglent SDG 20xx/60xx, Rigol DSA800 ("tested on a DSA815-TG") | No protocol tests | 0 vectors; ≈ 60 command strings (DSA800) |
| **InstrumentKit** (`_libs/InstrumentKit`) | **AGPLv3**, facts only | 2026-10-12 | Older lab gear; generic SCPI DMM/AFG; Rigol DS1000 (**old DS1000E/D**, not Z), MinghE MHS5200A | `expected_protocol(ik.Cls, [sent…], [replies…], sep="\n")`, replies author-written. One known wrong mock: 33220A `LOAD?` → `INF` | 33220A 26 sent / 12 replies; generic SCPI multimeter 34/21, function generator 12/5 |
| **python-ivi** (`_libs/python-ivi`) | MIT | 2017-02-18 (dead) | IVI-style drivers: Agilent 34401A/34461A, InfiniiVision DSO-X 2000A/3000A, Rigol DP800/DP832/DP831A, DM3068 (Agilent mode), E36xx | No protocol tests (only `test_ivi.py` for the core); command strings in code | 0 vectors |
| **pyvisa-sim** (`_libs/pyvisa-sim`) | MIT | 2026-10-12 | Format reference for the Qcodes sims | – | – |
| **libsigrok** (`_libs/libsigrok`) | GPL-3, facts only | 2025-11-20 | Hobby gear: `korad-kaxxxxp`, `gwinstek-gpd`, `manson-hcs-3xxx`, `scpi-pps` (Rigol DP700/DP800, OWON P4000/SPE6103), `scpi-dmm` (34401A, 34460A/65A, GDM9060/61, XDM1041/2041, SDM3055), `siglent-sdl10x0`, `itech-it8500` (binary), `rigol-ds` (DS1000Z, DS2000A, MSO5000), `siglent-sds` (SDS1000X-E), `rigol-dg` (DG800/900, DG1000Z) | No protocol tests; C command tables, per-model timing/quirk flags, udev IDs | 0 captured replies; ≈ 60 DMM + 91 scpi-pps + 16 Korad + 11 GPD + 8 Manson table entries |
| **scopehal** (`_libs/scopehal`, sparse) | BSD-3 | 2026-10-18 | ngscopeclient drivers for Rigol DS/MSO5/DHO, Siglent legacy and new sets, Keysight, Tek | No tests; firmware-specific workarounds in code, "tested on SDS1104X-E FW 6.1.37R8 / 8.2.6.1.37R9" | ≈ 150 command strings per driver |
| **lxi-tools** (`_libs/lxi-tools`) | BSD-3 | 2025-05 | Screenshot plugins; README list of hardware-tested models (DS1104Z, MSO5074, DSA815, SDS1202X-E, MSO-X 3024T …) | – | ≈ 8 screenshot commands |

### 1.2 Hobby and single-instrument libraries

★ = GitHub stars, date = last commit. "HW" = the author states it runs on real hardware.
**Cloned** = shallow clone in `_libs/<dir>`.

| Library | URL | ★ | Last | Licence | Our families | Tests / transcripts | Cloned | Vectors (method) |
|---|---|---|---|---|---|---|---|---|
| tenma-serial | github.com/kxtells/tenma-serial | 37 | 2026-04 | GPL-3 | tenma_72 (all 11), korad_ka | README "Tested on HW" for 2535/40/45/50, 2705/10, 2930, 13330, 13360 | `tenma-serial` | ≈ 60 command forms, 3 protocol variants |
| koradctl | github.com/attie/koradctl | 13 | 2026-09 | BSD-3 | tenma_72, korad_ka | hardware `test.py`; tested firmware list (72-2540 V2.1, V5.8) | `koradctl` | ≈ 12 |
| ka3005p | github.com/Nicoretti/ka3005p | 33 | 2025-11 | MIT/Apache | korad_ka, tenma_72 | works-list KA3005P, KD3005P, 72-2540, RS6005P | no | ≈ 10 |
| py-korad-serial | github.com/starforgelabs/py-korad-serial | 75 | 2024-01 | MIT | korad_ka | hardware TestCase | no | ≈ 12 |
| python-spd3303x | github.com/geissdoerfer/python-spd3303x | 26 | 2025-11 | MIT | siglent_spd3303x | pytest with fake instrument; real IDN | `python-spd3303x` | 9 + 1 IDN |
| siglent-mcp | github.com/DVSProductions/siglent-mcp | 0 | 2026-07 | MIT | siglent_spd1000x | "all confirmed against a real SPD1305X, fw 2.1.1.9" | `siglent-mcp` | ≈ 14 HW-confirmed |
| UDP3305S.py | github.com/philpagel/UDP3305S.py | 5 | 2026-06 | MIT | unit_udp3000s | hardware pytest (28 asserts) | `UDP3305S.py` | ≈ 25 |
| udp3305s (TestController def) | github.com/philpagel/udp3305s | 11 | 2024-10 | MIT | unit_udp3000s | device definition with IDN | `philpagel-udp3305s` | ≈ 70 lines |
| udp3000s-control | github.com/pnuding/udp3000s-control | 1 | 2026-07 | MIT | unit_udp3000s | quirks "confirmed on real hardware" | `udp3000s-control` | ≈ 20 |
| PyGPD3303S | github.com/akira-okumura/PyGPD3303S | 3 | 2023-06 | MIT | gwinstek_gpd | none | no | ≈ 15 |
| gpp4323 / gwinstekgpp | github.com/djacobow/gpp4323, github.com/jgraulle/gwinstekgpp | 1 / 2 | 2026-05 / 2025-02 | GPL-3 / Apache-2 | gwinstek_gpp | none | no | ≈ 15 / 30 |
| dpm8600 / DPM8624 | github.com/ianmaddox/dpm8600, github.com/JanBurkhardt/DPM8624 | 6 / 0 | 2024-11 / 2023-09 | none | juntek_dpm86xx | examples; exact reply strings in code | `dpm8600` | 12 / 13 |
| kiprim-psu | github.com/pasrom/kiprim-psu | 0 | 2026-07 | MIT | owon_spe (DC310S/DC605S) | pytest with fake serial (17 tests) | `kiprim-psu` | ≈ 20 |
| owon-spe (Rust), owon-spe6103-mcp, kiprim-dc310s | github.com/sjoerdsimons/owon-spe, kakuteki/owon-spe6103-mcp, maximweb/kiprim-dc310s | 0 / 0 / 21 | 2023 / 2026 / 2022 | MIT/Apache / MIT / none | owon_spe | parser tests / "measured on hardware" / reverse-engineered | no | ≈ 15 / 12 / 12 |
| owon_p4305, Python_SCPI (ODP3033) | github.com/cichy977/owon_p4305, Keksefresser/Python_SCPI | 2 / 0 | 2023 / 2024 | none | owon_p4000 / owon_odp | none | no | ≈ 12 each |
| PPS2320A | github.com/tkeller12/PPS2320A_Power_Supply | 0 | 2022-10 | MIT | hantek_pps | none | no | ≈ 10 |
| pyhcs | github.com/spomjaksilp/pyhcs | 0 | 2020-07 | MIT | manson_hcs | 1 hardware test | no | ≈ 8 |
| CPX400DP, mxpsu | github.com/RandomGgames/CPX400DP, HENRYMARTIN5/mxpsu | 4 / 0 | 2026-03 / 2025-06 | other / MIT | tti_cpx / tti_mx | none | no | ≈ 12 / 25 |
| dp800 (Rust) | github.com/newAM/dp800 | 4 | 2026-10 | MIT | rigol_dp800 (DP832, raw TCP 5555) | none | no | ≈ 17 |
| rusty_meter | github.com/markusdd/rusty_meter | 145 | 2026-09 | MIT | owon_xdm, owon_xdm3000 ("confirmed working" 1041/1241/1051/1251/2041/3041/3051) | 147 asserts (IDN classification, overload), 2 reply-stream tests | `rusty_meter` | ≈ 10 real replies + range tables |
| OWON-XDM1041 | github.com/TheHWcave/OWON-XDM1041 | 67 | 2021-12 | MIT | owon_xdm | trial-found command list, firmware-bug notes (HW) | `OWON-XDM1041` | – |
| Multimeter (XDM3000) | github.com/martin-bochum/Multimeter | 20 | 2023-02 | GPL-3 | owon_xdm3000 (+PeakTech/Voltcraft clones) | dmesg log, udev rule (HW) | `Multimeter-xdm3000` | ≈ 45 commands |
| DMM-Siglent-SDM, sdm3055 | github.com/martin-bochum/DMM-Siglent-SDM, djacobow/sdm3055 | 9 / 0 | 2026-01 / 2026-02 | GPL-3 | siglent_sdm3000 (HW, LAN) | none | `DMM-Siglent-SDM`, `sdm3055` | ≈ 60 / 10 |
| HP-34401A-Python | github.com/PY1CX/HP-34401A-Python | 10 | 2021-08 | MIT | agilent_34401a (RS-232) | **real session transcript** (`Test in Terminal.md`) | no | 24 replies |
| AgilentDMM, 34401AGUI | github.com/jones139/AgilentDMM, AkBKukU/34401AGUI | 5 / 16 | 2019 / 2018 | MIT / none | agilent_34401a | none | no | – |
| gdm-8341 | github.com/inflex/gdm-8341 | 6 | 2022-11 | BSD-3 | gwinstek_gdm8300 | none | no | ≈ 20 |
| kelctl | github.com/vorbeiei/kelctl | 4 | 2024-06 | MIT | korad_kel10x (RND 320-KEL103 V2.60, HW) | `KEL103-protocol.md` with example replies | `kelctl` | ≈ 34 |
| siglent_sdl1000x | github.com/JxR-TestMeasure/siglent_sdl1000x | 12 | 2020-07 | none | siglent_sdl1000x | `raw_scpi_commands.txt` | `siglent_sdl1000x` | 0 replies |
| LabInstruments | github.com/ulikoehler/LabInstruments | 6 | 2026-03 | Apache-2 | rigol_dl3000 (DL3021, HW), rigol_dm3000 | reply comments in code | `LabInstruments` | ≈ 6 |
| 8600_Series, 894-and-895_Series | github.com/BKPrecisionCorp/… | 2 / 0 | 2019 | Apache-2 | bk_8600, bk_894 (vendor examples) | none | `8600_Series` | ≈ 30 / 15 |
| TH2830_USER | github.com/haorucheng/TH2830_USER | 0 | 2026-08 | none | tonghui_th28xx | none (low confidence) | no | ≈ 10 |
| ET44.py | github.com/philpagel/ET44.py | 2 | 2026-08 | MIT | easttester_et44 (basis of LCR.md) | none | no | – |
| ds1054z | github.com/pklaus/ds1054z | 200 | 2017 (code) | GPL | rigol_ds1000z | live-scope unittest | `ds1054z` | ≈ 40 commands, 1 preamble reply |
| rigol-ds1000z | github.com/amosborne/rigol-ds1000z | 30 | 2026-10 | MIT | rigol_ds1000z | **42 hardware-in-the-loop pytests** | `rigol-ds1000z` | ≈ 74 fields |
| pyMSO5000 | github.com/mabl/pyMSO5000 | 3 | 2026-08 | MIT | rigol_mso5000 | 2236 command-header regexes extracted from firmware 01.03.03.00; MSO5074 HW | `pyMSO5000` | 2236 headers; ≈ 226 tuples |
| pydho800 | github.com/MasterJubei/pydho800 | 14 | 2024-05 | BSD-3 | rigol_dho800/1000 | none | `pydho800` | ≈ 45 |
| rigol-grab, DS1054Z_screen_capture | github.com/rdpoor/rigol-grab, RoGeorge/DS1054Z_screen_capture | 136 / 222 | 2024 / 2022 | MIT / GPL-2 | ds1000z, dho (screenshots) | firmware-difference issues | no | 2–5 |
| sdsremote, SDSpy | github.com/klumw/sdsremote, lheywang/SDSpy | 43 / 4 | 2026-10 / 2025-01 | Apache-2 / GPL-3 | siglent_sds1000xe (legacy set) | sdsremote claims SDS1202X-E/1104X-E/1204X-E; SDSpy untested | `sdsremote`, `SDSpy` | ≈ 30 / 80 |
| SiglentSDS2000xPlusPy | github.com/JoshGenao/SiglentSDS2000xPlusPy | 5 | 2023-10 | MIT | siglent_sds2000xplus | binary preamble offsets | no | ≈ 25 |
| tct_project TBS1000C notes | github.com/KTY137/tct_project (`docs/research/tbs1000c_scpi.md`) | 0 | 2026-07 | MIT | tektronix_tbs1000c | **verified live on a TBS1052C 2026-07-06** | no | ≈ 20 queries |
| tinySA_python (tsapython) | github.com/LC-Linkous/tinySA_python | 16 | 2026-10 | GPL-2 | tinysa (ZS405 HW, fw v1.4-143/252) | firmware compatibility tables, example replies | `tinySA_python` | ≈ 100 commands, 35 replies |
| QtTinySA | github.com/g4ixt/QtTinySA | 253 | 2026-10 | GPL-3 | tinysa | widely used GUI | `QtTinySA` | ≈ 25 |
| tinySA firmware | github.com/erikkaashoek/tinySA | 351 | 2026-10 | – | tinysa (ground truth: `sa_cmd.c`, `main.c`) | – | `tinySA-fw` (sparse) | – |
| Qcodes_contrib Rigol_DSA800, emc-toolkit, rigol-dsa815-python | see §1.1; github.com/fpgadeveloper/emc-toolkit, JovanMarkov96/rigol-dsa815-python | 70 / 6 / 1 | – | MIT / – / MIT | rigol_dsa800 | "tested on a DSA815-TG" (contrib) | no | ≈ 60 / 45 / 40 |
| pyssa3021x, LibreVNA SSA3000X driver | github.com/tspspi/pyssa3021x, jankae/LibreVNA | 1 / 1656 | 2024 / 2026 | other / GPL-3 | siglent_ssa3000x | LibreVNA is a shipped product | `pyssa3021x` | ≈ 11 / 14 |
| sdg2122-cli | github.com/LHX369963/sdg2122-cli | 0 | 2026-08 | MIT | siglent_sdg* (SDG2122X fw 2.01.01.23R8/39R7) | `docs/validation.md` + 44 validation JSON runs **checked against a scope** | `sdg2122-cli` | 52 test tuples; **240 live BSWV + 25 live OUTP replies** |
| devsignal | github.com/JacobBeningo/devsignal | 3 | 2026-07 | MIT | siglent_sdg* (SDG2042X fw 2.01.01.35R3B2) | fake-VISA tests; HW-verified per CHANGELOG | `devsignal` | 6 |
| pyawg | github.com/abhishekbawkar/pyawg | 0 | 2026-10 | GPL-3 | rigol_dg1000z (DG1032Z fw 03.01.12), siglent_sdg1000x | README hardware log | `pyawg` | ≈ 8 log lines |
| pysdg1032x, awg_scpi | github.com/tspspi/pysdg1032x, sgoadhouse/awg_scpi | 4 / 3 | 2022 | BSD / MIT | siglent_sdg1000x / sdg6000x | none | yes | 0 |
| fygen | github.com/mattwach/fygen | 115 | 2025-11 | LGPL-2.1 | feeltech_fy6900 (+FY6300/6600/6800) | `fygen_test.py` mocked wire | `fygen` | 59 |
| pyfy6900, fy6900commander, fy6x00 | github.com/tspspi/pyfy6900, katzenjens/fy6900commander, gitlab.com/adeuring/fy6x00 | 9 / 1 / 0 | 2024 / 2026 / 2024 | BSD-3 / GPL-3 / GPL | feeltech_fy6900 | HW notes (fw 1.3), firmware-bug list | all three | ≈ 15 table rows; 4 |
| utg900e, utg962, UTG900 | github.com/sdorunts/utg900e, renaatd/utg962, jarjuk/UTG900 | 0 / 6 / 32 | 2025 / 2025 / 2021 | MIT | unit_utg900e | utg900e: unit + HW integration test; UTG900: real transcripts (fw 1.08) | all three | 9 + wave-name table; 3 IDN lines |
| tektronix-func-gen | github.com/asvela/tektronix-func-gen | 21 | 2021-03 | MIT | tek_afg1000 ("tested with AFG1022/1062/3022") | README transcript | `tektronix-func-gen` | ≈ 5 |
| arbitrage, afg2225-sequencer | github.com/gabrielbenedikt/arbitrage, speicherl/afg2225-sequencer | 0 / 0 | 2020 / 2026 | GPL-3 | gwinstek_afg2000 (AFG-2105 only tested), gwinstek_afg2225 | none | both | 0 |
| owon-xdg-python-tools | github.com/fredzo/owon-xdg-python-tools | 0 | 2024-03 | MIT | owon_xdg (Multicomp MP750290) | none | yes | 0 |
| sds1004x_bode | github.com/4x1md/sds1004x_bode | 97 | 2023-02 | MIT | FY6600 driver; log of SDG-dialect commands sent by an SDS scope | – | yes | 0 replies |

Searched without result: UT8805E/UT8806E, TH2817B+/TH2810B+, SDM4055A/65A, GDM-8351/906x, OWON OEL,
IT8500+ over SCPI, rigol_dp700 (beyond sigrok), spd4000x (only a "Testing" script), gpd3303d,
udp6700, B&K 1685B/87B/88B, Hantek HDG2000B, BK 4050B/4060B, GW AFG-3000, EDU33210, Rigol DG2000,
UTG1000X/2000B/4000A, OWON DGE, UNI-T UPO2000/UTS, GW GSP-9300/GDS-1000B, Keithley 2100/2110,
DM858, DMM4040/4050.

---

## 2. Overlap matrix (95 families)

Legend: **pym** pymeasure · **qc** Qcodes · **qcc** Qcodes_contrib · **ik** InstrumentKit · **ivi** python-ivi · **sr** libsigrok · **sh** scopehal.
"Vec" = usable command/reply vectors (best source). "HW" = hardware evidence in the source
(`wd` = pymeasure `_with_device` test, `HW` = author says hardware, `HIL` = hardware-in-the-loop tests,
`cap` = captured live replies). "X" = already in [`crosscheck/`](crosscheck/README.md).

### PSU (22)

| Family | Big libraries (path · class · models) | sigrok / hobby | Vec | HW | X |
|---|---|---|---|---|---|
| korad_ka | – | sr `korad-kaxxxxp` (KA/KD3005P, KD6005P, RND, Velleman PS3005D, Stamos); ka3005p, py-korad-serial, koradctl, tenma-serial | ≈ 60 forms, 0 cap | HW | |
| tenma_72 | – | sr korad (2535/40/50, 2705/10); tenma-serial (all incl. 13320/30/60), koradctl | ≈ 60 forms + real IDNs | HW (per model) | |
| rigol_dp700 | – | sr scpi-pps `rigol_dp700_cmd` (DP711/712) | ≈ 20 table | – | |
| rigol_dp800 | qc `rigol/private/DP8xx.py` · `RigolDP8xxBase` · DP821/831/832; ivi `rigol/rigolDP800.py`, `rigolDP832.py`, `rigolDP831A.py` | sr scpi-pps; newAM/dp800 | 0 test, ≈ 40 table | – | |
| siglent_spd1000x | pym `siglenttechnologies/siglent_spd1168x.py`, `siglent_spd1305x.py` · `SPD1168X`, `SPD1305X` | siglent-mcp | 26 | HW (mcp) | X |
| siglent_spd3303x | – (pym `SPDBase` is the same dialect) | python-spd3303x, Swallowtail | 9 + IDN | HW | |
| siglent_spd4000x | – | HomeLabTools (testing) | 4 | – | |
| gwinstek_gpd | – | sr `gwinstek-gpd` (2303S, 3303S); PyGPD3303S, gpd2303s | ≈ 25 | – | |
| gwinstek_gpd3303d | – | (sr gpd old-firmware status layout) | – | – | |
| gwinstek_gpp | – | gpp4323, gwinstekgpp | ≈ 45 | – | |
| owon_odp | – | Python_SCPI (ODP3033) | ≈ 12 | – | |
| owon_spe | – | sr scpi-pps (SPE6103); owon-spe, kiprim-psu, owon-spe6103-mcp, kiprim-dc310s | ≈ 20 + tests | HW | |
| owon_p4000 | – | sr scpi-pps (P4305/P4603); owon_p4305 | ≈ 12 | – | |
| owon_spm | – (ours, bench-tested) | – | – | ours | |
| juntek_dpm86xx | – | dpm8600, DPM8624 | ≈ 25 real reply strings | HW | |
| hantek_pps | – | PPS2320A (and PPS2116A lib) | ≈ 10 | – | |
| manson_hcs | – | sr `manson-hcs-3xxx`; pyhcs, manson-cpp | ≈ 16 | HW (pyhcs) | |
| manson_hcs_10ma | – | sr `manson-hcs-3xxx` (3102/3104/3204) | ≈ 8 | – | |
| tti_cpx | sibling: pym `aimtti/aimttiPL.py` · `PLBase` (PL-P, same command set, 16 vec); qc `AimTTi/_AimTTi_PL_P.py` (+sim 3 q) | CPX400DP lib | ≈ 28 | – | |
| tti_mx | sibling as tti_cpx | mxpsu | ≈ 25 | – | |
| unit_udp3000s | – | UDP3305S.py, udp3305s, udp3000s-control, udp3000s-wrapper | ≈ 100 | HW | |
| unit_udp6700 | – | – | – | – | |

### DMM (19)

| Family | Big libraries | sigrok / hobby | Vec | HW | X |
|---|---|---|---|---|---|
| agilent_34401a | pym `hp/hp34401A.py` · `HP34401A`; qc `agilent/Agilent_34401A.py`; ivi `agilent/agilent34401A.py` | sr scpi-dmm; PY1CX transcript, AgilentDMM | 62 + 24 cap | wd, cap | X |
| keysight_truevolt | qc `Keysight/Keysight_3446{0,1,5}A_submodules.py`, `Keysight_34470A…` (sim 104 q); ivi `agilent34461A.py` | sr scpi-dmm (34460A/65A) | 104 (synthetic) | – | X |
| keysight_u3606 | – | u3606b_py (generic) | – | – | |
| keithley_2000 | pym `keithley/keithley2000.py` · `Keithley2000`; qc `Keithley/Keithley_2000.py` | – | 12 | – | X |
| keithley_2100 | – | – | – | – | |
| keithley_2110 | – | – | – | – | |
| keithley_dmm6500 | pym `keithley/keithleyDMM6500.py` · `KeithleyDMM6500`; qc `Keithley/Keithley_6500.py` | py-DMM6500 | 218 | wd | X |
| rigol_dm3000 | ivi `rigol/rigolDM3068Agilent.py` (Agilent-compat mode only) | LabInstruments (DM3058) | ≈ 6 | HW | |
| rigol_dm858 | – | – | – | – | |
| siglent_sdm3000 | – | sr scpi-dmm (SDM3055); DMM-Siglent-SDM, sdm3055 | ≈ 70 commands | HW | |
| siglent_sdm4000 | – | – | – | – | |
| owon_xdm | – (ours, bench-tested) | sr scpi-dmm (XDM1041/2041); rusty_meter, OWON-XDM1041 | ≈ 10 real replies | HW | |
| owon_xdm3000 | – | rusty_meter, Multimeter-xdm3000 | ≈ 45 | HW | |
| gwinstek_gdm8300 | – | gdm-8341 | ≈ 20 | – | |
| gwinstek_gdm9000 | – | sr scpi-dmm (GDM9060/61) | ≈ 10 | – | |
| hantek_hdm3000 | – | Hantek-UI (C#) | ≈ 3 | – | |
| tektronix_dmm4000 | – | – | – | – | |
| unit_ut8805e | – | – | – | – | |
| unit_ut8806e | – | – | – | – | |

### LCR (5) and ELL (6)

| Family | Big libraries | sigrok / hobby | Vec | HW | X |
|---|---|---|---|---|---|
| keysight_e4980a | pym `agilent/agilentE4980.py` · `AgilentE4980` · E4980A/AL; qc `Keysight/keysight_e4980a.py` (sim 9 q) | MariaPoliti/LCR, mitle/keysight_e4980al | 4 + 9 | wd | X |
| bk_894 | (E4980-like command set via our base) | BK vendor examples, impydance | ≈ 15 | – | |
| bk_880 | – | – | – | – | |
| tonghui_th28xx | – | TH2830_USER | ≈ 10 | weak | |
| easttester_et44 | – | ET44.py | – | – | |
| rigol_dl3000 | – | LabInstruments (DL3021) | ≈ 6 | HW | |
| siglent_sdl1000x | – | sr `siglent-sdl10x0`; JxR command list | ≈ 25 | – | |
| korad_kel10x | – | kelctl, KEL103Driver, python-kel102 | ≈ 34 | HW | |
| bk_8600 | – | BK 8600_Series examples | ≈ 30 | – | |
| itech_it8500plus | – | sr `itech-it8500` and itech-serial: **binary 26-byte protocol only** | 0 for SCPI | – | |
| owon_oel | – (ours, bench-tested) | – | – | ours | |

### AWG (24)

| Family | Big libraries | sigrok / hobby | Vec | HW | X |
|---|---|---|---|---|---|
| rigol_dg800 | pym `rigol/rigol_dg800.py` · `DG800` | sr `rigol-dg` | 22 | – | X |
| rigol_dg1000z | qc `rigol/Rigol_DG1062.py` · `RigolDG1062` (no tests) | sr `rigol-dg`; pyawg | ≈ 8 log | HW (pyawg) | |
| rigol_dg2000 | – | – | – | – | |
| rigol_dg4000 | qc `rigol/Rigol_DG4000.py` · `Rigol_DG4000` (no tests) | – | 0 | – | |
| siglent_sdg1000x | OEM: pym `teledyne/teledyneT3AFG.py` · `TeledyneT3AFG` (11 vec, BSWV dialect) | pysdg1032x, pyawg; sdg2122-cli replies (same base) | 265 cap (sibling) | cap | |
| siglent_sdg2000x | OEM as above; qcc `Siglent/sdg_channel.py` · `Siglent_SDG_20xx` | **sdg2122-cli**, devsignal | 265 cap | cap, HW | |
| siglent_sdg6000x | OEM as above; qcc `Siglent_SDG_60xx` | awg_scpi | (base) | – | |
| bk_4050b | OEM as above (Siglent protocol) | – | (base) | – | |
| bk_4060b | OEM as above | – | (base) | – | |
| keysight_33220a | pym `agilent/agilent33220A.py` · `Agilent33220A` (no test); ik `agilent/agilent33220a.py` (26/12); qc `Keysight/Keysight_33210a.py` | – | ≈ 38 (+74 pym 33250A sibling) | – | X |
| keysight_33500b | pym `agilent/agilent33500.py` · `Agilent33500`, `agilent33521A.py`; qc `KeysightAgilent_33XXX.py` + `Keysight_335xx/336xx` (sim 18 q) | – | 77 + 18 | wd | X |
| keysight_edu33210 | – | – | – | – | |
| tek_afg1000 | – | tektronix-func-gen | ≈ 5 | HW | |
| owon_xdg | – | owon-xdg-python-tools | 0 | – | |
| owon_dge | – (ours, bench-tested) | – | – | ours | |
| gwinstek_afg2000 | – | arbitrage | 0 | HW (2105) | |
| gwinstek_afg2225 | – | afg2225-sequencer | 0 | – | |
| gwinstek_afg3000 | – | – | – | – | |
| hantek_hdg2000b | – | – | – | – | |
| unit_utg900e | – | utg900e, utg962, UTG900 | 9 + 3 IDN | HW | |
| unit_utg1000x | – | – | – | – | |
| unit_utg2000b | – | – | – | – | |
| unit_utg4000a | – | – | – | – | |
| feeltech_fy6900 | – (ik `minghe/mhs5200a.py` is a different hobby AWG) | fygen, pyfy6900, fy6900commander, fy6x00 | 59 | HW | |

### OSC (14)

| Family | Big libraries | sigrok / hobby | Vec | HW | X |
|---|---|---|---|---|---|
| rigol_ds1000z | qc `rigol/Rigol_DS1074Z.py` · `RigolDS1074Z` (sim 13 q); ik `rigolds1000.py` is **old DS1000E** (different syntax, does not count) | sr `rigol-ds`; sh; ds1054z, **rigol-ds1000z (HIL)**, screen-capture tools | 13 + ≈ 74 fields | HIL | X |
| rigol_ds2000a | – | sr `rigol-ds` (DS2000A V3); ds2000 (WIP) | – | – | |
| rigol_mso5000 | – | sr `rigol-ds`; sh; **pyMSO5000** | 2236 headers, ≈ 226 tuples | HW (MSO5074) | |
| rigol_dho800 | pym `rigol/rigol_dho_base.py` · `DHOBase` | sh; pydho800, rigol-grab | 85 | wd + ours | X |
| rigol_dho1000 | pym `DHOBase` | sh; pydho800 | 85 | wd | X |
| siglent_sds1000xe | OEM: pym `lecroy/lecroyT3DSO1204.py` · `LeCroyT3DSO1204` (legacy set, 144 vec) | sr `siglent-sds`; sh (tested SDS1104X-E); sdsremote, SDSpy | 144 | wd, HW | |
| siglent_sds800xhd | sibling: pym `siglenttechnologies/siglent_sds1000xhd.py` · `SDS1000XHD` | sh | (210) | wd | |
| siglent_sds1000xhd | pym `SDS1000XHD` | sh | 210 | wd | X |
| siglent_sds2000xplus | sibling `SDS1000XHD` | sh; SiglentSDS2000xPlusPy | ≈ 25 | – | |
| siglent_sds2000xhd | sibling `SDS1000XHD` | sh | – | – | |
| keysight_infiniivision | pym `keysight/keysightDSOX1102G.py` · `KeysightDSOX1102G`; ivi `agilentBaseInfiniiVision.py` (DSO-X 2000A/3000A) | lxi-tools (MSO-X 3024T) | 122 | wd | X |
| tektronix_tbs1000c | – | **tct_project notes (live TBS1052C)**, WASIC | ≈ 20 verified | HW | |
| gwinstek_gds1000b | – (GDS-2000A/3000 scripts only) | – | – | – | |
| unit_upo2000 | – | – | – | – | |

### SAL (5)

| Family | Big libraries | sigrok / hobby | Vec | HW | X |
|---|---|---|---|---|---|
| tinysa | – | **tinySA firmware source**, tinySA_python, QtTinySA | ≈ 35 replies + firmware | HW | |
| rigol_dsa800 | qcc `Rigol_DSA800.py` ("tested on DSA815-TG") | lxi-tools, emc-toolkit, rigol-dsa815-python | ≈ 60 commands | HW | |
| siglent_ssa3000x | – | pyssa3021x, LibreVNA driver, lxi-tools | ≈ 25 | HW (LibreVNA) | |
| gwinstek_gsp9300 | – | – | – | – | |
| unit_uts | – | – | – | – | |

### Totals

| Coverage | Families |
|---|---|
| Direct driver in pymeasure / Qcodes / InstrumentKit / python-ivi | **18** |
| Only a sibling/OEM driver in those (Teledyne T3AFG/T3DSO = Siglent, PL-P ≈ CPX/MX, SDS1000XHD ≈ other HD/Plus) | **11** (total in big libraries 29) |
| Not in the big libraries but in libsigrok | **14** (big libraries or sigrok: 43) |
| Only hobby/single-instrument libraries | **27** |
| **Nothing independent** (manual only, or our own bench) | **25** (+ 3 bench-tested by us: owon_spm, owon_oel, owon_dge) |

---

## 3. Reliability and findings per family

Only families with outside evidence. "CONTRADICTS" = an outside source with hardware behind it
disagrees with our YAML (key named). "Risk" = plausible problem, not proven. Big-library
command disagreements already decided against the manual are in `crosscheck/*.md`, not repeated.

### 3.1 PSU

**korad_ka**: widely used; the main source is the [EEVblog "Korad KA3005P I/O Commands"](https://www.eevblog.com/forum/testgear/korad-ka3005p-io-commands) thread plus [sigrok wiki](https://sigrok.org/wiki/Korad_KAxxxxP_series). STATUS? bit meanings are still disputed ([ka3005p#17](https://github.com/Nicoretti/ka3005p/issues/17), [#7](https://github.com/Nicoretti/ka3005p/issues/7)).
- CONTRADICTS `timing {min_gap_ms: 20, gap_after_write_ms: 50}`: every library paces slower. sigrok waits 80 ms before *every* request and adds 450 ms on Velleman PS3005D V1.3 (`DEVICE_PROCESSING_TIME_MS`, `EXTRA_PROCESSING_TIME_MS`); py-korad-serial waits 100 ms, koradctl ≥ 50 ms, tenma-serial 200 ms. Add per-model timing for V1.3 at least.
- STATUS? bits 5/7 differ between sources (sigrok: bit5 OCP, bit7 OVP; Nicoretti: bit5 lock; koradctl on 72-2540: bit7 OVP/OCP). Our mapping of only bits 0, 1 and 6 is safe.
- ISET? may append the sixth character of a previous `*IDN?` reply on protocol 2.0 (sigrok wiki); `rx: idle` copes. Replies can carry trailing NULs (koradctl, py-korad-serial): check that `trim` strips `\x00`.
- LABPS3005DN (`QJE3005PV1.0`) uses a different, newline-terminated protocol ([Whadda forum](https://forum.whadda.com/t/velleman-labps3005dn/33229)); our regex correctly excludes it. Stamos S-LS-31 (IDN without vendor) is missing from our models.

**tenma_72**: tenma-serial tests per model on hardware; [tenma-serial#21](https://github.com/kxtells/tenma-serial/issues/21) (72-13360 not detected until it sent `\n`), [koradctl#1](https://github.com/attie/koradctl/issues/1) (V5.8 IDN `TENMA 72-2540 V5.8 SN:03211356`, our regex matches).
- CONTRADICTS (72-13320/13330): these use `\n`, per-channel output (`OUT1:1`, `OUT12:1`), a 2-byte STATUS? (bit6 OUT1, bit7 OUT2), no OCP/OVP, plus LOCK/TRACK. Our YAML gives them the 2540 framing (`tx terminator ""`, fixed `reply {bytes: N}`, `out: "6"`, global `OUT{on}`, `ocp`/`ovp`).
- CONTRADICTS (72-13360): a different protocol: `\n`, no channel digit (`VSET:12.00`, `OUT:1`), STATUS? bit1 = output. We send `VSET{ch}?` / `OUT{on}` and read bit6.
- Risk: fixed `reply {bytes: 6}` for ISET? on every model. Protocol 2.1 sends 5 bytes, so reads would run to their timeout; this is verified only on our 2540. The 72-2550 may identify as `KORADKA6003P` and then fall to korad_ka, which is acceptable.

**rigol_dp700 / rigol_dp800**: [EEVblog DP832 firmware/bug list](https://eevblog.com/forum/testgear/rigol-dp832-firmware-updates-and-bug-list): LAN dropouts (stuck in remote, recover with Back key), DHCP renewal every 5 min on fw 1.14, an `OUTP:TRAC?` timeout fixed in firmware.
- RESOLVED 2026-10-18 (tcp 5555 added) — was: CONTRADICTS the dp800 note "no raw LAN socket port": newAM/dp800 drives a DP832 over raw TCP 5555 with our exact commands. Add `tcp: {port: 5555}`.
- Risk (dp700): sigrok's DP700 profile uses no channel arguments (`:OUTP?`, `:MEAS:VOLT?`). We send `:OUTP? CH1` / `:MEAS:ALL? CH1` to a single-channel DP711/712; unverified.

**siglent_spd1000x**: pymeasure (26 vectors) and siglent-mcp (hardware-confirmed, fw 2.1.1.9) agree with our `stat` bits; the LAN server is single-client and can wedge after an abrupt disconnect (power cycle). [EEVblog](https://eevblog.com/forum/testgear/spd3303x-e-scpi-interface-issues/msg2483208): on fw 2.01.01.07R1, `OUTP`/`MODE:SET` had to be sent without `\n`, and one user waited 2 s after `OUTP CH1,ON`.

**siglent_spd3303x**: [EEVblog SCPI issues](https://eevblog.com/forum/testgear/spd3303x-e-scpi-interface-issues/msg2483208), [Siglent support thread](https://www.eevblog.com/forum/testgear/siglent-technical-support-join-in-eevblog/msg3138438/): on fw 1.01.01.02.05 over LAN 5025, commands sent with `\n` beep and are rejected; 300 ms pacing beats 100 ms; `OUTP CH1,OFF` quickly followed by `OUTP CH2,OFF` left CH2 on.
- CONTRADICTS `link.usbtmc` (only `0483:7540`): a unit enumerates as `F4EC:1430` ([python-spd3303x#2](https://github.com/geissdoerfer/python-spd3303x/issues/2)).
- CONTRADICTS the model match `^Siglent( Technologies)?, ?SPD3303X…`: RS PRO RSPD3303X-E answers a 3-field IDN that starts with a serial-like token and has no "Siglent" ([#3](https://github.com/geissdoerfer/python-spd3303x/issues/3)).
- CONTRADICTS (tcp, fw 1.01.01.02.05): the base `tx {terminator: "\n"}` on the 5025 socket is the failing case. The same firmware works with `\n` over USBTMC.

**gwinstek_gpd / gpd3303d**: sigrok re-sends `*IDN?` because the first reply may be `Invalid Character.`. STATUS? has an 8-character layout on new firmware (output = character 6, as in our gwinstek_gpd). On firmware < 2.00 the output is character 7 and the reply has extra lines; our `^[01]{8}$` misses that (already noted).

**gwinstek_gpp**: consistent (`:OUTP{n}:STAT?` → ON/OFF; 115200 baud / TCP 1026 per gpp4323; 9600 over the USB virtual COM port per gwinstekgpp). Our IDN regex is case-sensitive (`^GW INSTEK,`); mixed case "GW Instek" is unverified.

**owon_spe / owon_p4000 / owon_odp**: `OUTP?` answers ON/OFF on the SPE6103/Kiprim (the manual says 1/0; our map takes both). `SYST:STAT?` returns `ERR` on SPE6103. `MEAS:ALL:INFO?` (which we poll) is confirmed only on an SPE3102, and no Kiprim library uses it; kiprim-psu uses `measure:voltage?` etc. CH340 needs ≈ 0.3 s after the port opens (pasrom), so our `settle_ms: 50` may be short. Readings refresh about every 0.3 s.

**juntek_dpm86xx**: [akkudoktor forum](https://akkudoktor.net/t/dpm8624-485-software-anleitungen-gesucht/7290): the protocol menu must be set to 0; some units run at 115200.
- CONTRADICTS the model match `^:01r01[=:]5000,?$`: both libraries see replies as `:01r01=5000.` + CRLF (trailing `.`). The numeric regexes still work. The write ack is lowercase `:01ok`.

**hantek_pps**: the setter ack is lowercase `ok`; cosmetic.

**manson_hcs / manson_hcs_10ma**: [sigrok wiki](https://sigrok.org/wiki/Manson_HCS-3xxx_series): the manuals are "incomplete or even erroneous"; `SESS` times out after about 5 s.
- CONTRADICTS `state.output initial: "OFF"`: the output is on at power-up (sigrok wiki).
- Unresolved: sigrok divides GETD current by 100 for all models, including 3102/3104/3204. Our 10 mA variant uses ×0.001, backed by the manual example.

**tti_cpx / tti_mx**: consistent with the PL-P vectors (`V2?` → `V2 1.2`, IDN separated by comma+space). mxpsu shows `OVP1 30.50` where we expect `VP1 …`; our last-token regex handles both.

**unit_udp3000s**: identity, LAN 5025, 9600 baud and 200 ms query delay match.
- Risk: every library uses `OUTPut:OVP:VALue CHn,x` / `OUTP:OVP:STAT`; our `:SOUR{ch}:VOLT:PROT[:STAT]` (PM p.14) is used by none of them.
- Trip state: only `STAT:QUES:INST:ISUM<n>:EVENt?` latches (pnuding, hardware). Write `:EVEN` explicitly in `isum`.
- Lock: the libraries use `LOCK ON` / `SYST:LOCK ON`, where we use `:SYST:RWL`.

### 3.2 DMM

**agilent_34401a**: pymeasure has a `_with_device` test. A real RS-232 transcript ([PY1CX](https://github.com/PY1CX/HP-34401A-Python)) shows that after about 10 min idle `READ?` gets no reply until `SYST:REM` is sent again. Risk: `session.on_connect` sends `SYST:REM` only once; resend it on reconnect or after a timeout. Two framings are in use: 8N2 with XON/XOFF, and 7E2 with DSR/DTR. sigrok: AC readings take ≈ 1 s, so it uses a 1.5 s read timeout.

**keysight_truevolt**: Qcodes PR #1845: on firmware ≥ 3.0 the 34465A/70A digitizer is always enabled, while `*OPT?` shows DIG only when it was bought.

**keithley_2000**: Qcodes issue #333 (wrong mode string gives -113); PR #635: firmware A06 needs the long `:SENSe:` form for some queries; issue #961: `TRIG:COUN?` returns `+9.9e37` for INF.

**keithley_dmm6500**: requires `*LANG SCPI`. A meter left in TSP / SCPI2000 / SCPI34401 will not answer (pymeasure docstring, Qcodes PR #1541/#4121). Range/NPLC queries in a function that has no such setting hang with -113; our enums already avoid them.

**rigol_dm3000**: commands match ulikoehler's hardware-used ones. A DM3068 switched to the Agilent command set (python-ivi drives that mode) will not answer us. Rigol USBTMC is non-standard ([EEVblog](https://eevblog.com/forum/testgear/rigol-usbtmcvisa-interface-is-really-terrible)); old Linux kernels needed a patched usbtmc.

**siglent_sdm3000**: `tcp.port 5025` is confirmed on hardware (djacobow). The Siglent socket app note says 5024 ([PDF](https://siglentna.com/application-note/programming-example-controlling-an-spd-power-supply-via-sockets-ov/?pdf=19789)); document it as an alternative. LAN reads take 40–45 ms with `INIT`+`DATA:LAST?` ([EEVblog](https://eevblog.com/forum/testgear/sdm3045x-scpi-poll-rate)). Risk across `scpi_dmm`: `READ?` blocks if the trigger source was left at BUS/EXT. Consider `TRIG:SOUR IMM` on connect.

**owon_xdm** (bench-tested by us, but on one firmware): [TheHWcave](https://github.com/TheHWcave/OWON-XDM1041): fw 25052021 answers every non-query with `OK\n`. [rusty_meter#4](https://github.com/markusdd/rusty_meter/issues/4): XDM2041 over RS-232 stuck at identify. [#20](https://github.com/markusdd/rusty_meter/issues/20): XDM1051 overload value differs; beeper reply 0/1.
- CONTRADICTS `commands.func` DIOD/CONT mapping: rusty_meter swaps them for firmware < V4.3 (`src/app/serial.rs`).
- CONTRADICTS `status.MEAS` (no overload handling): the meters report overload as `1E9` (sigrok, rusty_meter), so we would show 1 GΩ.
- CONTRADICTS `features.modes` offering `FRESistance` on every model: 4-wire exists only on XDM2041/3041/3051.
- `RANGE?`/`AUTO?` exist on 1041/1241 (we assume no range query); they give no reply in CONT/DIOD/FREQ/PER.

**owon_xdm3000**:
- CONTRADICTS `link.usbtmc: null`: the meter is USBTMC `5345:1234` (martin-bochum dmesg + udev rule).
- CONTRADICTS `rate_set` values `F|M|L`: both libraries use `S` (slow), not `L`.
- CONTRADICTS `link.serial.baud: 115200`: martin-bochum says "do not use BAUD > 57600".
- Both libraries terminate with LF, not CR; not proven wrong.

**gwinstek_gdm8300**: `VAL1?` / `CONF:RANG?` confirmed; inflex reads the function with `SENS:FUNC1?` (ours: `CONF:FUNC?`). USB is a CP210x with GW's own VID:PID; older kernels do not bind it.

**gwinstek_gdm9000**: sigrok udev rule gives USBTMC PID `0059`; fill `usbtmc.pid`.

**hantek_hdm3000**: consistent. Overload reads `+9.90000000E+37` (means "open" in Continuity/Diode); LF terminator.

### 3.3 LCR and ELL

**keysight_e4980a**: no issues filed. Qcodes gates `:BIAS:RANGe:AUTO` on firmware ≥ A.02.10. Add `FORM ASC` on connect (pymeasure does): a meter left in binary format breaks `fetch`.

**tonghui_th28xx**: [EEVblog](https://www.eevblog.com/forum/testgear/lcr-meter-plot-software/): RS-232 needs a null-modem cable; USB works with PyVISA only in BUS MODE USBTMC. A low-confidence script waits 200 ms after setters and prefers `*TRG` over `TRIG` on fw 2.0. Risk: our base `gap_after_write_ms: 50` is probably too short; the baud rate must match the meter's setting (9600 vs 115200).

**bk_894**: consistent with BK's examples and two user scripts.

**rigol_dl3000**: [EEVblog](https://www.eevblog.com/forum/testgear/new-rigol-dc-load-d3000-series/msg3650038/): `*RST` does not work, the unit "locks up regularly", fw 1.04/1.05 broke battery mode, LAN on non-A models needs a licence.
- CONTRADICTS (risk) `bases/scpi_load` `meas_*` `[trim, number]`: a DL3021 over USBTMC returns `0.000067\n0` (two lines) for `:MEAS:VOLT?`. Parse the first line only; over TCP the stray `0` would land on the next reply.
- USB `1AB1:0E11` is shared with DP800 in sigrok's udev rule, so the identify reply must decide.

**siglent_sdl1000x**: consistent with sigrok `siglent-sdl10x0` (long-form `:FUNC?` replies such as CURRENT / RESISTANCE).

**korad_kel10x**: fully consistent with kelctl on hardware (115200, LF, 100 ms after each send, unit suffixes).

**bk_8600**: consistent with BK's vendor examples.

**itech_it8500plus**: CONTRADICTS (unverified premise) `link.serial` + `on_connect SYST:REM`. Every outside source (sigrok, itech-serial, an IT8511A+ owner on [EEVblog](https://eevblog.com/forum/testgear/itech-8500-series-(it8511it8512-)-(also-bk8500)-serial-(ttl)-connection/msg1135885)) shows only the 26-byte binary frame protocol on the TTL port. Mark the link unverified, or restrict it to models with a real RS-232/USB SCPI port.

### 3.4 AWG

**Siglent SDG family (siglent_sdg1000x/2000x/6000x, bk_4050b/4060b via `bases/siglent_sdg`)**: the best hardware evidence of the whole inventory.
- Reply parsing is confirmed: our regexes decode all 240 live BSWV replies and 25 OUTP replies captured on a hardware-validated SDG2122X (sdg2122-cli), plus the pymeasure T3AFG vectors. One-key-per-command setters and `%.6f` match devsignal's two hardware fixes (fw 2.01.01.35R3B2: multi-pair BSWV hung USB).
- CONTRADICTS `link.usbtmc` on Linux (SDG2122X fw 2.01.01): replies carry the previous request's bTag, so the kernel usbtmc driver rejects them. Replies also come one query late, a 4-byte all-zero packet sometimes appears, and the instrument refuses USBTMC CLEAR (sdg2122-cli `docs/validation.md`). Also seen: [EEVblog SDG1000X](https://www.eevblog.com/forum/testgear/siglent-sdg1000x-waveform-generators/msg4522292/) (second USB query fails, pyvisa-py froze it) and [pyvisa-py#468](https://github.com/pyvisa/pyvisa-py/issues/468). Prefer `tcp` 5025.
- Minor: `outp_q` pattern `OUTP\s*(ON|OFF)` fails if CHDR OFF was set on the instrument.

**Rigol DG (dg800/dg1000z/dg2000/dg4000)**: [pyvisa#521](https://github.com/pyvisa/pyvisa/issues/521): a DG811 failed reads until a power cycle; pyvisa-py carries Rigol USBTMC quirk handling ([#186](https://github.com/pyvisa/pyvisa-py/pull/186)). No contradictions; the DG1032Z IDN (`Rigol Technologies,DG1032Z,…,03.01.12`) matches. Risk: other libraries wait 200–250 ms after writes; ours waits 20 ms.

**Keysight 33220A/33500B**: [pymeasure#821](https://github.com/pymeasure/pymeasure/issues/821) (`%f` truncation; we use `%g`). Qcodes' per-model limits match ours.

**tek_afg1000**: the first `*IDN?` can come back empty (tektronix-func-gen re-sends it); identify should retry once. **owon_xdg**: Multicomp MP750290 rebrands may not answer `OWON,XDG…`.

**gwinstek_afg2000/2225**: afg2225-sequencer sends every setter with `;*OPC?` and repeats until the read-back matches, so commands evidently get dropped. arbitrage splits replies on CR only (AFG-2105, USB CDC 9600). Risk: terminator mismatch; verify with a capture.

**unit_utg900e**: add `usbtmc {vid: "6656", pid: "0834"}` (two libraries). CONTRADICTS the model patterns `UTG932E?`/`UTG962E?`: an old UTG962 (fw 1.08) answers `UNI-T Technologies,UTG900,1485061822,1.08`, and the only library for it used key emulation, so pre-E firmware may lack `:CHAN:BASE`. Joining several commands with `;` is unreliable (utg962).

**feeltech_fy6900**: [fygen#12](https://github.com/mattwach/fygen/issues/12), [#11](https://github.com/mattwach/fygen/issues/11), [pyfy6900#1](https://github.com/tspspi/pyfy6900/issues/1), [EEVblog FY6900 web software](https://www.eevblog.com/forum/testgear/feeltech-fy6900-web-based-software/msg6246993/): fw 1.3 writes frequency in µHz (14 digits); fw 1.8 writes Hz with 6 decimals. fw 1.8 inserted waveforms, shifting the arbitrary-wave indices. A malformed upload hangs the unit ([fygen#3](https://github.com/mattwach/fygen/issues/3)). Replies may start with blank lines (fy6x00). The CH340 driver is flaky on some OSes ([tspi.at](https://www.tspi.at/2022/12/30/fy6900withoutch340.html)).
- CONTRADICTS `freq_set` (`WMF{hz}` `%.6f`) for fw 1.3-class units. Read the version with `UVE` at identify and choose the format. Amplitude ÷10000, phase ÷1000 and offset mV / 2^32 agree across three libraries.

### 3.5 OSC

**rigol_ds1000z**: firmware 00.04.03/04 changed behaviour (some versions need `\n` after every command; 00.04.03 SP1 appends 2 bytes to the BMP) ([screen_capture#6](https://github.com/RoGeorge/DS1054Z_screen_capture/issues/6), [#8](https://github.com/RoGeorge/DS1054Z_screen_capture/issues/8)). pklaus recommends 00.04.04.SP3. Dense PNG captures arrive truncated ([pklaus#10](https://github.com/pklaus/ds1054z/issues/10)). RAW reads are capped at 250k points per transfer. The preamble example confirms our scaling. Risk: `vpp_q` `[trim, number]` fails on the `measure error!` reply; it should yield null.

**rigol_ds2000a**: deep-memory reads can return `#9000000000` with no data ([EEVblog](https://eevblog.com/forum/testgear/first-impressions-and-review-of-the-rigol-ds2072-ds2000-series-dso/250)); sigrok polls `:WAV:STAT?` before reading. 14 div / 1400 points match ours.

**rigol_mso5000**: all 43 of our command headers match pyMSO5000's firmware-extracted regexes. CONTRADICTS (latent) `scpi_osc session.on_connect`: we never reset `:WAV:STARt 1`/`:WAV:STOP`. If an earlier client narrowed the window, `:WAV:DATA?` returns fewer points than the preamble says. Firmware cannot be downgraded ([EEVblog](https://www.eevblog.com/forum/testgear/rigol-mso-5000-hardwaresoftware-revisions/msg2967032/)).

**rigol_dho800/dho1000**: [pymeasure#1524](https://github.com/pymeasure/pymeasure/issues/1524) (a second RAW/MAX read hangs after RUN); [pydho800#7](https://github.com/MasterJubei/pydho800/issues/7) (`:WAV:POIN 1000 NORM` gives -120); [EEVblog DHO bug reports](https://eevblog.com/forum/testgear/rigol-dho800900-oscilloscope-bug-reports-firmware/msg5180658). scopehal: in pagination DHO returns the page size instead of the memory depth. No contradictions for NORMal BYTE reads; check DHO1000's `:CLEar`.

**siglent_sds1000xe** (legacy set): consistent with pymeasure T3DSO1204 (144 vectors, `_with_device`), sigrok and scopehal (`CHDR OFF`, 25 codes/div, `DAT2,#9…` + two LFs). fw 6.1.37R10 clears roll-mode traces on screenshot ([lxi-tools#110](https://github.com/lxi-tools/lxi-tools/issues/110)). Minor: everyone sends `SANU? C1`; we send `SANU?`. `WAVE_X0` with TRDL remains unverified.

**siglent HD/Plus (sds800xhd/1000xhd/2000xplus/2000xhd)**: the BWLimit ON/OFF disagreement with pymeasure was decided in our favour in [`crosscheck/siglent_sds1000xhd.md`](crosscheck/siglent_sds1000xhd.md). New risk from scopehal: on SDS2000X+ fw ≤ 1.3.9R6 and SDS2000X HD before 1.2.3 the WORD block length counts samples, not bytes (read length ×2). `*OPT?` exists only since SDS2000X+ fw 1.5.2R1. scopehal reads SDS1000X HD as BYTE.

**keysight_infiniivision**: no contradictions; scaling matches python-ivi.

**tektronix_tbs1000c**: everything checked live on a TBS1052C agrees: `TRIGger:A:*` only, AUX/EXT, `SELect:CH1?` → 1/0, WFMOutpre field order, 20000-point record. Do not add `VERBose OFF` without changing our long-form maps.

**gwinstek_gds1000b**: Risk: GDS-2000A/3000 return a text preamble before `#` and little-endian int16; we assume `i16be`.

### 3.6 SAL

**tinysa**:
- CONTRADICTS `commands.swt_q` regex `([0-9.]+)s\s*$`: the firmware prints `%5.3Fs` with an SI prefix (`462ms`), so the regex yields null below 1 s.
- Risk `trace_q`/`mky_q`: firmware before v1.4-230 prints exact powers of ten with `:` as the leading digit (`-:.000000e+01`).
- Commands are capped at 47 bytes. From v1.4-222 a stuck command prints `Command timeout` after about 20 s with no prompt. Old firmware prints only usage for bare `rbw`; send `rbw ?`. Never send e-notation (`150e6` → 150 Hz; our `%.0f` is fine).
- `features.freq_range` for ZS406/ZS407 are harmonic-mode tops; the ultra-mode tops are ≈ 5.33/5.42/7.37 GHz.

**rigol_dsa800**: consistent (Qcodes_contrib tested on a DSA815-TG; `:PRIV:SNAP? BMP`).

**siglent_ssa3000x**: consistent (`:FORM ASC`, `:TRAC:DATA? 1` with a trailing comma; LibreVNA ports 5025 data / 5024 telnet).

---

## 4. Gaps

### 4.1 Our families no independent library covers (25 + 3 bench-tested by us)

PSU: gwinstek_gpd3303d (only sigrok's old-firmware GPD layout), unit_udp6700.
DMM: keithley_2100, keithley_2110, rigol_dm858, siglent_sdm4000, tektronix_dmm4000, unit_ut8805e, unit_ut8806e.
LCR/ELL: bk_880, itech_it8500plus (SCPI side).
AWG: rigol_dg2000, gwinstek_afg3000, hantek_hdg2000b, keysight_edu33210, unit_utg1000x, unit_utg2000b, unit_utg4000a.
OSC: gwinstek_gds1000b, unit_upo2000.
SAL: gwinstek_gsp9300, unit_uts.
Bench-tested by us, nothing outside: owon_spm, owon_oel, owon_dge.

These stay manual-only until a user's first-contact check. Most are UNI-T / GW Instek / Hantek,
which matches where hobby libraries are thinnest.

### 4.2 Hobby-relevant instruments the libraries cover that we do not

| Instrument | Where | Note |
|---|---|---|
| Riden RD6006/6012/6018, RK60xx (Modbus) | `_libs/rd6006`, sigrok `rdtech-dps` | being added (`riden_rd60xx`) |
| RDTech DPS/DPH (Modbus), JDS6600/FY6600/FY6800 | sigrok, fygen, sds1004x_bode | FY6x00 shares the FY6900 protocol with frequency-unit differences |
| MinghE / Koolertron MHS5200A AWG | InstrumentKit `minghe/mhs5200a.py` | cheap hobby AWG |
| Rigol DS1000E/D (old), DS4000 | InstrumentKit `rigolds1000.py`, Qcodes `Rigol_DS4000.py` | DS1000E is still common second-hand |
| Rigol DP1000 / DP1116A / DP1308A | python-ivi | older Rigol PSUs |
| Keysight E3631A / E364xA / E36312A, HP 6632B | pymeasure, InstrumentKit | classic used-market PSUs |
| Keithley 2200 / 2230 / 2260B / 2281S PSUs | pymeasure | |
| Aim-TTi PL-P series, QL355TP, LD400P load, EL302P | pymeasure, Qcodes, InstrumentKit | same TTi command family as our CPX/MX |
| TEXIO PSW (= GW Instek PSW) | pymeasure `texio/texioPSW360L30.py` | GW Instek PSW rebrand |
| BK 9130B PSU | pymeasure | |
| HP/Agilent 33120A AWG, 34450A DMM, 34410A DMM | pymeasure, InstrumentKit | used-market lab gear |
| Siglent SDS1072CML | pymeasure | older Siglent scope |
| Tek TDS2000 / TDS224 / TPS2012, AFG3152C | pymeasure, InstrumentKit, Qcodes | |
| Keithley 2400 / 2450 SMU | pymeasure, Qcodes | small-lab staple; no SMU class in BenchMesh |
| BK 8500 (binary protocol) / ITECH IT85xx binary | sigrok `itech-it8500`, itech-serial | would need binary framing |
| NanoVNA (same shell as tinySA) | nanovna-tools, QtTinySA lineage | no VNA class |

---

## 5. Pilot families for the next command/reply cross-check

The first wave (big-library vectors, 14 definitions) is done in `crosscheck/`. Its lesson was that
pymeasure/Qcodes replies are author-written and decided nothing against the manual. The second wave
should therefore favour **captured or hardware-checked replies** and **hobby relevance**.

| # | Family (definitions reached) | Source | Why |
|---|---|---|---|
| 1 | **Siglent SDG** (`bases/siglent_sdg` → sdg1000x, sdg2000x, sdg6000x, bk_4050b, bk_4060b) | sdg2122-cli: 240 BSWV + 25 OUTP **live replies** with scope-validated runs; pymeasure T3AFG 11; devsignal | Most real replies in the inventory; one base reaches 5 families; SDG1032X is a top hobby AWG. Parsing already checks out, so the pilot can concentrate on setters and the USBTMC-vs-TCP decision. |
| 2 | **Siglent SDS1000X-E** (`siglent_sds1000xe`) | pymeasure LeCroy T3DSO1204 (144 vectors, `_with_device`); scopehal (tested SDS1104X-E); sigrok; sdsremote | The most popular hobby scope with no cross-check yet; the legacy set has 3 independent implementations that agree. |
| 3 | **Korad / Tenma** (`korad_ka`, `tenma_72`) | tenma-serial (per-model HW), koradctl (firmware list), sigrok (timing table) | The most widespread hobby PSU protocol. Already shows real contradictions (13320/13330/13360 framing, pacing). Our 72-2540 bench capture gives ground truth for scoring the libraries. |
| 4 | **Rigol MSO5000 + DS1000Z** (`rigol_mso5000`, `rigol_ds1000z`) | pyMSO5000 (2236 firmware-extracted headers, ≈ 226 tuples, MSO5074 HW); amosborne rigol-ds1000z (42 HIL tests) | Firmware-derived headers are the closest thing to ground truth we can get; DS1054Z is the most common hobby scope (its first-wave check had only 13 synthetic Qcodes vectors). |
| 5 | **OWON XDM / XDM3000** (`owon_xdm`, `owon_xdm3000`) | rusty_meter (real replies, HW across 7 models), TheHWcave, martin-bochum | Cheap and very common. Four hardware-backed contradictions already found; owon_xdm is on our bench, so outside claims can be checked directly. |
| 6 | **tinySA** (`tinysa`) | firmware source (`sa_cmd.c`) + tinySA_python replies (ZS405 HW) + QtTinySA | The firmware source decides every format question (found the `swt_q` bug); a hugely popular hobby SA. |

Runners-up: siglent_spd1000x/spd3303x (siglent-mcp HW + python-spd3303x; SPD3303X LAN terminator
bug), unit_udp3000s (three HW-tested libraries), feeltech_fy6900 (fygen 59 vectors; needs a
firmware-version split first), agilent_34401a (PY1CX real transcript; already in the first wave).

---

## 6. Changes suggested to definitions (not applied)

Priority order. Each needs a manual check or first-contact capture before it is applied.

1. tenma_72: split 72-13320/13330 (LF, per-channel OUT, 2-byte status) and 72-13360 (RS485 variant).
2. owon_xdm3000: USBTMC `5345:1234` (added 2026-10-18), rate `S` not `L`, baud ≤ 57600.
3. owon_xdm: `1E9` overload → null/OL; DIOD/CONT swap below fw V4.3; FRES only on 2041/3041/3051.
4. tinysa: `swt_q` with an SI prefix (`m`, `u`).
5. Siglent SDG (and probably SPD3303X): prefer TCP; treat Linux USBTMC as unverified (bTag defect).
6. siglent_spd3303x: add USB `F4EC:1430`; RS PRO IDN; LAN commands without `\n` on fw 1.01.01.02.05.
7. juntek_dpm86xx: allow a trailing `.` in the identify match.
8. rigol_dl3000 (`scpi_load meas_*`): parse the first line only.
9. feeltech_fy6900: firmware-dependent `WMF` units (`UVE` at identify).
10. korad_ka / tenma_72: slower pacing (≥ 80 ms per request; 450 ms on PS3005D V1.3).
11. manson_hcs: output state unknown at start (on at power-up).
12. rigol_dp800: `tcp 5555`. unit_utg900e: USB `6656:0834`, UTG900 IDN form. gwinstek_gdm9000: PID `0059`.
13. scpi_osc (MSO5000): reset `:WAV:STAR/STOP` on connect. keysight_e4980a: `FORM ASC` on connect.
    agilent_34401a: resend `SYST:REM` after a timeout. itech_it8500plus: mark the serial SCPI link unverified.
