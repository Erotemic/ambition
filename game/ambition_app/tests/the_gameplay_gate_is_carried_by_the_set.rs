//! The mode gate lives on a SET now, and a set can lose its condition silently.
//!
//! ⛔⛔ THIS IS THE FAILURE THIS FILE EXISTS FOR. `gameplay_allowed` used to be
//! written on each of 83 systems, where forgetting it was a one-system bug you
//! could see in the diff. It is now carried by `GameplayGated`, configured in
//! ONE place — `configure_platformer2d_simulation_phases`. A composition that
//! registers those systems without calling that function gets a set with NO
//! condition, which compiles, links, starts, and runs every gameplay system at
//! a menu. There is no error to read; the whole gate just stops existing.

use bevy::prelude::*;

/// ⭐⭐ THE SHIPPED APP'S GATE, NOT THE CONFIGURATOR'S.
///
/// Calling `configure_platformer2d_simulation_phases` in the test and then
/// asserting it configured something would pin the FUNCTION and say nothing
/// about whether the app that ships ever calls it — which is exactly the hole.
/// So this builds the real app and reads the graph it actually assembled.
///
/// ⛔ IT MUST NOT PUMP A FRAME FIRST. `Schedule::initialize` moves every
/// condition out of the `ScheduleGraph` into a private executable, so one
/// `app.update()` before this read turns the assertion into `0 == 0` against a
/// drained graph. The read happens on a built, never-run app on purpose.
#[test]
fn the_gameplay_gate_is_carried_by_the_set() {
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);

    let schedules = app
        .world()
        .get_resource::<bevy::ecs::schedule::Schedules>()
        .expect("a built app has schedules");

    let mut found_in = Vec::new();
    let mut conditions_on_it = 0usize;
    let mut still_carried_per_system = 0usize;
    for (label, schedule) in schedules.iter() {
        let graph = schedule.graph();
        for (_key, set, conditions) in graph.system_sets.iter() {
            if format!("{set:?}").contains("GameplayGated") {
                found_in.push(format!("{label:?}"));
                conditions_on_it += conditions.len();
            }
        }
        // The premise: if the graphs were already drained, every count below is
        // zero for a reason that has nothing to do with the gate, and a green
        // arm would mean nothing.
        for (_key, _system, conditions) in graph.systems.iter() {
            for condition in conditions {
                if condition.condition.name().contains("gameplay_allowed") {
                    still_carried_per_system += 1;
                }
            }
        }
    }

    assert!(
        !found_in.is_empty(),
        "the shipped app registers no `GameplayGated` set at all, so nothing is \
         gated on the game mode"
    );
    assert!(
        conditions_on_it > 0,
        "`GameplayGated` exists in {found_in:?} but carries NO run condition. \
         Every system in it now runs at a menu, and nothing else in this build \
         would have said so"
    );

    // ⭐ THE OTHER HALF: the hoist is only a win if the per-system copies are
    // GONE. Four deliberate set-level attachments live in `portal_schedule.rs`
    // (`PortalSet::*.run_if(gameplay_allowed)`), which are the shape this change
    // moved everything else TO -- they are on SETS, so they are not counted
    // here, and this number is about systems.
    assert_eq!(
        still_carried_per_system, 0,
        "{still_carried_per_system} systems still carry `gameplay_allowed` \
         individually; each one is an evaluation per frame that the set already \
         answers"
    );
}
