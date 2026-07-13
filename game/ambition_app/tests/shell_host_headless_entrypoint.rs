//! Shipping-entrypoint headless composition proof.
//!
//! The no-display fallback must step the same shared host as the windowed
//! binary, not the legacy direct sandbox. Deterministic frame time lets the
//! real startup card auto-advance to the provider-derived launcher.

use ambition_app::app::run_shared_host_headless;

#[test]
fn shipping_shared_host_reaches_the_launcher_without_a_window() {
    let report = run_shared_host_headless(150);
    assert_eq!(report.ticks_run, 150);
    assert_eq!(report.active_route.as_deref(), Some("ambition_launcher"));
    assert!(report.launcher_active);
    assert!(!report.gameplay_session_active);
}
