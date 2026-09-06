//! Asset registries and load-time wiring.
//!
//! - `game_assets` — Bevy `AssetServer` wiring + fallback-friendly load
//!   paths for sprite/audio/font assets.
//! - `platformer_assets` — sandbox-side aggregator for the
//!   [`ambition_asset_manager`] catalog.
//! - `loading` — the startup asset COLLECTION, which is LDtk-shaped and so
//! lives behind `ldtk_runtime`. it was unconditional until,
//!   which made the feature a fiction: `bevy_ecs_ldtk` and `bevy_asset_loader`
//!   are declared optional in the manifest, but this one module named both
//!   unconditionally, so turning `ldtk_runtime` OFF did not yield a smaller
//!   crate — it yielded a crate that would not compile. A boundary the
//!   manifest states and the code does not honour is not a boundary.

pub mod game_assets;
#[cfg(feature = "ldtk_runtime")]
pub mod loading;
pub mod platformer_assets;
