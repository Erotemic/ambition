//! Same-room replay: rebuild the room you are standing in.
//!
//! ```text
//! prepared immutable content + durable occurrence facts + retention policy
//!                             |
//!                             v
//!                   RoomConstructionPlan
//!                             |
//!   new session -- transition -- REPLAY -- new-game reset -- hot reload
//! ```
//!
//! ⛔⛔ THE REPLAY USED TO BE THE ONE PATH OUTSIDE THAT DIAGRAM. It mutated
//! every surviving entity back toward a presumed spawn state through a
//! hand-kept list — pickups, chests, breakables, actor spawn state,
//! dispositions, aggression, pinned poses, boss health/phase/brain/anim,
//! hazards, switches, encounter entities — and every row of that list was a
//! fact somebody noticed was missing. The list could only grow, one bug report
//! at a time, and adding an authoritative family to fresh construction never
//! added it to the replay.
//!
//! ⭐ THIS MODULE NAMES A ROOM; IT DOES NOT BUILD ONE. A replay records a
//! lifecycle intent to re-enter the ACTIVE room, and the room-transition road
//! prepares, authorizes, commits and rebases it — the same road a door takes,
//! and the same road a checkpoint resume already takes
//! (`crate::shrine::resume_at_checkpoint_on_reset`). So the retention rules are
//! not restated here either: a transition retires `RoomResident` (room scope,
//! minus whatever is in somebody's hands) and prepares against what the world
//! remembers, which is exactly what a replay wants.
//!
//! ⛔⛔ AND THE AUTHORIZATION IS WHY IT MUST BE THAT ROAD AND NOT A SECOND
//! CALLER OF THE SAME PLAN. A room rebuild is a structural change to the
//! simulated world, so under a rollback host it may only commit at a confirmed
//! frame, after which GGRS rebases onto a fresh frame-zero baseline. A version
//! of this module that prepared and committed the plan itself passed every
//! eager-host test and desynced a sync-test session within three frames of a
//! death (`rollback_lifecycle_reset`, checksum mismatch at frames 105-107).
//! Consuming the same construction semantics is not enough; the lifecycle
//! authorization layer is part of the road.
//!
//! What a replay retires BEYOND the room is the one thing the transition road
//! cannot know about: `AttemptResidue`. Those are SESSION-scoped by lifetime —
//! a weapon you drop and walk back to is intended behaviour — so no room sweep
//! reaches them and the policy has to say so out loud.

use bevy::prelude::*;

use ambition_combat::events::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_platformer2d_world::rooms::RoomSet;

use crate::session::lifecycle_commit::{
    LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
};

/// What the PREVIOUS ATTEMPT made, which the room rebuild does not own.
///
/// Session-scoped by lifetime, so the transition's `RoomResident` sweep never
/// sees it. Each arm is a deliberate statement that the thing belongs to the
/// run rather than to the room or to the player:
///
/// - [`SpawnedThisAttempt`](crate::features::ecs::SpawnedThisAttempt) — loot the
///   attempt dropped. A weapon dropped in a room and found again on the way
///   back is intended; a weapon dropped in the fight being un-fought is not.
/// - `PostBossNpc` — the celebrant a defeated boss leaves behind. The boss is
///   about to be alive again.
/// - `LiveProjectile` — every shot belongs to the combat timeline being reset,
///   whoever fired it.
type AttemptResidue = Or<(
    With<crate::features::ecs::SpawnedThisAttempt>,
    With<ambition_combat::components::PostBossNpc>,
    With<ambition_projectiles::LiveProjectile>,
)>;

/// Ask the one room-construction road to rebuild the ACTIVE room, and retire
/// what the previous attempt left behind.
///
/// Consumes [`ResetRoomFeaturesEvent`] — the request every same-room reset road
/// already writes: the reset input, the death/hazard home policy, and the
/// content-emitted room replay.
pub fn reconstitute_the_active_room(
    mut requests: MessageReader<ResetRoomFeaturesEvent>,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    mut pending: ResMut<PendingLifecycleCommit>,
    boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    subjects: Query<
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut commands: Commands,
    residue: Query<Entity, AttemptResidue>,
) {
    // Drained unconditionally, so a request seen while no world exists cannot be
    // re-read several frames later against a different one.
    let reasons: Vec<RoomResetReason> = requests.read().map(|request| request.reason).collect();
    if reasons.is_empty() {
        return;
    }

    // The previous attempt's residue goes now, whether or not a rebuild can be
    // named: it belongs to the run, and the run is over either way.
    for entity in &residue {
        commands.entity(entity).despawn();
    }

    let Some(room_set) = room_set.as_deref() else {
        return;
    };
    // A transition names the body it moves, and a session whose avatar has not
    // been built cannot describe one.
    let Ok(subject) = subjects.single() else {
        return;
    };
    let active = room_set.active_spec();
    bevy::log::info!(
        target: "ambition_platformer2d::room_reset",
        "room replay requested: reasons={reasons:?} room={}",
        active.id,
    );
    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
        "room-reset reasons={reasons:?}"
    ));
    pending.record(
        boundary.map_or(0, |boundary| boundary.current),
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject: subject.clone(),
            target_room: active.id.clone(),
            arrival: active.world.spawn,
            // A replay is not a walk off the side of a room.
            edge_exit: false,
            // Silent on purpose: nobody opened a door.
            zone_sfx: None,
        }),
    );
}
