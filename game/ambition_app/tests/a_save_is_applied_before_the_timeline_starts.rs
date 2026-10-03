//! The durable-restore chain writes rollback state from `Update`, on the one
//! frame the save is applied (`SaveRestored` rises). A rewind does not replay
//! `Update`, so that frame must begin with no rollback session. The session
//! start gate (`durable_hydration_is_pending`) holds the first half; the check
//! `refuse_a_restore_over_a_live_timeline` (`rollback_ggrs/src/session.rs`)
//! holds the other half: a save that is applied again under a session that
//! stayed live is refused loudly.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim as _;
use ambition_app::{Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_app::AgentAction;
use ambition_platformer2d::actors::session::durable_horizon::SaveRestored;

/// Run a sync-test session until it is live, then, if `reload`, mark the save
/// unapplied as a teardown does and step on. Returns whether the session was
/// live before the reset, and the panic message of the steps after it, if any.
fn a_save_applied_again_under_a_live_session(reload: bool) -> (bool, Option<String>) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let live = sim
        .world()
        .contains_resource::<ambition_platformer2d::rollback::AmbitionGgrsSession>()
        && sim.world().resource::<SaveRestored>().0;
    if reload {
        sim.world_mut().resource_mut::<SaveRestored>().0 = false;
    }
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for _ in 0..10 {
            sim.step(AgentAction::default());
        }
    }));
    let message = outcome.err().map(|panic| {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default()
    });
    (live, message)
}

#[test]
fn a_save_applied_over_a_live_timeline_is_refused() {
    let (live, refused) = a_save_applied_again_under_a_live_session(true);
    assert!(live, "the premise: a session was live with the save applied");
    assert!(
        refused
            .as_deref()
            .is_some_and(|message| message.contains("applied over a live rollback timeline")),
        "the restore chain wrote over a live timeline and nothing refused it: {refused:?}"
    );
}

/// The control: the same session, stepped the same way with the save left
/// applied, runs on.
#[test]
fn a_live_timeline_with_the_save_applied_runs_on() {
    assert_eq!(a_save_applied_again_under_a_live_session(false), (true, None));
}
