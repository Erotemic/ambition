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
/// How far this session's startup checkpoint resume has got.
///
/// ⛔⛔ THIS WAS TWO `Local`s ON A SIM SYSTEM, AND A `Local` DOES NOT REWIND.
/// `restore_checkpoint_on_session_start` runs in `PlayerSimulation`, so a
/// rollback that crossed the frame it routed on would resimulate with the memory
/// already past the crossing: one timeline asks for the resume, the other
/// believes it already did.
///
/// ⛔⛔ AND IT WAS THEN TWO BOOLEAN-ISH LATCHES — `routed_for` and `applied_for`
/// — which is a SECOND completion mechanism beside the operation model the reset
/// road uses. A startup crossing and a death crossing are the same operation
/// asked twice; two ways of knowing one finished is how they drift. What
/// replaces them is not another pair: [`StartupResume::Routed`] names the
/// admitted operation by KEY, and it becomes `Satisfied` only when that
/// operation publishes its terminal outcome.
///
/// ⭐ THE GENERATION IS PART OF THE VALUE, so a memory left over from a retired
/// session simply does not match the live one and self-corrects. That is why
/// this is not also session-scoped state.
#[derive(bevy::prelude::Resource, Default, Clone, Debug, PartialEq)]
pub struct SessionStartupResume {
    /// The session generation this state describes, and how far it got.
    state: Option<(Option<u64>, StartupResume)>,
}

/// How far one session's startup resume got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupResume {
    /// A cross-room resume was admitted as this operation and is in flight.
    ///
    /// ⚠ NAMED BY KEY, NOT BY A FLAG. "A crossing was asked for" cannot tell
    /// whether the one that committed was THIS one; two crossings to the
    /// checkpoint's room with the same subject compare equal.
    Routed(CheckpointOperationKey),
    /// Nothing further is owed: the body was placed, the operation committed, or
    /// there was nothing to resume.
    Satisfied,
}

impl SessionStartupResume {
    /// This session's state, if the memory is about this session.
    pub fn state_for(&self, generation: Option<u64>) -> Option<StartupResume> {
        self.state
            .as_ref()
            .filter(|(remembered, _)| *remembered == generation)
            .map(|(_, state)| *state)
    }

    fn set(&mut self, generation: Option<u64>, state: StartupResume) {
        self.state = Some((generation, state));
    }

    /// ⭐ WHICH GENERATION AND HOW FAR, not merely "a memory exists". A presence
    /// probe satisfies the coverage oracle while seeing nothing of the value, and
    /// the value here is the whole decision: a restore that brought back the
    /// wrong generation makes one timeline re-ask for a crossing the other
    /// already spent.
    pub fn checksum(&self) -> u64 {
        let Some((generation, state)) = &self.state else {
            return 0;
        };
        let mut hash = match generation {
            None => 1,
            Some(generation) => generation ^ 0x9e37_79b9_7f4a_7c15,
        };
        hash = hash.rotate_left(1)
            ^ match state {
                StartupResume::Satisfied => 2,
                // ⭐ THE KEY'S OWN PROJECTION, not a second spelling of it.
                StartupResume::Routed(key) => {
                    let mut bytes = Vec::new();
                    key.write_into(&mut bytes);
                    ambition_platformer2d_core::snapshot::checksum_bytes(&bytes).rotate_left(5)
                }
            };
        hash | 1
    }
}

/// Movement goes through [`ae::movement::transit_body`] — the ONE transit
/// authority (ADR 0024) — so arrival is at rest with contacts and attachment
/// reconciled, not a raw position write that leaves the body believing it is
/// still standing on the floor it left.
///
/// Runs once per session: [`SessionStartupResume`] remembers which generation it
/// has resolved, so a later room transition does not yank the player back to the
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
    // ⛔⛔ NOT `Local`s. See [`SessionStartupResume`].
    mut progress: ResMut<SessionStartupResume>,
    // The SAME operation state the reset road uses. A startup crossing and a
    // death crossing are one operation asked twice, so they are admitted,
    // pinned, applied and answered by one mechanism.
    mut accepted: ResMut<AcceptedCheckpointRestore>,
    mut operations: ResMut<SessionCheckpointOperations>,
    outcomes: Res<SessionCheckpointOutcomes>,
    baselines: (
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>>,
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>>,
        Option<Res<crate::items::pickup::minted_horizon::MintedItemBaseline>>,
        Option<Res<crate::items::pickup::minted_horizon::OwnedItemsBaseline>>,
    ),
) {
    let Some(room_set) = room_set.as_deref() else {
        return;
    };
    let scope_id = scope.and_then(|scope| scope.current());
    let generation = scope_id.map(|id| id.0);
    match progress.state_for(generation) {
        Some(StartupResume::Satisfied) => return,
        // ⭐ THE ROUTED CROSSING IS FINISHED BY ITS OPERATION'S OUTCOME, not by a
        // flag this system sets when it asks. Waiting on the outcome is what
        // makes the startup road and the reset road one mechanism.
        Some(StartupResume::Routed(key)) => {
            if outcomes.outcome_for(key).is_some() {
                progress.set(generation, StartupResume::Satisfied);
            } else if accepted.inputs_for_key(key).is_none() {
                // ⛔ NEITHER OUTSTANDING NOR ANSWERED: the operation was retired
                // without committing — a cancelled crossing. The resume is owed
                // again rather than lost, which is the F1 rule one level up.
                progress.state = None;
            }
            return;
        }
        None => {}
    }
    let Some(checkpoint) = save.data().checkpoint() else {
        // Nothing to resume. Mark the session handled so this stops looking.
        progress.set(generation, StartupResume::Satisfied);
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
            progress.set(generation, StartupResume::Satisfied);
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
        // ⛔ BEFORE THE SLOT IS TAKEN, for the reason the reset road states: a
        // crossing this session cannot name would be admitted as an ordinary
        // transition and the resume spent on it.
        if !operations.can_admit() {
            bevy::log::error!(
                target: "ambition_platformer2d::session",
                "the checkpoint operation sequence is exhausted; this session \
                 cannot resume at its checkpoint",
            );
            return;
        }
        let frame = boundary.map_or(0, |boundary| boundary.current);
        let intent = crate::session::lifecycle_commit::LifecycleIntent::Transition(
            crate::session::lifecycle_commit::RoomTransitionIntent {
                subject,
                target_room: checkpoint.room_id.clone(),
                arrival: ae::Vec2::new(checkpoint.x as f32, checkpoint.y as f32),
                // A resume is not a walk off the side of a room.
                edge_exit: false,
                // silent on purpose: nobody opened a door.
                zone_sfx: None,
            },
        );
        let admission = pending.record(frame, intent.clone());
        if !admission.admitted() {
            return;
        }
        let Some(key) = operations.admit(scope_id) else {
            bevy::log::error!(
                target: "ambition_platformer2d::session",
                "the checkpoint operation sequence was exhausted between the \
                 capacity check and the admission; the slot is now held by a \
                 resume with no identity",
            );
            return;
        };
        // ⭐ A STARTUP CROSSING IS A CHECKPOINT RECONSTRUCTION, so it pins the
        // same inputs the reset road does. Before this it recorded a bare
        // transition and the destination was prepared from whatever the LIVE
        // ledger happened to hold — correct at session start only because the
        // load had just written the file's ledger into it.
        let (occurrences, custody, minted, owned) = baselines;
        accepted.accept(AcceptedRestore {
            key,
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
        progress.set(generation, StartupResume::Routed(key));
        return;
    }

    let Ok((clusters, mut model)) = bodies.single_mut() else {
        // No body yet — construction has not finished. Leave the startup state
        // unset so the next tick tries again, rather than marking a session
        // resolved that was never placed.
        return;
    };
    // ⭐ A SAME-ROOM STARTUP PLACEMENT IS NOT A RECONSTRUCTION and deliberately
    // does not become one: no room rebuild, no host rebase, no accepted
    // operation. It is a small rollback-registered simulation operation that
    // moves an already-constructed body, and manufacturing a room-reconstruction
    // intent to do it would be the opposite of what the protocol asks.
    progress.set(generation, StartupResume::Satisfied);
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
    mut operations: ResMut<SessionCheckpointOperations>,
    // WHOSE operation. Absent only in an explicit standalone profile, which has
    // one declared lifetime and cannot retain operations across destruction.
    scope: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
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
    // ⛔ BEFORE THE SLOT IS TAKEN. An operation this session cannot NAME must not
    // acquire the lifecycle slot: the room would be rebuilt as an ordinary
    // crossing with no pinned continuity and no domain restore, and the request
    // would be spent. Leaving `outstanding` set keeps it owed instead.
    if !operations.can_admit() {
        bevy::log::error!(
            target: "ambition_platformer2d::session",
            "the checkpoint operation sequence is exhausted; this session can \
             admit no further restores and the request stays owed",
        );
        return;
    }
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
    // Capacity was established before the slot was taken, so this cannot be
    // `None`; the arm exists because a silent `unwrap_or` would turn a future
    // ordering mistake into a nameless operation instead of a loud one.
    let Some(key) = operations.admit(scope.and_then(|scope| scope.current())) else {
        bevy::log::error!(
            target: "ambition_platformer2d::session",
            "the checkpoint operation sequence was exhausted between the capacity \
             check and the admission; the slot is now held by a restore with no \
             identity",
        );
        return;
    };
    let (occurrences, custody, minted, owned) = baselines;
    accepted.accept(AcceptedRestore {
        key,
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

/// The accepted restore: one operation, and every input it was accepted with.
///
/// ⭐ THE SESSION OWNS IT because the session coordinator owns these values'
/// consistency boundary, and because it may name item types `shared_tangle` must
/// never depend on. It is written only on admission, read by room preparation
/// and by the commit executors, and retired when the lifecycle slot gives up the
/// intent.
///
/// ⭐ WHY IT OUTLIVES ITS FRAME, measured 2026-09-08. Room preparation derives
/// the destination's occurrence outlook from a ledger, and uses it both to
/// validate a prefetched plan and to lower a fresh one. Reading the LIVE ledger
/// was only the right answer for a checkpoint reset because
/// `restore_occurrence_baseline` overwrote it earlier in the SAME FRAME —
/// `CheckpointRestore` sat in `PlayerInput`, room-transition readiness runs
/// after `RoomTransitionSet::Detect` in `RoomTransition`, and the phase order
/// put one before the other. So the room was prepared from a live resource
/// swapped to the checkpoint value in order to be read, which is exactly the
/// preparation shape the protocol forbids, and its correctness rested on a
/// phase-ordering accident nothing stated as a checkpoint requirement.
///
/// ⇒ Preparation and application both read THIS. The pinned values are the
/// operation's own, whatever the live baselines say by the time the load runs
/// and the commit lands.
///
/// ⛔ IT IS NOT AN AUTHORIZATION TOKEN, and there is deliberately no longer one.
/// A1c/1-2 published a one-frame `AdmittedCheckpointRestore` that every domain
/// reducer had to remember to consult; A1c/3b deleted it, because the reducers
/// now live in `CheckpointDomainApply` and only a commit executor runs that.
/// Holding these values does not permit a restore — being invoked by the commit
/// does. This is the operation's DATA.
#[derive(bevy::prelude::Resource, Default, Clone, Debug, PartialEq)]
pub struct AcceptedCheckpointRestore(Option<AcceptedRestore>);

/// Which restore operation this is.
///
/// ⛔⛔ **A FRAME NUMBER IS NOT AN IDENTITY, AND NEITHER IS AN INTENT.** The
/// accepted value was matched by `LifecycleIntent` equality, and two crossings
/// to one room with one subject and one arrival compare EQUAL — so a later,
/// unrelated transition could be served the earlier operation's pinned
/// population. A frame does not fix it either: a room rebase restarts the
/// rollback timeline at zero, so frame 12 happens repeatedly within one session.
///
/// ⭐ SO IT IS THE SESSION'S OWNERSHIP STAMP PLUS A SEQUENCE THAT ADVANCES ONLY
/// ON ADMISSION. The scope makes it unique across teardown and re-entry — an old
/// operation cannot act in a new session even if its integer matches — and the
/// sequence makes repeated identical intents distinguishable within one session.
///
/// ⚠ NOT THE HOST'S LOAD SEQUENCE, which counts room-transition transactions and
/// is a different identity with a different lifetime. Two crossings can share
/// neither, one, or both, and conflating them would let a load authorize a
/// restore it has nothing to do with.
///
/// ⛔ IT DOES NOT RESET AT A ROOM REBASE. Recycling a live identifier is how a
/// stale load becomes authorized merely because its integer matches; overflow is
/// refused instead — see [`SessionCheckpointOperations::admit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckpointOperationKey {
    /// The session that owns the operation. `None` only in an explicit
    /// standalone profile with no `ActiveSessionScope`, which has one declared
    /// lifetime and cannot retain operations across destruction.
    ///
    /// ⛔ AN ABSENT SCOPE IS NOT A WILDCARD. Two keys with `None` match only each
    /// other, never a scoped one.
    pub scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// Advances only when the lifecycle slot ADMITS a restore.
    pub sequence: u64,
}

impl CheckpointOperationKey {
    /// The ONE projection of this key, reused by every value that stores it.
    ///
    /// ⛔⛔ IT EXISTS BECAUSE THERE WERE THREE. Two callers folded an optional
    /// scope as `scope.0 | 1 << 63` while a third wrote an explicit presence tag
    /// and the full `u64`, so the same key projected differently depending on
    /// which resource held it — and the bit-or spelling silently collides a
    /// scope whose top bit is set with the absent case. One routine, tagged, so
    /// "absent" and "scope 0" are different answers everywhere.
    pub fn write_into(&self, bytes: &mut Vec<u8>) {
        use ambition_platformer2d_core::snapshot::{put_u64, put_u8};
        match self.scope {
            None => put_u8(bytes, 0),
            Some(scope) => {
                put_u8(bytes, 1);
                put_u64(bytes, scope.0);
            }
        }
        put_u64(bytes, self.sequence);
    }
}

/// The session's admitted-operation counter.
///
/// ⭐ SEPARATE FROM THE ACCEPTED VALUE because it must survive the accepted
/// value's retirement: the next operation's key has to differ from the last
/// one's, and the last one is gone by then.
#[derive(bevy::prelude::Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionCheckpointOperations {
    next_sequence: u64,
}

impl SessionCheckpointOperations {
    /// Whether this session can still name another operation.
    ///
    /// ⛔⛔ **ASKED BEFORE THE LIFECYCLE SLOT IS TAKEN, AND THAT ORDERING IS THE
    /// WHOLE POINT.** Both admission roads used to record the room intent first
    /// and mint the key second, so an exhausted counter left a lifecycle
    /// transition ADMITTED, the request SPENT, and no accepted operation behind
    /// it — the room would then have been rebuilt as an ordinary crossing, with
    /// no pinned continuity and no domain restore at all. "Overflow is refused"
    /// has to mean the operation does not happen, not that it happens without
    /// its identity.
    ///
    /// ⚠ It is remote — at one admission per frame at 60 Hz a `u64` lasts about
    /// ten billion years — and that is exactly why the failure path needs to be
    /// correct rather than plausible: nothing will ever exercise it in play.
    pub fn can_admit(&self) -> bool {
        self.next_sequence != u64::MAX
    }

    /// A counter with no capacity left, for the test that exercises refusal.
    ///
    /// ⚠ A CONSTRUCTOR RATHER THAN A PUBLIC FIELD: the sequence is minted in one
    /// place and this must not become a second way to choose one.
    #[cfg(test)]
    pub(crate) fn exhausted_for_test() -> Self {
        Self {
            next_sequence: u64::MAX,
        }
    }

    /// Mint the key for an operation the slot has just admitted.
    ///
    /// ⛔ CALLED ONLY ON ADMISSION. Incrementing on a request would make the
    /// counter a count of asks, and two peers that refused different numbers of
    /// requests would disagree about the identity of the same operation.
    ///
    /// ⚠ OVERFLOW IS REFUSED, NOT RECYCLED — reusing a live identifier is a
    /// stale load quietly authorized because its integer matched. Callers ask
    /// [`Self::can_admit`] BEFORE taking the slot; this `None` is the
    /// belt-and-braces half of that contract.
    pub fn admit(
        &mut self,
        scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    ) -> Option<CheckpointOperationKey> {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.checked_add(1)?;
        Some(CheckpointOperationKey { scope, sequence })
    }

    /// ⭐ THE VALUE, not its presence. A rewind that brought back a different
    /// count would mint a key another timeline had already spent.
    pub fn checksum(&self) -> u64 {
        self.next_sequence ^ 0x51ed_2701_a37f_9b13
    }
}

/// One accepted restore and the reconstruction inputs it was accepted with.
#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedRestore {
    /// WHICH operation this is. Every later stage names it: the commit that
    /// applies it, the verification that checks it, and the one terminal outcome
    /// published for it.
    pub key: CheckpointOperationKey,
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

    /// The pinned inputs, but only for `intent`.
    ///
    /// ⛔ MATCHED, NOT MERELY PRESENT. A door crossing recorded while a
    /// checkpoint restore is outstanding must be prepared from LIVE state; a
    /// preparation that took the pinned population for any transition would
    /// rebuild the wrong room's population from a checkpoint that is not about
    /// it.
    ///
    /// ⚠ **INTENT EQUALITY IS A WEAKER QUESTION THAN IDENTITY, so this is the
    /// ONE place allowed to ask it** — when a transaction opens, against the
    /// operation that is outstanding at that moment. Two crossings to one room
    /// with one subject and one arrival compare EQUAL, so every later stage
    /// names [`AcceptedRestore::key`] instead: see [`Self::inputs_for_key`].
    pub fn inputs_for(
        &self,
        intent: &crate::session::lifecycle_commit::LifecycleIntent,
    ) -> Option<&AcceptedRestore> {
        self.0.as_ref().filter(|accepted| &accepted.intent == intent)
    }

    /// The pinned inputs for exactly this operation.
    ///
    /// ⭐ WHAT EVERY STAGE AFTER THE TRANSACTION OPENS ASKS. A key names one
    /// admission in one session; nothing can be produced that resembles it.
    pub fn inputs_for_key(&self, key: CheckpointOperationKey) -> Option<&AcceptedRestore> {
        self.0.as_ref().filter(|accepted| accepted.key == key)
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
            key,
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
        // ⚠ AN ABSENT SCOPE IS NOT SCOPE ZERO, and the key owns that rule: one
        // projection, reused, so the same key cannot hash differently depending
        // on which resource is holding it.
        key.write_into(&mut bytes);
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
    key: CheckpointOperationKey,
) -> bool {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        CheckpointDomainApply, CheckpointRestoreInputs,
    };

    let Some(accepted) = world
        .get_resource::<AcceptedCheckpointRestore>()
        .and_then(|accepted| accepted.inputs_for_key(key))
        .cloned()
    else {
        // The operation this transaction was opened for is no longer the
        // outstanding one — retired, or replaced by a later admission. Applying
        // the CURRENT accepted operation instead would restore a checkpoint this
        // commit is not about.
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
    // buffer. Verification below therefore reads APPLIED state, not a queue.
    world.flush();

    let outcome = match verify_restored_domains(world, &accepted) {
        Ok(()) => CheckpointRestoreOutcome::Committed { key: accepted.key },
        Err(failure) => {
            // ⛔⛔ FAIL-CLOSED, AND WHAT THAT DOES AND DOES NOT PROMISE. The
            // destructive application has already run, so there is no old world
            // to return to and this contract does not pretend otherwise. What it
            // promises is that a world which did not come back correctly does
            // not become a world the player is allowed to act in: gameplay is
            // blocked and the failure names the operation and the domain.
            bevy::log::error!(
                target: "ambition_platformer2d::session",
                "checkpoint restore {:?} failed verification in {}: {}. Gameplay is \
                 blocked; the world was NOT returned to its previous state, which \
                 this contract does not offer",
                accepted.key,
                failure.failure.domain(),
                failure.detail,
            );
            if let Some(mut mode) = world.get_resource_mut::<bevy::prelude::NextState<
                ambition_platformer2d_shared_tangle::schedule::GameMode,
            >>() {
                mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Paused);
            }
            CheckpointRestoreOutcome::Failed {
                key: accepted.key,
                failure: failure.failure,
            }
        }
    };
    // ⛔ EXACTLY ONE TERMINAL OUTCOME PER OPERATION, and the session is its sole
    // writer. `publish` refuses a second one for a key it has already answered.
    if let Some(mut outcomes) = world.get_resource_mut::<SessionCheckpointOutcomes>() {
        outcomes.publish(outcome);
    }
    // The accepted operation is answered. Retiring it here rather than waiting
    // for the slot means a later transaction cannot be opened against an
    // operation that has already had its terminal outcome.
    if let Some(mut accepted_state) = world.get_resource_mut::<AcceptedCheckpointRestore>() {
        if accepted_state
            .accepted()
            .is_some_and(|outstanding| outstanding.key == accepted.key)
        {
            let _ = accepted_state.retire();
        }
    }
    true
}

/// What went wrong, and the sentence that explains it.
///
/// ⚠ THE SENTENCE IS DIAGNOSTIC AND GOES NO FURTHER THAN THE LOG. Which contract
/// broke is [`RestoreFailure`], a closed set two peers can agree about; a
/// free-form string is not authoritative state and must not reach a snapshot.
struct RestoreVerificationFailure {
    failure: RestoreFailure,
    detail: String,
}

/// Check the applied world against the snapshots the operation was accepted with.
///
/// ⛔⛔ IT READS THE PINNED VALUES, NOT THE LIVE BASELINES. Verifying against a
/// live baseline would compare the world with whatever the world last said —
/// `capture_*` writes those from live state — so a restore that applied nothing
/// at all would verify clean. The question is whether the world matches the
/// snapshot THIS OPERATION was accepted with.
///
/// ⚠ WHAT IT DOES NOT CHECK, AND WHY — because "not yet" and "deliberately not"
/// are different answers and a reader deserves to know which this is.
///
/// ⛔ **BODY PLACEMENT: DELIBERATELY NOT.** The only thing the operation could be
/// checked against is the intent's arrival, and `transit_body` legitimately
/// reconciles a body off it — contacts, attachment, the floor it landed on. A
/// tight tolerance would PAUSE A WORKING GAME (this verification is fail-closed);
/// a loose one measures nothing. Checking placement needs a postcondition the
/// transit authority states, not a coordinate comparison invented here.
///
/// ⛔ **CLOCKS AND PORTALS: DELIBERATELY NOT.** Nothing in the accepted snapshot
/// describes them, so a check would have to invent a snapshot to compare
/// against — which is a decision about what a checkpoint MEANS, not a
/// verification detail. It belongs to whoever adds that snapshot.
///
/// ⭐ Everything it does check is compared against a value the operation was
/// ACCEPTED with. That is the line: verification asks whether the world matches
/// what this operation promised, never whether the world looks reasonable.
fn verify_restored_domains(
    world: &mut bevy::prelude::World,
    accepted: &AcceptedRestore,
) -> Result<(), RestoreVerificationFailure> {
    use ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences;

    if let Some(live) = world.get_resource::<AuthoredOccurrences>() {
        if live != accepted.occurrences.remembered() {
            return Err(RestoreVerificationFailure {
                failure: RestoreFailure::OccurrenceLedger,
                detail: "the applied ledger does not match the population this \
                         operation was accepted with"
                    .to_string(),
            });
        }
    }
    if let Some(item) = accepted.item.as_ref() {
        if let Some(live) = world.get_resource::<ambition_items::OwnedItems>() {
            if live != item.owned.remembered() {
                return Err(RestoreVerificationFailure {
                    failure: RestoreFailure::Entitlements,
                    detail: "the applied bag does not match the quantities this \
                             operation was accepted with"
                        .to_string(),
                });
            }
        }
    }
    // ── THE ROOM THE OPERATION SAID IT WOULD LEAVE STANDING ──────────────────
    //
    // ⭐ CHEAP, AND NOT REDUNDANT WITH THE TRANSACTION'S OWN CHECKS. Those run
    // BEFORE the destructive application and ask whether the commit may proceed;
    // this runs after and asks what it actually did. A restore that rebuilt some
    // other room put every domain value back against the wrong world.
    {
        let mut rooms = world.query::<&ambition_platformer2d_world::rooms::RoomSet>();
        if let Some(room_set) = rooms.iter(world).next() {
            let standing = &room_set.active_spec().id;
            if standing != accepted.intent.target_room() {
                return Err(RestoreVerificationFailure {
                    failure: RestoreFailure::Room,
                    detail: format!(
                        "the operation reconstructs '{}' and the session is standing \
                         in '{standing}'",
                        accepted.intent.target_room()
                    ),
                });
            }
        }
    }

    // ── THE BODY THE OPERATION IS ABOUT ──────────────────────────────────────
    //
    // ⚠ PRESENCE, NOT POSITION. Where the subject ends up is the transition's
    // arrival contract and is checked by the transit authority; what this asks is
    // whether the body the restore was accepted FOR still exists, because every
    // custody row below names a custodian and a restore that lost its subject
    // restored a hand that is not there.
    if let Some(subject) = accepted.intent.subject() {
        let mut bodies = world.query::<&ambition_platformer2d_shared_tangle::sim_id::SimId>();
        if !bodies.iter(world).any(|live| live == subject) {
            return Err(RestoreVerificationFailure {
                failure: RestoreFailure::Subject,
                detail: format!(
                    "the body this operation restores around ({}) is not in the \
                     world it produced",
                    subject.as_str()
                ),
            });
        }
    }

    // ⭐ CUSTODY IS CHECKED AGAINST THE WORLD, not against a resource, because
    // that is the only domain whose restore SPAWNS: a row the reducer could not
    // materialize leaves no resource disagreeing with anything.
    //
    // ⛔⛔ AND IT CHECKS THE CUSTODIAN, NOT MERELY THAT SOMEBODY HAS IT. The
    // first version asked only whether the occurrence was in SOME custody, which
    // a restore that handed the banked object to the wrong body would satisfy —
    // and "the reward is back in the hand that banked it" is the contract, not
    // "the reward is in a hand".
    //
    // ⛔ AND THAT ONE ENTITY CARRIES THE IDENTITY. The materialization arm
    // rebuilds occurrences no live entity answers for; a duplicate is the other
    // way it can fail, and two live things behind one identity is a world every
    // later lookup answers differently about depending on iteration order.
    let mut unmet: Vec<String> = Vec::new();
    {
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        // WHO each entity is, by stable identity. The custody marker names a
        // holder by `Entity`, and the baseline names one by `SimId`; comparing
        // them needs this translation and nothing else.
        let named: std::collections::BTreeMap<bevy::prelude::Entity, SimId> = {
            let mut ids = world.query::<(bevy::prelude::Entity, &SimId)>();
            ids.iter(world)
                .map(|(entity, id)| (entity, id.clone()))
                .collect()
        };
        let mut custodians: std::collections::BTreeMap<SimId, Vec<Option<SimId>>> =
            std::collections::BTreeMap::new();
        {
            let mut held = world.query::<(
                &SimId,
                &ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf,
            )>();
            for (occurrence, custody) in held.iter(world) {
                custodians
                    .entry(occurrence.clone())
                    .or_default()
                    .push(named.get(&custody.0).cloned());
            }
        }
        for (occurrence, custodian) in accepted.custody.rows() {
            match custodians.get(occurrence) {
                None => unmet.push(format!(
                    "{} is in nobody's custody",
                    occurrence.as_str()
                )),
                Some(live) if live.len() > 1 => unmet.push(format!(
                    "{} is carried by {} entities at once",
                    occurrence.as_str(),
                    live.len()
                )),
                Some(live) if live[0].as_ref() != Some(custodian) => unmet.push(format!(
                    "{} is carried by {:?} and the checkpoint says {}",
                    occurrence.as_str(),
                    live[0].as_ref().map(|id| id.as_str()),
                    custodian.as_str()
                )),
                Some(_) => {}
            }
        }
    }
    // ── EVERY OCCURRENCE THE PINNED LEDGER PUTS IN THIS ROOM IS HERE ─────────
    //
    // ⭐ COMPLETENESS, which the ledger-equality check above cannot see. That one
    // compares two ledgers; this asks whether the WORLD produced what the ledger
    // describes. A room rebuilt from the right plan that failed to spawn half of
    // it leaves both ledgers agreeing and the player standing in an empty room.
    //
    // ⚠ ONLY THE ROWS THAT NAME THIS ROOM. An occurrence the checkpoint places
    // somewhere else is not this reconstruction's to produce, and one in custody
    // is the custody arm's below.
    {
        use ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts;
        use ambition_platformer2d_shared_tangle::sim_id::SimId;

        let live: std::collections::BTreeSet<SimId> = {
            let mut ids = world.query::<&SimId>();
            ids.iter(world).cloned().collect()
        };
        let missing: Vec<&str> = accepted
            .occurrences
            .remembered()
            .rows()
            .filter(|(_, whereabouts)| {
                matches!(
                    whereabouts,
                    OccurrenceWhereabouts::Placed { room, .. }
                        if room == accepted.intent.target_room()
                )
            })
            .filter(|(occurrence, _)| !live.contains(*occurrence))
            .map(|(occurrence, _)| occurrence.as_str())
            .collect();
        if !missing.is_empty() {
            return Err(RestoreVerificationFailure {
                failure: RestoreFailure::Population,
                detail: format!(
                    "the checkpoint places {} occurrence(s) in '{}' that the \
                     rebuilt room does not contain: {missing:?}",
                    missing.len(),
                    accepted.intent.target_room()
                ),
            });
        }
    }

    if !unmet.is_empty() {
        return Err(RestoreVerificationFailure {
            failure: RestoreFailure::Custody,
            detail: format!(
                "{} banked custody row(s) are not what the restore produced: \
                 {unmet:?}",
                unmet.len()
            ),
        });
    }
    Ok(())
}

/// Which contract a restore broke.
///
/// ⭐ A CLOSED SET, NOT A STRING, because this IS rollback state and a free-form
/// diagnostic is not. Two peers must agree on WHAT failed; the sentence
/// explaining it goes to the log, where a human reads it and no snapshot carries
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreFailure {
    /// The applied ledger is not the population the operation was accepted with.
    OccurrenceLedger,
    /// The applied bag is not the quantities it was accepted with.
    Entitlements,
    /// The session is not standing in the room the operation reconstructs.
    Room,
    /// The body the operation restores around is not in the world it produced.
    Subject,
    /// A banked custody row is missing, duplicated, or in the wrong hands.
    Custody,
    /// The rebuilt room does not contain an occurrence the checkpoint places in
    /// it.
    Population,
}

impl RestoreFailure {
    /// The domain's name, for the log line and for a test's assertion.
    pub fn domain(self) -> &'static str {
        match self {
            Self::OccurrenceLedger => "occurrence ledger",
            Self::Entitlements => "entitlements",
            Self::Room => "room",
            Self::Subject => "subject",
            Self::Custody => "custody",
            Self::Population => "population",
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::OccurrenceLedger => 1,
            Self::Entitlements => 2,
            Self::Room => 3,
            Self::Subject => 4,
            Self::Custody => 5,
            Self::Population => 6,
        }
    }
}

/// The terminal answer for one restore operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointRestoreOutcome {
    /// Applied and verified.
    Committed { key: CheckpointOperationKey },
    /// Applied and did not verify. The world was NOT returned to its previous
    /// state; gameplay is blocked instead.
    Failed {
        key: CheckpointOperationKey,
        failure: RestoreFailure,
    },
}

impl CheckpointRestoreOutcome {
    pub fn key(&self) -> CheckpointOperationKey {
        match self {
            Self::Committed { key } | Self::Failed { key, .. } => *key,
        }
    }

    pub fn committed(&self) -> bool {
        matches!(self, Self::Committed { .. })
    }

    /// The failure, if this outcome is one.
    pub fn failure(&self) -> Option<RestoreFailure> {
        match self {
            Self::Committed { .. } => None,
            Self::Failed { failure, .. } => Some(*failure),
        }
    }

    /// ⛔ EXHAUSTIVE BY DESTRUCTURE. A new field, or a new variant, stops this
    /// compiling rather than falling outside the projection two peers compare.
    fn write_into(&self, bytes: &mut Vec<u8>) {
        use ambition_platformer2d_core::snapshot::put_u8;
        match self {
            Self::Committed { key } => {
                put_u8(bytes, 0);
                key.write_into(bytes);
            }
            Self::Failed { key, failure } => {
                put_u8(bytes, 1);
                key.write_into(bytes);
                put_u8(bytes, failure.code());
            }
        }
    }
}

/// The terminal outcome of the most recent answered operation.
///
/// ⛔ ONE OUTCOME PER OPERATION, AND HERE IS EXACTLY HOW FAR THE STORAGE GOES.
/// This keeps the LATEST outcome, not a history, so `publish` can refuse a
/// second answer for the key it currently holds and nothing more: publishing
/// key 1, then key 2, then key 1 again would be accepted. What actually makes
/// "one outcome per operation" true is that the session is the sole writer and
/// retires the accepted operation as it answers it, so a second answer for one
/// key has no road to travel. The refusal below is a trip-wire on that, not the
/// guarantee itself — and saying so is the difference between a contract and a
/// comment.
///
/// ⚠ A consumer asking "did MY operation finish" names its key and gets `None`
/// once a later one has been answered, which is the honest reply to a consumer
/// that waited too long. There is at most one outstanding operation per session,
/// so that window is a frame or two.
///
/// ⚠ AND PRESENTATION SHOULD NOT POLL THIS. When a terminal notification is
/// wanted, publish a message at completion; a single-latest resource is the
/// session's own bookkeeping, not a feed.
#[derive(bevy::prelude::Resource, Default, Clone, Debug, PartialEq)]
pub struct SessionCheckpointOutcomes(Option<CheckpointRestoreOutcome>);

impl SessionCheckpointOutcomes {
    /// The terminal outcome for `key`, if that is the operation last answered.
    pub fn outcome_for(&self, key: CheckpointOperationKey) -> Option<&CheckpointRestoreOutcome> {
        self.0.as_ref().filter(|outcome| outcome.key() == key)
    }

    /// The most recent terminal outcome, whichever operation it belongs to.
    pub fn latest(&self) -> Option<&CheckpointRestoreOutcome> {
        self.0.as_ref()
    }

    /// Record the terminal outcome, refusing a second answer for the operation
    /// it currently holds.
    ///
    /// ⛔ `pub(crate)`: the session coordinator is the sole writer, and the sole
    /// writer being sole is what makes one-outcome-per-operation true. A public
    /// setter would invite a second author for whom this refusal is the only
    /// defence, and it is not a sufficient one.
    pub(crate) fn publish(&mut self, outcome: CheckpointRestoreOutcome) {
        if self
            .0
            .as_ref()
            .is_some_and(|existing| existing.key() == outcome.key())
        {
            bevy::log::error!(
                target: "ambition_platformer2d::session",
                "a second terminal outcome was published for checkpoint operation \
                 {:?}; keeping the first",
                outcome.key(),
            );
            return;
        }
        self.0 = Some(outcome);
    }

    /// ⭐ WHICH OPERATION, AND WHICH CONTRACT IT BROKE. A rewind that brought
    /// back a `Committed` for an operation the other timeline failed is a
    /// divergence in what the session believes about its own world — and so, one
    /// step finer, is a rewind that agreed on "failed" while disagreeing about
    /// WHERE, because the two timelines then blocked gameplay for different
    /// reasons.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_u8};
        let mut bytes = Vec::new();
        match &self.0 {
            None => put_u8(&mut bytes, 0),
            Some(outcome) => {
                put_u8(&mut bytes, 1);
                outcome.write_into(&mut bytes);
            }
        }
        checksum_bytes(&bytes)
    }
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
        app.init_resource::<SessionCheckpointOperations>();
        app.init_resource::<SessionCheckpointOutcomes>();
        app.init_resource::<SessionStartupResume>();
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
