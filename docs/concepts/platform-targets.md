---
id: platform-targets
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
implemented_by:
  - crates/ambition_runtime
  - crates/ambition_host
  - crates/ambition_asset_manager
  - crates/ambition_input
  - crates/ambition_touch_input
  - game/ambition_app
related_docs:
  - docs/recipes/web-build.md
  - docs/recipes/android-build.md
---

# Platform targets

Ambition supports desktop first while preserving web, Android/touch, controller,
and Steam Deck paths. iOS is deferred by available hardware/tooling, not ruled
out architecturally.

## Layering

- `ambition_runtime` composes headless-safe simulation and schedule ordering.
- provider crates contribute gameplay/content without assuming a windowing
  platform.
- `ambition_host` adds window/device/presentation policy.
- app crates choose product features and packaging.
- `ambition_input` defines semantic device adapters and presets.
- `ambition_touch_input` adds the mobile touch presentation/input adapter.
- asset/loading crates preserve logical identity while hosts choose platform
  source/packaging details.

## Invariants

- Outcome-changing code runs headlessly and is not hidden behind visible/window
  features.
- Platform `cfg` and feature flags live at the narrowest host/adapter boundary.
- Core/domain crates do not import Android, wasm, window, or device APIs merely
  to preserve a convenience call site.
- Web audio unlock and Android asset packaging are explicit readiness concerns.
- Touch/controller labels derive from semantic actions and `ControlPrompt`, not
  hard-coded keyboard text.
- Desktop filesystem existence does not prove web/static/APK availability.
- Platform-specific degradation is explicit and must not silently change game
  rules.

## Validation ladder

1. owner-crate tests with the relevant feature;
2. `./run_tests.sh` headless suite;
3. target compilation/build recipe;
4. device/browser manual acceptance for input, audio, focus, and lifecycle.

Use the current recipe rather than copying old Cargo feature strings:

```bash
./build_for_web.sh
./build_for_android.sh
```
