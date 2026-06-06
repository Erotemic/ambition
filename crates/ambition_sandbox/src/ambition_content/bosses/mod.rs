//! Named Ambition boss content registration.
//!
//! Owns the install of the default [`BossEncounterRegistry`] so the named
//! boss roster is constructed in one content-owned place. The general boss
//! machinery (profiles, specs, encounter registry/system, patterns) still
//! lives in `crate::boss_encounter`; this module owns the bespoke per-boss
//! *behavior* and *bark content* that names individual bosses:
//!
//! - [`gnu_ton`] — GNU-ton's bespoke arena gating (retreat-ladder reveal +
//!   floor-gate) and head-hurtbox regression coverage.
//! - [`banter`] — boss combat-banter lines + the idle-bark ticker
//!   ([`banter::install_boss_banter`] / [`banter::tick_boss_idle_barks`]),
//!   installed next to its dialogue registration.

use bevy::prelude::*;

pub mod banter;
pub mod gnu_ton;

pub use banter::{install_boss_banter, tick_boss_idle_barks};
pub use gnu_ton::gate_gnu_ton_arena_ladder;

/// Installs the default Ambition boss encounter registry resource.
pub struct AmbitionBossContentPlugin;

impl Plugin for AmbitionBossContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(crate::boss_encounter::BossEncounterRegistry::default());
    }
}
