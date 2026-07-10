//! **BD5 run over the shipped roster** — `boss-design.md` §3's rules, measured.
//!
//! §3's endgame is an install-time gate: *"missing telegraph, empty fair_counters,
//! unpunishable heavy, simultaneity budget exceeded = ERRORS (fight does not
//! install)."* That gate cannot be switched on today, and the honest reason is
//! written into the report below rather than into a silenced assertion: **the
//! bands are Calibration v0**, which the doc itself calls *"starting numbers …
//! BD7's pilot re-calibrates them against Jon's verdict."* Turning them into a
//! hard gate before that pilot would not make the fights fairer; it would make the
//! numbers unfalsifiable.
//!
//! So this test does what CC3's oracle does: it MEASURES, it prints a stable
//! report, and it pins the current findings so a change in a fight shows up as a
//! change in the report. The day BD7 recalibrates, `expected_errors` goes to zero
//! and the pin becomes the gate.

use std::collections::BTreeMap;

use ambition_actors::features::BossBehaviorProfile;
use ambition_characters::brain::boss_pattern::validator::{
    fight_beats, validate_fight, FightFinding, Severity,
};
use ambition_content::bosses::{seed_library, validator_bands, BOSS_PROFILES_RON};

fn profiles() -> BTreeMap<String, BossBehaviorProfile> {
    ron::from_str(BOSS_PROFILES_RON).expect("boss_profiles.ron parses")
}

fn findings_for(id: &str, profile: &BossBehaviorProfile) -> Vec<FightFinding> {
    let beats = fight_beats(
        &profile.attack_pattern,
        &profile.attacks,
        profile.attack_windup,
        profile.attack_active,
        profile.attack_cooldown,
    );
    validate_fight(id, &beats, seed_library(), validator_bands())
}

/// The bands parse, and they say what §3 pinned. A calibration file that drifted
/// from its own doc is worse than none.
#[test]
fn the_shipped_bands_are_section_threes_calibration_v0() {
    let b = validator_bands();
    assert_eq!(b.tick_hz, 60.0);
    assert_eq!(b.telegraph_ticks.light, 12.0);
    assert_eq!(b.telegraph_ticks.medium, 20.0);
    assert_eq!(b.telegraph_ticks.heavy, 30.0);
    assert_eq!(b.recovery_ticks.heavy, 24.0);
    assert_eq!(b.recovery_ticks.medium, 12.0);
    assert_eq!(b.warn_deviation_frac, 0.2);
}

/// Every move every shipped boss plays is catalogued by a seed. If this ever goes
/// red, BD4's coverage oracle went red first — but the validator would otherwise
/// report a green fight it could not actually judge, and that is the failure mode
/// worth two tests.
#[test]
fn the_validator_can_judge_every_move_the_roster_plays() {
    use ambition_characters::brain::boss_pattern::validator::Rule;
    let mut uncatalogued = Vec::new();
    for (id, profile) in profiles() {
        for f in findings_for(&id, &profile) {
            if f.rule == Rule::UncataloguedMove {
                uncatalogued.push(format!("{id}: {}", f.subject));
            }
        }
    }
    assert!(
        uncatalogued.is_empty(),
        "the validator cannot judge these moves — no seed catalogues them: {uncatalogued:?}"
    );
}

/// **THE MEASUREMENT.** Prints the full report and pins the error count.
///
/// Run it alone to read the report:
/// ```text
/// cargo test -p ambition_content --test boss_fight_validator -- --nocapture
/// ```
#[test]
fn the_shipped_roster_against_section_threes_rules() {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut report = String::new();

    for (id, profile) in profiles() {
        let findings = findings_for(&id, &profile);
        if findings.is_empty() {
            continue;
        }
        report.push_str(&format!("\n{id}:\n"));
        for f in &findings {
            match f.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
            report.push_str(&format!(
                "  {:7?} {:24?} {:22} {}\n",
                f.severity, f.rule, f.subject, f.detail
            ));
        }
    }
    eprintln!(
        "\n=== BD5: the shipped roster vs boss-design.md §3 ===\n{report}\n\
               {errors} error(s), {warnings} warning(s)\n"
    );

    // The pin. Not zero — see this file's header. A change in either number means
    // a fight changed, and the report above says which and how.
    assert_eq!(
        (errors, warnings),
        (EXPECTED_ERRORS, EXPECTED_WARNINGS),
        "the roster's fairness profile changed. Read the report above. If a fight \
         got FAIRER, lower these numbers in the same commit; if it got less fair, \
         that is the finding."
    );
}

/// **Measured 2026-07-10 against Calibration v0.**
///
/// All 8 errors are rule 3, all in **Enrage**, all the same shape: the tightened
/// enrage combos chain a `Strike` straight into the next `Telegraph`, leaving the
/// player no punish window at all. §3 calls that an unpunishable attack; the
/// authors called it escalation. Which of them is right is BD7's pilot to settle
/// with Jon, and it is exactly the argument the pipeline exists to make legible.
///
/// Of the 10 warnings, **nine are rule 5 (BD3, 2026-07-10): not one shipped boss
/// authors a telegraph identity.** Every attack in the game telegraphs by duration
/// alone. §3 makes that an ERROR after BD7's pilot; today it is a measurement, and
/// it is the single largest readability gap the pipeline has found.
///
/// The tenth: the smirking behemoth never demands a `WalkOut`. Its kit is a beam,
/// a sweep, a slam and a nova — every one answered by jumping or dashing. A player
/// never has to simply *step out of the way*.
///
/// Rule 1 (telegraph proportionality) fires **nowhere**, which corrects BD4 §7's
/// finding 3: the mockingbird's 26-tick cycle telegraphs are `Medium` attacks
/// (`sweep`, `dash_through`), whose floor is 20 ticks, not a heavy's 30.
const EXPECTED_ERRORS: usize = 8;
const EXPECTED_WARNINGS: usize = 10;
