//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod dash_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! S3d: dash as a body-enforced capability. These drive the REAL grounded
//! integration (`ActorMut::update` → the shared spine), so they prove the
//! body owns the burst — a possessing human and an AI brain dash identically
//! because both only set `dash_pressed` (invariants I2/I3).
use super::*;
use crate::features::ecs::actor_clusters::{ActorBody, ActorClusterSeed};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_entity_catalog::placements::CharacterBrain;

/// A wide solid floor; bodies rest on its top face at y = 100.
fn floored_world() -> ae::World {
    ae::World::new(
        "dash_test",
        ae::Vec2::new(4000.0, 800.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(-2000.0, 100.0),
            ae::Vec2::new(4000.0, 80.0),
        )],
    )
}

/// Drop a grounded body (dash-capable iff `can_dash`) and drive a full-right
/// dash for `ticks` steps; return how far it traveled along +x.
fn dash_run(can_dash: bool, ticks: u32) -> f32 {
    let world = floored_world();
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "dasher".to_string(),
        "Dasher".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    // Rest the body on the floor top (y = 100): center a half-height above it.
    let half_h = seed.kin.size.y * 0.5;
    seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
    seed.kin.vel = ae::Vec2::ZERO;
    seed.kin.facing = 1.0;
    seed.surface.gravity_scale = 1.0;
    // The dash ability lives on the body's movement `AbilitySet`, unioned in
    // from the character's movement kit; build the body from a dash-bearing kit
    // so the pipeline dash limb matches.
    seed.body = ActorBody::from_kit(
        ae::AbilitySet {
            dash: can_dash,
            ..ae::AbilitySet::NONE
        },
        false,
    );
    seed.body.0.ground.on_ground = true;
    let start_x = seed.kin.pos.x;
    let mut model = crate::features::MotionModel::default();
    let mut em = seed.as_actor_mut();
    let mut frame = ActorControlFrame::neutral();
    frame.locomotion = ae::Vec2::new(1.0, 0.0);
    frame.dash_pressed = true;
    frame.facing = 1.0;
    let dt = 1.0 / 60.0;
    for _ in 0..ticks {
        em.update(
            &world,
            ae::Vec2::new(2000.0, em.kin.pos.y),
            FeatureCombatTuning::default(),
            dt,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
            crate::time::feel::SandboxFeelTuning::default(),
            (0.0, 0.0),
        );
    }
    em.kin.pos.x - start_x
}

#[test]
fn a_dash_capable_body_covers_more_ground_than_a_walker_over_the_window() {
    // ~the dash window (DASH_TIME_S = 0.18 s ≈ 11 ticks), plus a tick of slack.
    let dashed = dash_run(true, 12);
    let walked = dash_run(false, 12);
    assert!(
        dashed > walked * 1.3,
        "the dash burst should cover meaningfully more ground than a top-speed \
         walk over the same window: dashed={dashed:.1}px walked={walked:.1}px"
    );
}

/// B2 (fable review §B2): a non-surface-walker's published reference-frame
/// normal must track LIVE gravity at its position, not stay pinned to the
/// spawn constant `(0,-1)`. Consumers derive the body frame as
/// `-surface_normal` (shield block side, slash knockback, ranged muzzle/aim);
/// if it stayed screen-down, a body under sideways/inverted gravity would
/// block/recoil/fire in the down-gravity frame while its movement obeyed the
/// real field. Regression guard for the `!surface_walker` LIVE write.
#[test]
fn a_non_surface_walker_keeps_its_frame_normal_live_under_gravity() {
    let world = floored_world();
    for gravity in [
        ae::Vec2::new(0.0, 1.0),  // down (baseline)
        ae::Vec2::new(1.0, 0.0),  // right
        ae::Vec2::new(0.0, -1.0), // up
        ae::Vec2::new(-1.0, 0.0), // left
    ] {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        let mut seed = ActorClusterSeed::new(
            "grunt".to_string(),
            "Grunt".to_string(),
            aabb,
            CharacterBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        // A plain (non-clinging) fighter; make the invariant explicit.
        seed.config.tuning.surface_walker = false;
        // Spawn-pinned to screen-down — the exact stale state B2 fixes.
        seed.surface.surface_normal = ae::Vec2::new(0.0, -1.0);
        seed.kin.pos = ae::Vec2::new(0.0, 40.0);
        let mut model = crate::features::MotionModel::default();
        let mut em = seed.as_actor_mut();
        em.update(
            &world,
            ae::Vec2::new(2000.0, em.kin.pos.y),
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            ActorControlFrame::neutral(),
            &mut model,
            ae::MotionFrame::from_direction(gravity, ae::GRAVITY),
            crate::time::feel::SandboxFeelTuning::default(),
            (0.0, 0.0),
        );
        let expected = -gravity;
        assert!(
            (em.surface.surface_normal - expected).length() < 1e-3,
            "gravity {gravity:?}: the frame normal must track live gravity; \
             got {:?}, want {expected:?}",
            em.surface.surface_normal
        );
    }
}

/// Drive a grounded walker (locomotion full-right) for `ticks` steps under
/// the given post-hit stagger `(hitstun_timer, recoil_lock_timer)`; return
/// the ground covered along +x. The §A2 step 7 witness rig.
fn walk_run_staggered(stagger: (f32, f32), ticks: u32) -> f32 {
    let world = floored_world();
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "staggered".to_string(),
        "Staggered".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    let half_h = seed.kin.size.y * 0.5;
    seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
    seed.kin.vel = ae::Vec2::ZERO;
    seed.kin.facing = 1.0;
    seed.surface.gravity_scale = 1.0;
    seed.body.0.ground.on_ground = true;
    let start_x = seed.kin.pos.x;
    let mut model = crate::features::MotionModel::default();
    let mut em = seed.as_actor_mut();
    let mut frame = ActorControlFrame::neutral();
    frame.locomotion = ae::Vec2::new(1.0, 0.0);
    frame.facing = 1.0;
    let dt = 1.0 / 60.0;
    for _ in 0..ticks {
        em.update(
            &world,
            ae::Vec2::new(2000.0, em.kin.pos.y),
            FeatureCombatTuning::default(),
            dt,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
            crate::time::feel::SandboxFeelTuning::default(),
            stagger,
        );
    }
    em.kin.pos.x - start_x
}

/// §A2 step 7: the post-hit stagger gates an actor's input through the SAME
/// rule the player's input bridge applies — recoil-lock is a hard zero (no
/// steering at all), hitstun leaves only reduced movement authority.
#[test]
fn a_staggered_body_loses_input_authority_like_the_player() {
    let free = walk_run_staggered((0.0, 0.0), 12);
    let recoil_locked = walk_run_staggered((0.0, 1.0), 12);
    let hitstunned = walk_run_staggered((1.0, 0.0), 12);
    assert!(
        free > 10.0,
        "sanity: an unstaggered walker covers real ground (got {free:.1}px)"
    );
    assert!(
        recoil_locked.abs() < 0.5,
        "a recoil-locked body has NO steering authority (moved {recoil_locked:.1}px)"
    );
    assert!(
        hitstunned < free * 0.8,
        "hitstun reduces movement authority (stunned {hitstunned:.1}px vs free {free:.1}px)"
    );
}

#[test]
fn an_uncapable_body_does_not_burst_and_just_walks() {
    // Sanity: with the capability off, `dash_pressed` never opens a window —
    // the body's attack state stays dash-inert (the body enforces the kit).
    let world = floored_world();
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "walker".to_string(),
        "Walker".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    let half_h = seed.kin.size.y * 0.5;
    seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
    seed.surface.gravity_scale = 1.0;
    seed.body = ActorBody::from_kit(ae::AbilitySet::NONE, false);
    seed.body.0.ground.on_ground = true;
    let mut model = crate::features::MotionModel::default();
    let mut em = seed.as_actor_mut();
    let mut frame = ActorControlFrame::neutral();
    frame.locomotion = ae::Vec2::new(1.0, 0.0);
    frame.dash_pressed = true;
    em.update(
        &world,
        ae::Vec2::new(2000.0, em.kin.pos.y),
        FeatureCombatTuning::default(),
        1.0 / 60.0,
        false,
        frame,
        &mut model,
        ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        crate::time::feel::SandboxFeelTuning::default(),
        (0.0, 0.0),
    );
    let crate::features::MotionModel::AxisSwept(axis) = &model else {
        panic!("test body is not axis-swept");
    };
    assert!(
        axis.state.dash_timer <= 0.0,
        "a body without the dash capability must not open a dash window"
    );
}

/// Witness for the aerial reconciliation: an aerial body (fly_enabled) is
/// steered by the brain's world-space `velocity_target` THROUGH the shared
/// pipeline's flight limb (the `velocity_target`→stick-intent bridge). It flies
/// toward the command and holds altitude (gravity-free flight, no idle bob).
#[test]
fn an_aerial_body_steers_toward_its_velocity_target_through_the_flight_limb() {
    let world = floored_world();
    // Hover in open air well above the floor (floor top is y = 100).
    let aabb = ae::Aabb::new(ae::Vec2::new(0.0, -200.0), ae::Vec2::new(24.0, 24.0));
    let mut seed = ActorClusterSeed::new(
        "flyer".to_string(),
        "Flyer".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    seed.kin.pos = ae::Vec2::new(0.0, -200.0);
    seed.kin.vel = ae::Vec2::ZERO;
    seed.surface.gravity_scale = 0.0;
    // Aerial body: is_aerial forces the fly ability + fly_enabled from spawn.
    seed.body = ActorBody::from_kit(ae::AbilitySet::NONE, true);
    let start = seed.kin.pos;
    let mut model = crate::features::MotionModel::default();
    let mut em = seed.as_actor_mut();
    let mut frame = ActorControlFrame::neutral();
    // Command a pure +x world velocity (the free-mover modality).
    frame.velocity_target = ae::Vec2::new(300.0, 0.0);
    let dt = 1.0 / 60.0;
    for _ in 0..60 {
        em.update(
            &world,
            ae::Vec2::new(2000.0, em.kin.pos.y),
            FeatureCombatTuning::default(),
            dt,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
            crate::time::feel::SandboxFeelTuning::default(),
            (0.0, 0.0),
        );
    }
    assert!(
        em.kin.pos.x - start.x > 100.0,
        "an aerial body should fly toward its +x velocity_target through the \
         shared flight limb; moved {:.1}px",
        em.kin.pos.x - start.x
    );
    assert!(
        (em.kin.pos.y - start.y).abs() < 50.0,
        "gravity-free flight holds altitude (no fall, no idle hover bob); \
         drifted {:.1}px on y",
        em.kin.pos.y - start.y
    );
    assert!(!em.ground.on_ground, "a flying body is never grounded");
}
