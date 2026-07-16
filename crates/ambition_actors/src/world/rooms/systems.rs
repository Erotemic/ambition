//! Bevy systems that drive room state from the data types in sibling modules.
//!
//! Syncs active-room metadata + music request (`sync_active_room_metadata`,
//! `sync_room_music_request`) and ticks gate-portal phases
//! (`tick_portal_phases_system`). The portal sprite/ring PRESENTATION
//! systems live render-side (`ambition_render::rendering::
//! gate_portal_visuals`, E4 slices 10+20) and consume the phase registry;
//! pure-data types/phase logic live in `gate_portal`/`metadata`/`room_graph`.

use bevy::prelude::{Entity, MessageWriter, Query, Res, ResMut};

use super::{
    tick_gate_portal_phase, ActiveRoomMetadata, GatePortalRegistry, LoadingZoneActivation,
    RoomMusicRequest, RoomSet, RoomSfxId, RoomTransitionRequested,
};
use ambition_engine_core as ae;
use ambition_time::WorldTime;

/// Reconcile `RoomSet::active_metadata()` into the sibling
/// `ActiveRoomMetadata` component on the same session root, but only when the
/// metadata actually changes. The
/// PartialEq guard means change-detection consumers (e.g. a future
/// room-music selector) only fire when the active room's biome /
/// music_track / ambient / theme really differ — not on every frame.
pub fn sync_active_room_metadata(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<RoomSet>,
    mut active: ambition_platformer_primitives::lifecycle::SessionWorldMut<ActiveRoomMetadata>,
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
    active: ambition_platformer_primitives::lifecycle::SessionWorldRef<ActiveRoomMetadata>,
    mut request: ambition_platformer_primitives::lifecycle::SessionWorldMut<RoomMusicRequest>,
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
pub fn tick_portal_phases_system(
    world_time: Res<WorldTime>,
    save: Res<ambition_persistence::save::SandboxSave>,
    mut portals: ResMut<GatePortalRegistry>,
) {
    // Scaled dt — pause / hitstop / bullet-time naturally freezes
    // or slows the portal boot/shutdown sequence so the ring spin
    // and one-shot anims stay in sync with everything else.
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for config in portals.portals.values_mut() {
        let switch_on = save.data().switch(&config.switch_id);
        tick_gate_portal_phase(&mut config.phase, switch_on, dt);
    }
}

/// Detect a loading-zone overlap and emit a [`RoomTransitionRequested`]
/// message. The actual room load (despawn old, spawn new, reset player
/// to spawn point) happens in the host's `apply_room_transition_system`,
/// which runs immediately after this system in the `CoreSimulation` chain.
///
/// Ordering is player tick → detect transition → apply transition. Attacks may
/// still advance on a transition frame, but replay fixtures confirm player-position
/// determinism because attacks do not push the player.
///
/// Gated by `gameplay_allowed` at the registration site: transitions must not
/// fire while paused or in dialogue. The apply system itself is unconditional
/// because it reads its own message queue and is a no-op when empty.
pub fn detect_room_transition_system(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<RoomSet>,
    sim_state: Res<crate::SandboxSimState>,
    portals: Res<GatePortalRegistry>,
    mut transition_writer: MessageWriter<RoomTransitionRequested>,
    // The transition subject is the CONTROLLED body: if the driven body (home
    // avatar or possessed actor) enters an exit/door, THAT body transitions. Future
    // door restrictions gate on body properties (size/shape/locomotion), never on
    // "is this the home avatar". Falls back to the primary player at startup.
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    mut slot_gestures: ResMut<crate::control::SlotInteractionState>,
    bodies: Query<&crate::actor::BodyKinematics>,
    primary_q: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    world_time: Res<WorldTime>,
) {
    if sim_state.room_transition_cooldown > 0.0 {
        return;
    }
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_q.single().ok());
    let Some(kin) = subject.and_then(|subject| bodies.get(subject).ok()) else {
        return;
    };
    // CC2 (§3.3): sweep the body's frame path into the zone so a fast body
    // can't tunnel an overlap-fire (`Walk`) loading zone between frames. The
    // discrete standing-in-it case is `delta == 0`, preserved exactly.
    let delta = kin.vel * world_time.sim_dt();
    let Some(zone) =
        room_set.transition_for_player(kin.aabb(), delta, slot_gestures.primary().buffered())
    else {
        return;
    };
    // Portal check: if this zone is registered as a portal, the
    // portal's own phase must be `On` for traversal to be allowed.
    // The switch only commands the boot/shutdown sequence — the
    // portal itself runs the state machine. Non-portal zones pass
    // through unchanged.
    if portals.is_portal(&zone.zone.id) && !portals.allows_traversal(&zone.zone.id) {
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
    // Clear the interact buffer so the same press doesn't re-trigger
    // a transition next frame before `load_room` resets it.
    slot_gestures.primary_mut().clear();
    transition_writer.write(RoomTransitionRequested::new(zone, zone_sfx));
}
