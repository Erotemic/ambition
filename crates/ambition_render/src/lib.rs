//! Ambition's Bevy presentation layer — the sandbox's default renderer.
//!
//! Everything here is downstream of the sim: no module on the gameplay critical
//! path lives in this crate. It reads `ambition_actors` machinery (player
//! clusters, features, rooms, assets) and mirrors it into Bevy sprites / UI; it
//! never mutates the sim. The sim/render seam is now a CRATE boundary — the
//! `ambition_actors` lib cannot import this crate (enforced by
//! `architecture_boundaries`), so render changes never rebuild the machinery.
//!
//! Modules are migrated here incrementally from the old
//! `ambition_actors::presentation` umbrella; consumers (content, app) import
//! `ambition_render::*` directly.

pub mod cutscene;
/// The dialog-box overlay UI (was `ambition_actors::dialog::ui`). Render-only;
/// reads the reusable dialog state in `ambition_dialog`.
pub mod dialog_ui;
pub mod fx;
/// The in-world HUD overlay: health/mana bars, ability pips, banner text.
pub mod hud;
pub mod quality;
pub mod rendering;
pub mod screen_effects;
pub mod ui_fonts;
