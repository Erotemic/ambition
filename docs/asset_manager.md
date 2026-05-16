# Ambition asset manager

`crates/ambition_asset_manager/` owns the **logical asset layer** for
Ambition: stable `AssetId`s, a manifest of `AssetEntry` records,
per-platform `AssetProfile` personas, missing/cache policy, and
preload-group tagging. It does **not** replace Bevy's `AssetServer`,
`AssetReader`, `AssetPath`, or load-state machinery — those continue to
own runtime async loading, handles, dependencies, and hot reload.

## Why a separate layer

Bevy already covers:

- async / non-blocking loading via `AssetServer` and typed `Handle`s
- pluggable byte backends via `AssetReader` / `AssetSource`
- source-qualified virtual paths via `AssetPath`
   (`embedded://...`, `remote://...`, ...)
- embedded asset macros + the `embedded` asset source
- load-state checks, dependencies, hot reload

What Bevy does **not** cover, and what `ambition_asset_manager` owns:

- logical Ambition asset ids (`sprite.entity.chest_closed`,
  `world.sandbox_ldtk`, `audio.sfx_bank`)
- per-platform / per-profile source selection (desktop loose,
  desktop installed, Android APK, iOS bundle, web HTTP / static,
  bundled-static, no-assets, headless, IPFS gateway placeholder)
- required-vs-optional policy with fallback rules
- preload groups by game domain (bootstrap, HUD, sandbox-core, zone, ...)
- SFX bank identity (a non-Bevy byte consumer)
- LDtk bootstrap policy
- IPFS gateway URL construction (placeholder; no native client)

The split mirrors the existing
`game_assets.rs::GameAssets` design rule: *callers see a high-level
catalog and don't care where handles came from* — the catalog policy
moves out of the sandbox into a reusable crate so future crates / tools
can share it.

## Architecture

```
ambition_asset_manager
  ├── core (no Bevy)
  │   AssetId, AssetKind, AssetEntry, AssetManifest,
  │   AssetLocation, AssetProfile, AssetSourceProfile,
  │   MissingAssetPolicy, CachePolicy, PreloadGroup, resolver
  │
  ├── bevy_integration (feature = "bevy")
  │   AmbitionAssetManagerPlugin, AmbitionAssetCatalog (Resource),
  │   AmbitionAssetProfile (Resource), load helpers that route through
  │   bevy AssetServer + AssetPath
  │
  └── sfx_integration (feature = "sfx")
      build_provider_from_resolved(&ResolvedAsset, Option<&[u8]>)
        -> Result<BankProvider, SfxBankResolveError>
```

The crate is `default-features = []` so headless / CLI tools can use the
core resolver without dragging Bevy or `ambition_sfx` into their
dependency graphs.

### Resolution flow

```
AssetId
  └── manifest.get(id) -> &AssetEntry
        └── for source in profile.preferred_sources():
              1. entry.locations[source].cloned()   if authored
              2. synthesize default from logical_path  for filesystem / embedded
        └── first non-Disabled hit -> ResolvedAsset
        └── nothing matched      -> Disabled (consult MissingAssetPolicy)
```

### Profile contract — live behavior matrix

| Profile                    | Preferred sources                                | Hot reload | LDtk | Sprites | Fonts | SFX bank | Music tracks | Notes |
| -------------------------- | ------------------------------------------------ | ---------- | ---- | ------- | ----- | -------- | ------------ | ----- |
| `DesktopDevLoose`          | LooseFilesystem → EmbeddedBinary → HttpRemote    | ✅         | ✅   | ✅      | ✅    | ✅       | ✅           | `cargo run` from the workspace; LDtk file watcher armed via `SandboxAssetCatalog::hot_reload_local_path`. |
| `DesktopInstalled`         | InstalledFilesystem → EmbeddedBinary → HttpRemote | ❌         | ✅   | ✅      | ✅    | ✅       | ✅           | Bevy `AssetReader` reads next to the binary; pre-check via `desktop_loose_file_exists`. |
| `SteamDeckInstalled`       | InstalledFilesystem → EmbeddedBinary → HttpRemote | ❌         | ✅   | ✅      | ✅    | ✅       | ✅           | Same shape as `DesktopInstalled`; kept distinct for future Deck-specific policy. |
| `AndroidBundle`            | AndroidApk → EmbeddedBinary                      | ❌         | ✅¹  | ✅²     | ✅²   | ✅¹      | ✅²          | Bevy Android `AssetReader` resolves through APK assets. ¹ LDtk via `static_map` embedded; SFX bank requires `static_sfx_bank`. ² Loaded if packaged. |
| `IosBundle`                | IosBundle → EmbeddedBinary                       | ❌         | ⚠️   | ⚠️      | ⚠️    | ⚠️       | ⚠️           | Profile modeled but no iOS build target yet — every loader honors the profile, packaging story is TBD. |
| `WebHttp`                  | HttpRemote → EmbeddedBinary                      | ❌³        | ⚠️   | ❌      | ❌    | ❌       | ❌           | Catalog produces `https://...` URLs when explicit candidates exist; today's sandbox has none, so optional assets resolve to `Disabled` for `should_attempt_optional_load`. ³ HTTP polling / ETag reload future. |
| `WebStatic`                | EmbeddedBinary → HttpRemote                      | ❌         | ✅¹  | ❌      | ❌    | ❌       | ❌           | Today's wasm first-pass build — LDtk via `static_map`; optional sprites / fonts / SFX / music are explicitly skipped (`should_attempt_optional_load` returns false). Renderer falls back to colored rectangles + Bevy default font. Wires up via `bevy_embedded_assets` in slice 9. |
| `BundledStatic`            | EmbeddedBinary                                   | ❌         | ✅¹  | ❌      | ❌    | ❌       | ❌           | Single-binary cross-platform demo build. Same status as `WebStatic` for optional assets — packaging is TBD. |
| `NoAssets`                 | (none)                                           | ❌         | 💀   | ❌      | ❌    | ❌       | ❌           | `--no-assets`; every entry resolves to `Disabled`. LDtk is `MissingAssetPolicy::Error` so `load_default` returns Err. |
| `Headless`                 | (none)                                           | ❌         | 💀   | ❌      | ❌    | ❌       | ❌           | Same as `NoAssets`; profile is marked `tolerates_missing_required` so callers can choose to keep going. |
| `IpfsGatewayPlaceholder`   | IpfsGateway → HttpRemote → EmbeddedBinary        | ❌         | ⚠️   | ⚠️      | ⚠️    | ⚠️       | ⚠️           | Builds `https://<gateway>/ipfs/<cid>/<path>` URLs from authored `IpfsGateway` candidates; no native IPFS client. No live assets carry CIDs today. |

Legend: ✅ working / ❌ explicitly skipped (placeholder/fallback) /
⚠️ profile modeled but no live build target / 💀 fatal (required asset).

### Hot reload preservation

`ResolvedAsset::supports_hot_reload()` is `true` only when **both** the
active profile and the resolved location report support. Today that
means: `DesktopDevLoose` + filesystem-backed location.

The sandbox's existing LDtk hot-reload path
(`crates/ambition_sandbox/src/ldtk_world/hot_reload.rs`) continues to
poll the filesystem. When the LDtk asset is migrated to the catalog,
the watcher will consult `ResolvedAsset::supports_hot_reload()` (and
`AssetLocation::as_local_path()` for the path to watch) instead of
hard-coding the desktop-only path resolver. Until then, the live
behavior is unchanged.

## Bevy integration

`AmbitionAssetManagerPlugin` inserts two resources:

- `AmbitionAssetCatalog(AssetManifest)` — the catalog.
- `AmbitionAssetProfile(AssetProfile)` — the active profile.

Helpers:

- `catalog.path_for(id, profile) -> Option<String>` — Bevy `AssetPath`
  string form, or `None` if disabled / non-Bevy-pathable.
- `catalog.load_optional::<T>(asset_server, id, profile) -> Option<Handle<T>>`
- `catalog.load_with_default::<T>(...) -> Handle<T>` — falls back to
  `Handle::default()` when disabled.

Source registrations (`embedded`, `http`, `https`, custom IPFS) are the
**consumer's responsibility**. Bevy's `AssetPlugin::source` is the
canonical hook. The crate intentionally doesn't auto-register sources
because the consumer knows which features it compiled with.

Recommended integrations:

- `BundledStatic` / `WebStatic` — `bevy_embedded_assets` with
  `ReplaceAndFallback` mode if a loose-fs fallback is wanted.
- `WebHttp` — enable Bevy's `http` and `https` features and the matching
  `AssetSource` registrations.
- `DesktopDevLoose` — default Bevy file `AssetSource` + `bevy/file_watcher`
  for hot reload.
- `bevy_asset_loader` — wrap preload groups with its
  `AssetCollection` / loading-state APIs.

## SFX integration

```rust
let resolved = catalog.resolve(&AssetId::new("audio.sfx_bank"), profile)?;
let provider = ambition_asset_manager::build_provider_from_resolved(
    &resolved,
    Some(include_bytes!("../assets/audio/sfx.bank")),
)?;
```

The adapter handles:

- `LocalPath` — calls `BankProvider::from_path`.
- `Embedded(_)` with caller-supplied bytes — calls `BankProvider::from_bytes`.
- `Disabled` / `HttpUrl` / `IpfsGateway` — explicit errors so the SFX
  system layers a `SilentProvider` or an async Bevy loader.

`ambition_sfx` retains its `BankProvider`, `FilesystemProvider`,
`SilentProvider`, and `LayeredProvider` layers. The asset manager
simply tells the SFX system *where* the bank bytes come from.

## Migration plan

| # | Slice | Status |
| - | ----- | ------ |
| 1 | Bootstrap entries (LDtk, sandbox RON) | **DONE** (2026-05-16) |
| 2 | Entity sprite + parallax layer loading | **DONE** (2026-05-16) |
| 3 | Character / boss spritesheet loading | **DONE** (2026-05-16) |
| 4 | SFX bank bytes (catalog-routed `BankProvider`) | **DONE** (2026-05-16) |
| 5 | LDtk hot-reload watcher consults `supports_hot_reload()` | **DONE** (2026-05-16) |
| 6 | UI fonts | **DONE** (2026-05-16) |
| 7 | Music tracks (asset path lookup) | **DONE** (2026-05-16) |
| 8 | Music cue layers (file-backed cues under `MusicCueCatalog`) | TODO |
| 9 | Bevy-native `AssetSource` for `embedded://` web bundle | TODO |

### Slice 2 — entity sprites + parallax layers (current)

Live entity-sprite loading and parallax-layer loading both run through
`ambition_asset_manager` as of 2026-05-16. The slice covers:

- **Stable logical ids** (`crates/ambition_sandbox/src/game_assets.rs`):
  - `entity_sprite_asset_id(EntitySprite) -> AssetId` →
    `sprite.entity.<lower_snake>`
  - `parallax_layer_asset_id(ParallaxTheme, ParallaxLayerAsset) -> AssetId` →
    `background.parallax.<theme>.<layer>`
- **Full manifest**: `sandbox_image_manifest(sprite_folder)` registers
  every `EntitySprite::ALL` variant + every
  `ParallaxTheme × ParallaxLayerAsset` pair as
  `MissingAssetPolicy::SilentPlaceholder` with preload groups
  `SandboxCore` (entity sprites) and `Zone` (parallax).
- **Active profile selection**: `GameAssetConfig::asset_profile` defaults
  via `default_asset_profile()` (cfg-driven: `WebStatic` on wasm,
  `AndroidBundle` on Android, `DesktopDevLoose` everywhere else). The
  `--no-assets` flag flips the profile to `NoAssets` so catalog
  resolution returns `Disabled` for every entry.
- **Loader rewrite**: `load_entity_sprites` and `load_parallax_layers`
  call `catalog.path_for(id, profile)` (Bevy `AssetServer` does the
  actual load). Both honor a single profile-gated
  `should_attempt_optional_image_load` helper that:
  - Pre-checks the host filesystem for `DesktopDevLoose` /
    `DesktopInstalled` / `SteamDeckInstalled` (preserves the
    colored-rectangle fallback for missing optional art).
  - Trusts the packager for `AndroidBundle` / `IosBundle`.
  - Skips the load on `WebStatic` / `WebHttp` / `BundledStatic` /
    `IpfsGatewayPlaceholder` (optional sprites aren't bundled yet —
    explicit `LocationCandidate`s will opt back in per asset once
    packaging lands).
- **`asset_exists` removed** from `game_assets.rs` (replaced by
  `desktop_loose_file_exists` consulted only when the active profile is
  desktop). No more `#[cfg(target_os = "android")]` branches in image
  loading. The standalone copies of `asset_exists` /
  `desktop_asset_exists` in `boss_sprites.rs` and `ui_fonts.rs` remain
  for now — those subsystems migrate in their own slices (3 and 6).

Behavior preserved (verified by tests in `game_assets.rs::tests`):

- `--sprite-folder custom` still rewrites every entity-sprite path; the
  catalog re-builds when `load_game_assets` runs.
- `--no-assets` still short-circuits with the existing log line; the
  catalog independently reports `Disabled` so any future catalog-only
  call site behaves consistently.
- Bevy `AssetServer` is still the actual loader.
- Desktop optional-image fallback to colored rectangles is unchanged.

### Slice 3 — full port (2026-05-16)

This slice flips every remaining live loader through the catalog and
deletes the legacy `asset_exists` / `desktop_asset_exists` copies that
were scattered across the sandbox.

- **`SandboxAssetCatalog`** (`crates/ambition_sandbox/src/sandbox_assets.rs`)
  is the single Bevy `Resource` that aggregates every asset id:
  - bootstrap (LDtk world, sandbox RON)
  - SFX bank
  - UI fonts (canonical + legacy fallbacks)
  - entity sprites + parallax layers
  - character spritesheets (player / robot / goblin / sandbag + every
    NPC sheet in `NPC_SPRITE_REGISTRY`)
  - boss spritesheets (gradient sentinel + mockingbird)
  - music tracks (one per `MusicTrackSpec` with an `asset_path`)
- Built once in `crate::app::init_sandbox_resources` from the live
  `GameAssetConfig` + the embedded `SandboxDataSpec`.
- `ambition_asset_manager::AssetProfile` selection now flows through
  `GameAssetConfig::asset_profile` (cfg-driven default + `--no-assets`
  override).
- Live loaders that asked for paths (entity sprites, parallax layers,
  character/boss sheets, fonts, SFX bank, LDtk world, sandbox RON,
  music tracks) all go through `SandboxAssetCatalog::path_for(...)` or
  `SandboxAssetCatalog::resolve(...)`.
- The **only** host-filesystem probe in the sandbox lives at
  `crate::sandbox_assets::desktop_loose_file_exists` (marked
  `[ambition_asset_manager_transition]`). Every other loader calls
  `catalog.should_attempt_optional_load(...)` or
  `catalog.should_attempt_required_load(...)`.
- LDtk hot reload preserved: `LdtkHotReloadState::from_catalog(...)`
  asks the catalog for a `LocalPath` via
  `SandboxAssetCatalog::hot_reload_local_path`; the watcher polls only
  that. On `WebStatic` / `AndroidBundle` / `BundledStatic` the
  `watch_path` is `None` and the watcher idles.
- `LdtkProject::load_default(&catalog)` is the production entry point;
  `LdtkProject::load_default_for_dev()` is the test/headless shortcut
  that builds a default desktop catalog internally.

### Remaining work (slices 8+)

8. **Music cue layers** — `MusicCueCatalog` cues
   (`crates/ambition_sandbox/src/music/director/loader.rs`) still build
   paths as `{cue.asset_root}/{source.path}`. Per-section/per-layer
   catalog ids would unify with the music-track path.
9. **Bevy-native `AssetSource` for `embedded://` web bundle** — once
   `bevy_embedded_assets` is wired into the wasm build,
   `should_attempt_optional_load` for `WebStatic` can flip to true for
   sprites/fonts known to be embedded.

## IPFS posture

First slice: gateway URL construction only. No `libp2p`, no
content-routing, no pinning. The
`AssetLocation::IpfsGateway { gateway, cid, path }` variant builds a
canonical HTTPS URL via `ipfs_gateway_url`; consumers fetch through
Bevy's `https` `AssetSource` like any other HTTP asset. A future slice
can grow this into native IPFS support behind a separate feature.

## How to add a new asset

1. **Pick an `AssetId`.** Stable, lowercase, dotted. Follow the existing
   prefix convention:
   - `sprite.entity.<name>`
   - `sprite.character.<name>`
   - `sprite.boss.<name>`
   - `background.parallax.<theme>.<layer>`
   - `font.<name>` (and optionally `font.<name>.legacy`)
   - `audio.<name>` (single clip) or `audio.<name>_bank` (packed bank)
   - `music.track.<id>`
   - `world.<name>` (LDtk)
   - `data.<name>` (RON)
2. **Add a builder for the id.** If the id is dynamic (e.g. derived
   from an enum / RON spec), add a `pub fn foo_asset_id(...) -> AssetId`
   next to the source enum. Otherwise add the constructor under
   `crate::sandbox_assets::ids`.
3. **Add a manifest entry.** Extend the right `extend_with_*` helper
   in `crate::sandbox_assets`. Pick:
   - `AssetKind` — Image / AudioClip / AudioBank / LdtkProject / RonData / Font / ...
   - `MissingAssetPolicy` — `Error` for required boot assets, `WarnAndPlaceholder`
     when the user should hear about it, `SilentPlaceholder` for fully optional art.
   - `PreloadGroup` — `Bootstrap` (boot blockers), `SandboxCore`
     (always-useful), `Zone` (per-room), `Hud`, `Cutscene`, `DevTools`.
   - `with_location(source, AssetLocation::*)` — only when a specific
     source needs an override; otherwise the synthesized default from
     the entry's `logical_path` is enough.
4. **Ask the catalog from the loader.** `catalog.path_for(&id)` returns
   `Option<String>` (the Bevy `AssetPath` string) or `None` when the
   profile disabled the asset. Then call `asset_server.load(path)` for
   Bevy-native kinds, or pull bytes via
   `ambition_asset_manager::build_provider_from_resolved` for the SFX
   bank. Gate optional images on
   `catalog.should_attempt_optional_load(&path)`.
5. **Test it.** The `sandbox_assets::tests` module already locks in
   uniqueness + required-policy contracts; add a per-domain test if the
   new asset has interesting per-profile behavior (HTTP-only,
   IPFS-only, etc.).

## Where things live

- `crates/ambition_asset_manager/src/id.rs` — `AssetId`
- `crates/ambition_asset_manager/src/kind.rs` — `AssetKind`
- `crates/ambition_asset_manager/src/location.rs` — `AssetLocation` + `ipfs_gateway_url`
- `crates/ambition_asset_manager/src/profile.rs` — `AssetProfile`, `AssetSourceProfile`
- `crates/ambition_asset_manager/src/policy.rs` — `MissingAssetPolicy`, `CachePolicy`
- `crates/ambition_asset_manager/src/preload.rs` — `PreloadGroup`
- `crates/ambition_asset_manager/src/manifest.rs` — `AssetManifest`, `AssetEntry`, `LocationCandidate`
- `crates/ambition_asset_manager/src/resolver.rs` — `resolve`, `resolve_all`, `ResolvedAsset`
- `crates/ambition_asset_manager/src/bevy_integration.rs` — Bevy plugin/resource/helpers
- `crates/ambition_asset_manager/src/sfx_integration.rs` — `BankProvider` adapter
- `crates/ambition_asset_manager/tests/end_to_end.rs` — cross-module integration tests
- `crates/ambition_sandbox/src/game_assets.rs::sandbox_image_manifest` — live entity-sprite + parallax-layer catalog
- `crates/ambition_sandbox/src/game_assets.rs::should_attempt_optional_image_load` — per-profile load gate (replaces the old `asset_exists`)
