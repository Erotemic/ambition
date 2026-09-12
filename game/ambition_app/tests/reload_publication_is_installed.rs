//! The reload transaction's PUBLICATION half is installed in the SHIPPED
//! schedule — asked of the schedule graph, not of the source.
//!
//! ⛔⛔ **A REGISTRATION IS THE MECHANISM; A DOC COMMENT SAYING IT IS REGISTERED
//! IS NOT EVIDENCE.** `reload::request_reload` STAGES a cast revision and asks
//! the shell to re-prepare; `publish_staged_reload_on_activation` is what lets
//! that revision through when the route activates and what DISCARDS it when the
//! preparation fails instead. Without the `add_systems` line the request road
//! stages edits that nothing ever publishes and nothing ever throws away — a
//! half-transaction with no second half.
//!
//! ⚠ **AND THE CRATE-LEVEL ARMS CANNOT WITNESS THIS.** Every one of them adds the
//! system itself, so all of them stay green if `AmbitionContentPlugin` registers
//! nothing. My first attempt at this test did the same thing one layer up and
//! was therefore vacuous; only the real `Update` schedule can answer it.

use bevy::ecs::schedule::{ScheduleGraph, Schedules};
use bevy::prelude::*;

/// ⛔ BY TYPE, NOT BY NAME: system names ride `bevy_utils/debug`, which only an
/// optional GUI crate turns on, so in this build every name is empty and a name
/// lookup finds nothing. (The sibling schedule test learned this the hard way.)
fn is_registered<M>(graph: &ScheduleGraph, f: impl IntoSystem<(), (), M>) -> usize {
    let wanted = IntoSystem::into_system(f).system_type();
    graph
        .systems
        .iter()
        .filter(|(_, system, _)| system.system_type() == wanted)
        .count()
}

#[test]
fn the_shipped_app_installs_the_reload_publication_system() {
    // ⚠ NOT UPDATED. Running the schedule moves its systems out of the graph
    // into the executor, and the graph then answers "no systems".
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the Update schedule exists")
        .graph();

    // ⛔ THE FLOOR. A graph that answered zero for everything would make the
    // assertion below fail for a reason that has nothing to do with the reload.
    assert!(
        graph.systems.len() > 100,
        "only {} systems in the shipped Update schedule — the graph is not the \
         shipped one, so this test is about nothing",
        graph.systems.len()
    );

    let count = is_registered(
        graph,
        ambition_content::reload::publish_staged_reload_on_activation,
    );
    assert_eq!(
        count, 1,
        "`publish_staged_reload_on_activation` is registered {count} time(s) in \
         the shipped Update schedule; it must be exactly once, or a requested \
         reload stages a cast revision that nothing publishes and nothing \
         discards"
    );
}
