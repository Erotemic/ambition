//! The external consumer's own acceptance gate (Phase-6 / GPT 5.6 review:
//! "the fixture should contain integration tests rather than relying only on
//! binaries that print success"). Run from the engine repo with
//! `cargo test --manifest-path fixtures/external_consumer/Cargo.toml` — the
//! independent workspace resolves its own dependency graph, so this is
//! exactly the build a third-party consumer gets.

/// Boot → activate → verify population → walk the ridge gate. One test, the
/// whole authored surface: the room (construction), the character (catalog),
/// the sentry (roster + stager, lowered as a construction plan row), and the
/// transition (`transit_body`) — all exercised through the public `ambition`
/// umbrella with zero engine edits.
#[test]
fn outlander_boots_activates_and_walks_the_ridge_gate() {
    let mut app = outlander::build_outlander_app();
    let report = outlander::run_outlander_walkthrough(&mut app)
        .unwrap_or_else(|error| panic!("the Outlander walkthrough failed: {error}"));
    assert!(
        report.player_pos.y < 300.0,
        "the gate must deliver the player to the upper ledge, got {:?}",
        report.player_pos
    );
}
