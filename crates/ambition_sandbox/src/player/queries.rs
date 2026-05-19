//! Player-query helpers that make singleton vs. multi-player intent
//! explicit at the call site.
//!
//! The game currently spawns exactly one player, but most call sites
//! that reach for `single_mut()` are implicitly relying on that fact.
//! These helpers give contributors obvious APIs to pick between:
//!
//! - **`PrimaryPlayerOnly` / `PrimaryPlayerOnlyMut`** — filtered query
//!   types you can use as a `SystemParam` field when you genuinely
//!   want the camera/HUD/dev-tool target.
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

use super::components::{PlayerEntity, PlayerSlot, PrimaryPlayer};

/// Filter for "the primary player only" queries. Use this as the
/// filter parameter on a Bevy `Query` when the system genuinely wants
/// the camera/HUD/dev-tool target rather than any player.
///
/// Example:
/// ```ignore
/// fn camera_follow(
///     primary: Query<&PlayerBody, PrimaryPlayerOnly>,
/// ) { … }
/// ```
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);

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
pub fn sort_players_by_slot<'w, D, F>(
    players: &'w Query<(Entity, &PlayerSlot, D), F>,
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
