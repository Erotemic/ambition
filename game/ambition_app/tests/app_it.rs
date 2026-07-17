//! Aggregated integration-test binary for `ambition_app`.
//!
//! Every former `tests/<name>.rs` top-level target is now a `mod <name>;`
//! submodule of this single binary. Rust links one integration-test binary
//! per top-level `tests/*.rs`; collapsing ~46 heavy (Bevy-linking) targets
//! into one removes ~45 link steps from every `cargo test` of this crate.
//! Each module keeps its own `#![cfg(feature = ...)]`, so feature gating and
//! the set of tests that run are unchanged. Filter a former target with
//! `--test app_it -- <module_name>` (e.g. `-- shell_host_startup`).
//!
//! Shared fixtures live in `mod common`, referenced as `crate::common::*`.

mod common;

// Guard: this aggregate must stay in sync with the tests/ directory (see the
// module for why `autotests = false` makes that a real hazard).
mod app_it_sync;

mod actor_phase_split;
mod app_local_catalog_composition;
mod blink_run_reachability;
mod boss_contact_iframes;
mod boss_draw_cursor;
mod boss_lifecycle;
mod boss_motion_parity;
mod boss_possession_specials;
mod boss_sheet_wiring;
mod collision_invariant_oracle;
mod crouch_stability;
mod dash_stability;
mod desync_canary;
mod dive_drill_reachability;
mod duel_arena;
mod enemy_attacks_player;
mod fuzz_random_walker;
mod gravity_room_reachability;
mod gravity_symmetry;
mod gravity_symmetry_room;
mod held_projectile_portal_transit;
mod input_stream_replay;
mod movement_axis;
mod player_clone_live;
mod player_phase_split;
mod player_pilots_mount_end_to_end;
mod player_robot_fights_player;
mod plugin_minimal_app;
mod portal_bridge_reachability;
mod portal_floor_bounce_no_fallthrough;
mod portal_lab_usable;
mod portal_reset_preserves_authored;
mod portal_translation_camera_continuity;
mod possession_end_to_end;
mod projectile_portal_transit;
mod replay_fixture_regression;
mod repro_walls;
mod room_spatial_integrity;
mod scripted_gameplay;
mod shell_host_headless_entrypoint;
mod shell_host_lifecycle;
mod shell_host_rendered;
mod shell_host_startup;
mod symmetry_attunement;
mod unified_body_movement;
mod unified_melee;
