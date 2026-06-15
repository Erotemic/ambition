//! Ambition's Bevy presentation layer — the sandbox's default renderer.
//!
//! Everything here is downstream of the sim: no module on the gameplay critical
//! path lives in this crate. It reads `ambition_sandbox` machinery (player
//! clusters, features, rooms, assets) and mirrors it into Bevy sprites / UI; it
//! never mutates the sim. The sim/render seam is now a CRATE boundary — the
//! `ambition_sandbox` lib cannot import this crate (enforced by
//! `architecture_boundaries`), so render changes never rebuild the machinery.
//!
//! Modules are migrated here incrementally from the old
//! `ambition_sandbox::presentation` umbrella; consumers (content, app) import
//! `ambition_render::*` directly.

pub mod fx;
/// The in-world HUD overlay: health/mana bars, ability pips, banner text.
pub mod hud;
