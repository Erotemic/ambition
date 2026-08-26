use super::*;

/// The same seed plays the same run.
///
/// The rig is only useful for calibration if a number that moves means the BRAIN
/// moved. Two reports from one seed must be identical row for row.
#[test]
fn one_seed_is_one_report() {
    assert_eq!(
        report(0x5EED),
        report(0x5EED),
        "the evaluation rig is not deterministic, so no measurement taken with \
         it can attribute a change to the thing being calibrated"
    );
}

/// A scenario the brain cannot see is not a scenario it passed.
///
/// The suite's own `unreproduced_by_placement` list exists because some premises
/// need more than two positions to stage. The rig must not silently report on
/// those as though it had exercised them.
#[test]
fn the_report_covers_every_scenario_the_suite_names() {
    let rows = report(0x5EED);
    for scenario in suite() {
        assert!(
            rows.iter().any(|row| row.scenario == scenario.name),
            "'{}' is in the suite but produced no row, so the report is quietly \
             narrower than the fixture set it claims to run",
            scenario.name
        );
    }
}

/// Every rung stays inside the press budget it authors — and actually presses.
///
/// the non-vacuity half is the point. This check first went in while the rig
/// handed the brain no attack kit: zero presses, within every cap, green and
/// worthless. The `presses` assertion is what stops it returning to that state
/// if a future change empties the kit again.
#[test]
fn no_rung_presses_faster_than_its_profile_allows() {
    let rows = report(0x5EED);
    assert!(
        rows.iter().any(|row| row.apm > 0.0),
        "the rig produced no presses at all, so a within-cap result means only \
         that nothing happened — arm the snapshot's attack kit before trusting \
         any APM number from it"
    );
    let over: Vec<(&str, u8, f32, f32)> = rows
        .iter()
        .filter(|row| !row.within_apm_cap())
        .map(|row| (row.scenario, row.level, row.apm, row.apm_cap))
        .collect();
    assert!(
        over.is_empty(),
        "these rungs emitted more actions per minute than their profile \
         authorises (scenario, level, apm, cap): {over:?}"
    );
}

/// The ladder is ordered by press rate.
///
/// this is a narrower claim than "stronger levels win", and deliberately
/// so. Winning is a survival/damage question that needs two bodies fighting;
/// this rig has one brain and a scripted opponent. What it can say is that the
/// rungs are not interchangeable and that they order the way the ladder intends:
/// mean APM rises monotonically.
///
/// ⛔⛔ THE NUMBERS IN THIS COMMENT HAD DRIFTED BY MORE THAN 2x, re-measured
/// 2026-08-26 at seed `0x5EED` (`probe_ladder_census`). It said *"28.5 at L1 to
/// 103.5 at L9"* and *"a QUARTER of its own cap"*. Current:
///
/// ```text
/// L1 mean apm 12.7 (cap 120)   L5 29.3 (270)   L9 46.0 (420)
/// ```
///
/// ⇒ the conclusion SURVIVES and gets stronger — every rung now sits near a
/// TENTH of its own cap, so the caps separate the levels even less than this
/// paragraph claimed. Reaction and decision cadence are what order the ladder;
/// raising a cap alone would still move nothing.
///
/// ⚠ a number written into a comment beside a live probe goes stale silently.
/// Re-run `probe_ladder_census` before quoting these.
#[test]
fn the_ladder_is_ordered_by_press_rate() {
    let rows = report(0x5EED);
    let mean_apm = |level: u8| -> f32 {
        let of: Vec<&ScenarioOutcome> = rows.iter().filter(|row| row.level == level).collect();
        of.iter().map(|row| row.apm).sum::<f32>() / of.len() as f32
    };
    let ladder: Vec<(u8, f32)> = (1..=9u8).map(|level| (level, mean_apm(level))).collect();
    for pair in ladder.windows(2) {
        assert!(
            pair[1].1 > pair[0].1,
            "level {} does not press more often than level {} ({:.1} vs {:.1}); \
             the ladder's rungs are not ordered, so difficulty selection is a \
             label: {ladder:?}",
            pair[1].0,
            pair[0].0,
            pair[1].1,
            pair[0].1
        );
    }
}

/// PROBE: what does the armed rig actually emit? Print-only; run with
/// `--ignored` to read the ladder.
#[test]
#[ignore = "PROBE, print-only: reports the ladder census"]
fn probe_ladder_census() {
    let rows = report(0x5EED);
    for level in 1..=9u8 {
        let of: Vec<&ScenarioOutcome> = rows.iter().filter(|r| r.level == level).collect();
        let apm: f32 = of.iter().map(|r| r.apm).sum::<f32>() / of.len() as f32;
        let frames: usize = of.iter().map(|r| r.distinct_frames).sum();
        let cap = of.first().map(|r| r.apm_cap).unwrap_or(0.0);
        println!("L{level}: mean apm {apm:.1} (cap {cap:.0})  distinct frames {frames}");
    }
}
