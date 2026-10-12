//! The demo's mutable sim state is in the rollback contract (Phase 5b).
//!
//! `MaryOLevelState` and `FlagSequence` are gameplay truth that lives in a
//! `game/` crate, where the engine's own registration sweep cannot name them.
//! The content plugin registers them through the same seam engine crates use;
//! this pins that the registration actually reaches a composed shell's
//! registry, so the demo is GGRS-ready rather than silently outside the
//! envelope.

use ambition_platformer2d::rollback::RollbackRegistry;
use ambition_demo_mary_o_app::build_demo_app;

#[test]
fn mary_o_sim_state_is_in_the_rollback_contract() {
    let app = build_demo_app();
    let registry = app.world().resource::<RollbackRegistry>();
    let names: Vec<&str> = registry
        .descriptors()
        .map(|descriptor| descriptor.name.as_str())
        .collect();
    // The holds of a body and their state: the pipe ride, the Snake Plant's
    // place in its cycle, and whose holds a body's pose is under.
    for expected in [
        "content.mary_o_level_state",
        "content.mary_o_flag_sequence",
        "content.mary_o_pipe_transit",
        "content.mary_o_snake_plant_out",
        "mount.pose_owned_externally",
    ] {
        assert!(
            names.contains(&expected),
            "`{expected}` is missing from the rollback registry — Mary-O's sim \
             state fell out of the rollback contract"
        );
    }
}
