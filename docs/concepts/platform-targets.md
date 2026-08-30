---
id: platform-targets
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-30
implemented_by:
  - crates/ambition_platformer2d_runtime
  - crates/ambition_platformer2d_host
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

- `ambition_platformer2d_runtime` composes headless-safe simulation and schedule ordering.
- provider crates contribute gameplay/content without assuming a windowing
  platform.
- `ambition_platformer2d_host` adds window/device/presentation policy.
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

## Shared visible composition

A platform host may differ in window/canvas/device setup, asset delivery, input
unlock policy and packaging. It should not hand-spell a second copy of the game
composition. Desktop, browser, offscreen/capture and future device hosts compose
the same visible game/session contract over different host foundations.

A compile/link check proves that a target builds; it does not prove that the
application installed its shell, route, session visuals or asset source. Keep a
behavioral composition test over the shared spec so a platform cannot silently
become a different application.

Packaging follows the same rule: publish the product asset contract, not one
implementation crate's `assets/` directory. A platform build must survive
internal crate moves when logical asset identity and provider catalogs are
unchanged.

## Validation ladder

1. owner-crate tests with the relevant feature;
2. behavioral composition/asset-contract tests against the shared host spec;
3. `./run_tests.sh` headless suite for simulation contracts;
4. target compilation/link and packaging recipe;
5. device/browser manual acceptance for input, audio, focus, lifecycle and an
   actually visible frame.

Use the current recipe rather than copying old Cargo feature strings:

```bash
./build_for_web.sh
./build_for_android.sh
```
