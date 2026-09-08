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
    mut accepted: ResMut<AcceptedCheckpointRestore>,
    // The pinned inputs, read ONCE on acceptance. `Option` because a composition
    // can install the session offer without the lifecycle or item ones.
    baselines: (
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>>,
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>>,
        Option<Res<crate::items::pickup::minted_horizon::MintedItemBaseline>>,
        Option<Res<crate::items::pickup::minted_horizon::OwnedItemsBaseline>>,
    ),
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
    // Built once and kept: the accepted operation NAMES the intent it owns, so
    // a later preparation can tell this reconstruction from an ordinary door.
    let intent = crate::session::lifecycle_commit::LifecycleIntent::Transition(
        crate::session::lifecycle_commit::RoomTransitionIntent {
            subject: subject.clone(),
            target_room,
            arrival,
            // A death is not a walk off the side of a room.
            edge_exit: false,
            // silent on purpose: nobody opened a door.
            zone_sfx: None,
        },
    );
    let admission = pending.record(frame, intent.clone());
    if !admission.admitted() {
        // ⛔ THE REQUEST SURVIVES A REFUSAL. Another lifecycle operation owns the
        // world right now; this one is asked again next tick, and until then NO
        // domain has been told anything.
        return;
    }
    outstanding.0 = false;
    // ⭐ PINNED HERE AND NOWHERE ELSE. The inputs a reconstruction is prepared
    // from and applied from are chosen at the moment the slot says yes, so a
    // later capture, a later pickup or a later ledger write cannot retarget an
    // operation already in flight.
    let (occurrences, custody, minted, owned) = baselines;
    accepted.accept(AcceptedRestore {
        frame,
        intent,
        occurrences: occurrences.map(|b| b.clone()).unwrap_or_default(),
        custody: custody.map(|b| b.clone()).unwrap_or_default(),
        item: minted.zip(owned).map(|(minted, owned)| {
            crate::items::pickup::minted_horizon::ItemCheckpointRestoreInputs {
                minted: minted.clone(),
                owned: owned.clone(),
            }
        }),
    });
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

/// The accepted restore, pinned at admission and OUTLIVING ITS FRAME.
///
/// ⛔⛔ **THIS IS THE HALF THE SHARED TOKEN MUST NOT BECOME.**
/// `AdmittedCheckpointRestore` is a one-frame authorization every domain reads:
/// which operation, and whose. This is the SESSION's own accepted-operation
/// state — the pinned inputs that operation was accepted with — and it is here
/// because the session coordinator owns their consistency boundary and may name
/// item types `shared_tangle` must never depend on.
///
/// ⭐ WHY IT HAS TO OUTLIVE THE FRAME, measured 2026-09-08. Room preparation
/// derives the destination's occurrence outlook from the LIVE ledger, and uses
/// it both to validate a prefetched plan and to lower a fresh one. On a
/// checkpoint reset that ledger is only the right answer because
/// `restore_occurrence_baseline` overwrote it earlier in the SAME FRAME —
/// `CheckpointRestore` sits in `PlayerInput`, room-transition readiness runs
/// after `RoomTransitionSet::Detect` in `RoomTransition`, and the phase order
/// puts one before the other. So the room was prepared from a live resource
/// swapped to the checkpoint value in order to be read, which is exactly the
/// preparation shape the protocol forbids, and its correctness rested on a
/// phase-ordering accident nothing states as a checkpoint requirement.
///
/// ⇒ Preparation reads THIS instead. The pinned population is the operation's
/// own, whatever the live ledger says by the time the load runs.
///
/// ⚠ IT PINS WHAT PREPARATION CONSUMES, and nothing else. Custody and
/// entitlement snapshots are not here because their reducers still run in the
/// restore set against live baselines in the same frame they are admitted —
/// pinning a value no one reads differently is the shape that produced four
/// dead `LifecycleIntent` variants. They join when application moves to the
/// commit boundary and the difference becomes observable.
#[derive(bevy::prelude::Resource, Default, Clone, Debug, PartialEq)]
pub struct AcceptedCheckpointRestore(Option<AcceptedRestore>);

/// One accepted restore and the reconstruction inputs it was accepted with.
#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedRestore {
    /// The sim frame the admission happened on.
    pub frame: i32,
    /// The room intent this operation was admitted for.
    ///
    /// ⭐ THIS IS HOW A LATER PREPARATION KNOWS THE TRANSITION IT IS PREPARING
    /// IS THIS RESTORE. A checkpoint reset records an ordinary `Transition` —
    /// indistinguishable from a door by its shape — so the accepted operation
    /// names the intent it owns rather than letting preparation guess from the
    /// destination.
    pub intent: crate::session::lifecycle_commit::LifecycleIntent,
    /// The occurrence population this operation reconstructs, as it stood when
    /// the slot accepted it.
    pub occurrences: ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline,
    /// The custody relation it restores.
    pub custody: ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline,
    /// The item domain's half: how to rebuild the runtime mints those custody
    /// rows name, and the entitlement quantities. `None` in a composition with
    /// no item domain, which is "not participating" and not "erase the bag".
    ///
    /// ⚠ THE MINTS TRAVEL WITH THE LEDGER for the reason `OccurrenceContinuity`
    /// states: the memory without the means to act on it deletes the object.
    pub item: Option<crate::items::pickup::minted_horizon::ItemCheckpointRestoreInputs>,
}

impl AcceptedCheckpointRestore {
    /// The accepted operation, if one is outstanding.
    pub fn accepted(&self) -> Option<&AcceptedRestore> {
        self.0.as_ref()
    }

    /// The pinned reconstruction inputs, but only for `intent`.
    ///
    /// ⛔ MATCHED, NOT MERELY PRESENT. A door crossing recorded while a
    /// checkpoint restore is outstanding must be prepared from LIVE state; a
    /// preparation that took the pinned population for any transition would
    /// rebuild the wrong room's population from a checkpoint that is not about
    /// it.
    pub fn inputs_for(
        &self,
        intent: &crate::session::lifecycle_commit::LifecycleIntent,
    ) -> Option<&AcceptedRestore> {
        self.0.as_ref().filter(|accepted| &accepted.intent == intent)
    }

    /// Accept an operation. Called only by the session coordinator, with an
    /// `Admission` in hand.
    pub fn accept(&mut self, accepted: AcceptedRestore) {
        self.0 = Some(accepted);
    }

    /// Retire the operation once its lifecycle intent has left the slot.
    pub fn retire(&mut self) -> Option<AcceptedRestore> {
        self.0.take()
    }

    /// ⭐ EVERY FIELD THAT CAN CHANGE WHAT THIS OPERATION BUILDS.
    ///
    /// ⛔⛔ ITS FIRST VERSION COVERED THREE OF FOUR AND WAS WRONG FOR IT. It
    /// hashed the frame, the pinned ledger and the intent's TARGET ROOM — so two
    /// accepted restores with different pinned MINT RECIPES agreed, and so did
    /// two crossings that differ only in subject, arrival, edge or door cue.
    /// Both halves are consumed: room preparation lowers the fresh plan from the
    /// ledger AND the mints, and `inputs_for` matches on the whole intent by
    /// equality. A projection that covers part of a value reports agreement
    /// between peers holding different operations, which is worse than having
    /// none. The intent's own leg is exhaustive by destructure.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_i32, put_u64, put_u8};
        let Some(AcceptedRestore {
            frame,
            intent,
            occurrences,
            custody,
            item,
        }) = &self.0
        else {
            return 0;
        };
        let mut bytes = Vec::new();
        put_i32(&mut bytes, *frame);
        put_u64(&mut bytes, intent.checksum());
        put_u64(&mut bytes, occurrences.checksum());
        // ⚠ ABSENT AND EMPTY ARE DIFFERENT ANSWERS. "No mint baseline installed"
        // is a composition without the item domain; "installed and empty" is a
        // checkpoint that saw no runtime mints. Folding them together would let
        // a peer with no item domain agree with one that has an empty baseline.
        put_u64(&mut bytes, custody.checksum());
        match item {
            None => put_u8(&mut bytes, 0),
            Some(item) => {
                put_u8(&mut bytes, 1);
                put_u64(&mut bytes, item.minted.checksum());
                put_u64(&mut bytes, item.owned.checksum());
            }
        }
        checksum_bytes(&bytes)
    }
}

/// Retire an accepted restore once its intent has left the lifecycle slot.
///
/// ⛔ THE SLOT IS THE AUTHORITY ON WHETHER THE OPERATION IS STILL LIVE. The
/// commit takes the intent; a retraction removes it. Either way the accepted
/// inputs have no operation left to describe, and holding them would let a
/// later unrelated transition match a stale intent by value.
pub fn retire_accepted_checkpoint_restore(
    mut accepted: ResMut<AcceptedCheckpointRestore>,
    pending: Res<crate::session::lifecycle_commit::PendingLifecycleCommit>,
) {
    let still_pending = accepted.accepted().is_some_and(|accepted| {
        pending
            .peek()
            .is_some_and(|pending| pending.kind == accepted.intent)
    });
    if accepted.accepted().is_some() && !still_pending {
        let _ = accepted.retire();
    }
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

/// Run the committed restore's domain application, if this commit is one.
///
/// ⛔⛔ **THIS IS THE ONE ENTRY POINT, AND BOTH HOSTS CALL IT.** The eager host
/// reaches it from an exclusive runner ordered after its room commit; the
/// confirmed host calls it from `commit_confirmed_lifecycle`'s exclusive tail,
/// before the rebase — otherwise its first restore would undo the checkpoint it
/// just restored. They differ in what authorizes the commit, never in what the
/// commit does.
///
/// ⭐ THE INPUTS ARE INSTALLED AND REMOVED HERE, on every path. A reducer that
/// ran with none does nothing, which is what makes an accidental invocation a
/// no-op instead of a restore to an empty baseline. Returns whether it applied.
pub fn apply_committed_checkpoint_restore(
    world: &mut bevy::prelude::World,
    intent: &crate::session::lifecycle_commit::LifecycleIntent,
) -> bool {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        CheckpointDomainApply, CheckpointRestoreInputs,
    };

    let Some(accepted) = world
        .get_resource::<AcceptedCheckpointRestore>()
        .and_then(|accepted| accepted.inputs_for(intent))
        .cloned()
    else {
        // An ordinary door. Nothing about this commit is a checkpoint restore.
        return false;
    };

    world.insert_resource(CheckpointRestoreInputs {
        occurrences: accepted.occurrences.clone(),
        custody: accepted.custody.clone(),
    });
    if let Some(item) = accepted.item.clone() {
        world.insert_resource(item);
    }
    // The schedule exists only where the lifecycle offer is installed; a
    // composition without it has nothing to apply, which is not an error.
    if world.try_run_schedule(CheckpointDomainApply).is_err() {
        bevy::log::warn!(
            target: "ambition_platformer2d::session",
            "a checkpoint restore committed in a composition with no domain-apply \
             schedule; no domain state was restored",
        );
    }
    // ⛔ REMOVED ON EVERY PATH, including the missing-schedule one above. A
    // context left installed is a reducer that can be invoked as an effective
    // restore by anything that later runs that schedule.
    world.remove_resource::<CheckpointRestoreInputs>();
    world
        .remove_resource::<crate::items::pickup::minted_horizon::ItemCheckpointRestoreInputs>();
    // ⭐ THE STRUCTURAL WORK THE CUSTODY RESTORE QUEUED. It materializes
    // occurrences the checkpoint remembers in a hand; a caller that returned
    // before this flush would leave them as queued commands nobody applied, and
    // a test observing "the object came back" would be observing a command
    // buffer.
    world.flush();
    true
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

        app.init_resource::<AcceptedCheckpointRestore>();
        app.init_resource::<CheckpointResumeProgress>();
        app.init_resource::<OutstandingCheckpointRequest>();
        // ⭐ ADMISSION IS ALL THAT REMAINS IN THE SIMULATION. The restore itself
        // runs from the commit executor, so `CheckpointRestore` now contains the
        // session's admission and the retirement that follows the slot — and
        // nothing that mutates a domain.
        app.add_systems(
            sim,
            (resume_at_checkpoint_on_reset, retire_accepted_checkpoint_restore)
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestore),
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
