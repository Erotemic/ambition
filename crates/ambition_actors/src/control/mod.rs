//! **The local control seam** — device frame → slot → the body carrying that
//! slot's player brain.
//!
//! This is not player-centrism; it is the wire between a human and a body, and
//! naming it is most of what "player-ness is a brain and a slot, not a directory"
//! means. Read it in order:
//!
//! 1. [`components`] — the state. `LocalPlayer` (this slot's input is local),
//!    `PlayerInputFrame` (a body's own frame), `SlotGestures` /
//!    [`SlotInteractionState`] (a CONTROLLER's gestures, which follow it onto
//!    whatever body it drives).
//! 2. [`input_systems`] — the device layer: edge/timer derivation and gesture
//!    recognition off the raw `ControlFrame`.
//! 3. [`slots`] — the two bridges: device→slot, then slot→body.
//! 4. [`queries`] — slot-explicit player lookups, so a call site says whether it
//!    means "the primary player" or "every player".
//!
//! **Downstream of this module, nothing holds `Res<ControlFrame>`.** A body reads
//! its own `PlayerInputFrame`, or its brain's `ActorControl`
//! (`ambition_characters::actor::control` — the brain→body contract, the far end
//! of this same wire). `ambition_runtime/tests/control_frame_lint.rs` enforces it,
//! and its allowlist is almost exactly this module's contents.
//!
//! Extracted from `crate::avatar` in the S5/S6 fold (refactor-chain R6c): the
//! slot machinery was never player-only state, and keeping it under `player/`
//! was one of the reasons that module read as a universal dependency sink.

pub mod components;
pub mod input_systems;
pub mod queries;
pub mod slots;

pub use components::{
    LocalPlayer, PlayerInputFrame, PlayerSlot, SlotGestures, SlotInteractionState,
};
pub use input_systems::{cleanup_timers_system, input_timer_system, interaction_input_system};
pub use queries::{controlled_frame_down, primary_player_entity, sort_players_by_slot};
pub use slots::{populate_slot_controls, sync_local_player_input_frame};
