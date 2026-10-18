# Third-Party Software Licenses

BenchMesh includes or depends upon third-party software components that are distributed under various open source licenses. This document lists those components and their respective licenses.

---

## Python Dependencies

### pyserial
- **License**: BSD-3-Clause
- **Copyright**: Copyright (c) 2001-2020 Chris Liechti
- **URL**: https://github.com/pyserial/pyserial

### PyYAML
- **License**: MIT
- **Copyright**: Copyright (c) 2017-2021 Ingy döt Net, Copyright (c) 2006-2016 Kirill Simonov
- **URL**: https://github.com/yaml/pyyaml

### FastAPI
- **License**: MIT
- **Copyright**: Copyright (c) 2018 Sebastián Ramírez
- **URL**: https://github.com/tiangolo/fastapi

### Uvicorn
- **License**: BSD-3-Clause
- **Copyright**: Copyright © 2017-present, Encode OSS Ltd.
- **URL**: https://github.com/encode/uvicorn

### Pydantic
- **License**: MIT
- **Copyright**: Copyright (c) 2017 to present Pydantic Services Inc. and individual contributors
- **URL**: https://github.com/pydantic/pydantic

### HTTPX
- **License**: BSD-3-Clause
- **Copyright**: Copyright © 2019, Encode OSS Ltd.
- **URL**: https://github.com/encode/httpx

### SQLAlchemy
- **License**: MIT
- **Copyright**: Copyright (c) 2005-2024 Michael Bayer and contributors
- **URL**: https://github.com/sqlalchemy/sqlalchemy

### websockets
- **License**: BSD-3-Clause
- **Copyright**: Copyright (c) 2013-2024 Aymeric Augustin and contributors
- **URL**: https://github.com/python-websockets/websockets

---

## Rust Dependencies

Direct dependencies of the Rust crates (`crates/`). Transitive dependencies are under permissive
licenses (MIT, Apache-2.0, BSD-3-Clause, Zlib, Unicode-3.0, BSL-1.0, 0BSD, Unlicense); the only
exceptions are `serialport` (MPL-2.0, listed below), `unescaper` (MIT OR GPL-3.0-only, used
under MIT; pulled in by `serialport`) and `r-efi` (MIT OR Apache-2.0 OR LGPL-2.1+, used under MIT).

### serde, serde_json
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/serde-rs/serde, https://github.com/serde-rs/json

### serde_yaml_ng
- **License**: MIT
- **URL**: https://github.com/acatton/serde-yaml-ng

### schemars
- **License**: MIT
- **URL**: https://github.com/GREsau/schemars

### regex
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/rust-lang/regex

### cel (cel-rust)
- **License**: MIT
- **URL**: https://github.com/cel-rust/cel-rust

### serialport (serialport-rs)
- **License**: MPL-2.0 (file-level copyleft: the crate's own source files stay under MPL-2.0;
  using it from MIT code is permitted; modifications to its files must be shared under MPL-2.0)
- **URL**: https://github.com/serialport/serialport-rs
- Used without the `libudev` feature.

### libc
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/rust-lang/libc

### windows-sys (bm-transport, Windows only)
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/microsoft/windows-rs

### io-kit-sys, core-foundation, core-foundation-sys (bm-transport, macOS only)
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/jtakakura/io-kit-rs, https://github.com/servo/core-foundation-rs

### nusb (bm-transport)
- **License**: Apache-2.0 OR MIT (used under MIT)
- **URL**: https://github.com/kevinmehall/nusb
- Userspace USB access for USBTMC instruments on Windows (WinUSB) and macOS.

### libloading (bm-transport)
- **License**: ISC
- **URL**: https://github.com/nagisa/rust_libloading
- Loads an installed VISA library at run time (no VISA code is bundled).

### rusqlite (bm-store) with bundled SQLite
- **License**: rusqlite MIT; SQLite is in the public domain
- **URL**: https://github.com/rusqlite/rusqlite, https://sqlite.org/copyright.html

### chrono
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/chronotope/chrono

### axum, tokio, tower-http (bm-server)
- **License**: MIT
- **URL**: https://github.com/tokio-rs/axum, https://github.com/tokio-rs/tokio, https://github.com/tower-rs/tower-http

### tauri, tauri-build, tauri-plugin-updater (benchmesh-app)
- **License**: Apache-2.0 OR MIT (used under MIT)
- **URL**: https://github.com/tauri-apps/tauri, https://github.com/tauri-apps/plugins-workspace
- Desktop app shell; the updater checks for, verifies (minisign signature) and installs new versions.

### tower, tokio-tungstenite, futures-util (test-only)
- **License**: MIT / MIT / MIT OR Apache-2.0
- **URL**: https://github.com/tower-rs/tower, https://github.com/snapview/tokio-tungstenite, https://github.com/rust-lang/futures-rs

### flate2 (test-only)
- **License**: MIT OR Apache-2.0
- **URL**: https://github.com/rust-lang/flate2-rs

---

## Node.js Dependencies

### Node-RED
- **License**: Apache-2.0
- **Copyright**: Copyright JS Foundation and other contributors
- **URL**: https://github.com/node-red/node-red

---

## Frontend Dependencies

### React & React-DOM
- **License**: MIT
- **Copyright**: Copyright (c) Meta Platforms, Inc. and affiliates
- **URL**: https://github.com/facebook/react

### react-markdown
- **License**: MIT
- **Copyright**: Copyright (c) 2015 Espen Hovlandsdal
- **URL**: https://github.com/remarkjs/react-markdown

### remark-gfm
- **License**: MIT
- **Copyright**: Copyright (c) 2020 Titus Wormer
- **URL**: https://github.com/remarkjs/remark-gfm

### swagger-ui-react
- **License**: Apache-2.0
- **Copyright**: Copyright 2023 SmartBear Software
- **URL**: https://github.com/swagger-api/swagger-ui

### echarts
- **License**: Apache-2.0
- **Copyright**: Copyright (c) 2017-2024 The Apache Software Foundation
- **URL**: https://github.com/apache/echarts

### react-icons
- **License**: MIT (icon sets under their own licenses; BenchMesh uses VS Code Codicons, CC-BY-4.0)
- **Copyright**: Copyright (c) 2018 kamijin_fanta
- **URL**: https://github.com/react-icons/react-icons

### B612 and B612 Mono fonts (@fontsource/b612, @fontsource/b612-mono)
- **License**: SIL Open Font License 1.1 (fonts); MIT (Fontsource packaging)
- **Copyright**: Copyright 2012 The B612 Project Authors (Airbus, Intactile Design)
- **URL**: https://github.com/polarsys/b612 , https://github.com/fontsource/font-files
- Bundled with the UI (instrument readouts and labels); the fonts are unmodified.

### Chakra Petch font (@fontsource/chakra-petch)
- **License**: SIL Open Font License 1.1 (font); MIT (Fontsource packaging)
- **Copyright**: Copyright 2018 The Chakra Petch Project Authors (Cadson Demak)
- **URL**: https://github.com/cadsondemak/Chakra-Petch
- Bundled with the UI (panel titles); the font is unmodified.

---

## Electron Dependencies

### tree-kill
- **License**: MIT
- **Copyright**: Copyright (c) 2014 Peter Kvitek
- **URL**: https://github.com/pkrumins/node-tree-kill

### Electron (Development Dependency)
- **License**: MIT
- **Copyright**: Copyright (c) Electron contributors, Copyright (c) 2013-2020 GitHub Inc.
- **URL**: https://github.com/electron/electron

### electron-builder (Development Dependency)
- **License**: MIT
- **Copyright**: Copyright © 2015 Loopline Systems
- **URL**: https://github.com/electron-userland/electron-builder

---

## License Texts

### MIT License

```
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### BSD-3-Clause License

```
Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors
   may be used to endorse or promote products derived from this software without
   specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### Apache-2.0 License

The full text of the Apache License 2.0 can be found at:
https://www.apache.org/licenses/LICENSE-2.0

Key permissions:
- Commercial use
- Modification
- Distribution
- Patent use
- Private use

Key conditions:
- License and copyright notice must be included
- State changes made to the code
- Include NOTICE file if present

---

## Summary

All dependencies use permissive open source licenses that allow:
- ✓ Commercial use
- ✓ Modification
- ✓ Distribution
- ✓ Private use

No copyleft (GPL-style) licenses are used in BenchMesh or its dependencies.

The bundled fonts are under the SIL Open Font License 1.1, which permits bundling them with
software; the fonts themselves may not be sold on their own.
