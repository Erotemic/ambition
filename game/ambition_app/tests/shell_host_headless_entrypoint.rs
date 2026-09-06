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
    assert_eq!(report.ticks_run, ticks);
    assert_eq!(report.active_route.as_deref(), Some("ambition_launcher"));
    assert!(report.launcher_active);
    assert!(!report.gameplay_session_active);
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
