//! Player-query helpers that make singleton vs. multi-player intent
//! explicit at the call site.
//!
//! The game currently spawns exactly one player, but most call sites
//! that reach for `single_mut()` are implicitly relying on that fact.
//! These helpers give contributors obvious APIs to pick between:
//!
//! - **`PrimaryPlayerOnly`** — filter type usable on any `Query`
//!   (immutable or mutable component access) when the system genuinely
//!   wants the camera/HUD/dev-tool target. In Bevy the same filter
//!   works for both read and write queries, so there is no separate
//!   `…Mut` variant.
//! - **`primary_player_entity`** — finds the primary player's `Entity`
//!   from any `Query<Entity, With<PrimaryPlayer>>` without panicking.
//! - **`sort_players_by_slot`** — collects player entities ordered by
//!   `PlayerSlot` so future iteration is deterministic.
//!
//! Use these *only* where the singleton intent matters. The bulk of
//! existing systems still use `single_mut()` and that's fine for now —
//! the goal of this module is to make new singleton assumptions
//! visible, not to rewrite every old one.

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::prelude::*;

use super::components::PlayerSlot;

/// The "primary player only" query filter is body vocabulary now — it lives in
/// [`crate::actor`] (its single definition). Re-exported here for the player
/// module's own consumers; new non-player code should import it from
/// `crate::actor` directly.
pub use crate::actor::PrimaryPlayerOnly;

/// Convenience: resolve the primary player's `Entity`. Returns `None`
/// if no primary player exists yet (e.g. during pre-spawn startup) or
/// if — unexpectedly — more than one entity carries `PrimaryPlayer`.
pub fn primary_player_entity(primary: &Query<Entity, PrimaryPlayerOnly>) -> Option<Entity> {
    primary
        .iter()
        .next()
        .filter(|_| primary.iter().count() == 1)
}

/// Collect every player entity + slot ordered by `PlayerSlot`. Use
/// when a system intentionally iterates over all players (HUD widgets
/// that show every slot's status, debug overlays, etc.). Cheap today
/// because there's exactly one player; the explicit sort keeps the
/// order deterministic once a second player is added.
pub fn sort_players_by_slot<D, F>(
    players: &Query<(Entity, &PlayerSlot, D), F>,
) -> Vec<(Entity, PlayerSlot)>
where
    D: QueryData,
    F: QueryFilter,
{
    let mut out: Vec<(Entity, PlayerSlot)> =
        players.iter().map(|(e, slot, _)| (e, *slot)).collect();
    out.sort_by_key(|(_, slot)| *slot);
    out
}

/// The CONTROLLED body's per-tick resolved "down" (ADR 0024): the frame every
/// slot-0 gesture (fast-fall double-tap, possession Down+Interact, interact
/// suppression) is interpreted in. Resolution order: the `ControlledSubject`
/// (a possessed body reads ITS frame), else the primary player's body, else the
/// engine default. This reads the frame-resolution artifact — it never
/// reconstructs a frame from a gravity field.
pub fn controlled_frame_down(
    controlled: Option<&ambition_platformer_primitives::markers::ControlledSubject>,
    primary: Option<Entity>,
    frames: &Query<&ambition_platformer_primitives::frame_env::ResolvedMotionFrame>,
) -> ambition_engine_core::Vec2 {
    controlled
        .and_then(|subject| subject.0)
        .or(primary)
        .and_then(|entity| frames.get(entity).ok())
        .map_or(ambition_engine_core::DEFAULT_GRAVITY_DIR, |frame| {
            frame.down()
        })
}
