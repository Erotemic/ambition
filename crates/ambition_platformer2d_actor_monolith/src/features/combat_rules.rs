//! Fold the active room's declared combat rules over the world's baseline. (AE6)
//!
//! The type this produces —
//! [`ResolvedCombatTuning`](ambition_combat::rules::ResolvedCombatTuning) — lives
//! in `ambition_combat`, because `on_hit`, `hitbox` and the damage paths are its
//! readers and a type must sit at or below its readers. The FOLD lives here,
//! one layer up, because its inputs do not both live down there: friendly fire
//! is combat's own baseline, and `di_max_angle` belongs to this crate's feel
//! tuning. Ownership travels down with the type; the projection happens where
//! the facts are visible.
//!
//! this is a DERIVED resource — rebuilt every tick from inputs that are
//! themselves either rollback state (the active room) or authored constants
//! (the declarations), so a rewind does not need to restore it and must not
//! try to.

use bevy::prelude::{Commands, Res};

/// Rebuild [`ResolvedCombatTuning`] from the active room's declared rules and
/// the baseline.
///
/// Runs in `Platformer2dSimulationPhaseMonolith::WorldPrep`, which is before every reader: the damage
/// paths are in `PlayerSimulation`/`Combat`, and a resolution landing after them
/// would give the hit kernel last tick's rules on the tick a match opens — the
/// one tick where they differ.
pub fn project_combat_rules(
    mut commands: Commands,
    declared: crate::session::governing_rules::GoverningRules<ambition_combat::rules::CombatRules>,
    baseline_feel: Option<Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
    baseline_ff: Option<Res<ambition_combat::targeting::FriendlyFire>>,
) {
    // `Option` on both baselines: a minimal headless world that never stands
    // up the tuning resources still resolves, and `resolve_over` says what an
    // absent baseline stands at.
    commands.insert_resource(ambition_combat::rules::ResolvedCombatTuning::resolve_over(
        declared.get(),
        baseline_feel.as_deref(),
        baseline_ff.as_deref(),
    ));
}
