//! Presentation-side systems: VFX, cutscene playback, sprite atlases,
//! the Bevy rendering glue (including parallax), and UI font setup.
//!
//! Everything here is downstream of the sim — no module in this umbrella
//! should be on the gameplay critical path. The seam between sim and
//! presentation is the long-term shape we want; grouping these together
//! makes the future `ambition` framework crate extraction mechanical.

pub mod character_sprites;
pub mod cutscene;
pub mod fx;
/// The in-world HUD overlay (was the root `crate::hud_overlay`): health/mana
/// bars, ability pips, banner text. Presentation-only.
pub mod hud;
pub mod rendering;
pub mod screen_effects;
pub mod ui_fonts;
