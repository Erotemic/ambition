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

/// **THIS SEAT'S INPUT FOR THIS TICK, whichever clock the composition runs on.**
///
/// ⛔⛔ **TWO CLOCKS, and one source is wrong on one of them.** A gesture stage
/// runs inside `InputSet::Route`, and at that moment:
///
/// ```text
/// latch host      SlotControls already holds the DRAINED latch —
///                 publish_latched_slot_controls ran .before(Route) — so it is
///                 this tick's input, edges from every sub-tick sample included
/// latchless host  SlotControls is LAST frame's; the raw row is the sample being
///                 assembled now, and the publish that copies it runs after Route
/// ```
///
/// ⚠ **reading the raw row on a latch host drops sub-tick taps.**
/// `interact_pressed` is OR-accumulated by `ControlFrameLatch`, so a press that
/// opens and closes between two ticks lives in the latch and in no single
/// sample — which is the entire reason the latch exists.
///
/// ⭐ **the global `ControlFrame` used to answer this for both hosts**, because
/// it was written by whichever path was live: the drained latch on one, the raw
/// device write on the other. That dual meaning is what made it a bus, and is
/// why removing it (D175) needs the question asked explicitly instead.
pub fn seat_frame_this_tick(
    latches: Option<&ambition_characters::brain::SlotControlLatches>,
    rollback: Option<&ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
    slots: &ambition_characters::brain::SlotControls,
    raw: &ambition_characters::brain::SeatRawFrames,
    slot: PlayerSlot,
) -> ambition_platformer2d_core::ControlFrame {
    if another_authority_publishes(latches, rollback) {
        slots.get(slot)
    } else {
        raw.get(slot)
    }
}

/// **SHAPE THIS SEAT'S FRAME FOR THIS TICK**, wherever this composition keeps it.
///
/// ⭐ **READ through the predicate, WRITE to both, and that asymmetry is the
/// point.** Which table holds the tick's input depends on the host
/// ([`seat_frame_this_tick`]); which table a shaped value must reach does not.
/// The slot is what the body reads this tick; the raw row is what the next fold
/// carries into the encoded rollback input. Writing the table that is not
/// authoritative is harmless — it is overwritten by the authority that owns it —
/// and writing only one loses a host.
///
/// ⛔ this is the seam the global `ControlFrame` used to be. It answered for
/// every host by being written by whichever authority was live, which is what
/// made it a bus and what made every shaping stage seat zero's.
pub fn shape_seat_frame(
    latches: Option<&ambition_characters::brain::SlotControlLatches>,
    rollback: Option<&ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
    slots: &mut ambition_characters::brain::SlotControls,
    raw: &mut ambition_characters::brain::SeatRawFrames,
    slot: PlayerSlot,
    edit: impl FnOnce(&mut ambition_platformer2d_core::ControlFrame),
) {
    let mut frame = seat_frame_this_tick(latches, rollback, slots, raw, slot);
    edit(&mut frame);
    slots.set(slot, frame);
    raw.set(slot, frame);
}

/// **DOES SOMETHING ELSE ALREADY OWN THIS SEAT'S PUBLISHED FRAME THIS TICK?**
///
/// ⛔⛔ **THREE hosts, and asking about only one of them cost 20 app_it tests.**
/// The first version of this predicate asked `latches.is_some()`, which is true
/// on a fixed-tick host and false on BOTH of the others — so under a rollback
/// host the gesture stage read a neutral raw row and wrote it back over the
/// input GGRS had just confirmed, and every rollback fixture went still.
///
/// ```text
/// fixed-tick  a latch exists; publish_latched_slot_controls drained it before Route
/// rollback    the SESSION published, from the input GGRS confirmed
/// frame-step  neither — the raw row IS this tick's input, being assembled now
/// ```
///
/// ⚠ **the rollback marker is a PRESENCE question, not a value one.** Its own
/// doc: *"ordinary render-frame and fixed-tick hosts leave this resource
/// absent"* — `replaying_history` says whether THIS pass is a replay, which is a
/// different question and the wrong one here.
pub fn another_authority_publishes(
    latches: Option<&ambition_characters::brain::SlotControlLatches>,
    rollback: Option<&ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
) -> bool {
    latches.is_some() || rollback.is_some()
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
