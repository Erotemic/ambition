use super::*;
use ambition_platformer2d_core::movement::{MotionModel, SurfaceMomentumMotion as MomentumMotion};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_platformer2d_core as ae;

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
}

fn rig(world: ae::World) -> Rig {
    Rig {
        scratch: crate::avatar::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all()),
        model: MotionModel::SurfaceMomentum(MomentumMotion::new(ae::MomentumParams::default())),
        hurtbox: ae::CenteredAabb::new(world.spawn, ae::Vec2::splat(10.0)),
        frame_out: PlayerBodyFrameOutput::default(),
        world,
    }
}

fn step(r: &mut Rig, frame: ActorControlFrame) -> Option<ae::Vec2> {
    step_as(
        r,
        frame,
        ambition_characters::actor::Invulnerability::none(),
    )
}

fn step_as(
    r: &mut Rig,
    frame: ActorControlFrame,
    invulnerable: ambition_characters::actor::Invulnerability,
) -> Option<ae::Vec2> {
    let mut clusters = r.scratch.as_mut();
    integrate_home_body(
        frame,
        &r.world,
        &mut clusters,
        &mut BodyCombat::default(),
        invulnerable,
        false,
        // In play: this rig is about momentum, not about dying.
        false,
        // Not tumbling: these fixtures are about momentum, not the floor game.
        false,
        &mut r.hurtbox,
        &mut r.frame_out,
        &mut r.model,
        ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
        ae::DEFAULT_TUNING,
        Platformer2dFeelTuningMonolith::default(),
        // No move playing on a momentum rig: full steering authority.
        1.0,
        DT,
        DT,
        // No move playing, so this rig is never helpless — it is about momentum.
        None,
        // A momentum rig with one body in it: nobody to be solid to.
        ae::BodyContactField::NONE,
    )
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
    let mut riding_up = None;
    for _ in 0..240 {
        riding_up = step(&mut r, run);
        if r.scratch.ground.on_ground && r.scratch.kinematics.pos.x > 500.0 {
            mid_run = true;
            break;
        }
    }
    assert!(mid_run, "rode the chain and advanced past x=500");
    // The ridden-surface fact publishes the flat chain's outward normal (the
    // roll reflex plants the rider's feet on it).
    let up = riding_up.expect("a riding momentum body publishes its surface up");
    assert!(
        (up - ae::Vec2::new(0.0, -1.0)).length() < 1e-3,
        "flat-floor ride publishes world-up: {up:?}"
    );
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
    let airborne_up = step(&mut r, jump);
    assert!(!r.scratch.ground.on_ground, "left the surface");
    assert!(
        airborne_up.is_none(),
        "an airborne tick clears the ridden-surface fact"
    );
    assert!(
        r.scratch.kinematics.vel.y < -400.0,
        "launched along +normal: {:?}",
        r.scratch.kinematics.vel
    );
}

#[test]
fn momentum_home_body_rides_ordinary_block_floors() {
    // A worn momentum body must land, run, and jump on plain solids — blocks are surfaces
    // (`Block::boundary_chain`), not just obstacles.
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

/// Falling out REPORTS, and does not relocate (ADR 0033).
///
///  and the new assertion is the one that earns its keep. "She dies where
/// she died" was a 300-line death beat in Mary-O — a pose pinned every frame,
/// re-armed against a respawn that had already happened — and the pin, being
/// outside the world, re-fired this very gate 192 times per death. Nothing moves
/// the body now, so dying in place is what falls out for free.
#[test]
fn momentum_home_body_flags_the_pit_and_is_left_where_it_fell() {
    // The chain ends mid-world; running off it drops the body past the
    // world bottom — the Q16 hazard/OOB parity gate must fire.
    let mut r = rig(chain_world());
    let mut run = ActorControlFrame::neutral();
    run.locomotion.x = 1.0;
    let mut saw_reset = false;
    let mut where_it_fell = r.scratch.kinematics.pos;
    for _ in 0..1800 {
        where_it_fell = r.scratch.kinematics.pos;
        step(&mut r, run);
        if r.frame_out.reset.is_some() {
            saw_reset = true;
            break;
        }
    }
    assert!(saw_reset, "fell out and the reset flagged");
    assert_ne!(
        r.scratch.kinematics.pos, r.world.spawn,
        "the movement phase must NOT teleport a body home on the frame it \
         reports a death — the respawn is an authored consequence now"
    );
    assert!(
        r.scratch.kinematics.pos.y > where_it_fell.y,
        "it is still out there, still falling: {:?} was {:?}",
        r.scratch.kinematics.pos,
        where_it_fell
    );
    assert!(
        matches!(
            r.model,
            MotionModel::SurfaceMomentum(MomentumMotion {
                state: ae::SurfaceMotion::Airborne,
                ..
            })
        ),
        "and it is airborne rather than still 'riding' the chain it left — \
         which the fall itself establishes, with no reset needed to fix it up"
    );
}

/// The skid fact fires exactly when a rider steers AGAINST fast travel — not
/// during ordinary running, not below skid speed, not airborne.
#[test]
fn surface_skidding_reads_opposing_input_against_fast_travel() {
    let riding = |v_t: f32| {
        let mut m = MomentumMotion::new(ae::MomentumParams::default());
        m.state = ae::SurfaceMotion::Riding {
            on: ae::SurfaceRef::Chain(0),
            s: 0.0,
            v_t,
        };
        MotionModel::SurfaceMomentum(m)
    };
    // Fast travel + opposing input = skid; aligned input or neutral is not.
    assert!(super::surface_skidding(&riding(600.0), -1.0));
    assert!(super::surface_skidding(&riding(-600.0), 1.0));
    assert!(!super::surface_skidding(&riding(600.0), 1.0));
    assert!(!super::surface_skidding(&riding(600.0), 0.0));
    // A walk-speed direction change is a step, not a skid.
    assert!(!super::surface_skidding(&riding(120.0), -1.0));
    // Airborne there is no tangent to fight.
    let airborne = MotionModel::SurfaceMomentum(MomentumMotion::new(ae::MomentumParams::default()));
    assert!(!super::surface_skidding(&airborne, -1.0));
}

// ── a hazard tile is DAMAGE, and damage asks whether the body can be hurt ────

/// A room whose floor has a spike strip in the middle of it.
fn spiked_world() -> ae::World {
    ae::World::new(
        "spiked",
        ae::Vec2::new(1200.0, 800.0),
        ae::Vec2::new(100.0, 500.0),
        vec![
            ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 560.0),
                ae::Vec2::new(1200.0, 40.0),
            ),
            ae::Block::hazard(
                "spikes",
                ae::Vec2::new(300.0, 520.0),
                ae::Vec2::new(200.0, 40.0),
            ),
        ],
    )
}

/// Run right until the body either reaches the spikes' far side or is reset.
/// Returns the reset the integration published, if any.
fn run_into_the_spikes(
    invulnerable: ambition_characters::actor::Invulnerability,
) -> Option<ae::ResetCause> {
    let mut r = rig(spiked_world());
    let mut run = ActorControlFrame::neutral();
    run.locomotion.x = 1.0;
    run.facing = 1.0;
    for _ in 0..240 {
        r.frame_out.reset = None;
        step_as(&mut r, run, invulnerable);
        if let Some(reset) = r.frame_out.reset {
            return Some(reset.cause);
        }
        if r.scratch.kinematics.pos.x > 560.0 {
            // Cleared the strip without being reset.
            return None;
        }
    }
    None
}

#[test]
fn a_body_that_cannot_be_hurt_runs_straight_over_a_hazard_tile() {
    assert_eq!(
        run_into_the_spikes(ambition_characters::actor::Invulnerability::none()),
        Some(ae::ResetCause::Hazard),
        "an ordinary body is still reset by spikes — the control half of this probe"
    );
    let empowered = {
        let mut set = ambition_characters::actor::Invulnerability::none();
        set.set(ambition_characters::actor::Invulnerability::EMPOWERED, true);
        set
    };
    assert_eq!(
        run_into_the_spikes(empowered),
        None,
        "a body that cannot be hurt must run straight over them"
    );
}

///  and the void still wins, which is the line this change must not cross.
/// `resolve_body_hit` already states it for damage — *"you cannot be invulnerable
/// to the edge of the world"* — and the reset seam has to agree: leaving the
/// world is not damage, so no reason set exempts a body from it.
#[test]
fn no_reason_set_exempts_a_body_from_leaving_the_world() {
    let mut r = rig(ae::World::new(
        "void",
        ae::Vec2::new(600.0, 400.0),
        ae::Vec2::new(300.0, 100.0),
        Vec::new(),
    ));
    let empowered = {
        let mut set = ambition_characters::actor::Invulnerability::none();
        set.set(ambition_characters::actor::Invulnerability::EMPOWERED, true);
        set
    };
    let mut fell = None;
    for _ in 0..600 {
        r.frame_out.reset = None;
        step_as(&mut r, ActorControlFrame::neutral(), empowered);
        if let Some(reset) = r.frame_out.reset {
            fell = Some(reset.cause);
            break;
        }
    }
    assert_eq!(
        fell,
        Some(ae::ResetCause::LeftTheWorld),
        "an empowered body still falls out of the world"
    );
}
