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
    /// **What a DOWNWARD hit does to the attacker.**
    ///
    /// ⭐⭐ **one move, two games** (Jon's redirect §16, ledger D82). The robot's
    /// down-air is one authored swing with one hitbox and one launch direction —
    /// and Ambition reads it as a POGO that bounces the attacker up off whatever
    /// it hit, while a platform fighter reads it as a SPIKE that drives the
    /// victim down and ends a stock offstage. Both readings are correct for
    /// their game, and neither belongs on the move.
    ///
    /// ⛔ **this is what stopped the protagonist carrying its own repertoire.**
    /// Attaching the canonical moveset to `player_robot_v3` turned
    /// `gravity_symmetry::pogo_bounces_away_from_gravity` red, and the wrong fix
    /// — authoring the robot a second, Ambition-only down-air — is the
    /// duplicate-moves outcome §16 explicitly forbids.
    ///
    /// ⚠ [`DownwardHitStyle::Pogo`] is the baseline BECAUSE it is today's
    /// behaviour: the effect is authored on the volume, so an undeclared world
    /// keeps firing it. A stage that wants spikes says so.
    pub downward_hit: DownwardHitStyle,
    /// **How long a body spiked out of the AIR cannot recover** (seconds).
    /// `0.0` — the baseline — is no meteor rule at all, which is what an
    /// exploration game wants: a downward hit there is a pogo or a shove, not a
    /// sentence.
    ///
    /// ⭐ **it belongs beside [`Self::downward_hit`] and nowhere else.** That
    /// field already decides whether this game reads a downward hit as a rebound
    /// or a SPIKE; how long the spiked body is silent is the same question one
    /// step further, and a game that declares `Spike` is exactly the game that
    /// wants to answer it. It briefly lived on the global feel tuning, where it
    /// had no way to be true for one experience and false for another.
    ///
    /// ⚠ what the genre calls "meteor cancel" is this window ENDING. There is no
    /// second verb to press.
    pub meteor_lock_time: f32,
    /// **RAGE — how much a body's OWN accumulated damage raises the knockback it
    /// DEALS**, per point. `0.0` (the baseline) is no rage at all.
    ///
    /// ⭐ the mirror of the percent mechanic and the reason a losing fighter is
    /// dangerous: a body already scales the knockback it TAKES by its own
    /// damage, so without this the fighter behind is punished twice — easier to
    /// launch and no harder to be launched by. Rage is what makes a comeback a
    /// thing the rules produce rather than a thing a player hopes for.
    ///
    /// ⚠ capped by [`Self::rage_max_scale`], because uncapped it turns the last
    /// stock into a coin flip.
    pub rage_per_damage: f32,
    /// The ceiling on [`Self::rage_per_damage`], as a multiplier. `1.0` = rage
    /// can never help, whatever the per-point rate says.
    pub rage_max_scale: f32,
    /// **STALING — how much of its strength a move loses per recent landing of
    /// the same move.** `0.0` (the baseline) is no staling.
    ///
    /// ⭐ it exists to stop one good answer being the ONLY answer. A fighter
    /// with a reliable kill move should have to vary, and a fighter who has
    /// worn one out should find the others suddenly worth throwing.
    ///
    /// ⚠ what is remembered is what LANDED, not what was thrown. A whiff did
    /// not answer anything.
    pub stale_step: f32,
    /// The floor [`Self::stale_step`] cannot take a move below, as a multiplier.
    /// `1.0` = staling can never weaken anything.
    pub stale_floor: f32,
    /// **CROUCH CANCEL — what a CROUCHING victim multiplies an incoming launch
    /// by.** `1.0` (the baseline) = crouching buys nothing but a shorter
    /// hurtbox.
    ///
    /// ⭐ it makes ducking a defensive READ rather than only a shape. ⚠ flat,
    /// with no percent threshold, because the threshold is emergent: 85% of a
    /// kill move is still a kill, so the option stops mattering by itself
    /// exactly where the genre stops using it.
    pub crouch_cancel_scale: f32,
    /// **JOSTLE — how hard two overlapping GROUNDED bodies push each other
    /// apart**, in px/s² of separating ACCELERATION at full overlap. `0.0` (the
    /// baseline) is off, and every body in every composition that does not
    /// declare it moves exactly as it did.
    ///
    /// ⭐⭐ **Jon, 2026-08-20, on why this exists and where it may live:** *"The
    /// no pushout rule I think is for portals, because I wanted them to be
    /// elegant. For bodies I think it might be ok. This isn't a hack, it is a
    /// game feel feature… This is something that games will want, so we should
    /// be able to express it. It should never be a mandatory part of the
    /// movement kernel though. It should be composable and not add to tech
    /// dept."*
    ///
    /// ⇒ so it is a DECLARED rule read by an opt-in body-vs-body pass
    /// (`features::ecs::jostle`), the fourth beside capture, footstool and the
    /// ledge trump — and NOT a term in `step_body`. A kernel that jostled
    /// unconditionally would make every body in every game pay for a
    /// platform-fighter rule, which is the shape the stale-move ring and the
    /// capture timeout both had to be moved OUT of.
    ///
    /// ⚠ **an acceleration, not a displacement, and the difference is
    /// reversibility.** The pass writes VELOCITY and the kernel integrates it
    /// like any other force, so position is never written by anything but the
    /// body's own motion and a rewind restores the same answer. It separates
    /// more slowly than a shove would, which in this genre reads as weight
    /// rather than as a bug.
    pub jostle_accel: f32,
    /// **How long a grab holds a body at 0%**, in seconds. Ultimate's 90 frames.
    pub grab_hold_base_seconds: f32,
    /// **How much longer per point of the CAPTIVE's damage.** Ultimate's 1.7
    /// frames per percent, so a fighter at 100% is held roughly twice as long.
    ///
    /// ⭐ it makes the grab a percent mechanic like everything else here: the
    /// body that is losing is the body a grab is worth throwing at.
    ///
    /// ⚠ read ONCE, when the hold begins — pummelling does not extend it, which
    /// is the genre's rule and the reason a pummel is a decision rather than a
    /// free extension of your own advantage.
    pub grab_hold_per_damage: f32,
    /// The ceiling on a hold however hurt the captive is. Also the answer to
    /// *"what ends a hold nobody ends"*: a captor who grabs and then does
    /// nothing must not hold a body for the rest of the match.
    pub grab_hold_max_seconds: f32,
    /// **What one mash press buys the captive**, in seconds off the hold.
    /// Ultimate's 14.4 frames.
    pub grab_mash_seconds: f32,
    /// Whether same-faction bodies damage each other.
    ///
    /// ⚠ a match with declared TEAMS should leave this `false`. `MatchTeam`
    /// outranks faction for "may this land", and switching global friendly fire
    /// on to let two humans hit each other also makes teammates hittable — the
    /// 2v2 bug this seam exists to stop recurring.
    pub friendly_fire: bool,
    /// **What a body that authored NO melee swings, in this experience.**
    ///
    /// ⭐⭐ **the third authority finally owning a scaffold that had been spelled
    /// twice** (2026-08-12). Two places answered this question independently:
    /// Smash's `smash_fighter_kit()` granted a seated fighter one swipe, and the
    /// PROVOCATION path handed a peaceful body a whole enemy archetype to get a
    /// melee out of it. Putting their numbers side by side is what settled it —
    /// `0.22/0.08/0.26`, 4 damage, 34 reach on the stage against
    /// `0.28/0.08/0.32`, 1 damage, 28 reach in exploration. Faster, harder,
    /// longer: a platform fighter's floor is not an exploration provoke, and the
    /// difference is a RULESET's to state.
    ///
    /// ⇒ same shape as `knockback_growth` one field up, and for the same reason:
    /// *what a stage says when the move authors nothing*. A character that
    /// states its own repertoire never reaches this.
    ///
    /// ⚠ `None` means *this experience does not say*, and the engine's own
    /// exploration default stands — which is every room in Ambition, and is why
    /// this is an `Option` rather than a value every declaration must invent.
    ///
    /// ⛔ its goal is DELETION, per character rather than per mode: when every
    /// body in an experience authors its own kit, that experience's declaration
    /// goes back to `None` and the scaffold has no adopters left.
    pub unarmed_melee: Option<ambition_characters::brain::MeleeActionSpec>,
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
    /// See [`DeclaredCombatRules::downward_hit`].
    pub downward_hit: DownwardHitStyle,
    /// See [`DeclaredCombatRules::meteor_lock_time`].
    pub meteor_lock_time: f32,
    /// See [`DeclaredCombatRules::rage_per_damage`].
    pub rage_per_damage: f32,
    /// See [`DeclaredCombatRules::rage_max_scale`].
    pub rage_max_scale: f32,
    /// See [`DeclaredCombatRules::stale_step`].
    pub stale_step: f32,
    /// See [`DeclaredCombatRules::stale_floor`].
    pub stale_floor: f32,
    /// See [`DeclaredCombatRules::crouch_cancel_scale`].
    pub crouch_cancel_scale: f32,
    /// See [`DeclaredCombatRules::jostle_accel`].
    pub jostle_accel: f32,
    /// See [`DeclaredCombatRules::grab_hold_base_seconds`].
    pub grab_hold_base_seconds: f32,
    /// See [`DeclaredCombatRules::grab_hold_per_damage`].
    pub grab_hold_per_damage: f32,
    /// See [`DeclaredCombatRules::grab_hold_max_seconds`].
    pub grab_hold_max_seconds: f32,
    /// See [`DeclaredCombatRules::grab_mash_seconds`].
    pub grab_mash_seconds: f32,
    pub friendly_fire: bool,
}

/// **How this game reads a downward attack.** See
/// [`DeclaredCombatRules::downward_hit`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DownwardHitStyle {
    /// The ATTACKER rebounds off what it hit — Hollow Knight's down-slash, and
    /// Ambition's. The default, because it is what an authored `pogo_bounce`
    /// effect already does and an undeclared world must not change.
    #[default]
    Pogo,
    /// The attacker keeps falling and the VICTIM is driven down — a platform
    /// fighter's spike, which is a kill offstage and would be nonsense if it
    /// also bounced you back to safety.
    Spike,
}

impl DeclaredCombatRules {
    /// Whether `owner` is the experience that declared these rules — the
    /// question `releasing_owned` asks on the way out.
    pub fn is_declared_by(&self, owner: &str) -> bool {
        self.declared_by == owner
    }
}

/// **The hold an UNDECLARED world gives**, in seconds — flat, whatever the
/// captive's damage. It is the `CAPTURE_HOLD_LIMIT_SECONDS` that used to live in
/// `smash_capture`, kept here as the baseline so a world that declares no combat
/// rules behaves exactly as it did rather than releasing everybody instantly.
pub const FLAT_GRAB_HOLD_SECONDS: f32 = 4.0;
/// **What one mash press buys in an undeclared world**, in seconds. Twenty
/// presses cleared the old fractional accumulator, and twenty of these clear
/// [`FLAT_GRAB_HOLD_SECONDS`].
pub const FLAT_GRAB_MASH_SECONDS: f32 = FLAT_GRAB_HOLD_SECONDS / 20.0;

impl ResolvedCombatTuning {
    /// **What a move is worth after `occurrences` recent landings of it**, as a
    /// multiplier, floored. `1.0` for a game that declares no staling and for a
    /// move nobody has thrown lately.
    pub fn stale_scale(self, occurrences: u32) -> f32 {
        if self.stale_step <= 0.0 || occurrences == 0 {
            return 1.0;
        }
        (1.0 - self.stale_step * occurrences as f32).max(self.stale_floor.clamp(0.0, 1.0))
    }

    /// **What an attacker's own damage multiplies its knockback by**, capped.
    /// `1.0` for a game that declares no rage, and for a fresh fighter in one
    /// that does.
    pub fn rage_scale(self, attacker_damage_taken: i32) -> f32 {
        if self.rage_per_damage <= 0.0 {
            return 1.0;
        }
        (1.0 + self.rage_per_damage * attacker_damage_taken.max(0) as f32)
            .min(self.rage_max_scale.max(1.0))
    }

    /// **How long a grab holds a body at this damage**, in seconds, capped.
    ///
    /// ⚠ the caller asks ONCE, as the hold begins, and stores the answer: this
    /// is the captive's percent AT THE GRAB, so damage dealt during the hold
    /// does not extend it.
    pub fn grab_hold_seconds(self, captive_damage_taken: i32) -> f32 {
        (self.grab_hold_base_seconds
            + self.grab_hold_per_damage * captive_damage_taken.max(0) as f32)
            .min(self.grab_hold_max_seconds)
    }

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
                downward_hit: rules.downward_hit,
                meteor_lock_time: rules.meteor_lock_time,
                rage_per_damage: rules.rage_per_damage,
                rage_max_scale: rules.rage_max_scale,
                stale_step: rules.stale_step,
                stale_floor: rules.stale_floor,
                crouch_cancel_scale: rules.crouch_cancel_scale,
                jostle_accel: rules.jostle_accel,
                grab_hold_base_seconds: rules.grab_hold_base_seconds,
                grab_hold_per_damage: rules.grab_hold_per_damage,
                grab_hold_max_seconds: rules.grab_hold_max_seconds,
                grab_mash_seconds: rules.grab_mash_seconds,
                friendly_fire: rules.friendly_fire,
            },
            // ⚠ growth has NO world baseline to fall back to, unlike DI and
            // friendly fire: nothing outside a declaration authors it, so an
            // undeclared world is flat — which is every Ambition room today.
            None => Self {
                di_max_angle: baseline_di,
                knockback_growth: 0.0,
                // ⚠ an undeclared world POGOS, because that is what the authored
                // effect already does. Anything else would change every Ambition
                // room to buy a Smash feature.
                downward_hit: DownwardHitStyle::Pogo,
                meteor_lock_time: 0.0,
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                stale_step: 0.0,
                stale_floor: 1.0,
                crouch_cancel_scale: 1.0,
                jostle_accel: 0.0,
                grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
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
            downward_hit: DownwardHitStyle::Pogo,
            meteor_lock_time: 0.0,
            rage_per_damage: 0.0,
            rage_max_scale: 1.0,
            stale_step: 0.0,
            stale_floor: 1.0,
            crouch_cancel_scale: 1.0,
            jostle_accel: 0.0,
            grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_hold_per_damage: 0.0,
            grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
            friendly_fire: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A GRAB HOLDS THE HURT FIGHTER LONGER, AND STILL LETS GO.**
    ///
    /// ⛔ three points and not one: the base alone would pass with the rate at
    /// zero, and the rate alone would pass with no ceiling — which is the shape
    /// that turns a grab at high percent into a body removed from the match.
    #[test]
    fn a_grab_holds_longer_the_more_damage_the_captive_has_taken() {
        let rules = ResolvedCombatTuning {
            grab_hold_base_seconds: 1.5,
            grab_hold_per_damage: 0.02,
            grab_hold_max_seconds: 3.0,
            ..Default::default()
        };
        assert_eq!(rules.grab_hold_seconds(0), 1.5);
        assert_eq!(rules.grab_hold_seconds(50), 2.5);
        assert_eq!(
            rules.grab_hold_seconds(999),
            3.0,
            "a hold at high percent outlived its own ceiling"
        );
        // ⚠ an undeclared world is FLAT, not zero: a rate of zero here would be
        // an instant release rather than "no percent mechanic".
        let flat = ResolvedCombatTuning::default();
        assert_eq!(flat.grab_hold_seconds(0), flat.grab_hold_seconds(300));
        assert!(flat.grab_hold_seconds(0) > 0.0);
    }

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
                downward_hit: DownwardHitStyle::Pogo,
                meteor_lock_time: 0.0,
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                stale_step: 0.0,
                stale_floor: 1.0,
                crouch_cancel_scale: 1.0,
                jostle_accel: 0.0,
                grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
                friendly_fire: false,
                unarmed_melee: None,
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
            downward_hit: DownwardHitStyle::Pogo,
            meteor_lock_time: 0.0,
            rage_per_damage: 0.0,
            rage_max_scale: 1.0,
            stale_step: 0.0,
            stale_floor: 1.0,
            crouch_cancel_scale: 1.0,
            jostle_accel: 0.0,
            grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_hold_per_damage: 0.0,
            grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
            friendly_fire: true,
            unarmed_melee: None,
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
                downward_hit: DownwardHitStyle::Pogo,
                meteor_lock_time: 0.0,
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                stale_step: 0.0,
                stale_floor: 1.0,
                crouch_cancel_scale: 1.0,
                jostle_accel: 0.0,
                grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
                friendly_fire: false,
            }
        );
    }
}

#[cfg(test)]
mod rage_tests {
    use super::*;

    fn raging(per_damage: f32, max_scale: f32) -> ResolvedCombatTuning {
        ResolvedCombatTuning {
            rage_per_damage: per_damage,
            rage_max_scale: max_scale,
            ..ResolvedCombatTuning::default()
        }
    }

    /// **A LOSING FIGHTER HITS HARDER, UP TO A CEILING.**
    ///
    /// ⛔ the reason rage exists at all: a body already scales the knockback it
    /// TAKES by its own damage, so without this the fighter behind is punished
    /// twice — easier to launch and no harder to launch with. And the cap is not
    /// decoration: uncapped, the last stock stops being a fight.
    #[test]
    fn rage_grows_with_the_attackers_own_damage_and_stops_at_the_cap() {
        let rules = raging(0.01, 1.5);
        assert_eq!(rules.rage_scale(0), 1.0, "a fresh fighter got a bonus");
        assert_eq!(rules.rage_scale(50), 1.5);
        assert!(rules.rage_scale(20) > 1.0 && rules.rage_scale(20) < 1.5);
        assert_eq!(
            rules.rage_scale(500),
            1.5,
            "rage ran past its ceiling, so the last stock is a coin flip"
        );
        assert_eq!(rules.rage_scale(-7), 1.0, "healed below zero paid a bonus");
    }

    /// **AND A GAME THAT DECLARES NO RAGE NEVER GETS ANY.**
    ///
    /// ⛔ the floor that keeps Ambition's PvE unchanged: the baseline declares
    /// `0.0`, and a rate of zero must be exactly `1.0` however hurt the attacker
    /// is — not `1.0 + 0.0 * n` rounded, but the early return.
    #[test]
    fn an_undeclared_world_has_no_rage() {
        let plain = ResolvedCombatTuning::default();
        assert_eq!(plain.rage_per_damage, 0.0);
        for damage in [0, 1, 50, 999] {
            assert_eq!(plain.rage_scale(damage), 1.0);
        }
        // ⚠ and a rate with a ceiling of 1.0 cannot help either, whatever the
        // rate says — the cap is the authority.
        assert_eq!(raging(0.05, 1.0).rage_scale(200), 1.0);
    }
}

#[cfg(test)]
mod stale_tests {
    use super::*;

    fn staling(step: f32, floor: f32) -> ResolvedCombatTuning {
        ResolvedCombatTuning {
            stale_step: step,
            stale_floor: floor,
            ..ResolvedCombatTuning::default()
        }
    }

    /// **A MOVE THROWN AGAIN AND AGAIN IS WORTH LESS, DOWN TO A FLOOR.**
    #[test]
    fn staling_falls_with_repetition_and_stops_at_the_floor() {
        let rules = staling(0.1, 0.5);
        assert_eq!(rules.stale_scale(0), 1.0, "a fresh move was already stale");
        assert!((rules.stale_scale(1) - 0.9).abs() < 1e-6);
        assert!((rules.stale_scale(3) - 0.7).abs() < 1e-6);
        assert_eq!(
            rules.stale_scale(9),
            0.5,
            "staling ran past its floor, so a worn move stops being a move"
        );
    }

    /// **AND AN UNDECLARED WORLD NEVER STALES ANYTHING.**
    #[test]
    fn an_undeclared_world_has_no_staling() {
        let plain = ResolvedCombatTuning::default();
        assert_eq!(plain.stale_step, 0.0);
        for n in [0, 1, 5, 9] {
            assert_eq!(plain.stale_scale(n), 1.0);
        }
        // A floor of 1.0 cannot weaken anything either, whatever the step says.
        assert_eq!(staling(0.2, 1.0).stale_scale(9), 1.0);
    }
}
