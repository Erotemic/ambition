use super::*;

/// **The same seed plays the same run.**
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

/// **A scenario the brain cannot see is not a scenario it passed.**
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
