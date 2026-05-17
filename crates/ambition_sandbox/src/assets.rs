//! Asset registries and load-time wiring.
//!
//! - `game_assets`     — Bevy `AssetServer` wiring + fallback-friendly load
//!                       paths for sprite/audio/font assets.
//! - `sandbox_assets`  — sandbox-side aggregator for the
//!                       [`ambition_asset_manager`] catalog.
//! - `loading`         — asset-loading foundation shared by both.

pub mod game_assets;
pub mod loading;
pub mod sandbox_assets;
