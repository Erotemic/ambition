//! Participant driving authority, seat input, and body control acceptance.
//! Controllers publish `ActorControl`; common gates accept it before execution.

mod acceptance;
pub use acceptance::{BodyControlGate, blank_scripted_control_frames, drive_commanded_moves, gate_body_control};

pub mod authority;
pub mod components;
pub mod input_systems;
pub mod possession;
pub mod queries;
mod raw_publication;
mod participant_input;
pub use participant_input::{ControlledBrainTick, HeldControlFrame, tick_controlled_brains};

pub use authority::project_driving_participant;
pub use input_systems::{
    InputTimersAdvanced, InteractionInputBuffered, cleanup_timers_system, derive_slot_direction_gestures, interaction_input_system,
    tick_room_transition_cooldown,
};
pub use queries::{
    another_authority_publishes, body_driving_seat, controlled_frame_down, primary_player_entity,
    seat_frame_down, seat_frame_this_tick, shape_seat_frame,
};
pub use raw_publication::publish_seat_controls_when_nobody_else_does;

