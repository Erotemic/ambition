//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod home_momentum_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::*;
use crate::features::{MomentumMotion, MotionModel};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_engine_core as ae;

const DT: f32 = 1.0 / 60.0;

fn chain_world() -> ae::World {
    ae::World::new(
        "home-momentum",
        ae::Vec2::new(3000.0, 1200.0),
        ae::Vec2::new(200.0, 500.0),
        Vec::new(),
    )
    .with_chains(vec![ae::SurfaceChain::open(
        "floor",
        vec![ae::Vec2::new(0.0, 600.0), ae::Vec2::new(1500.0, 600.0)],
    )])
}

struct Rig {
    scratch: ae::BodyClusterScratch,
    model: MotionModel,
    hurtbox: ae::CenteredAabb,
    frame_out: PlayerBodyFrameOutput,
    world: ae::World,
    overlay: FeatureEcsWorldOverlay,
}

fn rig(world: ae::World) -> Rig {
    Rig {
        scratch: crate::avatar::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all()),
        model: MotionModel::SurfaceMomentum(MomentumMotion::new(ae::MomentumParams::default())),
        hurtbox: ae::CenteredAabb::new(world.spawn, ae::Vec2::splat(10.0)),
        frame_out: PlayerBodyFrameOutput::default(),
        world,
        overlay: FeatureEcsWorldOverlay::default(),
    }
}

fn step(r: &mut Rig, frame: ActorControlFrame) {
    let mut clusters = r.scratch.as_mut();
    integrate_home_body(
        frame,
        &r.world,
        &mut clusters,
        &BodyCombat::default(),
        &mut r.hurtbox,
        &mut r.frame_out,
        &[],
        &mut r.model,
        ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        ae::DEFAULT_TUNING,
        SandboxFeelTuning::default(),
        DT,
        DT,
        &r.overlay,
    );
}

#[test]
fn worn_momentum_home_body_rides_runs_and_jumps() {
    let mut r = rig(chain_world());
    // Fall onto the chain, then run right.
    let mut run = ActorControlFrame::neutral();
    run.locomotion.x = 1.0;
    run.facing = 1.0;
    // Sample mid-run: kept running, the body (correctly) launches off the
    // chain's open end around x=1500 and falls out — not this test's
    // subject.
    let mut mid_run = false;
    for _ in 0..240 {
        step(&mut r, run);
        if r.scratch.ground.on_ground && r.scratch.kinematics.pos.x > 500.0 {
            mid_run = true;
            break;
        }
    }
    assert!(mid_run, "rode the chain and advanced past x=500");
    // The hurtbox publish followed the body.
    assert!((r.hurtbox.center - r.scratch.kinematics.pos).length() < 40.0);
    // The frame reports ride contacts (the contact vocabulary reaches the
    // home body's FrameEvents).
    assert!(
        r.frame_out.events.contacts.iter().any(|c| matches!(
            c.source,
            ae::collision_semantics::ContactSource::Chain { .. }
        )),
        "ride contact published"
    );
    // Jump: the GATED input path maps jump_pressed through.
    let mut jump = run;
    jump.jump_pressed = true;
    step(&mut r, jump);
    assert!(!r.scratch.ground.on_ground, "left the surface");
    assert!(
        r.scratch.kinematics.vel.y < -400.0,
        "launched along +normal: {:?}",
        r.scratch.kinematics.vel
    );
}

#[test]
fn momentum_home_body_rides_ordinary_block_floors() {
    // THE Sanic-in-a-normal-room regression (Jon, 2026-07-05): every
    // sandbox room floors with AABB `Block`s, not authored chains. A
    // worn momentum body must land, run, and jump on plain solids —
    // blocks are surfaces (`Block::boundary_chain`), not just obstacles.
    let world = ae::World::new(
        "home-momentum-blocks",
        ae::Vec2::new(3000.0, 1200.0),
        ae::Vec2::new(200.0, 500.0),
        vec![ae::world::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 600.0),
            ae::Vec2::new(2800.0, 100.0),
        )],
    );
    let mut r = rig(world);
    let mut run = ActorControlFrame::neutral();
    run.locomotion.x = 1.0;
    run.facing = 1.0;
    let mut mid_run = false;
    for _ in 0..240 {
        step(&mut r, run);
        if r.scratch.ground.on_ground && r.scratch.kinematics.pos.x > 500.0 {
            mid_run = true;
            break;
        }
    }
    assert!(mid_run, "rode the block floor and advanced past x=500");
    assert!(
        r.frame_out.events.contacts.iter().any(|c| matches!(
            c.source,
            ae::collision_semantics::ContactSource::Block { .. }
        )),
        "block ride contact published"
    );
    let mut jump = run;
    jump.jump_pressed = true;
    step(&mut r, jump);
    assert!(!r.scratch.ground.on_ground, "left the floor");
    assert!(
        r.scratch.kinematics.vel.y < -400.0,
        "jumped off a block floor: {:?}",
        r.scratch.kinematics.vel
    );
}

#[test]
fn momentum_home_body_dies_in_pits_and_respawns_airborne() {
    // The chain ends mid-world; running off it drops the body past the
    // world bottom — the Q16 hazard/OOB parity gate must fire.
    let mut r = rig(chain_world());
    let mut run = ActorControlFrame::neutral();
    run.locomotion.x = 1.0;
    let mut saw_reset = false;
    for _ in 0..1800 {
        step(&mut r, run);
        if r.frame_out.reset {
            saw_reset = true;
            break;
        }
    }
    assert!(saw_reset, "fell out and the reset flagged");
    assert_eq!(
        r.scratch.kinematics.pos, r.world.spawn,
        "engine-level body reset to spawn"
    );
    assert!(
        matches!(
            r.model,
            MotionModel::SurfaceMomentum(MomentumMotion {
                state: ae::SurfaceMotion::Airborne,
                ..
            })
        ),
        "respawns airborne, never 'riding' a chain it left"
    );
}
