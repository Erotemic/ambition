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
#[derive(Component, Default)]
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
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    const G: ae::Vec2 = ae::Vec2::new(0.0, 1450.0);

    fn chain_world() -> ae::World {
        ae::World::new(
            "momentum-test",
            ae::Vec2::new(4000.0, 2000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
        .with_chains(vec![ae::SurfaceChain::open(
            "ramp",
            vec![
                ae::Vec2::new(0.0, 600.0),
                ae::Vec2::new(1200.0, 600.0),
                ae::Vec2::new(1800.0, 300.0),
            ],
        )])
    }

    fn kin_at(pos: ae::Vec2) -> ae::BodyKinematics {
        ae::BodyKinematics {
            pos,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(28.0, 28.0),
            facing: 1.0,
        }
    }

    #[test]
    fn momentum_body_falls_lands_and_runs_up_the_ramp() {
        let world = chain_world();
        let mut kin = kin_at(ae::Vec2::new(200.0, 400.0));
        let mut m = MomentumMotion::new(ae::surface::MomentumParams::default());
        let mut on_ground = false;
        let mut normal = ae::Vec2::new(0.0, -1.0);
        // Run right until the body is riding partway UP the ramp segment
        // (sample there — kept running, it would launch off the open end,
        // which is the solver's correct behavior, not this test's subject).
        let mut on_ramp = None;
        for _ in 0..600 {
            step_momentum_body(
                &mut kin,
                &mut on_ground,
                &mut normal,
                &mut m,
                &world,
                G,
                1.0,
                false,
                1.0,
                DT,
            );
            if on_ground && kin.pos.x > 1300.0 && kin.pos.x < 1600.0 {
                on_ramp = Some((kin.pos, normal));
                break;
            }
        }
        let (pos, ramp_normal) = on_ramp.expect("landed, ran the flat, and climbed the ramp");
        assert!(pos.y < 590.0, "climbing the ramp: {pos:?}");
        // The body frame follows the slope: on the ramp the normal tilts.
        assert!(
            ramp_normal.x.abs() > 0.1 && ramp_normal.y < -0.5,
            "surface_normal follows the ridden slope: {ramp_normal:?}"
        );
    }

    #[test]
    fn jump_intent_launches_and_facing_writes_back() {
        let world = chain_world();
        let mut kin = kin_at(ae::Vec2::new(200.0, 400.0));
        let mut m = MomentumMotion::new(ae::surface::MomentumParams::default());
        let mut on_ground = false;
        let mut normal = ae::Vec2::new(0.0, -1.0);
        // Settle onto the chain.
        for _ in 0..120 {
            step_momentum_body(
                &mut kin,
                &mut on_ground,
                &mut normal,
                &mut m,
                &world,
                G,
                0.0,
                false,
                0.0,
                DT,
            );
        }
        assert!(on_ground);
        // Jump (any controller pressing jump looks identical here).
        step_momentum_body(
            &mut kin,
            &mut on_ground,
            &mut normal,
            &mut m,
            &world,
            G,
            0.0,
            true,
            -1.0,
            DT,
        );
        assert!(!on_ground, "left the surface");
        assert!(kin.vel.y < -400.0, "launched along +normal: {:?}", kin.vel);
        assert_eq!(kin.facing, -1.0, "explicit facing intent applied");
        assert_eq!(normal, ae::Vec2::new(0.0, -1.0), "airborne frame = gravity");
    }
}
