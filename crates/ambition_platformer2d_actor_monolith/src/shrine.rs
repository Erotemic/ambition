//! Healing / save-point shrine.
//!
//! An interactable shrine that, on a single `Interact`, heals the player to
//! full (health + mana) and acts as a save point (decided: one Interact
//! does both).
//!
//! The autosave compares values, so the marker wrote nothing; and there was no checkpoint field
//! to write into even if it had. Both halves matter: a checkpoint nothing records is a lie, and
//! a checkpoint nothing restores is a number in a file.
//!
//! [`PersistedCheckpoint`]: ambition_persistence::save_data::PersistedCheckpoint
//!
//! Handoff / not-yet-built:
//! - placement is LDtk-authored (`ShrineSpawn`); routing the heal/save through
//!   the affordance/prompt system via an `Interactable` is the follow-up (see
//!   TODO "Healing / save-point shrine").

use ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse;
use bevy::prelude::*;

use ambition_characters::actor::BodyHealth;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::BodyMana;
use ambition_platformer2d_core::{self as ae, AabbExt};

/// A healing / save-point shrine the player can `Interact` with.
#[derive(Component, Clone, Copy, Debug)]
pub struct HealShrine {
    pub pos: Vec2,
    pub half_extent: Vec2,
}

// The heal/save shrine is now an LDtk-authored `ShrineSpawn` entity (spawned at
// room load through the installed placement-lowering registry); the old debug
// spawner is retired.

/// `Interact` while overlapping a [`HealShrine`] heals the body to full
/// (health + mana) and writes a save checkpoint. `interact_pressed` is an edge,
/// so one press = one heal.
///
/// Acts on the controlled subject — the body the player is driving — reading
/// its body-generic [`ActorControl`] interact intent (populated for any body
/// holding the primary seat) and healing THAT body. So a possessed actor resting
/// at a shrine heals itself, not the vacated home avatar. The intent belongs to
/// the body at the shrine, not to one machine-wide input frame (relativity
/// principle / §4 of the restructuring blueprint). Falls back to the primary
/// player for the startup frame before the subject resolver has run.
pub fn heal_save_shrine_system(
    // ⭐ EVERY DRIVEN BODY HEALS. A shrine heals the body that touched it, and
    // "the body that touched it" was one entity by construction — so a couch's
    // second seat could stand in the shrine and press interact forever.
    driven: crate::items::pickup::DrivenBodies,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &mut BodyHealth,
        &mut BodyMana,
    )>,
    // ⚠ THE STARTUP-FRAME FALLBACK SUBJECT, and nothing else. Before a seat is
    // attached there is no driven body at all, and the primary avatar is the
    // subject every single-player fixture expects.
    //
    // ⛔ It is NOT the checkpoint's owner. That comment lived here and the code
    // never followed it — see the checkpoint write below for which rule is real.
    primary: Query<Entity, ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    shrines: Query<&HealShrine>,
    // WHICH room the checkpoint is in. A position with no room is not a
    // checkpoint — it is a pair of numbers that will one day be applied in the
    // wrong place. Optional so narrow fixtures without a room set still heal.
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    // THE INSTANT, which is the half a `PersistedCheckpoint` cannot carry.
    // That value says WHERE the body comes back; this says WHEN the rest of the
    // world was last agreed, and every domain that has reset-relevant state
    // snapshots itself off it. `Option` so a narrow fixture with no horizon
    // installed still heals and still writes its checkpoint.
    mut committed: Option<
        MessageWriter<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>,
    >,
    mut activation: ResMut<ShrineActivationPulse>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    // ⚠ THE FALLBACK IS THE STARTUP FRAME and nothing else.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.single().ok());
    }
    // ⛔⛔ THE CHECKPOINT IS ONE FACT AND THE HEAL IS N. Two seats resting on the
    // same tick heal two bodies and must not write two checkpoints — the second
    // would silently overwrite the first. It is written by the FIRST body in
    // `driven.entities()`' rewind-stable order that actually rests, so the value
    // does not depend on query order.
    //
    // ⚠ WHICH BODY SHOULD OWN IT IS AN OPEN QUESTION, and this preserves today's
    // answer rather than deciding it: the comment below has long claimed the
    // checkpoint is "the PRIMARY player's session, not the possessed subject's
    // body" while the code has always written the RESTING body's position — so a
    // checkpoint taken while possessing resumes the primary avatar somewhere it
    // never stood. See D-SHRINE-CHECKPOINT-OWNER in `docs/planning/queue.md`;
    // changing it is a save-compatibility ruling, not a side effect of a
    // multi-seat conversion.
    let mut checkpoint_written = false;
    for subject in subjects {
        let Ok((control, kin, mut health, mut mana)) = bodies.get_mut(subject) else {
            continue;
        };
        if !control.0.interact_pressed {
            continue;
        }
        let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
        let touching = shrines
            .iter()
            .any(|s| player_aabb.strict_intersects(ae::Aabb::new(s.pos, s.half_extent)));
        if !touching {
            continue;
        }
        health.reset(); // health to full
        mana.meter.refill_full(); // mana to full
        if checkpoint_written {
            // Healed, and the session already has its checkpoint for this tick.
            continue;
        }
        checkpoint_written = true;

        // THE CHECKPOINT.
        //
        // Written for the PRIMARY player's session, not the possessed subject's body:
        // the checkpoint is where this player resumes, and a possessed actor's
        // position is not where the player will be standing next session. The heal
        // above is the subject's; the checkpoint is the session's.
        if let Some(room_set) = room_set.as_deref() {
            let checkpoint = ambition_persistence::save_data::PersistedCheckpoint::new(
                room_set.active_spec().id.clone(),
                kin.pos.x.round() as i32,
                kin.pos.y.round() as i32,
            );
            // Assign only on a real change, so resting twice at the same shrine does
            // not churn the file.
            if save.data().checkpoint.as_ref() != Some(&checkpoint) {
                save.data_mut().checkpoint = Some(checkpoint);
            }
        }
        // RAISED UNCONDITIONALLY, and NOT inside the change guard above.
        // Resting twice at the same shrine writes the same position, so that guard
        // is right about the FILE and would be badly wrong about the horizon: the
        // second rest is a real checkpoint at which the player may be carrying
        // something they were not carrying at the first. Suppressing it would leave
        // the baseline describing the earlier visit, and a later death would take
        // back an object the player had legitimately banked.
        //
        // raised even when there is no room set, for the same reason the heal
        // above is: a composition with no rooms still has hands.
        if let Some(committed) = committed.as_mut() {
            committed.write(ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted);
        }
        activation.remaining = 0.78;
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
            pos: kin.pos,
        });
        bevy::log::info!(
            target: "ambition_platformer2d::shrine",
            "shrine: healed to full + checkpoint recorded"
        );
    }
}

/// Resume where the player last rested.
///
/// A checkpoint you cannot return to is a number in a file. This is the other
/// half: once per constructed session, if the save names a checkpoint in the room
/// that was just built, the primary body starts THERE instead of at the room's
/// authored spawn.
///
/// Deliberately a separate system rather than a branch inside session setup.
/// Construction has one job and already has more parameters than it should; a
/// post-construction placement is additive, is testable on its own, and cannot
/// make the authored-spawn path behave differently for every existing test.
///
/// How far the once-per-session checkpoint resume has got, per session
/// generation.
///
/// ⛔⛔ THIS WAS TWO `Local`s ON A SIM SYSTEM, AND A `Local` DOES NOT REWIND.
/// `restore_checkpoint_on_session_start` runs in `PlayerSimulation`, so a
/// rollback that crossed the frame it routed on would resimulate with the memory
/// already past the crossing: one timeline asks for the resume, the other
/// believes it already did.
///
/// ⚠ UNREACHABLE TODAY, AND THAT IS NOT A REASON TO LEAVE IT. A confirmed room
/// transition rebases GGRS onto a new frame zero, so no rewind crosses the
/// commit — which makes this a correctness that holds because some OTHER layer
/// rebases, and it moves when the rebase does. Same argument, same verdict, as
/// the Mary-O room memory in `rollback_room_memory.rs`; see its `⚠ WHAT THIS
/// FILE DOES NOT PIN` note for the honest statement of what a guard here can and
/// cannot see.
///
/// ⭐ THE GENERATION IS PART OF THE VALUE, so a memory left over from a retired
/// session simply does not match the live one and self-corrects. That is why
/// this is not also session-scoped state.
#[derive(bevy::prelude::Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct CheckpointResumeProgress {
    /// The session generation this resume has finished placing the body for.
    pub applied_for: Option<Option<u64>>,
    /// The session generation this resume has already asked for a crossing on.
    pub routed_for: Option<Option<u64>>,
}

impl CheckpointResumeProgress {
    /// ⭐ WHICH GENERATION, not merely "a memory exists". A presence probe
    /// satisfies the coverage oracle while seeing nothing of the value, and the
    /// value here is the whole decision: a restore that brought back the wrong
    /// generation makes one timeline re-ask for a crossing the other already
    /// spent.
    pub fn checksum(&self) -> u64 {
        fn leg(slot: Option<Option<u64>>) -> u64 {
            match slot {
                None => 0,
                Some(None) => 1,
                Some(Some(generation)) => generation ^ 0x9e37_79b9_7f4a_7c15,
            }
        }
        leg(self.applied_for).rotate_left(1) ^ leg(self.routed_for)
    }
}

/// Movement goes through [`ae::movement::transit_body`] — the ONE transit
/// authority (ADR 0024) — so arrival is at rest with contacts and attachment
/// reconciled, not a raw position write that leaves the body believing it is
/// still standing on the floor it left.
///
/// Runs once per session: `applied_for` remembers which session generation it has
/// already placed, so a later room transition does not yank the player back to the
/// shrine they woke up at.
pub fn restore_checkpoint_on_session_start(
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    scope: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    mut pending: ResMut<crate::session::lifecycle_commit::PendingLifecycleCommit>,
    boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    mut bodies: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    // The body being RESUMED, by stable identity. A transition names the body it moves: the
    // resume is the primary avatar by definition — it is the body the save is about — and
    // saying so is what keeps the commit from asking, several frames later, whoever happens to
    // be controlled then. Disjoint from `bodies`, which borrows no `SimId`.
    subjects: Query<
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    // ⛔⛔ NOT `Local`s. See [`CheckpointResumeProgress`].
    mut progress: ResMut<CheckpointResumeProgress>,
) {
    let Some(room_set) = room_set.as_deref() else {
        return;
    };
    let generation = scope.and_then(|scope| scope.current()).map(|id| id.0);
    if progress.applied_for == Some(generation) {
        return;
    }
    let Some(checkpoint) = save.data().checkpoint.as_ref() else {
        // Nothing to resume. Mark the session handled so this stops looking.
        progress.applied_for = Some(generation);
        return;
    };

    // ROUTE FIRST. The room id was only COMPARED against whatever room the session happened to
    // open, and a mismatch returned: rest in B, quit, start a session that opens in A, and the
    // checkpoint was silently ignored.
    //
    // Requesting an ordinary transition rather than repointing the room set:
    // staging a room is a transaction with content, geometry and authorization
    // in it, and "the one place rooms are staged" is worth more than saving a
    // message.
    if checkpoint.room_id != room_set.active_spec().id {
        // Once per session. A transition takes several frames to commit, and
        // re-requesting every frame would restart it forever.
        if progress.routed_for == Some(generation) {
            return;
        }
        if !room_set
            .rooms
            .iter()
            .any(|room| room.id == checkpoint.room_id)
        {
            // Not fatal and not silent: the session keeps its own starting room.
            bevy::log::warn!(
                target: "ambition_platformer2d::shrine",
                "checkpoint names room `{}`, which this world does not contain; \
                 starting at the session's own room instead",
                checkpoint.room_id
            );
            progress.applied_for = Some(generation);
            return;
        }
        // resolved BEFORE the latch: a session whose avatar has not been built
        // yet cannot name its subject, and marking the route done would spend the
        // once-per-session request on a crossing nobody could describe. Try again
        // next tick instead.
        let Ok(subject) = subjects.single() else {
            return;
        };
        let subject = subject.clone();
        progress.routed_for = Some(generation);
        // The intent can: a resume is a body, a destination and an arrival, which is all a
        // crossing ever was. The synthetic zone is deleted with the message, and so is the
        // room-INDEX lookup that only existed to fill it.
        // ⚠ A REFUSAL IS ORDINARY HERE and costs nothing: nothing above this
        // line has changed the world, and the checkpoint resume is re-asked on
        // the next `ResetToCheckpoint`.
        let _ = pending.record(
            boundary.map_or(0, |boundary| boundary.current),
            crate::session::lifecycle_commit::LifecycleIntent::Transition(
                crate::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room: checkpoint.room_id.clone(),
                    arrival: ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
                    // A resume is not a walk off the side of a room.
                    edge_exit: false,
                    // silent on purpose: nobody opened a door.
                    zone_sfx: None,
                },
            ),
        );
        return;
    }

    let Ok((clusters, mut model)) = bodies.single_mut() else {
        // No body yet — construction has not finished. Leave `applied_for`
        // untouched so the next tick tries again, rather than marking a session
        // handled that was never placed.
        return;
    };
    progress.applied_for = Some(generation);
    let mut item = clusters;
    let mut clusters = item.as_clusters_mut();
    ae::movement::transit_body(
        &mut model,
        &mut clusters,
        ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
        ae::movement::TransitVelocity::Zero,
    );
    bevy::log::info!(
        target: "ambition_platformer2d::shrine",
        "resumed at the checkpoint in `{}` ({}, {})",
        checkpoint.room_id, checkpoint.x, checkpoint.y
    );
}

/// Resume at the checkpoint because the player DIED — the placement domain's
/// leg of the reset horizon.
///
/// `RoomReplayRequested` is NOT that road — its own consumer's doc says so in as many words: it
/// resets feature state in place, never sweeps `RoomScopedEntity`, and never re-runs authored
/// construction. Driving it and asserting "the room came back" is measuring a road you did not
/// take.
///
/// so a death is a checkpoint RESUME, and it records the same description
/// [`restore_checkpoint_on_session_start`] records — a body, a destination, an
/// arrival. That the two triggers reach one operation is the point: a session
/// opening at a checkpoint and a death returning to one are the same question
/// asked twice.
///
/// with no checkpoint recorded, it rebuilds the ACTIVE room at its authored
/// spawn. That is the empty-baseline case rather than a missing one: a game
/// with no checkpoints restores every authored occurrence to where its record
/// puts it, which is exactly what a sandbox reset means.
pub fn resume_at_checkpoint_on_reset(
    mut resets: bevy::prelude::MessageReader<
        ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint,
    >,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    mut pending: ResMut<crate::session::lifecycle_commit::PendingLifecycleCommit>,
    boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    subjects: Query<
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut admitted: bevy::prelude::MessageWriter<ambition_combat::events::RoomReplayAdmitted>,
) {
    // Drained unconditionally, so a reset seen while no body exists cannot be
    // re-read several frames later against a different world.
    let requested = resets.read().count() > 0;
    if !requested {
        return;
    }
    let Some(room_set) = room_set.as_deref() else {
        return;
    };
    // the subject is resolved BEFORE anything is recorded: a transition names the body it
    // moves, and a session whose avatar has not been built cannot describe one.
    let Ok(subject) = subjects.single() else {
        return;
    };
    let active = room_set.active_spec();
    let (target_room, arrival) = match save.data().checkpoint.as_ref() {
        // Not fatal: fall through to rebuilding where the player actually is.
        Some(checkpoint)
            if room_set
                .rooms
                .iter()
                .any(|room| room.id == checkpoint.room_id) =>
        {
            (
                checkpoint.room_id.clone(),
                ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
            )
        }
        _ => (active.id.clone(), active.world.spawn),
    };
    // ⚠ Same rule as the session-start resume above: nothing has been mutated,
    // so a refused slot simply means another lifecycle operation is already
    // taking the world somewhere.
    //
    // ⭐ AND THIS IS THE REPLAY'S ADMISSION ON THE DEATH ROAD. `ResetToCheckpoint`
    // is the death/retry horizon; the room rebuild it schedules IS the replay,
    // so announcing it here is what lets the death's consequences run — and
    // stops the death asking for two lifecycle operations that then fight over
    // one slot.
    let admission = pending.record(
        boundary.map_or(0, |boundary| boundary.current),
        crate::session::lifecycle_commit::LifecycleIntent::Transition(
            crate::session::lifecycle_commit::RoomTransitionIntent {
                subject: subject.clone(),
                target_room,
                arrival,
                // A death is not a walk off the side of a room.
                edge_exit: false,
                // silent on purpose: nobody opened a door.
                zone_sfx: None,
            },
        ),
    );
    if admission.admitted() {
        admitted.write(
            ambition_combat::events::RoomReplayAdmitted::because(
                // A checkpoint resume is the DEATH/RETRY horizon by contract, so
                // its policy is a death's: the player's placed gun portals
                // survive, where a deliberate retry clears them.
                ambition_combat::RoomResetReason::PlayerDeath,
            )
            .for_subject(subject.clone()),
        );
    }
}

#[cfg(test)]
mod tests;
