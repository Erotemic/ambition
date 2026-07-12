//! Unified, frame-aware movement-kernel facade.
//!
//! Every movement policy consumes the same body clusters, local input artifact,
//! world, current [`MotionFrame`](crate::MotionFrame), and timestep. The
//! environment resolves that frame once from an explicit reference basis plus
//! the complete world-space acceleration for the body tick. It never lives
//! inside a model spec and is never rebuilt by an individual solver.

use crate::collision_semantics::Contact;
use crate::{BodyClustersMut, MotionFrame, SweepSample, Vec2, World};

use super::model::MotionModel;
use super::surface_momentum::{self, SurfaceBody, SurfaceInputs};
use super::{touching_hazard_aabb, FrameEvents, InputState};

/// One deterministic movement tick's complete external context.
#[derive(Clone, Copy)]
pub struct MotionStepContext<'a> {
    pub world: &'a World,
    /// Controlled-body-local input: x=side/right, y=toward feet.
    pub input: InputState,
    /// The body's current acceleration/reference frame.  Both model arms receive
    /// this exact value; neither is permitted to derive a private gravity frame.
    pub frame: MotionFrame,
    pub facing_intent: f32,
    pub dt: f32,
}

/// Common observations produced by either movement policy.
#[derive(Clone, Debug)]
pub struct MotionStepResult {
    pub events: FrameEvents,
    /// Outward support normal while attached; opposite the resolved reference
    /// frame's down axis otherwise.
    pub surface_normal: Vec2,
}

/// Step one body through its selected movement policy.
///
/// This is the only public movement-kernel gateway.  Model dispatch happens
/// inside the trusted kernel, while body/controller identity remains outside.
pub fn step_motion(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    match model {
        MotionModel::AxisSwept(axis) => {
            let events = super::update_body_with_frame_clusters(
                ctx.world,
                clusters,
                ctx.input,
                ctx.frame,
                ctx.dt,
                axis.params,
            );
            MotionStepResult {
                surface_normal: support_normal(&events.contacts, ctx.frame),
                events,
            }
        }
        MotionModel::SurfaceMomentum(momentum) => {
            step_surface_momentum(momentum, clusters, ctx)
        }
    }
}

fn step_surface_momentum(
    motion: &mut super::SurfaceMomentumMotion,
    clusters: &mut BodyClustersMut<'_>,
    ctx: MotionStepContext<'_>,
) -> MotionStepResult {
    let sweep_entry = (clusters.kinematics.pos, clusters.kinematics.vel);
    let mut body = SurfaceBody {
        pos: clusters.kinematics.pos,
        vel: clusters.kinematics.vel,
        radius: clusters.kinematics.size.min_element() * 0.5,
        depth_lane: motion.depth_lane,
        motion: motion.state,
    };
    let mut contacts = Vec::new();
    surface_momentum::step_surface_body(
        &mut body,
        ctx.world,
        &motion.params,
        ctx.frame,
        SurfaceInputs {
            local_axis: Vec2::new(ctx.input.axis_x, ctx.input.axis_y),
            jump_pressed: ctx.input.jump_pressed,
        },
        ctx.dt,
        Some(&mut contacts),
    );

    clusters.kinematics.pos = body.pos;
    clusters.kinematics.vel = body.vel;
    if ctx.facing_intent.abs() > 0.001 {
        clusters.kinematics.facing = ctx.facing_intent.signum();
    }
    clusters.ground.on_ground = body.riding();
    motion.state = body.motion;
    motion.depth_lane = body.depth_lane;
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = SweepSample {
            prev: sweep_entry.0,
            curr: body.pos,
            vel: sweep_entry.1,
            half: clusters.kinematics.size * 0.5,
        };
    }

    let mut events = FrameEvents {
        contacts,
        ..FrameEvents::default()
    };
    let pos = clusters.kinematics.pos;
    let clamped = Vec2::new(
        pos.x.clamp(0.0, ctx.world.size.x),
        pos.y.clamp(0.0, ctx.world.size.y),
    );
    let fell_out = (pos - clamped).dot(ctx.frame.down()) > 200.0;
    if touching_hazard_aabb(ctx.world, clusters.kinematics.aabb()) || fell_out {
        events.hazard = true;
        events.reset = true;
    }

    MotionStepResult {
        surface_normal: support_normal(&events.contacts, ctx.frame),
        events,
    }
}

fn support_normal(contacts: &[Contact], frame: MotionFrame) -> Vec2 {
    contacts
        .iter()
        .rev()
        .map(|contact| contact.normal)
        .find(|normal| normal.length_squared() > 0.5)
        .unwrap_or(-frame.down())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AbilitySet, AxisSweptParams, BodyClusterScratch, MotionModelSpec};

    fn empty_world() -> World {
        World::new(
            "frame_covariance",
            Vec2::splat(10_000.0),
            Vec2::splat(500.0),
            Vec::new(),
        )
    }

    fn one_free_tick(
        model: &mut MotionModel,
        frame: MotionFrame,
        input: InputState,
    ) -> (Vec2, Vec2) {
        let world = empty_world();
        let start = Vec2::splat(500.0);
        let mut scratch = BodyClusterScratch::new_with_abilities(start, AbilitySet::default());
        let mut clusters = scratch.as_mut();
        step_motion(
            model,
            &mut clusters,
            MotionStepContext {
                world: &world,
                input,
                frame,
                facing_intent: 0.0,
                dt: 1.0 / 60.0,
            },
        );
        (clusters.kinematics.pos - start, clusters.kinematics.vel)
    }

    fn rotate(v: Vec2, radians: f32) -> Vec2 {
        let (sin, cos) = radians.sin_cos();
        Vec2::new(cos * v.x - sin * v.y, sin * v.x + cos * v.y)
    }

    #[test]
    fn swapping_models_preserves_shared_world_state_and_frame_is_external() {
        let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::ZERO, AbilitySet::default());
        scratch.kinematics.pos = Vec2::new(12.0, 34.0);
        scratch.kinematics.vel = Vec2::new(56.0, -78.0);
        let before = scratch.kinematics;

        let frame = MotionFrame::from_acceleration(Vec2::new(900.0, 0.0)).expect("non-zero acceleration");
        let mut model = MotionModel::default();
        model.apply_spec(MotionModelSpec::SurfaceMomentum(
            super::super::surface_momentum::MomentumParams::default(),
        ));

        assert_eq!(scratch.kinematics, before);
        assert_eq!(frame.down(), Vec2::X);
        assert_eq!(frame.magnitude(), 900.0);
    }

    #[test]
    fn both_physics_policies_are_covariant_under_an_arbitrary_frame_rotation() {
        let angle = 0.731_f32;
        let acceleration = Vec2::new(130.0, 900.0);
        let base = MotionFrame::from_acceleration(acceleration).expect("non-zero acceleration");
        let rotated = MotionFrame::from_acceleration(rotate(acceleration, angle)).expect("non-zero acceleration");
        let input = InputState {
            axis_x: 0.6,
            axis_y: -0.2,
            ..InputState::default()
        };

        for mut model in [
            MotionModel::axis_swept(AxisSweptParams::default()),
            MotionModel::surface_momentum(
                super::super::surface_momentum::MomentumParams::default(),
            ),
        ] {
            let mut rotated_model = model.clone();
            let (base_delta, base_vel) = one_free_tick(&mut model, base, input);
            let (rotated_delta, rotated_vel) =
                one_free_tick(&mut rotated_model, rotated, input);

            assert!((rotate(base_delta, angle) - rotated_delta).length() < 1e-4);
            assert!((rotate(base_vel, angle) - rotated_vel).length() < 1e-4);
        }
    }
}
