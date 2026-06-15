//! Presentation-side systems: VFX, cutscene playback, sprite atlases,
//! the Bevy rendering glue (including parallax), and UI font setup.
//!
//! Everything here is downstream of the sim — no module in this umbrella
//! should be on the gameplay critical path. The seam between sim and
//! presentation is the long-term shape we want; grouping these together
//! makes the future `ambition` framework crate extraction mechanical.

pub mod cutscene;
pub mod rendering;
pub mod screen_effects;
pub mod ui_fonts;

// `fx` and `hud` were extracted to the `ambition_render` crate (the sim/render
// seam is now a crate boundary). Consumers import `ambition_render::{fx, hud}`.
