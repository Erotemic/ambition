//! Shipping-entrypoint headless composition proof.
//!
//! The no-display fallback must step the same shared host as the windowed
//! binary, not the legacy direct sandbox. Deterministic frame time lets the
//! real startup card auto-advance to the provider-derived launcher.

use ambition_app::app::{run_shared_host_headless, shared_host_startup_ticks};

#[test]
fn shipping_shared_host_reaches_the_launcher_without_a_window() {
    // Budget derived from the composed run-in, not hardcoded: the sequence has
    // already grown from one card to two, and a stale constant turns this into
    // "asserts the host is still showing card one" instead of failing.
    let ticks = shared_host_startup_ticks();
    let report = run_shared_host_headless(ticks);
    assert_eq!(report.outer_fixed_steps, ticks);
    assert_eq!(report.active_route.as_deref(), Some("ambition_launcher"));
    assert!(report.launcher_active);
    assert!(!report.gameplay_session_active);

    // ⛔⛤ **AND THE SIMULATION RAN ZERO TIMES, WHICH IS THE POISON THE GPT
    // REVIEW OF 2026-09-16 ASKED FOR AND WHICH THIS RUN SUPPLIES FOR FREE.**
    // The field was called `ticks_run` and counted Bevy `FixedUpdate`, while
    // this composition is `SimulationHost::Rollback` and the rollback backend
    // advances `GgrsSchedule` from `PreUpdate` — different clocks, with no
    // invariant equating them. The review's suggested poison is "permit Bevy
    // `FixedUpdate` while preventing the GGRS session from advancing; the report
    // must not claim simulation ticks occurred", and a launcher-idle run IS that
    // state: no gameplay session exists, so nothing simulates however many outer
    // frames pass.
    //
    // ⇒ The old single number reported {ticks} ticks for this run. Anything
    // reading it as "the simulation ran" was reading a host frame count.
    assert_eq!(
        report.simulation_advances, 0,
        "a launcher-idle shared-host run advanced the simulation {} time(s) over \
         {} outer fixed steps. If that is intended — a session now exists on the \
         launcher — this assertion is the thing to update, but the two counts \
         must stay SEPARATE: conflating them is what made the old `ticks_run` \
         claim simulation where there was none.",
        report.simulation_advances, report.outer_fixed_steps
    );
}

#[test]
fn shipping_shared_host_executes_the_full_multi_provider_acceptance_cycle() {
    let report = ambition_app::app::run_shared_host_acceptance_cycle();
    assert!(report.completed, "{report}");
    assert_eq!(report.title_zero_state_stops, 5);
    assert!(report.exit_requested);
    assert_eq!(
        report.route_stops,
        vec![
            "ambition_launcher",
            "ambition_gameplay",
            "ambition_launcher",
            "sanic_gameplay",
            "ambition_launcher",
            "mary_o_gameplay",
            "ambition_launcher",
            "sanic_gameplay",
            "ambition_launcher",
        ]
    );
}

/// ⭐⭐ **THE POSITIVE CONTROL FOR THE ZERO ABOVE, AND WITHOUT IT THAT ZERO IS
/// UNINTERPRETABLE.** `simulation_advances == 0` on a launcher-idle run is what a
/// correct instrument reports AND what a counter that was never installed
/// reports. This drives the same runner into a gameplay ROOM, where a rollback
/// session exists, and requires the count to move.
///
/// ⛔ It is reachable at all only because the gameplay room became an ARGUMENT.
/// It was read from `AMBITION_HEADLESS_GAMEPLAY_ROOM` inside the function body,
/// so the only way to take this road was to mutate process environment — which a
/// parallel test binary cannot safely do, and which is why the field spent its
/// life with no control. One road, stated at the call site;
/// `run_shared_host_headless` reads the variable once and hands it over.
#[test]
fn a_gameplay_room_run_actually_advances_the_simulation() {
    let report =
        ambition_app::app::run_shared_host_headless_in_room(Some("hall_of_characters".into()), 30);
    assert_eq!(
        report.active_route.as_deref(),
        Some("ambition_gameplay"),
        "the run never reached the gameplay route, so the counts below are about \
         the launcher: {report}"
    );
    assert!(
        report.gameplay_session_active,
        "the gameplay route is active but no session exists, so there is nothing \
         for the rollback host to advance: {report}"
    );
    assert!(
        report.simulation_advances > 0,
        "a run that reached a gameplay room with a live session advanced the \
         simulation ZERO times. ⇒ THE COUNTER IS NOT WIRED, which makes the \
         `simulation_advances == 0` asserted for the launcher-idle run a \
         statement about this instrument rather than about that run: {report}"
    );
    // ⚠ NOT asserted equal to `outer_fixed_steps`, and the inequality is the
    // whole point of splitting the two. A rollback host advances `GgrsSchedule`
    // from `PreUpdate` under its own synchronisation, so zero, one or several
    // advances per outer frame are all legal. Pinning a ratio here would
    // re-assert the invariant the GPT review of 2026-09-16 pointed out does not
    // exist.
    eprintln!("gameplay-room run: {report}");
}
