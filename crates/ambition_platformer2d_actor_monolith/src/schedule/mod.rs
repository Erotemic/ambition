//! Schedule + input-frame vocabulary shared by the machinery lib, the
//! content crate, and the app crate.
//!
//! What stays here is the vocabulary other layers order against: the
//! `Platformer2dSimulationPhaseMonolith` schedule labels (+ the content/machinery slot sets)
//! and the device -> ControlFrame populate systems the menu/host layers anchor to.

mod input_systems;
mod schedule;

pub use input_systems::declare_gameplay_input_context;
#[cfg(feature = "input")]
pub use input_systems::declare_in_session_input_contexts;
#[cfg(feature = "input")]
pub use input_systems::{
    apply_menu_frame_to_cutscene_request, commit_seat_raw_frames, decode_menu_frame,
    freeze_local_seating_for_the_decided_match, mirror_primary_slot_to_control_frame,
    populate_menu_control_frame_from_actions, populate_seat_control_frames,
    populate_seat_menu_frames, publish_latched_slot_controls,
    publish_seat_controls_when_nobody_else_does, seat_input_participants_for_roster,
    spawn_primary_input_participant, sync_primary_recipe_from_settings,
    toggle_player_trail_emission_from_actions, MenuFrameConsume, MenuFrameCutsceneSkip,
    MenuFramePopulate, MenuNavConsume, SeatBurstTriggerState,
};
pub use schedule::configure_platformer2d_simulation_phases;
