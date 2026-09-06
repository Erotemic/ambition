//! Fold the match's declared rules over the world's baseline. (AE6)
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
//! themselves either rollback state or route lifecycle, so a rewind does not
//! need to restore it and must not try to.

use bevy::prelude::{Commands, Res};

/// Rebuild [`ResolvedCombatTuning`] from the declaration and the baseline.
///
/// Runs in `Platformer2dSimulationPhaseMonolith::WorldPrep`, which is before every reader: the damage
/// paths are in `PlayerSimulation`/`Combat`, and a resolution landing after them
/// would give the hit kernel last tick's rules on the tick a match opens — the
/// one tick where they differ.
pub fn project_combat_rules(
    mut commands: Commands,
    declared: Option<Res<ambition_combat::rules::DeclaredCombatRules>>,
    baseline_feel: Option<Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
    baseline_ff: Option<Res<ambition_combat::targeting::FriendlyFire>>,
) {
    // `Option` on both baselines for the same reason every other reader has it:
    // a minimal headless world that never stands up the tuning resources still
    // resolves, to the engine defaults rather than to nothing.
    let baseline_di = baseline_feel.map(|f| f.di_max_angle).unwrap_or_default();
    let baseline_ff = baseline_ff.map(|f| f.enabled).unwrap_or_default();
    commands.insert_resource(ambition_combat::rules::ResolvedCombatTuning::resolve(
        declared.map(|d| d.clone()),
        baseline_di,
        baseline_ff,
    ));
}
