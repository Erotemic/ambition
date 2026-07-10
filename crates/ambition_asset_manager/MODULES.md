# `ambition_asset_manager` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_asset_manager** — Ambition asset catalog + source/profile policy.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`asset_publish`](src/asset_publish/mod.rs) | Publish/install boundary for generated sprite assets. |
| [`bevy_integration`](src/bevy_integration.rs) | Bevy plugin / resource / helper layer. |
| [`id`](src/id.rs) | [`AssetId`] — stable logical identifier for an asset entry. |
| [`kind`](src/kind.rs) | [`AssetKind`] — coarse Ambition-side classification of an asset. |
| [`location`](src/location.rs) | [`AssetLocation`] — where the bytes for a logical [`crate::AssetId`] live for a given [`crate::profile::AssetProfile`]. |
| [`manifest`](src/manifest.rs) | [`AssetManifest`] — the catalog of [`AssetEntry`] records. |
| [`policy`](src/policy.rs) | Policy enums for handling missing assets and caching. |
| [`preload`](src/preload.rs) | [`PreloadGroup`] — coarse "load-this-set-up-front" tag. |
| [`profile`](src/profile.rs) | [`AssetProfile`] — the active platform/runtime persona that drives which [`crate::location::AssetLocation`] the resolver returns for a given [`crate::AssetId`]. |
| [`resolver`](src/resolver.rs) | Resolve `(AssetId, AssetProfile) -> ResolvedAsset`. |
| [`sandbox_assets`](src/sandbox_assets/mod.rs) | Sandbox-side aggregator for the [`ambition_asset_manager`] catalog. |

_11 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
