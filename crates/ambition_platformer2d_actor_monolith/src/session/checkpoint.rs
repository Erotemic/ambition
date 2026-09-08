//! Restoring a session to its checkpoint: the startup resume and the
//! death/retry reset.
//!
//! ⭐ THIS IS SESSION LIFECYCLE, NOT SHRINE BEHAVIOUR, and the split is the
//! point. A shrine owns an interaction — heal the body that touched me, and ask
//! for a checkpoint to be committed. Coming BACK to a checkpoint is a question
//! about the session: which room it opens in, which body is its subject, and
//! whether the one lifecycle slot will admit the crossing. Those three lived in
//! `shrine.rs` because the write and the read were authored together, and the
//! file's own doc said so — *"a checkpoint nothing restores is a number in a
//! file"* — but the reader never had anything to do with the shrine entity.
//!
//! ⛔ INSTALLATION MOVED WITH THEM. `ItemPickupSimulationPlugin` initialized
//! [`CheckpointResumeProgress`] and installed the startup resume, which made a
//! composition's ability to resume a session depend on it having held items.
//! [`SessionCheckpointHorizonPlugin`] owns both now.

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae};

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
    let Some(checkpoint) = save.data().checkpoint() else {
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
        // The intent can: a resume is a body, a destination and an arrival, which is all a
        // crossing ever was. The synthetic zone is deleted with the message, and so is the
        // room-INDEX lookup that only existed to fill it.
        //
        // ⛔⛔ THE LATCH BELONGS TO THE ADMISSION, NOT TO THE ATTEMPT. This wrote
        // `routed_for` BEFORE asking, and discarded the answer: a slot already
        // owned by another lifecycle intent refused the crossing while the
        // session recorded that it had spent its one resume. The player then
        // stayed in the room the session happened to open in, forever, because
        // the only road back is gated on a generation this line already burned.
        //
        // ⚠ A REFUSAL IS ORDINARY HERE and costs nothing: nothing above this
        // line has changed the world, so the resume is simply re-asked on the
        // next tick, and the incumbent operation keeps the slot it won.
        let admission = pending.record(
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
        if admission.admitted() {
            progress.routed_for = Some(generation);
        }
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
    mut outstanding: ResMut<OutstandingCheckpointRequest>,
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
    mut restore: ResMut<ambition_platformer2d_shared_tangle::lifecycle::AdmittedCheckpointRestore>,
    mut admitted: bevy::prelude::MessageWriter<ambition_combat::events::RoomReplayAdmitted>,
) {
    // ⭐ THE CHANNEL IS DRAINED EVERY FRAME AND THE REQUEST IS REMEMBERED, which
    // are two different things and used to be one. Draining alone meant a reset
    // arriving on a frame that could not describe the operation — no room set,
    // no constructed body, a slot somebody else owned — was simply GONE, and
    // with it every consequence the domains would have applied. Repeated
    // requests coalesce into this one outstanding bit, which is what makes
    // "there is at most one outstanding checkpoint request per session" true
    // rather than aspirational.
    if resets.read().count() > 0 {
        outstanding.0 = true;
    }
    if !outstanding.0 {
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
    let (target_room, arrival) = match save.data().checkpoint() {
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
    let frame = boundary.map_or(0, |boundary| boundary.current);
    let admission = pending.record(
        frame,
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
    if !admission.admitted() {
        // ⛔ THE REQUEST SURVIVES A REFUSAL. Another lifecycle operation owns the
        // world right now; this one is asked again next tick, and until then NO
        // domain has been told anything.
        return;
    }
    outstanding.0 = false;
    restore.admit(
        ambition_platformer2d_shared_tangle::lifecycle::AdmittedRestore {
            frame,
            subject: subject.clone(),
        },
    );
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

/// A checkpoint restore that has been ASKED FOR and not yet admitted.
///
/// ⭐ ONE BIT, AND IT IS ENOUGH. The plan's request value carries a reason and
/// an optional subject; neither has a consumer yet, and a field nothing reads is
/// how the four dead `LifecycleIntent` variants happened. What this must express
/// today is exactly "the session is still owed a restore", and repeated requests
/// coalescing into it is the coalescing rule.
///
/// ⛔⛔ IT IS ROLLBACK STATE. It outlives its frame by construction — that is its
/// whole job — so a rewind past the frame the request arrived on must take the
/// request with it, or one timeline restores a checkpoint the other never asked
/// for.
#[derive(bevy::prelude::Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutstandingCheckpointRequest(pub bool);

impl OutstandingCheckpointRequest {
    /// ⭐ THE VALUE, not its presence. This resource always exists, so a
    /// presence probe would report a constant and see nothing of the one bit
    /// that decides whether a rewound timeline is still owed a restore.
    pub fn checksum(&self) -> u64 {
        u64::from(self.0)
    }
}

/// Retire the admitted operation once every domain has applied it.
///
/// ⛔ THE SESSION RETIRES IT, not a domain. A reducer that cleared the token it
/// had just consumed would decide for its siblings whether they had run.
pub fn retire_admitted_checkpoint_restore(
    mut restore: ResMut<ambition_platformer2d_shared_tangle::lifecycle::AdmittedCheckpointRestore>,
) {
    let _ = restore.retire();
}
/// The session's leg of the reset/checkpoint horizon.
///
/// ⭐ COMPOSED BESIDE THE ITEM OFFER, NOT BY IT. A profile that wants
/// checkpoints and no held items installs this and stops; that is what makes
/// "restoration is session lifecycle" a fact about installation rather than a
/// claim about file placement.
pub struct SessionCheckpointHorizonPlugin;

impl Plugin for SessionCheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        let sim = ambition_platformer2d_shared_tangle::schedule::SimScheduleExt::sim_schedule(app);

        // ⛔ THE PHASES AND THE TOKEN HAVE ONE OWNER, and it is the thing that
        // admits. Splitting `configure_sets` for these steps across the domain
        // offers would give the restore two authorities on its own order.
        app.configure_sets(
            sim,
            (
                ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreStep::Admit,
                ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreStep::Apply,
                ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreStep::Retire,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestore),
        );
        app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::AdmittedCheckpointRestore>();
        app.init_resource::<CheckpointResumeProgress>();
        app.init_resource::<OutstandingCheckpointRequest>();
        app.add_systems(
            sim,
            resume_at_checkpoint_on_reset.in_set(
                ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreStep::Admit,
            ),
        );
        app.add_systems(
            sim,
            retire_admitted_checkpoint_restore.in_set(
                ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreStep::Retire,
            ),
        );
        // ⚠ THE EDGES ARE INHERITED VERBATIM from the item-pickup chain this
        // system was carved out of, and A1b is a move: preserving them is what
        // keeps the churn attributable. They are NOT a derived requirement —
        // what the resume actually needs is stated in its own doc (the first
        // tick a constructed session has a body, ungated by `gameplay_allowed`),
        // and nothing in it concerns a hand. `ItemPickupSet::CoreHeldItems` is
        // nested in `PlayerSimulation` by its owner, so dropping the membership
        // would also move the phase — see A1c, which re-derives this ordering as
        // part of the accepted-restore road.
        app.add_systems(
            sim,
            restore_checkpoint_on_session_start
                .in_set(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                .after(crate::shrine::heal_save_shrine_system)
                .before(ambition_platformer2d_shared_tangle::schedule::HeldItemStep::Release),
        );
    }
}

#[cfg(test)]
mod tests;
