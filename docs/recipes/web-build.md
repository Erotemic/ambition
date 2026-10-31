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
./build_for_web.sh --game mary-o --copy-assets
```

Mary-O uses its standalone provider and rendered shell. Its default web build
uses `web-release`: size optimization, thin LTO, and stripped symbols. It includes
audio and the shared touch controls. The page uses WebGL2.

`static_content` embeds pack texts and the library's declared fallback assets.
`served_content` embeds metadata and leaves images and audio to the platform asset
reader. Mary-O's `web` feature selects `served_content`. Both modes use the same
content identities and simulation.

Mary-O's LDtk source and editor sidecars are under `authoring/worlds`.
The provider compiles the same source for each host. These authoring inputs
do not belong in the served asset tree.

The output is `game/ambition_demo_mary_o_app/web/`. Use `--copy-assets` before
moving it to another host. The default development output links to source files.
The current package contains the full selected library. The browser loads asset
bytes on demand.

The local server sends gzip representations with the original MIME type. A
different host must set `Content-Encoding: gzip` when it sends a `.gz`
representation and must serve WASM as `application/wasm`.
Code representations are beside their source files. Audio-bank representations
are under `.encoded/assets/`. Serve each representation at the original asset
URL with its original MIME type and `Content-Encoding: gzip`.

To serve an existing output without a Cargo build:

```bash
python3 scripts/web_bundle.py serve --root game/ambition_demo_mary_o_app/web --port 8000
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
NODE_PATH=/tmp/ambition-web-tools/node_modules node scripts/measure_web.cjs http://localhost:8000/ target/mary-o-web
```

The capture uses a fresh temporary browser profile for both visits. It records
a fresh visit, a cached visit, HTTP traffic, WASM memory,
console output, and screenshots before and after keyboard input. Canvas startup
is a host milestone. Inspect the screenshots to confirm gameplay. Set
`AMBITION_SOFTWARE_GPU=1` to use SwiftShader in a headless environment; its frame
rate is not a hardware performance measurement.

## Validate

```bash
./run_tests.sh -p ambition_asset_manager
./run_tests.sh -p ambition_audio
./run_tests.sh -p ambition_content
./build_for_web.sh --doctor
```

Then use [`web-audio-manual-test.md`](web-audio-manual-test.md) when audio or
browser gesture/unlock behavior changed. Browser-only presentation/device issues
should not be “fixed” in simulation code.
