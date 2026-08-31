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

#[test]
fn the_developer_hud_flash_still_winds_down() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("sandbox sim builds");

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
