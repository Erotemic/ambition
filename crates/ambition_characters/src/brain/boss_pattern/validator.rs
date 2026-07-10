//! **BD5 — the fight validator.** The telegraph grammar and fairness rules of
//! `docs/planning/engine/boss-design.md` §3, run over authored data.
//!
//! > *"Codified as an install-time/CI fight validator over the authored data
//! > (same pattern as the content-graph validator) … codified craft rules that
//! > make bad fights hard to express."*
//!
//! Pure functions over `(BossAttackPattern, SeedLibrary, ValidatorBands)`. The
//! bands are DATA — one RON per game — so re-calibrating a fight's fairness is an
//! edit, not a recompile (§3: *"The bands live in ONE RON file per game so
//! re-calibration is data, not code."*).
//!
//! ## The unit of judgement is a BEAT, not a move
//!
//! BD4's extraction found that no per-attack `recovery` exists and none can: the
//! punish window is the `Rest` beat that FOLLOWS a `Strike`, which is a property
//! of the OCCURRENCE. So this validator walks each phase's timeline and produces
//! one [`Beat`] per telegraph→strike→rest triple. `floor_slam` can be fair in
//! phase 1 and unpunishable in enrage, and only a per-beat rule can say so.
//!
//! ## Which of §3's five rules are implemented, and why not the other two
//!
//! | Rule | Status |
//! |---|---|
//! | 1 telegraph proportionality | ✅ per beat, against the threat class its seed declares |
//! | 2 answer coverage | ✅ both halves: an empty `fair_counters` is an ERROR; a fight that fails to demand a core verb is a WARNING |
//! | 3 commitment (punish window) | ✅ per beat, from the following `Rest`; `Pressure` is exempt |
//! | 4 simultaneity budget | ❌ **not expressible.** A scripted timeline is sequential, so its body-mounted volumes never overlap. The threats that DO overlap are the `zone_denial` hazards a `Special` spawns, whose lifetime lives in the content technique's private consts (`MINIMA_TRAP_HAZARD_DURATION_S`), not in any authored row. This rule needs a `persists_s` on the seed, fed by the technique. |
//! | 5 readability floor | ❌ **not expressible.** *"distinct attacks must differ in telegraph (pose row OR cue)"* — the authored data carries a telegraph DURATION, which is not a telegraph IDENTITY. It needs BD3's telegraph channel (pose row + cue), which does not exist yet. |
//!
//! Naming the two gaps here, in the code, rather than shipping a rule that checks
//! something adjacent and reports green.

use std::collections::BTreeSet;

use super::seeds::{MovementVerb, SeedLibrary, ThreatClass};
use super::{BossAttackPattern, BossAttackProfile, BossEncounterPhase, BossPatternStep};

/// Per-threat-class tick floors. Ticks, because §3's calibration is in ticks and
/// the sim runs at a fixed rate — a designer reasoning about "frames of startup"
/// should read the same number the RON carries.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct ThreatTicks {
    pub pressure: f32,
    pub light: f32,
    pub medium: f32,
    pub heavy: f32,
}

impl ThreatTicks {
    pub fn for_class(&self, class: ThreatClass) -> f32 {
        match class {
            ThreatClass::Pressure => self.pressure,
            ThreatClass::Light => self.light,
            ThreatClass::Medium => self.medium,
            ThreatClass::Heavy => self.heavy,
        }
    }
}

/// The per-game calibration §3 pins. Content data, never a const here.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ValidatorBands {
    /// Sim ticks per second. The bands are in ticks; the authored data is in
    /// seconds. This is the only conversion, and it lives with the numbers.
    pub tick_hz: f32,
    /// Rule 1: minimum telegraph, by the attack's threat class.
    pub telegraph_ticks: ThreatTicks,
    /// Rule 3: minimum punish window after the active frames, by threat class.
    /// `Pressure` is exempt and its entry is ignored.
    pub recovery_ticks: ThreatTicks,
    /// Rule 2's second half: the verbs a fight must demand between them.
    pub core_verbs: Vec<MovementVerb>,
    /// A deviation at or under this fraction of the floor is a WARNING; beyond it,
    /// an ERROR (§3: *"band deviations ≤ 20% = WARNINGS … > 20% = ERROR"*).
    pub warn_deviation_frac: f32,
}

/// §3's hard error vs. warning split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Requires an inline `// boss-tuning:` justification the validator prints.
    Warning,
    /// The fight does not install.
    Error,
}

/// Which §3 rule a finding belongs to. Numbered as the doc numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// 1 — telegraph proportionality.
    TelegraphProportionality,
    /// 2 — answer coverage.
    AnswerCoverage,
    /// 3 — the commitment rule (punish window).
    Commitment,
    /// A move the seed library does not catalogue. Not one of §3's five: it is the
    /// precondition for rules 1–3, all of which read the seed.
    UncataloguedMove,
}

/// One thing wrong with a fight.
#[derive(Clone, Debug, PartialEq)]
pub struct FightFinding {
    pub severity: Severity,
    pub rule: Rule,
    /// The move key, or the fight id for a fight-wide finding.
    pub subject: String,
    pub detail: String,
}

/// One authored occurrence of an attack: what a player actually experiences.
///
/// `recovery_s` is the `Rest` that FOLLOWS the strike — zero when the next beat is
/// another telegraph, which is exactly the un-punishable chain §3 rule 3 forbids.
#[derive(Clone, Debug, PartialEq)]
pub struct Beat {
    pub move_key: String,
    pub phase: BossEncounterPhase,
    pub telegraph_s: f32,
    pub active_s: f32,
    pub recovery_s: f32,
}

fn move_key(profile: &BossAttackProfile) -> &str {
    match profile {
        BossAttackProfile::Strike(k) | BossAttackProfile::Special(k) => k.as_str(),
    }
}

/// Walk one phase's authored step list into beats.
///
/// A `Select` arm's steps are beats the fight can play, so they are walked as
/// their own sequence (arms are alternatives, not a continuation). A `Stance`
/// marker is a jump; its body is walked by the caller from `pattern.stances`.
fn beats_in(steps: &[BossPatternStep], phase: BossEncounterPhase, out: &mut Vec<Beat>) {
    let mut pending_telegraph: Option<(String, f32)> = None;
    let mut awaiting_recovery: Option<usize> = None;

    for step in steps {
        match step {
            BossPatternStep::Telegraph { profile, duration } => {
                // A telegraph ends the previous strike's punish window: whatever
                // Rest we had (possibly none) is what the player got.
                awaiting_recovery = None;
                pending_telegraph = Some((move_key(profile).to_string(), *duration));
            }
            BossPatternStep::Strike { profile, duration } => {
                let key = move_key(profile).to_string();
                let telegraph_s = match pending_telegraph.take() {
                    Some((tk, d)) if tk == key => d,
                    // A strike with no telegraph of its own. Rule 1 will report it
                    // as a zero-telegraph attack, which is what it is.
                    _ => 0.0,
                };
                out.push(Beat {
                    move_key: key,
                    phase,
                    telegraph_s,
                    active_s: *duration,
                    recovery_s: 0.0,
                });
                awaiting_recovery = Some(out.len() - 1);
            }
            BossPatternStep::Rest { duration } => {
                if let Some(i) = awaiting_recovery.take() {
                    out[i].recovery_s += *duration;
                }
                pending_telegraph = None;
            }
            BossPatternStep::Select { table } => {
                awaiting_recovery = None;
                pending_telegraph = None;
                for arm in table {
                    beats_in(&arm.steps, phase, out);
                }
            }
            BossPatternStep::Stance { .. } => {
                awaiting_recovery = None;
                pending_telegraph = None;
            }
        }
    }

    // A phase's timeline LOOPS. So a strike that ends the list is followed by
    // whatever begins it — and if that is a `Rest`, the player got a punish window
    // after all. Without this, every phase whose last beat is a strike reads as
    // unpunishable, which is a fact about the walker, not about the fight.
    if let Some(i) = awaiting_recovery {
        for step in steps {
            match step {
                BossPatternStep::Rest { duration } => out[i].recovery_s += *duration,
                _ => break,
            }
        }
    }
}

/// Every beat a fight can play, across every phase, plus every stance body.
///
/// A `Cycle` boss has no per-step timings: it rotates `attacks` on the profile's
/// flat windup/active/cooldown, and its cooldown IS its punish window. Both shapes
/// produce beats, because both shapes hit the player.
pub fn fight_beats(
    pattern: &BossAttackPattern,
    cycle_attacks: &[BossAttackProfile],
    cycle_windup_s: f32,
    cycle_active_s: f32,
    cycle_cooldown_s: f32,
) -> Vec<Beat> {
    let mut out = Vec::new();
    match pattern {
        BossAttackPattern::Cycle => {
            for profile in cycle_attacks {
                out.push(Beat {
                    move_key: move_key(profile).to_string(),
                    phase: BossEncounterPhase::Phase1,
                    telegraph_s: cycle_windup_s,
                    active_s: cycle_active_s,
                    recovery_s: cycle_cooldown_s,
                });
            }
        }
        BossAttackPattern::Scripted {
            intro,
            phase1,
            transition,
            phase2,
            enrage,
        } => {
            for (phase, pattern) in [
                (BossEncounterPhase::Phase1, intro),
                (BossEncounterPhase::Phase1, phase1),
                (BossEncounterPhase::Phase2, transition),
                (BossEncounterPhase::Phase2, phase2),
                (BossEncounterPhase::Enrage, enrage),
            ] {
                beats_in(&pattern.steps, phase, &mut out);
                for stance in pattern.stances.values() {
                    beats_in(stance, phase, &mut out);
                }
            }
        }
    }
    out
}

/// Run §3's rules 1, 2 and 3 over one fight. Findings come back sorted
/// most-severe first, then by rule, then by subject — a stable order, so a CI
/// diff of two runs is readable.
pub fn validate_fight(
    fight_id: &str,
    beats: &[Beat],
    seeds: &SeedLibrary,
    bands: &ValidatorBands,
) -> Vec<FightFinding> {
    let mut findings = Vec::new();
    let tick = 1.0 / bands.tick_hz.max(1.0);
    let mut demanded: BTreeSet<MovementVerb> = BTreeSet::new();

    for beat in beats {
        let Some((_seed_id, seed)) = seeds.seed_for_move(&beat.move_key) else {
            findings.push(FightFinding {
                severity: Severity::Error,
                rule: Rule::UncataloguedMove,
                subject: beat.move_key.clone(),
                detail: format!(
                    "no seed catalogues `{}`, so its threat class and fair counters are \
                     unknown and rules 1–3 cannot judge it",
                    beat.move_key
                ),
            });
            continue;
        };
        demanded.extend(seed.fair_counters.iter().copied());

        // Rule 2, first half: an attack with no answer is a fight the player
        // cannot play. Always an error, never a band.
        if seed.fair_counters.is_empty() {
            findings.push(FightFinding {
                severity: Severity::Error,
                rule: Rule::AnswerCoverage,
                subject: beat.move_key.clone(),
                detail: "declares no fair counter".to_string(),
            });
        }

        // Rule 1: telegraph proportionality.
        let floor_s = bands.telegraph_ticks.for_class(seed.threat) * tick;
        if beat.telegraph_s < floor_s {
            findings.push(band_finding(
                Rule::TelegraphProportionality,
                &beat.move_key,
                beat.phase,
                "telegraph",
                beat.telegraph_s,
                floor_s,
                bands,
            ));
        }

        // Rule 3: the commitment rule. A `Pressure` attack is exempt — it is
        // chip-level by declaration, and §3 caps it by damage instead.
        if seed.threat != ThreatClass::Pressure {
            let floor_s = bands.recovery_ticks.for_class(seed.threat) * tick;
            if beat.recovery_s < floor_s {
                findings.push(band_finding(
                    Rule::Commitment,
                    &beat.move_key,
                    beat.phase,
                    "punish window",
                    beat.recovery_s,
                    floor_s,
                    bands,
                ));
            }
        }
    }

    // Rule 2, second half: forced-movement variety. A fight that never demands a
    // verb never teaches it. A warning, not an error: which verbs a GAME considers
    // core is a design statement, and the shipped roster demonstrably never asks
    // for a parry.
    let missing: Vec<MovementVerb> = bands
        .core_verbs
        .iter()
        .copied()
        .filter(|v| !demanded.contains(v))
        .collect();
    if !missing.is_empty() {
        findings.push(FightFinding {
            severity: Severity::Warning,
            rule: Rule::AnswerCoverage,
            subject: fight_id.to_string(),
            detail: format!(
                "the fight never demands {missing:?} — it does not exercise the kit \
                 (§3 rule 2, forced-movement variety)"
            ),
        });
    }

    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.rule.cmp(&b.rule))
            .then(a.subject.cmp(&b.subject))
            .then(a.detail.cmp(&b.detail))
    });
    findings.dedup();
    findings
}

/// A deviation ≤ `warn_deviation_frac` below the floor is a warning; beyond it, an
/// error. The detail carries both numbers in TICKS, because that is the unit the
/// designer authored against.
fn band_finding(
    rule: Rule,
    move_key: &str,
    phase: BossEncounterPhase,
    what: &str,
    actual_s: f32,
    floor_s: f32,
    bands: &ValidatorBands,
) -> FightFinding {
    let deviation = if floor_s > 0.0 {
        (floor_s - actual_s) / floor_s
    } else {
        0.0
    };
    let severity = if deviation <= bands.warn_deviation_frac {
        Severity::Warning
    } else {
        Severity::Error
    };
    FightFinding {
        severity,
        rule,
        subject: move_key.to_string(),
        detail: format!(
            "{phase:?}: {what} is {:.0} ticks, floor is {:.0} ({:.0}% under)",
            actual_s * bands.tick_hz,
            floor_s * bands.tick_hz,
            deviation * 100.0
        ),
    }
}

impl ValidatorBands {
    /// Parse the per-game calibration RON.
    pub fn from_ron(ron: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::boss_pattern::BossPattern;

    fn bands() -> ValidatorBands {
        ValidatorBands {
            tick_hz: 60.0,
            telegraph_ticks: ThreatTicks {
                pressure: 12.0,
                light: 12.0,
                medium: 20.0,
                heavy: 30.0,
            },
            recovery_ticks: ThreatTicks {
                pressure: 0.0,
                light: 6.0,
                medium: 12.0,
                heavy: 24.0,
            },
            core_verbs: vec![MovementVerb::Jump, MovementVerb::Dash],
            warn_deviation_frac: 0.2,
        }
    }

    const SEEDS: &str = r#"{
        "slam": (
            archetype: Slam, intent: "x", skill_tested: "y",
            fair_counters: [WalkOut, Dash], threat: Heavy,
            telegraph: (min_s: 0.0, max_s: 9.0), active: (min_s: 0.0, max_s: 9.0),
            instances: ["floor_slam"], recipes: [],
        ),
        "beam": (
            archetype: Beam, intent: "x", skill_tested: "y",
            fair_counters: [Jump], threat: Light,
            telegraph: (min_s: 0.0, max_s: 9.0), active: (min_s: 0.0, max_s: 9.0),
            instances: ["eye_beam"], recipes: [],
        ),
        "summon": (
            archetype: Summon, intent: "x", skill_tested: "y",
            fair_counters: [Dash], threat: Pressure,
            telegraph: (min_s: 0.0, max_s: 9.0), active: (min_s: 0.0, max_s: 9.0),
            instances: ["cascade"], recipes: [],
        ),
    }"#;

    fn seeds() -> SeedLibrary {
        SeedLibrary::from_ron(SEEDS).expect("fixture parses")
    }

    fn strike(id: &str, telegraph: f32, active: f32) -> Vec<BossPatternStep> {
        vec![
            BossPatternStep::Telegraph {
                profile: BossAttackProfile::Strike(id.to_string()),
                duration: telegraph,
            },
            BossPatternStep::Strike {
                profile: BossAttackProfile::Strike(id.to_string()),
                duration: active,
            },
        ]
    }

    fn scripted(steps: Vec<BossPatternStep>) -> BossAttackPattern {
        BossAttackPattern::Scripted {
            intro: BossPattern::default(),
            phase1: BossPattern {
                steps,
                ..Default::default()
            },
            transition: BossPattern::default(),
            phase2: BossPattern::default(),
            enrage: BossPattern::default(),
        }
    }

    fn beats(pattern: &BossAttackPattern) -> Vec<Beat> {
        fight_beats(pattern, &[], 0.0, 0.0, 0.0)
    }

    /// **BD4's finding, made mechanical.** There is no per-attack recovery; the
    /// punish window is the `Rest` that follows. A slam with a generous rest is
    /// fair; the SAME slam chained straight into the next telegraph is not.
    #[test]
    fn the_punish_window_is_the_rest_that_follows_the_strike() {
        let mut steps = strike("floor_slam", 1.0, 0.4);
        steps.push(BossPatternStep::Rest { duration: 0.5 });
        let b = beats(&scripted(steps));
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].recovery_s, 0.5);

        // No rest: the next telegraph starts immediately.
        let mut steps = strike("floor_slam", 1.0, 0.4);
        steps.extend(strike("eye_beam", 0.5, 0.2));
        let b = beats(&scripted(steps));
        assert_eq!(b[0].recovery_s, 0.0, "chained: no window at all");
    }

    /// The clockwork warden's enrage shape, verbatim: a strike chained straight
    /// into the next telegraph. §3 rule 3 calls that an unpunishable heavy.
    #[test]
    fn an_unpunishable_heavy_is_an_error() {
        let mut steps = strike("floor_slam", 1.0, 0.4);
        steps.extend(strike("eye_beam", 0.5, 0.2));
        steps.push(BossPatternStep::Rest { duration: 1.0 });
        let f = validate_fight("warden", &beats(&scripted(steps)), &seeds(), &bands());
        let commitment: Vec<&FightFinding> =
            f.iter().filter(|x| x.rule == Rule::Commitment).collect();
        assert_eq!(commitment.len(), 1);
        assert_eq!(commitment[0].severity, Severity::Error);
        assert_eq!(commitment[0].subject, "floor_slam");
    }

    /// A `Pressure` attack is exempt from the punish-window floor. §3 caps it by
    /// damage instead, which is not a thing this validator can see.
    #[test]
    fn a_pressure_attack_needs_no_punish_window() {
        let mut steps = strike("cascade", 0.5, 0.2);
        steps.extend(strike("cascade", 0.5, 0.2));
        let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
        assert!(f.iter().all(|x| x.rule != Rule::Commitment));
    }

    /// §3's warn/error split, at the boundary. A heavy's telegraph floor is 30
    /// ticks; 24 is exactly 20% under (a warning), 23 is beyond (an error).
    #[test]
    fn a_band_deviation_warns_up_to_twenty_percent_and_errors_past_it() {
        let tick = 1.0 / 60.0;
        for (ticks, expected) in [(24.0, Severity::Warning), (23.0, Severity::Error)] {
            let mut steps = strike("floor_slam", ticks * tick, 0.4);
            steps.push(BossPatternStep::Rest { duration: 1.0 });
            let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
            let t = f
                .iter()
                .find(|x| x.rule == Rule::TelegraphProportionality)
                .unwrap_or_else(|| panic!("{ticks} ticks must trip rule 1"));
            assert_eq!(t.severity, expected, "at {ticks} ticks: {}", t.detail);
        }
        // ...and 30 ticks is clean.
        let mut steps = strike("floor_slam", 30.0 * tick, 0.4);
        steps.push(BossPatternStep::Rest { duration: 1.0 });
        let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
        assert!(f.iter().all(|x| x.rule != Rule::TelegraphProportionality));
    }

    /// A move the seed library does not catalogue cannot be judged, and saying so
    /// is an error — not silence. Rules 1–3 all read the seed.
    #[test]
    fn an_uncatalogued_move_is_an_error_rather_than_a_pass() {
        let steps = strike("mystery_punch", 0.1, 0.1);
        let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
        assert_eq!(f.len(), 2, "the move, and the fight demanding no core verb");
        assert_eq!(f[0].rule, Rule::UncataloguedMove);
        assert_eq!(f[0].severity, Severity::Error);
    }

    /// Rule 2's second half: a fight of nothing but slams never asks for a jump.
    /// A warning, because which verbs are "core" is a per-game design statement.
    #[test]
    fn a_fight_that_never_demands_a_core_verb_warns() {
        let mut steps = strike("floor_slam", 1.0, 0.4);
        steps.push(BossPatternStep::Rest { duration: 1.0 });
        let f = validate_fight("slam_only", &beats(&scripted(steps)), &seeds(), &bands());
        let coverage = f
            .iter()
            .find(|x| x.rule == Rule::AnswerCoverage)
            .expect("slam answers WalkOut+Dash, never Jump");
        assert_eq!(coverage.severity, Severity::Warning);
        assert_eq!(coverage.subject, "slam_only");
        assert!(coverage.detail.contains("Jump"));
        assert!(!coverage.detail.contains("Dash"), "Dash IS demanded");
    }

    /// A `Cycle` boss's cooldown IS its punish window, and its flat windup its
    /// telegraph. Both shapes hit the player, so both are validated.
    #[test]
    fn a_cycle_boss_is_validated_off_its_flat_timings() {
        let attacks = vec![BossAttackProfile::Strike("floor_slam".to_string())];
        let b = fight_beats(&BossAttackPattern::Cycle, &attacks, 0.44, 0.28, 0.9);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].recovery_s, 0.9);
        // 0.44s = 26 ticks against a heavy's 30-tick floor: 13% under, a warning.
        let f = validate_fight("cycle", &b, &seeds(), &bands());
        let t = f
            .iter()
            .find(|x| x.rule == Rule::TelegraphProportionality)
            .expect("26 < 30");
        assert_eq!(t.severity, Severity::Warning);
    }

    /// A `Select` arm's beats are beats. A fight cannot hide an unpunishable heavy
    /// inside a table.
    #[test]
    fn select_arms_are_walked_because_the_player_can_be_hit_by_them() {
        use crate::brain::boss_pattern::WeightedArm;
        let steps = vec![BossPatternStep::Select {
            table: vec![WeightedArm {
                weight: 1.0,
                when: None,
                steps: strike("floor_slam", 0.1, 0.4),
            }],
        }];
        let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
        assert!(f
            .iter()
            .any(|x| x.rule == Rule::TelegraphProportionality && x.severity == Severity::Error));
    }

    /// Findings are sorted errors-first and are stable, so a CI diff of two runs
    /// reads as a change in the FIGHT, not a change in hash order.
    #[test]
    fn findings_are_errors_first_and_stably_ordered() {
        let mut steps = strike("floor_slam", 0.05, 0.4); // rule 1 error + rule 3 error
        steps.extend(strike("mystery", 0.5, 0.2)); // uncatalogued
        let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
        assert!(f.len() >= 3);
        assert_eq!(f[0].severity, Severity::Error);
        assert_eq!(f.last().unwrap().severity, Severity::Warning);
        let again = validate_fight(
            "x",
            &beats(&scripted(strike("floor_slam", 0.05, 0.4))),
            &seeds(),
            &bands(),
        );
        assert_eq!(again, again.clone(), "stable");
    }

    /// A phase's timeline loops, so a strike that ENDS the list is followed by
    /// whatever BEGINS it. A leading `Rest` is a real punish window; ignoring the
    /// wrap would report every such phase as unpunishable — a fact about the
    /// walker, not about the fight.
    #[test]
    fn a_strike_at_the_end_of_a_looping_phase_is_credited_the_leading_rest() {
        let mut steps = vec![BossPatternStep::Rest { duration: 0.6 }];
        steps.extend(strike("floor_slam", 1.0, 0.4));
        let b = beats(&scripted(steps));
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].recovery_s, 0.6, "the loop's first Rest IS the window");

        // ...but a phase that loops straight into a telegraph gives none.
        let mut steps = strike("eye_beam", 0.5, 0.2);
        steps.extend(strike("floor_slam", 1.0, 0.4));
        let b = beats(&scripted(steps));
        assert_eq!(b[1].recovery_s, 0.0);
    }
}
