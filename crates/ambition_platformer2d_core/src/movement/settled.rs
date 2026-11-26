use bevy_ecs::component::Component;

use crate::body_clusters::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyDashState, BodyDodgeState,
    BodyFlightState, BodyGroundState, BodyJumpState, BodyKinematics, BodyLedgeState, BodyModeState,
    BodyRestartLatch, BodyShieldState, BodyWallState,
};
use crate::frame::{
    MapConvention,
    thread::{MotionAdmission, PieceBudget},
};
use crate::layered::LayeredIdentity;
use crate::motion_codec::same_axis_motion;
use crate::snapshot::{ExactState, SnapshotState};
use crate::world::WorldEdgeMargins;
use crate::{Block, BodyClustersMut, MotionFrame, SurfaceChain, SweepSample, Vec2, World};

use super::events::{FrameEvents, StepContacts};
use super::kernel::{MotionStepContext, MotionStepResult, SupportFact, solve_motion};
use super::{GroundContactTransition, InputState, MotionModel};

/// Result of a movement step that left its input state unchanged.
/// The cache is derived. A restored body must match the same inputs to use it.
#[derive(Component, Default)]
pub struct SettledMotion {
    step: Option<Box<SettledStep>>,
    prior: Vec<u8>,
    current: Vec<u8>,
    status: SettledMotionStatus,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettledMotionStatus {
    #[default]
    Unstepped,
    Motion,
    Control,
    Environment,
    Aperture,
    Events,
    ReadyToProve,
    ChangedState,
    Proved(ProofMiss),
    Reused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProofMiss {
    New,
    StepInput,
    Blocks,
    Chains,
    WorldBounds,
    BodyState,
}

struct SettledStep {
    body: FixedBodyState,
    axis_motion: Option<crate::AxisSweptMotion>,
    environment: Geometry,
    input: StepInput,
    contacts: StepContacts,
    support: SupportFact,
    surface_normal: Vec2,
    sample: SweepSample,
}

impl SettledStep {
    fn result(&self) -> MotionStepResult {
        // A settled step has contacts and no motion events.
        MotionStepResult {
            events: FrameEvents { contacts: self.contacts.clone(), ..Default::default() },
            support: self.support,
            surface_normal: self.surface_normal,
        }
    }
}

struct Geometry {
    blocks: LayeredIdentity<Block>,
    chains: LayeredIdentity<SurfaceChain>,
    size: [u32; 2],
    edges: [Option<u32>; 3],
}

impl Geometry {
    fn capture(world: &World) -> Self {
        Self {
            blocks: world.blocks.identity(),
            chains: world.chains.identity(),
            size: bits(world.size),
            edges: edge_bits(world),
        }
    }

    fn changed(&self, world: &World) -> Option<ProofMiss> {
        if !self.blocks.matches(&world.blocks) {
            Some(ProofMiss::Blocks)
        } else if !self.chains.matches(&world.chains) {
            Some(ProofMiss::Chains)
        } else if self.size != bits(world.size) || self.edges != edge_bits(world) {
            Some(ProofMiss::WorldBounds)
        } else {
            None
        }
    }
}

#[derive(PartialEq)]
struct StepInput {
    dt: u32,
    grounding: crate::Grounding,
    frame: [[u32; 2]; 5],
    facing: u32,
    control: [[u32; 2]; 3],
    prior_down: [u32; 2],
    carrier: Option<u64>,
    aperture_policy: Option<(PieceBudget, MapConvention)>,
}

impl StepInput {
    fn for_body(
        ctx: MotionStepContext<'_>,
        body: &BodyClustersMut<'_>,
        prior_down: Vec2,
    ) -> Result<Self, SettledMotionStatus> {
        let MotionStepContext {
            world,
            admission,
            input,
            frame,
            facing_intent,
            dt,
            contact,
            pose_owned_externally,
            recovery_commitment_outstanding,
        } = ctx;
        if !(dt > 0.0 && dt.is_finite() && (body.ground.on_ground || body.flight.fly_enabled))
            || body.sweep.is_none()
            || body.kinematics.vel != Vec2::ZERO
        {
            return Err(SettledMotionStatus::Motion);
        }
        if input != InputState::default()
            || input.control_dt.to_bits() != 0
            || frame.response_refused()
            || pose_owned_externally
            || recovery_commitment_outstanding
            || !contact.is_inert()
        {
            return Err(SettledMotionStatus::Control);
        }
        if !world.water_regions.is_empty()
            || !world.climbable_regions.is_empty()
            || *body.env_contact != Default::default()
        {
            return Err(SettledMotionStatus::Environment);
        }
        let aperture_policy = match admission {
            None => None,
            Some(MotionAdmission {
                links: [],
                first: None,
                budget,
                convention,
            }) => Some((budget, convention)),
            Some(_) => return Err(SettledMotionStatus::Aperture),
        };
        Ok(Self {
            dt: dt.to_bits(),
            grounding: body.grounding,
            frame: frame_bits(frame),
            facing: facing_intent.to_bits(),
            control: [
                bits(input.axes.vec()),
                bits(input.blink_quick_dir.vec()),
                bits(input.blink_aim_step.vec()),
            ],
            prior_down: bits(prior_down),
            carrier: body.carrier,
            aperture_policy,
        })
    }
}

#[inline]
fn bits(vector: Vec2) -> [u32; 2] {
    [vector.x.to_bits(), vector.y.to_bits()]
}

fn frame_bits(frame: MotionFrame) -> [[u32; 2]; 5] {
    [
        bits(frame.down()),
        bits(frame.side()),
        bits(frame.gravity_acceleration()),
        bits(frame.external_acceleration()),
        bits(frame.carried_acceleration()),
    ]
}

fn edge_bits(world: &World) -> [Option<u32>; 3] {
    let WorldEdgeMargins { fall, side, rise } = world.edges;
    [Some(fall.to_bits()), side.map(f32::to_bits), rise.map(f32::to_bits)]
}

// Retain fixed-size values once per proof. A reuse check reads the live values.
macro_rules! fixed_body_state {
    ($($field:ident: $ty:ty),+ $(,)?) => {
        struct FixedBodyState { $($field: $ty),+ }

        impl FixedBodyState {
            fn capture(body: &BodyClustersMut<'_>) -> Self {
                let BodyClustersMut {
                    $($field,)+ settled: _, grounding: _, carrier: _, sweep: _, env_contact: _,
                    resources: _, effects: _,
                } = body;
                Self { $($field: **$field),+ }
            }

            fn matches(&self, body: &BodyClustersMut<'_>) -> bool {
                $(self.$field.same_bits(body.$field))&&+
            }
        }
    };
}

fixed_body_state! {
    abilities: BodyAbilities,
    kinematics: BodyKinematics,
    base_size: BodyBaseSize,
    ground: BodyGroundState,
    wall: BodyWallState,
    jump: BodyJumpState,
    dash: BodyDashState,
    flight: BodyFlightState,
    blink: BodyBlinkState,
    ledge: BodyLedgeState,
    dodge: BodyDodgeState,
    shield: BodyShieldState,
    body_mode: BodyModeState,
    action_buffer: BodyActionBuffer,
    restart: BodyRestartLatch,
}

fn encode_variable_state(model: &MotionModel, body: &BodyClustersMut<'_>, out: &mut Vec<u8>) {
    out.clear();
    let encode_model = !matches!(model, MotionModel::AxisSwept(_));
    let has_effects = !body.effects.is_empty();
    let has_resources = body.resources.is_some();
    let fields = u8::from(encode_model) | (u8::from(has_effects) << 1) | (u8::from(has_resources) << 2);
    if fields == 0 {
        return;
    }
    out.push(fields);
    if encode_model {
        model.encode(out);
    }
    if has_effects {
        body.effects.encode(out);
    }
    if let Some(resources) = body.resources.as_deref() {
        resources.encode(out);
    }
}

fn can_retain_result(result: &MotionStepResult, body: &BodyClustersMut<'_>) -> bool {
    let FrameEvents {
        motion_refusal,
        operations,
        blinks,
        reset,
        ground_contact,
        contacts: _,
        constraint_conflicts,
    } = &result.events;
    (result.support.is_held() || body.flight.fly_enabled)
        && motion_refusal.is_none()
        && operations.is_empty()
        && blinks.is_empty()
        && reset.is_none()
        && *ground_contact == GroundContactTransition::Unchanged
        && constraint_conflicts.is_empty()
        && *body.env_contact == Default::default()
}

impl SettledMotion {
    /// Whether the most recent movement step reused its settled result.
    pub fn reused(&self) -> bool {
        self.status == SettledMotionStatus::Reused
    }

    pub fn status(&self) -> SettledMotionStatus {
        self.status
    }

    pub(super) fn step(
        &mut self,
        model: &mut MotionModel,
        body: &mut BodyClustersMut<'_>,
        ctx: MotionStepContext<'_>,
        prior_down: Vec2,
    ) -> MotionStepResult {
        let input = match StepInput::for_body(ctx, body, prior_down) {
            Ok(input) => input,
            Err(status) => {
                self.step = None;
                self.status = status;
                return solve_motion(model, body, ctx, prior_down);
            }
        };
        // Record terrain reads again after the solver produces a quiet step.
        if self.status == SettledMotionStatus::Events {
            let result = solve_motion(model, body, ctx, prior_down);
            if can_retain_result(&result, body) {
                self.status = SettledMotionStatus::ReadyToProve;
            }
            return result;
        }
        let current_motion = axis_motion(model);
        encode_variable_state(model, body, &mut self.current);
        let miss = if let Some(step) = &self.step {
            if step.input != input {
                ProofMiss::StepInput
            } else if let Some(changed) = step.environment.changed(ctx.world) {
                changed
            } else if !step.body.matches(body)
                || self.prior != self.current
                || !same_motion(step.axis_motion.as_ref(), current_motion)
            {
                ProofMiss::BodyState
            } else {
                *body
                    .sweep
                    .as_deref_mut()
                    .expect("settled step records movement") = step.sample;
                self.status = SettledMotionStatus::Reused;
                return step.result();
            }
        } else {
            ProofMiss::New
        };
        let prior_body = FixedBodyState::capture(body);
        let prior_motion = current_motion.copied();
        self.step = None;
        self.status = SettledMotionStatus::Events;
        let mut observed = ctx.world.clone();
        observed.blocks.record_reads();
        observed.chains.record_reads();
        let mut result = solve_motion(
            model,
            body,
            MotionStepContext {
                world: &observed,
                ..ctx
            },
            prior_down,
        );
        if can_retain_result(&result, body) {
            encode_variable_state(model, body, &mut self.prior);
            if prior_body.matches(body)
                && self.prior == self.current
                && same_motion(prior_motion.as_ref(), axis_motion(model))
            {
                result.events.contacts.share();
                self.status = SettledMotionStatus::Proved(miss);
                self.step = Some(Box::new(SettledStep {
                    body: prior_body,
                    axis_motion: prior_motion,
                    environment: Geometry::capture(&observed),
                    input,
                    contacts: result.events.contacts.clone(),
                    support: result.support,
                    surface_normal: result.surface_normal,
                    sample: *body
                        .sweep
                        .as_deref()
                        .expect("settled step records movement"),
                }));
            } else {
                self.status = SettledMotionStatus::ChangedState;
            }
        }
        result
    }
}

fn axis_motion(model: &MotionModel) -> Option<&crate::AxisSweptMotion> {
    match model {
        MotionModel::AxisSwept(axis) => Some(axis),
        _ => None,
    }
}

fn same_motion(a: Option<&crate::AxisSweptMotion>, b: Option<&crate::AxisSweptMotion>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => same_axis_motion(a, b),
        (None, None) => true,
        _ => false,
    }
}
