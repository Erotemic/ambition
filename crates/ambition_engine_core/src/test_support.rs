//! Test-only helpers that drive bodies through the SAME trusted boundary
//! production uses ([`step_motion`] and the environment-supplied frame).
//!
//! The historical whole-policy entry points (`update_player_*`) are gone from
//! the engine; these reconstruct their ergonomic shapes for the invariant
//! tests. [`TestTuning`] pairs the flat authored [`MovementTuning`] with the
//! explicit frame direction the ENVIRONMENT would resolve in production — the
//! direction is test-fixture state here precisely because it may no longer
//! live inside any tuning/parameter type.

use crate::body_clusters::{reset_body_clusters, BodyClusterScratch, BodyClustersMut};
use crate::movement::{
    step_motion, AxisSweptParams, FrameEvents, InputState, MotionModel, MotionStepContext,
    MovementTuning, DEFAULT_GRAVITY_DIR, DEFAULT_TUNING,
};
use crate::{LedgeContact, LedgeGrabState, MotionFrame, Vec2, World};

/// Authored tuning + the test's explicit environment frame direction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TestTuning {
    pub base: MovementTuning,
    /// The frame direction the environment resolver would supply.
    pub gravity_dir: Vec2,
    /// Legacy Y-sign mirror some fixtures still set; derived consumers should
    /// use `gravity_dir`.
    pub gravity_sign: f32,
}

pub(crate) const TEST_TUNING: TestTuning = TestTuning {
    base: DEFAULT_TUNING,
    gravity_dir: DEFAULT_GRAVITY_DIR,
    gravity_sign: 1.0,
};

impl Default for TestTuning {
    fn default() -> Self {
        TEST_TUNING
    }
}

impl std::ops::Deref for TestTuning {
    type Target = MovementTuning;
    fn deref(&self) -> &MovementTuning {
        &self.base
    }
}

impl std::ops::DerefMut for TestTuning {
    fn deref_mut(&mut self) -> &mut MovementTuning {
        &mut self.base
    }
}

impl From<MovementTuning> for TestTuning {
    fn from(base: MovementTuning) -> Self {
        Self {
            base,
            ..TEST_TUNING
        }
    }
}

impl TestTuning {
    /// The frame the environment resolver would produce for this fixture.
    pub fn frame(&self) -> MotionFrame {
        MotionFrame::from_direction(self.gravity_dir, self.base.gravity)
    }

    pub fn params(&self) -> AxisSweptParams {
        self.base.axis_swept_params()
    }
}

/// Whole-tick axis-swept step through [`step_motion`] + the home respawn
/// policy (a flagged reset teleports to `world.spawn`).
pub(crate) fn update_player_with_tuning_clusters(
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
    tuning: TestTuning,
) -> FrameEvents {
    let mut model = MotionModel::axis_swept(tuning.params());
    let result = step_motion(
        &mut model,
        clusters,
        MotionStepContext {
            world,
            input,
            frame: tuning.frame(),
            facing_intent: 0.0,
            dt: raw_dt,
        },
    );
    if result.events.reset {
        reset_body_clusters(clusters, world.spawn);
    }
    result.events
}

pub(crate) fn update_player_with_tuning_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    raw_dt: f32,
    tuning: TestTuning,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    update_player_with_tuning_clusters(world, &mut clusters, input, raw_dt, tuning)
}

pub(crate) fn update_player_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_with_tuning_scratch(world, scratch, input, raw_dt, TEST_TUNING)
}

pub(crate) fn update_player_clusters(
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_with_tuning_clusters(world, clusters, input, raw_dt, TEST_TUNING)
}

/// Control PHASE only (kernel-private phase vocabulary) + the home respawn
/// policy, for tests that pin phase-level behavior such as the two-clock split.
pub(crate) fn update_player_control_with_tuning_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    control_dt: f32,
    tuning: TestTuning,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    let events = crate::movement::update_body_control_in_frame(
        world,
        &mut clusters,
        input,
        control_dt,
        tuning.frame(),
        tuning.params(),
    );
    if events.reset {
        reset_body_clusters(&mut clusters, world.spawn);
    }
    events
}

pub(crate) fn update_player_control_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    control_dt: f32,
) -> FrameEvents {
    update_player_control_with_tuning_scratch(world, scratch, input, control_dt, TEST_TUNING)
}

/// Simulation PHASE only (kernel-private phase vocabulary) + the home respawn
/// policy.
pub(crate) fn update_player_simulation_with_tuning_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    raw_dt: f32,
    tuning: TestTuning,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    update_player_simulation_with_clusters(world, &mut clusters, input, raw_dt, tuning)
}

pub(crate) fn update_player_simulation_with_clusters(
    world: &World,
    clusters: &mut BodyClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
    tuning: TestTuning,
) -> FrameEvents {
    let events = crate::movement::update_body_simulation_in_frame(
        world,
        clusters,
        input,
        raw_dt,
        tuning.frame(),
        tuning.params(),
    );
    if events.reset {
        reset_body_clusters(clusters, world.spawn);
    }
    events
}

pub(crate) fn update_player_simulation_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_simulation_with_tuning_scratch(world, scratch, input, raw_dt, TEST_TUNING)
}

/// Scratch-based wrapper over the frame-explicit ledge runtime.
pub(crate) fn tick_active_ledge_grab_scratch(
    scratch: &mut BodyClusterScratch,
    input: InputState,
    dt: f32,
    tuning: TestTuning,
    events: &mut FrameEvents,
) -> bool {
    let mut clusters = scratch.as_mut();
    crate::ledge_grab::tick_active_ledge_grab_clusters_in_frame(
        &mut clusters,
        input,
        dt,
        tuning.frame(),
        tuning.params(),
        events,
    )
}

/// Scratch-based wrapper over the frame-explicit ledge acquisition probe.
pub(crate) fn try_start_ledge_grab_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    events: &mut FrameEvents,
) -> bool {
    let mut clusters = scratch.as_mut();
    crate::ledge_grab::try_start_ledge_grab_clusters_in_frame(
        world,
        &mut clusters,
        input,
        TEST_TUNING.frame(),
        events,
    )
}

pub(crate) fn ledge_boost(
    momentum_at_grab: Vec2,
    contact: LedgeContact,
    elapsed_at_initiation: f32,
    tuning: &TestTuning,
) -> Vec2 {
    crate::ledge_grab::ledge_boost_in_frame(
        momentum_at_grab,
        contact,
        elapsed_at_initiation,
        tuning.frame(),
        &tuning.params(),
    )
}

pub(crate) fn ledge_boost_for_state(state: LedgeGrabState, tuning: &TestTuning) -> Vec2 {
    crate::ledge_grab::ledge_boost_for_state_in_frame(state, tuning.frame(), &tuning.params())
}
