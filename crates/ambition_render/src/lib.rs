//! Ambition's Bevy presentation layer — the sandbox's default renderer.
//!
//! Everything here is downstream of the sim: no module on the gameplay critical
//! path lives in this crate. It reads `ambition_gameplay_core` machinery (player
//! clusters, features, rooms, assets) and mirrors it into Bevy sprites / UI; it
//! never mutates the sim. The sim/render seam is now a CRATE boundary — the
//! `ambition_gameplay_core` lib cannot import this crate (enforced by
//! `architecture_boundaries`), so render changes never rebuild the machinery.
//!
//! Modules are migrated here incrementally from the old
//! `ambition_gameplay_core::presentation` umbrella; consumers (content, app) import
//! `ambition_render::*` directly.

pub mod cutscene;
/// The dialog-box overlay UI (was `ambition_gameplay_core::dialog::ui`). Render-only;
/// reads the sim-side dialog state in `ambition_gameplay_core::dialog`.
pub mod dialog_ui;
pub mod fx;
/// The in-world HUD overlay: health/mana bars, ability pips, banner text.
pub mod hud;
pub mod rendering;
pub mod screen_effects;
pub mod ui_fonts;
