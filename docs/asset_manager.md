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
| `DesktopInstalled`         | InstalledFilesystem → EmbeddedBinary → HttpRemote | ❌         | ✅   | ✅      | ✅    | ✅       | ✅           | Bevy `AssetReader` reads next to the binary; pre-check via `SandboxAssetCatalog::resolve_local_file_path`. |
| `SteamDeckInstalled`       | InstalledFilesystem → EmbeddedBinary → HttpRemote | ❌         | ✅   | ✅      | ✅    | ✅       | ✅           | Same shape as `DesktopInstalled`; kept distinct for future Deck-specific policy. |
| `AndroidBundle`            | AndroidApk → EmbeddedBinary                      | ❌         | ✅¹  | ✅²     | ✅²   | ✅¹      | ✅²          | Bevy Android `AssetReader` resolves through APK assets. ¹ LDtk via `static_map` embedded; SFX bank requires `static_sfx_bank`. ² Loaded if packaged. |
| `IosBundle`                | IosBundle → EmbeddedBinary                       | ❌         | ⚠️   | ⚠️      | ⚠️    | ⚠️       | ⚠️           | Profile modeled but no iOS build target yet — every loader honors the profile, packaging story is TBD. |
| `WebHttp`                  | HttpRemote → EmbeddedBinary                      | ❌³        | ⚠️   | ❌      | ❌    | ❌       | ❌           | Catalog produces `https://...` URLs when explicit candidates exist; today's sandbox authors none, so optional assets `try_path_for_load` returns `None`. The `http`/`https` `AssetSource` registration is the missing wiring (slice 18). ³ HTTP polling / ETag reload future. |
| `WebStatic`                | EmbeddedBinary → HttpRemote                      | ❌         | ✅   | ✅²    | ✅²    | ⚠️       | ❌           | wasm build under `--features web`. LDtk, UI fonts, and a core sprite subset (player/robot/goblin/sandbag + chests/pickups/doors/projectile/tiles) are embedded under `static_core_assets`. Parallax, optional NPCs, and music still fall back. SFX via `static_sfx_bank` when enabled. ² Embedded via `AmbitionAssetSourcePlugin`. |
| `WebServedAssets`          | InstalledFilesystem → EmbeddedBinary             | ❌         | ✅⁴  | ✅⁵   | ✅⁵   | ✅⁵     | ✅⁵         | wasm build under `--features web_served_assets`. The "same game in the browser." `./build_for_web.sh --served` symlinks `crates/ambition_sandbox/assets/` into `web/assets/`; Bevy's wasm `HttpAssetReader` fetches `/assets/<path>` for every catalog entry that doesn't author an explicit Embedded candidate. ⁴ LDtk still uses the authored Embedded candidate (kept in-binary via `static_map`). ⁵ Path resolves; playback / display depends on the asset being served from `/assets/`. Audio playback also requires the `audio` feature (not in `visible_web_served` today). |
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

The LDtk hot-reload watcher
(`crates/ambition_sandbox/src/ldtk_world/hot_reload.rs::LdtkHotReloadState::from_catalog`)
asks `SandboxAssetCatalog::hot_reload_local_path(world.sandbox_ldtk)`
at startup. If the catalog hands back a `PathBuf` (DesktopDevLoose
profile with a real on-disk file), the watcher polls it. Every other
profile gets `None` and the watcher idles. Hot-reload re-parses go
through `LdtkProject::load_from_disk_at(&path, &catalog)` so secondary
worlds (`world.intro_ldtk`) re-merge against the same catalog.

## Bevy integration

The sandbox installs two paired pieces during `init_sandbox_resources` /
`run_visible` / `run_web`:

- **`SandboxAssetCatalog`** Bevy `Resource` — the catalog itself, plus
  the active `AssetProfile`. Built once via
  `crate::sandbox_assets::build_sandbox_catalog(&config, &audio)`.
  Inserted before any loader runs.
- **`AmbitionAssetSourcePlugin::for_profile(profile)`** — installed
  AFTER `DefaultPlugins` so `EmbeddedAssetRegistry` (set up by
  `AssetPlugin`) is already present. Registers the embedded asset
  bytes the catalog's authored `EmbeddedBinary` candidates point at.

The canonical loader API is **`SandboxAssetCatalog::try_path_for_load(&id)
-> Option<String>`**. Returns the Bevy `AssetPath` string when the
loader should hand it to `AssetServer::load`; returns `None` when the
loader should fall back (colored rectangle, silent SFX, Bevy default
font). Every visible loader in the sandbox is a one-liner:

```rust
if let Some(path) = catalog.try_path_for_load(&id) {
    handles.insert(key, asset_server.load(path));
}
```

Byte-only consumers (the SFX bank loader, the LDtk hot-reload watcher)
go through `SandboxAssetCatalog::resolve_local_file_path(rel) ->
Option<PathBuf>` for the host on-disk path — the only candidate-roots
walker in the sandbox.

`ambition_asset_manager::AmbitionAssetManagerPlugin` is the *generic*
Bevy plugin shipped by the asset-manager crate; the sandbox doesn't
use it directly because `SandboxAssetCatalog` owns extra per-sandbox
policy (the local-file probe + per-profile load gate). Third-party
crates that just want the resolver layer can install the generic
plugin instead.

### Where each profile pulls its bytes from

| Profile | Embedded source | Filesystem source | HTTP source |
| ------- | --------------- | ----------------- | ----------- |
| `DesktopDevLoose` | `AmbitionAssetSourcePlugin` (only LDtk under `static_map`) | Bevy default file `AssetReader` + `bevy/file_watcher` for hot reload | not registered |
| `DesktopInstalled` / `SteamDeckInstalled` | `AmbitionAssetSourcePlugin` | Bevy default file `AssetReader` (no watcher) | not registered |
| `AndroidBundle` / `IosBundle` | `AmbitionAssetSourcePlugin` | Bevy platform `AssetReader` (APK / .app) | not registered |
| `WebStatic` / `BundledStatic` | `AmbitionAssetSourcePlugin` under `static_map` + `static_core_assets` | n/a (no host filesystem) | not registered |
| `WebHttp` | `AmbitionAssetSourcePlugin` (fallback) | n/a | future: `add_http_asset_source(app)` (currently a documented stub) |
| `NoAssets` / `Headless` | `AmbitionAssetSourcePlugin` still installs (cheap) | n/a | not registered |

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
| 8 | Secondary LDtk worlds (`world.intro_ldtk`) catalog-driven | **DONE** (2026-05-17) |
| 9 | `AmbitionAssetSourcePlugin` + embedded LDtk source registration | **DONE** (2026-05-17) |
| 10 | `ResolvedAsset::authored_candidate` + `try_path_for_load` API | **DONE** (2026-05-17) |
| 11 | `AMBITION_SFX_BANK_PATH` as authored catalog override | **DONE** (2026-05-17) |
| 12 | Guardrail tests (no asset_exists / no BEVY_ASSET_ROOT probes / hot-reload only on DesktopDevLoose) | **DONE** (2026-05-17) |
| 13 | Embedded core fonts + primary character sheets + core entity sprites for `WebStatic`/`BundledStatic` (`static_core_assets`) | **DONE** (2026-05-17) |
| 14 | Intro plugin sprites flow through catalog ids (`sprite.character.intro_*` namespace) | **DONE** (2026-05-17) |
| 15 | Delete `should_attempt_optional_load(&str)` | **DONE** (2026-05-17) |
| 16 | `WebServedAssets` profile + `web_served_assets` Cargo feature + `./build_for_web.sh --served`. Two-pass resolver so authored Embedded candidates beat synthesized defaults. | **DONE** (2026-05-17) |
| 17 | FPS / frame-time overlay (`crate::fps_overlay`, default-on for wasm, F3 toggles) | **DONE** (2026-05-17) |
| 18 | `audio` for `WebServedAssets` (compile + browser autoplay handshake) | TODO |
| 19 | Out-of-set sprites (breakable variants, soft/hard blink walls, lock-wall tile) embedded under `static_core_assets` — needed only if pure WebStatic is the target | TODO |
| 20 | Music cue layers (file-backed cues under `MusicCueCatalog`) | TODO |

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
- **Loader rewrite**: every loader (entity sprites, parallax layers,
  character sheets, boss sheets, UI fonts, SFX bank, music tracks) is
  a single `if let Some(path) = catalog.try_path_for_load(&id) { ... }`
  call. Bevy `AssetServer` does the actual load. The per-profile load
  gate ([`SandboxAssetCatalog::should_attempt_resolved_load`]) decides:
  - Pre-checks the host filesystem for `DesktopDevLoose` /
    `DesktopInstalled` / `SteamDeckInstalled` via
    `resolve_local_file_path` (the only candidate-roots walker).
  - Trusts the packager for `AndroidBundle` / `IosBundle`.
  - Attempts the load on `WebStatic` / `BundledStatic` only when the
    entry carries an **authored** `EmbeddedBinary` candidate
    (`ResolvedAsset::authored_candidate == true`). Speculative
    synthesized `embedded://` URLs are skipped so the rendering layer
    keeps the colored-rectangle fallback.
  - Attempts on `WebHttp` / `IpfsGatewayPlaceholder` only when an
    authored `HttpRemote` / `IpfsGateway` candidate is present.
- **`asset_exists` removed** from `game_assets.rs`, `boss_sprites.rs`,
  `ui_fonts.rs`, and `character_sprites/assets.rs`. The only host-
  filesystem probe in the sandbox is `desktop_candidate_roots` in
  `sandbox_assets.rs`, exposed via
  `SandboxAssetCatalog::resolve_local_file_path`. No
  `#[cfg(target_os = "android")]` branches in any loader.

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
  music tracks) all go through `SandboxAssetCatalog::try_path_for_load(...)`
  / `::resolve(...)`.
- The **only** host-filesystem probe in the sandbox lives in
  `crate::sandbox_assets`, exposed publicly as
  `SandboxAssetCatalog::resolve_local_file_path(rel) -> Option<PathBuf>`.
  Every other loader / byte adapter calls through it.
- LDtk hot reload preserved: `LdtkHotReloadState::from_catalog(...)`
  asks the catalog for a `LocalPath` via
  `SandboxAssetCatalog::hot_reload_local_path`; the watcher polls only
  that. On `WebStatic` / `AndroidBundle` / `BundledStatic` the
  `watch_path` is `None` and the watcher idles.
- `LdtkProject::load_default(&catalog)` is the production entry point;
  `LdtkProject::load_default_for_dev()` is the test/headless shortcut
  that builds a default desktop catalog internally.

### Slice 4 — platform asset story (2026-05-17)

Closes the remaining transition shims; makes the catalog drive
runtime asset source registration instead of merely describing assets.

- **Centralized disk probing.** The only host-filesystem candidate
  walker in the sandbox is `desktop_candidate_roots` in
  `crates/ambition_sandbox/src/sandbox_assets.rs`, exposed publicly as
  `SandboxAssetCatalog::resolve_local_file_path(rel) -> Option<PathBuf>`.
  Every other consumer (SFX bank loader, LDtk hot-reload watcher, font
  loader, sprite loader) calls through it. `setup.rs::resolve_to_disk_path`
  is **deleted**.
- **`SandboxAssetCatalog::try_path_for_load(id)`** combines resolution
  + per-profile load gate into one call. Returns `Some(path)` when the
  loader should hand the string to `AssetServer::load`; `None` when the
  loader should fall back. Every visible loader (sprites, fonts, boss
  sheets, parallax, character sheets) is now a single
  `if let Some(path) = catalog.try_path_for_load(&id) { ... }`.
- **`ResolvedAsset::authored_candidate: bool`** distinguishes "authored
  explicit candidate" from "resolver-synthesized speculative default."
  WebStatic/BundledStatic gates the load on the authored flag — a
  speculative `embedded://...` URL with no real bytes never triggers
  an `AssetServer::load`.
- **`AmbitionAssetSourcePlugin`** is the seam where Bevy asset sources
  get registered. Behind `static_map` it inserts the LDtk world JSON
  bytes into `EmbeddedAssetRegistry` under
  `embedded://ambition_sandbox/ambition/worlds/{sandbox,intro}.ldtk` —
  matching the explicit `EmbeddedBinary` candidates the catalog
  authors on the corresponding entries. WebStatic now **actually loads
  the LDtk world** instead of synthesizing a URL into the void.
- **`AMBITION_SFX_BANK_PATH` env var** is now an authored
  `LooseFilesystem` `LocationCandidate` on the SFX bank entry — visible
  catalog policy instead of an invisible side-path in `setup.rs`.
- **Secondary LDtk worlds (`world.intro_ldtk`)** are catalog-driven:
  `merge_secondary_worlds_via_catalog` walks `secondary_world_ids()`
  and reads the resolved `LocalPath` for each entry. Embedded
  candidates flow through the same `AmbitionAssetSourcePlugin`
  registration.

### Transition shims still present

> **Empty as of 2026-05-17.**

Every transition shim that was visible in slice 3 has been retired:

- `setup.rs::resolve_to_disk_path` — **deleted**.
- `setup.rs::try_load_sfx_bank` legacy candidate-roots walk — **deleted**.
- `ldtk_world::loading::SECONDARY_WORLD_FILES` const — **deleted**;
  catalog-driven `secondary_world_ids()` replaces it.
- `ambition_asset_manager_transition` markers in
  `sandbox_assets::desktop_loose_file_exists` — **deleted** (the
  function itself is gone; `desktop_candidate_roots` survives as the
  single owner).
- `SandboxAssetCatalog::should_attempt_optional_load(path: &str)` —
  **deleted** (2026-05-17); intro plugin sprites flow through
  catalog ids + `try_path_for_load` like everything else.

### Remaining work (slices 13+)

13. **Sprite / font embedding for `WebStatic`** — author per-asset
    `EmbeddedBinary` candidates + `EmbeddedAssetRegistry::insert_asset`
    calls under `AmbitionAssetSourcePlugin`. Likely behind a
    `static_sprites` feature so the desktop dev build isn't paying
    the embed cost. Or adopt `bevy_embedded_assets` with a filter.
14. **HTTP/HTTPS `AssetSource`** for `WebHttp`. The catalog already
    emits `https://...` URLs from authored `HttpRemote` candidates;
    `add_http_asset_source` in `sandbox_assets.rs` is the wiring stub.
15. **Music cue layers** — `MusicCueCatalog` cues
    (`crates/ambition_sandbox/src/music/director/loader.rs`) still build
    paths as `{cue.asset_root}/{source.path}`. Per-section/per-layer
    catalog ids would unify with the music-track path.
16. **Intro plugin sprite catalog entries** — let
    `load_intro_npc_sprites_system` author its rows into the catalog
    so the legacy `should_attempt_optional_load(&str)` API can go.

## IPFS posture

First slice: gateway URL construction only. No `libp2p`, no
content-routing, no pinning. The
`AssetLocation::IpfsGateway { gateway, cid, path }` variant builds a
canonical HTTPS URL via `ipfs_gateway_url`; consumers fetch through
Bevy's `https` `AssetSource` like any other HTTP asset. A future slice
can grow this into native IPFS support behind a separate feature.

## How platform source registration works

`SandboxAssetCatalog` decides *what* an `AssetId` resolves to;
`AmbitionAssetSourcePlugin` makes those URLs actually resolvable at
runtime. Together they form the catalog-as-policy + Bevy-as-runtime
split.

```text
SandboxAssetCatalog            AmbitionAssetSourcePlugin
  ───────────────              ───────────────────────────
  id → AssetPath               registers concrete AssetReaders /
  "embedded://X"               EmbeddedAssetRegistry entries that
       │                       answer "embedded://X" with real bytes
       ▼
   Bevy AssetServer ───────────► Bevy AssetReader (Embedded / File / Http)
```

### Install order

```text
DefaultPlugins              (creates AssetPlugin → EmbeddedAssetRegistry)
SandboxSimulationPlugin
SandboxLdtkPlugin
SandboxPresentationPlugin
AmbitionAssetSourcePlugin   ◄── must run AFTER DefaultPlugins
```

`AmbitionAssetSourcePlugin::for_profile(profile)` takes the active
profile so future profile-conditional registration ("only register
the `http` source on `WebHttp`") has a single switch site.

### Adding a new embedded asset

1. Add a `with_location(EmbeddedBinary, AssetLocation::embedded("ambition_sandbox/your/path.foo"))`
   candidate to the catalog entry in
   `crate::sandbox_assets::extend_with_*`.
2. Add a matching
   `EmbeddedAssetRegistry::insert_asset(PathBuf::new(),
   Path::new("ambition_sandbox/your/path.foo"),
   include_bytes!("../assets/your/path.foo") as &[u8])`
   call in `register_embedded_assets` (or a new gated helper, e.g.
   behind a `static_sprites` feature).
3. The catalog's `try_path_for_load` immediately flips to true under
   WebStatic/BundledStatic for that asset, because the candidate is
   now `authored_candidate = true`.

### Adding an HTTP-served asset (future)

1. Author a `LocationCandidate { source: HttpRemote, location: HttpUrl("https://...") }`
   on the entry.
2. Wire Bevy's `http`/`https` features in the sandbox `Cargo.toml`
   and call `add_http_asset_source(app)` from
   `AmbitionAssetSourcePlugin::build`. (Currently a documented stub
   that's safe to call early.)
3. WebHttp's `try_path_for_load` flips to true.

## WebStatic packaging status

The `web` Cargo feature composes `visible_web` which now enables both
`static_map` (LDtk world JSON) and `static_core_assets` (UI fonts +
primary character sheets + core entity sprites). The wasm build boots,
accepts keyboard input, renders the LDtk world with real tiles, draws
the protagonist + goblins + sandbags with their authored
spritesheets, paints chests / pickups / doors / projectiles / tiles
with real art, and uses the bundled UI fonts for HUD + dialog text.

| Asset class | WebStatic status | Notes |
| ----------- | ---------------- | ----- |
| LDtk worlds (`world.sandbox_ldtk`, `world.intro_ldtk`) | ✅ embedded | `static_map` feature; `AmbitionAssetSourcePlugin` registers under `embedded://ambition_sandbox/ambition/worlds/...`. |
| sandbox RON (`data.sandbox`) | ✅ via `SandboxDataSpec::load_embedded` (always-on `include_str!`) | The bytes are `include_str!`'d directly into the binary; the Bevy `AssetServer` handle is informational (hot reload), not load-critical. |
| UI fonts (`font.dialog_regular`, `font.dialog_semibold`, `font.debug_mono`) | ✅ embedded under `static_core_assets` | `embedded://ambition_sandbox/fonts/bundled/...`. Legacy `font.*.legacy` fallbacks resolve to `Disabled` on WebStatic. |
| Primary character sheets (`player`, `robot`, `goblin`, `sandbag`) | ✅ embedded under `static_core_assets` | `embedded://ambition_sandbox/sprites/<name>_spritesheet.png`. |
| Core entity sprites (chests, pickups, door / edge exit, projectile, solid / one-way / hazard tiles, boss core) | ✅ embedded under `static_core_assets` | `embedded://ambition_sandbox/sprites/entities/...`. |
| Out-of-set entity sprites (breakable variants, soft/hard blink walls, lock-wall tile, pogo orb, rebound pad, moving platform, NPC terminal, hazard spikes, sandbag dummy, solid block) | ❌ placeholder rectangles | No authored `EmbeddedBinary` candidate. Slice 16 work. |
| Parallax layers | ❌ placeholder | Slice 17 work. |
| Optional NPC + intro spritesheets | ❌ placeholder | The `sprite.character.npc_*` and `sprite.character.intro_*` entries have catalog ids but no embedded candidates. Adding them is a per-asset opt-in. |
| Boss spritesheets (`gradient_sentinel`, `mockingbird`) | ❌ placeholder | `sprite.boss.*` entries exist; embedding TBD. |
| SFX bank (`audio.sfx_bank`) | ⚠️ `static_sfx_bank` feature | When enabled, `try_load_static_sfx_bank` uses `include_bytes!`. Otherwise the wasm build falls back to procedural fundsp SFX. |
| Music tracks | ❌ procedural fallback in `AudioLibrary::new` | No `EmbeddedBinary` candidates; the music director silently falls back to `render_lofi_theme` synths. |

### Adding more embedded assets

Three-line change per asset (see "How platform source registration works"):
1. add a `const FOO_URL: &str = "ambition_sandbox/..."` to `embedded_core` + append to `ALL_URLS`;
2. add `with_embedded_core_candidate(entry, FOO_URL)` on the catalog entry;
3. add `EmbeddedAssetRegistry::insert_asset(..., include_bytes!("../assets/..."))` in `register_embedded_core_assets`.

The `embedded_core_urls_have_authored_catalog_candidates` test fails if any of those three pieces is missing.

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
4. **Ask the catalog from the loader.** Call
   `catalog.try_path_for_load(&id) -> Option<String>` from the loader.
   `Some(path)` means hand it to `asset_server.load(path)`. `None`
   means the profile disabled the asset OR the per-profile load gate
   says skip (out-of-set on WebStatic, missing on DesktopDevLoose, etc.) —
   fall back to placeholders. For non-Bevy byte adapters (the SFX bank
   loader), pull the resolved local path via
   `catalog.resolve_local_file_path(rel)` and feed it to
   `ambition_asset_manager::build_provider_from_resolved`.
5. **If the asset is embedded under `static_core_assets`**, add an
   `EmbeddedBinary` `LocationCandidate` to the entry AND an
   `EmbeddedAssetRegistry::insert_asset(...)` call in
   `crate::sandbox_assets::register_embedded_core_assets`. The
   declarative `embed_core_assets!` table in `sandbox_assets.rs`
   wraps both — add one row to the table and both pieces fall into
   place.
6. **Test it.** The `sandbox_assets::tests` module already locks in
   uniqueness + required-policy + embedded-pairing contracts; add a
   per-domain test if the new asset has interesting per-profile
   behavior (HTTP-only, IPFS-only, etc.).

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
- `crates/ambition_sandbox/src/sandbox_assets.rs::SandboxAssetCatalog` — the single Bevy `Resource` every loader queries
- `crates/ambition_sandbox/src/sandbox_assets.rs::AmbitionAssetSourcePlugin` — registers embedded asset bytes with Bevy
- `crates/ambition_sandbox/src/sandbox_assets.rs::desktop_candidate_roots` — the only host-filesystem candidate walker in the sandbox
