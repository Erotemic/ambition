//! The launch law. This is the only copy.
//!
//! `ambition_combat` resolves a hit's launch as
//! `base + growth × growth_base(base) × growth_scale × victim_damage / weight`,
//! folded with rage. The fighter brain must rank moves with the same law.
//!
//! The factors other than `victim_damage` multiply the percent term, not
//! `base`, so they move the crossover between two candidates instead of
//! scaling both alike. For two volumes `(b₁, g₁)` and `(b₂, g₂)` the crossover
//! is at
//!
//! ```text
//! d* = (b₂ − b₁) · weight / (growth_scale · growth_base · (g₁ − g₂))
//! ```
//!
//! Every factor is in it. Example: George Booul's forward smash is
//! `(185, 3.45)` and his up smash `(178, 6.28)`. At two points of victim
//! damage, the damage-only law prefers forward (191.90 against 190.56); the
//! full law against a Robot v2 (weight 0.85, percent scale 1.25) gives about
//! 195.15 against 196.47, so the up smash already wins.
//!
//! So there is one function, in the crate both sides can see.
//! `ambition_combat` owns the ruleset that resolves these numbers; this owns
//! the arithmetic.

/// How much steeper a heavy hit's percent curve is than a light one's.
///
/// The roster's authoring is homogeneous: across the bound roles,
/// `knockback_growth / knockback` is about 0.02, so a jab's percent curve and
/// a forward smash's differ only by `base`. This curve reads each volume's own
/// `base`, so it can separate moves.
///
/// It is not `victim_percent_knockback_scale` under another name. That knob
/// multiplies every move's percent term by one factor, the same as raising
/// every authored growth, so it does not separate moves.
///
/// The cost: wherever a curve is declared, an authored `knockback_growth` is
/// multiplied before it is used, so it no longer reads at face value.
///
/// It lives in the catalog and a ruleset declares it: `ambition_combat::rules`
/// chooses which curve a stage uses, and this defines what a curve does. The
/// fighter brain must use the same curve as the hit resolver and cannot see
/// `ambition_combat`.
///
/// No shipped ruleset declares one today. `ambition_demo_smash` sets
/// `growth_base: None` (its reasons are beside it), so every live world
/// resolves to [`Self::IDENTITY`]. The knob stays declarable, and the law must
/// evaluate it either way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrowthBaseCurve {
    /// The base knockback at which the steepening is exactly `1.0`. Pick a
    /// poke-sized base: every volume at or below it is unchanged.
    pub pivot: f32,
    /// How sharply the steepening climbs. `0.0` is identity at every pivot.
    pub exponent: f32,
    /// The most this curve may multiply any growth by.
    ///
    /// A ceiling is needed. The largest base on the roster (`bivalence`, 367.2)
    /// is a deliberate huge-base, low-growth finisher (`growth/base` 0.0093).
    /// Uncapped, it would get the largest multiplier, the opposite of its
    /// author's intent.
    pub ceiling: f32,
}

impl GrowthBaseCurve {
    /// The law as first written: a no-op at every base. Every undeclared world
    /// (every Ambition room) resolves to this.
    pub const IDENTITY: Self = Self {
        pivot: 1.0,
        exponent: 0.0,
        ceiling: f32::INFINITY,
    };

    /// What this curve multiplies a volume's authored growth by.
    ///
    /// It can only steepen. The `.max(1.0)` on the ratio and on the ceiling
    /// keep the factor at or above `1.0`, so a mistyped pivot cannot weaken a
    /// jab.
    ///
    /// It cannot revive a fixed-knockback move: [`launch_speed`] multiplies,
    /// so `growth == 0.0` stays `0.0`, and the law returns `base` early.
    /// `Some(0.0)` stays fixed at every curve.
    pub fn scale(self, base: f32) -> f32 {
        if self.exponent == 0.0 || self.pivot <= 0.0 || base <= 0.0 {
            return 1.0;
        }
        (base / self.pivot)
            .max(1.0)
            .powf(self.exponent)
            .min(self.ceiling.max(1.0))
    }
}

impl Default for GrowthBaseCurve {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Everything about the world and the victim that a launch depends on: the
/// half of [`launch_speed`] that is not the move.
///
/// A value, not five arguments, because the fighter brain evaluates a whole
/// kit against one opponent. Building it once per decision guarantees every
/// candidate is priced under the same conditions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchConditions {
    /// The victim's accumulated damage — the percent axis.
    pub victim_damage: i32,
    /// Knockback weight: heavier bodies launch less under the same growth
    /// term. `1.0` is the reference body; non-positive reads as `1.0`.
    pub victim_weight: f32,
    /// The ruleset's scale on the percent term alone: its
    /// `victim_percent_knockback_scale` folded with this move's staling. `1.0`
    /// is the law as first written.
    pub growth_scale: f32,
    /// The ruleset's per-`base` steepening.
    pub growth_base: GrowthBaseCurve,
    /// The ruleset's fallback growth, as a fraction of `base`: what a volume
    /// that authors `knockback_growth: None` grows by.
    ///
    /// `None` and `Some(0.0)` differ. `Some(0.0)` is fixed knockback. `None`
    /// means "the ruleset decides", which on a stage that declares
    /// `knockback_growth` is `base * this`. Only [`launch_speed`] collapses the
    /// two; a caller must not collapse them first.
    ///
    /// `0.0` is the undeclared world (the default of `DeclaredCombatRules`),
    /// so an Ambition room resolves `None` to a set launch.
    pub ruleset_growth: f32,
    /// The attacker's rage multiplier, already resolved. Applied to the whole
    /// launch, and declined by a set-knockback move (see [`launch_speed`]).
    pub rage: f32,
}

impl LaunchConditions {
    /// A fresh reference body under the undeclared ruleset: no percent, weight
    /// `1.0`, identity curve, no rage.
    ///
    /// This is a fixture value, not a default. A caller that does not know the
    /// conditions will rank a kit wrong; it should carry them (the perception
    /// view exists for that).
    pub const AGAINST_A_FRESH_REFERENCE_BODY: Self = Self {
        victim_damage: 0,
        victim_weight: 1.0,
        growth_scale: 1.0,
        growth_base: GrowthBaseCurve::IDENTITY,
        ruleset_growth: 0.0,
        rage: 1.0,
    };

    /// The same conditions with a different victim meter: the one axis a
    /// scorer sweeps.
    pub fn at_damage(self, victim_damage: i32) -> Self {
        Self {
            victim_damage,
            ..self
        }
    }
}

/// The launch speed one authored `(base, growth)` produces under
/// `conditions`.
///
/// Set knockback is the first branch, and it ignores percent, weight, ruleset
/// scale and rage. A volume or throw with zero growth launches the same at 0%
/// and at 150%. Rage is derived from percent, so applying it to a set launch
/// would add back the percent dependence the author removed.
///
/// The authored growth decides that, not the scaled one. `growth_base` and
/// `growth_scale` could reach zero and make a scaling move look set, which
/// would switch rage off for it.
///
/// This is where the two authoring roads collapse, which is why `growth` is an
/// `Option`. `Some(0.0)` is a fixed launch; `None` means "the ruleset
/// decides". They are equal only in a world that declares no growth. (On the
/// smash stage, `cellular_pulse`, base 140 with `None`, is 490px/s at 100%.)
///
/// The scale applies to the percent term only. `base` is the value against a
/// fresh opponent; a steeper percent curve increases the difference between a
/// fresh and a worn opponent. Scaling the sum would make a 0% poke stronger.
///
/// At 0% the term is exactly zero at every scale: `victim_damage` zeroes it
/// before the scale applies.
pub fn launch_speed(base: f32, growth: Option<f32>, conditions: LaunchConditions) -> f32 {
    let authored = growth.unwrap_or_else(|| base * conditions.ruleset_growth.max(0.0));
    if authored == 0.0 {
        return base.max(0.0);
    }
    let weight = if conditions.victim_weight > 0.0 {
        conditions.victim_weight
    } else {
        1.0
    };
    let grown = authored * conditions.growth_base.scale(base);
    let speed = base
        + grown * conditions.growth_scale.max(0.0) * conditions.victim_damage.max(0) as f32
            / weight;
    speed.max(0.0) * conditions.rage
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every condition set with the identity curve, so the arms below sweep
    /// the three scalars the law was first written around.
    fn plain(victim_damage: i32, victim_weight: f32, growth_scale: f32) -> LaunchConditions {
        LaunchConditions {
            victim_damage,
            victim_weight,
            growth_scale,
            growth_base: GrowthBaseCurve::IDENTITY,
            ruleset_growth: 0.0,
            rage: 1.0,
        }
    }

    /// growth == 0 returns the flat base for any damage and weight.
    #[test]
    fn a_zero_growth_launch_is_its_base_at_every_damage_and_weight() {
        for dmg in [0, 5, 50, 999] {
            for w in [0.5, 1.0, 4.0] {
                assert_eq!(launch_speed(7.5, Some(0.0), plain(dmg, w, 1.0)), 7.5);
            }
        }
    }

    /// Fixed knockback is fixed at every percent scale, and staleness does not
    /// weaken it.
    ///
    /// `Some(0.0)` growth is how a move ignores percent (jab-lock finishers and
    /// set-knockback throws use it). The test sweeps the scale as well as the
    /// damage: neither may move the answer off `base`.
    #[test]
    fn fixed_knockback_ignores_the_percent_scale_and_staleness_alike() {
        for dmg in [0, 5, 50, 700, 999] {
            for w in [0.5, 1.0, 4.0] {
                // A typical sweep, a fully stale knockback scale (`0.865`) and
                // an extreme value, so the claim covers the whole knob.
                for scale in [0.0, 0.865, 1.0, 1.5, 2.0, 2.5, 100.0] {
                    assert_eq!(
                        launch_speed(46.0, Some(0.0), plain(dmg, w, scale)),
                        46.0,
                        "a fixed-knockback move moved at {dmg}% / weight {w} / scale {scale}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_launch_grows_with_damage_and_divides_by_weight() {
        // base + growth * damage / weight.
        assert_eq!(launch_speed(10.0, Some(2.0), plain(0, 1.0, 1.0)), 10.0);
        assert_eq!(launch_speed(10.0, Some(2.0), plain(30, 1.0, 1.0)), 70.0);
        // Twice the weight -> half the growth contribution.
        assert_eq!(launch_speed(10.0, Some(2.0), plain(30, 2.0, 1.0)), 40.0);
        // Monotonic in accumulated damage.
        assert!(
            launch_speed(10.0, Some(2.0), plain(60, 1.0, 1.0))
                > launch_speed(10.0, Some(2.0), plain(30, 1.0, 1.0))
        );
        // Degenerate weight falls back to the reference body (never divides by 0).
        assert_eq!(launch_speed(10.0, Some(2.0), plain(10, 0.0, 1.0)), 30.0);
    }

    /// The percent scale moves the percent term and never the base.
    ///
    /// A 0% hit must be unchanged at any scale, while a high-percent hit moves
    /// by the full factor. Both ends in one test stop the knob from becoming a
    /// global launch buff.
    #[test]
    fn the_percent_scale_scales_the_percent_term_alone() {
        // At 0% the term is already zero, so no scale can reach it. An
        // equality, not a tolerance.
        for scale in [0.0, 1.0, 1.5, 2.0, 2.5] {
            assert_eq!(
                launch_speed(50.0, Some(1.05), plain(0, 1.0, scale)),
                50.0,
                "a 0% hit changed under percent scale {scale}"
            );
        }
        // At 100% on a reference body the whole percent term is `growth * 100`,
        // so doubling the scale doubles that term and leaves the base alone:
        // 50 + 1.05*100 = 155 fresh, 50 + 2*1.05*100 = 260 at 2x.
        assert_eq!(launch_speed(50.0, Some(1.05), plain(100, 1.0, 1.0)), 155.0);
        assert_eq!(launch_speed(50.0, Some(1.05), plain(100, 1.0, 2.0)), 260.0);
        // The gap is the percent term, not the launch: 260 is not 2x155. If it
        // were, the scale would have included the base.
        assert!(
            launch_speed(50.0, Some(1.05), plain(100, 1.0, 2.0))
                < 2.0 * launch_speed(50.0, Some(1.05), plain(100, 1.0, 1.0)),
            "the scale reached the base"
        );
        // A negative scale is clamped rather than inverting the launch.
        assert_eq!(launch_speed(50.0, Some(1.05), plain(700, 1.0, -3.0)), 50.0);
    }

    /// The growth-base curve can only steepen; it never weakens a poke.
    #[test]
    fn the_growth_base_curve_only_ever_steepens() {
        let curve = GrowthBaseCurve {
            pivot: 48.0,
            exponent: 0.25,
            ceiling: 1.40,
        };
        // A poke at or under the pivot is untouched.
        assert_eq!(curve.scale(20.0), 1.0);
        assert_eq!(curve.scale(48.0), 1.0);
        // A smash is steepened, and never past the ceiling.
        assert!(curve.scale(185.0) > 1.0 && curve.scale(185.0) <= 1.40);
        assert_eq!(curve.scale(10_000.0), 1.40);
        // And the identity is inert at every base.
        for base in [0.0, 1.0, 48.0, 367.2] {
            assert_eq!(GrowthBaseCurve::IDENTITY.scale(base), 1.0);
        }
    }

    /// George Booul's forward smash `(185, 3.45)` and up smash `(178, 6.28)`,
    /// which is the pair the review named.
    // `6.28` is George's authored growth, copied from
    // `george_booul_moveset.rs` so the fixture is the shipped move. Do not
    // round it to satisfy the lint; the authoring site has the same allow.
    #[allow(clippy::approx_constant)]
    fn george() -> (crate::LaunchEnvelope, crate::LaunchEnvelope) {
        (
            crate::LaunchEnvelope::default().with_volume(185.0, Some(3.45)),
            crate::LaunchEnvelope::default().with_volume(178.0, Some(6.28)),
        )
    }

    /// The smash stage's law by value: `SMASH_VICTIM_PERCENT_KNOCKBACK_SCALE`
    /// `1.25` against a Robot v2's `knockback_weight` of `0.85`.
    ///
    /// The curve is `IDENTITY` because the stage declares `growth_base: None`
    /// (see [`GrowthBaseCurve`]). Two of the other factors move the crossover
    /// on their own.
    ///
    /// Copied, not imported: this crate is below the demo and must not depend
    /// on it. If the demo retunes, the claim (the factors reorder a kit)
    /// still holds.
    fn the_smash_stage(victim_damage: i32) -> LaunchConditions {
        LaunchConditions {
            victim_damage,
            victim_weight: 0.85,
            growth_scale: 1.25,
            growth_base: GrowthBaseCurve::IDENTITY,
            // The stage's fallback growth, `SMASH_KNOCKBACK_GROWTH`. It does not
            // change the arms below (both smashes author their own growth); a
            // `0.0` here would be the undeclared world.
            ruleset_growth: 0.02,
            rage: 1.0,
        }
    }

    /// The factors other than victim damage move the crossover, so they can
    /// reorder a kit at the same victim damage.
    ///
    /// The first arm is the control: under the identity law, the damage-only
    /// answer is right, so the difference comes from the conditions.
    #[test]
    fn the_omitted_factors_reorder_a_kit_at_the_same_victim_damage() {
        let (forward, up) = george();
        let fresh = LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY.at_damage(2);
        assert!(
            forward.at(fresh) > up.at(fresh),
            "under the identity law the forward smash still wins at 2 damage \
             ({} vs {}) — if it does not, this arm's control is gone and the \
             flip below says nothing about the conditions",
            forward.at(fresh),
            up.at(fresh),
        );

        let declared = the_smash_stage(2);
        assert!(
            up.at(declared) > forward.at(declared),
            "at the SAME victim damage, under a declared ruleset and against a \
             body that is not the reference weight, the up smash has already \
             overtaken — forward {} vs up {}. A brain that ranks its finishers \
             by the identity law picks the wrong one here",
            forward.at(declared),
            up.at(declared),
        );
    }

    /// Set knockback ignores every condition; a shared law must not undo
    /// `Some(0.0)`.
    #[test]
    fn a_set_launch_is_the_same_under_every_condition() {
        let set = crate::LaunchEnvelope::default().with_volume(120.0, Some(0.0));
        assert_eq!(
            set.at(LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY),
            120.0
        );
        assert_eq!(set.at(the_smash_stage(150)), 120.0);
        // And rage does not reach it either: it is a percent-derived
        // multiplier, and a set launch has no percent in it.
        assert_eq!(
            set.at(LaunchConditions {
                rage: 1.8,
                ..the_smash_stage(150)
            }),
            120.0
        );
    }

    /// A zero-percent hit is its base at every scale: the victim's damage
    /// zeroes the term before any factor applies. Otherwise the percent knob
    /// would be a knockback buff.
    #[test]
    fn no_ruleset_factor_can_move_a_launch_at_zero_percent() {
        let jab = crate::LaunchEnvelope::default().with_volume(40.0, Some(0.8));
        assert_eq!(jab.at(the_smash_stage(0)), 40.0);
        assert_eq!(
            jab.at(LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY),
            40.0
        );
    }
}
