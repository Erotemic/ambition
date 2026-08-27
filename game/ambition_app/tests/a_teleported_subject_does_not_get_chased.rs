#![cfg(feature = "rl_sim")]
//! ⛔⛔ THE FOLLOW CAMERA SNAPPED ONLY ON A ROOM CHANGE OR A BLINK.
//!
//! Jon, measured: *"the camera snaps ONLY on a room change or a blink, never on
//! 'the subject teleported'. A synthetic teleport inside one room panned it
//! 440px over about 40 ticks."*
//!
//! ⭐ THE MULTI-FIGHTER CAST CAMERA ALREADY HAD THE MISSING TERM —
//! `CastFraming::teleported` arms a settle allowance, added when a respawning
//! fighter collapsed the framing box by the width of the stage in one tick. It
//! is the single-subject FOLLOW path that had none, so the same discontinuity
//! was smoothed on one road and chased on the other.

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot;

fn camera_center(sim: &mut Platformer2dSimHarness) -> ae::Vec2 {
    let world = sim.world_mut();
    let view = ambition_platformer2d::sim_view::the_only_view(world);
    world
        .entity(view)
        .get::<ResolvedCameraSnapshot>()
        .expect("the view carries its resolved snapshot")
        .snapshot
        .center_world
}

fn player(sim: &mut Platformer2dSimHarness) -> (bevy::prelude::Entity, ae::Vec2) {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<(bevy::prelude::Entity, &BodyKinematics), PrimaryPlayerOnly>();
    let (entity, kin) = q.single(world).expect("primary player");
    (entity, kin.pos)
}

/// ⭐ THE ARMS STRADDLE THE PREDICATE, and the second one is what stops this
/// becoming "the camera teleports whenever the subject moves fast": a body that
/// covered the SAME GROUND under its own velocity is not placed, and must still
/// be eased toward.
#[test]
fn the_camera_snaps_to_a_subject_that_was_put_somewhere_and_eases_to_one_that_ran() {
    let settle = |sim: &mut Platformer2dSimHarness| {
        for _ in 0..90 {
            sim.step(AgentAction::default());
        }
    };

    // How far the camera still is from the subject a few frames after the jump.
    let lag_after_jump = |give_it_the_velocity: bool| -> f32 {
        let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
            .expect("sandbox sim builds");
        settle(&mut sim);

        let (body, from) = player(&mut sim);
        let to = from + ae::Vec2::new(440.0, 0.0);
        {
            let world = sim.world_mut();
            let mut kin = world.get_mut::<BodyKinematics>(body).expect("player body");
            kin.pos = to;
            if give_it_the_velocity {
                // Fast enough that 440px in one tick is ordinary travel — the
                // predicate's whole job is telling these two apart.
                kin.vel = ae::Vec2::new(440.0 * 60.0, 0.0);
            }
        }
        // A few frames: enough for a snap to have happened and far too few for
        // an ease to have covered 440px.
        // ONE step: a snap has happened by now and an ease has covered almost
        // none of 440px. Measuring later would let the body's own velocity
        // muddy the second arm.
        sim.step(AgentAction::default());
        let (_, now) = player(&mut sim);
        (camera_center(&mut sim).x - now.x).abs()
    };

    let placed = lag_after_jump(false);
    assert!(
        placed < 40.0,
        "the subject was PUT 440px away and the camera was still {placed:.0}px \
         behind it a frame later — the follow path is easing across a \
         teleport, which is the pan Jon measured"
    );

    let ran = lag_after_jump(true);
    assert!(
        ran > 100.0,
        "a body that covered the same ground UNDER ITS OWN VELOCITY was snapped \
         to ({ran:.0}px behind) — the term is firing on speed rather than on \
         placement, so the camera now teleports whenever the subject is fast"
    );
}
