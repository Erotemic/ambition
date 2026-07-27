//! The external consumer's own acceptance gate (Phase-6 / GPT 5.6 review:
//! "the fixture should contain integration tests rather than relying only on
//! binaries that print success"). Run from the engine repo with
//! `cargo test --manifest-path fixtures/external_consumer/Cargo.toml` — the
//! independent workspace resolves its own dependency graph, so this is
//! exactly the build a third-party consumer gets.

/// Boot → activate → verify population → charge the beacon → walk the ridge
/// gate. One test, the whole authored surface: the room (construction), the
/// character (catalog), the sentry (roster + stager, lowered as a construction
/// plan row), the consumer's own authoritative component (§authority), and the
/// transition (`transit_body`) — all through the public `ambition` umbrella with
/// zero engine edits.
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
    assert!(
        report.beacon.is_full(),
        "the gate is supposed to be GATED on the consumer's own authoritative \
         state; a gate that fired on an uncharged beacon is testing nothing: {:?}",
        report.beacon
    );
}

/// **Task 1's exit criterion, answered from outside the engine.**
///
/// *"A feature-owned authoritative component and system are mechanically
/// accounted, run under the simulation gate, and survive real
/// rewind/resimulation without edits to a giant runtime list."*
///
/// Every word of that is checked here, and it has to be here: the engine's own
/// registrations are crate-private conveniences away from being unusable by
/// anyone else, and a test living inside the workspace cannot tell the
/// difference. `BeaconCharge` is declared in this crate, encoded by this crate,
/// registered by this crate through `ambition::runtime::rollback`, and named in
/// no engine file.
///
/// The rewind is REAL, not simulated: a GGRS sync-test session resimulates every
/// frame from a restored snapshot and compares checksums, so a component that
/// failed to round-trip — or an encoder that dropped `ticks` while keeping
/// `seconds` — panics inside the engine before this test's own assertions run.
/// What the assertions add is the part a checksum cannot see: that the state was
/// non-trivial, and that it landed on the same value the fixed-tick host reached.
#[test]
fn consumer_owned_authoritative_state_survives_real_resimulation() {
    let mut app = outlander::build_outlander_rollback_app()
        .unwrap_or_else(|error| panic!("the Outlander rollback host failed to start: {error}"));
    let rollback = outlander::run_outlander_walkthrough(&mut app).unwrap_or_else(|error| {
        panic!("the Outlander walkthrough failed under the rollback host: {error}")
    });

    assert!(
        rollback.beacon.ticks > 0 && rollback.beacon.is_full(),
        "the beacon never charged under the rollback host, so the resimulation \
         compared a component that was `default()` on every frame and the \
         checksum agreement is vacuous: {:?}",
        rollback.beacon
    );
    assert!(
        rollback.player_pos.y < 300.0,
        "the gate never fired under the rollback host, so nothing downstream of \
         the rewound state was exercised, got {:?}",
        rollback.player_pos
    );

    // Same content, same input, two hosts. The fixed-tick run is the reference
    // timeline; a rollback host that resimulates correctly has to reproduce it
    // exactly, and `ticks` is an integer so "exactly" is literal.
    let mut fixed = outlander::build_outlander_app();
    let reference = outlander::run_outlander_walkthrough(&mut fixed)
        .unwrap_or_else(|error| panic!("the fixed-tick reference walk failed: {error}"));
    assert_eq!(
        rollback.beacon, reference.beacon,
        "the rollback host reached a different authoritative state than the \
         fixed-tick host from the same content and the same inputs"
    );
    assert_eq!(
        rollback.ticks_to_gate, reference.ticks_to_gate,
        "the two hosts opened the gate on different ticks, so one of them is not \
         running the timeline the other is"
    );
}
