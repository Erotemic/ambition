//! Local control seam from device input to the body driven by a participant.
//!
//! This module owns local-slot publication and gesture state. Downstream body
//! mechanics consume `ActorControl`; they do not read the raw `ControlFrame`.
//! Player identity is not a simulation path: driving authority selects the body,
//! and the same body mechanics apply regardless of controller kind.

pub mod acting;
pub mod authority;
pub mod components;
pub mod input_systems;
pub mod queries;
pub mod slots;

pub use acting::ActingParticipant;
pub use ambition_characters::control::DrivingParticipant;
pub use authority::project_driving_participant;
// ⛔ RE-EXPORTED FROM THE FLOOR CRATE, NOT DEFINED HERE. `LocalPlayer` moved to
// `shared_tangle::markers` beside `ControlledSubject` and `PrimaryPlayer`, which
// is where a zero-field content-free marker belongs — it was filed next to the
// control systems that first read it, and that placement was the whole of the
// `avatar -> control` edge in the kernel's cyclic component.
pub use ambition_platformer2d_shared_tangle::markers::LocalPlayer;
pub use input_systems::{
    cleanup_timers_system, derive_slot_direction_gestures, interaction_input_system,
    tick_home_body_reaction_timers, tick_room_transition_cooldown, InputTimersAdvanced,
    InteractionInputBuffered,
};
pub use queries::{
    another_authority_publishes, body_driving_seat, controlled_frame_down, primary_player_entity,
    seat_frame_down, seat_frame_this_tick, shape_seat_frame, sort_players_by_slot,
};
pub use slots::PrimarySlotInputCommit;

