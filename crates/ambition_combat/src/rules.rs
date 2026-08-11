//! **The combat rules a match plays under — resolved, not borrowed.** (AE6)
//!
//! The versus stage used to switch its rules on by WRITING the world's globals
//! (`Platformer2dFeelTuningMonolith::di_max_angle`, `FriendlyFire::enabled`) on route entry
//! and putting them back on exit. Saving and restoring made that correct, but
//! correct *by discipline*: any other writer during the match wins silently, a
//! crash between entry and exit leaves the borrow outstanding, and "restore"
//! was one refactor away from meaning "reset to the engine default" — which it
//! did mean, for a while, and no test could tell because every test started from
//! the default.
//!
//! The shape was the bug, not the bookkeeping. A route mutating global tuning
//! and undoing it afterwards is a lifecycle borrowing an authority it does not
//! own. So the match DECLARES its rules ([`DeclaredCombatRules`]), a projection
//! folds them over the world's baseline every tick, and combat reads the result
//! ([`ResolvedCombatTuning`]). Nothing is written back, so there is nothing to
//! restore and no window in which the restore has not happened yet: removing the
//! declaration IS the exit.
//!
//! ## Why the type is here and the projection is not
//!
//! `ResolvedCombatTuning` has to live at or below `ambition_combat`, because
//! `on_hit`, `hitbox` and `targeting` are its readers. Its INPUTS do not both
//! live there — `di_max_angle` belongs to `ambition_platformer2d_actor_monolith`' feel tuning — so
//! the projection system lives in `ambition_platformer2d_actor_monolith`, one layer up, where both
//! inputs are visible. Ownership travels down with the type; the fold happens
//! where the facts are.

use bevy::prelude::Resource;

/// **What a match asks for.** Present means a match (or any other owner of a
/// combat lifecycle) has declared rules; absent means the world's baseline
/// stands on its own.
///
/// Deliberately not `Option` fields: a rule a match does not care about is the
/// baseline's, and expressing that as "declare the baseline's value" would make
/// the declaration a snapshot of the world at declaration time — which is the
/// borrow again, wearing a different hat. A declarer that wants the world's DI
/// omits the whole resource, or reads it and re-declares deliberately.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct DeclaredCombatRules {
    /// **Which shell experience declared these rules.**
    ///
    /// ⛔ required, not optional, and it is a LIFECYCLE field rather than a
    /// label. Two stages declare combat rules — the versus route and the smash
    /// demo — and each gives its declaration back when its experience leaves. A
    /// giveback that removed the resource BY TYPE would delete the other stage's
    /// live rules on the way out, which is the "whichever left first deleted the
    /// other's match" bug this repo has already paid for twice (the participant
    /// roster, then the prepared match). Naming the declarer is what lets the
    /// release ask *is this mine* instead of assuming it.
    pub declared_by: String,
    /// How far a launched body may steer its own trajectory (CM2). `0.0`
    /// disables directional influence entirely, which is Ambition's PvE answer.
    pub di_max_angle: f32,
    /// **How much a launch GROWS with the victim's accumulated damage**, as a
    /// fraction of the move's own base launch speed per point of damage.
    ///
    /// `0.01` means *a hit doubles its launch at 100 damage* — the platform
    /// fighter's whole loop, where a fresh opponent is unlaunchable and a
    /// hundred-percent one dies to the same jab. `0.0` is flat knockback, which
    /// is Ambition's PvE answer and the engine baseline.
    ///
    /// ⭐ **a RULESET fact, not a per-move one, and that is the point.** A move
    /// may still author its own `knockback_growth` on a hit volume and that wins
    /// outright; this is what a stage says when its moves author none. Without
    /// it, every prefab-derived swing in the game carries `knockback_growth: 0.0` — so
    /// Smash's duelists launched identically at 0% and 150%, which is exactly
    /// what Jon reported as *"there does not seem to be any knockback"*.
    ///
    /// ⚠ it scales the move's BASE launch rather than being an absolute
    /// px/s-per-point, so a jab grows less than a smash out of one number. That
    /// is the property a per-move table would otherwise have to restate for
    /// every move.
    pub knockback_growth: f32,
    /// Whether same-faction bodies damage each other.
    ///
    /// ⚠ a match with declared TEAMS should leave this `false`. `MatchTeam`
    /// outranks faction for "may this land", and switching global friendly fire
    /// on to let two humans hit each other also makes teammates hittable — the
    /// 2v2 bug this seam exists to stop recurring.
    pub friendly_fire: bool,
}

/// **The rules combat actually reads this tick.**
///
/// Derived every tick from [`DeclaredCombatRules`] folded over the world's
/// baseline. A reader must never consult the baseline resources directly: that
/// is how a stage's rules and the world's rules got to disagree.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ResolvedCombatTuning {
    pub di_max_angle: f32,
    /// See [`DeclaredCombatRules::knockback_growth`]. `0.0` = flat knockback.
    pub knockback_growth: f32,
    pub friendly_fire: bool,
}

impl DeclaredCombatRules {
    /// Whether `owner` is the experience that declared these rules — the
    /// question `releasing_owned` asks on the way out.
    pub fn is_declared_by(&self, owner: &str) -> bool {
        self.declared_by == owner
    }
}

impl ResolvedCombatTuning {
    /// The fold: a declaration wins outright, the baseline stands otherwise.
    pub fn resolve(
        declared: Option<DeclaredCombatRules>,
        baseline_di: f32,
        baseline_ff: bool,
    ) -> Self {
        match declared {
            Some(rules) => Self {
                di_max_angle: rules.di_max_angle,
                knockback_growth: rules.knockback_growth,
                friendly_fire: rules.friendly_fire,
            },
            // ⚠ growth has NO world baseline to fall back to, unlike DI and
            // friendly fire: nothing outside a declaration authors it, so an
            // undeclared world is flat — which is every Ambition room today.
            None => Self {
                di_max_angle: baseline_di,
                knockback_growth: 0.0,
                friendly_fire: baseline_ff,
            },
        }
    }

    /// The friendly-fire toggle in the shape `can_damage` already takes, so the
    /// targeting rule keeps ONE signature whichever side supplies it.
    pub fn friendly_fire(self) -> crate::targeting::FriendlyFire {
        crate::targeting::FriendlyFire {
            enabled: self.friendly_fire,
        }
    }
}

impl Default for ResolvedCombatTuning {
    /// The engine baseline: no directional influence, no friendly fire. Matches
    /// `Platformer2dFeelTuningMonolith::default().di_max_angle` and
    /// `FriendlyFire::default().enabled`, and exists so a composition that never
    /// installs the projection still resolves rather than reading `None` as
    /// "zero rules".
    fn default() -> Self {
        Self {
            di_max_angle: 0.0,
            knockback_growth: 0.0,
            friendly_fire: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_undeclared_world_reads_its_own_baseline() {
        let resolved = ResolvedCombatTuning::resolve(None, 0.12, true);
        assert_eq!(resolved.di_max_angle, 0.12);
        assert!(resolved.friendly_fire);
    }

    /// The case the borrow could not express: a match's rules apply WITHOUT the
    /// baseline changing, so the world an experience authored is still there
    /// when the match ends.
    #[test]
    fn a_declaration_wins_without_disturbing_the_baseline() {
        let baseline_di = 0.12;
        let resolved = ResolvedCombatTuning::resolve(
            Some(DeclaredCombatRules {
                declared_by: "a_stage".to_string(),
                di_max_angle: 0.30,
                knockback_growth: 0.0,
                friendly_fire: false,
            }),
            baseline_di,
            true,
        );
        assert_eq!(resolved.di_max_angle, 0.30);
        assert!(!resolved.friendly_fire);
        // The baseline is a value this function READ; there is no path by which
        // it could have been written. That is the whole point of the seam, and
        // asserting it here is the cheapest place to say so.
        assert_eq!(baseline_di, 0.12);
    }

    /// Dropping the declaration is the exit. No restore step, so no window in
    /// which the restore has not happened yet.
    #[test]
    fn dropping_the_declaration_returns_to_the_baseline_with_no_restore_step() {
        let declared = Some(DeclaredCombatRules {
            declared_by: "a_stage".to_string(),
            di_max_angle: 0.30,
            knockback_growth: 0.0,
            friendly_fire: true,
        });
        assert_eq!(
            ResolvedCombatTuning::resolve(declared, 0.12, false).di_max_angle,
            0.30
        );
        assert_eq!(
            ResolvedCombatTuning::resolve(None, 0.12, false),
            ResolvedCombatTuning {
                di_max_angle: 0.12,
                knockback_growth: 0.0,
                friendly_fire: false,
            }
        );
    }
}
