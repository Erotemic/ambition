//! Host-side room-transition adapters that stay in the app.
//!
//! What is left here is what only a host with an asset pipeline can do:
//!
//! - the [`room_transition_assets`] submodule (room manifests, handle readiness,
//!   neighbor prefetch) — the engine's asset CONTRIBUTOR;
//! - the [`room_transition_presentation`] submodule (cover-first adaptive UI);
//! - the [`parallax_residency`] submodule — which zone backdrops may stay
//!   resident once the player has walked on;
//! - the [`first_room_art`] submodule — the same asset question asked of a
//!   shell route's FIRST room, before it activates.

mod first_room_art;
pub(crate) mod parallax_residency;
pub(crate) mod room_transition_assets;
mod room_transition_presentation;
pub(crate) use room_transition_assets::{
    build_loaded_room_asset_manifest, demand_room_character_sheets, inspect_demanded_characters,
    inspect_room_asset_manifest, realized_character_count, room_character_tokens,
    RoomAssetManifest,
};
pub(crate) use room_transition_presentation::install_room_transition_presentation;
/// public because a SCHEDULE SEAM has to be nameable to be checkable.
/// The set carries the one ordering edge the room-transition cover's
/// correctness rests on, and Bevy compiles system NAMES out — so a test cannot
/// find the system any other way, and `RoomTransitionCoverRoot`'s precedent
/// (matched by debug `Name` from a test, because widening it would put a
/// presentation marker in the public surface) does not apply to sets. A set is
/// a name by design; every other ordering seam in this workspace is public too.
pub use room_transition_presentation::RoomTransitionCoverSet;
