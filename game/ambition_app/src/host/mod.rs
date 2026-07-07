//! Host-platform integration: per-OS plugin selection (desktop, android,
//! …) and window/display-mode controls.
//!
//! Lives outside `app/` because these modules describe what runs the
//! Bevy app rather than how the schedule is wired. (The touch / mobile input
//! adapter moved OUT to the sibling `ambition_touch_input` crate — app-thinness:
//! reusable engine input, not host glue.)

#[cfg(feature = "frame_pacing")]
pub mod framepace;
pub mod platform;
// `windowing` (display-mode vocabulary) stays in the machinery lib
// (`ambition_actors::host::windowing`) — the settings model reads it.
pub use ambition_actors::host::windowing;
