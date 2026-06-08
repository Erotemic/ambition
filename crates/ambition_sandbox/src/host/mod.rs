//! Host-platform integration: per-OS plugin selection (desktop, android,
//! …), window/display-mode controls, and touch / mobile input adapters.
//!
//! Lives outside `app/` because these modules describe what runs the
//! Bevy app rather than how the schedule is wired.

#[cfg(feature = "frame_pacing")]
pub mod framepace;
pub mod mobile_input;
pub mod platform;
pub mod windowing;
