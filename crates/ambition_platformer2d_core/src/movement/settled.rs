use bevy_ecs::component::Component;

use crate::frame::{
    MapConvention,
    thread::{MotionAdmission, PieceBudget},
};
use crate::layered::LayeredIdentity;
use crate::snapshot::SnapshotState;
use crate::world::WorldEdgeMargins;
use crate::{Block, BodyClustersMut, MotionFrame, SurfaceChain, SweepSample, Vec2, World};

use super::kernel::{MotionStepContext, MotionStepResult, solve_motion};
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
    environment: Geometry,
    input: StepInput,
    result: MotionStepResult,
    sample: SweepSample,
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
        if !(dt > 0.0 && dt.is_finite() && body.ground.on_ground)
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

fn bits(vector: Vec2) -> [u32; 2] {
    vector.to_array().map(f32::to_bits)
}

fn frame_bits(frame: MotionFrame) -> [[u32; 2]; 5] {
    [
        frame.down(),
        frame.side(),
        frame.gravity_acceleration(),
        frame.external_acceleration(),
        frame.carried_acceleration(),
    ]
    .map(bits)
}

fn edge_bits(world: &World) -> [Option<u32>; 3] {
    let WorldEdgeMargins { fall, side, rise } = world.edges;
    [Some(fall), side, rise].map(|x| x.map(f32::to_bits))
}

// Use the simulation codecs so reuse and rollback make the same distinction
// between numeric states, including the sign of zero.
fn encode_body(model: &MotionModel, body: &BodyClustersMut<'_>, out: &mut Vec<u8>) {
    let BodyClustersMut {
        settled: _,
        carrier: _,
        sweep: _,
        env_contact: _,
        abilities,
        kinematics,
        base_size,
        ground,
        wall,
        jump,
        dash,
        flight,
        blink,
        ledge,
        dodge,
        shield,
        body_mode,
        resources,
        effects,
        action_buffer,
        restart,
    } = body;
    out.clear();
    model.encode(out);
    abilities.encode(out);
    kinematics.encode(out);
    base_size.encode(out);
    ground.encode(out);
    wall.encode(out);
    jump.encode(out);
    dash.encode(out);
    flight.encode(out);
    blink.encode(out);
    ledge.encode(out);
    dodge.encode(out);
    shield.encode(out);
    body_mode.encode(out);
    effects.encode(out);
    action_buffer.encode(out);
    restart.encode(out);
    out.push(u8::from(resources.is_some()));
    if let Some(resources) = resources.as_deref() {
        resources.encode(out);
    }
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
        encode_body(model, body, &mut self.current);
        let miss = if let Some(step) = &self.step {
            if step.input != input {
                ProofMiss::StepInput
            } else if let Some(changed) = step.environment.changed(ctx.world) {
                changed
            } else if self.prior != self.current {
                ProofMiss::BodyState
            } else {
                *body
                    .sweep
                    .as_deref_mut()
                    .expect("settled step records movement") = step.sample;
                self.status = SettledMotionStatus::Reused;
                return step.result.clone();
            }
        } else {
            ProofMiss::New
        };
        self.step = None;
        self.status = SettledMotionStatus::Events;
        let mut observed = ctx.world.clone();
        observed.blocks.record_reads();
        observed.chains.record_reads();
        let result = solve_motion(
            model,
            body,
            MotionStepContext {
                world: &observed,
                ..ctx
            },
            prior_down,
        );
        let events = &result.events;
        if result.support.is_held()
            && events.motion_refusal.is_none()
            && events.reset.is_none()
            && events.operations.is_empty()
            && events.blinks.is_empty()
            && events.constraint_conflicts.is_empty()
            && events.ground_contact == GroundContactTransition::Unchanged
            && *body.env_contact == Default::default()
        {
            encode_body(model, body, &mut self.prior);
            if self.prior == self.current {
                self.status = SettledMotionStatus::Proved(miss);
                self.step = Some(Box::new(SettledStep {
                    environment: Geometry::capture(&observed),
                    input,
                    result: result.clone(),
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
