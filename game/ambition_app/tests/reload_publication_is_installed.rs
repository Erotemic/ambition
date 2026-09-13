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

    for (name, count) in [
        (
            "adopt_preparation_transaction",
            is_registered(graph, ambition_content::reload::adopt_preparation_transaction),
        ),
        (
            "commit_content_generation",
            is_registered(graph, ambition_content::reload::commit_content_generation),
        ),
    ] {
        assert_eq!(
            count, 1,
            "`{name}` is registered {count} time(s) in the shipped Update \
             schedule; it must be exactly once, or a requested reload stages a \
             cast revision that nothing publishes and nothing discards"
        );
    }
}

/// ⛔⛔⛔ **THE COMMIT SITS BETWEEN THE ACTIVATION AND THE WORLD BUILT FROM IT —
/// AND THIS TEST USED TO ASSERT ONLY HALF OF THAT.**
///
/// ⛔⛤ **THE HALF IT ASSERTED WAS SATISFIED VACUOUSLY, AND A REVIEW CAUGHT IT.**
/// It required `publication → GameplaySessionSet::Providers` and got the edge —
/// because ONE system did both jobs and was ordered `.before(
/// PlatformerPreparationSet)`, which is `in_set(AmbitionLoadSet::Contributors)`,
/// so it ran FAR earlier than `Providers` and earlier than the activation too.
/// The shell chain is `Contributors → Commands → AmbitionGameShellSet::{Commands,
/// Pending}`, and `advance_pending_route` pushes `RouteActivated` in `Pending`.
///
/// ⇒ MEASURED 2026-09-12 by driving `build_visible_app` with nothing injected:
/// the activation landed on frame 2, the session provider built the new world on
/// frame 2, and the family published on frame **3**. An edge to a LATE set says
/// nothing about a message produced in a set BEFORE it.
///
/// ⭐⭐ **SO THE RELATIONSHIP IS `Pending → commit → Providers` AND BOTH ENDS ARE
/// ASSERTED.** Either one alone is satisfiable by a system that does nothing
/// useful at the time it runs.
///
/// ⚠ ASKED OF THE GRAPH, BY KEY. The provider's and router's systems are private
/// to their crates, so the edges are asserted against the SETS they belong to —
/// which is also the seam the ordering is actually written against.
#[test]
fn the_commit_sits_between_the_activation_and_the_world_built_from_it() {
    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the Update schedule exists")
        .graph();

    fn set_key<S: bevy::ecs::schedule::SystemSet>(
        graph: &ScheduleGraph,
        set: S,
        why: &str,
    ) -> bevy::ecs::schedule::SystemSetKey {
        graph
            .system_sets
            .get_key(bevy::ecs::schedule::SystemSet::intern(&set))
            .unwrap_or_else(|| panic!("{why}"))
    }

    let commit = key_of(graph, ambition_content::reload::commit_content_generation);
    let adopt = key_of(graph, ambition_content::reload::adopt_preparation_transaction);
    let dependencies = graph.dependency().graph();

    // ⛔ END ONE: the activation must already EXIST when the commit runs.
    let pending = set_key(
        graph,
        ambition_platformer2d::game_shell::AmbitionGameShellSet::Pending,
        "`AmbitionGameShellSet::Pending` is a set in the shipped Update schedule \
         — if it is not, `RouteActivated` is produced somewhere else and this \
         ordering names nothing",
    );
    assert!(
        dependencies.contains_edge(NodeId::Set(pending), NodeId::System(commit)),
        "the generation commit has no ordering edge AFTER \
         `AmbitionGameShellSet::Pending`, which is where `advance_pending_route` \
         produces `RouteActivated` — so the commit reads the activation a FRAME \
         LATE and the world is built from generation N"
    );

    // ⛔ END TWO: and the world must not be built until it has.
    let providers = set_key(
        graph,
        ambition_platformer2d::game_shell::GameplaySessionSet::Providers,
        "`GameplaySessionSet::Providers` is a set in the shipped Update schedule \
         — if it is not, the provider stopped constructing sessions there and \
         this ordering names nothing",
    );
    assert!(
        dependencies.contains_edge(NodeId::System(commit), NodeId::Set(providers)),
        "the generation commit has no ordering edge to \
         `GameplaySessionSet::Providers`, so a route activation may construct the \
         new session from the PREVIOUS cast"
    );

    // ⛔⛔ **AND THE ADOPTION HALF RUNS AT THE OTHER END, BEFORE THE PREPARATION
    // THAT READS ITS IDENTITY CLAIM.** It stakes `PendingGenerationInputs` from
    // `ShellEvent::PreparationRequested`, and `prepare_requested_sessions` reads
    // the SAME message and fingerprints against that claim. A preparation that
    // ran first would MISS the claim and fall back to the App's active identity
    // — a wrong answer rather than a missing one. ⇒ This is why the two jobs are
    // two systems: one edge wants the earliest end of the frame and the other
    // wants the latest, and no single system can hold both.
    let preparation = set_key(
        graph,
        ambition_platformer2d::provider::PlatformerPreparationSet,
        "`PlatformerPreparationSet` is a set in the shipped Update schedule — if \
         it is not, the provider stopped preparing sessions there and this \
         ordering names nothing",
    );
    assert!(
        dependencies.contains_edge(NodeId::System(adopt), NodeId::Set(preparation)),
        "the reload's identity claim has no ordering edge to \
         `PlatformerPreparationSet`, so a preparation may fingerprint the new \
         generation against the PREVIOUS content identity"
    );
}

/// ⛔⛤ **Q118: NOTHING ORDERS THE ROLLBACK SESSION START AGAINST THE GENERATION
/// COMMIT, SO THE PUBLICATION-LEGALITY INTERVAL CAN BE CROSSED.**
///
/// `admit_candidate` asks `publication_boundary` at REQUEST time and refuses a
/// live rollback timeline. The generation then waits in `PendingGeneration`
/// through shell preparation to `RouteActivated`, and
/// `commit_content_generation` asks nothing — deliberately: *"there is nothing
/// in it that can say no."* Two arms in `ambition_content`'s reload tests
/// measure what happens when a timeline goes live inside that window: the
/// generation publishes into a world the same check would have refused.
///
/// ⇒ **THIS IS THE OTHER HALF — WHETHER THE SHIPPED LIFECYCLE CAN PRODUCE IT.**
/// Those arms install the authority by hand. The road that would do it for real
/// is `local_session::maintain_local_session`, which runs every `Update` and
/// starts a GGRS session when gameplay becomes active — and a reload
/// re-prepares the route the shell is already on, so the session world is torn
/// down and rebuilt INSIDE the pending interval.
///
/// ⭐⭐ **WHAT THIS ASKS IS REACHABILITY IN THE DEPENDENCY GRAPH, NOT A DIRECT
/// EDGE**, because "ordered" is transitive and a direct-edge check would call an
/// ordered pair unordered. If there is no path either way, the two are
/// AMBIGUOUS — Bevy is free to run them in either order, and the interval
/// crossing is not merely possible but a race.
#[test]
fn nothing_orders_the_rollback_session_start_against_the_generation_commit() {
    use std::collections::HashSet;

    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the Update schedule exists")
        .graph();

    let commit = NodeId::System(key_of(
        graph,
        ambition_content::reload::commit_content_generation,
    ));
    let maintain = NodeId::Set(
        graph
            .system_sets
            .get_key(bevy::ecs::schedule::SystemSet::intern(
                &ambition_platformer2d::rollback::local_session::LocalSessionSet::Maintain,
            ))
            .expect(
                "`LocalSessionSet::Maintain` is a set in the shipped Update schedule — if it \
                 is not, the GGRS session no longer starts there and this measurement names \
                 nothing",
            ),
    );

    let dependencies = graph.dependency().graph();
    let reaches = |from: NodeId, to: NodeId| -> bool {
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut stack = vec![from];
        while let Some(node) = stack.pop() {
            if node == to {
                return true;
            }
            if !seen.insert(node) {
                continue;
            }
            stack.extend(dependencies.neighbors(node));
        }
        false
    };

    // ⛔⛔ **THE CONTROL, AND WITHOUT IT THE FINDING BELOW IS A CLAIM ABOUT MY
    // TRAVERSAL RATHER THAN ABOUT THE SCHEDULE.** "No path exists" and "my BFS
    // cannot find a path" are indistinguishable from the outside, so the same
    // traversal is first asked a question whose answer is already known: the
    // commit IS ordered after `AmbitionGameShellSet::Pending` — the arm above
    // asserts that edge directly.
    let pending_control = NodeId::Set(
        graph
            .system_sets
            .get_key(bevy::ecs::schedule::SystemSet::intern(
                &ambition_platformer2d::game_shell::AmbitionGameShellSet::Pending,
            ))
            .expect("`AmbitionGameShellSet::Pending` is a set in the shipped Update schedule"),
    );

    let commit_first = reaches(commit, maintain);
    let session_first = reaches(maintain, commit);

    // ⚠ THE PREMISE, so a graph that lost both nodes cannot read as "ambiguous".
    assert!(
        dependencies.contains_node(commit) && dependencies.contains_node(maintain),
        "one of the two nodes is not in the dependency graph at all, so neither \
         direction below means anything"
    );

    assert!(
        reaches(pending_control, commit),
        "the traversal cannot find the ordering edge this file already asserts \
         directly (`Pending` -> the commit), so its verdict about the rollback \
         session below is a finding about the traversal"
    );

    assert!(
        !(commit_first && session_first),
        "the graph claims both orders, which is a cycle rather than a measurement"
    );

    assert!(
        !commit_first && !session_first,
        "MEASURED GAP CLOSED? This arm records that NOTHING orders the GGRS \
         session start against the content-generation commit — so a timeline can \
         become live between a reload's admission and its publication, which is \
         the state `publication_boundary` refuses at request time. If an edge now \
         exists (commit-first: {commit_first}, session-first: {session_first}), \
         say which way and this arm becomes the assertion that the interval is \
         sealed. See `Q118`."
    );
}

/// ⛔⛤ **NOTHING ORDERS THE RETIRED SCOPE'S SWEEP AGAINST THE INCOMING SESSION'S
/// CONSTRUCTION, AND A CENSUS SAYS THAT COSTS A WHOLE ROOM.**
///
/// MEASURED 2026-09-13 across the whole `app_it` suite: SIX room-construction
/// transactions are refused, and the two largest are on the HOT-RELOAD road — 8
/// placements and **18, the entire contents of `central_hub_complex`**. The world
/// log gives the mechanism in two lines:
///
/// ```text
/// f16  session-end   activation=2 scope=0
/// f16  session-start activation=3 scope=1
///      room-refused central_hub_complex :: 18x Duplicated, 18x ReconstructedOldSurvived
/// ```
///
/// ⇒ A session handoff retires the old scope and starts the new one in the SAME
/// FRAME, and the incoming room's transaction captures its baseline while the
/// OUTGOING scope's placements are still live — so every one of them is a
/// duplicate of a root the new room is about to mint.
///
/// ⚠ **HARMLESS TODAY, WHICH IS WHY IT WAS INVISIBLE.** A refusal costs only the
/// `RoomLoaded` message and that message has no production reader
/// (`content_staging.rs`: *"`RoomLoaded` remains notification-only"*). ⛔ But
/// under A10's candidate bracket the same refusal DROPS THE WHOLE ROOM, so every
/// hot reload would produce an empty world — which is what makes A10's flag
/// unflippable, and it is NOT the gameplay ruling `Q124` asks for.
///
/// ⭐⭐ **THE READING THAT PREDICTS IT IS TWO `chain()`s THAT NEVER MEET.**
/// `SessionScopeSet` chains `Activate -> .. -> RetireAuthority -> Cleanup` (the
/// sweep is in `Cleanup`), and the shell chains
/// `Activate -> GameplaySessionSet::Providers -> ..`. Both are after `Activate`
/// and neither is ordered against the other. This arm asks the graph rather than
/// trusting that reading — the same question, and the same control, as the
/// arm above.
#[test]
fn nothing_orders_the_retired_scopes_sweep_against_the_incoming_sessions_construction() {
    use std::collections::HashSet;

    let app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules
        .get(Update)
        .expect("the Update schedule exists")
        .graph();

    macro_rules! set_node {
        ($set:expr, $what:literal) => {
            NodeId::Set(
                graph
                    .system_sets
                    .get_key(bevy::ecs::schedule::SystemSet::intern(&$set))
                    .expect(concat!(
                        "`",
                        $what,
                        "` is not a set in the shipped Update schedule, so this \
                         measurement names nothing"
                    )),
            )
        };
    }

    let cleanup = set_node!(
        ambition_platformer2d::platformer::lifecycle::SessionScopeSet::Cleanup,
        "SessionScopeSet::Cleanup"
    );
    let providers = set_node!(
        ambition_platformer2d::game_shell::GameplaySessionSet::Providers,
        "GameplaySessionSet::Providers"
    );
    // ⚠ THE CONTROL PAIR, and BOTH are edges some `chain()` writes down
    // literally — `Bridge -> Activate` in the shell, `Activate -> Presentation`
    // in `SessionScopePlugin` — so neither depends on the ordering under test.
    let bridge_control = set_node!(
        ambition_platformer2d::game_shell::GameplaySessionSet::Bridge,
        "GameplaySessionSet::Bridge"
    );
    let activate_control = set_node!(
        ambition_platformer2d::platformer::lifecycle::SessionScopeSet::Activate,
        "SessionScopeSet::Activate"
    );
    let presentation_control = set_node!(
        ambition_platformer2d::platformer::lifecycle::SessionScopeSet::Presentation,
        "SessionScopeSet::Presentation"
    );

    let dependencies = graph.dependency().graph();
    let reaches = |from: NodeId, to: NodeId| -> bool {
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut stack = vec![from];
        while let Some(node) = stack.pop() {
            if node == to {
                return true;
            }
            if !seen.insert(node) {
                continue;
            }
            stack.extend(dependencies.neighbors(node));
        }
        false
    };

    // ⚠ THE PREMISE, so a graph that lost a node cannot read as "ambiguous".
    assert!(
        dependencies.contains_node(cleanup) && dependencies.contains_node(providers),
        "one of the two sets is not in the dependency graph at all, so neither \
         direction below means anything"
    );

    // ⛔⛔ THE CONTROL. "No path exists" and "my traversal cannot find one" are
    // indistinguishable from the outside, so the same traversal is first asked a
    // question whose answer is known: both sets are chained AFTER `Activate`.
    assert!(
        reaches(bridge_control, activate_control)
            && reaches(activate_control, presentation_control),
        "the traversal cannot find the `Bridge -> Activate` and \
         `Activate -> Presentation` edges that two `chain()`s declare \
         literally, so its verdict below is a finding about the traversal"
    );

    let sweep_first = reaches(cleanup, providers);
    let build_first = reaches(providers, cleanup);

    assert!(
        !(sweep_first && build_first),
        "the graph claims both orders, which is a cycle rather than a measurement"
    );

    // ⛔⛤ **THE ANSWER WAS NOT THE ONE THE READING PREDICTED, AND THAT IS THE
    // FINDING.** I expected AMBIGUITY — two `chain()`s that never meet. The
    // graph said `build_first: true`: `Providers` is `.before(Presentation)`,
    // and `Presentation` chained ahead of `RetireAuthority -> Cleanup`, so the
    // incoming room was ordered BEFORE the dying scope's sweep **by
    // declaration**. Not a race. A rule.
    //
    // ⇒ `SessionScopeSet` now chains `RetireAuthority -> Cleanup -> Activate ->
    // Presentation`, and this arm is the assertion that it stays that way.
    assert!(
        sweep_first && !build_first,
        "A RETIRED SCOPE'S SWEEP MUST PRECEDE THE INCOMING SESSION'S ROOM \
         CONSTRUCTION (sweep-first: {sweep_first}, build-first: {build_first}). \
         A shell handoff retires and activates in ONE frame, so with the build \
         first the new room's transaction captures a baseline that still holds \
         the outgoing scope's placements and is refused wholesale — measured at \
         18 of 18 roots in `central_hub_complex`. If this reddened, something \
         re-ordered `SessionScopeSet` or moved `GameplaySessionSet::Providers` \
         out from under it."
    );
}
