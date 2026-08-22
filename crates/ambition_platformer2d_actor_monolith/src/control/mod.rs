//! The local control seam — device frame → slot → the body carrying that
//! slot's player brain.
//!
//! This is not player-centrism; it is the wire between a human and a body, and
//! naming it is most of what "player-ness is a brain and a slot, not a directory"
//! means. Read it in order:
//!
//! 1. [`components`] — the state. `LocalPlayer` (this body's input source is
//!    local) plus `SlotGestures` / [`SlotInteractionState`] (a CONTROLLER's
//!    gestures, which follow it onto whatever body it drives).
//! 2. [`input_systems`] — the device layer: edge/timer derivation and gesture
//!    recognition off the raw `ControlFrame`.
//! 3. [`slots`] — the local-device → canonical-slot publication boundary.
//! 4. [`queries`] — slot-explicit player lookups, so a call site says whether it
//!    means "the primary player" or "every player".
//!
//! Downstream of this module, nothing holds `Res<ControlFrame>`. Controller
//! adapters read `SlotControls`; body mechanics read the brain's `ActorControl`
//! (`ambition_characters::actor::control` — the brain→body contract, the far end
//! of this same wire). The workspace `ControlFrame` policy enforces it,
//! and its allowlist is almost exactly this module's contents.
//!
//! Extracted from `crate::avatar` in the S5/S6 fold (refactor-chain R6c): the
//! slot machinery was never player-only state, and keeping it under `player/`
//! was one of the reasons that module read as a universal dependency sink.

pub mod acting;
pub mod authority;
pub mod components;
pub mod input_systems;
pub mod queries;
pub mod slots;

pub use acting::ActingParticipant;
pub use authority::{project_driving_participant, DrivingParticipant};
pub use components::{LocalPlayer, PlayerSlot};
pub use input_systems::{
    cleanup_timers_system, input_timer_system, interaction_input_system, InputTimersAdvanced,
    InteractionInputBuffered,
};
pub use queries::{
    another_authority_publishes, body_driving_seat, controlled_frame_down, primary_player_entity,
    seat_frame_down, seat_frame_this_tick, shape_seat_frame, sort_players_by_slot,
};
pub use slots::PrimarySlotInputCommit;
