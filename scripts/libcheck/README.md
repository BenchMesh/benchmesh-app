# libcheck - cross-check device definitions against open-source instrument libraries

Dev tooling, never shipped. Two steps:

1. **`extract.py`** parses (with `ast` / YAML, nothing is imported or executed) the protocol tests of
   PyMeasure (`expected_protocol(cls, [(write, reply), …])`), InstrumentKit
   (`expected_protocol(cls, [writes], [replies])`, replies paired with the writes containing `?` in
   order) and QCoDeS pyvisa-sim YAML (`dialogues`, `properties` getter/setter with the default value)
   into neutral JSON vectors:
   `{library, file, test, instrument_class, exchanges: [{write, reply|null}], note?}`.
   `pytest.mark.parametrize` (stacked), module constants, local assignments, `for` loops, f-strings,
   `%`/`.format` are expanded; tests that need fixtures or hypothesis are skipped (counted as
   `unresolved`).
2. **`crates/bm-driver/examples/crosscheck.rs`** matches the vectors against our definitions
   (`mapping.yaml`) and writes `ai_context/migration/coverage/crosscheck/<id>.md` + `README.md`.

Output of step 1 goes **outside the repository**, `BenchMeshDOC/_vectors/<library>/<class>.json`:
InstrumentKit is AGPLv3 - its tests are used as facts only, nothing of it is copied into BenchMesh
(the script refuses an output dir inside the repo). PyMeasure and QCoDeS are MIT.

## Setup

The libraries are clones under `BenchMeshDOC/_libs/` (`pymeasure`, `Qcodes`, `InstrumentKit`).
Only PyYAML is needed:

```bash
python3 -m venv ~/.venvs/libcheck
~/.venvs/libcheck/bin/pip install pyyaml
~/.venvs/libcheck/bin/python -I scripts/libcheck/extract.py        # --libs DIR --out DIR --only pymeasure,qcodes,instrumentkit
cargo run -q -p bm-driver --example crosscheck                     # --vectors DIR --mapping FILE --devices DIR --out DIR --only id,id
```

## What the cross-check reports

- **(a) replies**: a vector query that matches one of our commands goes through our parse pipeline
  (`bm_driver::parse::parse_reply`, after our framing's trailing strip); failures first.
- **(b) writes**: each vector setter (split on `;` with SCPI path rules) against our `send`
  templates - `{ch}` and enum params expanded to their wire values, other params wildcards checked with
  `bm_driver::wire::format_param` (min/max, format). Verdicts: equivalent · equivalent (optional nodes) ·
  value rejected/changed · same header, other spelling/value · no such command.
- **(c) ours unseen**: polled/get queries and setters no vector exercises.

Normalisation: case-insensitive, SCPI short forms (4 letters, 3 when the 4th is a vowel), optional
leading `:`, node suffix default 1, numbers compared numerically, `ON|OFF` ≡ `1|0` (flagged). A second
pass drops `SOUR`/`SENS` at the root and `IMM`/`AMPL`/`LEV`/`STAT`/`DC` inside.

Reports keep a hand-written `## Review` section (per family) and `## Pilot` (summary) across
regeneration. Library vectors are other people's expectations: decide each disagreement with the
manual, change a definition only when the manual sides with the library.

## Mapping

`mapping.yaml`: definition id → `{transport, vectors: {library: [instrument_class]}, note}`. Extend it
from the overlap matrix in `ai_context/migration/coverage/LIBRARIES.md`.
