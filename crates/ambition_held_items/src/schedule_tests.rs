//! Check the domain schedule phase, custody order, and gameplay gating.

use super::HeldItemSimulationPlugin;
use ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled;
use ambition_platformer2d_shared_tangle::schedule::{
    GameplayGated, HeldItemStep, ItemPickupSet, Platformer2dSimulationPhase,
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
                set_key(graph, Platformer2dSimulationPhase::PlayerSimulation),
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
fn the_steps_are_a_chain_and_simulation_steps_are_gated() {
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
