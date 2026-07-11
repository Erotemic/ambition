//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
            telegraph: None,
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
    let commitment: Vec<&FightFinding> = f.iter().filter(|x| x.rule == Rule::Commitment).collect();
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
    // The move; the fight demanding no core verb; the fight authoring no
    // telegraph identity (rule 5's warning, since BD3).
    assert_eq!(f.len(), 3);
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

// ── Rule 5: the readability floor (expressible only since BD3) ────────────

fn telegraphed(id: &str, pose: &str, cue: &str) -> Vec<BossPatternStep> {
    vec![
        BossPatternStep::Telegraph {
            profile: BossAttackProfile::Strike(id.to_string()),
            duration: 1.0,
            telegraph: Some(TelegraphSpec {
                pose: Some(pose.to_string()),
                cue: Some(cue.to_string()),
                vfx: None,
            }),
        },
        BossPatternStep::Strike {
            profile: BossAttackProfile::Strike(id.to_string()),
            duration: 0.4,
        },
        BossPatternStep::Rest { duration: 1.0 },
    ]
}

/// **The rule 5 that a duration cannot express.** Two attacks wind up for the
/// same 1.0s and look identical: nothing on screen tells them apart. A
/// telegraph DURATION says how long the player has; a telegraph IDENTITY says
/// what they are looking at.
#[test]
fn two_attacks_that_share_a_pose_and_a_cue_are_unreadable() {
    let mut steps = telegraphed("floor_slam", "rear_up", "boss_growl");
    steps.extend(telegraphed("eye_beam", "rear_up", "boss_growl"));
    let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
    let hit = f
        .iter()
        .find(|x| x.rule == Rule::ReadabilityFloor && x.severity == Severity::Error)
        .expect("identical telegraph identities");
    assert!(hit.subject.contains("eye_beam") && hit.subject.contains("floor_slam"));
}

/// Differing in EITHER half is enough — §3 says "pose row OR cue".
#[test]
fn differing_in_the_pose_or_in_the_cue_is_enough() {
    for (pose_b, cue_b) in [("crouch", "boss_growl"), ("rear_up", "boss_hiss")] {
        let mut steps = telegraphed("floor_slam", "rear_up", "boss_growl");
        steps.extend(telegraphed("eye_beam", pose_b, cue_b));
        let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
        assert!(
            f.iter()
                .all(|x| !(x.rule == Rule::ReadabilityFloor && x.severity == Severity::Error)),
            "({pose_b}, {cue_b}) should be distinguishable"
        );
    }
}

/// The SAME attack telegraphing the same way in two phases is not two attacks.
/// A fight is allowed to repeat itself; it is not allowed to lie.
#[test]
fn one_attack_repeated_shares_its_own_identity_without_complaint() {
    let mut steps = telegraphed("floor_slam", "rear_up", "boss_growl");
    steps.extend(telegraphed("floor_slam", "rear_up", "boss_growl"));
    let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
    assert!(f
        .iter()
        .all(|x| !(x.rule == Rule::ReadabilityFloor && x.severity == Severity::Error)));
}

/// An all-`None` spec is authored noise and reads as ABSENT — otherwise every
/// attack that "authored a telegraph" of nothing would collide with every other.
#[test]
fn an_empty_telegraph_spec_reads_as_no_telegraph_at_all() {
    let empty = TelegraphSpec::default();
    assert!(!empty.is_authored());

    let steps = vec![
        BossPatternStep::Telegraph {
            profile: BossAttackProfile::Strike("floor_slam".to_string()),
            duration: 1.0,
            telegraph: Some(empty),
        },
        BossPatternStep::Strike {
            profile: BossAttackProfile::Strike("floor_slam".to_string()),
            duration: 0.4,
        },
        BossPatternStep::Rest { duration: 1.0 },
    ];
    let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
    let warn = f
        .iter()
        .find(|x| x.rule == Rule::ReadabilityFloor && x.severity == Severity::Warning)
        .expect("an unauthored telegraph is reported once per fight");
    assert!(warn.detail.contains("floor_slam"));
}

/// An attack with no telegraph identity is a WARNING today, because the shipped
/// roster authors none. §3 promises an error after BD7's pilot; this test is
/// where that promise gets kept.
#[test]
fn unidentified_telegraphs_are_reported_once_per_fight_not_once_per_beat() {
    let mut steps = strike("floor_slam", 1.0, 0.4);
    steps.push(BossPatternStep::Rest { duration: 1.0 });
    steps.extend(strike("floor_slam", 1.0, 0.4));
    steps.push(BossPatternStep::Rest { duration: 1.0 });
    let f = validate_fight("x", &beats(&scripted(steps)), &seeds(), &bands());
    let warns: Vec<&FightFinding> = f
        .iter()
        .filter(|x| x.rule == Rule::ReadabilityFloor)
        .collect();
    assert_eq!(warns.len(), 1, "one report per fight, not per beat");
    assert_eq!(warns[0].severity, Severity::Warning);
}

/// **BD3 byte-parity.** A pre-BD3 `Telegraph` row parses with no `telegraph`
/// field and behaves exactly as before.
#[test]
fn a_pre_bd3_telegraph_row_still_parses() {
    use crate::brain::boss_pattern::BossPattern;
    let p: BossPattern = ron::from_str(
        r#"(steps: [
            Telegraph(profile: Strike("floor_slam"), duration: 1.2),
            Strike(profile: Strike("floor_slam"), duration: 0.4),
        ])"#,
    )
    .expect("a pre-BD3 row parses");
    match &p.steps[0] {
        BossPatternStep::Telegraph { telegraph, .. } => assert!(telegraph.is_none()),
        _ => panic!("expected a Telegraph"),
    }
}

/// ...and a BD3 row parses its anticipation.
#[test]
fn a_bd3_telegraph_row_parses_its_pose_and_cue() {
    use crate::brain::boss_pattern::BossPattern;
    let p: BossPattern = ron::from_str(
        r#"(steps: [
            Telegraph(
                profile: Strike("floor_slam"),
                duration: 1.2,
                telegraph: Some((pose: Some("rear_up"), cue: Some("warden_slam_tell"))),
            ),
            Strike(profile: Strike("floor_slam"), duration: 0.4),
        ])"#,
    )
    .expect("a BD3 row parses");
    match &p.steps[0] {
        BossPatternStep::Telegraph { telegraph, .. } => {
            let spec = telegraph.as_ref().unwrap();
            assert!(spec.is_authored());
            assert_eq!(spec.identity(), (Some("rear_up"), Some("warden_slam_tell")));
        }
        _ => panic!("expected a Telegraph"),
    }
}
