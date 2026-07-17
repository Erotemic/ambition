//! Readiness-gated ordinary room transitions.
//!
//! A detected loading-zone crossing no longer tears down the active room in the
//! same system pass. It first becomes an exact [`ambition::load`] transaction,
//! preflights the target without mutating room authority, and may commit only on
//! a later simulation tick after the required barrier is ready and one-shot
//! authorization succeeds.
//!
//! This is Phase 2 of `docs/planning/engine/room-transition-loading.md`. The
//! target construction is still synchronous at commit time; Phase 3 adds an
//! opaque rendered cover before known-expensive commits and conditionally
//! reveals the generic loading foreground.

use bevy::prelude::{
    AssetServer, MessageReader, MessageWriter, Res, ResMut, Resource,
};

use ambition::actors::rooms;
use ambition::load::{
    BarrierReadiness, LoadBarrierRef, LoadBarrierSpec, LoadCommitRejection,
    LoadCoordinator, LoadEvent, LoadFailure, LoadId, LoadPlanSpec, LoadWorkId, LoadWorkSpec,
    LoadWorkState,
};
use ambition::time::SimTick;

const ROOM_READY_BARRIER: &str = "room-transition.ready";
const TARGET_LOOKUP_WORK: &str = "room-transition.target-lookup";
const ARRIVAL_VALIDATION_WORK: &str = "room-transition.arrival-validation";
const CONSTRUCTION_PREFLIGHT_WORK: &str = "room-transition.construction-preflight";
const PRESENTATION_REQUEST_WORK: &str = "room-transition.presentation-request";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RoomTransitionLoadPhase {
    AwaitingReadiness,
    CommitAuthorized,
    Failed,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveRoomTransitionLoad {
    pub(crate) sequence: u64,
    pub(crate) session_scope: Option<ambition::platformer::lifecycle::SessionScopeId>,
    pub(crate) source_room: usize,
    pub(crate) source_room_id: String,
    pub(crate) target_room_id: String,
    pub(crate) request: rooms::RoomTransitionRequested,
    pub(crate) barrier: LoadBarrierRef,
    pub(crate) commit_not_before_tick: u64,
    pub(crate) phase: RoomTransitionLoadPhase,
    pub(crate) failure: Option<String>,
}

impl ActiveRoomTransitionLoad {
    fn same_destination(
        &self,
        request: &rooms::RoomTransitionRequested,
        session_scope: Option<ambition::platformer::lifecycle::SessionScopeId>,
    ) -> bool {
        self.session_scope == session_scope
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
pub(crate) struct RoomTransitionLoadState {
    next_sequence: u64,
    pub(crate) active: Option<ActiveRoomTransitionLoad>,
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
    command: ambition::load::LoadCommand,
) {
    for event in loads.apply(command) {
        events.write(event);
    }
}

fn set_work_state(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    load_id: &LoadId,
    work_id: &str,
    state: LoadWorkState,
) {
    apply_load_command(
        loads,
        events,
        ambition::load::LoadCommand::SetWorkState {
            load_id: load_id.clone(),
            work_id: LoadWorkId::new(work_id),
            state,
        },
    );
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
        LoadWorkState::Failed(
            LoadFailure::new(player_message, developer_detail).retryable(true),
        ),
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
        ambition::load::LoadCommand::SetDiscovery {
            load_id: barrier.load_id.clone(),
            barrier_id: barrier.barrier_id.clone(),
            open: false,
            forecast: None,
        },
    );
}

/// Convert loading-zone detections into exact load transactions and perform the
/// mutation-free target preflight.
///
/// All preparation in this phase is intentionally cheap and synchronous. It
/// establishes the correctness seam first: the old room remains authoritative
/// until a later tick observes a ready barrier and receives one-shot commit
/// authorization. Room asset readiness and asynchronous construction artifacts
/// join the same barrier in later phases.
#[allow(clippy::too_many_arguments)]
pub(crate) fn begin_room_transition_load_system(
    mut requests: MessageReader<rooms::RoomTransitionRequested>,
    mut state: ResMut<RoomTransitionLoadState>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<rooms::RoomSet>,
    content_staging: Res<ambition::actors::features::RoomContentStagingRegistry>,
    placement_lowering: Res<ambition::actors::world::placements::PlacementLoweringRegistry>,
    mut game_assets: Option<ResMut<ambition::sprite_sheet::game_assets::GameAssets>>,
    sandbox_catalog: Option<Res<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>>,
    asset_server: Option<Res<AssetServer>>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ambition::platformer::lifecycle::ActiveSessionScope>>,
    tick: Res<SimTick>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: MessageWriter<LoadEvent>,
) {
    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    for request in requests.read() {
        if state
            .active
            .as_ref()
            .is_some_and(|active| active.same_destination(request, current_session))
        {
            // Zone overlap may emit every tick until the eventual commit moves
            // the body. One transaction owns that destination; trigger noise is
            // not a new request.
            continue;
        }

        let superseded = state
            .active
            .take()
            .map(|active| active.barrier.load_id);
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

        let mut plan = LoadPlanSpec::new(
            load_id.clone(),
            format!("Prepare room {target_label}"),
        );
        plan.supersedes = superseded.clone();
        apply_load_command(
            &mut loads,
            &mut load_events,
            ambition::load::LoadCommand::Begin(plan),
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
            ambition::load::LoadCommand::DeclareBarrier {
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
                PRESENTATION_REQUEST_WORK,
                "Request target presentation assets",
                ROOM_READY_BARRIER,
            ),
        ] {
            apply_load_command(
                &mut loads,
                &mut load_events,
                ambition::load::LoadCommand::UpsertWork {
                    load_id: load_id.clone(),
                    spec,
                },
            );
        }

        let mut active = ActiveRoomTransitionLoad {
            sequence,
            session_scope: current_session,
            source_room,
            source_room_id,
            target_room_id: target_label,
            request: request.clone(),
            barrier: barrier.clone(),
            // Even when every contributor resolves immediately, commit happens
            // on a later simulation step. This makes readiness and commit two
            // real phases and gives Phase 3 a place to insert cover rendering.
            commit_not_before_tick: tick.get().saturating_add(1),
            phase: RoomTransitionLoadPhase::AwaitingReadiness,
            failure: None,
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
            for work in [
                ARRIVAL_VALIDATION_WORK,
                CONSTRUCTION_PREFLIGHT_WORK,
                PRESENTATION_REQUEST_WORK,
            ] {
                set_work_state(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    work,
                    LoadWorkState::Skipped,
                );
            }
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
            for work in [CONSTRUCTION_PREFLIGHT_WORK, PRESENTATION_REQUEST_WORK] {
                set_work_state(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    work,
                    LoadWorkState::Skipped,
                );
            }
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

        let construction_result = placement_lowering
            .validate_room(&target_spec.id, &target_spec.placements)
            .map_err(|err| err.to_string())
            .and_then(|()| {
                content_staging
                    .try_requests_for(target_spec)
                    .map(|_| ())
                    .map_err(|err| err.to_string())
            });
        if let Err(detail) = construction_result {
            fail_work(
                &mut loads,
                &mut load_events,
                &load_id,
                CONSTRUCTION_PREFLIGHT_WORK,
                "The destination room could not be prepared.",
                detail.clone(),
            );
            set_work_state(
                &mut loads,
                &mut load_events,
                &load_id,
                PRESENTATION_REQUEST_WORK,
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
            CONSTRUCTION_PREFLIGHT_WORK,
            LoadWorkState::Complete,
        );

        match (
            game_assets.as_deref_mut(),
            sandbox_catalog.as_deref(),
            asset_server.as_deref(),
        ) {
            (Some(assets), Some(catalog), Some(asset_server)) => {
                ambition::sprite_sheet::game_assets::ensure_parallax_layers_for_room(
                    assets,
                    catalog,
                    asset_server,
                    &target_spec.metadata,
                    quality.as_deref().map(|q| &q.budget),
                );
                set_work_state(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    PRESENTATION_REQUEST_WORK,
                    LoadWorkState::Complete,
                );
            }
            _ => {
                // Headless/minimal hosts have no presentation asset authority.
                // Their room barrier is still honest: this contributor is not
                // applicable rather than silently pretending an asset loaded.
                set_work_state(
                    &mut loads,
                    &mut load_events,
                    &load_id,
                    PRESENTATION_REQUEST_WORK,
                    LoadWorkState::Skipped,
                );
            }
        }
        close_discovery(&mut loads, &mut load_events, &barrier);
        state.active = Some(active);
    }
}

/// Observe the required barrier and obtain one-shot commit authorization.
///
/// The deliberate next-tick gate prevents the old request/apply same-pass path
/// from reappearing even while all current contributors are immediate.
pub(crate) fn authorize_ready_room_transition_system(
    tick: Res<SimTick>,
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
        BarrierReadiness::Ready => match loads.request_commit(
            &active.barrier.load_id,
            &active.barrier.barrier_id,
        ) {
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
        },
        BarrierReadiness::Failed
        | BarrierReadiness::Cancelled
        | BarrierReadiness::Superseded => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition::engine_core as ae;
    use ambition::world::rooms::{LoadingZone, LoadingZoneActivation, RoomTransition};

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
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room_id: "b".to_string(),
            request: request("door", 1),
            barrier: LoadBarrierRef::new("load", "ready"),
            commit_not_before_tick: 1,
            phase: RoomTransitionLoadPhase::AwaitingReadiness,
            failure: None,
        };
        assert!(active.same_destination(&request("door", 1), None));
        assert!(!active.same_destination(&request("other", 1), None));
        assert!(!active.same_destination(&request("door", 2), None));
        assert!(!active.same_destination(
            &request("door", 1),
            Some(ambition::platformer::lifecycle::SessionScopeId(9)),
        ));
    }
}
