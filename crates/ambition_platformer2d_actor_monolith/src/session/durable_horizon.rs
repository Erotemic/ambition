//! Durable persistence for occurrence, custody, and minted-item checkpoint state.
//!
//! The on-disk representation stores domain descriptions, not ECS component snapshots:
//! authored occurrence whereabouts, custody links, and dynamic-mint identity/provenance/spec ids.
//! Loading installs those baselines and triggers the ordinary checkpoint-reset path, so the
//! checkpoint restore remains the single reconstruction authority.
//!
//! Placed runtime mints can be recreated from their saved description. In-flight mints that were
//! never admitted to the occurrence ledger are outside this horizon. `OwnedItems` remains a
//! quantity table; consumed occurrences round-trip but currently have no live producer.

use std::collections::BTreeMap;

use bevy::prelude::*;

pub use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{
    PersistedCustody, PersistedOccurrence, PersistedWhereabouts,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_custody_rows, AuthoredOccurrences, CustodyBaseline, CustodyDurability, InCustodyOf,
    OccurrenceBaseline,
    OccurrenceWhereabouts, ResetToCheckpoint, RoomScopedEntity,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Whether the loaded save has been applied to this world.
///
/// This is the single durable-restore latch across inventory and occurrence
/// domains. It gates behavior and is therefore rollback state, not a cache.
#[derive(Resource, Default, Clone)]
pub struct SaveRestored(pub bool);

/// Adopt the lifecycle/occurrence domain from a loaded file.
///
/// This is deliberately one domain adapter rather than one parameter per
/// checkpoint baseline in a global census. Item-domain baselines are adopted by
/// `items::persist::restore_inventory_from_save`, beside the item state the file
/// actually restores.
pub fn adopt_occurrence_checkpoint_from_save(
    restored: Res<SaveRestored>,
    save: Res<AmbitionGameSave>,
    bodies: Query<
        &ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    // ⛔ THE POPULATION IS "EXACTLY ONE", NOT "AT LEAST ONE", AND THE SPELLING
    // IS `complete_durable_restore`'S ON PURPOSE. The completer raises the
    // latch only for one primary body with a wallet. A wider guard here adopts
    // the ledger on a population where the latch can never rise, so the write
    // repeats on every tick and overwrites the baselines the checkpoint commit
    // keeps. Held by
    // `a_population_the_restore_cannot_complete_on_is_written_to_by_nobody`.
    if restored.0 || bodies.single().is_err() {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    adopt_the_ledger(
        save.data(),
        occurrences,
        occurrence_baseline,
        custody_baseline,
    );
}

/// ⛔⛤ **THE LEDGER HAS ONE ADOPTION ROAD FOR A SESSION BEING BUILT, AND IT IS
/// THE CANDIDATE'S HORIZON.** There used to be a second:
/// `adopt_the_occurrence_ledger_at_activation`, a system on
/// `SessionScopeActivated` that re-read `AmbitionGameSave` into
/// `AuthoredOccurrences`, `OccurrenceBaseline` and `CustodyBaseline` process-wide.
/// It was the A10.5-era answer to a load that constructed its first room knowing
/// nothing; preparing the candidate before the route activates moved that moment
/// EARLIER still, and left the system writing the same three resources from the
/// same file one system before [`CandidateDurableHorizon::install`] wrote them
/// again.
///
/// ⚠ MEASURED 2026-09-15 over the shipped `app_it` composition: **639 gameplay
/// activations, 639 of them ran both writers, and the values agreed in every
/// one** (`save_rows` equalled the installed row count, 637 x 0 and 2 x 1). They
/// agreed because both read the same resource in the same frame — which is how a
/// second authority hides. The order was `reset_session_scoped_resources_on_activation`
/// -> the system -> `install`, uniformly, so the candidate's value won by
/// SCHEDULE ACCIDENT rather than by any edge. A save swapped between preparing
/// the candidate and publishing it, or a schedule that ever put those two the
/// other way round, would have silently made the live file authoritative over
/// the horizon the candidate was validated against.
///
/// ⭐ THE SAME MEASUREMENT FOUND ZERO UNPREPARED ACTIVATIONS — the shell's
/// `None => active_scope.begin()` branch, the road for an activation nobody
/// prepared a candidate for, was taken 0 times in 639 while the other two probes
/// fired 639 times each, so the zero is the instrument working rather than a
/// build that never ran. A mid-session load still has
/// [`adopt_occurrence_checkpoint_from_save`] above, which is a different trigger
/// (the `SaveRestored` latch) rather than a second answer to this question.
///
/// The save that a candidate session is built from: the save of ITS
/// experience.
///
/// ⛔ NOT THE LIVE SAVE. A candidate is built hidden, before its route is
/// activated, and the activation is what gives the live save to its
/// experience (`ambition_persistence::save::hand_the_save_to`). Until
/// 2026-10-04 the builder read `AmbitionGameSave`, so a candidate that was
/// prepared while another experience played took the durable horizon of that
/// experience. Measured in `app_it`
/// (`a_session_prepared_while_another_experience_plays_is_built_from_its_own_save`):
/// an Ambition session that replaced a Sanic session had, for its first 3
/// frames, an item that Ambition's save says is gone for good. A fresh host
/// with the same save never has it.
///
/// ⭐ THE VALUE IS THE ONE THE ACTIVATION HANDS OVER. Persistence puts the
/// prepared save aside for its experience and changes no ownership, so a
/// refused candidate leaves the live save with the session that plays.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CandidateSave<'w> {
    /// Absent in a composition with no durable horizon. An empty horizon is
    /// the answer there.
    live: Option<Res<'w, AmbitionGameSave>>,
    /// Absent in an App that hosts one experience: the live save is its save.
    ownership: Option<ResMut<'w, ambition_persistence::save::SaveOwner>>,
    /// Absent in an App that persists nothing: no file is read.
    root: Option<Res<'w, ambition_persistence::PersistenceRoot>>,
}

impl CandidateSave<'_> {
    /// The durable horizon of a candidate session of `experience`, and the
    /// save value it was read from.
    pub fn horizon_of(&mut self, experience: &str) -> (CandidateDurableHorizon, PreparedFromSave) {
        let Some(live) = self.live.as_deref() else {
            return (CandidateDurableHorizon::default(), PreparedFromSave(None));
        };
        let save = match self.ownership.as_deref_mut() {
            None => live.data(),
            Some(ownership) => ambition_persistence::save::prepare_the_save_of(
                experience,
                ownership,
                live,
                self.root.as_deref().map(|root| root.0.as_path()),
            ),
        };
        (CandidateDurableHorizon::from_save(save), PreparedFromSave(Some(save.clone())))
    }
}

/// The save value that a candidate session was built from.
///
/// A candidate of the experience that plays (a restart) is built from the
/// live save, and the session that plays can change that save before the
/// candidate is adopted. The adoption gives the save to nobody, because the
/// experience has it. Measured 2026-10-05 in `app_it`
/// (`a_session_that_replaces_its_own_experience_is_built_from_the_save_at_its_adoption`):
/// the adopted world had an item that the live save said was gone for good.
///
/// ⛔ THE CANDIDATE IS NOT REPAIRED AND THE EARLIER SAVE IS NOT PUT BACK. A
/// candidate whose save changed is stale. Its builder discards it and builds
/// a new one from the save as it is ([`Self::is_current`]).
///
/// The whole save is compared, not only the part that construction reads: the
/// save has no field that changes each frame, and a rule that names fields
/// goes out of date when construction reads one more.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreparedFromSave(Option<ambition_persistence::save_data::AmbitionGameSaveData>);

impl PreparedFromSave {
    /// A candidate built from `save`. `None`: a composition with no save.
    pub fn of(save: Option<ambition_persistence::save_data::AmbitionGameSaveData>) -> Self {
        Self(save)
    }

    /// Does `experience` get this same value if it is activated now?
    pub fn is_current(&self, world: &World, experience: &str) -> bool {
        let now = world.get_resource::<AmbitionGameSave>().and_then(|live| {
            match world.get_resource::<ambition_persistence::save::SaveOwner>() {
                None => Some(live.data()),
                Some(ownership) => {
                    ambition_persistence::save::the_save_of(experience, ownership, live)
                }
            }
        });
        self.0.as_ref() == now
    }
}

/// A candidate session's durable horizon, held as a VALUE.
///
/// ⛔⛤ **REVIEW FINDING 1, 2026-09-15: PREPARING A CANDIDATE MUST NOT WRITE THE
/// LIVE SESSION'S CHECKPOINT STATE.** `adopt_the_ledger_for_a_pending_candidate`
/// installed `AuthoredOccurrences`, `OccurrenceBaseline` and `CustodyBaseline`
/// process-wide while the OUTGOING session was still the live one — and the two
/// baselines are rollback-authoritative checkpoint state, deliberately NOT equal
/// to the current save/live projection. A candidate that then refused left A
/// playable with different death semantics than it had a moment before, which is
/// precisely what the last-good-world invariant forbids.
///
/// ⇒ **THE SAME SHAPE `SessionMechanics` AND `MovingPlatformSet` ALREADY USE:**
/// built from the save as a value, carried on the candidate, and installed by
/// ADOPTION. A refused candidate drops it and A's resources were never touched.
/// ⛔ Deliberately NOT another process-global `PendingFoo` mirror.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandidateDurableHorizon {
    occurrences: AuthoredOccurrences,
    custody: BTreeMap<SimId, SimId>,
    /// ⛔⛤ **THE SECOND DESCRIPTOR CANDIDATE CONSTRUCTION NEEDS — REVIEW FINDING
    /// 2.** A ledger row says WHERE a runtime-minted occurrence is; only this
    /// says WHAT it is. Carrying the first without the second made the candidate
    /// plan from B's whereabouts and A's descriptions.
    ///
    /// ⚠ **THE RULE IS NARROW, deliberately**: every durable value consumed in
    /// deciding whether candidate B's world can be CONSTRUCTED comes from B's
    /// horizon. `OwnedItemsBaseline`, the wallet and the rest do not participate
    /// in candidate room construction and are not pulled in here.
    minted: crate::items::pickup::minted_horizon::MintedItemBaseline,
    /// What the save says about the bodies of the first room (dead, provoked,
    /// cleared). The third value that construction reads from a save. It is
    /// not installed: the first room commit reads it and no resource holds it.
    fates: crate::construction::PersistedFates,
}

impl CandidateDurableHorizon {
    /// Read the save into a value. Touches no resource.
    ///
    /// ⛔ THE SAVE OF THE CANDIDATE'S EXPERIENCE, which is not the live save
    /// while another experience plays. [`CandidateSave::horizon_of`] is the
    /// production caller.
    pub fn from_save(save: &ambition_persistence::save_data::AmbitionGameSaveData) -> Self {
        let (rows, custody, mints) = ledger_from_save(save);
        let mut occurrences = AuthoredOccurrences::default();
        occurrences.adopt_rows(rows);
        occurrences.adopt_mints(mints);
        Self {
            occurrences,
            custody,
            minted: crate::items::pickup::minted_horizon::minted_baseline_from_save(save),
            fates: crate::construction::PersistedFates::from_save(save),
        }
    }

    /// What the commit of the candidate's first room reads for the fates of
    /// its bodies: the candidate's own save. No occurrence is scheduled to
    /// return, because the world-time schedule of a new session is empty.
    pub fn first_room_facts(&self) -> crate::construction::CommitFactsSource {
        crate::construction::CommitFactsSource::Stated(self.fates.clone())
    }

    /// What the candidate's construction reads to rebuild a runtime mint.
    pub fn minted(&self) -> &crate::items::pickup::minted_horizon::MintedItemBaseline {
        &self.minted
    }

    /// What the candidate's construction reads for `OccurrenceContinuity` —
    /// the candidate's own ledger, never the live session's.
    pub fn occurrences(&self) -> &AuthoredOccurrences {
        &self.occurrences
    }

    /// Make this horizon authoritative. Called by ADOPTION and by nothing else.
    pub fn install(self, world: &mut bevy::prelude::World) {
        let Self {
            occurrences,
            custody,
            minted,
            fates: _,
        } = self;
        if let Some(mut live) = world.get_resource_mut::<AuthoredOccurrences>() {
            *live = occurrences.clone();
        }
        if let Some(mut baseline) = world.get_resource_mut::<OccurrenceBaseline>() {
            baseline.adopt(occurrences);
        }
        if let Some(mut baseline) = world.get_resource_mut::<CustodyBaseline>() {
            baseline.adopt(custody);
        }
        if let Some(mut baseline) = world
            .get_resource_mut::<crate::items::pickup::minted_horizon::MintedItemBaseline>()
        {
            *baseline = minted;
        }
    }
}

/// The save's two ledgers, as plain maps. Pure: the half of `adopt_the_ledger`
/// that reads, split out so a candidate can have the value without the write.
fn ledger_from_save(
    data: &ambition_persistence::save_data::AmbitionGameSaveData,
) -> (
    BTreeMap<SimId, OccurrenceWhereabouts>,
    BTreeMap<SimId, SimId>,
    Vec<SimId>,
) {
    let ledger_rows: BTreeMap<SimId, OccurrenceWhereabouts> = data
        .occurrences()
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.id.clone()),
                match &row.whereabouts {
                    PersistedWhereabouts::InCustody => OccurrenceWhereabouts::InCustody,
                    PersistedWhereabouts::Placed { room, x, y } => OccurrenceWhereabouts::Placed {
                        room: room.clone(),
                        at: Vec2::new(*x as f32, *y as f32),
                    },
                    PersistedWhereabouts::Consumed => OccurrenceWhereabouts::Consumed,
                    PersistedWhereabouts::Spent => OccurrenceWhereabouts::Spent,
                },
            )
        })
        .collect();
    let held: BTreeMap<SimId, SimId> = data
        .custody()
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.occurrence.clone()),
                SimId::from_snapshot(row.custodian.clone()),
            )
        })
        .collect();

    // The rows the file marks as runtime mints (provenance, beside the row).
    let mints: Vec<SimId> = data
        .occurrences()
        .iter()
        .filter(|row| row.mint)
        .map(|row| SimId::from_snapshot(row.id.clone()))
        .collect();

    (ledger_rows, held, mints)
}

fn adopt_the_ledger(
    data: &ambition_persistence::save_data::AmbitionGameSaveData,
    mut occurrences: ResMut<AuthoredOccurrences>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    let (ledger_rows, held, mints) = ledger_from_save(data);
    occurrences.adopt_rows(ledger_rows);
    occurrences.adopt_mints(mints);
    if let Some(mut baseline) = occurrence_baseline {
        baseline.adopt(occurrences.clone());
    }
    if let Some(mut baseline) = custody_baseline {
        baseline.adopt(held);
    }
}

/// Mark durable adoption complete and request the ordinary checkpoint resume.
///
/// This is the only global completion point. Domain adopters run before it and
/// touch only their own state; this system states that every adopter in the
/// chain has had its turn. Keeping the request here prevents an item or lifecycle
/// domain from becoming the coordinator for its siblings.
pub fn complete_durable_restore(
    mut restored: ResMut<SaveRestored>,
    save: Res<AmbitionGameSave>,
    ready_body: Query<
        &ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut resets: MessageWriter<ResetToCheckpoint>,
) {
    if restored.0 || ready_body.single().is_err() {
        return;
    }
    restored.0 = true;
    let data = save.data();
    if !data.occurrences().is_empty()
        || !data.custody().is_empty()
        || !data.minted_items().is_empty()
    {
        resets.write(ResetToCheckpoint);
    }
}

/// Where a domain's durable adapters run in the simulation. A domain installs
/// its own systems here; this module orders the slots.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DurableHorizonSet {
    /// Other domains' live → save mirrors. Each domain chains its own members:
    /// they all take `ResMut<AmbitionGameSave>`.
    DomainMirror,
    /// This module's live → save mirrors, after every domain's.
    SessionMirror,
}

/// Where a loaded file is applied, in the simulation schedule.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DurableRestoreSet {
    /// The lifecycle/occurrence baseline, which the domains' restores build on.
    Lifecycle,
    /// Each domain applies its half of the file and adopts its own baselines.
    Domains,
    /// The file is fully applied; a checkpoint resume may now be requested.
    Complete,
}

/// Install the session's durable-save application/mirroring chain and the
/// slots other domains' adapters join. The generic runtime composes this with
/// each domain's own installation.
pub fn install_durable_save_horizon(app: &mut App) {
    // ⭐ A NEW GAME'S FRESH-RUN STATE IS NOT RESET HERE. A New Game is a
    // checkpoint restore to the fresh baseline, so each domain's fresh-run
    // reducer runs in `CheckpointDomainApply`, on the confirmed-frame commit.
    // The mirrors below then copy the fresh live state into the wiped save.
    let sim = app.sim_schedule();
    // ⛔⛤ **THE LIVE→SAVE MIRRORS CROSS THE SAME ROLLBACK BOUNDARY AS THE STATE
    // THEY MIRROR.** They ran in top-level `Update` — once per FRAME — while
    // `AmbitionGameSave` is `rollback_resource_clone_checksum` and is compared
    // once per TICK. A rewind re-simulates ticks and does not replay `Update`, so
    // the same historical frame saw a different save: measured, 1 of 364 probed
    // checksum entries diverged and it was this one, with the replay xor CONSTANT
    // across frames 2, 3 and 4 while the first-pass xor moved every frame.
    //
    // ⚠ NO DISK I/O MOVES HERE. These three derive the save RESOURCE from live
    // simulation state and nothing else; autosave and the file write stay outside
    // the simulation. The layering is rollback-owned durable representation →
    // persistence projection → disk, and only the first hop is in this schedule.
    //
    // ⚠ `.chain()` is load-bearing rather than tidy: all three take
    // `ResMut<AmbitionGameSave>`, so an unordered set would leave their relative
    // order ambiguous, and an ambiguous order inside a rewinding schedule is
    // nondeterminism the checksum would then report as a desync.
    //
    // ⭐ AND THE VISIT COUNTER JOINS THEM, for the `ResMut<AmbitionGameSave>`
    // reason the `.chain()` note above gives. Its `.after` edge is the one thing
    // it needs beyond theirs: it fires on the tick a conversation OPENED, so
    // running before the opening would mean the edge is gone by the next tick and
    // the visit is never counted at all.
    //
    // ⛔ AND AFTER THE ITEM RESIDENCY CHAIN. The custody rows are read from
    // `InCustodyOf`, which is derived: a rollback load does not restore it, and
    // `project_custody_onto_residency` inserts it again through `Commands` each
    // tick. A mirror that ran before that projection read the value the latest
    // forward frame left, not the value of the frame being resimulated, and the
    // sync test saw the save's custody rows diverge after a release
    // (SAVE-DIVERGES-AFTER-RELEASE). The edge also brings the command flush.
    app.configure_sets(
        sim,
        (DurableHorizonSet::DomainMirror, DurableHorizonSet::SessionMirror)
            .chain()
            .after(ambition_platformer2d_shared_tangle::schedule::ResidencyStep::Project),
    );
    app.add_systems(
        sim,
        (
            persist_occurrence_horizon_to_save,
            count_the_dialogue_visit_when_a_conversation_opens
                .after(crate::features::interact_ecs_actors_and_switches),
        )
            .chain()
            .in_set(DurableHorizonSet::SessionMirror),
    );
    // ⭐ THE SAVE IS APPLIED BY THE SIMULATION (BODY-BORN-ON-THE-TIMELINE,
    // 2026-10-03). The save, the latch and every value the chain writes are
    // rollback state, so a step that applies the file is an ordinary
    // deterministic step: a rewind past it restores the unapplied save and the
    // resimulation applies it again, on the same tick, to the same values. Two
    // peers with the same file apply it on the same tick, whether the body was
    // built before the timeline started or by it.
    //
    // ⛔ IT USED TO RUN IN `Update`, and that needed three guards to be safe:
    // a session-start gate (`Q135`) that waited for hydration, a check that no
    // save was applied over a live timeline, and a declared exception for the
    // compositions whose body is born on the timeline (where the gate had
    // nothing to wait for). All three are gone with the window.
    //
    // ⚠ AT THE HEAD OF THE GAMEPLAY ROOT, after the clock names the tick and
    // before the core step, so a conversation that opens on the first tick a
    // body exists finds the save applied (the visit counter's `!restored` guard
    // has nothing to drop).
    app.init_resource::<SaveRestored>()
        .configure_sets(
            sim,
            (
                DurableRestoreSet::Lifecycle,
                DurableRestoreSet::Domains,
                DurableRestoreSet::Complete,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(ambition_platformer2d_shared_tangle::schedule::SimClockHead)
                .before(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::CoreSimulation,
                ),
        )
        .add_systems(
            sim,
            (
                // Lifecycle state first: the room/custody baseline must be present
                // before the load asks the ordinary checkpoint-resume road to act.
                adopt_occurrence_checkpoint_from_save.in_set(DurableRestoreSet::Lifecycle),
                // The completion point comes last: only now is the file fully
                // applied, and only now may a checkpoint resume be requested.
                complete_durable_restore.in_set(DurableRestoreSet::Complete),
            )
                .chain(),
        );
}

/// Count a dialogue visit on the tick its conversation opened. (sim)
///
/// ⛔⛤ THIS REPLACES AN INCREMENT IN `dispatch_pending_dialog_requests`, WHICH
/// RAN IN TOP-LEVEL `Update` AND LOST THE VISIT ON EVERY REWIND. Measured: an
/// `Update` write to this save is taken back by the restore, while the same
/// increment inside this schedule lands exactly once per tick across every
/// replay of that tick — `a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind`
/// and `an_increment_inside_the_tick_is_made_idempotent_by_the_restore` in
/// `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`.
///
/// ⭐ THE RESTORE IS WHAT MAKES AN INCREMENT SAFE HERE, and that is the
/// non-obvious part. A resimulated tick does not add to the value the previous
/// run left: the snapshot puts the save back to its state before the tick, so
/// every replay adds one to the same base. The sibling mirrors above converge
/// because they DERIVE the save from sim state; this one converges for a
/// different reason, and needs no derivation.
///
/// ⚠ THE EDGE IS `opened_at == now`, NOT CHANGE DETECTION. A restore marks a
/// rollback-registered resource changed, so `Res::is_changed` fires every frame
/// under GGRS. The instance's opening tick is a pure function of rollback state
/// and names exactly one tick, so the same tick replayed re-fires it and no
/// other tick does.
///
/// ⚠ NO TIMELINE, NO VISIT. Without `SimTick` every frame would match
/// `opened_at == 0` and the count would climb forever. That is the degenerate
/// clock `ConversationInstanceId::opened_at` documents rather than a case to
/// support: a composition that cannot tell two visits apart has no visit edge.
/// Every shipped composition has the clock.
pub fn count_the_dialogue_visit_when_a_conversation_opens(
    // `Option` because a composition may install the save without the
    // conversation domain, or the clock without either.
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    tick: Option<Res<ambition_time::SimTick>>,
    restored: Res<SaveRestored>,
    save: Option<ResMut<AmbitionGameSave>>,
) {
    // The siblings' guard, for the siblings' reason: before the latch the save is
    // still being applied from the file, and a visit written into it now is
    // written into a value the load is about to replace.
    if !restored.0 {
        return;
    }
    let (Some(conversation), Some(tick), Some(mut save)) = (conversation, tick, save) else {
        return;
    };
    let Some(live) = conversation.live() else {
        return;
    };
    if live.opened_at() != tick.0 {
        return;
    }
    save.data_mut().increment_dialog_visit(live.dialogue_id());
}

/// Mirror the current occurrence horizon into the save after restore completes. Writes are
/// value-compared so an unchanged horizon does not retrigger autosave.
///
/// ⭐ THE LEDGER ROWS ARE MIRRORED ONLY WHEN SOMETHING THEY DEPEND ON CHANGED
/// (FI9). The rows are a function of the ledger, the save they are compared
/// with, and the restorable set. When none of them changed since this system
/// last ran, the rows in the save are still right, and the walk over every
/// row (each dormant `Placed` row of every room that is not live) is skipped.
/// Measured before the gate: 6.1 ms a tick with 10,000 dormant rows, 65 µs with
/// none. The live custody rows are read from live bodies only, so they are
/// compared every tick.
///
/// ⛔ THE GATE ASKS WHETHER THE INPUTS ARE THE SAME VALUES, NOT WHETHER THEY WERE
/// WRITTEN (M2 cut C). It asked `is_changed()` of the ledger, the save and
/// `SaveRestored`. A rollback load writes all three on every resimulated
/// frame, so under a rollback host the gate opened on every frame and walked
/// every row: 10 ms a tick with 10,000 dormant rows in the sync test. The key
/// ([`OccurrenceMirrorKey`]) holds the ledger's shared rows and the save's
/// shared rows, so a load of unchanged rows matches it by allocation, and a
/// load of other rows does not.
///
/// Persist a custody relationship only when its owning domain can reconstruct that custody after
/// process restart. `ItemCustody` qualifies; transient body possession does not. Other occurrence
/// states cross the durable horizon directly.
#[allow(clippy::too_many_arguments)]
pub fn persist_occurrence_horizon_to_save(
    restored: Res<SaveRestored>,
    occurrences: Option<Res<AuthoredOccurrences>>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    // The occurrences whose custody survives a process boundary, because the item
    // domain saves `ItemCustody` and applies it again on load.
    //
    // ⚠ THIS MATCHES THE COMPONENT, NOT THE `Held` VARIANT, and the two readings differ:
    // `ItemCustody` is an enum with `InWorld` as well as `Held { holder }`, so this set
    // also contains items lying on the ground. That is WIDER than the filter comment
    // below claims ("a hand it can reconstruct"), and wider is the safe direction — it
    // drops FEWER occurrence rows, so it cannot strand a `custody` or `minted_items` row
    // whose occurrence went missing. ⇒ Recorded rather than tightened: narrowing this to
    // `Held` would change what the save omits, which is a durability decision and not a
    // tidy-up. See `docs/planning/engine/item-custody-and-accounting.md`.
    durably_held: Query<&SimId, With<ambition_held_items::ItemCustody>>,
    mut save: ResMut<AmbitionGameSave>,
    // The inputs the rows were last mirrored with: a cache of live inputs,
    // not state. A rewind that changes one is seen as a difference.
    mut mirrored_with: Local<Option<OccurrenceMirrorKey>>,
) {
    if !restored.0 {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    let restorable_now: std::collections::BTreeSet<SimId> = durably_held.iter().cloned().collect();
    let key_now = OccurrenceMirrorKey {
        ledger: occurrences.clone(),
        save_rows: save.data().occurrences_shared().clone(),
        restorable: restorable_now,
    };
    let rows_may_differ = mirrored_with.as_ref() != Some(&key_now);
    let rows: Option<Vec<PersistedOccurrence>> = rows_may_differ.then(|| {
        let restorable: std::collections::BTreeSet<&str> =
            key_now.restorable.iter().map(SimId::as_str).collect();
        occurrences
            .rows()
            // An `InCustody` row is a claim that something is holding this, and the
            // file may only make that claim about a hand it can reconstruct. Every
            // other whereabouts — `Placed`, `Consumed` — is a fact about the world
            // itself and crosses unconditionally.
            .filter(|(sim_id, whereabouts)| {
                !matches!(whereabouts, OccurrenceWhereabouts::InCustody)
                    || restorable.contains(sim_id.as_str())
            })
            .map(|(sim_id, whereabouts)| {
                PersistedOccurrence::new(
                    sim_id.as_str(),
                    match whereabouts {
                        OccurrenceWhereabouts::InCustody => PersistedWhereabouts::InCustody,
                        OccurrenceWhereabouts::Placed { room, at } => PersistedWhereabouts::Placed {
                            room: room.clone(),
                            // INTEGER pixels — see `PersistedWhereabouts::Placed`.
                            // A float would cost the save's `Eq` and make a NaN
                            // rewrite the file every frame forever.
                            x: at.x.round() as i32,
                            y: at.y.round() as i32,
                        },
                        OccurrenceWhereabouts::Consumed => PersistedWhereabouts::Consumed,
                        OccurrenceWhereabouts::Spent => PersistedWhereabouts::Spent,
                    },
                )
                .minted(occurrences.is_mint(sim_id))
            })
            .collect()
    });
    // ⭐⭐ **THE LIVE CUSTODY ROWS ASK THE RELATION, NOT THE SUBJECT'S DOMAIN.**
    // `InCustodyOf::durability` is stated by whichever producer wrote the row —
    // `Restored` for an item in a hand, `SessionOnly` for a rider, a limb or a
    // possession. This was `restorable`, i.e. *"does the subject carry
    // `ambition_held_items::ItemCustody`"*, which made a THIRD producer
    // non-durable by default and silently: the accepted-control writer map named
    // that risk in writing, and the field is the answer to it.
    //
    // ⚠ THE OCCURRENCE FILTER ABOVE STILL ASKS `restorable`, AND THE TWO
    // POPULATIONS ARE NOT THE SAME ONE. That filter runs over
    // `AuthoredOccurrences` ROWS, not over live entities, so an occurrence
    // recorded `InCustody` whose entity carries no `InCustodyOf` — an
    // inconsistent state, but one this function must not make worse — is kept by
    // the wider marker and would be DROPPED by the field. Dropping a save row is
    // the dangerous direction; see this parameter's own note.
    let durable: std::collections::BTreeSet<&str> = carried
        .iter()
        .filter(|(_, custody)| custody.durability == CustodyDurability::Restored)
        .map(|(sim_id, _)| sim_id.as_str())
        .collect();
    let custody: Vec<PersistedCustody> = live_custody_rows(&carried, &custodians)
        .into_iter()
        .filter(|(occurrence, _)| durable.contains(occurrence.as_str()))
        .map(|(occurrence, custodian)| {
            PersistedCustody::new(occurrence.as_str(), custodian.as_str())
        })
        .collect();
    let data = save.data();
    let rows_differ = rows.as_ref().is_some_and(|rows| data.occurrences() != rows.as_slice());
    if rows_differ || data.custody() != custody {
        let rows = rows.unwrap_or_else(|| data.occurrences().to_vec());
        // ⛔ `data_mut()` ONLY PAST THE GUARD ABOVE. Reaching it derefs the
        // `ResMut`, which marks the resource changed whether or not the value
        // differs -- that is what the guard protects.
        save.data_mut().set_durable_horizon(rows, custody);
    }
    // The key holds the save's rows as they are now, after the write, so the
    // write does not open the gate on the next tick.
    *mirrored_with = Some(OccurrenceMirrorKey {
        save_rows: save.data().occurrences_shared().clone(),
        ..key_now
    });
}

/// The inputs the save's occurrence rows were last mirrored with: the
/// ledger, the save's rows, and the restorable set. The ledger and the save's
/// rows are shared allocations, so a match is by allocation first and costs
/// nothing while they are unchanged.
pub struct OccurrenceMirrorKey {
    ledger: AuthoredOccurrences,
    save_rows: std::sync::Arc<Vec<PersistedOccurrence>>,
    restorable: std::collections::BTreeSet<SimId>,
}

impl PartialEq for OccurrenceMirrorKey {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.save_rows, &other.save_rows)
            && self.ledger == other.ledger
            && self.restorable == other.restorable
    }
}

#[cfg(test)]
mod tests;
