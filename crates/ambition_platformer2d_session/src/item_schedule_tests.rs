#[cfg(test)]
mod held_item_steps {
    //! Custody settlement and held-item steps share the simulation phase.
    use ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled;
    use ambition_platformer2d_shared_tangle::schedule::{
        HeldItemStep, ItemPickupSet, Platformer2dSimulationPhase, SimScheduleExt as _,
    };
    use bevy::app::App;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

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
        app.add_plugins((
            ambition_held_items::HeldItemSimulationPlugin,
            ambition_platformer2d_items::pickup::ItemPickupSimulationPlugin,
        ));
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        f(schedules.get(sim).expect("the sim schedule exists").graph());
    }

    #[test]
    fn held_item_steps_follow_custody_settlement_in_the_simulation_phase() {
        with_graph(|graph| {
            let steps = [
                HeldItemStep::Release,
                HeldItemStep::Pickup,
                HeldItemStep::Use,
                HeldItemStep::Throw,
                HeldItemStep::Settle,
                HeldItemStep::Physics,
                HeldItemStep::Residency,
            ];
            let core = set_key(graph, ItemPickupSet::CoreHeldItems);
            for pair in steps.windows(2) {
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
            for step in steps {
                assert!(
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(core, set_key(graph, step)),
                    "{step:?} must be inside CoreHeldItems"
                );
            }
            assert!(
                graph.hierarchy().graph().contains_edge(
                    set_key(graph, Platformer2dSimulationPhase::PlayerSimulation),
                    core
                ),
                "CoreHeldItems must be inside PlayerSimulation — outside the phase it runs on \
                 a tick the simulation did not advance"
            );
            assert!(
                graph
                    .dependency()
                    .graph()
                    .contains_edge(set_key(graph, BodyCustodySettled), core),
                "CoreHeldItems must run after custody settles"
            );
        });
    }
}
