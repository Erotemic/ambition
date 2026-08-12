#![cfg(feature = "rl_sim")]
//! **A LANDED HIT SHAKES THE SCREEN, AND THE SEAM THAT MAKES IT SO IS WIRED.**
//!
//! ⛔⛔ `CameraShakeState::kick` had exactly TWO production call sites in the
//! whole workspace — a boss phase change and a hard-fall landing — so a smash
//! that sent a fighter to the blast zone and a jab moved the camera identically
//! (campaign P4.37, audited 2026-08-12). Hitlag alone carried the entire
//! difference between a strong hit and a weak poke.
//!
//! ⚠ **the law is unit-tested in `camera_ease`; this is the other half.** A pure
//! function nobody calls shakes nothing, which is exactly the failure D106 was
//! about one layer up — a mechanism present, correct, and disconnected, with a
//! test saying yes. This drives the real sim and asks the camera.

use ambition_app::AmbitionSim;
use ambition_app::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::platformer::camera_ease::CameraShakeState;

fn sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("hall_of_characters"),
    )
    .expect("the sim harness builds in the Hall")
}

/// Arm the primary player's hitstop as a landed hit of `scale` × the ROUTE's
/// reference would, step one frame, and report the shake the camera ended with.
///
/// ⚠ the reference is read from the running app rather than restated: a route
/// that retunes its hitlag retunes what counts as a hard hit, and a literal here
/// would be a second number agreeing with the first by coincidence.
fn shake_after_a_connect_of(scale: f32) -> f32 {
    let mut sim = sim();
    // The player body is staged by room construction, not by `build`.
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }

    let world = sim.world_mut();
    let reference = world
        .get_resource::<ambition_platformer2d::actors::time::feel::Platformer2dFeelTuningMonolith>()
        .expect("the composed sim installs the monolith's feel tuning")
        .hitlag_time;

    let mut players = world.query_filtered::<
        &mut ambition_platformer2d::characters::actor::BodyCombat,
        ambition_platformer2d::actors::actor::PrimaryPlayerOnly,
    >();
    let mut armed = 0usize;
    for mut combat in players.iter_mut(world) {
        combat.hitstop_timer = reference * scale;
        armed += 1;
    }
    assert_eq!(
        armed, 1,
        "the fixture must find exactly one primary player to hit; {armed} is a \
         harness that has stopped measuring the shipped body"
    );

    // Clear whatever the boot frames left, so what is read below is this hit's.
    sim.world_mut()
        .resource_mut::<CameraShakeState>()
        .amplitude_px = 0.0;
    sim.step(AgentAction::default());
    sim.world_mut().resource::<CameraShakeState>().amplitude_px
}

/// **The strong hit moves the camera and the standard one does not** — both
/// terms, because either alone is satisfiable by a broken seam.
///
/// ⭐ a camera that shakes on EVERYTHING passes the first assertion; one that is
/// simply disconnected passes the second. The pair is the claim.
#[test]
fn a_hard_connect_shakes_the_camera_and_a_standard_one_does_not() {
    let hard = shake_after_a_connect_of(4.0);
    assert!(
        hard > 0.0,
        "the hardest connect the hitlag band allows left the camera perfectly \
         still — the shake law is computed and nobody kicks with it"
    );

    let standard = shake_after_a_connect_of(1.0);
    assert_eq!(
        standard, 0.0,
        "a reference-strength connect shook the camera, so the dead zone is not \
         being applied and every jab now rattles the screen"
    );
}
