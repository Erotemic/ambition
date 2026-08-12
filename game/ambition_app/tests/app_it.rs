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
mod authored_fighter_ladder;
mod blink_run_reachability;
mod boot_budget;
mod boss_contact_iframes;
mod boss_draw_cursor;
mod boss_lifecycle;
mod boss_motion_parity;
mod boss_possession_specials;
mod boss_sheet_wiring;
mod causal_explains_the_real_app;
mod character_containment;
mod character_provider_namespace;
mod collision_invariant_oracle;
mod composes_through_the_sdk;
mod content_dormancy;
mod crouch_stability;
mod d71_transaction_census;
mod dash_stability;
mod declared_art_resolves;
mod desync_canary;
mod direct_and_shell_agree;
mod dive_drill_reachability;
mod door_entry;
mod duel_arena;
mod effect_quarantine;
mod enemy_attacks_player;
mod every_character_says_something;
mod experience_scope_ownership;
mod falling_sand_room;
mod fb6_shadow_fidelity;
mod fuzz_random_walker;
mod gameplay_presentation_ggrs;
mod gameplay_presentation_profiles;
mod gravity_room_reachability;
mod gravity_symmetry;
mod gravity_symmetry_room;
mod hall_barks;
mod hall_scale_spread;
mod hall_transition_cover;
mod held_projectile_portal_transit;
mod hit_shakes_the_camera;
mod input_stream_replay;
mod isolated_persistence;
mod mary_o_hud_surround;
mod movement_axis;
mod participant_input;
mod player_bubble_shield;
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
mod presentation_ui_lifecycle;
mod projectile_portal_transit;
mod registered_character_art_resolves;
mod rendered_identities_are_registered;
mod replay_fixture_regression;
mod repro_walls;
mod resolved_combat_tuning;
mod rollback_contact;
mod rollback_coverage;
mod rollback_exit_oracle;
mod rollback_full_reset;
mod rollback_lifecycle_reset;
mod rollback_provoked_actor;
mod rollback_match_activation;
mod rollback_room_transition;
mod rollback_schema_baseline;
mod rollback_seat_devices;
mod room_boundary_unclaimed_views;
mod room_replay_seam;
mod room_spatial_integrity;
mod scripted_gameplay;
mod shell_host_headless_entrypoint;
mod shell_host_lifecycle;
mod shell_host_rendered;
mod shell_host_startup;
mod shield_ring_probe;
mod sim_phase_pins;
mod smash_in_the_host;
mod smash_roster_movesets;
mod starting_character_selection;
mod stocks;
mod symmetry_attunement;
mod unified_body_movement;
mod unified_melee;
mod update_schedule_census;
mod versus_stage;
mod versus_through_the_sdk;
mod world_manifest_parameterization;

mod enemy_body_scale;
