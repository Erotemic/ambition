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

use bevy::prelude::{
    DetectChanges, MessageReader, MessageWriter, NextState, Res, ResMut, Resource,
};

use ambition_actors::rooms;
use ambition_load::{
    BarrierReadiness, LoadBarrierRef, LoadBarrierSpec, LoadCommitRejection, LoadCoordinator,
    LoadEvent, LoadFailure, LoadId, LoadPlanSpec, LoadWorkId, LoadWorkSpec, LoadWorkState,
};
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
pub fn advance_room_transition_content_epoch_system(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<rooms::RoomSet>,
    placement_lowering: Res<ambition_actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Res<ambition_actors::features::RoomContentStagingRegistry>,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<ambition_actors::features::CharacterRoster>,
    boss_catalog: Res<ambition_actors::boss_encounter::BossCatalog>,
    mut epoch: ResMut<RoomTransitionContentEpoch>,
) {
    if room_set.is_changed()
        || placement_lowering.is_changed()
        || content_staging.is_changed()
        || character_catalog.is_changed()
        || character_roster.is_changed()
        || boss_catalog.is_changed()
    {
        epoch.bump();
    }
}

#[derive(Clone, Debug)]
pub struct ActiveRoomTransitionLoad {
    pub sequence: u64,
    pub content_epoch: u64,
    pub session_scope: Option<ambition_platformer_primitives::lifecycle::SessionScopeId>,
    pub source_room: usize,
    pub source_room_id: String,
    pub target_room_id: String,
    pub request: rooms::RoomTransitionRequested,
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
    fn same_destination(
        &self,
        request: &rooms::RoomTransitionRequested,
        session_scope: Option<ambition_platformer_primitives::lifecycle::SessionScopeId>,
        content_epoch: u64,
    ) -> bool {
        self.content_epoch == content_epoch
            && self.session_scope == session_scope
            && self.request.transition.target_room == request.transition.target_room
            && self.request.transition.zone.id == request.transition.zone.id
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
    bevy::log::error!(target: "ambition::room_transition", "{detail}");
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
    mut requests: MessageReader<rooms::RoomTransitionRequested>,
    mut state: ResMut<RoomTransitionLoadState>,
    content_epoch: Res<RoomTransitionContentEpoch>,
    active_binding: Option<Res<ambition_actors::rooms::ActiveContentBinding>>,
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<rooms::RoomSet>,
    construction_services: (
        Res<ambition_actors::world::placements::PlacementLoweringRegistry>,
        Res<ambition_actors::features::RoomContentStagingRegistry>,
        Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
        Res<ambition_actors::features::CharacterRoster>,
        Res<ambition_actors::boss_encounter::BossCatalog>,
        Res<ambition_actors::construction::ActorConstructionRegistry>,
        // Provider-authored sheets (U1 stage B): room construction sizes bodies
        // from their sheets, so a transition needs it beside the catalog.
        Res<ambition_actors::character_sprites::AuthoredSheets>,
    ),
    asset_contributor: Option<Res<RoomTransitionAssetContributor>>,
    mut plan_prefetch: Option<ResMut<super::prefetch::RoomConstructionPlanPrefetch>>,
    real_time: Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
    active_session: Option<Res<ambition_platformer_primitives::lifecycle::ActiveSessionScope>>,
    presentation_available: Option<Res<RoomTransitionPresentationAvailable>>,
    tick: Res<SimTick>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: MessageWriter<LoadEvent>,
    mut next_mode: ResMut<NextState<ambition_platformer_primitives::schedule::GameMode>>,
) {
    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    for request in requests.read() {
        if state.active.as_ref().is_some_and(|active| {
            active.same_destination(request, current_session, content_epoch.get())
        }) {
            // Zone overlap may emit every tick until the eventual commit moves
            // the body. One transaction owns that destination; trigger noise is
            // not a new request.
            continue;
        }

        let superseded = state.active.take().map(|active| active.barrier.load_id);
        let sequence = state.mint_sequence();
        let source_room = room_set.active;
        let source_room_id = room_set
            .rooms
            .get(source_room)
            .map(|room| room.id.clone())
            .unwrap_or_else(|| format!("<room-index-{source_room}>"));
        let target_label = room_set
            .rooms
            .get(request.transition.target_room)
            .map(|room| room.id.clone())
            .unwrap_or_else(|| format!("<room-index-{}>", request.transition.target_room));
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

        next_mode.set(ambition_platformer_primitives::schedule::GameMode::RoomTransition);

        let cover_required = presentation_available.is_some();
        let mut active = ActiveRoomTransitionLoad {
            sequence,
            content_epoch: content_epoch.get(),
            session_scope: current_session,
            source_room,
            source_room_id,
            target_room_id: target_label,
            request: request.clone(),
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

        let Some(target_spec) = room_set.rooms.get(request.transition.target_room) else {
            let detail = format!(
                "transition from '{}' targets missing room index {}",
                active.source_room_id, request.transition.target_room,
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
            bevy::log::error!(target: "ambition::room_transition", "{detail}");
            state.active = Some(active);
            continue;
        };
        set_work_state(
            &mut loads,
            &mut load_events,
            &load_id,
            TARGET_LOOKUP_WORK,
            LoadWorkState::Complete,
        );

        if !request.transition.arrival.is_finite() {
            let detail = format!(
                "transition into '{}' has non-finite arrival {:?}",
                target_spec.id, request.transition.arrival,
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
            bevy::log::error!(target: "ambition::room_transition", "{detail}");
            state.active = Some(active);
            continue;
        }
        set_work_state(
            &mut loads,
            &mut load_events,
            &load_id,
            ARRIVAL_VALIDATION_WORK,
            LoadWorkState::Complete,
        );

        let Some(session_scope) =
            ambition_platformer_primitives::lifecycle::SessionSpawnScope::for_optional_active_session(
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
            bevy::log::error!(target: "ambition::room_transition", "{detail}");
            state.active = Some(active);
            continue;
        };
        // AMBITION_REVIEW(determinism): same as `commit_duration` — wall clock
        // feeding the write-only `construction_preflight_duration` diagnostic,
        // observed by no sim decision and not rollback-registered. Preflight cost
        // is a player-facing latency question, so the feel clock is the right one.
        #[cfg(not(target_arch = "wasm32"))]
        let construction_preflight_started = std::time::Instant::now();
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
                request.transition.target_room,
                &construction_services.0,
                &construction_services.1,
                &construction_services.2,
                &construction_services.6,
                &construction_services.3,
                &construction_services.4,
                session_scope,
                {
                    let mut context = ambition_actors::features::ActorConstructionContext::new(
                        &construction_services.5,
                        ambition_engine_core::ContentEpoch(content_epoch.get()),
                    );
                    // A transition rebuilds a room the ACTIVE content already
                    // defines, so the plan states the session's LIVE binding —
                    // the transition-local counter is a prefetch cache key,
                    // not a content generation (same fix reset received).
                    if let Some(active) = active_binding.as_deref() {
                        context.binding = active.0;
                    }
                    context
                },
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
                bevy::log::error!(target: "ambition::room_transition", "{detail}");
                state.active = Some(active);
                continue;
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            active.construction_preflight_duration = Some(construction_preflight_started.elapsed());
        }
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
                    bevy::log::error!(target: "ambition::room_transition", "{detail}");
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
            bevy::log::error!(target: "ambition::room_transition", "{detail}");
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
    mut next_mode: ResMut<NextState<ambition_platformer_primitives::schedule::GameMode>>,
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
    next_mode.set(ambition_platformer_primitives::schedule::GameMode::Playing);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;
    use ambition_world::rooms::{LoadingZone, LoadingZoneActivation, RoomTransition};

    fn request(zone: &str, target_room: usize) -> rooms::RoomTransitionRequested {
        rooms::RoomTransitionRequested::new(
            RoomTransition {
                zone: LoadingZone {
                    id: zone.to_string(),
                    name: zone.to_string(),
                    activation: LoadingZoneActivation::Door,
                    aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::ONE),
                },
                target_room,
                arrival: ae::Vec2::ZERO,
            },
            None,
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
            target_room_id: "b".to_string(),
            request: request("door", 1),
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
        assert!(active.same_destination(&request("door", 1), None, 1));
        assert!(!active.same_destination(&request("other", 1), None, 1));
        assert!(!active.same_destination(&request("door", 2), None, 1));
        assert!(!active.same_destination(&request("door", 1), None, 2));
        assert!(!active.same_destination(
            &request("door", 1),
            Some(ambition_platformer_primitives::lifecycle::SessionScopeId(9)),
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
            request: request("door", 1),
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
