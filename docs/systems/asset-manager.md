
# Asset manager

`crates/ambition_asset_manager/` owns Ambition's logical asset catalog: stable IDs, source/profile selection, preload groups, required-vs-optional policy, and platform-aware resolution. It is the project vocabulary for "which asset is this?" and "where may this platform load it from?"

It does **not** replace Bevy's `AssetServer`, handles, load states, dependencies, hot reload, or `AssetReader` infrastructure. Bevy remains the runtime loader.

`library_source::register_sources` installs the default and `content://` readers
before Bevy starts its asset server. Both headless and windowed foundations use
this function. The headless foundation also installs the CPU image decoder, so
asset readiness does not require a renderer.

## Current policy

- Runtime code should refer to stable Ambition asset IDs rather than hard-coded platform paths when the asset is part of the game catalog.
- The catalog selects a source according to an `AssetProfile`: loose desktop files, installed files, embedded/static assets, Android APK assets, served web assets, or headless/no-asset modes.
- Required assets should fail loudly unless the active profile explicitly tolerates missing assets.
- Optional/presentation assets may fall back, skip display, or use embedded placeholders depending on the caller.
- Generated assets become runtime assets only after an explicit publish/install step.
- A stale or previously accepted asset remains usable in native development.
  Freshness and provenance are advisory. Avoid automatic rebuild and deep
  integrity scans during startup and normal asset reads. A release may use an
  explicit, stronger preflight. See the
  [asset-library lifecycle plan](../planning/engine/asset-library-editor-and-delivery.md).

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

The asset manager answers the catalog question. The caller still decides whether it needs a Bevy handle, an audio/SFX provider built by the audio layer, a filesystem path for a tool, or a skip/fallback path.

An embedded library index does not mean that all its bytes are embedded.
Installed desktop builds load external library files from
`assets/__libraries/<library-id>/<logical-path>` under `BEVY_ASSET_ROOT`.
The catalog's optional-file check uses the same path as the library reader.
An absent optional file still uses its placeholder or a supported quality tier.

Use `python3 scripts/content_status.py` to inspect missing and stale generated
content. This query needs no build or renderer. Its default result is advisory;
only `--require-complete` makes missing or stale content return a failure.

## Adding an asset family

Add stable catalog IDs and manifest entries, choose the relevant profiles and
required/optional policy, and provide an explicit publish step for generated
assets. Document generator commands under `docs/tools/` where appropriate.
Check the actual affected profile; platform-specific loading changes deserve a
focused constrained-platform check, not a broad test campaign.

## Current gaps

- Library freshness does not prove that an installed directory has the bytes
  or that the runtime catalog admits them. Package hashes and runtime path
  checks answer different questions.
- Web/audio/mobile behavior is platform-sensitive; validate the target feature set instead of assuming desktop behavior.
- iOS is a modeled profile, not a tested shipping target.

## Focused verification

The base asset manager is Bevy-free (`default = []`). Its image-stage code is
behind the `bevy` feature. When changing that feature, a focused command is:

```bash
cargo test -p ambition_asset_manager --features bevy
```

For a source reader or packaging change, verify the affected platform/profile
using the relevant build recipe. A passing no-feature crate test does not
exercise Bevy image-stage behavior.
