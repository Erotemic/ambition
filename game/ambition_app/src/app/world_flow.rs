//! Host-side room-transition adapters that stay in the app.
//!
//! The transaction itself — readiness, authorization, and the commit — moved to
//! [`ambition_platformer2d::runtime::room_transition`] (2026-07-25) so a demo host can change
//! rooms. What is left here is what only a host with an asset pipeline can do:
//!
//! - the [`room_transition_assets`] submodule (room manifests, handle readiness,
//!   neighbor prefetch) — the engine's asset CONTRIBUTOR;
//! - the [`room_transition_presentation`] submodule (cover-first adaptive UI).

mod room_transition_assets;
mod room_transition_presentation;
pub(crate) use room_transition_assets::{
    build_loaded_room_asset_manifest, demand_room_character_sheets, inspect_room_asset_manifest,
    RoomAssetManifest,
};
pub(crate) use room_transition_presentation::install_room_transition_presentation;
