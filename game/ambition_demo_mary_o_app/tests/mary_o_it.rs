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
mod the_sky_of_1_1_leads_to_the_warp_room;

mod autoplay;
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
mod snake_geometry_timeline;
mod room_replay;
mod scripted_level_run;
mod shell_cycle;
mod the_first_quasar_is_drawn;
mod the_transform_beat_reads_real_art;
mod two_rooms;
mod her_rules_ask_their_scope_once;
mod weaker_form_refusal;
mod an_attempt_and_its_powers;
mod crouch_clearance_probe;
mod morph_ball_is_not_hers;
mod the_player_is_built_whole;
mod one_tick_one_clock;
mod a_pose_says_whose_geometry_it_carries;
mod the_cast_is_its_pack;
mod body_rig_trial;
mod placed_actors_start_on_the_ground;
mod her_spark_leaves_her_hand;
mod a_pipe_ride_is_not_a_landing;
mod a_hold_is_its_holders;
mod a_void_visual_has_its_bodys_aura;
mod the_form_is_its_wearers;
mod a_snake_is_each_players;
mod a_mob_is_squashed_by_each_player;
mod a_second_player_has_her_powers;
mod eternal_cosmic;
mod the_switch_ends_the_castle;
mod the_laden_swallow_is_a_module;
mod the_void_lizard_is_a_module;
mod the_host_omits_only_what_the_game_does_not_use;
