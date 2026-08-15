//! Readiness-gated ordinary room transitions.
//!
//! A detected loading-zone crossing no longer tears down the active room in the
//! same system pass. It first becomes an exact [`ambition_load`] transaction,
//! preflights the target without mutating room authority, and may commit only on
//! a later simulation tick after the required barrier is ready and one-shot
//! authorization succeeds.
//!
//! The transition now carries both construction preflight and concrete room
//! presentation readiness. Target construction is still synchronous at commit
//! time, but visible hosts cover it before authorization and neighboring-room
//! prefetch can promote the same exact asset handles into this transaction.

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::{DetectChanges, MessageWriter, NextState, Res, ResMut, Resource};

use ambition_load::{
    BarrierReadiness, LoadBarrierRef, LoadBarrierSpec, LoadCommitRejection, LoadCoordinator,
    LoadEvent, LoadFailure, LoadId, LoadPlanSpec, LoadWorkId, LoadWorkSpec, LoadWorkState,
};
use ambition_platformer2d_actor_monolith::rooms;
use ambition_platformer2d_actor_monolith::session::lifecycle_commit::RoomTransitionIntent;
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
/// When absent, a headless transition may commit as soon as readiness is
/// authorized. When present, the visible adapter must prove an opaque cover
/// survived a presentation frame before the synchronous commit can begin.
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
/// Present ⇒ the engine leaves the item `Running` and a host system
/// (`ambition_app`'s `contribute_room_transition_assets`) owns resolving it.
/// Absent ⇒ the engine skips it as it always did for headless.
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
/// ⛔⛔ **THE ROOM SET IS COMPARED BY VALUE, NOT BY `is_changed()`, AND THAT IS
/// NOT A STYLE CHOICE.** `RoomSet` is rollback-registered state, and a rollback
/// host RESTORES it every frame — so `is_changed()` is TRUE every frame there,
/// whatever the content is doing. This system's write is not idempotent (a bump
/// is a bump), which is exactly the *"change ticks don't rewind"* trap.
///
/// It was invisible until D71 (2026-08-14), because the rollback host opened no
/// readiness transactions at all. The moment it did, the epoch advanced every
/// frame, `same_destination` therefore matched nothing, and `begin` minted a
/// FRESH transaction every frame that superseded the one before it — measured as
/// `seq=1,2,3,… epoch=18,19,20,…`, a transaction that reached `Ready` and could
/// never reach the next tick's commit gate because it was replaced first.
///
/// The registries below are ordinary content resources, are not restored, and
/// keep their change-tick reads. The room set is compared against the ids it
/// last had: a restore reproduces them exactly and bumps nothing, while a
/// hot-reload that changes the world's rooms still bumps.
pub fn advance_room_transition_content_epoch_system(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<rooms::RoomSet>,
    placement_lowering: Res<
        ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry,
    >,
    content_staging: Res<
        ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry,
    >,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    boss_catalog: Res<ambition_platformer2d_actor_monolith::boss_encounter::BossCatalog>,
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
    pub target_room_id: String,
    /// The destination's INDEX in the live `RoomSet`, resolved once from
    /// [`Self::intent`]'s authored room id. Host-side only — the intent names the
    /// room by id because it is rollback state and an index is not stable across
    /// a content reload.
    pub target_room: usize,
    /// **What this transaction is FOR** — the semantic crossing, subject
    /// included, exactly as detection recorded it.
    ///
    /// ⭐ this was a `RoomTransitionRequested` message (D71, 2026-08-14). The
    /// message described the same crossing a second time, with a resolved zone
    /// and no subject, and only an eager host ever produced one — which is why
    /// the shipped rollback host opened no transaction at all and every room
    /// change in the game went uncovered.
    pub intent: RoomTransitionIntent,
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
    /// **When `last_asset_progress` last MOVED**, on the feel clock — the only
    /// thing that separates "waiting" from "stuck".
    ///
    /// ⛔ **a barrier that knows it is blocked must be able to say what on, and
    /// this one could not.** Jon's browser sat at 99% entering Hall of
    /// Characters (a room staging 129 distinct characters), and 99% means only
    /// *"not Ready"* — `LoadPresentationModel::from_snapshot` clamps every
    /// un-Ready barrier to `0.999`. The poll computes `RoomAssetReadiness`,
    /// which carries `pending` BY NAME, and then keeps `(settled, total)` and
    /// drops the names. Every frame. So a barrier correctly waiting on the
    /// slowest of 129 fetches and one deadlocked on an asset that will never
    /// arrive looked identical, and they want opposite fixes.
    pub asset_progress_since: Option<Duration>,
    /// **The explanation for the current stall, once it has earned one** — which
    /// assets are still pending, how many, and for how long.
    ///
    /// ⭐ STATE rather than only a log line, and deliberately: a test can assert
    /// on it, a dev overlay can show it, and it stays available for whoever asks
    /// next instead of scrolling past once. `None` while the barrier is moving,
    /// which is also what makes it the "already explained" flag — one report per
    /// stall, not one per frame.
    pub asset_stall_report: Option<String>,
    pub prefetch_hit: bool,
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
    /// **Record what the asset barrier has settled, and keep the stall clock
    /// honest.** Returns whether the count actually MOVED.
    ///
    /// ⛔ **two systems write this** — the contributor writes it once when it
    /// builds the manifest, the poll writes it every time it changes — and the
    /// stall clock was first taught to only one of them. The result was a
    /// barrier stuck at `(0, 164)` whose `asset_progress_since` was `None`
    /// forever, so it could never become old enough to explain itself: the
    /// contributor had already stored the key the poll would have called a
    /// change. Caught by the Hall pin on its first run, 2026-08-15.
    ///
    /// ⭐ so the pair moves together or not at all. A caller cannot record
    /// progress without restarting the clock, because recording IS this call.
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
        intent: &RoomTransitionIntent,
        session_scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        content_epoch: u64,
    ) -> bool {
        // ⭐ **the SEMANTIC destination, which is not the zone and not the room.**
        //
        // This ANDed in `zone.id` while its own caller's comment said *"one
        // transaction owns that destination; trigger noise is not a new
        // request"* — so two zones into one room opened two transactions, the
        // comment's opposite. ⛔ but the fix is not "compare target_room": two
        // doors can lead to the SAME room at DIFFERENT arrivals, and a room-only
        // key silently lands the second crossing at the first one's coordinates.
        //
        // So the key is what a crossing IS: who is crossing, where they are
        // going, where they come out, and by which kind of exit. Repeated
        // detection from one zone repeats all four and dedupes; two exits into
        // one room differ in `arrival` and do not; and once a second participant
        // can transit, two bodies differ in `subject` and do not either — which
        // is why `subject` is in the KEY and not merely in the payload.
        //
        // ⚠ `arrival` is compared exactly, and that is correct rather than
        // sloppy: both sides come from the same authored `RoomSpec` field by the
        // same path, so trigger noise reproduces the same bits. A near-miss here
        // means a genuinely different destination.
        self.content_epoch == content_epoch
            && self.session_scope == session_scope
            && self.intent.subject == intent.subject
            && self.intent.target_room == intent.target_room
            && self.intent.arrival == intent.arrival
            && self.intent.edge_exit == intent.edge_exit
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

/// Convert a post-authorization commit precondition failure into ordinary load
/// evidence. The source room is still intact at every caller, so visible hosts
/// can offer retry/cancel and headless hosts can retire the failed transaction.
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

/// **The confirmed crossing waiting to be loaded**, or nothing.
///
/// Two resources that are always read together and mean nothing apart: the
/// pending intent, and how far the host has confirmed. Bundled because
/// [`begin_room_transition_load_system`] sits at Bevy's 16-parameter ceiling and
/// because "is this crossing safe to act on yet" is one question, not two.
///
/// ⚠ **a host with no `ConfirmedFrameBoundary` confirms on arrival**, which is
/// not a special case: an eager host has no speculative frames, so nothing it
/// records can be un-recorded. That is the whole of the host difference.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ConfirmedRoomTransitionIntent<'w> {
    pending: Res<
        'w,
        ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit,
    >,
    boundary: Option<Res<'w, ambition_platformer2d_core::ConfirmedFrameBoundary>>,
}

impl ConfirmedRoomTransitionIntent<'_> {
    /// Whether this host defers on confirmed frames. The one thing about the
    /// host that the readiness transaction has to know: what it may do to state
    /// the simulation reads.
    fn is_rollback_host(&self) -> bool {
        self.boundary.is_some()
    }

    fn get(&self) -> Option<&RoomTransitionIntent> {
        let confirmed = self
            .boundary
            .as_deref()
            .map_or(i32::MAX, |boundary| boundary.confirmed);
        match &self.pending.confirmed(confirmed)?.kind {
            ambition_platformer2d_actor_monolith::session::lifecycle_commit::LifecycleIntent::Transition(transition) => Some(transition),
            // The in-place resets are not transitions and open no transaction;
            // the confirmed-lifecycle committer owns them.
            _ => None,
        }
    }
}

/// Convert loading-zone detections into exact load transactions and perform the
/// mutation-free target preflight.
///
/// Construction preflight is mutation-free, and presentation-capable hosts
/// attach a concrete room asset manifest whose Bevy handles remain required
/// work until they settle. The old room remains authoritative until a later
/// tick observes the complete barrier and receives one-shot authorization.
#[allow(clippy::too_many_arguments)]
pub fn begin_room_transition_load_system(
    // **The one description of a crossing**, recorded by whatever detected it
    // (a loading zone, a checkpoint resume, a level flag, a retry) and read here
    // only once its originating frame can never be simulated again.
    //
    // ⭐ this read a `MessageReader<RoomTransitionRequested>` until 2026-08-14,
    // and only an EAGER host ever wrote one — so the shipped rollback host
    // recorded intents that nothing opened a transaction for, and every room
    // change in the game happened with no cover, no failure reporting and no
    // asset accounting (D71). ⚠ a host with no `ConfirmedFrameBoundary` has no
    // speculative frames, so everything it records is confirmed on arrival.
    pending: ConfirmedRoomTransitionIntent,
    mut state: ResMut<RoomTransitionLoadState>,
    content_epoch: Res<RoomTransitionContentEpoch>,
    active_binding: Option<Res<ambition_platformer2d_actor_monolith::rooms::ActiveContentBinding>>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<rooms::RoomSet>,
    construction_services: (
        Res<ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry>,
        Res<ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>,
        Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
        Res<ambition_platformer2d_actor_monolith::boss_encounter::BossCatalog>,
        Res<ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry>,
        // Provider-authored sheets (U1 stage B): room construction sizes bodies
        // from their sheets, so a transition needs it beside the catalog.
        Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    ),
    // The prepared cast (D73 phase 1), so an NPC rebuilt by a transition still
    // takes its CHARACTER's default autonomous profile rather than only its
    // catalog row's. ⚠ `Option`, and absence is a legal answer: a composition
    // with no registered characters is the ordinary case, and an empty registry
    // means "no character states a default" — which is what this route assumed
    // before a definition could state one.
    //
    // ⛔ outside the `construction_services` tuple deliberately: that tuple reads
    // positionally at the call site (`construction_services.5`), so one more
    // member would be one more number for a reader to decode.
    // ⚠ **PAIRED, and only because a Bevy system stops at sixteen params.** The
    // two authorities travel together anyway: a placement names a character and
    // may name the policy that drives it.
    character_authorities: (
        Option<
            Res<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>,
        >,
        // **The published controller policies**, so an enemy placement may name
        // one. A composition that publishes none is ordinary, and a placement
        // naming into an absent registry is what refuses.
        Option<Res<ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
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
    let (prepared_characters, brain_profiles) = character_authorities;
    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    let Some(intent) = pending.get() else {
        return;
    };
    {
        if state.active.as_ref().is_some_and(|active| {
            active.same_destination(intent, current_session, content_epoch.get())
        }) {
            // The intent SURVIVES until its commit moves the body, so this system
            // sees it again every frame in between. One transaction owns that
            // destination; a still-pending crossing is not a new one.
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
        let target_index = room_set.room_index_by_id(&intent.target_room);
        let target_label = target_index
            .and_then(|index| room_set.rooms.get(index))
            .map(|room| room.id.clone())
            .unwrap_or_else(|| intent.target_room.clone());
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
        // ⛔⛔ **A ROLLBACK HOST DOES NOT PAUSE FOR ITS OWN LOADING SCREEN.**
        //
        // `GameMode::RoomTransition` fails `gameplay_allowed`, which gates
        // `detect_room_transition_system` and other SIM systems. Requesting it
        // from here — host-side, outside the rewound schedule — makes the
        // simulation's behaviour depend on non-rollback state: frames resimulated
        // after the mode flipped skip work the original pass did, and the
        // sync-test checksum diverges. Measured 2026-08-14: mismatch at frames
        // [15, 16, 17] the moment the shipped host started opening transactions.
        //
        // ⭐ and it is not merely unsound, it is wrong for the thing this
        // enables: peers do not stop simulating because one of them is loading.
        // The COVER still goes up — it is driven off `RoomTransitionLoadState`,
        // not off the mode — so the player sees the same screen; the world
        // behind it keeps its own time.
        if !pending.is_rollback_host() {
            next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::RoomTransition);
        }

        let cover_required = presentation_available.is_some();
        // **A transition always says it started.**
        //
        // The completion telemetry
        // (`ambition_platformer2d::room_transition::performance`) is emitted at RETIREMENT,
        // behind `runtime.owner.take()`, so a transition that never spawned a
        // cover reports nothing at all — and a 34.6s desktop capture containing a
        // real hub -> Hall transition produced ZERO lines from that target while
        // this module's other logs printed fine (2026-07-30). An instrument that
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
            target_room_id: target_label,
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
                active.source_room_id, intent.target_room,
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

        if !intent.arrival.is_finite() {
            let detail = format!(
                "transition into '{}' has non-finite arrival {:?}",
                target_spec.id, intent.arrival,
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
        // ⛔ **THIS WAS `#[cfg(not(target_arch = "wasm32"))] std::time::Instant`,
        // so the one platform whose numbers would explain a slow transition was
        // the one platform that recorded none.** `bevy::platform::time::Instant`
        // is `web-time` on wasm and `std` elsewhere, already in the graph, and
        // sub-frame on both — `Time<Real>` is NOT a substitute here, because it
        // advances once per frame and a within-frame span measures zero.
        let construction_preflight_started = bevy::platform::time::Instant::now();
        let prefetched_construction = plan_prefetch.as_deref_mut().and_then(|cache| {
            cache.promote(
                content_epoch.get(),
                current_session,
                &active.source_room_id,
                target_spec,
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
                &construction_services.2,
                &construction_services.5,
                &construction_services.3,
                session_scope,
                // A transition rebuilds a room the ACTIVE content already
                // defines, so the plan states the session's LIVE binding — the
                // transition-local counter is a prefetch cache key, not a
                // content generation (same fix reset received). Plus the cast,
                // and the policies a PLACEMENT may name: a room whose enemy
                // spawn authors `brain_profile` resolves it against those.
                ambition_platformer2d_actor_monolith::features::ActorConstructionContext::for_room_construction(
                    &construction_services.4,
                    ambition_platformer2d_core::ContentEpoch(content_epoch.get()),
                    active_binding.as_deref(),
                    prepared_characters.as_deref(),
                    brain_profiles.as_deref(),
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

    fn request(target_room: &str) -> RoomTransitionIntent {
        request_by(target_room, ae::Vec2::ZERO, hero())
    }

    fn request_by(target_room: &str, arrival: ae::Vec2, subject: SimId) -> RoomTransitionIntent {
        RoomTransitionIntent {
            subject,
            target_room: target_room.to_string(),
            arrival,
            edge_exit: false,
            zone_sfx: None,
        }
    }

    #[test]
    fn repeated_zone_detection_is_one_destination() {
        let active = ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 1,
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room_id: "b".to_string(),
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
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: None,
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        };
        // Trigger noise: the same body re-detecting the same crossing. The
        // intent SURVIVES until its commit moves the body, so the transaction
        // sees it again every frame in between.
        assert!(active.same_destination(&request("b"), None, 1));
        // ⛔⛔ **THE POISON, and the reason the key is not just `target_room`.**
        // Two exits can lead to the same room at different arrivals. A room-only
        // key collapses them and lands this crossing at the other one's
        // coordinates.
        assert!(!active.same_destination(
            &request_by("b", ae::Vec2::new(0.0, 64.0), hero()),
            None,
            1
        ));
        // ⛔ and a DIFFERENT BODY crossing to the same place is a different
        // crossing, not noise. Nothing can trigger this today — one participant
        // transits — which is why it is asserted rather than discovered later.
        assert!(!active.same_destination(
            &request_by("b", ae::Vec2::ZERO, SimId::placement("other_body")),
            None,
            1
        ));
        assert!(!active.same_destination(&request("c"), None, 1));
        assert!(!active.same_destination(&request("b"), None, 2));
        assert!(!active.same_destination(
            &request("b"),
            Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(9)),
            1,
        ));
    }

    #[test]
    fn visible_transition_requires_cover_acknowledgment() {
        let mut active = ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 1,
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room_id: "b".to_string(),
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
