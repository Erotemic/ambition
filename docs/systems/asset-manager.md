
# Asset manager

`crates/ambition_asset_manager/` owns Ambition's logical asset catalog: stable IDs, source/profile selection, preload groups, required-vs-optional policy, and platform-aware resolution. It is the project vocabulary for "which asset is this?" and "where may this platform load it from?"

It does **not** replace Bevy's `AssetServer`, handles, load states, dependencies, hot reload, or `AssetReader` infrastructure. Bevy remains the runtime loader.

## Current policy

- Runtime code should refer to stable Ambition asset IDs rather than hard-coded platform paths when the asset is part of the game catalog.
- The catalog selects a source according to an `AssetProfile`: loose desktop files, installed files, embedded/static assets, Android APK assets, served web assets, or headless/no-asset modes.
- Required assets should fail loudly unless the active profile explicitly tolerates missing assets.
- Optional/presentation assets may fall back, skip display, or use embedded placeholders depending on the caller.
- Generated assets become runtime assets only after an explicit publish/install step.

## Platform profile matrix

| Profile family | Current role | Hot reload | Notes |
|---|---|---:|---|
| Desktop dev loose | Main local authoring path | yes | LDtk and loose assets can reload from disk. |
| Desktop/Steam Deck installed | Installed desktop builds | no | Same logical catalog, different root policy. |
| Web static / served assets | Browser builds | no | Static embedding and `/assets/...` serving both matter. Keep wasm constraints visible. |
| Android bundle | Mobile/APK builds | no | APK asset reader plus touch/controller expectations. |
| iOS bundle | Deferred target | no | Keep profile vocabulary, but do not claim tested support until macOS hardware exists. |
| Headless / no-assets | CI and simulation | no | Gameplay tests should not require presentation assets unless explicitly testing loading. |

## Runtime flow

```text
AssetId
  -> AssetManifest entry
  -> AssetProfile preferred sources
  -> ResolvedAsset path/source/policy
  -> Bevy AssetServer or non-Bevy byte consumer
```

The asset manager answers the catalog question. The caller still decides whether it needs a Bevy handle, an SFX bank byte provider, a filesystem path for a tool, or a skip/fallback path.

## Edit protocol

When adding a new asset family:

1. Add stable IDs and manifest entries.
2. Decide which platform profiles can load it.
3. Decide required vs optional behavior.
4. Add publish/install tooling if the asset is generated.
5. Add focused validation for at least one local profile and one constrained profile when feasible.
6. Update `docs/tools/` if a generator or packer owns the source artifact.

## Current gaps

- Some generated visual/audio assets still have tool-local workflows rather than one unified publish manifest.
- Web/audio/mobile behavior is platform-sensitive; validate the target feature set instead of assuming desktop behavior.
- iOS is a modeled profile, not a tested shipping target.

## Validation

Use targeted checks first:

```bash
cargo test -p ambition_asset_manager
cargo test -p ambition_gameplay_core --lib asset
python scripts/check_agent_kb.py
```

For platform changes, also run the relevant build recipe under `docs/recipes/`.
