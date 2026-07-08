//! Ambition's Bevy presentation layer — the sandbox's default renderer.
//!
//! Everything here is downstream of the sim: no module on the gameplay critical
//! path lives in this crate. It reads lower read-model crates (`ambition_sim_view`, `ambition_world`,
//! `ambition_sprite_sheet`, `ambition_platformer_primitives`) and mirrors them
//! into Bevy sprites / UI; it never mutates the sim. The sim/render seam is now
//! a CRATE boundary in both directions: render does not depend on
//! actor machinery, and actor machinery cannot import render (enforced by
//! `architecture_boundaries`).
//!
//! Modules are migrated here incrementally from the old
//! the old actor-side presentation umbrella; consumers (content, app) import
//! `ambition_render::*` directly.

pub mod cutscene;
/// The dialog-box overlay UI. Render-only; reads the reusable dialog state in
/// `ambition_dialog`.
pub mod dialog_ui;
pub mod fx;
/// The in-world HUD overlay: health/mana bars, ability pips, banner text.
pub mod hud;
pub mod quality;
pub mod rendering;
pub mod screen_effects;
pub mod ui_fonts;
