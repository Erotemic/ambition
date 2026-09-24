//! The schedule this crate owns, checked by shape (D33,
//! `actor-monolith-decomposition.md`): the plugin alone on a bare `App`, so
//! the kernel's `configure_sets` cannot supply a missing edge. Members are
//! counted, not named: Bevy 0.19 hides system names without
//! `bevy_ecs/debug`.
//!
//! Absence is checked too. This crate must not configure `CoreHeldItems`
//! (owned by `ambition_held_items`); `the_chain_is_not_ours` fails if the
//! three-set chain is added here.
//!
//! Verified poisons: removing `.in_set(PlayerSimulation)` fails the phase
//! test; removing `ranged::meteor::fire_meteor_system` from the wielded tuple
//! makes the count 12.

use super::AbilitySimulationPlugin;
use ambition_platformer2d_shared_tangle::schedule::{
    ItemPickupSet, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
};
use bevy::app::App;
use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

/// The two sets' member counts, as the kernel registered them before the
/// split. The move had to keep these exactly.
const THROWN_MEMBERS: usize = 5;
const WIELDED_MEMBERS: usize = 13;

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
    app.add_plugins(AbilitySimulationPlugin);
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    f(schedules.get(sim).expect("the sim schedule exists").graph());
}

#[test]
fn both_sets_this_crate_owns_are_inside_the_player_phase() {
    with_graph(|graph| {
        let phase = set_key(graph, Platformer2dSimulationPhaseMonolith::PlayerSimulation);
        for set in [
            ItemPickupSet::ThrownItemEffects,
            ItemPickupSet::WieldedAbilities,
        ] {
            assert!(
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(phase, set_key(graph, set)),
                "{set:?} must be inside PlayerSimulation — outside the phase it \
                 runs off the rollback clock, and nothing else here would say so"
            );
        }
    });
}

#[test]
fn every_member_moved_and_none_was_left_behind() {
    with_graph(|graph| {
        assert_eq!(
            direct_system_members(graph, ItemPickupSet::ThrownItemEffects),
            THROWN_MEMBERS,
            "the thrown-effects group lost or gained a system in the carve"
        );
        assert_eq!(
            direct_system_members(graph, ItemPickupSet::WieldedAbilities),
            WIELDED_MEMBERS,
            "the wielded-abilities group lost or gained a system in the carve"
        );
    });
}

#[test]
fn the_chain_is_not_ours() {
    // `CoreHeldItems` belongs to `ambition_held_items`, and the three-set
    // chain is the kernel's edge (it orders sets from two other crates). This
    // plugin alone must leave `CoreHeldItems` out of the graph entirely. The
    // check is absence, not emptiness: a set no plugin names is not in the
    // graph, and a lookup would panic.
    with_graph(|graph| {
        assert!(
            graph
                .system_sets
                .get_key(ItemPickupSet::CoreHeldItems.intern())
                .is_none(),
            "this crate brought `CoreHeldItems` into the graph — it belongs to \
             `ambition_held_items`, and naming it here is either a member this \
             crate does not own or the kernel's three-variant chain, which \
             orders sets owned by two OTHER crates and must stay in the kernel"
        );
    });
}
