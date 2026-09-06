//! The far-side portal compositor, in the SHIPPED composition.
//!
//! ⛔⛔ EVERY OTHER TEST OF THIS FEATURE IS CRATE-LEVEL, AND CRATE-LEVEL WAS
//! FULLY GREEN WHILE THREE PRODUCTION DEFECTS SHIPPED (2026-09-05, found by
//! review): the publisher's query excluded `PlayerVisual` so the player was
//! never a candidate; the bounds ignored the feet anchor; and publication had no
//! ordering edge to the animator or to `PortalPresentationSet`.
//!
//! ⭐ Those were all defects INSIDE systems that were registered. This asks the
//! prior question, which no crate test can: does the shipped app register them at
//! all? A crate test wires the systems itself, so it proves the systems work and
//! is blind to a composition that never adds them.
//!
//! ⚠ It reads a BUILT, NEVER-RUN app's schedule graph, the same way
//! `the_gameplay_gate_is_carried_by_the_set` does — running first drains the
//! graphs and every count below would be zero for a reason unrelated to portals.

/// The two halves of the far-side repair, by the names the graph carries.
const PUBLISHER: &str = "publish_portal_compositing_candidates";
const COMPOSITOR: &str = "composite_far_side_bodies";

#[test]
fn the_shipped_app_registers_both_halves_of_the_far_side_compositor() {
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app
        .world()
        .get_resource::<bevy::ecs::schedule::Schedules>()
        .expect("a built app has schedules");

    let mut publisher = 0usize;
    let mut compositor = 0usize;
    let mut total_systems = 0usize;
    for (_label, schedule) in schedules.iter() {
        let graph = schedule.graph();
        for (_key, system, _conditions) in graph.systems.iter() {
            total_systems += 1;
            let name = system.name().to_string();
            if name.contains(PUBLISHER) {
                publisher += 1;
            }
            if name.contains(COMPOSITOR) {
                compositor += 1;
            }
        }
    }

    // ⛔⛔ THE PREMISE, FIRST. If the graphs were drained or the names are not
    // carried, both counts are zero for a reason that has nothing to do with
    // portals, and a "both missing" failure would be a lie about the app.
    assert!(
        total_systems > 100,
        "only {total_systems} systems visible in the shipped graph — the premise \
         of this test (readable, undrained schedules) does not hold, so its \
         verdict about portals would mean nothing"
    );

    assert_eq!(
        publisher, 1,
        "the shipped app registers `{PUBLISHER}` {publisher} times; without it \
         NOTHING is ever a compositing candidate and every far-side body punches \
         through every pane, with the compositor working perfectly on an empty set"
    );
    assert_eq!(
        compositor, 1,
        "the shipped app registers `{COMPOSITOR}` {compositor} times; candidates \
         are published and nothing consumes them"
    );
}
