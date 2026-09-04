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
//! ⭐ NOBODY BUILDS A ROOM HERE. `runtime::sandbox_reset::admit_room_replay`
//! records a lifecycle intent to re-enter the ACTIVE room, and the
//! room-transition road prepares, authorizes, commits and rebases it — the same
//! road a door takes, and the same road a checkpoint resume already takes
//! (`crate::shrine::resume_at_checkpoint_on_reset`). So the retention rules are
//! not restated here either: a transition retires `RoomResident` (room scope,
//! minus whatever is in somebody's hands) and prepares against what the world
//! remembers, which is exactly what a replay wants.
//!
//! What is left in this module is the one thing that road cannot know about:
//! the previous ATTEMPT's residue.
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

use ambition_combat::events::{RoomReplayAdmitted, RoomResetReason};

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
type AttemptResidue = (
    Or<(
        With<crate::features::ecs::SpawnedThisAttempt>,
        With<ambition_combat::components::PostBossNpc>,
        With<ambition_projectiles::LiveProjectile>,
    )>,
    // ⛔⛔ NOT WHAT A BODY IS CARRYING, and the room sweep already says so:
    // its roster is `(With<RoomScopedEntity>, Without<InCustodyOf>)`. The same
    // rule, for the same reason — an object in a hand is not the room's to
    // retire and is not the attempt's either, because the CHECKPOINT has a
    // claim on it that outranks the attempt being un-fought.
    Without<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>,
);

/// Retire what the PREVIOUS ATTEMPT left behind, once the replay is admitted.
///
/// ⛔⛔ IT USED TO DO THIS ON THE REQUEST, AND ALSO RECORD THE INTENT — in that
/// order. So a replay whose lifecycle slot was already owned by another
/// operation despawned the attempt's loot, its post-boss NPCs and its in-flight
/// shots, and then failed to schedule any rebuild at all. Admission is
/// `runtime::sandbox_reset::admit_room_replay`'s job now; this reacts to the
/// FACT, and the room's own rebuild is the transition road's.
pub fn retire_the_previous_attempt(
    mut admitted: MessageReader<RoomReplayAdmitted>,
    mut commands: Commands,
    residue: Query<Entity, AttemptResidue>,
) {
    let reasons: Vec<RoomResetReason> = admitted.read().map(|admitted| admitted.reason).collect();
    if reasons.is_empty() {
        return;
    }
    bevy::log::info!(
        target: "ambition_platformer2d::room_reset",
        "retiring the previous attempt's residue: reasons={reasons:?}",
    );
    for entity in &residue {
        commands.entity(entity).despawn();
    }
}
