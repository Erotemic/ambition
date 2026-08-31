//! Bevy systems that drive room state from the data types in sibling modules.
//!
//! Syncs active-room metadata + music request (`sync_active_room_metadata`,
//! `sync_room_music_request`) and ticks gate-portal phases
//! (`tick_portal_phases_system`). The portal sprite/ring PRESENTATION
//! systems live render-side (`ambition_render::rendering::
//! gate_portal_visuals`, E4 slices 10+20) and consume the phase registry;
//! pure-data types/phase logic live in `gate_portal`/`metadata`/`room_graph`.

use bevy::prelude::{Entity, MessageWriter, Query, Res, ResMut, Without};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::rooms::{
    tick_gate_portal_phase, ActiveRoomMetadata, GatePortalPhases, GatePortalRegistry,
    LoadingZoneActivation, RoomMusicRequest, RoomSet, RoomSfxId,
};
use ambition_time::WorldTime;

/// The set [`sync_active_room_metadata`] runs in — the active room is current.
///
/// Mode teardown waits for it: a transition into a different mode tears the old
/// mode down on the same frame it becomes stale, which requires the room
/// metadata to already describe the NEW room.
///
/// ONE member. The chained neighbours (`sync_room_music_request`,
/// `tick_portal_phases_system`) are CONSUMERS of the fresh metadata, not part of
/// establishing it, so widening would make teardown wait on music and portals.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActiveRoomMetadataSynced;

/// Reconcile `RoomSet::active_metadata()` into the sibling
/// `ActiveRoomMetadata` component on the same session root, but only when the
/// metadata actually changes. The
/// PartialEq guard means change-detection consumers (e.g. a future
/// room-music selector) only fire when the active room's biome /
/// music_track / ambient / theme really differ — not on every frame.
pub fn sync_active_room_metadata(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>,
    mut active: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<ActiveRoomMetadata>,
) {
    let current = room_set.active_metadata().clone();
    if current != active.0 {
        active.0 = current;
    }
}

/// Push the active room's `music_track` into `RoomMusicRequest` so the
/// audio system knows the room-default track when no encounter
/// override is active. Empty values clear the request, falling back to
/// the music registry's `default_track`.
pub fn sync_room_music_request(
    active: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ActiveRoomMetadata>,
    mut request: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<RoomMusicRequest>,
) {
    let next = active.0.music_track.clone();
    if next != request.desired_track {
        request.desired_track = next;
    }
}

/// Advance every registered portal's phase based on its controlling
/// switch's state + the per-phase timer. Pure state update — sprite
/// visibility + ring rotation are downstream presentation systems.
///
/// The switch's "true / false" state is what tells the portal what
/// it *should* be doing (boot or shutdown); the portal still runs
/// its own one-shot Opening / Closing animations between Off and
/// On, so the traversal check (only `On` allows it) remains stable
/// even when the switch flickers.
///
/// this runs in the SIM schedule — `GgrsSchedule` under the shipped
/// rollback host — so every line below executes on speculative frames and is
/// re-executed on resimulation. The switch it integrates lives in the
/// rollback-registered `AmbitionGameSave`; the integral it produces lives in the
/// rollback-registered [`GatePortalPhases`]. Both halves must rewind together or
/// the portal opens on a different frame for each peer, and
/// `detect_room_transition_system` below turns that into a room transition one
/// peer takes and the other refuses.
pub fn tick_portal_phases_system(
    world_time: Res<WorldTime>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    portals: Res<GatePortalRegistry>,
    mut phases: ResMut<GatePortalPhases>,
) {
    // Scaled dt — pause / hitstop / bullet-time naturally freezes
    // or slows the portal boot/shutdown sequence so the ring spin
    // and one-shot anims stay in sync with everything else.
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    // iteration order over the authored map is arbitrary and that is FINE:
    // each portal's tick reads only its own switch and writes only its own
    // phase, so no portal can observe another's update. Nothing here folds an
    // order-dependent accumulator.
    for (zone_id, config) in &portals.portals {
        let switch_on = save.data().switch(&config.switch_id);
        tick_gate_portal_phase(phases.phase_mut(zone_id), switch_on, dt);
    }
}

/// Detect a loading-zone overlap and RECORD the crossing it describes. The host
/// opens a readiness transaction from that record while the current room remains
/// authoritative; the actual room load (despawn old, spawn new, place the
/// recorded body at its arrival) commits only later, once the transaction
/// authorizes it — and, on a rollback host, once the recording frame is
/// confirmed.
///
/// Attacks may still advance on the detection frame, but replay fixtures confirm
/// player-position determinism because attacks do not push the player.
///
/// Gated by `gameplay_allowed` at the registration site: transitions must not
/// fire while paused or in dialogue. The host coordinator is unconditional and
/// is a no-op when no transition transaction is active.
pub fn detect_room_transition_system(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>,
    sim_state: Res<ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown>,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    // The transition subject is the CONTROLLED body: if the driven body (home
    // avatar or possessed actor) enters an exit/door, THAT body transitions. Future
    // door restrictions gate on body properties (size/shape/locomotion), never on
    // "is this the home avatar". Falls back to the primary player at startup.
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    mut slot_gestures: ResMut<ambition_characters::control::SlotInteractionState>,
    // Use the movement kernel's `SweepSample` for boundary crossings because collision may zero
    // velocity at time of impact.
    // TODO(compat-remove): once every mover publishes `SweepSample`, remove the `vel * dt`
    // fallback for bodies without one.
    bodies: Query<
        (
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ae::SweepSample>,
        ),
        Without<ambition_combat::death_rules::OutOfPlay>,
    >,
    // The triggering body's rollback-stable identity, recorded into the deferred transition so
    // the confirmed commit transports the body that CROSSED the exit — not whatever is
    // controlled later, after a possession change.
    sim_ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    primary_q: Query<Entity, ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    world_time: Res<WorldTime>,
    // Track B: under a rollback host, defer the transition instead of engaging the
    // (not-rollback-registered) multi-tick load machine on a speculative frame.
    boundary: Option<Res<ae::ConfirmedFrameBoundary>>,
    mut pending_lifecycle: ResMut<crate::session::lifecycle_commit::PendingLifecycleCommit>,
) {
    if sim_state.remaining > 0.0 {
        return;
    }
    let Some(subject_entity) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_q.single().ok())
    else {
        return;
    };
    let Ok((kin, sweep)) = bodies.get(subject_entity) else {
        return;
    };
    // CC2 (§3.3): sweep the body's frame path into the zone so a fast body
    // can't tunnel an overlap-fire (`Walk`) loading zone between frames. The
    // discrete standing-in-it case is `delta == 0`, preserved exactly — a body
    // that did not move produces a zero-length sample and the test degrades to
    // the overlap it always was.
    let delta = sweep
        .map(|sample| sample.delta())
        .unwrap_or_else(|| kin.vel * world_time.sim_dt());
    let wants_interact = slot_gestures.primary().buffered();
    let Some(zone) = room_set.transition_for_player(kin.aabb(), delta, wants_interact) else {
        // `warn_once`: a stuck body re-enters this branch every tick, and the
        // situation is a standing one — the first report is the whole message.
        // and it costs nothing on the normal path: it runs only after the
        // swept test has already declined, and only for a body actually
        // overlapping an authored zone.
        //
        // Suppressing that silences the instrument in its own founding scenario.
        //
        //  WARN when the press HAPPENED and nothing moved (unambiguous), DEBUG
        // when it did not (ordinary, and still one log level away). Every fact
        // stays in the message either way. `EdgeExit`/`Walk` need no press, so
        // they are anomalous whenever they are touched without transitioning.
        use ae::AabbExt as _;
        if let Some(touching) = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| kin.aabb().strict_intersects(zone.aabb))
        {
            let ordinary_unpressed = !wants_interact
                && matches!(
                    touching.activation,
                    ambition_platformer2d_world::rooms::LoadingZoneActivation::Door
                );
            if ordinary_unpressed {
                bevy::log::debug_once!(
                    target: "crate::rooms",
                    "the controlled body is touching `{}` (Door) and has not \
                     pressed interact — ordinary; raised to WARN once a press \
                     is buffered and the transition still does not fire.",
                    touching.id,
                );
                return;
            }
            bevy::log::warn_once!(
                target: "crate::rooms",
                "the controlled body is TOUCHING loading zone `{}` ({:?}) and the \
                 transition did not fire. path delta = {:?} (sweep sample {}), \
                 interact buffered = {wants_interact}. A `Door` needs the press; \
                 an `EdgeExit` does not. A zero delta on a body that moved means \
                 the path is being reconstructed from a velocity collision has \
                 already zeroed.",
                touching.id,
                touching.activation,
                delta,
                if sweep.is_some() { "present" } else { "ABSENT" },
            );
        }
        return;
    };
    // Portal check: if this zone is registered as a portal, the
    // portal's own phase must be `On` for traversal to be allowed.
    // The switch only commands the boot/shutdown sequence — the
    // portal itself runs the state machine. Non-portal zones pass
    // through unchanged.
    if portals.is_portal(&zone.zone.id) && !phases.allows_traversal(&zone.zone.id) {
        return;
    }
    let zone_sfx = match zone.zone.activation {
        LoadingZoneActivation::Door => Some(RoomSfxId::new("world.door.open")),
        // Walk-through zones (mid-room portals and side-edge exits)
        // both use the portal-enter sfx — the door-open sound only
        // fits the discrete interact door beat.
        LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk => {
            Some(RoomSfxId::new("world.portal.enter"))
        }
    };
    // Two descriptions of one crossing that disagreed about the body is exactly the fork exists
    // to close, so the refusal below is now universal: a body we cannot name is a crossing we
    // cannot describe, on any host.
    let Ok(subject) = sim_ids.get(subject_entity) else {
        bevy::log::error_once!(
            "transition subject {:?} has no SimId; refusing an ambiguous crossing",
            subject_entity
        );
        return;
    };
    // ONE description, recorded the same way on every host.
    //
    // Two descriptions of one crossing, and only the message opened the readiness transaction — so
    // the SHIPPED game, which composes the rollback host, changed rooms with no cover, no failure
    // reporting and no asset accounting. Now both hosts record the intent and the transaction is
    // its only consumer; they differ only in WHEN it is safe to act on, which is the frame stamped
    // here.
    //
    // the intent names the room by ID, so it needs the target's spec. A
    // failure here leaves the press buffered on purpose (the transition is still
    // wanted; we just cannot describe it yet), so this system re-runs every tick
    // the body stays on the exit — `_once` keeps a stuck exit out of the log.
    let Some(target_spec) = room_set.spec_at(zone.target_room) else {
        bevy::log::error_once!(
            "transition target {:?} has no room spec; leaving input buffered",
            zone.target_room
        );
        return;
    };
    // Consume the gesture only after every invariant required to describe the
    // crossing has been validated.
    slot_gestures.primary_mut().clear();
    // ⚠ A refused slot is the ordinary dedupe: a loading zone re-emits every
    // tick the body overlaps it, so the crossing is asked again next frame and
    // nothing here has mutated anything.
    let _ = pending_lifecycle.record(
        // an eager host has no frames to be ahead of. `0` is not a
        // placeholder: with no `ConfirmedFrameBoundary` there is no speculation,
        // so the intent is confirmed the instant it is recorded, which is what
        // `ConfirmedRoomTransitionIntent` reads it as.
        boundary.map_or(0, |boundary| boundary.current),
        crate::session::lifecycle_commit::LifecycleIntent::Transition(
            crate::session::lifecycle_commit::RoomTransitionIntent {
                subject: subject.clone(),
                target_room: target_spec.id.clone(),
                arrival: zone.arrival,
                edge_exit: matches!(zone.zone.activation, LoadingZoneActivation::EdgeExit),
                // Carried because the commit happens far from the zone that
                // named it.
                zone_sfx: zone_sfx.as_ref().map(|id| id.as_str().to_string()),
            },
        ),
    );
}
