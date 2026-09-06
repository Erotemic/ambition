//! THE SCHEDULE THIS CRATE OWNS, BY SHAPE (D33 rule,
//! `actor-monolith-decomposition.md`): the carved plugin ALONE on a bare
//! `App`, so the kernel's `configure_sets` cannot supply an edge this crate
//! failed to declare. Membership and order are counted, never named — Bevy
//! hides system names without `bevy_ecs/debug`, and a name lookup passes or
//! fails by who else is in the build. Poison: delete `.in_set(PlayerSimulation)`
//! from `CoreHeldItems` and the phase test reads red; delete
//! `.in_set(HeldItemStep::Release)` from `return_released_items` and the
//! Release count reads 0.

use super::HeldItemSimulationPlugin;
use ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled;
use ambition_platformer2d_shared_tangle::schedule::{
    GameplayGated, HeldItemStep, ItemPickupSet, Platformer2dSimulationPhaseMonolith,
    SimScheduleExt as _,
};
use bevy::app::App;
use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

const STEPS: [HeldItemStep; 7] = [
    HeldItemStep::Release,
    HeldItemStep::Pickup,
    HeldItemStep::Use,
    HeldItemStep::Throw,
    HeldItemStep::Settle,
    HeldItemStep::Physics,
    HeldItemStep::Residency,
];

fn set_key<S: SystemSet + Copy + std::fmt::Debug>(graph: &ScheduleGraph, set: S) -> NodeId {
    NodeId::Set(
        graph
            .system_sets
            .get_key(set.intern())
            .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet")),
    )
}

fn direct_system_members<S: SystemSet + Copy + std::fmt::Debug>(
    graph: &ScheduleGraph,
    set: S,
) -> usize {
    let set_node = set_key(graph, set);
    graph
        .systems
        .iter()
        .filter(|(key, _, _)| {
            graph
                .hierarchy()
                .graph()
                .contains_edge(set_node, NodeId::System(*key))
        })
        .count()
}

fn with_graph(f: impl FnOnce(&ScheduleGraph)) {
    let mut app = App::new();
    app.add_plugins(HeldItemSimulationPlugin);
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    f(schedules.get(sim).expect("the sim schedule exists").graph());
}

#[test]
fn the_set_this_crate_owns_is_inside_the_phase_and_after_custody() {
    with_graph(|graph| {
        let core = set_key(graph, ItemPickupSet::CoreHeldItems);
        assert!(
            graph.hierarchy().graph().contains_edge(
                set_key(graph, Platformer2dSimulationPhaseMonolith::PlayerSimulation),
                core
            ),
            "CoreHeldItems must be inside PlayerSimulation — outside the phase it runs on a \
             tick the simulation did not advance (the defect ambition_world_items shipped with)"
        );
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(set_key(graph, BodyCustodySettled), core),
            "CoreHeldItems must run after custody settles"
        );
        for step in STEPS {
            assert!(
                graph.hierarchy().graph().contains_edge(core, set_key(graph, step)),
                "{step:?} must be inside CoreHeldItems"
            );
        }
    });
}

#[test]
fn the_steps_are_a_chain_and_each_holds_exactly_its_systems() {
    with_graph(|graph| {
        for pair in STEPS.windows(2) {
            assert!(
                graph
                    .dependency()
                    .graph()
                    .contains_edge(set_key(graph, pair[0]), set_key(graph, pair[1])),
                "{:?} -> {:?} must be an explicit edge",
                pair[0],
                pair[1]
            );
        }
        // One system per step, three in Residency (residency, the whereabouts
        // ledger, the authored-occurrence projection); the portal arming, when
        // the feature is on, is a direct member of CoreHeldItems and of no step.
        let expected = [1usize, 1, 1, 1, 1, 1, 3];
        for (step, expected) in STEPS.iter().zip(expected) {
            assert_eq!(
                direct_system_members(graph, *step),
                expected,
                "{step:?} holds exactly {expected} system(s)"
            );
        }
        let total: usize = expected.iter().sum::<usize>() + usize::from(cfg!(feature = "portal"));
        assert_eq!(
            graph.systems.iter().count(),
            total,
            "this plugin adds exactly the domain's systems and nothing else"
        );
        // Every gated step's system is gated; residency is deliberately not.
        let gated = set_key(graph, GameplayGated);
        for step in &STEPS[..6] {
            let node = set_key(graph, *step);
            let members: Vec<_> = graph
                .systems
                .iter()
                .filter(|(key, _, _)| {
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(node, NodeId::System(*key))
                })
                .collect();
            for (key, _, _) in members {
                assert!(
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(gated, NodeId::System(key)),
                    "{step:?}'s system must be gated on gameplay being live"
                );
            }
        }
    });
}
