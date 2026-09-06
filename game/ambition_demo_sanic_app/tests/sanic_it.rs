//! Aggregated integration-test binary for `ambition_demo_sanic_app`.
//!
//! Rust links one integration-test binary per top-level `tests/*.rs`, and each
//! one here links the whole engine + Bevy graph. Collapsing these 11 targets
//! into a single binary removes that many link steps from every `cargo test` of
//! this crate. Each module keeps its own attributes, so which tests run is
//! unchanged; filter a former target with
//! `--test sanic_it -- <module_name>`.
//!
//! `autotests = false` makes a forgotten `mod` line silently skip a whole
//! file — `sanic_it_sync` is the guard that turns that into a failure.

mod sanic_it_sync;

mod act_completion;
mod exit_3;
mod ov1_draws_the_world;
mod persona_architecture;
mod presentation_schedule_handoff;
mod rollback_registration;
mod rollback_restore;
mod room_replay;
mod session_isolation;
mod shell_cycle;
mod spikes_spend_rings;
mod standard_input_path;
