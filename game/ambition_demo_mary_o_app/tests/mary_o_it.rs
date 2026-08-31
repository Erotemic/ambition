//! Aggregated integration-test binary for `ambition_demo_mary_o_app`.
//!
//! Rust links one integration-test binary per top-level `tests/*.rs`, and each
//! one here links the whole engine + Bevy graph. Collapsing these 11 targets
//! into a single binary removes that many link steps from every `cargo test` of
//! this crate. Each module keeps its own attributes, so which tests run is
//! unchanged; filter a former target with
//! `--test mary_o_it -- <module_name>`.
//!
//! `autotests = false` makes a forgotten `mod` line silently skip a whole
//! file — `mary_o_it_sync` is the guard that turns that into a failure.

mod enemy_quad_matches_its_box;
mod mary_o_it_sync;

mod course_playthrough;
mod only_run_and_jump;
mod death_reset_timing;
mod level_circuit;
mod level_lap;
mod one_placement_one_actor;
mod exit_3;
mod hud_placement;
mod level_1_acceptance;
mod ov1_draws_the_world;
mod painted_blocks_still_change_their_art;
mod presentation_schedule_handoff;
mod rollback_registration;
mod rollback_restore;
mod rollback_room_memory;
mod replay_rebuilds_the_snakes;
mod room_replay;
mod scripted_level_run;
mod shell_cycle;
mod the_transform_beat_reads_real_art;
mod two_rooms;
