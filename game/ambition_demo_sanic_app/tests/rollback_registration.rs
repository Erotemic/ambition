//! **The demo's mutable sim state is in the rollback contract (Phase 5b).**
//!
//! `BallDash`, `BallDashInput`, and `SanicActState` are gameplay truth that
//! lives in a `game/` crate, where the engine's own registration sweep cannot
//! name them. The content plugin registers them through the same seam engine
//! crates use; this pins that the registration actually reaches a composed
//! shell's registry, so the demo is GGRS-ready rather than silently outside
//! the envelope.

use ambition::runtime::rollback::RollbackRegistry;
use ambition_demo_sanic_app::build_demo_app;

#[test]
fn sanic_sim_state_is_in_the_rollback_contract() {
    let app = build_demo_app();
    let registry = app.world().resource::<RollbackRegistry>();
    let names: Vec<&str> = registry
        .descriptors()
        .map(|descriptor| descriptor.name.as_str())
        .collect();
    for expected in [
        "content.sanic_ball_dash",
        "content.sanic_ball_dash_input",
        "content.sanic_act_state",
    ] {
        assert!(
            names.contains(&expected),
            "`{expected}` is missing from the rollback registry — Sanic's sim \
             state fell out of the rollback contract"
        );
    }
}
