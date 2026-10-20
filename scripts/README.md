# Scripts

Development tools; none of them is shipped with the app.

| Script | What it does |
|---|---|
| `bench_setters.sh` | Live setter check of the device definitions on the bench with `bm-probe`: every block sets a value, verifies it by read-back and restores what it found. Safe only with nothing connected to any instrument output or input. |
| `bench_scope_setters.sh [config] [device-id]` | The same for every RIGOL DHO800/DHO900 setter (configuration only, nothing energises). |
| `libcheck/` | Cross-checks the definitions against the protocol tests of open-source instrument libraries (PyMeasure, InstrumentKit, QCoDeS); see its README. |
| `logo/build.mjs` | Writes the logo files, the favicon and the desktop app icons from `frontend/src/ui/kit/brand.ts` (needs Inkscape and the Tauri CLI). |

Build `bm-probe` first for the bench scripts: `cargo build -p bm-probe`.
