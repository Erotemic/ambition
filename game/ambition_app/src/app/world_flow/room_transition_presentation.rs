//! Adaptive visible presentation for ordinary room-transition transactions.
//!
//! The simulation-side coordinator owns readiness and commit authority. This
//! adapter owns only the player-visible cover and generic loading foreground:
//!
//! - every visible transition gets an opaque cover;
//! - the cover must have existed across a rendered frame before the synchronous
//!   room commit may begin;
//! - the generic loading foreground remains in hidden grace for fast loads and
//!   reveals honest barrier evidence only when preparation takes long enough;
//! - the cover remains for one complete target frame after commit, then the
//!   transaction is retired and gameplay resumes.

use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use ambition::load::{LoadCommand, LoadCoordinator, LoadEvent};
use ambition::load_presentation::{
    LoadExperienceSpec, LoadPresentationCommand, LoadPresentationEvent, LoadPresentationModel,
    LoadPresentationOwnerId, LoadPresentationSet, ReadyTransitionPolicy,
};
use ambition::platformer::schedule::GameMode;

use super::room_transition_assets::{
    poll_room_transition_asset_readiness_system, prefetch_neighbor_room_preparation_system,
    RoomPreparationPrefetchState,
};
use super::room_transition_loading::{
    RoomTransitionLoadPhase, RoomTransitionLoadState, RoomTransitionPresentationAvailable,
};

const ROOM_TRANSITION_EXPERIENCE: &str = "ambition.room-transition";

/// Visible-host policy for adaptive room-transition presentation.
///
/// The opaque cover is immediate and correctness-critical. The explicit loading
/// foreground is delayed so a normal room change does not flash a progress UI.
#[derive(Resource, Clone, Debug)]
pub(crate) struct RoomTransitionPresentationConfig {
    pub(crate) loading_reveal_after: Duration,
    pub(crate) minimum_visible: Duration,
    /// A commit below this budget should ordinarily be hidden by the normal
    /// transition treatment rather than requiring explicit load foreground.
    pub(crate) no_cover_commit_budget: Duration,
    /// Covered commits above this provisional budget are correctness-safe but
    /// still performance regressions that need construction/render optimization.
    pub(crate) covered_commit_budget: Duration,
}

impl Default for RoomTransitionPresentationConfig {
    fn default() -> Self {
        Self {
            loading_reveal_after: Duration::from_millis(250),
            minimum_visible: Duration::from_millis(300),
            no_cover_commit_budget: Duration::from_millis(4),
            covered_commit_budget: Duration::from_millis(50),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RoomTransitionTimingSample {
    pub(crate) sequence: u64,
    pub(crate) source_room: String,
    pub(crate) target_room: String,
    pub(crate) construction_preflight: Option<Duration>,
    pub(crate) asset_manifest_build: Option<Duration>,
    pub(crate) asset_wait: Option<Duration>,
    pub(crate) request_to_ready: Option<Duration>,
    pub(crate) cover_request_to_presented: Option<Duration>,
    /// Time spent enqueueing the covered construction commit itself.
    pub(crate) commit_enqueue: Option<Duration>,
    /// Wall-clock interval from commit start through the first complete target
    /// presentation frame. This includes deferred-command application and is
    /// the meaningful Hall hitch budget.
    pub(crate) commit_to_first_target_frame: Option<Duration>,
    pub(crate) covered: bool,
    pub(crate) prefetch_hit: bool,
    pub(crate) loading_foreground_visible: bool,
    pub(crate) loading_foreground_visible_duration: Duration,
}

/// Bounded evidence for transition budgeting and regression probes.
#[derive(Resource, Debug)]
pub(crate) struct RoomTransitionTelemetry {
    samples: VecDeque<RoomTransitionTimingSample>,
    capacity: usize,
}

impl Default for RoomTransitionTelemetry {
    fn default() -> Self {
        Self {
            samples: VecDeque::new(),
            capacity: 64,
        }
    }
}

impl RoomTransitionTelemetry {
    pub(crate) fn samples(&self) -> impl DoubleEndedIterator<Item = &RoomTransitionTimingSample> {
        self.samples.iter()
    }

    fn record(
        &mut self,
        sample: RoomTransitionTimingSample,
        config: &RoomTransitionPresentationConfig,
    ) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        let budget = if sample.covered {
            config.covered_commit_budget
        } else {
            config.no_cover_commit_budget
        };
        let observed_commit = sample
            .commit_to_first_target_frame
            .or(sample.commit_enqueue);
        if observed_commit.is_some_and(|duration| duration > budget) {
            bevy::log::warn!(
                target: "ambition::room_transition::performance",
                "room transition {} {} -> {} commit-to-first-frame {:.3} ms exceeded {:.3} ms budget (covered={}, prefetch_hit={}, loading_visible={})",
                sample.sequence,
                sample.source_room,
                sample.target_room,
                observed_commit
                    .map(|duration| duration.as_secs_f64() * 1000.0)
                    .unwrap_or_default(),
                budget.as_secs_f64() * 1000.0,
                sample.covered,
                sample.prefetch_hit,
                sample.loading_foreground_visible,
            );
        }
        bevy::log::info!(
            target: "ambition::room_transition::performance",
            "room transition {} {} -> {}: construction_preflight_ms={:?} asset_manifest_ms={:?} asset_wait_ms={:?} ready_ms={:?} cover_present_ms={:?} commit_enqueue_ms={:?} commit_to_first_frame_ms={:?} loading_visible_ms={:.3} covered={} prefetch_hit={} loading_visible={}",
            sample.sequence,
            sample.source_room,
            sample.target_room,
            sample.construction_preflight.map(|d| d.as_secs_f64() * 1000.0),
            sample.asset_manifest_build.map(|d| d.as_secs_f64() * 1000.0),
            sample.asset_wait.map(|d| d.as_secs_f64() * 1000.0),
            sample.request_to_ready.map(|d| d.as_secs_f64() * 1000.0),
            sample.cover_request_to_presented.map(|d| d.as_secs_f64() * 1000.0),
            sample.commit_enqueue.map(|d| d.as_secs_f64() * 1000.0),
            sample.commit_to_first_target_frame.map(|d| d.as_secs_f64() * 1000.0),
            sample.loading_foreground_visible_duration.as_secs_f64() * 1000.0,
            sample.covered,
            sample.prefetch_hit,
            sample.loading_foreground_visible,
        );
        self.samples.push_back(sample);
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct RoomTransitionCoverRoot {
    sequence: u64,
}

#[derive(Resource, Default, Debug)]
struct RoomTransitionPresentationState {
    sequence: Option<u64>,
    owner: Option<LoadPresentationOwnerId>,
    update_serial: u64,
    cover_spawned_at: u64,
    commit_observed_at: Option<u64>,
    visible_before_commit: bool,
    foreground_finished: bool,
    visible_elapsed: Duration,
}

fn owner_for(sequence: u64) -> LoadPresentationOwnerId {
    LoadPresentationOwnerId::new(format!("room-transition:{sequence}"))
}

fn experience(config: &RoomTransitionPresentationConfig) -> LoadExperienceSpec {
    let mut spec = LoadExperienceSpec::basic(ROOM_TRANSITION_EXPERIENCE);
    spec.reveal_after = config.loading_reveal_after;
    spec.ready_policy = ReadyTransitionPolicy::AutoAdvance;
    spec.activity = None;
    spec
}

/// Install the visible half of room-transition loading.
///
/// Headless simulation does not install this adapter and therefore does not
/// require a cover acknowledgment. Windowed and no-window presentation hosts
/// install it through `add_presentation_plugins` and use the exact same room
/// transaction state as simulation.
pub(crate) fn install_room_transition_presentation(app: &mut App) {
    app.init_resource::<RoomTransitionPresentationAvailable>()
        .init_resource::<RoomTransitionPresentationConfig>()
        .init_resource::<RoomTransitionPresentationState>()
        .init_resource::<RoomTransitionTelemetry>()
        .init_resource::<RoomPreparationPrefetchState>()
        .add_systems(
            Update,
            (
                poll_room_transition_asset_readiness_system,
                drive_room_transition_presentation,
            )
                .chain()
                .before(LoadPresentationSet::Observe),
        )
        .add_systems(
            Update,
            prefetch_neighbor_room_preparation_system
                .after(LoadPresentationSet::Finalize)
                .run_if(ambition::platformer::lifecycle::session_world_exists),
        )
        .add_systems(
            Update,
            handle_room_transition_presentation_events
                .after(LoadPresentationSet::Actions)
                .before(LoadPresentationSet::Finalize),
        );
}

/// Synchronize one opaque cover + generic loading foreground with the current
/// room transaction.
///
/// A newly spawned cover is never acknowledged in the same update. Seeing the
/// exact root on a later update proves it survived one presentation frame,
/// which is the gate the simulation-side authorizer consumes.
#[allow(clippy::too_many_arguments)]
fn drive_room_transition_presentation(
    mut commands: Commands,
    time: Res<Time<Real>>,
    config: Res<RoomTransitionPresentationConfig>,
    model: Res<LoadPresentationModel>,
    mut runtime: ResMut<RoomTransitionPresentationState>,
    mut transitions: ResMut<RoomTransitionLoadState>,
    covers: Query<(Entity, &RoomTransitionCoverRoot)>,
    mut presentation: MessageWriter<LoadPresentationCommand>,
    mut loads: ResMut<LoadCoordinator>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut telemetry: ResMut<RoomTransitionTelemetry>,
) {
    runtime.update_serial = runtime.update_serial.saturating_add(1);
    let update_serial = runtime.update_serial;

    let Some(active_snapshot) = transitions.active.as_ref().cloned() else {
        if let Some(owner) = runtime.owner.take() {
            presentation.write(LoadPresentationCommand::Cancel { owner });
        }
        for (entity, _) in &covers {
            commands.entity(entity).despawn();
        }
        runtime.sequence = None;
        runtime.commit_observed_at = None;
        runtime.visible_before_commit = false;
        runtime.foreground_finished = false;
        runtime.visible_elapsed = Duration::ZERO;
        return;
    };

    if runtime.sequence != Some(active_snapshot.sequence) {
        if let Some(owner) = runtime.owner.take() {
            presentation.write(LoadPresentationCommand::Cancel { owner });
        }
        for (entity, _) in &covers {
            commands.entity(entity).despawn();
        }

        let owner = owner_for(active_snapshot.sequence);
        presentation.write(LoadPresentationCommand::Begin {
            owner: owner.clone(),
            barrier: active_snapshot.barrier.clone(),
            spec: experience(&config),
        });
        commands.spawn((
            RoomTransitionCoverRoot {
                sequence: active_snapshot.sequence,
            },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(900),
            Name::new(format!(
                "room transition cover {}",
                active_snapshot.sequence
            )),
        ));

        runtime.sequence = Some(active_snapshot.sequence);
        runtime.owner = Some(owner);
        runtime.cover_spawned_at = update_serial;
        runtime.commit_observed_at = None;
        runtime.visible_before_commit = false;
        runtime.foreground_finished = false;
        runtime.visible_elapsed = Duration::ZERO;
        return;
    }

    let owner_matches = runtime
        .owner
        .as_ref()
        .is_some_and(|owner| model.owner.as_ref() == Some(owner));
    if owner_matches && model.visible {
        runtime.visible_elapsed = runtime
            .visible_elapsed
            .saturating_add(Duration::from_secs_f32(time.delta_secs()));
    }

    let exact_cover_exists = covers
        .iter()
        .any(|(_, root)| root.sequence == active_snapshot.sequence);
    if active_snapshot.cover_required
        && !active_snapshot.cover_presented
        && exact_cover_exists
        && update_serial > runtime.cover_spawned_at
    {
        if let Some(active) = transitions
            .active
            .as_mut()
            .filter(|active| active.sequence == active_snapshot.sequence)
        {
            active.cover_presented = true;
            active.cover_presented_at = Some(time.elapsed());
        }
    }

    if active_snapshot.phase != RoomTransitionLoadPhase::Committed {
        runtime.commit_observed_at = None;
        return;
    }

    let commit_observed_at = match runtime.commit_observed_at {
        Some(observed_at) => observed_at,
        None => {
            runtime.commit_observed_at = Some(update_serial);
            runtime.visible_before_commit = owner_matches && model.visible;
            if !runtime.visible_before_commit {
                if let Some(owner) = runtime.owner.as_ref() {
                    presentation.write(LoadPresentationCommand::Finish {
                        owner: owner.clone(),
                    });
                    runtime.foreground_finished = true;
                }
            }
            update_serial
        }
    };
    let target_rendered_under_cover = update_serial > commit_observed_at;
    let foreground_minimum_satisfied =
        !runtime.visible_before_commit || runtime.visible_elapsed >= config.minimum_visible;
    if !target_rendered_under_cover || !foreground_minimum_satisfied {
        return;
    }

    let Some(owner) = runtime.owner.take() else {
        return;
    };
    if !runtime.foreground_finished {
        presentation.write(LoadPresentationCommand::Finish { owner });
    }
    for (entity, root) in &covers {
        if root.sequence == active_snapshot.sequence {
            commands.entity(entity).despawn();
        }
    }
    let now = time.elapsed();
    telemetry.record(
        RoomTransitionTimingSample {
            sequence: active_snapshot.sequence,
            source_room: active_snapshot.source_room_id.clone(),
            target_room: active_snapshot.target_room_id.clone(),
            construction_preflight: active_snapshot.construction_preflight_duration,
            asset_manifest_build: active_snapshot.asset_manifest_duration,
            asset_wait: active_snapshot
                .requested_at
                .zip(active_snapshot.asset_ready_at)
                .map(|(start, ready)| ready.saturating_sub(start)),
            request_to_ready: active_snapshot
                .requested_at
                .zip(active_snapshot.ready_at)
                .map(|(start, ready)| ready.saturating_sub(start)),
            cover_request_to_presented: active_snapshot
                .requested_at
                .zip(active_snapshot.cover_presented_at)
                .map(|(requested, covered)| covered.saturating_sub(requested)),
            commit_enqueue: active_snapshot.commit_duration,
            commit_to_first_target_frame: active_snapshot
                .committed_at
                .map(|committed| now.saturating_sub(committed)),
            covered: active_snapshot.cover_required,
            prefetch_hit: active_snapshot.prefetch_hit,
            loading_foreground_visible: runtime.visible_before_commit,
            loading_foreground_visible_duration: runtime.visible_elapsed,
        },
        &config,
    );
    loads.retire(&active_snapshot.barrier.load_id);
    transitions.active = None;
    next_mode.set(GameMode::Playing);
    runtime.sequence = None;
    runtime.commit_observed_at = None;
    runtime.visible_before_commit = false;
    runtime.foreground_finished = false;
    runtime.visible_elapsed = Duration::ZERO;
}

fn apply_load_command(
    loads: &mut LoadCoordinator,
    events: &mut MessageWriter<LoadEvent>,
    command: LoadCommand,
) {
    for event in loads.apply(command) {
        events.write(event);
    }
}

/// Route generic loading actions back to the room-transition owner.
///
/// Retry mints a fresh room request instead of attempting to resurrect a failed
/// load record. Cancel and Quit both return to the still-valid source room.
#[allow(clippy::too_many_arguments)]
fn handle_room_transition_presentation_events(
    mut events: MessageReader<LoadPresentationEvent>,
    mut transitions: ResMut<RoomTransitionLoadState>,
    mut loads: ResMut<LoadCoordinator>,
    mut load_events: MessageWriter<LoadEvent>,
    mut presentation: MessageWriter<LoadPresentationCommand>,
    mut room_requests: MessageWriter<ambition::actors::rooms::RoomTransitionRequested>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    for event in events.read() {
        let Some(active) = transitions.active.as_ref() else {
            continue;
        };
        let expected_owner = owner_for(active.sequence);
        let event_owner = match event {
            LoadPresentationEvent::ContinueRequested { owner }
            | LoadPresentationEvent::RetryRequested { owner, .. }
            | LoadPresentationEvent::CancelRequested { owner }
            | LoadPresentationEvent::QuitRequested { owner } => owner,
        };
        if event_owner != &expected_owner {
            continue;
        }

        match event {
            LoadPresentationEvent::ContinueRequested { .. } => {
                // Room transitions use AutoAdvance and should never require an
                // extra confirmation after readiness.
            }
            LoadPresentationEvent::RetryRequested { .. } => {
                let request = active.request.clone();
                let load_id = active.barrier.load_id.clone();
                apply_load_command(
                    &mut loads,
                    &mut load_events,
                    LoadCommand::Cancel {
                        load_id: load_id.clone(),
                    },
                );
                loads.retire(&load_id);
                transitions.active = None;
                presentation.write(LoadPresentationCommand::Cancel {
                    owner: expected_owner,
                });
                room_requests.write(request);
            }
            LoadPresentationEvent::CancelRequested { .. }
            | LoadPresentationEvent::QuitRequested { .. } => {
                let load_id = active.barrier.load_id.clone();
                apply_load_command(
                    &mut loads,
                    &mut load_events,
                    LoadCommand::Cancel {
                        load_id: load_id.clone(),
                    },
                );
                loads.retire(&load_id);
                transitions.active = None;
                presentation.write(LoadPresentationCommand::Cancel {
                    owner: expected_owner,
                });
                next_mode.set(GameMode::Playing);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_transition_experience_auto_advances_after_hidden_grace() {
        let config = RoomTransitionPresentationConfig::default();
        let spec = experience(&config);
        assert_eq!(spec.reveal_after, Duration::from_millis(250));
        assert_eq!(spec.ready_policy, ReadyTransitionPolicy::AutoAdvance);
        assert!(spec.activity.is_none());
    }

    #[test]
    fn room_transition_owner_is_exact_per_sequence() {
        assert_eq!(owner_for(7).as_str(), "room-transition:7");
        assert_ne!(owner_for(7), owner_for(8));
    }

    fn timing_sample(sequence: u64) -> RoomTransitionTimingSample {
        RoomTransitionTimingSample {
            sequence,
            source_room: "source".to_string(),
            target_room: "target".to_string(),
            construction_preflight: None,
            asset_manifest_build: None,
            asset_wait: None,
            request_to_ready: None,
            cover_request_to_presented: None,
            commit_enqueue: None,
            commit_to_first_target_frame: None,
            covered: true,
            prefetch_hit: false,
            loading_foreground_visible: false,
            loading_foreground_visible_duration: Duration::ZERO,
        }
    }

    #[test]
    fn transition_telemetry_is_bounded_and_keeps_newest_samples() {
        let mut telemetry = RoomTransitionTelemetry {
            samples: VecDeque::new(),
            capacity: 2,
        };
        let config = RoomTransitionPresentationConfig::default();
        telemetry.record(timing_sample(1), &config);
        telemetry.record(timing_sample(2), &config);
        telemetry.record(timing_sample(3), &config);
        assert_eq!(
            telemetry
                .samples()
                .map(|sample| sample.sequence)
                .collect::<Vec<_>>(),
            vec![2, 3],
        );
    }
}
