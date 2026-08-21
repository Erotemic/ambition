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

/// **THE BODY DRIVING A SEAT**: the entity carrying `DrivingParticipant(slot)`.
///
/// ⛔⛔ **there were THREE hand-written copies of this loop and they disagreed
/// about the error case.** `resolve_controlled_subject` counted holders,
/// debug-asserted and logged before taking the first; the camera resolve took
/// the first silently while its own comment called a second holder an error; a
/// third asked only about slot zero. One body drives one slot — a second holder
/// is a seat possession or vacate never retracted — and how loudly that is said
/// must not depend on which caller happened to ask.
///
/// ⚠ **it still returns the first rather than refusing.** A hard failure here
/// would take down a running game over a control bug that a debug build already
/// shouts about, and every caller's fallback (the engine's default gravity, the
/// session's default view) is a survivable answer.
pub fn body_driving_seat(
    drivers: &Query<(Entity, &crate::control::DrivingParticipant)>,
    slot: PlayerSlot,
) -> Option<Entity> {
    let mut holders = drivers
        .iter()
        .filter(|(_, driver)| driver.0 == slot)
        .map(|(entity, _)| entity);
    let first = holders.next();
    let extra = holders.count();
    if extra > 0 {
        debug_assert!(
            false,
            "control invariant violated: {} entities hold DrivingParticipant({slot:?}) \
             (expected at most one); possession/vacate left a stale seat",
            extra + 1,
        );
        bevy::log::error!(
            "control invariant: {} entities hold DrivingParticipant({slot:?}); using the first",
            extra + 1,
        );
    }
    first
}

/// **THE FRAME ONE SEAT'S GESTURES ARE INTERPRETED IN** — the resolved "down"
/// (ADR 0024) of whichever body is driving that seat.
///
/// ⭐ **the generalisation of [`controlled_frame_down`], which asks this for
/// slot zero only.** A double-tap means *down* relative to the body the person
/// is steering, and on a couch that is a different body per seat — so a seat's
/// gesture cannot be resolved against the primary's gravity without giving
/// player two player one's idea of down.
///
/// `fallback` is consulted when nobody holds the seat, which is the load-frame
/// case slot zero has always had.
pub fn seat_frame_down(
    drivers: &Query<(Entity, &crate::control::DrivingParticipant)>,
    slot: PlayerSlot,
    frames: &Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    fallback: Option<Entity>,
) -> ambition_platformer2d_core::Vec2 {
    body_driving_seat(drivers, slot)
        .or(fallback)
        .and_then(|entity| frames.get(entity).ok())
        .map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |frame| {
            frame.down()
        })
}

/// The CONTROLLED body's per-tick resolved "down" (ADR 0024): the frame every
/// slot-0 gesture (fast-fall double-tap, possession Down+Interact, interact
/// suppression) is interpreted in. Resolution order: the `ControlledSubject`
/// (a possessed body reads ITS frame), else the primary player's body, else the
/// engine default. This reads the frame-resolution artifact — it never
/// reconstructs a frame from a gravity field.
pub fn controlled_frame_down(
    controlled: Option<&ambition_platformer2d_shared_tangle::markers::ControlledSubject>,
    primary: Option<Entity>,
    frames: &Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
) -> ambition_platformer2d_core::Vec2 {
    controlled
        .and_then(|subject| subject.0)
        .or(primary)
        .and_then(|entity| frames.get(entity).ok())
        .map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |frame| {
            frame.down()
        })
}
