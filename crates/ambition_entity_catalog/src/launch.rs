//! **THE LAUNCH LAW, AND THE ONLY COPY OF IT.**
//!
//! ⛔⛤ **IT USED TO BE TWO, AND THE SECOND ONE RANKED THE FIGHTER BRAIN'S KILL
//! MOVES — REVIEWED 2026-09-20.** `ambition_combat` resolves a hit's launch as
//! `base + growth × growth_base(base) × growth_scale × victim_damage / weight`,
//! all folded with rage. [`crate::LaunchEnvelope::at`] evaluated
//! `base + growth × victim_damage` and argued that the omitted factors are
//! *"COMMON to every candidate one attacker weighs against one opponent, so
//! none of them can reorder a kit"*.
//!
//! **That argument is wrong, and it is wrong in the one place it matters.** The
//! omitted factors multiply the PERCENT TERM and not `base`, so they move the
//! CROSSOVER between two candidates rather than scaling both alike. For two
//! volumes `(b₁, g₁)` and `(b₂, g₂)` the crossover sits at
//!
//! ```text
//! d* = (b₂ − b₁) · weight / (growth_scale · growth_base · (g₁ − g₂))
//! ```
//!
//! — every omitted factor is in it. George Booul's forward smash is
//! `(185, 3.45)` and his up smash `(178, 6.28)`: at two points of victim damage
//! the brain preferred forward (191.90 against 190.56), while the runtime
//! against a Robot v2 (weight 0.85, the smash ruleset's percent scale 1.25)
//! gives about 195.15 against 196.47 — up smash has already overtaken.
//!
//! ⇒ One function, in the crate BOTH sides can see. `ambition_combat` owns the
//! ruleset that RESOLVES these numbers; it does not own the arithmetic that
//! spends them.

/// HOW MUCH STEEPER A HEAVY HIT'S PERCENT CURVE IS THAN A LIGHT ONE'S.
///
/// ⭐⭐ THIS EXISTS BECAUSE THE ROSTER'S AUTHORING IS HOMOGENEOUS, and that was
/// MEASURED rather than supposed. Across all 22 bound roles the ratio
/// `knockback_growth / knockback` sits in 0.019-0.021 — a jab's percent curve
/// and a forward smash's are the SAME curve, differing only by the constant
/// `base`. So a kill move is a jab times a number, and the thing a platform
/// fighter needs — "this one closes stocks and that one does not" — is not
/// expressible in what the roster currently authors.
///
/// ⛔ AND IT IS NOT `victim_percent_knockback_scale` UNDER ANOTHER NAME. That
/// knob is base-INDEPENDENT: raising it multiplies every move's percent term by
/// one factor, which is arithmetically identical to raising every authored
/// growth, and leaves the roster exactly as undifferentiated as it started.
/// This one reads the volume's own `base`, so it separates moves that knob
/// cannot.
///
/// ⛔ THE PRICE, STATED HERE RATHER THAN DISCOVERED LATER: wherever this is
/// declared, an authored `knockback_growth` stops reading as px/s-per-percent
/// at face value, because the number an author writes is multiplied before it
/// is spent.
///
/// ⚠ **IT LIVES IN THE CATALOG AND IS DECLARED BY A RULESET**, which is not a
/// contradiction: `ambition_combat::rules` owns *which* curve a stage runs
/// under, and this owns what a curve DOES. The fighter brain has to spend the
/// same one the hit resolver does, and it cannot see `ambition_combat`.
///
/// ⛔⛤ **AND NO SHIPPED RULESET DECLARES ONE TODAY — READ THAT BEFORE CITING
/// THIS AS THE SMASH STAGE'S LAW.** The smash demo declared
/// `48 / 0.25 / 1.40` and then RETIRED it (`ambition_demo_smash`'s
/// `growth_base: None`, with its reasons beside it): base knockback is not a
/// move's role, so the curve distorted a deliberate high-base/low-growth
/// shove, made an authored `knockback_growth` stop meaning what it says, and
/// never reached throws at all. The homogeneous authoring it was synthesising
/// around has since been authored explicitly. ⇒ Every live world resolves to
/// [`Self::IDENTITY`]; this stays because the knob is still declarable and the
/// law must be evaluable either way, not because anything spends it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrowthBaseCurve {
    /// The base knockback at which the steepening is exactly `1.0`. Pick a
    /// POKE-SIZED base: every volume at or below it is left untouched.
    pub pivot: f32,
    /// How sharply the steepening climbs. `0.0` is identity at every pivot.
    pub exponent: f32,
    /// The most this curve may multiply any growth by.
    ///
    /// ⭐ A CEILING IS NOT DECORATION HERE, and the outlier that motivates it
    /// was measured. Three bound pulses carry a base past the largest smash
    /// (185); the largest, `bivalence` at 367.2, authors `growth/base` =
    /// 0.0093 — less than half the roster's 0.019-0.021. It is deliberately a
    /// huge-base, low-growth finisher, i.e. the ONE move that most consciously
    /// departs from the homogeneity this curve keys on. Uncapped it would
    /// collect the largest multiplier on the roster, which is the opposite of
    /// what its author said about it.
    pub ceiling: f32,
}

impl GrowthBaseCurve {
    /// The law exactly as it was first written: a no-op at every base. Every
    /// undeclared world — which is every Ambition room — resolves to this.
    pub const IDENTITY: Self = Self {
        pivot: 1.0,
        exponent: 0.0,
        ceiling: f32::INFINITY,
    };

    /// What this curve multiplies a volume's authored growth by.
    ///
    /// ⛔ IT CAN ONLY EVER STEEPEN. The `.max(1.0)` on the ratio and the
    /// `.max(1.0)` on the ceiling each hold the factor at or above `1.0`, so a
    /// declared curve is never a second way to nerf pokes — nothing measured
    /// here asked for one, and a curve that could weaken a jab would otherwise
    /// be reachable by accident from a mistyped pivot.
    ///
    /// ⛔ AND IT CANNOT RESURRECT A FIXED-KNOCKBACK MOVE, because [`launch_speed`]
    /// multiplies: `growth == 0.0` times any factor is still `0.0`, and the law
    /// short-circuits on that to return `base`. `Some(0.0)` — the documented way
    /// to author a launch that ignores percent — stays exactly that at every
    /// curve.
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

/// Everything about the WORLD and the VICTIM that a launch depends on — the
/// half of [`launch_speed`] that is not the move.
///
/// ⭐ **A VALUE RATHER THAN FIVE ARGUMENTS, because the fighter brain evaluates
/// a whole kit against ONE opponent** and every candidate shares it. Building
/// it once per decision is also what makes it obvious that a candidate cannot
/// quietly be priced under different conditions than its rival.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchConditions {
    /// The victim's accumulated damage — the percent axis.
    pub victim_damage: i32,
    /// Knockback weight (CM1): heavier bodies launch less under the same growth
    /// term. `1.0` is the reference body; non-positive reads as `1.0`.
    pub victim_weight: f32,
    /// The ruleset's scale on the PERCENT TERM ALONE — its
    /// `victim_percent_knockback_scale` folded with this move's staling
    /// influence. `1.0` is the law as first written.
    pub growth_scale: f32,
    /// The ruleset's per-`base` steepening.
    pub growth_base: GrowthBaseCurve,
    /// **THE RULESET'S FALLBACK GROWTH, AS A FRACTION OF `base`** — what a
    /// volume that authors `knockback_growth: None` grows by.
    ///
    /// ⛔⛤ **`None` AND `Some(0.0)` ARE DIFFERENT AUTHORINGS AND THIS IS WHY
    /// THE DIFFERENCE HAS TO TRAVEL — REVIEWED 2026-09-20.** `Some(0.0)` is
    /// FIXED knockback: the author asked for a launch that ignores percent.
    /// `None` is *"the ruleset decides"*, and on a stage that declares
    /// `knockback_growth` it decides `base * this`. [`launch_speed`] collapses
    /// the two roads and it is the only place that does; a caller that
    /// collapses first has thrown the distinction away before the law sees it.
    ///
    /// ⚠ `0.0` IS THE UNDECLARED WORLD and is exactly what
    /// `DeclaredCombatRules`' own default carries — *"growth has NO world
    /// baseline to fall back to"* — so an Ambition room resolves `None` to a
    /// set launch, which is what it has always been.
    pub ruleset_growth: f32,
    /// The attacker's rage multiplier, already resolved. Applied to the whole
    /// launch, and DECLINED by a set-knockback move — see [`launch_speed`].
    pub rage: f32,
}

impl LaunchConditions {
    /// A fresh reference body under the undeclared ruleset: no percent, weight
    /// `1.0`, identity curve, no rage.
    ///
    /// ⚠ **THIS IS A FIXTURE'S ANSWER, NOT A DEFAULT TO REACH FOR.** A caller
    /// that does not know the conditions is a caller that will rank a kit
    /// wrong; it should carry them, which is what the perception view exists
    /// for.
    pub const AGAINST_A_FRESH_REFERENCE_BODY: Self = Self {
        victim_damage: 0,
        victim_weight: 1.0,
        growth_scale: 1.0,
        growth_base: GrowthBaseCurve::IDENTITY,
        ruleset_growth: 0.0,
        rage: 1.0,
    };

    /// The same conditions with a different victim meter — the one axis a
    /// scorer sweeps.
    pub fn at_damage(self, victim_damage: i32) -> Self {
        Self {
            victim_damage,
            ..self
        }
    }
}

/// **The launch speed one authored `(base, growth)` produces under `conditions`.**
///
/// ⛔ **SET KNOCKBACK IS THE FIRST BRANCH AND IT DECLINES EVERYTHING** —
/// percent, weight, ruleset scale and rage alike. A volume or throw written
/// with zero growth launches the same at 0% and at 150% BY CONSTRUCTION; it is
/// the genre's combo starter and its kill set-up, and rage is a percent-derived
/// multiplier, so folding it over a set launch reintroduces exactly the percent
/// dependence the author wrote `0.0` to remove. Ultimate excludes set knockback
/// from rage for the same reason.
///
/// ⚠ **THE AUTHORED GROWTH DECIDES THAT, NOT THE SCALED ONE.** `growth_base`
/// and `growth_scale` are knobs a ruleset turns; either could reach zero and
/// make a percent-scaling move momentarily look set, which would silently
/// switch rage off game-wide. The short-circuit reads what the author wrote.
///
/// ⛔⛤ **AND THE TWO AUTHORING ROADS ARE COLLAPSED HERE, WHICH IS THE WHOLE
/// REASON `growth` IS AN `Option` — REVIEWED 2026-09-20.** `Some(0.0)` is a
/// FIXED launch and `None` is *"the ruleset decides"*; they are the same
/// number only in a world that declares no growth. Both sides of the law used
/// to collapse them, and they collapsed them DIFFERENTLY: the hit resolver
/// read `None` as `base * ruleset_growth` and the fighter brain's envelope
/// read it as `0.0`. On the smash stage — `knockback_growth: 0.02`,
/// `victim_percent_knockback_scale: 1.25` — `cellular_pulse` (base 140,
/// `None`) resolves to **490px/s at 100%** for the hit resolver and **140** for
/// the brain, and the brain also called it a set launch and declined its rage.
/// ⇒ The collapse happens once, here, after the conditions are known.
///
/// ⭐⭐ **THE SCALE RIDES THE PERCENT TERM AND NOTHING ELSE, which is the whole
/// shape of this law.** `base` is what a move is worth against a FRESH
/// opponent, and a ruleset asking for a steeper percent curve is not asking for
/// a stronger jab — it is asking for the DIFFERENCE between a fresh opponent
/// and a worn one to be larger. Folding the scale over the sum instead would
/// inflate every launch in the game by the same factor and make a 0% poke
/// lethal, which is precisely what a percent mechanic exists not to do.
///
/// ⛔ AND 0% STILL CONTRIBUTES EXACTLY ZERO, AT EVERY SCALE: the
/// `victim_damage` factor zeroes the term before the scale can touch it, so no
/// value of `growth_scale` can move a 0% hit.
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

    /// ⛔ growth == 0 returns the flat base for ANY damage/weight — the
    /// byte-parity pin that keeps every un-authored volume unchanged.
    #[test]
    fn a_zero_growth_launch_is_its_base_at_every_damage_and_weight() {
        for dmg in [0, 5, 50, 999] {
            for w in [0.5, 1.0, 4.0] {
                assert_eq!(launch_speed(7.5, Some(0.0), plain(dmg, w, 1.0)), 7.5);
            }
        }
    }

    /// FIXED KNOCKBACK IS FIXED AT EVERY PERCENT SCALE, and a stale fixed move
    /// is not weakened either.
    ///
    /// ⛔ `Some(0.0)` growth is the documented way to author a move whose
    /// launch does not care about percent — jab-lock finishers and
    /// set-knockback throws depend on it. A percent-curve knob is exactly the
    /// kind of change that quietly turns those into percent-scaling moves, so
    /// the pin sweeps the SCALE as well as the damage: no value of either may
    /// move the answer off `base`.
    #[test]
    fn fixed_knockback_ignores_the_percent_scale_and_staleness_alike() {
        for dmg in [0, 5, 50, 700, 999] {
            for w in [0.5, 1.0, 4.0] {
                // the sweep range Jon asked for, plus a fully-stale knockback
                // scale (`0.865`) and an absurd value, so the claim is about
                // the whole knob and not about the value we happened to choose.
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

    /// THE PERCENT SCALE MOVES THE PERCENT TERM AND NEVER THE BASE.
    ///
    /// ⭐ The distinction is the entire design, and it is the one a "just
    /// multiply the knockback" fix gets wrong: a 0% hit must be untouched at
    /// ANY scale, while a high-percent hit moves by the full factor. Asserting
    /// both ends in one test is what stops the knob degenerating into a global
    /// launch buff.
    #[test]
    fn the_percent_scale_scales_the_percent_term_alone() {
        // At 0% the term is already zero, so no scale can reach it. An
        // EQUALITY, not a tolerance.
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
        // ⛔ AND THE GAP IS THE PERCENT TERM, NOT THE LAUNCH: 260 is not 2x155.
        // If it ever were, the scale would have swallowed the base too.
        assert!(
            launch_speed(50.0, Some(1.05), plain(100, 1.0, 2.0))
                < 2.0 * launch_speed(50.0, Some(1.05), plain(100, 1.0, 1.0)),
            "the scale reached the base"
        );
        // A negative scale is clamped rather than inverting the launch.
        assert_eq!(launch_speed(50.0, Some(1.05), plain(700, 1.0, -3.0)), 50.0);
    }

    /// ⛔ AND THE GROWTH-BASE CURVE CAN ONLY STEEPEN, never nerf a poke — the
    /// property its own doc claims and the one a mistyped pivot would reach.
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
    // `6.28` is George's AUTHORED growth, copied from
    // `george_booul_moveset.rs` so this fixture is the shipped move and not a
    // number that resembles it — rounding it away to satisfy the lint would
    // silently make the test about something else. The authoring site carries
    // the same allow for the same reason.
    #[allow(clippy::approx_constant)]
    fn george() -> (crate::LaunchEnvelope, crate::LaunchEnvelope) {
        (
            crate::LaunchEnvelope::default().with_volume(185.0, Some(3.45)),
            crate::LaunchEnvelope::default().with_volume(178.0, Some(6.28)),
        )
    }

    /// The smash stage's law as it is declared TODAY, by value:
    /// `SMASH_VICTIM_PERCENT_KNOCKBACK_SCALE` `1.25` against a Robot v2's
    /// authored `knockback_weight` of `0.85`.
    ///
    /// ⛔ THE CURVE IS `IDENTITY` BECAUSE THE STAGE DECLARES `growth_base:
    /// None` — see [`GrowthBaseCurve`]. Writing the retired `48 / 0.25 / 1.40`
    /// in here would make the arm below certify a law nothing runs, and it is
    /// not needed: TWO of the omitted factors move the crossover on their own.
    ///
    /// ⚠ **COPIED RATHER THAN IMPORTED, ON PURPOSE.** This crate is below the
    /// demo and must not depend on it; what the arm needs is a law SHAPED like
    /// a real one, and the citation is what ties it to the shipped numbers. If
    /// the demo retunes, this test still asserts the same thing — that the
    /// factors reorder a kit — which is the claim, and not the tuning.
    fn the_smash_stage(victim_damage: i32) -> LaunchConditions {
        LaunchConditions {
            victim_damage,
            victim_weight: 0.85,
            growth_scale: 1.25,
            growth_base: GrowthBaseCurve::IDENTITY,
            // ⭐ AND THE STAGE'S FALLBACK GROWTH, `SMASH_KNOCKBACK_GROWTH`. It
            // moves nothing in the arms below — both smashes author their own
            // growth — and it is stated because a `0.0` here would be the
            // undeclared world wearing the stage's name.
            ruleset_growth: 0.02,
            rage: 1.0,
        }
    }

    /// ⛔⛤ **THE FACTORS THE BRAIN OMITTED MOVE THE CROSSOVER, WHICH IS THE
    /// WHOLE OF THE REVIEW FINDING.**
    ///
    /// `LaunchEnvelope::at` took only the victim's damage, and its doc argued
    /// that `growth_scale`, the victim's weight and rage are *"COMMON to every
    /// candidate one attacker weighs against one opponent, so none of them can
    /// reorder a kit"*. They multiply the PERCENT TERM and not `base`, so they
    /// reorder exactly by moving where two lines cross.
    ///
    /// ⭐ THE CONTROL IS THE FIRST ARM: under the identity law the OLD answer
    /// is the right one, so this is a statement about the conditions and not
    /// about the two numbers.
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

    /// ⛔ SET KNOCKBACK DECLINES EVERY ONE OF THEM, which is what authoring
    /// `Some(0.0)` means and the one thing a shared law must not quietly undo.
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

    /// ⛔ AND A ZERO-PERCENT HIT IS ITS BASE AT EVERY SCALE, because the
    /// victim's damage zeroes the term before any factor can touch it. Without
    /// this the percent knob would be a knockback buff.
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
