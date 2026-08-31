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

mod a_dropped_item_falls;
mod a_game_governs_only_its_own_rooms;
mod a_hit_on_the_player_freezes_the_match;
mod a_ron_game_installs_no_ldtk_world;
mod a_save_remembers_where_you_left_things;
mod a_teleported_subject_does_not_get_chased;
mod actor_phase_split;
mod admiral_gun_sword;
mod app_local_catalog_composition;
mod asset_id_platform_parity;
mod author_teleport_blink;
mod authored_fighter_ladder;
mod blink_run_reachability;
mod boomerang_hits_both_legs;
mod boot_budget;
mod boss_contact_iframes;
mod boss_draw_cursor;
mod boss_lifecycle;
mod boss_motion_parity;
mod boss_possession_specials;
mod boss_sheet_wiring;
mod camera_names_its_view;
mod canonical_reconstitution;
mod carried_item_crosses_rooms;
mod causal_explains_the_real_app;
mod character_containment;
mod character_provider_namespace;
mod collision_invariant_oracle;
mod composes_through_the_sdk;
mod content_dormancy;
mod crouch_stability;
mod d71_transaction_census;
mod dash_stability;
mod death_restores_the_checkpoint;
mod declared_art_resolves;
mod desync_canary;
mod direct_and_shell_agree;
mod dive_drill_reachability;
mod door_entry;
mod door_with_the_touch_overlay;
mod duel_arena;
mod effect_quarantine;
mod enemy_attacks_player;
mod every_character_says_something;
mod experience_scope_ownership;
mod falling_sand_room;
mod fb6_shadow_fidelity;
mod fly_to_the_hall_of_characters;
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
mod input_stream_under_rollback;
mod isolated_persistence;
mod mary_o_hud_surround;
mod mary_o_lap_in_the_host;
mod movement_axis;
mod neighbor_prefetch_prepares_rooms;
mod no_character_resolves_art_by_an_ambiguous_root;
mod one_character_two_contexts;
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
mod projectile_speed_stays_under_the_swept_threshold;
mod quality_change_keeps_each_character;
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
mod rollback_match_activation;
mod rollback_provoked_actor;
mod rollback_room_transition;
mod rollback_schema_baseline;
mod the_developer_hud_flash_still_winds_down;
mod rollback_seat_devices;
mod room_boundary_unclaimed_views;
mod room_replay_seam;
mod room_spatial_integrity;
mod scheduler_perturbation;
mod scripted_gameplay;
mod shell_host_headless_entrypoint;
mod shell_host_lifecycle;
mod shell_host_rendered;
mod shell_host_startup;
mod shield_ring_probe;
mod sim_phase_pins;
mod sky_census;
mod smash_cpu_cognition;
mod smash_cpus_damage_each_other;
mod smash_in_the_host;
mod one_update_one_tick;
mod zero_duration_pump;
mod a_knockout_takes_you_home;
mod the_gameplay_gate_is_carried_by_the_set;
mod smash_ride;
mod the_trap_holds_her_under;
mod smash_roster_movesets;
mod starting_character_selection;
mod stocks;
mod symmetry_attunement;
mod the_engine_can_be_asked_questions;
mod the_engine_can_be_told_to_do_things;
mod the_engine_ships_its_own_effects;
mod twintrack_split_has_two_viewports;
mod two_fighters_author_a_grab;
mod two_persistence_authorities_for_one_item;
mod two_seats_two_items;
mod unified_body_movement;
mod unified_melee;
mod update_schedule_census;
mod versus_stage;
mod versus_through_the_sdk;
mod visible_composition_contract;
mod walking_into_a_loading_zone;
mod world_manifest_parameterization;

mod enemy_body_scale;
