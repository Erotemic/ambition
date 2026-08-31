//! Does anything still decay the developer HUD's preset flash?
//!
//! ⭐⭐ THE REGISTRATION IS THE CLAIM, NOT THE ARITHMETIC. `preset_flash` was
//! wound down by one line inside the actor kernel's `cleanup_timers_system`, and
//! that line was the only reason the simulation kernel held a
//! `ResMut<DeveloperRuntimeState>` at all (queue row D33). It moved to
//! `ambition_dev_tools::decay_developer_presentation_flash`, in the same
//! schedule — and the crate-local test can prove the subtraction but not that
//! anything RUNS it: `DevToolsSimPlugin`'s siblings need resources
//! `ambition_dev_tools` does not depend on, so its schedule cannot run in a bare
//! `App`, and bevy reports every system as `<Enable the debug feature to see the
//! name>` without a feature that crate will not turn on for a test.
//!
//! ⛔ A TIMER NOBODY DECAYS IS A HUD FLASH THAT NEVER CLEARS — visible on screen,
//! and nothing else in the tree would have caught it.

use ambition_app::{AmbitionSim, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::dev_tools::DeveloperRuntimeState;

fn harness() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("sandbox sim builds")
}

#[test]
fn the_developer_hud_flash_still_winds_down() {
    let mut sim = harness();

    sim.world_mut()
        .resource_mut::<DeveloperRuntimeState>()
        .preset_flash = 1.0;

    // ⛔ THE PREMISE. A harness that never advanced its schedule would satisfy a
    // "did not decay" assertion and every other one here.
    for _ in 0..30 {
        sim.step_frame(Default::default());
    }
    let after = sim.world().resource::<DeveloperRuntimeState>().preset_flash;
    assert!(
        after < 1.0,
        "the developer HUD flash did not wind down over 30 ticks, so nothing in \
         the shipped app runs `decay_developer_presentation_flash` since it left \
         the actor kernel's `cleanup_timers_system`: {after}"
    );

    // …and it reaches zero rather than running negative, which is what the HUD's
    // `preset_flash > 0.0` depends on to stop drawing.
    for _ in 0..600 {
        sim.step_frame(Default::default());
    }
    assert_eq!(
        sim.world().resource::<DeveloperRuntimeState>().preset_flash,
        0.0,
        "the flash ran past zero"
    );
}

/// Developer slow-motion still slows the world, now that the kernel stops
/// looking for it.
///
/// ⭐⭐ THE ASK MOVED, SO THE ANSWER HAS TO BE RE-PROVEN END TO END. Slow-motion
/// was rung 4 of the actor kernel's time-scale ladder, reading
/// `DeveloperRuntimeState` directly. It is now a `ClockScaleRequest` published by
/// `ambition_dev_tools::request_developer_slow_motion` and reduced by `min` in
/// `apply_clock_scale_requests` — three things that each have to be true, and a
/// unit test of any one of them would pass while the toggle did nothing.
///
/// ⛔ THE CONTROL IS THE POINT. The kernel writes `default` 1.0 every frame, so
/// an arm that only checked "the scale is 1.0 with slowmo off" would pass on a
/// world where the request never arrives.
#[test]
fn developer_slow_motion_still_reaches_the_clock() {
    use ambition_platformer2d::time::time_control::RequestedClockScale;

    // The control: the toggle off, the world at pace.
    let mut sim = harness();
    for _ in 0..10 {
        sim.step_frame(Default::default());
    }
    let at_pace = sim.world().resource::<RequestedClockScale>().sim_clock;
    assert_eq!(
        at_pace, 1.0,
        "the sandbox is not running at real-time pace with no slow-down asked \
         for, so the arm below cannot show that slow-motion caused anything"
    );

    // And the toggle on.
    let mut sim = harness();
    sim.world_mut()
        .resource_mut::<DeveloperRuntimeState>()
        .slowmo = true;
    for _ in 0..10 {
        sim.step_frame(Default::default());
    }
    let slowed = sim.world().resource::<RequestedClockScale>().sim_clock;
    assert!(
        slowed < 1.0,
        "developer slow-motion asked for nothing. Since it left the kernel's \
         time-scale ladder the ask is `ambition_dev_tools::\
         request_developer_slow_motion`, and either it is unregistered or its \
         request is not being granted: {slowed}"
    );
    assert_eq!(
        slowed,
        sim.world().resource::<DeveloperRuntimeState>().slowmo_scale,
        "the clock did not arrive at the scale the developer state authors — \
         something else is winning the `min`"
    );
}
