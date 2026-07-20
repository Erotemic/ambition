//! Developer-only in-game proof that the live simulation is really rolling back.
//!
//! Developer-visible builds run their authoritative simulation through
//! [`rollback::GgrsSchedule`]. During ordinary local play this plugin owns a
//! zero-distance local `SyncTestSession`, so GGRS remains the
//! single simulation authority without deliberately rewinding every frame.
//! Non-developer release compositions keep their existing simulation host.
//!
//! Press F9 to request one bounded six-frame SyncTest pulse. The observatory
//! waits for one real rollback/resimulation and the following SyncTest checksum
//! comparison, captures the restored historical poses, then immediately rebases
//! back to its cheap zero-distance baseline. It never leaves the expensive
//! determinism stress test running continuously.
//!
//! [`RollbackObservatoryControl`] is deliberately input-agnostic. Android or a
//! future developer-settings menu, authored switch, or debug item can request
//! the same proof pulse without changing rollback machinery.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::platformer::developer_hotkeys::DeveloperAction;
use ambition::render::ui_fonts::{UiFontWeight, UiFonts};
use ambition::runtime::rollback::{
    self, AdvanceWorld, AdvanceWorldSystems, AmbitionGgrsSession, ConfirmedFrameCount, LoadWorld,
    LoadWorldSystems, Rollback, RollbackFrameCount, RollbackSessionStatus, RunGgrsSystems,
    SyncTestSettings,
};

const DEFAULT_CHECK_DISTANCE: usize = 6;
const DEFAULT_MAX_PREDICTION_WINDOW: usize = 8;
const GHOST_SECONDS: f32 = 0.85;
const HUD_SECONDS: f32 = 5.0;

/// Stable tuning for the developer proof session.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RollbackProofSettings {
    pub check_distance: usize,
    pub max_prediction_window: usize,
}

impl Default for RollbackProofSettings {
    fn default() -> Self {
        Self {
            check_distance: DEFAULT_CHECK_DISTANCE,
            max_prediction_window: DEFAULT_MAX_PREDICTION_WINDOW,
        }
    }
}

/// Input-agnostic host control for the observatory.
///
/// F9 requests one proof pulse. Mobile developer settings, an authored switch,
/// or a debug item can call [`Self::request_proof`] through the same seam; the
/// platform adapter does not own GGRS session lifecycle.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RollbackObservatoryControl {
    requested_proofs: u64,
}

impl RollbackObservatoryControl {
    pub fn request_proof(&mut self) {
        self.requested_proofs = self.requested_proofs.saturating_add(1);
    }

    fn requested_proofs(self) -> u64 {
        self.requested_proofs
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnedSessionMode {
    Baseline,
    Proof,
}

impl OwnedSessionMode {
    fn check_distance(self, settings: RollbackProofSettings) -> usize {
        match self {
            Self::Baseline => 0,
            Self::Proof => settings.check_distance,
        }
    }
}

#[derive(Clone, Debug)]
struct RollbackGhost {
    sim_id: String,
    loaded_pos: ae::Vec2,
    size: ae::Vec2,
    entity_recreated: bool,
}

/// Presentation/instrumentation state owned by the observatory, never by the
/// authoritative simulation.
#[derive(Resource, Debug)]
pub(crate) struct RollbackProofState {
    owns_session: bool,
    session_mode: Option<OwnedSessionMode>,
    startup_error: Option<String>,
    handled_request: u64,
    proof_completed: bool,
    pre_ggrs_frame: i32,
    current_frame: i32,
    confirmed_frame: i32,
    last_loaded_frame: Option<i32>,
    last_rollback_depth: u32,
    max_rollback_depth: u32,
    advance_runs: u64,
    resimulated_runs: u64,
    resimulating: bool,
    load_runs: u64,
    last_recreated_entities: usize,
    max_recreated_entities: usize,
    mismatch_frames: Vec<i32>,
    pre_entities: BTreeMap<String, Entity>,
    ghosts: Vec<RollbackGhost>,
    ghost_seconds_left: f32,
    hud_seconds_left: f32,
}

impl Default for RollbackProofState {
    fn default() -> Self {
        Self {
            owns_session: false,
            session_mode: None,
            startup_error: None,
            handled_request: 0,
            proof_completed: false,
            pre_ggrs_frame: 0,
            current_frame: 0,
            confirmed_frame: -1,
            last_loaded_frame: None,
            last_rollback_depth: 0,
            max_rollback_depth: 0,
            advance_runs: 0,
            resimulated_runs: 0,
            resimulating: false,
            load_runs: 0,
            last_recreated_entities: 0,
            max_recreated_entities: 0,
            mismatch_frames: Vec::new(),
            pre_entities: BTreeMap::new(),
            ghosts: Vec::new(),
            ghost_seconds_left: 0.0,
            hud_seconds_left: 0.0,
        }
    }
}

pub(crate) fn reset_for_content_reload(world: &mut World) {
    let handled_request = world
        .get_resource::<RollbackObservatoryControl>()
        .map_or(0, |control| control.requested_proofs());
    if let Some(mut state) = world.get_resource_mut::<RollbackProofState>() {
        *state = RollbackProofState {
            handled_request,
            ..default()
        };
    }
}

pub(crate) fn mark_baseline_restarted(world: &mut World) {
    if let Some(mut state) = world.get_resource_mut::<RollbackProofState>() {
        state.owns_session = true;
        state.session_mode = Some(OwnedSessionMode::Baseline);
        state.startup_error = None;
    }
}

pub(crate) fn mark_baseline_restart_failed(world: &mut World, error: &str) {
    if let Some(mut state) = world.get_resource_mut::<RollbackProofState>() {
        state.owns_session = false;
        state.session_mode = None;
        state.startup_error = Some(format!(
            "LDtk reload committed, but local GGRS baseline restart failed: {error}"
        ));
        state.hud_seconds_left = HUD_SECONDS;
    }
}

#[derive(Component)]
struct RollbackProofText;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RollbackProofUpdateSet {
    Control,
    Session,
    InputLatch,
    Observe,
    Finish,
    Present,
}

/// Visible-app instrumentation for the real GGRS rollback schedules.
pub struct RollbackObservatoryPlugin;

impl Plugin for RollbackObservatoryPlugin {
    fn build(&self, app: &mut App) {
        if app
            .world()
            .get_resource::<ambition::runtime::SimulationHost>()
            .copied()
            != Some(ambition::runtime::SimulationHost::Ggrs)
        {
            return;
        }
        app.add_message::<DeveloperAction>()
            .init_resource::<RollbackProofSettings>()
            .init_resource::<RollbackObservatoryControl>()
            .init_resource::<RollbackProofState>()
            .init_resource::<ae::ControlFrameLatch>()
            .configure_sets(
                Update,
                (
                    RollbackProofUpdateSet::Control,
                    RollbackProofUpdateSet::Session,
                    RollbackProofUpdateSet::InputLatch,
                    RollbackProofUpdateSet::Observe,
                    RollbackProofUpdateSet::Finish,
                    RollbackProofUpdateSet::Present,
                )
                    .chain(),
            )
            .add_systems(Startup, spawn_rollback_proof_hud)
            .add_systems(PreUpdate, record_pre_ggrs_state.before(RunGgrsSystems))
            .add_systems(
                LoadWorld,
                capture_loaded_world.after(LoadWorldSystems::Mapping),
            )
            .add_systems(
                AdvanceWorld,
                count_advance_world_run.in_set(AdvanceWorldSystems::Last),
            )
            .add_systems(
                Update,
                request_rollback_proof.in_set(RollbackProofUpdateSet::Control),
            )
            .add_systems(
                Update,
                maintain_local_ggrs_session.in_set(RollbackProofUpdateSet::Session),
            )
            .add_systems(
                Update,
                ae::accumulate_control_frame_latch
                    .after(ambition::input::InputSet::Route)
                    .in_set(RollbackProofUpdateSet::InputLatch),
            )
            .add_systems(
                Update,
                observe_completed_host_update.in_set(RollbackProofUpdateSet::Observe),
            )
            .add_systems(
                Update,
                finish_completed_proof_pulse.in_set(RollbackProofUpdateSet::Finish),
            )
            .add_systems(
                Update,
                (
                    update_rollback_proof_hud,
                    draw_rollback_ghosts.run_if(rollback_proof_visible),
                )
                    .in_set(RollbackProofUpdateSet::Present),
            );
    }
}

/// Keep one local GGRS session attached to the current gameplay session.
///
/// Baseline mode uses `check_distance = 0`, which makes SyncTest issue one
/// `AdvanceFrame` and no save/load requests. An F9 request temporarily installs
/// the configured proof session. Once a rollback has been resimulated and the
/// following SyncTest tick has verified its checksum,
/// [`finish_completed_proof_pulse`] immediately restores the baseline. The
/// expensive N-frame resimulation is bounded to the proof request instead of
/// continuing for the rest of gameplay.
fn maintain_local_ggrs_session(world: &mut World) {
    let control = *world.resource::<RollbackObservatoryControl>();
    let settings = *world.resource::<RollbackProofSettings>();
    let gameplay_active = ambition::platformer::lifecycle::session_world_entity(world).is_some();
    let ggrs_active = world.contains_resource::<AmbitionGgrsSession>();
    let (owns_session, current_mode, handled_request) = {
        let state = world.resource::<RollbackProofState>();
        (
            state.owns_session,
            state.session_mode,
            state.handled_request,
        )
    };

    if !gameplay_active {
        if owns_session && ggrs_active {
            rollback::stop_session(world);
        }
        let mut state = world.resource_mut::<RollbackProofState>();
        state.owns_session = false;
        state.session_mode = None;
        state.proof_completed = false;
        state.ghosts.clear();
        state.ghost_seconds_left = 0.0;
        state.hud_seconds_left = 0.0;
        return;
    }

    // A future Matchbox/P2P session is authoritative. The observatory may
    // inspect it, but must never replace a session it does not own.
    if ggrs_active && !owns_session {
        return;
    }

    let proof_requested = control.requested_proofs() > handled_request
        && current_mode != Some(OwnedSessionMode::Proof);
    let requested_mode = if proof_requested {
        OwnedSessionMode::Proof
    } else if current_mode.is_none()
        || (current_mode == Some(OwnedSessionMode::Baseline) && !ggrs_active)
    {
        OwnedSessionMode::Baseline
    } else {
        return;
    };

    if ggrs_active && owns_session {
        rollback::stop_session(world);
    }

    let session_settings = SyncTestSettings {
        check_distance: requested_mode.check_distance(settings),
        max_prediction_window: settings.max_prediction_window,
    };
    match rollback::start_sync_test_session(world, session_settings) {
        Ok(()) => {
            if requested_mode == OwnedSessionMode::Proof {
                *world.resource_mut::<RollbackProofState>() = RollbackProofState {
                    owns_session: true,
                    session_mode: Some(OwnedSessionMode::Proof),
                    handled_request: control.requested_proofs(),
                    hud_seconds_left: HUD_SECONDS,
                    ..default()
                };
                info!(
                    "GGRS rollback proof pulse started ({} frames)",
                    settings.check_distance
                );
            } else {
                let mut state = world.resource_mut::<RollbackProofState>();
                state.owns_session = true;
                state.session_mode = Some(OwnedSessionMode::Baseline);
                state.startup_error = None;
            }
        }
        Err(error) => {
            error!("failed to start local GGRS observatory session: {error}");
            let mut state = world.resource_mut::<RollbackProofState>();
            state.startup_error = Some(format!("failed to start local GGRS session: {error}"));
            state.handled_request = control.requested_proofs();
            state.hud_seconds_left = HUD_SECONDS;
            state.owns_session = false;
            state.session_mode = None;
        }
    }
}

fn record_pre_ggrs_state(
    session: Option<Res<AmbitionGgrsSession>>,
    frame: Option<Res<RollbackFrameCount>>,
    entities: Query<(Entity, &ambition::platformer::sim_id::SimId), With<Rollback>>,
    mut state: ResMut<RollbackProofState>,
) {
    if state.session_mode != Some(OwnedSessionMode::Proof) || session.is_none() {
        return;
    }
    state.resimulating = false;
    state.pre_ggrs_frame = frame.as_deref().map_or(0, |frame| frame.0);
    state.pre_entities.clear();
    state.pre_entities.extend(
        entities
            .iter()
            .map(|(entity, sim_id)| (sim_id.as_str().to_owned(), entity)),
    );
}

/// Runs after GGRS has restored entities, components/resources, and mapped raw
/// `Entity` references. The captured pose is therefore a real historical world,
/// not a prediction made by the overlay.
fn checksum_comparison_completed(load_runs: u64) -> bool {
    // The first historical load performs the resimulation. SyncTest compares
    // that result at the beginning of the following host tick; reaching the
    // second load means the comparison passed and another rollback began.
    load_runs >= 2
}

fn capture_loaded_world(
    frame: Res<RollbackFrameCount>,
    bodies: Query<
        (
            Entity,
            &ambition::platformer::sim_id::SimId,
            &ae::BodyKinematics,
        ),
        With<Rollback>,
    >,
    mut state: ResMut<RollbackProofState>,
) {
    if state.session_mode != Some(OwnedSessionMode::Proof) {
        return;
    }

    state.load_runs = state.load_runs.saturating_add(1);
    state.resimulating = true;
    state.last_loaded_frame = Some(frame.0);
    let depth = state.pre_ggrs_frame.saturating_sub(frame.0).max(0) as u32;
    state.last_rollback_depth = depth;
    state.max_rollback_depth = state.max_rollback_depth.max(depth);

    let mut recreated = 0_usize;
    let mut ghosts = Vec::new();
    for (entity, sim_id, body) in &bodies {
        let entity_recreated = state
            .pre_entities
            .get(sim_id.as_str())
            .is_some_and(|before| *before != entity);
        recreated += usize::from(entity_recreated);
        ghosts.push(RollbackGhost {
            sim_id: sim_id.as_str().to_owned(),
            loaded_pos: body.pos,
            size: body.size,
            entity_recreated,
        });
    }
    ghosts.sort_by(|left, right| left.sim_id.cmp(&right.sim_id));
    state.last_recreated_entities = recreated;
    state.max_recreated_entities = state.max_recreated_entities.max(recreated);
    state.ghosts = ghosts;
    state.ghost_seconds_left = GHOST_SECONDS;
    state.hud_seconds_left = HUD_SECONDS;
    // SyncTest compares a resimulated checksum at the beginning of the next
    // host tick. The second LoadWorld therefore proves that the first rollback
    // completed and passed GGRS's checksum comparison.
    state.proof_completed = checksum_comparison_completed(state.load_runs);
}

fn count_advance_world_run(mut state: ResMut<RollbackProofState>) {
    if state.session_mode == Some(OwnedSessionMode::Proof) {
        state.advance_runs = state.advance_runs.saturating_add(1);
        if state.resimulating {
            state.resimulated_runs = state.resimulated_runs.saturating_add(1);
        }
    }
}

fn observe_completed_host_update(
    time: Res<Time>,
    frame: Option<Res<RollbackFrameCount>>,
    confirmed: Option<Res<ConfirmedFrameCount>>,
    status: Option<Res<RollbackSessionStatus>>,
    mut state: ResMut<RollbackProofState>,
) {
    if state.session_mode != Some(OwnedSessionMode::Proof)
        && state.hud_seconds_left <= 0.0
        && state.ghost_seconds_left <= 0.0
        && state.startup_error.is_none()
    {
        return;
    }
    state.current_frame = frame.as_deref().map_or(0, |frame| frame.0);
    state.confirmed_frame = confirmed.as_deref().map_or(-1, |frame| frame.0);
    if let Some(status) = status.as_deref() {
        state
            .mismatch_frames
            .extend(status.mismatch_frames.iter().copied());
        state.mismatch_frames.sort_unstable();
        state.mismatch_frames.dedup();
    }
    state.ghost_seconds_left = (state.ghost_seconds_left - time.delta_secs()).max(0.0);
    state.hud_seconds_left = (state.hud_seconds_left - time.delta_secs()).max(0.0);
    if state.ghost_seconds_left == 0.0 {
        state.ghosts.clear();
    }
}

fn request_rollback_proof(
    mut actions: MessageReader<DeveloperAction>,
    mut control: ResMut<RollbackObservatoryControl>,
) {
    if actions
        .read()
        .any(|action| *action == DeveloperAction::RequestRollbackProof)
    {
        control.request_proof();
        info!("GGRS rollback proof pulse requested");
    }
}

/// Return to the zero-distance baseline in the same rendered update that
/// observes a successful checksum comparison (or mismatch). This prevents the
/// SyncTest stress loop from continuing after the bounded proof completes.
fn finish_completed_proof_pulse(world: &mut World) {
    let should_finish = {
        let state = world.resource::<RollbackProofState>();
        state.owns_session
            && state.session_mode == Some(OwnedSessionMode::Proof)
            && (state.proof_completed || !state.mismatch_frames.is_empty())
    };
    if !should_finish {
        return;
    }

    let settings = *world.resource::<RollbackProofSettings>();
    if world.contains_resource::<AmbitionGgrsSession>() {
        rollback::stop_session(world);
    }
    match rollback::start_sync_test_session(
        world,
        SyncTestSettings {
            check_distance: OwnedSessionMode::Baseline.check_distance(settings),
            max_prediction_window: settings.max_prediction_window,
        },
    ) {
        Ok(()) => {
            let mut state = world.resource_mut::<RollbackProofState>();
            state.owns_session = true;
            state.session_mode = Some(OwnedSessionMode::Baseline);
            state.hud_seconds_left = state.hud_seconds_left.max(HUD_SECONDS);
            info!("GGRS rollback proof verified; restored zero-distance baseline");
        }
        Err(error) => {
            error!("failed to restore GGRS baseline after proof: {error}");
            let mut state = world.resource_mut::<RollbackProofState>();
            state.startup_error = Some(format!(
                "rollback proof ran, but baseline restart failed: {error}"
            ));
            state.owns_session = false;
            state.session_mode = None;
            state.proof_completed = false;
            state.hud_seconds_left = HUD_SECONDS;
        }
    }
}

fn spawn_rollback_proof_hud(mut commands: Commands, ui_fonts: Option<Res<UiFonts>>) {
    let font = ui_fonts
        .map(|fonts| fonts.text_font(13.0, UiFontWeight::Monospace))
        .unwrap_or(TextFont {
            font_size: 13.0,
            ..default()
        });
    commands.spawn((
        Text::new(""),
        font,
        TextColor(Color::srgba(0.72, 0.96, 1.0, 0.96)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.025, 0.045, 0.82)),
        Visibility::Hidden,
        GlobalZIndex(1200),
        Name::new("GGRS Rollback Proof HUD"),
        RollbackProofText,
    ));
}

fn update_rollback_proof_hud(
    state: Res<RollbackProofState>,
    session: Option<Res<AmbitionGgrsSession>>,
    rollback_entities: Query<Entity, With<Rollback>>,
    mut text_q: Query<(&mut Text, &mut Visibility), With<RollbackProofText>>,
) {
    let Ok((mut text, mut visibility)) = text_q.single_mut() else {
        return;
    };
    let visible = state.hud_seconds_left > 0.0
        || state.session_mode == Some(OwnedSessionMode::Proof)
        || state.startup_error.is_some();
    *visibility = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if !visible {
        return;
    }

    let active = session.is_some();
    let loaded = state
        .last_loaded_frame
        .map_or_else(|| "--".to_owned(), |frame| frame.to_string());
    let checksum = if !state.mismatch_frames.is_empty() {
        format!("MISMATCH {:?}", state.mismatch_frames)
    } else if state.proof_completed {
        "VERIFIED".to_owned()
    } else if state.load_runs > 0 {
        "resimulated; awaiting checksum comparison".to_owned()
    } else {
        "waiting for first rollback".to_owned()
    };
    let status = state.startup_error.as_deref().unwrap_or(
        if state.session_mode == Some(OwnedSessionMode::Proof) {
            "RUNNING ONE-SHOT SYNC TEST"
        } else if state.load_runs > 0 {
            "VERIFIED; BASELINE RESTORED"
        } else if active && !state.owns_session {
            "OBSERVING EXTERNAL SESSION"
        } else {
            "WAITING FOR PROOF"
        },
    );

    text.0 = format!(
        "GGRS ROLLBACK PROOF  [{status}]\n\
         frame              {:>8}\n\
         confirmed          {:>8}\n\
         last loaded        {:>8}\n\
         rollback depth     {:>8}  (max {})\n\
         LoadWorld calls    {:>8}\n\
         AdvanceWorld calls {:>8}\n\
         resimulated frames {:>8}\n\
         rollback entities  {:>8}\n\
         recreated entities {:>8}  (max {})\n\
         checksum           {}\n\
         historical ghosts  {:>8}\n\
         F9: run another proof pulse",
        state.current_frame,
        state.confirmed_frame,
        loaded,
        state.last_rollback_depth,
        state.max_rollback_depth,
        state.load_runs,
        state.advance_runs,
        state.resimulated_runs,
        rollback_entities.iter().count(),
        state.last_recreated_entities,
        state.max_recreated_entities,
        checksum,
        state.ghosts.len(),
    );
}

fn rollback_proof_visible(state: Res<RollbackProofState>) -> bool {
    state.ghost_seconds_left > 0.0
}

fn draw_rollback_ghosts(
    state: Res<RollbackProofState>,
    world_q: Query<&ae::RoomGeometry, With<ambition::platformer::lifecycle::SessionRoot>>,
    current_bodies: Query<
        (&ambition::platformer::sim_id::SimId, &ae::BodyKinematics),
        With<Rollback>,
    >,
    mut gizmos: Gizmos,
) {
    if state.ghost_seconds_left <= 0.0 {
        return;
    }
    let Ok(world) = world_q.single() else {
        return;
    };

    let current: BTreeMap<&str, ae::Vec2> = current_bodies
        .iter()
        .map(|(sim_id, body)| (sim_id.as_str(), body.pos))
        .collect();
    let fade = (state.ghost_seconds_left / GHOST_SECONDS).clamp(0.15, 1.0);

    for ghost in &state.ghosts {
        let color = if ghost.entity_recreated {
            Color::srgba(1.0, 0.42, 0.88, 0.90 * fade)
        } else {
            Color::srgba(0.28, 0.92, 1.0, 0.72 * fade)
        };
        draw_world_aabb(
            &mut gizmos,
            &world.0,
            ghost.loaded_pos,
            ghost.size * 0.5,
            color,
        );
        if let Some(&live_pos) = current.get(ghost.sim_id.as_str()) {
            let loaded = world_to_gizmo(&world.0, ghost.loaded_pos);
            let live = world_to_gizmo(&world.0, live_pos);
            if loaded.distance_squared(live) > 1.0 {
                gizmos.line_2d(loaded, live, color);
            }
        }
    }
}

fn world_to_gizmo(world: &ae::World, point: ae::Vec2) -> Vec2 {
    ae::config::world_to_bevy(world, point, 0.0).truncate()
}

fn draw_world_aabb(
    gizmos: &mut Gizmos,
    world: &ae::World,
    center: ae::Vec2,
    half: ae::Vec2,
    color: Color,
) {
    let min = center - half;
    let max = center + half;
    let top_left = world_to_gizmo(world, ae::Vec2::new(min.x, min.y));
    let top_right = world_to_gizmo(world, ae::Vec2::new(max.x, min.y));
    let bottom_right = world_to_gizmo(world, ae::Vec2::new(max.x, max.y));
    let bottom_left = world_to_gizmo(world, ae::Vec2::new(min.x, max.y));
    gizmos.line_2d(top_left, top_right, color);
    gizmos.line_2d(top_right, bottom_right, color);
    gizmos.line_2d(bottom_right, bottom_left, color);
    gizmos.line_2d(bottom_left, top_left, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_window_has_a_valid_sync_test_distance() {
        let settings = RollbackProofSettings::default();
        assert!(settings.check_distance >= 2);
        assert!(settings.check_distance < settings.max_prediction_window);
    }

    #[test]
    fn desktop_and_mobile_controls_share_one_platform_neutral_request() {
        let mut control = RollbackObservatoryControl::default();
        assert_eq!(control.requested_proofs(), 0);
        control.request_proof();
        assert_eq!(control.requested_proofs(), 1);
        control.request_proof();
        assert_eq!(control.requested_proofs(), 2);
    }

    #[test]
    fn baseline_is_cheap_and_proof_is_bounded() {
        let settings = RollbackProofSettings::default();
        assert_eq!(OwnedSessionMode::Baseline.check_distance(settings), 0);
        assert_eq!(
            OwnedSessionMode::Proof.check_distance(settings),
            settings.check_distance
        );
        assert!(!checksum_comparison_completed(1));
        assert!(checksum_comparison_completed(2));
    }
}
