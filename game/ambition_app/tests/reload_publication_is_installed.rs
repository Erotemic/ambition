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

use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemKey};
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

fn key_of<M>(graph: &ScheduleGraph, f: impl IntoSystem<(), (), M>) -> SystemKey {
    let wanted = IntoSystem::into_system(f).system_type();
    graph
        .systems
        .iter()
        .find(|(_, system, _)| system.system_type() == wanted)
        .map(|(key, _, _)| key)
        .expect("the system is registered in this schedule")
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

/// ⛔⛔ **AND IT RUNS BEFORE THE WORLD IS BUILT FROM THE CAST IT PUBLISHES.**
///
/// `activate_prepared_platformer_sessions` sits in `GameplaySessionSet::
/// Providers` on this same schedule and builds the session's actors through
/// `PlatformerSessionBuilder`, which reads `PreparedCharacterRegistry`. The
/// shell's `RouteActivated` and the bridge's `GameplaySessionEvent::Activated`
/// are the same frame, so with no edge between them the activation that
/// AUTHORIZED a reload could construct generation N+1's world out of generation
/// N's moves — a half-transaction that no assertion on either side can see,
/// because each half is individually correct.
///
/// ⚠ ASKED OF THE GRAPH, BY KEY. The provider's system is private to its crate,
/// so the edge is asserted against the SET it belongs to — which is also the
/// seam the ordering is actually written against.
#[test]
fn the_publication_precedes_provider_session_construction() {
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the Update schedule exists")
        .graph();

    let publication = key_of(
        graph,
        ambition_content::reload::publish_staged_reload_on_activation,
    );
    let providers = graph
        .system_sets
        .get_key(bevy::ecs::schedule::SystemSet::intern(
            &ambition_platformer2d::game_shell::GameplaySessionSet::Providers,
        ))
        .expect(
            "`GameplaySessionSet::Providers` is a set in the shipped Update \
             schedule — if it is not, the provider stopped constructing sessions \
             there and this ordering names nothing",
        );

    assert!(
        graph
            .dependency()
            .graph()
            .contains_edge(NodeId::System(publication), NodeId::Set(providers)),
        "the reload publication has no ordering edge to \
         `GameplaySessionSet::Providers`, so a route activation may construct \
         the new session from the PREVIOUS cast"
    );

    // ⛔⛔ **AND BEFORE THE PREPARATION THAT READS ITS IDENTITY CLAIM.** The
    // ADOPTION half of the same system stakes `PendingContentIdentity` from
    // `ShellEvent::PreparationRequested`, and `prepare_requested_sessions` reads
    // the SAME message and fingerprints against that claim. A preparation that
    // ran first would MISS the claim and fall back to the App's active identity
    // — stamping generation N+1's session with N's content, which is a wrong
    // answer rather than a missing one.
    let preparation = graph
        .system_sets
        .get_key(bevy::ecs::schedule::SystemSet::intern(
            &ambition_platformer2d::provider::PlatformerPreparationSet,
        ))
        .expect(
            "`PlatformerPreparationSet` is a set in the shipped Update schedule \
             — if it is not, the provider stopped preparing sessions there and \
             this ordering names nothing",
        );
    assert!(
        graph
            .dependency()
            .graph()
            .contains_edge(NodeId::System(publication), NodeId::Set(preparation)),
        "the reload's identity claim has no ordering edge to \
         `PlatformerPreparationSet`, so a preparation may fingerprint the new \
         generation against the PREVIOUS content identity"
    );
}
