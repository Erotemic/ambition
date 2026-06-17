---
id: platform-targets
status: current
aliases:
  - web build
  - Android build
  - mobile touch
  - Steam Deck
  - controller support
  - iOS deferred
implemented_by:
  - crates/ambition_gameplay_core/Cargo.toml
  - build_for_web.sh
  - build_for_android.sh
  - deploy_to_steamdeck.sh
related_docs:
  - docs/recipes/web-build.md
  - docs/recipes/android-build.md
  - docs/systems/mobile-touch-controls.md
  - docs/systems/asset-manager.md
last_verified: 2026-05-17
---

# Platform targets

## Definition

Ambition should keep a wide platform surface healthy: desktop, web, Android/mobile touch, controller, and Steam Deck. iOS is deferred until macOS test hardware is available.

## Core invariant

Do not make a subsystem desktop-only by accident. If a feature touches input, assets, app features, audio, save/settings, or rendering, consider all active platform profiles.

## Current target matrix

| Target | Current stance |
|---|---|
| Desktop | Primary dev/runtime target with debug tools and inspector paths. |
| Web | Active wasm target with static and served-asset personas. |
| Android | Active phone-test target with APK asset packaging and touch controls. |
| Mobile touch | Touch controls should be used where available, not treated as a later rewrite. |
| Controller / Steam Deck | Controller semantics and asset-root robustness matter. |
| iOS | Deferred only because macOS test hardware is unavailable. |

## Edit protocol

When changing platform-sensitive code:

1. Check feature flags in `crates/ambition_gameplay_core/Cargo.toml`.
2. Check the relevant recipe (`web-build`, `android-build`, mobile touch, asset manager).
3. Preserve headless/minimal test paths where possible.
4. Avoid assuming host filesystem access on web/Android.
5. Preserve controller/touch semantic input paths.

## Common failure modes

- Adding a desktop-only plugin to a web/Android feature set.
- Assuming loose filesystem assets instead of using the asset manager.
- Updating keyboard controls but not touch/controller mapping.
- Replacing broad app files and losing platform-specific entrypoints.
