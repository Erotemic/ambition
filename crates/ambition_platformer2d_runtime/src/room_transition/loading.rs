//! Readiness-gated ordinary room transitions.
//!
//! It first becomes an exact [`ambition_load`] transaction, preflights the target without
//! mutating room authority, and may commit only on a later simulation tick after the required
//! barrier is ready and one-shot authorization succeeds.
//!
//! The transition now carries both construction preflight and concrete room presentation
//! readiness.

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::{DetectChanges, MessageWriter, NextState, Res, ResMut, Resource};

use ambition_load::{
    BarrierReadiness, LoadBarrierRef, LoadBarrierSpec, LoadCommitRejection, LoadCoordinator,
    LoadEvent, LoadFailure, LoadId, LoadPlanSpec, LoadWorkId, LoadWorkSpec, LoadWorkState,
};
use ambition_platformer2d_actor_monolith::rooms;
use ambition_platformer2d_world::rooms as world_rooms;

use ambition_platformer2d_actor_monolith::session::lifecycle_commit::LifecycleIntent;
use ambition_time::SimTick;

const ROOM_READY_BARRIER: &str = "room-transition.ready";
const TARGET_LOOKUP_WORK: &str = "room-transition.target-lookup";
const ARRIVAL_VALIDATION_WORK: &str = "room-transition.arrival-validation";
const CONSTRUCTION_PREFLIGHT_WORK: &str = "room-transition.construction-preflight";
const ROOM_ASSET_WORK_PREFIX: &str = "room-transition.assets";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoomTransitionLoadPhase {
    AwaitingReadiness,
    CommitAuthorized,
    Committed,
    Failed,
}

/// Marker installed only by presentation-capable hosts.
///
/// When absent, a headless transition may commit as soon as readiness is authorized.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RoomTransitionPresentationAvailable;

/// Marker installed by a host that answers the room-asset readiness contributor.
///
/// The barrier always DECLARES the asset work item, because "did the destination
/// room's art arrive" is a real question about a room transition whatever host
/// asks it. What differs is who can answer: a host with a sprite catalog, an
/// asset server, and a resolved visual quality builds the target room's manifest
/// and reports settled/failed against it; a headless or minimal host cannot, and
/// its work item is honestly `Skipped` rather than silently pretending an asset
/// loaded.
///
/// Present  the engine leaves the item `Running` and a host system
/// (`ambition_app`'s `contribute_room_transition_assets`) owns resolving it.
/// Absent  the engine skips it as it always did for headless.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RoomTransitionAssetContributor;

/// Monotonic identity for the App-local inputs that define room construction.
///
/// This is the room-transition consumer of the broader immutable-content epoch:
/// any change to room data, lowering, stagers, or actor catalogs invalidates
/// prefetched plans and prevents an in-flight plan from committing stale content.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RoomTransitionContentEpoch {
    value: u64,
}

impl RoomTransitionContentEpoch {
    pub fn get(&self) -> u64 {
        self.value
    }

    /// Invalidate every in-flight and prefetched plan.
    ///
    /// The engine advances this for the construction inputs it owns. A host with
    /// construction inputs of its own — Ambition's asset catalog and resolved
    /// visual quality, which decide what a committed room DRAWS — calls this
    /// when those change, instead of the engine naming them. Same epoch, one
    /// meaning: "anything a prepared plan assumed may no longer hold".
    pub fn bump(&mut self) {
        self.value = self.value.wrapping_add(1).max(1);
    }
}

/// Advance the transition content epoch when any construction input changes.
///
/// THE ROOM SET IS COMPARED BY VALUE, NOT BY `is_changed()`, AND THAT IS
/// NOT A STYLE CHOICE. `RoomSet` is rollback-registered state, and a rollback
/// host RESTORES it every frame — so `is_changed()` is TRUE every frame there,
/// whatever the content is doing. This system's write is not idempotent (a bump
/// is a bump), which is exactly the *"change ticks don't rewind"* trap.
///
/// It was invisible until, because the rollback host opened no readiness transactions at all.
///
/// The registries below are ordinary content resources, are not restored, and
/// keep their change-tick reads. The room set is compared against the ids it
/// last had: a restore reproduces them exactly and bumps nothing, while a
/// hot-reload that changes the world's rooms still bumps.
pub fn advance_room_transition_content_epoch_system(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<world_rooms::RoomSet>,
    placement_lowering: Res<
        ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry,
    >,
    content_staging: Res<
        ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry,
    >,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    boss_catalog: Res<ambition_boss_encounter::BossCatalog>,
    mut epoch: ResMut<RoomTransitionContentEpoch>,
    // The room ids this last saw. Allocation-free on the steady path: the
    // comparison walks the two lists and only clones when they genuinely differ.
    mut last_rooms: bevy::prelude::Local<Vec<String>>,
) {
    let rooms_changed = last_rooms.len() != room_set.rooms.len()
        || last_rooms
            .iter()
            .zip(room_set.rooms.iter())
            .any(|(had, now)| had != &now.id);
    if rooms_changed {
        last_rooms.clear();
        last_rooms.extend(room_set.rooms.iter().map(|room| room.id.clone()));
    }
    if rooms_changed
        || placement_lowering.is_changed()
        || content_staging.is_changed()
        || character_catalog.is_changed()
        || boss_catalog.is_changed()
    {
        epoch.bump();
    }
}

#[derive(Clone, Debug)]
pub struct ActiveRoomTransitionLoad {
    pub sequence: u64,
    pub content_epoch: u64,
    pub session_scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    pub source_room: usize,
    pub source_room_id: String,
    /// The destination's INDEX in the live `RoomSet`, resolved once from
    /// [`Self::intent`]'s authored room id. Host-side only — the intent names the
    /// room by id because it is rollback state and an index is not stable across
    /// a content reload.
    pub target_room: usize,
    /// What this transaction is FOR — the semantic crossing, subject
    /// included, exactly as detection recorded it.
    ///
    /// this was a `RoomTransitionRequested` message. The
    /// message described the same crossing a second time, with a resolved zone
    /// and no subject, and only an eager host ever produced one — which is why
    /// the shipped rollback host opened no transaction at all and every room
    /// change in the game went uncovered.
    /// ⭐ THE WHOLE LIFECYCLE INTENT, not only a crossing. A replay in a
    /// composition with no controlled body records
    /// `LifecycleIntent::ReconstituteRoom`, which has no subject and no arrival
    /// and therefore cannot be a `RoomTransitionIntent` — see the accessors
    /// (`target_room`, `subject`, `arrival`, `edge_exit`) this file reads it
    /// through, each of which answers for both shapes.
    pub intent: LifecycleIntent,
    pub construction_plan: Option<Arc<rooms::RoomConstructionPlan>>,
    pub barrier: LoadBarrierRef,
    pub commit_not_before_tick: u64,
    pub cover_required: bool,
    pub cover_presented: bool,
    pub phase: RoomTransitionLoadPhase,
    pub failure: Option<String>,
    pub asset_work_id: LoadWorkId,
    /// The actor names this transition's plan stages, published for the host's
    /// asset contributor so it does not become a second authority on them.
    pub staged_actor_names: Vec<String>,
    pub asset_readiness_complete: bool,
    pub last_asset_progress: Option<(usize, usize)>,
    /// When `last_asset_progress` last MOVED, on the feel clock — the only
    /// thing that separates "waiting" from "stuck".
    ///
    /// The poll computes `RoomAssetReadiness`, which carries `pending` BY NAME, and then keeps
    /// `(settled, total)` and drops the names. Every frame. So a barrier correctly waiting on the
    /// slowest of 129 fetches and one deadlocked on an asset that will never arrive looked
    /// identical, and they want opposite fixes.
    pub asset_progress_since: Option<Duration>,
    /// The explanation for the current stall, once it has earned one — which
    /// assets are still pending, how many, and for how long.
    ///
    /// STATE rather than only a log line, and deliberately: a test can assert
    /// on it, a dev overlay can show it, and it stays available for whoever asks
    /// next instead of scrolling past once. `None` while the barrier is moving,
    /// which is also what makes it the "already explained" flag — one report per
    /// stall, not one per frame.
    pub asset_stall_report: Option<String>,
    pub prefetch_hit: bool,
    /// WHICH checkpoint restore this transaction is preparing, if it is one.
    ///
    /// ⛔⛔ RESOLVED ONCE, WHEN THE TRANSACTION OPENS, and carried from there.
    /// Asking "does an accepted restore have an equal intent" at each later stage
    /// is not the same question: two crossings to one room, with one subject and
    /// one arrival, compare EQUAL, so a transaction opened for a door could be
    /// served a later restore's snapshot — or the reverse — merely by looking
    /// alike. The key is the session's own admitted-operation identity and
    /// cannot be produced by resemblance.
    ///
    /// ⚠ `None` is an ordinary crossing, and it must stay possible: a door
    /// recorded while a restore is outstanding prepares and commits from LIVE
    /// state, which is exactly right.
    pub checkpoint_operation: Option<
        ambition_platformer2d_actor_monolith::session::checkpoint::CheckpointOperationKey,
    >,
    pub construction_preflight_duration: Option<Duration>,
    pub asset_manifest_duration: Option<Duration>,
    pub requested_at: Option<Duration>,
    pub asset_ready_at: Option<Duration>,
    pub ready_at: Option<Duration>,
    pub cover_presented_at: Option<Duration>,
    pub commit_duration: Option<Duration>,
    pub committed_at: Option<Duration>,
}

impl ActiveRoomTransitionLoad {
    /// The destination room's authored id.
    ///
    /// ⭐ DERIVED, NOT STORED, and that is the point. This was a `String` field
    /// sitting beside `intent`, set once from `intent.target_room()` — one fact
    /// in two places on ONE struct, held in agreement by nothing but the
    /// constructor happening to be its only writer. Nothing enforced that, and a
    /// second writer would have been invisible.
    ///
    /// ⚠ IT IS NOT A CACHE. Sibling `target_room` (the `usize` index) IS one and
    /// stays a field: resolving it needs the live `RoomSet`, which this has no
    /// access to. The id needs nothing — the intent is right there — so storing
    /// it bought a copy and no lookup.
    pub fn target_room_id(&self) -> &str {
        self.intent.target_room()
    }

    /// Record settled asset progress and restart the stall clock only when the
    /// progress key changes. All writers use this method so progress and its
    /// timestamp cannot diverge.
    pub fn observe_asset_progress(&mut self, settled: usize, total: usize, now: Duration) -> bool {
        let key = (settled, total);
        if self.last_asset_progress == Some(key) {
            return false;
        }
        self.last_asset_progress = Some(key);
        self.asset_progress_since = Some(now);
        // A moving barrier is not a stalled one, and the next stall will be a
        // different stall — so the explanation is owed again.
        self.asset_stall_report = None;
        true
    }

    fn same_destination(
        &self,
        intent: &LifecycleIntent,
        session_scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        content_epoch: u64,
    ) -> bool {
        // the SEMANTIC destination, which is not the zone and not the room.
        //
        // This ANDed in `zone.id` while its own caller's comment said *"one transaction owns
        // that destination; trigger noise is not a new request"* — so two zones into one room
        // opened two transactions, the comment's opposite.
        //
        // So the key is what a crossing IS: who is crossing, where they are
        // going, where they come out, and by which kind of exit. Repeated
        // detection from one zone repeats all four and dedupes; two exits into
        // one room differ in `arrival` and do not; and once a second participant
        // can transit, two bodies differ in `subject` and do not either — which
        // is why `subject` is in the KEY and not merely in the payload.
        //
        // `arrival` is compared exactly, and that is correct rather than
        // sloppy: both sides come from the same authored `RoomSpec` field by the
        // same path, so trigger noise reproduces the same bits. A near-miss here
        // means a genuinely different destination.
        //
        // ⭐ A RECONSTITUTION ANSWERS ALL FOUR TOO: no subject, no arrival, never
        // an edge exit. So two bodyless rebuilds of one room dedupe with each
        // other, and a rebuild never dedupes against a crossing into the same
        // room — the subject differs (`None` vs `Some`), which is the honest
        // answer, because one places a body there and the other does not.
        self.content_epoch == content_epoch
            && self.session_scope == session_scope
            && self.intent.subject() == intent.subject()
            && self.intent.target_room() == intent.target_room()
            && self.intent.arrival() == intent.arrival()
            && self.intent.edge_exit() == intent.edge_exit()
    }
}

/// Host-side transaction authority for one ordinary room transition.
///
/// There is exactly one active transition. Repeated detection from the same
/// loading zone is ignored while it is in flight; a genuinely different target
/// supersedes it through the load coordinator's exact supersession path.
#[derive(Resource, Default, Debug)]
pub struct RoomTransitionLoadState {
    next_sequence: u64,
    pub active: Option<ActiveRoomTransitionLoad>,
}

impl RoomTransitionLoadState {
    fn mint_sequence(&mut self) -> u64 {
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.next_sequence
    }
}

fn apply_load_command(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    command: ambition_load::LoadCommand,
) {
    for event in loads.apply(command) {
        events.write(event);
    }
}

pub fn set_room_transition_work_state(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    load_id: &LoadId,
    work_id: LoadWorkId,
    state: LoadWorkState,
) {
    apply_load_command(
        loads,
        events,
        ambition_load::LoadCommand::SetWorkState {
            load_id: load_id.clone(),
            work_id,
            state,
        },
    );
}

fn set_work_state(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    load_id: &LoadId,
    work_id: &str,
    state: LoadWorkState,
) {
    set_room_transition_work_state(loads, events, load_id, LoadWorkId::new(work_id), state);
}

fn fail_work(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    load_id: &LoadId,
    work_id: &str,
    player_message: &str,
    developer_detail: String,
) {
    set_work_state(
        loads,
        events,
        load_id,
        work_id,
        LoadWorkState::Failed(LoadFailure::new(player_message, developer_detail).retryable(true)),
    );
}

fn close_discovery(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    barrier: &LoadBarrierRef,
) {
    apply_load_command(
        loads,
        events,
        ambition_load::LoadCommand::SetDiscovery {
            load_id: barrier.load_id.clone(),
            barrier_id: barrier.barrier_id.clone(),
            open: false,
            forecast: None,
        },
    );
}

/// The source room is still intact at every caller, so visible hosts can offer retry/cancel and
/// headless hosts can retire the failed transaction.
pub fn fail_room_transition_commit_precondition(
    state: &mut RoomTransitionLoadState,
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    sequence: u64,
    detail: String,
) {
    let Some(active) = state
        .active
        .as_mut()
        .filter(|active| active.sequence == sequence)
    else {
        return;
    };
    set_work_state(
        loads,
        events,
        &active.barrier.load_id,
        CONSTRUCTION_PREFLIGHT_WORK,
        LoadWorkState::Failed(
            LoadFailure::new(
                "The destination room could not be activated.",
                detail.clone(),
            )
            .retryable(true),
        ),
    );
    active.phase = RoomTransitionLoadPhase::Failed;
    active.failure = Some(detail.clone());
    bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
}

/// The confirmed crossing waiting to be loaded, or nothing.
///
/// Three resources answer one question: the pending intent, the app's STABLE
/// simulation host, and (for rollback) how far the CURRENT session has confirmed.
/// Bundled because [`begin_room_transition_load_system`] sits at Bevy's
/// 16-parameter ceiling and because "is this crossing safe to act on yet" is one
/// question, not three.
///
/// `ConfirmedFrameBoundary` PRESENCE IS SESSION STATE, NOT HOST IDENTITY.
/// `rollback::stop_session` deliberately removes it while leaving
/// [`crate::SimulationHost::Rollback`] installed. That is a permanent loading screen.
///
/// The stable host owns the policy instead:
/// - render/fixed hosts confirm on arrival;
/// - a rollback host requires both a live boundary and healthy confirmation authority;
///   absence or invalidation means confirmation authority is unavailable, so no
///   transition may begin or continue toward commit.
///
/// ⛔⛔ AND THE AUTHORITY IT ASKS IS THE LIVE GAMEPLAY SESSION'S, which is why
/// [`SessionRollbackConfirmation`] is here rather than a bare resource read.
/// This gate is where the cross-game contamination SURFACED — quit a Smash match
/// to the title, start Ambition, and every door refused, because the unhealthy
/// value this read belonged to the match that had already ended. The fix is not
/// a check in this system; it is that the authority now names its owner and
/// there is no way to read it without naming a scope.
///
/// [`SessionRollbackConfirmation`]: crate::SessionRollbackConfirmation
#[derive(bevy::ecs::system::SystemParam)]
pub struct ConfirmedRoomTransitionIntent<'w, 's> {
    pending: Res<
        'w,
        ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit,
    >,
    host: Res<'w, crate::SimulationHost>,
    boundary: Option<Res<'w, ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    confirmation: crate::SessionRollbackConfirmation<'w, 's>,
    /// ⭐ HAS THE "every transition is refused" EPISODE ALREADY BEEN REPORTED?
    ///
    /// A `Local`, deliberately: this is a property of the LOG and not of the
    /// world, so it must never become rollback state or reach a snapshot. Its
    /// only job is to keep a per-frame condition from printing per frame.
    ///
    /// ⛔ It lives INSIDE this bundle rather than beside it because
    /// `begin_room_transition_load_system` sits at Bevy's system-parameter
    /// ceiling - which is what this module's own doc says, and adding a
    /// seventeenth parameter fails to compile with a `chain` trait-bound error
    /// that names none of this.
    refusal_reported: bevy::prelude::Local<'s, bool>,
}

fn confirmation_frame_for_host(
    host: crate::SimulationHost,
    boundary: Option<&ambition_platformer2d_core::ConfirmedFrameBoundary>,
    confirmation: crate::RollbackConfirmationState,
) -> Option<i32> {
    match host {
        crate::SimulationHost::Rollback => {
            if !confirmation.is_healthy() {
                return None;
            }
            boundary.map(|boundary| boundary.confirmed)
        }
        crate::SimulationHost::RenderFrame | crate::SimulationHost::Fixed60Hz => Some(i32::MAX),
    }
}

impl ConfirmedRoomTransitionIntent<'_, '_> {
    /// Whether this APP is a rollback host. The boundary may disappear when its
    /// current session stops; that does not change the app's simulation host.
    fn is_rollback_host(&self) -> bool {
        self.host.is_rollback()
    }

    /// A stopped rollback session has no authority to promote a speculative intent
    /// into host-side loading work. Crucially, it is NOT an eager host.
    fn rollback_confirmation_unavailable(&self) -> bool {
        self.is_rollback_host()
            && confirmation_frame_for_host(
                *self.host,
                self.boundary.as_deref(),
                self.confirmation.state(),
            )
            .is_none()
    }

    /// The intent WITHOUT asking the confirmation authority — "is somebody
    /// waiting for a room change at all".
    ///
    /// ⛔ Only for reporting. Acting on an unconfirmed intent is exactly what
    /// [`Self::rollback_confirmation_unavailable`] exists to refuse; this is how
    /// the refusal can say whether it is refusing ANYTHING, so a silently
    /// stalled game is distinguishable from an idle one.
    fn get_unconfirmed(&self) -> Option<&LifecycleIntent> {
        Some(&self.pending.peek()?.kind)
    }

    fn get(&self) -> Option<&LifecycleIntent> {
        let confirmed = confirmation_frame_for_host(
            *self.host,
            self.boundary.as_deref(),
            self.confirmation.state(),
        )?;
        // ⭐ TWO VARIANTS NOW. The four in-place reset variants deleted in v140
        // are still gone — a same-room replay BY A BODY is a transition to the
        // room you are standing in. What v146 added is the case that is not a
        // crossing at all: a replay with no controlled body, which rebuilds the
        // room with nobody in it. This road serves both, through the accessors.
        Some(&self.pending.confirmed(confirmed)?.kind)
    }
}

/// Convert loading-zone detections into exact load transactions and perform the
/// mutation-free target preflight.
///
/// Construction preflight is mutation-free, and presentation-capable hosts attach a concrete
/// room asset manifest whose Bevy handles remain required work until they settle.
#[allow(clippy::too_many_arguments)]
pub fn begin_room_transition_load_system(
    // The one description of a crossing, recorded by whatever detected it
    // (a loading zone, a checkpoint resume, a level flag, a retry) and read here
    // only once its originating frame can never be simulated again.
    //
    // Host identity comes from `SimulationHost`; an eager host confirms on arrival, while a
    // rollback host with no current boundary or an unhealthy timeline has no confirmation authority
    // and must not open a transaction.
    mut pending: ConfirmedRoomTransitionIntent,
    mut state: ResMut<RoomTransitionLoadState>,
    content_epoch: Res<RoomTransitionContentEpoch>,
    active_binding: Option<Res<ambition_platformer2d_actor_monolith::rooms::ActiveContentBinding>>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<world_rooms::RoomSet>,
    construction_services: (
        Res<ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry>,
        Res<ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>,
        Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
        Res<ambition_boss_encounter::BossCatalog>,
        Res<ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry>,
        // Provider-authored sheets (U1 stage B): room construction sizes bodies
        // from their sheets, so a transition needs it beside the catalog.
        Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
        // WHAT BECAME OF THE OCCURRENCES THIS ROOM ALREADY MINTED. A
        // transition is the one road that rebuilds a room the session has been
        // LIVING in, so it is the road that owes the question an answer: an
        // authored object carried out of a room and back is still alive, and
        // re-authoring its `SimId::placement(..)` would put two live things
        // behind one identity. `Option` — a composition that remembers
        // nothing carries no ledger, which is the ordinary case.
        //
        // it is a seventh tuple member and not a seventeenth param because
        // a Bevy system stops at sixteen, and it belongs with the other
        // construction authorities in any case.
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
        // THE SECOND DESCRIBER, for the debts no record can settle. The
        // ledger above says an occurrence lies here; for a RUNTIME MINT no room
        // authors a record to rebuild it from, and this is what does. They
        // travel together for the same reason `remembered` and `world` do —
        // handing over the memory without the means to act on it deletes the
        // object.
        Option<
            Res<
                ambition_platformer2d_actor_monolith::items::pickup::minted_horizon::MintedItemBaseline,
            >,
        >,
        // ⛔⛔ THE PINNED CONTINUITY, AND WHY THE LIVE ONES ABOVE ARE NOT ENOUGH
        // FOR A CHECKPOINT RESET. The two members above are the LIVE ledger and
        // the LIVE minted baseline, which are the right inputs for a door. For a
        // checkpoint reconstruction they were only ever right because
        // `restore_occurrence_baseline` had overwritten the live ledger with the
        // checkpoint's earlier in the SAME FRAME — `CheckpointRestore` sits in
        // `PlayerInput` and this readiness runs after `RoomTransitionSet::Detect`
        // in `RoomTransition`. The room was therefore prepared from a live
        // resource swapped to the checkpoint value IN ORDER TO BE READ, and its
        // correctness rested on a phase order nothing states as a checkpoint
        // requirement. The accepted operation carries its own population.
        Option<Res<ambition_platformer2d_actor_monolith::session::checkpoint::AcceptedCheckpointRestore>>,
    ),
    // `Option`, and absence is a legal answer: a composition with no registered characters is
    // the ordinary case, and an empty registry means "no character states a default" — which is
    // what this route assumed before a definition could state one.
    //
    // outside the `construction_services` tuple deliberately: that tuple reads
    // positionally at the call site (`construction_services.5`), so one more
    // member would be one more number for a reader to decode.
    // PAIRED, and only because a Bevy system stops at sixteen params. The
    // two authorities travel together anyway: a placement names a character and
    // may name the policy that drives it.
    character_authorities: (
        Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
        // The published controller policies, so an enemy placement may name
        // one. A composition that publishes none is ordinary, and a placement
        // naming into an absent registry is what refuses.
        Option<Res<ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
        // ⭐ AND WHAT A DEVELOPER HAS FORCED THEM ALL TO, which is a third
        // authority about the same subject and travels with the pair for the
        // same sixteen-parameter reason. Lowering consults it while BUILDING a
        // brain; it used to reach into `ambition_dev_tools` from inside the
        // actor kernel to ask. Absent = no developer tools = the author decides.
        //
        // ⛔ AND IT MUST BE STATED HERE OR THE KNOB IS DEAD ON THIS ROAD. Walking
        // into the Hall is a TRANSITION, and the hall is the one room the brain
        // override exists to re-cast.
        Option<Res<ambition_characters::brain::AuthoredBrainOverride>>,
        // The developer's actor population cap: the same class of authority,
        // stated here for the same reason (the hall is the room it caps).
        Option<Res<ambition_characters::actor::AuthoredPopulationCap>>,
        // ⛔⛤ **THE GENERATION THIS SESSION WAS ACTIVATED UNDER, AND IT OUTRANKS
        // THE APP REGISTRIES ABOVE.** A door is the road a review measured going
        // back to whatever the App held: a session prepared under generation N
        // could walk through a door and have the next room built from N+1's cast
        // because a reload had published one in between. The frozen values win
        // where they exist; see `SessionMechanics`.
        Option<Res<ambition_platformer2d_actor_monolith::session::mechanics::SessionMechanics>>,
    ),
    asset_contributor: Option<Res<RoomTransitionAssetContributor>>,
    mut plan_prefetch: Option<ResMut<super::prefetch::RoomConstructionPlanPrefetch>>,
    real_time: Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    presentation_available: Option<Res<RoomTransitionPresentationAvailable>>,
    tick: Res<SimTick>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: MessageWriter<LoadEvent>,
    mut next_mode: ResMut<NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
) {
    let (prepared_characters, brain_profiles, forced_brains, population_cap, generation) =
        character_authorities;
    // ⛔ THE GENERATION'S VALUES WHEN THERE IS ONE. See `GenerationMechanics`.
    let mechanics =
        ambition_platformer2d_actor_monolith::session::mechanics::GenerationMechanics::new(
            generation.as_deref(),
            prepared_characters.as_deref(),
            &construction_services.5,
            &construction_services.3,
        );

    // A rollback app stays a rollback app when its session is stopped. If readiness was already
    // in flight, retire only the HOST-SIDE derivative. The rollback-state intent is
    // deliberately untouched here: `Update` is outside the rewound schedule and has no
    // authority to spend it.
    //
    // `Committed` is exempt because the room mutation already happened and the
    // presentation adapter merely owes its post-commit settle/telemetry tail.
    if pending.rollback_confirmation_unavailable() {
        let orphan = state
            .active
            .as_ref()
            .filter(|active| active.phase != RoomTransitionLoadPhase::Committed)
            .map(|active| {
                (
                    active.sequence,
                    active.source_room_id.clone(),
                    active.target_room_id().to_string(),
                    active.barrier.load_id.clone(),
                )
            });
        if let Some((sequence, source_room_id, target_room_id, load_id)) = orphan {
            apply_load_command(
                &mut loads,
                &mut load_events,
                ambition_load::LoadCommand::Cancel {
                    load_id: load_id.clone(),
                },
            );
            loads.retire(&load_id);
            state.active = None;
            bevy::log::error!(
                target: "ambition_platformer2d::room_transition",
                "retired room transition {sequence} ({source_room_id} -> {target_room_id}) because \
                 rollback confirmation authority is unavailable (boundary_present={}, confirmation={:?}, \
                 authority_owner/live_scope={:?}); \
                 refusing to reinterpret a stopped or unhealthy rollback timeline as an eager host",
                pending.boundary.is_some(),
                pending.confirmation.state(),
                pending.confirmation.ownership(),
            );
        }
        // ⛔⛔ AND SAY SO WHEN THERE WAS NOTHING TO ORPHAN, WHICH IS THE CASE
        // THAT WAS SILENT. The error above only fires when a transition was
        // already in flight. A transition that is REFUSED BEFORE IT BEGINS
        // printed nothing at all — so a game that has silently stopped being
        // able to change rooms produced a log in which the only clue was a
        // downstream watchdog eight seconds later saying *"check the
        // room_transition log for a BEGIN with no retirement"*, in a capture
        // that contains no BEGIN because the transition never started.
        //
        // Jon, 2026-08-23, on Mary-O replaying 1-1 forever: *"I don't know why
        // this bug keeps coming back. There is a structural problem... Not sure
        // if logs give any useful information."* They did not, and this is why.
        //
        // ⭐ ONCE PER EPISODE, not per frame: the refusal repeats every tick for
        // as long as the authority is missing, and a per-frame line would bury
        // the thing it is trying to show. The latch clears the moment a
        // transition is admitted again, so a second episode reports a second
        // time.
        else if pending.get_unconfirmed().is_some() && !*pending.refusal_reported {
            *pending.refusal_reported = true;
            bevy::log::error!(
                target: "ambition_platformer2d::room_transition",
                "REFUSING every room transition: this app is a rollback host and its \
                 confirmation authority is unavailable (boundary_present={}, confirmation={:?}, \
                 authority_owner/live_scope={:?}). \
                 A pending intent cannot be promoted, so the room will never change and \
                 whatever asked for it will time out. This is session-scoped state - it is \
                 normally the shape of a session that stopped or never started healthily. \
                 When the two scopes DIFFER, the authority belongs to a session that is over \
                 and this one has none installed yet.",
                pending.boundary.is_some(),
                pending.confirmation.state(),
                pending.confirmation.ownership(),
            );
            ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
                "room-transition REFUSED boundary={} confirmation={:?} owner/live={:?}",
                pending.boundary.is_some(),
                pending.confirmation.state(),
                pending.confirmation.ownership(),
            ));
        }
        return;
    }
    if *pending.refusal_reported {
        *pending.refusal_reported = false;
        bevy::log::info!(
            target: "ambition_platformer2d::room_transition",
            "room transitions are admitted again; the confirmation authority came back",
        );
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-transition admitted-again"
        ));
    }

    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    let Some(intent) = pending.get() else {
        // On a rollback host, absence in the current simulation can be
        // speculative. Host-side derived state may only be retired by the
        // confirmed lifecycle path, never inferred from that temporary absence.
        if pending.is_rollback_host() {
            return;
        }

        // A transaction is host-side DERIVED state for an eager simulation
        // intent. If the simulation retracts that intent — most importantly when
        // the crossing body dies — the transaction has lost its authority and
        // must not outlive it.
        //
        // `Committed` is the one deliberate exception: at that point the intent has been spent
        // successfully and the transaction has been handed to the presentation adapter for its
        // post-commit settle/telemetry retirement.
        let orphan = state
            .active
            .as_ref()
            .filter(|active| active.phase != RoomTransitionLoadPhase::Committed)
            .map(|active| (active.sequence, active.barrier.load_id.clone()));
        if let Some((sequence, load_id)) = orphan {
            apply_load_command(
                &mut loads,
                &mut load_events,
                ambition_load::LoadCommand::Cancel {
                    load_id: load_id.clone(),
                },
            );
            loads.retire(&load_id);
            state.active = None;
            ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
                ambition_platformer2d_shared_tangle::schedule::GameMode::Playing,
                "room_transition_intent_retracted",
            );
            next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
            bevy::log::info!(
                target: "ambition_platformer2d::room_transition",
                "room transition {sequence} cancelled because its confirmed lifecycle intent was retracted"
            );
        }
        return;
    };
    {
        if state.active.as_ref().is_some_and(|active| {
            active.same_destination(intent, current_session, content_epoch.get())
        }) {
            // One transaction owns that destination; a still-pending crossing is not a new one.
            return;
        }

        let superseded = state.active.take().map(|active| active.barrier.load_id);
        let sequence = state.mint_sequence();
        let source_room = room_set.active;
        let source_room_id = room_set
            .rooms
            .get(source_room)
            .map(|room| room.id.clone())
            .unwrap_or_else(|| format!("<room-index-{source_room}>"));
        // The intent names its destination by AUTHORED ID, because it is rollback
        // state and an index is not stable across a content reload. Resolving it
        // is this transaction's first job — and its first way to fail.
        let target_index = room_set.room_index_by_id(intent.target_room());
        // ⭐ THE INTENT'S OWN ID IS THE LABEL, and that is now a fact rather than
        // a coincidence. This used to resolve the index and read `rooms[i].id`
        // back out, falling back to the intent — a round trip that CANONICALISED,
        // because the lookup also accepted a room's display title. It matches the
        // authored id and nothing else now (`RoomSet::room_index_by_id`), so both
        // branches produced this string and the round trip only hid that a room
        // could arrive here under a second name.
        let target_label = intent.target_room().to_string();
        let load_id = LoadId::new(format!(
            "room-transition:{sequence}:{source_room_id}->{target_label}"
        ));
        let barrier = LoadBarrierRef::new(load_id.clone(), ROOM_READY_BARRIER);
        let asset_work_id = LoadWorkId::new(format!("{ROOM_ASSET_WORK_PREFIX}:{}", target_label,));

        let mut plan = LoadPlanSpec::new(load_id.clone(), format!("Prepare room {target_label}"));
        plan.supersedes = superseded.clone();
        apply_load_command(
            &mut loads,
            &mut load_events,
            ambition_load::LoadCommand::Begin(plan),
        );
        if let Some(old) = superseded {
            // The supersession event has already been published. The room
            // adapter owns no historical telemetry yet, so retire the obsolete
            // resident plan instead of leaking one record per retrigger.
            loads.retire(&old);
        }
        apply_load_command(
            &mut loads,
            &mut load_events,
            ambition_load::LoadCommand::DeclareBarrier {
                load_id: load_id.clone(),
                spec: LoadBarrierSpec::new(ROOM_READY_BARRIER, "Preparing destination room"),
            },
        );
        for spec in [
            LoadWorkSpec::required(
                TARGET_LOOKUP_WORK,
                "Resolve target room",
                ROOM_READY_BARRIER,
            ),
            LoadWorkSpec::required(
                ARRIVAL_VALIDATION_WORK,
                "Validate target arrival",
                ROOM_READY_BARRIER,
            ),
            LoadWorkSpec::required(
                CONSTRUCTION_PREFLIGHT_WORK,
                "Preflight room construction",
                ROOM_READY_BARRIER,
            ),
            LoadWorkSpec::required(
                asset_work_id.clone(),
                format!("Load presentation assets for {target_label}"),
                ROOM_READY_BARRIER,
            ),
        ] {
            apply_load_command(
                &mut loads,
                &mut load_events,
                ambition_load::LoadCommand::UpsertWork {
                    load_id: load_id.clone(),
                    spec,
                },
            );
        }

        ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
            ambition_platformer2d_shared_tangle::schedule::GameMode::RoomTransition,
            "room_transition_begin",
        );
        // A ROLLBACK HOST DOES NOT PAUSE FOR ITS OWN LOADING SCREEN.
        //
        // and it is not merely unsound, it is wrong for the thing this
        // enables: peers do not stop simulating because one of them is loading.
        // The COVER still goes up — it is driven off `RoomTransitionLoadState`,
        // not off the mode — so the player sees the same screen; the world
        // behind it keeps its own time.
        if !pending.is_rollback_host() {
            next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::RoomTransition);
        }

        let cover_required = presentation_available.is_some();
        // A transition always says it started.
        //
        // The completion telemetry
        // (`ambition_platformer2d::room_transition::performance`) is emitted at RETIREMENT,
        // behind `runtime.owner.take()`, so a transition that never spawned a
        // cover reports nothing at all — and a 34.6s desktop capture containing a
        // real hub -> Hall transition produced ZERO lines from that target while
        // this module's other logs printed fine. An instrument that
        // is silent exactly when the thing it measures goes wrong is worse than
        // no instrument: it reads as "no transitions happened".
        //
        // This one is unconditional and at the START, so the pair (begin, retire)
        // is readable in any capture and a MISSING retirement is itself the
        // signal.
        bevy::log::info!(
            target: "ambition_platformer2d::room_transition::performance",
            "room transition {sequence} BEGIN {source_room_id} -> {target_label}              (cover_required={cover_required})",
        );
        // The same fact on the `[world-event]` marker channel. Not a second
        // emission point: the transition is announced here and only here, and
        // this is that announcement reaching the sink that survives to Android
        // logcat and to a profile timeline, carrying the frame number the
        // tracing line has no room for.
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-transition begin seq={sequence} {source_room_id} -> {target_label} \
             cover_required={cover_required}"
        ));
        let mut active = ActiveRoomTransitionLoad {
            sequence,
            content_epoch: content_epoch.get(),
            session_scope: current_session,
            source_room,
            source_room_id,
            target_room: target_index.unwrap_or(usize::MAX),
            intent: intent.clone(),
            construction_plan: None,
            barrier: barrier.clone(),
            // Even when every contributor resolves immediately, commit happens
            // on a later simulation step. This makes readiness and commit two
            // real phases and gives Phase 3 a place to insert cover rendering.
            commit_not_before_tick: tick.get().saturating_add(1),
            cover_required,
            cover_presented: !cover_required,
            phase: RoomTransitionLoadPhase::AwaitingReadiness,
            failure: None,
            asset_work_id: asset_work_id.clone(),
            staged_actor_names: Vec::new(),
            asset_readiness_complete: false,
            last_asset_progress: None,
            asset_progress_since: None,
            asset_stall_report: None,
            prefetch_hit: false,
            // ⛔⛔ **ADOPTED AT OPEN, NOT AT PREFLIGHT, AND THAT IS A REPAIR.**
            // It used to be resolved further down, inside the construction
            // preflight — so a transaction that failed BEFORE reaching there
            // (unknown destination, non-finite arrival, no session scope) carried
            // no key, `abandon_failed_checkpoint_restore_system` found nothing to
            // note, and the operation was never ended. Measured, not reasoned: a
            // confirmed-host fixture aimed at a room no content pack authors
            // opened 597 transactions for one intent in 600 frames. The earliest
            // failures are exactly the ones a checkpoint restore's protocol calls
            // terminal, so the key has to be on the record before any of them can
            // return.
            checkpoint_operation: construction_services
                .8
                .as_deref()
                .and_then(|accepted| accepted.inputs_for(&intent))
                .map(|accepted| accepted.key),
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: real_time.as_deref().map(|time| time.elapsed()),
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        };

        let Some(target_spec) = target_index.and_then(|index| room_set.rooms.get(index)) else {
            let detail = format!(
                "transition from '{}' targets room '{}', which this world does not contain",
                active.source_room_id,
                intent.target_room(),
            );
            fail_work(
                &mut loads,
                &mut load_events,
                &load_id,
                TARGET_LOOKUP_WORK,
                "The destination room is unavailable.",
                detail.clone(),
            );
            for work in [ARRIVAL_VALIDATION_WORK, CONSTRUCTION_PREFLIGHT_WORK] {
                set_work_state(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    work,
                    LoadWorkState::Skipped,
                );
            }
            set_room_transition_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                asset_work_id.clone(),
                LoadWorkState::Skipped,
            );
            close_discovery(&mut loads, &mut load_events, &barrier);
            active.phase = RoomTransitionLoadPhase::Failed;
            active.failure = Some(detail.clone());
            bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
            state.active = Some(active);
            return;
        };
        // Resolved above and proven present by the arm just closed.
        let resolved_target_index = target_index.expect("target_spec resolved from this index");
        set_work_state(
            &mut loads,
            &mut load_events,
            &load_id,
            TARGET_LOOKUP_WORK,
            LoadWorkState::Complete,
        );

        // ⭐ A REBUILD WITH NOBODY IN IT HAS NO ARRIVAL TO VALIDATE, and that is
        // not the same as an arrival that failed validation — `is_finite` on a
        // substituted zero would PASS and place a body at the origin. `None`
        // skips the check because there is nothing being placed.
        if intent.arrival().is_some_and(|arrival| !arrival.is_finite()) {
            let detail = format!(
                "transition into '{}' has non-finite arrival {:?}",
                target_spec.id,
                intent.arrival(),
            );
            fail_work(
                &mut loads,
                &mut load_events,
                &load_id,
                ARRIVAL_VALIDATION_WORK,
                "The destination arrival is invalid.",
                detail.clone(),
            );
            set_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                CONSTRUCTION_PREFLIGHT_WORK,
                LoadWorkState::Skipped,
            );
            set_room_transition_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                asset_work_id.clone(),
                LoadWorkState::Skipped,
            );
            close_discovery(&mut loads, &mut load_events, &barrier);
            active.phase = RoomTransitionLoadPhase::Failed;
            active.failure = Some(detail.clone());
            bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
            state.active = Some(active);
            return;
        }
        set_work_state(
            &mut loads,
            &mut load_events,
            &load_id,
            ARRIVAL_VALIDATION_WORK,
            LoadWorkState::Complete,
        );

        let Some(session_scope) =
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
                active_session.as_deref(),
            )
        else {
            let detail = "room transition has no active session construction scope".to_string();
            fail_work(
                &mut loads,
                &mut load_events,
                &load_id,
                CONSTRUCTION_PREFLIGHT_WORK,
                "The destination room could not be prepared.",
                detail.clone(),
            );
            set_room_transition_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                asset_work_id.clone(),
                LoadWorkState::Skipped,
            );
            close_discovery(&mut loads, &mut load_events, &barrier);
            active.phase = RoomTransitionLoadPhase::Failed;
            active.failure = Some(detail.clone());
            bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
            state.active = Some(active);
            return;
        };
        // AMBITION_REVIEW(determinism): same as `commit_duration` — wall clock
        // feeding the write-only `construction_preflight_duration` diagnostic,
        // observed by no sim decision and not rollback-registered. Preflight cost
        // is a player-facing latency question, so the feel clock is the right one.
        // THIS WAS `#[cfg(not(target_arch = "wasm32"))] std::time::Instant`,
        // so the one platform whose numbers would explain a slow transition was
        // the one platform that recorded none. `bevy::platform::time::Instant`
        // is `web-time` on wasm and `std` elsewhere, already in the graph, and
        // sub-frame on both — `Time<Real>` is NOT a substitute here, because it
        // advances once per frame and a within-frame span measures zero.
        let construction_preflight_started = bevy::platform::time::Instant::now();
        // What this operation reconstructs FROM, read once: it decides both
        // whether a cached plan still describes it and what a fresh plan must
        // leave out.
        // ⭐ ONE SELECTION, FEEDING BOTH THE CACHE KEY AND THE FRESH PLAN. The
        // outlook validates a prefetched plan and the continuity lowers a fresh
        // one; redirecting only the second would leave a plan prepared against
        // the live population free to be promoted for a reconstruction that is
        // about a different one.
        // ⭐ RESOLVED ONCE AND REMEMBERED, at the top of this system where the
        // transaction record is built. Every later stage — this one included —
        // names the KEY rather than re-asking the accepted operation whether it
        // owns this intent.
        let selected_restore = active.checkpoint_operation.and_then(|key| {
            construction_services
                .8
                .as_deref()
                .and_then(|accepted| accepted.inputs_for_key(key))
        });
        let selected_ledger = selected_restore
            .map(|accepted| accepted.occurrences.remembered())
            .or(construction_services.6.as_deref());
        let selected_minted = match selected_restore {
            Some(accepted) => accepted.item.as_ref().map(|item| &item.minted),
            None => construction_services.7.as_deref(),
        };
        // the outlook is ROOM-SCOPED, so it is derived for the room being built
        // and for no other: the same ledger answers differently for two rooms,
        // because an occurrence lying in one of them is reinstated there and is
        // simply not that other room's business.
        let occurrence_outlook = selected_ledger
            .map(|ledger| ledger.outlook_for(&target_spec.id))
            .unwrap_or_default();
        let prefetched_construction = plan_prefetch.as_deref_mut().and_then(|cache| {
            cache.promote(
                &super::prefetch::PrefetchIdentity::new(
                    content_epoch.get(),
                    current_session,
                    &active.source_room_id,
                ),
                target_spec,
                &occurrence_outlook,
            )
        });
        active.prefetch_hit = prefetched_construction.is_some();
        let construction_plan_result = match prefetched_construction {
            Some(plan) => Ok(plan),
            None => rooms::RoomConstructionPlan::prepare_from_parts(
                &room_set,
                resolved_target_index,
                &construction_services.0,
                &construction_services.1,
                mechanics.bosses(),
                session_scope,
                // A transition rebuilds a room the ACTIVE content already
                // defines, so the plan states the session's LIVE binding — the
                // transition-local counter is a prefetch cache key, not a
                // content generation (same fix reset received). Plus the cast,
                // and the policies a PLACEMENT may name: a room whose enemy
                // spawn authors `brain_profile` resolves it against those.
                ambition_platformer2d_actor_monolith::features::ActorConstructionContext::for_room_construction(
                    &construction_services.4,
                    &construction_services.2,
                    mechanics.sheets(),
                    ambition_platformer2d_core::ContentEpoch(content_epoch.get()),
                    active_binding.as_deref(),
                    mechanics.characters(),
                    brain_profiles.as_deref(),
                    // THE ROAD THAT REBUILDS A ROOM THE SESSION LIVED IN.
                    // This is the only construction road that can meet an
                    // occurrence of its own making still alive, and it is
                    // therefore the only one that states a ledger.
                    //
                    // and it states the WORLD'S DEFINITIONS with it,
                    // because an occurrence lying in the destination may have
                    // been minted by a record next door: a body carried it
                    // through this very road and put it down. The ledger has
                    // already told that record's home room not to author it, so
                    // handing over the memory without the definitions to act on
                    // it would delete the object from the world. They are one
                    // value for exactly that reason.
                    selected_ledger.map(|remembered| {
                        ambition_platformer2d_actor_monolith::features::OccurrenceContinuity {
                            remembered,
                            world: &room_set.rooms,
                            minted: selected_minted,
                        }
                    }),
                    forced_brains.as_deref(),
                    population_cap.as_deref(),
                ),
            )
            .map(Arc::new),
        };
        let construction_plan = match construction_plan_result {
            Ok(plan) => plan,
            Err(error) => {
                let detail = error.to_string();
                fail_work(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    CONSTRUCTION_PREFLIGHT_WORK,
                    "The destination room could not be prepared.",
                    detail.clone(),
                );
                set_room_transition_work_state(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    asset_work_id.clone(),
                    LoadWorkState::Skipped,
                );
                close_discovery(&mut loads, &mut load_events, &barrier);
                active.phase = RoomTransitionLoadPhase::Failed;
                active.failure = Some(detail.clone());
                bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
                state.active = Some(active);
                return;
            }
        };
        active.construction_preflight_duration = Some(construction_preflight_started.elapsed());
        let staged_names = construction_plan.content_staged_names();
        active.construction_plan = Some(construction_plan);
        set_work_state(
            &mut loads,
            &mut load_events,
            &load_id,
            CONSTRUCTION_PREFLIGHT_WORK,
            LoadWorkState::Complete,
        );

        // ── The asset contributor seam ──────────────────────────────────
        //
        // "Has the destination room's art arrived" is a real question about a
        // room transition, so the work item is always DECLARED. Answering it
        // needs a sprite catalog, an asset server, and a resolved visual
        // quality — host property, not engine property. A host that has them
        // installs `RoomTransitionAssetContributor` and owns resolving this
        // item; a headless or minimal host has no presentation asset authority
        // and its barrier stays honest by SKIPPING the contributor rather than
        // pretending an asset loaded.
        //
        // `staged_names` is published for the contributor because it is derived
        // from the plan the engine just preflighted — recomputing it host-side
        // would be a second authority on what the target room stages.
        if asset_contributor.is_some() {
            active.staged_actor_names = staged_names;
            set_room_transition_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                asset_work_id.clone(),
                LoadWorkState::Running { progress: None },
            );
        } else {
            active.asset_readiness_complete = true;
            active.asset_ready_at = active.requested_at;
            set_room_transition_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                asset_work_id.clone(),
                LoadWorkState::Skipped,
            );
        }
        close_discovery(&mut loads, &mut load_events, &barrier);
        state.active = Some(active);
    }
}

/// Observe the required barrier and obtain one-shot commit authorization.
///
/// The deliberate next-tick gate prevents the old request/apply same-pass path
/// from reappearing even while all current contributors are immediate.
pub fn authorize_ready_room_transition_system(
    tick: Res<SimTick>,
    real_time: Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
    mut state: ResMut<RoomTransitionLoadState>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: MessageWriter<LoadEvent>,
) {
    let Some(active) = state.active.as_mut() else {
        return;
    };
    if active.phase != RoomTransitionLoadPhase::AwaitingReadiness
        || tick.get() < active.commit_not_before_tick
    {
        return;
    }
    let Some(snapshot) = loads.snapshot(&active.barrier.load_id, &active.barrier.barrier_id) else {
        return;
    };
    match snapshot.readiness {
        BarrierReadiness::Ready => {
            if active.ready_at.is_none() {
                active.ready_at = real_time.as_deref().map(|time| time.elapsed());
            }
            if active.cover_required && !active.cover_presented {
                return;
            }
            match loads.request_commit(&active.barrier.load_id, &active.barrier.barrier_id) {
                Ok(()) => {
                    load_events.write(LoadEvent::CommitAuthorized {
                        load_id: active.barrier.load_id.clone(),
                        barrier_id: active.barrier.barrier_id.clone(),
                    });
                    active.phase = RoomTransitionLoadPhase::CommitAuthorized;
                }
                Err(LoadCommitRejection::AlreadyAuthorized) => {
                    // The authorization belongs to this exact transaction. Treat an
                    // idempotent re-observation as authorized rather than opening a
                    // second commit path.
                    active.phase = RoomTransitionLoadPhase::CommitAuthorized;
                }
                Err(reason) => {
                    let detail = format!(
                        "room transition {} commit authorization was rejected: {reason:?}",
                        active.sequence,
                    );
                    load_events.write(LoadEvent::CommitRejected {
                        load_id: active.barrier.load_id.clone(),
                        barrier_id: active.barrier.barrier_id.clone(),
                        reason,
                    });
                    active.phase = RoomTransitionLoadPhase::Failed;
                    active.failure = Some(detail.clone());
                    bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
                }
            }
        }
        BarrierReadiness::Failed | BarrierReadiness::Cancelled | BarrierReadiness::Superseded => {
            let detail = format!(
                "room transition {} cannot commit because its barrier is {:?}",
                active.sequence, snapshot.readiness,
            );
            active.phase = RoomTransitionLoadPhase::Failed;
            active.failure = Some(detail.clone());
            bevy::log::error!(target: "ambition_platformer2d::room_transition", "{detail}");
        }
        BarrierReadiness::Preparing => {}
    }
}

/// Hand a failed checkpoint restore to the commit executor to be ENDED, on both
/// hosts.
///
/// ⛔⛔ WITHOUT THIS A FAILED PREPARATION RETRIED FOREVER. The transaction was
/// torn down while its lifecycle intent stayed PENDING, so readiness opened a
/// new transaction for the same intent on the next frame, against the same
/// invalid definition — and the accepted operation was never retired, so nothing
/// said the session was stuck.
///
/// ⛔⛔ **IT DOES NOT DO THE RETRACTION ITSELF, AND MAY NOT.** An earlier version
/// took `ResMut<AcceptedCheckpointRestore>` and `ResMut<PendingLifecycleCommit>`
/// here — rollback-registered state, written from `Update`, which never rewinds.
/// `check_rollback_mutators_run_in_sim` found it: a retraction that survives a
/// rollback desyncs the run. What this may touch is the HOST-side transaction it
/// already owns, so it takes the key off the failed load and leaves it in
/// [`AbandonedCheckpointOperation`]; the commit executor spends it.
///
/// ⚠ CHECKPOINT OPERATIONS ONLY. A door whose preparation failed transiently may
/// legitimately be retried; a checkpoint restore's protocol says an invalid
/// definition is terminal, and only the checkpoint road recorded the intent this
/// cancels. Widening it to every transition is a separate ruling.
///
/// ⭐ IT RUNS BEFORE THE HEADLESS FINALIZER AND ON THE VISIBLE HOST ALIKE,
/// because the transaction reaches `Failed` in seven places and is torn down in
/// two — noting at the phase rather than at either teardown is what keeps the two
/// hosts from needing separate answers. `take()` makes it idempotent: a second
/// pass over the same failed transaction finds no key.
pub fn abandon_failed_checkpoint_restore_system(
    mut state: ResMut<RoomTransitionLoadState>,
    // ⚠ READ, NOT WRITTEN. Reading rollback state in `Update` is ordinary; what
    // the guard forbids — and what this system used to do — is WRITING it. The
    // note is value-complete because the live operation is right here, so a
    // caller cannot record half of an identity.
    accepted: Res<ambition_platformer2d_actor_monolith::session::checkpoint::AcceptedCheckpointRestore>,
    mut abandoned: ResMut<
        ambition_platformer2d_actor_monolith::session::checkpoint::AbandonedCheckpointOperation,
    >,
) {
    let Some(active) = state.active.as_mut() else {
        return;
    };
    if active.phase != RoomTransitionLoadPhase::Failed {
        return;
    }
    let Some(key) = active.checkpoint_operation.take() else {
        return;
    };
    // The operation must still be the outstanding one. If it is not, it has
    // already been answered and there is nothing for a commit boundary to end.
    let Some(operation) = accepted.inputs_for_key(key) else {
        return;
    };
    abandoned.note(operation);
}

/// Retire failed transitions in hosts that deliberately install no visible
/// presentation adapter. A windowed host keeps the failed transaction resident
/// so the loading foreground can offer retry/cancel while the source room stays
/// intact.
pub fn finalize_unpresented_room_transition_failure_system(
    presentation_available: Option<Res<RoomTransitionPresentationAvailable>>,
    mut state: ResMut<RoomTransitionLoadState>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: MessageWriter<LoadEvent>,
    mut next_mode: ResMut<NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
) {
    if presentation_available.is_some()
        || !state
            .active
            .as_ref()
            .is_some_and(|active| active.phase == RoomTransitionLoadPhase::Failed)
    {
        return;
    }
    let active = state
        .active
        .take()
        .expect("failed room transition was present above");
    apply_load_command(
        &mut loads,
        &mut load_events,
        ambition_load::LoadCommand::Cancel {
            load_id: active.barrier.load_id.clone(),
        },
    );
    loads.retire(&active.barrier.load_id);
    ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
        ambition_platformer2d_shared_tangle::schedule::GameMode::Playing,
        "room_transition_abandoned",
    );
    next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core as ae;

    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    fn hero() -> SimId {
        SimId::placement("hero")
    }

    use ambition_platformer2d_actor_monolith::session::lifecycle_commit::RoomTransitionIntent;

    fn request(target_room: &str) -> LifecycleIntent {
        request_by(target_room, ae::Vec2::ZERO, hero())
    }

    fn request_by(target_room: &str, arrival: ae::Vec2, subject: SimId) -> LifecycleIntent {
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject,
            target_room: target_room.to_string(),
            arrival,
            edge_exit: false,
            zone_sfx: None,
        })
    }

    /// A rebuild with nobody in it, of the same room a `request` crosses into.
    fn rebuild(target_room: &str) -> LifecycleIntent {
        LifecycleIntent::ReconstituteRoom(
            ambition_platformer2d_actor_monolith::session::lifecycle_commit::RoomReconstitutionIntent {
                target_room: target_room.to_string(),
            },
        )
    }

    #[test]
    fn repeated_zone_detection_is_one_destination() {
        let active = ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 1,
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room: 1,
            intent: request("b"),
            construction_plan: None,
            barrier: LoadBarrierRef::new("load", "ready"),
            commit_not_before_tick: 1,
            cover_required: false,
            cover_presented: true,
            phase: RoomTransitionLoadPhase::AwaitingReadiness,
            failure: None,
            asset_work_id: LoadWorkId::new("room-transition.assets:b"),
            staged_actor_names: Vec::new(),
            asset_readiness_complete: false,
            last_asset_progress: None,
            asset_progress_since: None,
            asset_stall_report: None,
            prefetch_hit: false,
            checkpoint_operation: None,
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: None,
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        };
        // Trigger noise: the same body re-detecting the same crossing.
        assert!(active.same_destination(&request("b"), None, 1));
        // THE POISON, and the reason the key is not just `target_room`.
        // Two exits can lead to the same room at different arrivals. A room-only
        // key collapses them and lands this crossing at the other one's
        // coordinates.
        assert!(!active.same_destination(
            &request_by("b", ae::Vec2::new(0.0, 64.0), hero()),
            None,
            1
        ));
        // and a DIFFERENT BODY crossing to the same place is a different
        // crossing, not noise. Nothing can trigger this today — one participant
        // transits — which is why it is asserted rather than discovered later.
        assert!(!active.same_destination(
            &request_by("b", ae::Vec2::ZERO, SimId::placement("other_body")),
            None,
            1
        ));
        // ⭐ AND A BODYLESS REBUILD OF THE SAME ROOM IS NOT THIS CROSSING (v146).
        // One places a body at an arrival and the other places nobody, so a
        // transaction opened for the crossing must not be reused for the
        // rebuild — the subject differs (`Some` vs `None`), which is the
        // difference that matters.
        assert!(!active.same_destination(&rebuild("b"), None, 1));
        assert!(!active.same_destination(&request("c"), None, 1));
        assert!(!active.same_destination(&request("b"), None, 2));
        assert!(!active.same_destination(
            &request("b"),
            Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(9)),
            1,
        ));
    }

    #[test]
    fn stopped_rollback_session_is_not_reclassified_as_eager() {
        assert_eq!(
            confirmation_frame_for_host(
                crate::SimulationHost::Rollback,
                None,
                crate::RollbackConfirmationState::Healthy,
            ),
            None,
            "a rollback app without a live boundary has no confirmation authority"
        );
        assert_eq!(
            confirmation_frame_for_host(
                crate::SimulationHost::Fixed60Hz,
                None,
                crate::RollbackConfirmationState::Unavailable,
            ),
            Some(i32::MAX),
            "a true eager host confirms on arrival"
        );
        assert_eq!(
            confirmation_frame_for_host(
                crate::SimulationHost::RenderFrame,
                None,
                crate::RollbackConfirmationState::Unavailable,
            ),
            Some(i32::MAX),
            "the render-frame host is eager too"
        );
    }

    #[test]
    fn live_rollback_session_uses_its_confirmed_frame() {
        let boundary = ambition_platformer2d_core::ConfirmedFrameBoundary {
            current: 17,
            confirmed: 12,
            session: 4,
        };
        assert_eq!(
            confirmation_frame_for_host(
                crate::SimulationHost::Rollback,
                Some(&boundary),
                crate::RollbackConfirmationState::Healthy,
            ),
            Some(12)
        );
    }

    #[test]
    fn unhealthy_rollback_session_has_no_confirmation_authority_even_with_a_boundary() {
        let boundary = ambition_platformer2d_core::ConfirmedFrameBoundary {
            current: 17,
            confirmed: 12,
            session: 4,
        };
        assert_eq!(
            confirmation_frame_for_host(
                crate::SimulationHost::Rollback,
                Some(&boundary),
                crate::RollbackConfirmationState::Unhealthy,
            ),
            None,
            "a replacement session carrying an invalidation must not authorize a load"
        );
    }

    #[test]
    fn visible_transition_requires_cover_acknowledgment() {
        let mut active = ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 1,
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room: 1,
            intent: request("b"),
            construction_plan: None,
            barrier: LoadBarrierRef::new("load", "ready"),
            commit_not_before_tick: 1,
            cover_required: true,
            cover_presented: false,
            phase: RoomTransitionLoadPhase::AwaitingReadiness,
            failure: None,
            asset_work_id: LoadWorkId::new("room-transition.assets:b"),
            staged_actor_names: Vec::new(),
            asset_readiness_complete: false,
            last_asset_progress: None,
            asset_progress_since: None,
            asset_stall_report: None,
            prefetch_hit: false,
            checkpoint_operation: None,
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: None,
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        };
        assert!(active.cover_required && !active.cover_presented);
        active.cover_presented = true;
        assert!(active.cover_presented);
    }
}

#[cfg(test)]
mod checkpoint_failure_tests {
    use super::*;
    use ambition_platformer2d_actor_monolith::session::checkpoint::{
        AbandonedCheckpointOperation, AcceptedCheckpointRestore, AcceptedRestore,
        RestoreCancellation, SessionCheckpointOperations, SessionCheckpointOutcomes,
    };
    use crate::room_transition::commit::terminalize_abandoned_checkpoint_restore_system;
    use bevy::prelude::IntoScheduleConfigs;
    use ambition_platformer2d_actor_monolith::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    use bevy::prelude::App;

    fn crossing() -> LifecycleIntent {
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject: SimId::placement("hero"),
            target_room: "a_room_that_cannot_be_prepared".into(),
            arrival: ambition_platformer2d_core::Vec2::ZERO,
            edge_exit: false,
            zone_sfx: None,
        })
    }

    /// ⛔⛔ **AN ADMITTED RESTORE WHOSE PREPARATION FAILS MUST END, AND END ONCE.**
    ///
    /// This is the protocol's *"admitted / invalid preparation"* row, and before
    /// this system it was half-true. Preparation failure tore the transaction
    /// down and left the lifecycle intent PENDING, so readiness opened a new
    /// transaction for the same intent on the very next frame — against the same
    /// invalid definition, forever. The accepted operation was never retired and
    /// no outcome was ever published, so a session could spend every frame
    /// failing one preparation with nothing saying so.
    ///
    /// ⚠ "NOTHING DESTRUCTIVE RAN" WAS NOT ENOUGH, and I recorded this row as
    /// closed on exactly that argument. It is true — every domain reducer lives
    /// in the commit's schedule — and it proves only the *retain live state* half.
    /// TERMINALIZATION and *no permanent retry* are separate claims and needed
    /// this.
    #[test]
    fn a_failed_preparation_ends_the_operation_once_and_does_not_retry_it() {
        let mut app = App::new();
        app.init_resource::<RoomTransitionLoadState>();
        app.init_resource::<AcceptedCheckpointRestore>();
        app.init_resource::<SessionCheckpointOutcomes>();
        app.init_resource::<PendingLifecycleCommit>();

        let key = SessionCheckpointOperations::default()
            .admit(None)
            .expect("a fresh counter mints a key");
        app.world_mut()
            .resource_mut::<AcceptedCheckpointRestore>()
            .accept(AcceptedRestore {
                key,
                frame: 0,
                intent: crossing(),
                occurrences: Default::default(),
                custody: Default::default(),
                item: None,
            });
        assert!(app
            .world_mut()
            .resource_mut::<PendingLifecycleCommit>()
            .record(0, crossing())
            .admitted());

        let mut active = ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 0,
            session_scope: None,
            source_room: 0,
            source_room_id: "here".to_string(),
            target_room: 1,
            intent: crossing(),
            construction_plan: None,
            barrier: LoadBarrierRef::new("load", "ready"),
            commit_not_before_tick: 0,
            cover_required: false,
            cover_presented: true,
            phase: RoomTransitionLoadPhase::Failed,
            failure: Some("the destination room could not be prepared".into()),
            asset_work_id: LoadWorkId::new("room-transition.assets:x"),
            staged_actor_names: Vec::new(),
            asset_readiness_complete: false,
            last_asset_progress: None,
            asset_progress_since: None,
            asset_stall_report: None,
            prefetch_hit: false,
            checkpoint_operation: Some(key),
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: None,
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        };
        active.failure = Some("preparation failed".into());
        app.world_mut()
            .resource_mut::<RoomTransitionLoadState>()
            .active = Some(active);

        app.init_resource::<AbandonedCheckpointOperation>();
        // ⭐ BOTH HALVES, because either alone proves nothing. The `Update` noter
        // may not touch rollback state and the commit executor never sees the
        // transaction; the terminal road only exists when they are composed.
        app.add_systems(bevy::prelude::Update, abandon_failed_checkpoint_restore_system);
        app.add_systems(
            bevy::prelude::Update,
            terminalize_abandoned_checkpoint_restore_system
                .after(abandon_failed_checkpoint_restore_system),
        );
        app.update();

        // ── ONE TERMINAL ANSWER, AND IT SAYS WHY ────────────────────────────
        let outcome = *app
            .world()
            .resource::<SessionCheckpointOutcomes>()
            .outcome_for(key)
            .expect("a failed preparation must terminate its operation");
        assert_eq!(
            outcome.cancellation(),
            Some(RestoreCancellation::PreparationFailed),
            "the operation ended without saying why, or ended as a FAILURE — \
             which would claim destructive application had run and left the world \
             unverified. Nothing was applied: this is a cancellation"
        );
        assert!(
            outcome.failure().is_none(),
            "a cancellation before destructive application must not report a \
             verification failure"
        );

        // ── AND IT WILL NOT BE TRIED AGAIN ──────────────────────────────────
        assert!(
            app.world()
                .resource::<AcceptedCheckpointRestore>()
                .accepted()
                .is_none(),
            "the candidate outlived the operation it belonged to"
        );
        assert!(
            app.world()
                .resource::<PendingLifecycleCommit>()
                .peek()
                .is_none(),
            "the lifecycle intent stayed pending after its preparation failed, so \
             readiness opens a NEW transaction for it next frame — against the \
             same invalid definition, forever"
        );

        // ⛔ IDEMPOTENT. The transaction is still `Failed` this frame; a second
        // pass must not publish a second answer for one operation.
        app.update();
        assert_eq!(
            app.world()
                .resource::<SessionCheckpointOutcomes>()
                .outcome_for(key)
                .map(|outcome| outcome.cancellation()),
            Some(Some(RestoreCancellation::PreparationFailed)),
            "a second terminal answer was published for one operation"
        );
    }

    /// ⛔ A FAILED DOOR IS NOT THIS SYSTEM'S BUSINESS. A transient preparation
    /// failure on an ordinary crossing may legitimately be retried; only the
    /// checkpoint road's protocol calls an invalid definition terminal, and only
    /// it recorded the intent this would spend.
    #[test]
    fn a_failed_ordinary_crossing_is_left_alone() {
        let mut app = App::new();
        app.init_resource::<RoomTransitionLoadState>();
        app.init_resource::<AcceptedCheckpointRestore>();
        app.init_resource::<SessionCheckpointOutcomes>();
        app.init_resource::<PendingLifecycleCommit>();
        assert!(app
            .world_mut()
            .resource_mut::<PendingLifecycleCommit>()
            .record(0, crossing())
            .admitted());

        let active = ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 0,
            session_scope: None,
            source_room: 0,
            source_room_id: "here".to_string(),
            target_room: 1,
            intent: crossing(),
            construction_plan: None,
            barrier: LoadBarrierRef::new("load", "ready"),
            commit_not_before_tick: 0,
            cover_required: false,
            cover_presented: true,
            phase: RoomTransitionLoadPhase::Failed,
            failure: Some("preparation failed".into()),
            asset_work_id: LoadWorkId::new("room-transition.assets:x"),
            staged_actor_names: Vec::new(),
            asset_readiness_complete: false,
            last_asset_progress: None,
            asset_progress_since: None,
            asset_stall_report: None,
            prefetch_hit: false,
            // No checkpoint operation: an ordinary door.
            checkpoint_operation: None,
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: None,
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        };
        app.world_mut()
            .resource_mut::<RoomTransitionLoadState>()
            .active = Some(active);
        app.init_resource::<AbandonedCheckpointOperation>();
        app.add_systems(bevy::prelude::Update, abandon_failed_checkpoint_restore_system);
        app.add_systems(
            bevy::prelude::Update,
            terminalize_abandoned_checkpoint_restore_system
                .after(abandon_failed_checkpoint_restore_system),
        );
        app.update();

        assert!(
            app.world()
                .resource::<PendingLifecycleCommit>()
                .peek()
                .is_some(),
            "an ordinary crossing's intent was spent by the checkpoint road's \
             terminalization, so a retryable door failure became permanent"
        );
        assert!(app
            .world()
            .resource::<SessionCheckpointOutcomes>()
            .latest()
            .is_none());
    }
}

#[cfg(test)]
mod checkpoint_failure_installation_tests {
    /// ⛔⛔ **THE LOGIC TEST ABOVE ADDS THE SYSTEM ITSELF, so it says nothing
    /// about whether anything runs it.** A terminalization nobody installs is a
    /// failed preparation that retries forever, and that test would be green
    /// throughout. Ask the SCHEDULE.
    ///
    /// ⚠ IT COUNTS RATHER THAN NAMES, because a system's name is
    /// `"<Enable the debug feature to see the name>"` in an ordinary build —
    /// measured, after writing the naming version first. A count fires on
    /// REMOVAL, which is the regression that matters here; it cannot say WHICH
    /// four, so the four are named in this comment instead:
    ///
    /// ```text
    /// begin_room_transition_load_system
    /// authorize_ready_room_transition_system
    /// abandon_failed_checkpoint_restore_system      <- the one this defends
    /// finalize_unpresented_room_transition_failure_system
    /// ```
    ///
    /// ⭐ THEIR ORDER NEEDS NO ASSERTION: the host installs them as one
    /// `.chain()`, so the terminalization runs before the teardown that takes
    /// the transaction away by construction. What could silently go is
    /// MEMBERSHIP.
    #[test]
    fn the_readiness_chain_still_carries_the_checkpoint_terminalization() {
        use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};
        use bevy::prelude::{App, Update};

        let mut app = App::new();
        app.add_plugins(super::super::RoomTransitionComposerPlugin);
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules
            .get(Update)
            .expect("the plugin installs Update systems")
            .graph();
        let set = NodeId::Set(
            graph
                .system_sets
                .get_key(super::super::RoomTransitionReadinessSet.intern())
                .expect("the readiness set is registered"),
        );
        let members = graph
            .systems
            .iter()
            .filter(|(key, _, _)| {
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(set, NodeId::System(*key))
            })
            .count();
        assert_eq!(
            members, 4,
            "the room-transition readiness chain does not hold four systems. If \
             the checkpoint terminalization is the one that left, a failed \
             preparation keeps its lifecycle intent and readiness reopens it \
             every frame, against the same invalid definition, forever"
        );
    }

    /// ⛔⛔ **AND THE OTHER HALF, WHICH THE COUNT ABOVE CANNOT SEE.** The
    /// readiness chain only NOTES the abandoned operation; if the commit-side
    /// terminalizer is not installed, the note is never spent and the defect is
    /// exactly the one the split was supposed to keep fixed — with the readiness
    /// count still green at four. Two sets, two counts.
    ///
    /// ```text
    /// advance_room_transition_content_epoch_system
    /// commit_ready_room_transition_system
    /// apply_committed_room_transition_restore
    /// terminalize_abandoned_checkpoint_restore_system   <- the one this defends
    /// ```
    #[test]
    fn the_commit_chain_still_carries_the_checkpoint_terminalization() {
        use ambition_platformer2d_shared_tangle::schedule::{RoomTransitionSet, SimScheduleExt};
        use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};
        use bevy::prelude::App;

        let mut app = App::new();
        app.add_plugins(super::super::RoomTransitionComposerPlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules
            .get(sim)
            .expect("the plugin installs sim systems")
            .graph();
        let set = NodeId::Set(
            graph
                .system_sets
                .get_key(RoomTransitionSet::Apply.intern())
                .expect("the apply set is registered"),
        );
        let members = graph
            .systems
            .iter()
            .filter(|(key, _, _)| {
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(set, NodeId::System(*key))
            })
            .count();
        assert_eq!(
            members, 4,
            "the room-transition commit chain does not hold four systems. If the \
             abandoned-operation terminalizer is the one that left, readiness \
             still takes the key off the failed transaction and NOTHING ever \
             answers it — the accepted restore and its lifecycle intent stay \
             live forever, which is the wedge with one more indirection"
        );
    }
}
