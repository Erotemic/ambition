//! Query helpers for explicit primary-player, all-player, and controlled-body
//! intent. Use primary-player helpers only for presentation/session semantics;
//! generic simulation should operate on bodies or control authority instead.

use ambition_platformer2d_shared_tangle::markers::{PrimaryPlayerOnly};
use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::prelude::*;

use ambition_characters::control::PlayerSlot;


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

/// Resolve the unique body carrying `DrivingParticipant(slot)`.
///
/// Zero or multiple holders resolve to `None`; query order must never choose an
/// arbitrary body for an ambiguous control identity. Multiple holders are logged
/// as an invariant violation so possession/vacate ownership can be repaired.
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
        // Keep invariant handling identical in debug and release: log and
        // refuse ambiguous control authority.
        bevy::log::error!(
            "control invariant: {} entities hold DrivingParticipant({slot:?}); \
             refusing ambiguous authority, so this seat drives nothing until one \
             of them vacates",
            extra + 1,
        );
        // Ambiguous control identity drives no body this tick.
        return None;
    }
    first
}

/// THIS SEAT'S INPUT FOR THIS TICK, whichever clock the composition runs on.
///
/// TWO CLOCKS, and one source is wrong on one of them. A gesture stage
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
/// reading the raw row on a latch host drops sub-tick taps.
/// `interact_pressed` is OR-accumulated by `ControlFrameLatch`, so a press that
/// opens and closes between two ticks lives in the latch and in no single
/// sample — which is the entire reason the latch exists.
pub fn seat_frame_this_tick(
    latches: Option<&ambition_characters::control::SlotControlLatches>,
    rollback: Option<&ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
    slots: &ambition_characters::control::SlotControls,
    raw: &ambition_characters::control::SeatRawFrames,
    slot: PlayerSlot,
) -> ambition_platformer2d_core::ControlFrame {
    if another_authority_publishes(latches, rollback) {
        slots.get(slot)
    } else {
        raw.get(slot)
    }
}

/// SHAPE THIS SEAT'S FRAME FOR THIS TICK, wherever this composition keeps it.
///
/// READ through the predicate, WRITE to both, and that asymmetry is the
/// point. Which table holds the tick's input depends on the host
/// ([`seat_frame_this_tick`]); which table a shaped value must reach does not.
/// The slot is what the body reads this tick; the raw row is what the next fold
/// carries into the encoded rollback input. Writing the table that is not
/// authoritative is harmless — it is overwritten by the authority that owns it —
/// and writing only one loses a host.
///
/// It answered for every host by being written by whichever authority was live, which is what made
/// it a bus and what made every shaping stage seat zero's.
///
/// ⛔⛔ **MIGRATION INFRASTRUCTURE. DO NOT MAKE THIS THE PERMANENT MODEL, AND DO
/// NOT ADD CLIENTS.** Three systems use it (fast-fall derivation, the app's
/// reset read, the portal adapter). That is the ceiling until the stages below
/// exist, and the reason is written into the name of what it writes:
/// `SeatRawFrames` is documented as the RAW proposal before shaping, and this
/// function shapes into it. The type stopped describing its contents the day
/// this was written, which is a bridge admitting it is one.
///
/// A shaper reaching for this has to accommodate all three publication
/// architectures at once — fixed tick, rollback, frame-step — because
/// `seat_frame_this_tick` decides which representation is authoritative and this
/// writes every representation to make the hosts agree. That works, and it does
/// not scale to ten mechanics.
///
/// The endpoint is staged rather than dual-written:
///
/// ```text
/// physical sample → SeatInputProposal → (latch / rollback agreement)
///                 → ConfirmedSeatInput → deterministic derivation
///                 → EffectiveSlotControls
/// ```
///
/// Each transformation then names ONE stage: device calibration on the proposal
/// side, anything that must be transmitted as participant input at the agreement
/// boundary, and derived gestures like double-tap and fast-fall on the
/// simulation side, from confirmed edges plus rollback-backed history. Build
/// that when the next input change needs the boundary — not as a rewrite for its
/// own sake, and not one client later than that.
pub fn shape_seat_frame(
    latches: Option<&ambition_characters::control::SlotControlLatches>,
    rollback: Option<&ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
    slots: &mut ambition_characters::control::SlotControls,
    raw: &mut ambition_characters::control::SeatRawFrames,
    slot: PlayerSlot,
    edit: impl FnOnce(&mut ambition_platformer2d_core::ControlFrame),
) {
    let mut frame = seat_frame_this_tick(latches, rollback, slots, raw, slot);
    edit(&mut frame);
    slots.set(slot, frame);
    raw.set(slot, frame);
}

/// DOES SOMETHING ELSE ALREADY OWN THIS SEAT'S PUBLISHED FRAME THIS TICK?
pub fn another_authority_publishes(
    latches: Option<&ambition_characters::control::SlotControlLatches>,
    rollback: Option<&ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>,
) -> bool {
    latches.is_some() || rollback.is_some()
}

/// THE FRAME ONE SEAT'S GESTURES ARE INTERPRETED IN — the resolved "down"
/// (ADR 0024) of whichever body is driving that seat.
///
/// the generalisation of [`controlled_frame_down`], which asks this for
/// slot zero only. A double-tap means *down* relative to the body the person
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

#[cfg(test)]
mod tests {
    use super::*;
    

    /// Ask the real query the real question, through a system — a hand-built
    /// iterator would test my arithmetic rather than the function's contract.
    fn seat_holder(app: &mut App, slot: PlayerSlot) -> Option<Entity> {
        #[derive(Resource, Default)]
        struct Answer(Option<Entity>, bool);
        app.init_resource::<Answer>();
        app.world_mut().resource_mut::<Answer>().1 = false;
        let mut system = bevy::ecs::system::IntoSystem::into_system(
            move |drivers: Query<(Entity, &crate::control::DrivingParticipant)>,
                  mut answer: ResMut<Answer>| {
                answer.0 = body_driving_seat(&drivers, slot);
                answer.1 = true;
            },
        );
        system.initialize(app.world_mut());
        // ⛔ THE RUN'S `Result` IS DELIBERATELY DROPPED, and the assertion below
        // is why: a system that failed to run leaves `answer.1` false, which is
        // a better message than an unwrap's. Named rather than silently unused —
        // CI compiles with `-D warnings`.
        let _ran = system.run((), app.world_mut());
        let answer = app.world().resource::<Answer>();
        assert!(answer.1, "the probe system never ran");
        answer.0
    }

    /// ⛔⛔ **two holders of one seat resolve to NOBODY, not to whichever body
    /// the query yielded first.**
    ///
    /// The old behaviour returned `first`, so a possession that forgot to
    /// vacate handed one person's stick to an arbitrary second body — arbitrary
    /// because query order is archetype order, which depends on spawn history.
    /// This pins all three arities at once: the answer for a broken invariant
    /// is the same as the answer for an empty one.
    #[test]
    fn two_bodies_claiming_one_seat_resolve_to_no_body() {
        let slot = PlayerSlot::PRIMARY;
        let mut app = App::new();

        assert_eq!(seat_holder(&mut app, slot), None, "nobody holds it yet");

        let only = app
            .world_mut()
            .spawn(crate::control::DrivingParticipant(slot))
            .id();
        assert_eq!(
            seat_holder(&mut app, slot),
            Some(only),
            "one holder is the whole point of the query"
        );

        // The stale-seat bug: a second body claims the same seat.
        let second = app
            .world_mut()
            .spawn(crate::control::DrivingParticipant(slot))
            .id();
        assert_ne!(only, second);
        assert_eq!(
            seat_holder(&mut app, slot),
            None,
            "an ambiguous seat drove a body — the caller was handed one of two \
             claimants chosen by archetype order"
        );
    }
}
