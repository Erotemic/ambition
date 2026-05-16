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

### Profile contract

| Profile                    | Preferred sources                                | Hot reload | Notes |
| -------------------------- | ------------------------------------------------ | ---------- | ----- |
| `DesktopDevLoose`          | LooseFilesystem → EmbeddedBinary → HttpRemote    | ✅         | `cargo run` from the workspace; LDtk file watcher armed here. |
| `DesktopInstalled`         | InstalledFilesystem → EmbeddedBinary → HttpRemote | ❌         | Asset tree next to the binary, no `CARGO_MANIFEST_DIR`. |
| `SteamDeckInstalled`       | InstalledFilesystem → EmbeddedBinary → HttpRemote | ❌         | Same shape as `DesktopInstalled`; kept distinct for future Deck-specific policy. |
| `AndroidBundle`            | AndroidApk → EmbeddedBinary                      | ❌         | Bevy Android `AssetReader` resolves through APK assets. |
| `IosBundle`                | IosBundle → EmbeddedBinary                       | ❌         | No iOS build yet; here so the schema is forward-compatible. |
| `WebHttp`                  | HttpRemote → EmbeddedBinary                      | ❌¹        | Bevy `http` / `https` `AssetSource` features. |
| `WebStatic`                | EmbeddedBinary → HttpRemote                      | ❌         | Today's wasm first-pass build; LDtk via `static_map`. |
| `BundledStatic`            | EmbeddedBinary                                   | ❌         | Single-binary cross-platform demo build. |
| `NoAssets`                 | (none)                                           | ❌         | `--no-assets`; everything resolves to `Disabled`. |
| `Headless`                 | (none)                                           | ❌         | Same as `NoAssets`; tolerates missing required assets. |
| `IpfsGatewayPlaceholder`   | IpfsGateway → HttpRemote → EmbeddedBinary        | ❌         | Builds `https://<gateway>/ipfs/<cid>/<path>` URLs; no native IPFS dependency. |

¹ HTTP polling / ETag-based reload is a future addition.

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

The first slice does not migrate sandbox loaders. The intent is:

1. **Bootstrap entries** (LDtk, default font) — author manifest entries
   with `MissingAssetPolicy::Error`, switch the sandbox loader to ask
   the catalog for paths.
2. **Sprite folder** — every `EntitySprite` already has a stable
   `AssetId` via `entity_sprite_asset_id(...)` (see
   `crates/ambition_sandbox/src/game_assets.rs`). The
   `demo_asset_catalog` function shows the resolver producing the
   exact paths the live loader synthesizes. Next: author the full
   sprite manifest and have `load_entity_sprites` consult the catalog.
3. **SFX bank** — replace the current `static_sfx_bank` feature wiring
   with a catalog entry per-platform; the web build's static-bytes
   path resolves through `build_provider_from_resolved`.
4. **LDtk hot reload** — the watcher consults
   `ResolvedAsset::supports_hot_reload()` for the path to watch instead
   of hard-coded `CARGO_MANIFEST_DIR` walks.

## IPFS posture

First slice: gateway URL construction only. No `libp2p`, no
content-routing, no pinning. The
`AssetLocation::IpfsGateway { gateway, cid, path }` variant builds a
canonical HTTPS URL via `ipfs_gateway_url`; consumers fetch through
Bevy's `https` `AssetSource` like any other HTTP asset. A future slice
can grow this into native IPFS support behind a separate feature.

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
- `crates/ambition_sandbox/src/game_assets.rs::demo_asset_catalog` — first demonstration wiring
