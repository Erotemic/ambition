#[cfg(test)]
/// THE HELD-ITEM CHAIN, BY SHAPE (D33 rule, `actor-monolith-decomposition.md`).
///
/// The kernel's plugin alone, on a bare `App`: the seven `HeldItemStep`s are
/// a chain inside `CoreHeldItems`, `CoreHeldItems` is inside `PlayerSimulation`
/// and after `BodyCustodySettled`, each step holds exactly the systems it
/// should (counted, not named — Bevy hides names without `bevy_ecs/debug`,
/// and a name lookup passes or fails by who else is in the build), and the
/// two foreign systems that USED to be links of one chain attach to a step
/// by an explicit edge (the third, the puppy-slug gun, is a wielded ability in
/// `ambition_abilities` now). Poison: delete `.in_set(HeldItemStep::Release)` from
/// `return_released_items` and the Release count reads 0; delete the shrine
/// pair's `.before(HeldItemStep::Release)` and the edge count reads 0.
mod held_item_steps {
    //! The kernel's attachments to the carved domain's chain — the domain's
    //! own steps are pinned in `ambition_held_items::schedule_tests`; here the
    //! kernel's plugin is built beside the domain's, because the attachments
    //! name steps the domain configures.
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

    /// Systems that are direct members of `inside` AND carry a dependency edge
    /// `after -> system -> before` against the named steps (either may be
    /// `None` for "no edge required on that side").
    fn attached_between(
        graph: &ScheduleGraph,
        after: Option<HeldItemStep>,
        before: Option<HeldItemStep>,
    ) -> usize {
        let core = set_key(graph, ItemPickupSet::CoreHeldItems);
        graph
            .systems
            .iter()
            .filter(|(key, _, _)| {
                let node = NodeId::System(*key);
                graph.hierarchy().graph().contains_edge(core, node)
                    && after.is_none_or(|s| {
                        graph.dependency().graph().contains_edge(set_key(graph, s), node)
                    })
                    && before.is_none_or(|s| {
                        graph.dependency().graph().contains_edge(node, set_key(graph, s))
                    })
            })
            .count()
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
    fn held_item_steps_are_a_chain_and_the_attached_systems_sit_where_they_say() {
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
            // The domain's own links, one per step, and the residency triple.
            for (step, expected) in [
                (HeldItemStep::Release, 1),
                (HeldItemStep::Pickup, 1),
                (HeldItemStep::Use, 1),
                (HeldItemStep::Throw, 1),
                (HeldItemStep::Settle, 1),
                // The step and the roll easing (`right_ground_items`).
                (HeldItemStep::Physics, 2),
                (HeldItemStep::Residency, 3),
            ] {
                assert_eq!(
                    direct_system_members(graph, step),
                    expected,
                    "{step:?} holds exactly {expected} system(s)"
                );
            }
            // The attachments that were links of the old single chain: the
            // shrine heal before Release and the match spawn between Throw and
            // Settle. The gun that sat between Use and Throw is a wielded
            // ability now, registered by `ambition_abilities`.
            assert_eq!(
                attached_between(graph, None, Some(HeldItemStep::Release)),
                1,
                "the shrine heal runs before any hand changes"
            );
            assert_eq!(
                attached_between(graph, Some(HeldItemStep::Throw), Some(HeldItemStep::Settle)),
                1,
                "the match spawn drops before the physics that settles it"
            );
        });
    }
}
