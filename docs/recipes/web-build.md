---
status: current
last_verified: 2026-10-30
---

# Web build and local serve

The supported interface is `build_for_web.sh`. Its `--help` and `--doctor`
output are authoritative for current features, bindgen target, output paths, and
asset persona.

## Prerequisites

```bash
./scripts/setup/web_prereq.sh --doctor
./scripts/setup/web_prereq.sh --with-server
./build_for_web.sh --doctor
```

## Common workflows

```bash
# Embedded/static core-asset web build.
./build_for_web.sh

# Build and serve locally.
./build_for_web.sh --serve
./build_for_web.sh --serve 9000 --open

# Bind to all IPv4 interfaces so other devices can connect.
./build_for_web.sh --serve --host all

# Full served-assets persona; exposes provider assets under /assets/.
./build_for_web.sh --served --serve

# Faster development compile.
./build_for_web.sh --debug --serve
```

`--served` is a distinct asset profile, not a gameplay fork. Runtime/provider
selection and simulation semantics must remain the same across browser asset
transport modes.

## Standalone Mary-O

```bash
./build_for_web.sh --game mary-o --serve
./build_for_web.sh --game mary-o --serve --host 0.0.0.0
./build_for_web.sh --game mary-o --copy-assets --brotli
```

The server uses `localhost` (`127.0.0.1`) by default. Set `--host all` to
listen on `0.0.0.0` (all IPv4 interfaces), or pass `--host 0.0.0.0` directly.
You can also set `AMBITION_WEB_HOST=all`; the `--host` option takes priority.
Use `http://<LAN-IP>:8000/` on other devices, with the actual IP address of
the serving computer. A firewall must permit the connection. `--open` uses
a local browser address even when the server listens on all interfaces.

Mary-O uses its standalone provider and rendered shell. Its default web build
uses `web-release`: size optimization, thin LTO, and stripped symbols. It includes
audio and the shared touch controls. The page uses WebGL2.

`static_content` embeds pack texts and the library's declared fallback assets.
`served_content` embeds metadata and leaves images and audio to the platform asset
reader. Mary-O's `web` feature selects `served_content`. Both modes use the same
content identities and simulation.

Build scripts compile sprite-part metadata from the shared schema. The browser
does not parse that RON at startup. The shared first-room loading job waits for
the initial images and music before session activation. The page shows download
progress before the engine starts; the engine then shows its loading screen.

Mary-O's LDtk source and editor sidecars are under `authoring/worlds`.
The provider compiles the same source for each host. These authoring inputs
do not belong in the served asset tree.

The output is `game/ambition_demo_mary_o_app/web/`. Use `--copy-assets` before
moving it to another host. The default development output links to source files.
The current package contains the full selected library. The browser loads asset
bytes on demand.

The optional `--brotli` flag requires the `brotli` command. On Debian or Ubuntu,
install it with `sudo apt-get install brotli`. The build writes both Brotli and
gzip representations. Without the flag, the build writes gzip and removes old
Brotli representations.

The local server selects a representation the browser accepts and keeps the
original MIME type. A different host must send `Content-Encoding: br` for
Brotli or `Content-Encoding: gzip` for gzip. WASM needs `application/wasm`.
Code representations are beside their source files. Audio-bank representations
are under `.encoded/assets/`. Serve each representation at the original asset
URL.

To serve an existing output without a Cargo build:

```bash
python3 scripts/package/web_bundle.py serve --root game/ambition_demo_mary_o_app/web --port 8000 --host all
```

## Measure transfer size

Each build writes `bundle-report.json` beside the page. It records raw and gzip
code sizes, compressed audio-bank sizes, and the available asset inventory.
Available assets are not an initial-download measurement.

The optional `--wasm-opt` step requires Binaryen. Compare its gzip output before
keeping it: a smaller raw module can have the same or a larger transfer size.

For a browser capture:

```bash
npm install --prefix /tmp/ambition-web-tools playwright
npx --prefix /tmp/ambition-web-tools playwright install chromium
NODE_PATH=/tmp/ambition-web-tools/node_modules node scripts/measure/measure_web.cjs http://localhost:8000/ target/mary-o-web
```

The capture uses a fresh temporary browser profile for both visits. It records
a fresh visit, a cached visit, HTTP traffic, WASM memory,
console output, long tasks, and screenshots before and after keyboard input.
It records canvas startup and first-room reveal as separate milestones. The
capture waits for the reveal. Inspect the screenshots to confirm gameplay. Set
`AMBITION_SOFTWARE_GPU=1` to use SwiftShader in a headless environment; its frame
rate is not a hardware performance measurement.

## Validate

Start with `./build_for_web.sh --doctor` when diagnosing toolchain or
configuration problems. For a web change, build the affected game/profile and
open it in a browser; verify startup, input and the behavior you changed.

Use [the browser audio check](web-audio-manual-test.md) for changes to audio
or gesture handling. Run focused Rust tests when changing a shared loader,
serialization contract or other behavior they actually exercise. Browser-only
device/presentation defects should be fixed at the relevant boundary.
