---
id: asset-management
aliases:
  - asset manager
  - asset catalog
  - Bevy asset loading
  - platform assets
implemented_by:
  - crates/ambition_asset_manager/src
  - crates/ambition_asset_manager/src/sandbox_assets
  - crates/ambition_actors/src/assets/mod.rs
  - crates/ambition_actors/assets/ambition/sandbox.ron
related_docs:
  - docs/systems/asset-manager.md
  - docs/systems/asset-manager.md
  - docs/planning/engine/architecture.md
related_memory:
  - dev/journals/lessons_learned.md
last_verified: 2026-07-07
---

# Asset management

## Definition

Asset management covers logical asset IDs, manifests, preload/profile policy, platform-aware resolution, and the bridge from generated/source assets into Bevy runtime handles.

`ambition_asset_manager::sandbox_assets` owns the sandbox catalog/resource,
stable ids, manifest builders, and embedded-source plugin. Gameplay-core's
`assets` module is now an adapter/handle-loading tail: it assembles game rows
for the catalog and keeps Bevy image handles plus gameplay presentation
vocabulary until the E3/E6/E7 carves move those dependencies.

## Core invariants

- Logical asset identity should survive platform differences.
- Desktop host-path checks do not imply Android APK asset availability.
- Generated assets should be reproducible from source specs and generators.
- Asset loading state should become explicit over time rather than hiding failures in ad hoc startup code.

## Edit protocol

1. Identify whether the change is logical catalog, runtime load state, generated output, or platform packaging.
2. Preserve platform-aware paths and Android APK behavior.
3. Search dev memory for asset/platform lessons before changing broad startup code.
4. Update manifest/docs when adding a durable asset category.

## Validation

```bash
cargo test -p ambition_asset_manager
cargo test -p ambition_actors --lib assets
cargo run -p ambition_app --bin headless
```
