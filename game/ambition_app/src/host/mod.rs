//! Host-platform integration: per-OS plugin selection (desktop, android,
//! …) and window/display-mode controls.

#[cfg(feature = "frame_pacing")]
pub mod framepace;
pub mod platform;
pub mod render_recovery;
pub mod vsync;
// `windowing` (display-mode vocabulary) stays in the machinery lib
// (`ambition_platformer2d::actors::host::windowing`) — the settings model reads it.
pub use ambition_platformer2d::windowed_host as windowing;
