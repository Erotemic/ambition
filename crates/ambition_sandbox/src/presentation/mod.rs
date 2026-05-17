//! Presentation-side systems: VFX, cutscene playback, sprite atlases,
//! parallax backgrounds, the Bevy rendering glue, and UI font setup.
//!
//! Everything here is downstream of the sim — no module in this umbrella
//! should be on the gameplay critical path. The seam between sim and
//! presentation is the long-term shape we want; grouping these together
//! makes the future `ambition` framework crate extraction mechanical.

pub mod character_sprites;
pub mod cutscene;
pub mod fx;
pub mod parallax;
pub mod rendering;
pub mod ui_fonts;
