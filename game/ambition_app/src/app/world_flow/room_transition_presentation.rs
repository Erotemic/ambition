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

use ambition_platformer2d::load::{LoadCommand, LoadCoordinator, LoadEvent};
use ambition_platformer2d::load_presentation::{
    LoadExperienceSpec, LoadPresentationCommand, LoadPresentationEvent, LoadPresentationModel,
    LoadPresentationOwnerId, LoadPresentationSet, ReadyTransitionPolicy,
};
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::render::rendering::UnclaimedFeatureViews;
use ambition_platformer2d::sim::Platformer2dSimulationPhaseMonolith;

use super::room_transition_assets::{
    contribute_room_transition_assets_system, poll_room_transition_asset_readiness_system,
    prefetch_neighbor_room_preparation_system, RoomPreparationPrefetchState,
};
use ambition_platformer2d::runtime::room_transition::{
    RoomTransitionLoadPhase, RoomTransitionLoadState, RoomTransitionPresentationAvailable,
};

const ROOM_TRANSITION_EXPERIENCE: &str = "ambition.room-transition";

/// Where the cover decides whether the destination room is presentable.
///
/// One member ([`drive_room_transition_presentation`]), and it exists so the
/// ordering against the presentation floor's census is a NAMED, testable edge
/// rather than an attribute nobody can check.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomTransitionCoverSet;

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
    /// How long the cover may wait, after commit, for the target room to become
    /// PRESENTABLE — every feature view the sim published has a render family's
    /// picture on it.
    ///
    /// Generous on purpose. It is not a performance budget; it is the point at
    /// which waiting longer is worse than showing the diagnosis, and a room that
    /// legitimately draws slowly should get the loading screen rather than a
    /// flash of magenta. The `warn!` on expiry is what turns a slow room into a
    /// reported number instead of a feeling.
    pub(crate) presentation_settle_deadline: Duration,
}

impl Default for RoomTransitionPresentationConfig {
    fn default() -> Self {
        Self {
            loading_reveal_after: Duration::from_millis(250),
            minimum_visible: Duration::from_millis(300),
            no_cover_commit_budget: Duration::from_millis(4),
            covered_commit_budget: Duration::from_millis(50),
            presentation_settle_deadline: Duration::from_secs(8),
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
    #[cfg(test)]
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
                target: "ambition_platformer2d::room_transition::performance",
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
            target: "ambition_platformer2d::room_transition::performance",
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
    // This host CAN answer "did the destination room's art arrive", so it
    // installs the engine's contributor marker and owns resolving that work
    // item. A demo host that installs neither marker gets a barrier that
    // honestly skips the contributor.
    app.init_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionAssetContributor>()
        // And the same answer for a shell route's FIRST room, before it
        // activates: the provider leaves `prepare-first-room-art` running for
        // a host that installs this marker, and the system below completes it.
        .init_resource::<ambition_platformer2d::provider::FirstRoomArtContributor>()
        .init_resource::<super::first_room_art::FirstRoomArtJobs>()
        .init_resource::<super::room_transition_assets::ContributedRoomAssets>()
        .init_resource::<RoomTransitionPresentationAvailable>()
        // The presentation floor owns this and fills it every frame. Named here
        // too because a `Res<..>` that does not EXIST fails param validation and
        // silently skips the system — which is a cover that never retires. A
        // host without the render plugin then behaves as it does today: nothing
        // is undrawn because nothing is drawing.
        .init_resource::<UnclaimedFeatureViews>()
        .init_resource::<RoomTransitionPresentationConfig>()
        .init_resource::<RoomTransitionPresentationState>()
        .init_resource::<RoomTransitionTelemetry>()
        .init_resource::<RoomPreparationPrefetchState>()
        .add_systems(
            Update,
            (
                super::first_room_art::prepare_first_room_art_system,
                contribute_room_transition_assets_system,
                poll_room_transition_asset_readiness_system,
                // the census must be THIS frame's. The presentation floor
                // republishes `UnclaimedFeatureViews` at the tail of the visual
                // chain; the cover retires on it being empty. Both ends are in
                // `Update` — checked, not assumed, because an `.after` across
                // schedules is silently vacuous and this one going quiet
                // reintroduces the exact flash it fixes.
                drive_room_transition_presentation.in_set(RoomTransitionCoverSet),
            )
                .chain()
                .before(LoadPresentationSet::Observe),
        )
        .configure_sets(
            Update,
            RoomTransitionCoverSet
                .after(Platformer2dSimulationPhaseMonolith::PresentationVisualSync),
        )
        .add_systems(
            Update,
            prefetch_neighbor_room_preparation_system
                .after(LoadPresentationSet::Finalize)
                .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
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
    // Features the sim published that no render family has drawn yet. The cover
    // waits on these: see the retirement block below.
    //
    // NOT the magenta placeholders. That marker is a diagnostic with a
    // grace period, so its population is deliberately EMPTY during the first
    // frames of a room draw — precisely the frames the cover must not retire in.
    // See `UnclaimedFeatureViews`, and `RoomTransitionCoverSet` for the ordering
    // this read depends on.
    unclaimed: Res<UnclaimedFeatureViews>,
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

    // PRESENTABLE, not merely committed.
    //
    // It is not a general condition: render families spawn through `Commands`, and a room with
    // many actors takes several flushes to draw. `draw_unclaimed_feature_views` fills the gap
    // with a deliberately-ugly magenta box — a DIAGNOSIS, per its own docs — so retiring the
    // cover here showed the player that diagnosis as a stage of loading.
    //
    // placeholder sprite, and then it flashes to the characters. It looks
    // disorienting. its not smooth. It doesn't work. It would just be better to
    // have a loading screen if the load is going to take a hot second."* And on
    // the way out too, because leaving is another transition.
    //
    // it counts UNDRAWN VIEWS, not magenta boxes, and that distinction is
    // the fix. Counting boxes meant the cover could not tell "the
    // art has not arrived yet" from "the art will never arrive", because the box
    // is a diagnosis of the second and was being read as evidence of the first.
    // give-ups — the two facts were both true because the thing it waited on was
    // never the thing it needed to know.
    let unsettled = unclaimed.len();
    let since_commit = active_snapshot
        .committed_at
        .map(|committed| time.elapsed().saturating_sub(committed))
        .unwrap_or_default();
    if unsettled > 0 {
        // never a hang.
        if since_commit < config.presentation_settle_deadline {
            return;
        }
        bevy::log::warn!(
            target: "ambition_platformer2d::room_transition::performance",
            "room transition {} {} -> {} revealed with {unsettled} feature view(s) \
             still undrawn after {:.0} ms — the cover gave up waiting. Those are \
             features the sim published and no render family claimed, so either a \
             spawn path is missing its family marker or this room draws more \
             slowly than the deadline allows. Undrawn: {:?}",
            active_snapshot.sequence,
            active_snapshot.source_room_id,
            active_snapshot.target_room_id,
            since_commit.as_secs_f64() * 1000.0,
            unclaimed.ids().collect::<Vec<_>>(),
        );
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
    ambition_platformer2d::platformer::world_log::note_game_mode_request(
        GameMode::Playing,
        "room_transition_retire",
    );
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
                // RETRY RE-ISSUES NOTHING, and that is the whole change.
                //
                // It re-minted a `RoomTransitionRequested` from the failed transaction — a
                // description that could not name a body, so a retry after a possession change
                // transited whoever was driving by then. Dropping the transaction is the entire
                // retry: `begin` opens a fresh one for the same crossing on the next frame.
                //
                // and re-recording it would have been worse than redundant.
                // `PendingLifecycleCommit` is ROLLBACK STATE and this system runs
                // in `Update`, which never rewinds — writing it here would drift
                // from the peers' copy on every rewind, silently.
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
            }
            // What abandonment needs is for the `PendingLifecycleCommit` intent to STOP being
            // pending — and the arm above spells out why this system may not do that: the intent is
            // ROLLBACK STATE and this runs in `Update`, which never rewinds, so a write here drifts
            // from the peers' copy silently. So this arm does the only thing it can — drop the
            // transaction — and `begin_room_transition_load_system` sees the same still-pending
            // intent on the very next frame and opens an identical transaction, new sequence and
            // all. Escape during a Hall load therefore RESTARTS the load (discarding a prepared
            // plan and its asset manifest); it does not leave it. `shell_actions.back` is ungated
            // by phase (`basic_load_keyboard`), so this is reachable throughout, not only on a
            // failure.
            //
            // A player pressing Escape is a player INTENT, and the deterministic channel for one is
            // the input stream the sim already reads — not a presentation message, which
            // `clear_message_on_rollback` wipes. That is a slice with a design decision in it (does
            // a crossing become abandonable at all, or is a transition committed-on-request once
            // the body has crossed the zone?), so it is NAMED here rather than improvised: the
            // honest answer today is that a room transition is committed on request, and this arm
            // is a restart button.
            //
            // Quit rides the same arm and wants the same drop for a different
            // reason — it is leaving the session, so the transaction must not
            // outlive it.
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
                ambition_platformer2d::platformer::world_log::note_game_mode_request(
                    GameMode::Playing,
                    "room_transition_cancelled",
                );
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
