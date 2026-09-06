//! Ambition's Bevy presentation layer — the sandbox's default renderer.
//!
//! Everything here is downstream of the sim: no module on the gameplay critical
//! path lives in this crate. It reads lower read-model crates (`ambition_sim_view`, `ambition_platformer2d_world`,
//! `ambition_sprite_sheet`, `ambition_platformer2d_shared_tangle`) and mirrors them
//! into Bevy sprites / UI; it never mutates the sim. The sim/render seam is now
//! a CRATE boundary in both directions: render does not depend on
//! actor machinery, and actor machinery cannot import render (enforced by
//! `architecture_boundaries`).

pub mod asset_census;
#[cfg(feature = "capture")]
pub mod capture;
pub mod cutscene;
/// Provider-selectable dialogue presentation: one shared lifecycle / ordering
/// seam plus a plain opt-in default renderer over `ambition_sim_view::DialogView`.
pub mod dialog_ui;
pub mod fx;
pub mod gameplay_surround;
/// The in-world HUD overlay: health/mana bars, ability pips, banner text.
pub mod hud;
/// The presentation face a demo can add (oracle-violation OV1). See its module docs.
pub mod platformer_presentation;
pub mod quality;
pub mod reading_layout;
pub mod rendering;
/// Profiling-only presentation census: cameras/views, offscreen targets, portal
/// capture rigs, the draw population, and Bevy's render-pass diagnostics.
pub mod runtime_census;
pub mod screen_effects;
pub mod ui_fonts;
