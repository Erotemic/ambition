//! `MotionModel` — movement identity as body DATA (fable review 2026-07-05,
//! AJ11 / R9.1).
//!
//! A typed per-body policy in the exact shape of `Perception` (R1.2b): a body
//! WITHOUT the component is `AxisSwept` — today's axis-role swept-AABB path,
//! byte-identical, zero migration. `SurfaceMomentum` opts the body into the
//! surface-follower solver (`ae::surface`) — slopes, loops, momentum — inside
//! the ONE shared `integrate_actor_body`, as a dispatch on body data, never a
//! parallel system.
//!
//! Movement identity travels WITH the body: possession is brain transfer
//! (`Brain::Player` lands on the body; its components stay), and this step
//! reads only the brain-produced `ActorControlFrame` — so a possessed
//! momentum body rides exactly as its AI does, by construction. Capabilities
//! belong to bodies; control authority drives them.

use ambition_engine_core as ae;
use bevy::prelude::Component;

/// How this body turns intent + world into motion.
#[derive(Component, Default, Debug)]
pub enum MotionModel {
    /// The axis-role swept-AABB path every current body uses (the default —
    /// an absent component reads as this).
    #[default]
    AxisSwept,
    /// The surface-follower solver: momentum locomotion over `SurfaceChain`s.
    SurfaceMomentum(MomentumMotion),
}

/// The surface-momentum policy's params + persistent solver state.
#[derive(Debug)]
pub struct MomentumMotion {
    /// RON-authorable motion feel (archetype row → here at spawn; R9.2).
    pub params: ae::surface::MomentumParams,
    /// The follower's persistent state (airborne vs riding-at-arc-length).
    pub state: ae::surface::SurfaceMotion,
}

impl MomentumMotion {
    pub fn new(params: ae::surface::MomentumParams) -> Self {
        Self {
            params,
            state: ae::surface::SurfaceMotion::Airborne,
        }
    }
}

/// One frame of surface-momentum motion for an actor body — the pure core
/// `integrate_actor_body` dispatches to when the body carries
/// `MotionModel::SurfaceMomentum`. Reads the same brain-produced intent every
/// controller writes (`run` = `locomotion.x`, `jump_pressed`), so human / AI /
/// RL / possession are indistinguishable here.
///
/// Writes back: position/velocity, `on_ground` (= riding), and the body's
/// reference frame (`surface_normal` follows the ridden chain — the footprint
/// and sprite orient to the surface, the same §B2 rule surface-walkers use).
/// Returns the step's resolved [`ae::collision_semantics::Contact`]s so the
/// caller can publish them (the home path routes them into
/// `FrameEvents.contacts`).
#[allow(clippy::too_many_arguments)]
pub fn step_momentum_body(
    kin: &mut ae::BodyKinematics,
    on_ground: &mut bool,
    surface_normal: &mut ae::Vec2,
    m: &mut MomentumMotion,
    world: &ae::World,
    gravity: ae::Vec2,
    run: f32,
    jump_pressed: bool,
    facing: f32,
    dt: f32,
) -> Vec<ae::collision_semantics::Contact> {
    let mut body = ae::surface::SurfaceBody {
        pos: kin.pos,
        vel: kin.vel,
        radius: kin.size.min_element() * 0.5,
        motion: m.state,
    };
    let mut contacts = Vec::new();
    ae::surface::step_surface_body(
        &mut body,
        world,
        &m.params,
        gravity,
        ae::surface::SurfaceInputs { run, jump_pressed },
        dt,
        Some(&mut contacts),
    );
    kin.pos = body.pos;
    kin.vel = body.vel;
    m.state = body.motion;
    *on_ground = body.riding();
    if facing != 0.0 {
        kin.facing = facing;
    }
    *surface_normal = if body.riding() {
        contacts
            .iter()
            .rev()
            .find(|c| {
                matches!(
                    c.source,
                    ae::collision_semantics::ContactSource::Chain { .. }
                )
            })
            .map(|c| c.normal)
            .unwrap_or_else(|| -gravity.normalize_or_zero())
    } else {
        -gravity.normalize_or_zero()
    };
    contacts
}

#[cfg(test)]
mod tests;
