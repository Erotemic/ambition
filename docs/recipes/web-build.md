---
status: current
last_verified: 2026-07-18
---

# Web build and local serve

The supported interface is `build_for_web.sh`. Its `--help` and `--doctor`
output are authoritative for current features, bindgen target, output paths, and
asset persona.

## Prerequisites

```bash
./scripts/setup_web_prereq.sh --doctor
./scripts/setup_web_prereq.sh --with-server
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
