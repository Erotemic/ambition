//! THE SCHEDULE THIS CRATE OWNS, BY SHAPE (D33 rule,
//! `actor-monolith-decomposition.md`): the carved plugin ALONE on a bare `App`,
//! so the kernel's `configure_sets` cannot supply an edge this crate failed to
//! declare. Membership is COUNTED, never named — Bevy 0.19 hides system names
//! without `bevy_ecs/debug`, so a name lookup passes or fails by who else is in
//! the build rather than by what this plugin did.
//!
//! ⛔ AND THE ABSENCE IS ASSERTED TOO, which is the half a "does it register?"
//! test misses. This crate must NOT configure `CoreHeldItems` — that set belongs
//! to `ambition_held_items`, and two crates configuring one set is the exact
//! failure D33 exists to prevent. `the_chain_is_not_ours` is red the moment
//! somebody "helpfully" adds the three-variant chain here.
//!
//! Poison, both verified when written: delete `.in_set(PlayerSimulation)` and
//! the phase test reads red; delete `ranged::meteor::fire_meteor_system` from
//! the wielded tuple and the count reads 12.

use super::AbilitySimulationPlugin;
use ambition_platformer2d_shared_tangle::schedule::{
    ItemPickupSet, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
};
use bevy::app::App;
use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

/// The two variants' member counts, as the kernel registered them before the
/// carve. These are the numbers the move had to preserve exactly.
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
    // ⛔ `CoreHeldItems` is `ambition_held_items`'s set and the three-variant
    // chain is the KERNEL's edge, because it orders sets owned by two other
    // crates. This plugin alone must therefore leave `CoreHeldItems` with no
    // members and no ordering to our two — a composition that installs only
    // this crate gets the wielded half correctly nested and nothing else.
    // ⚠ THE ASSERTION IS ABSENCE, NOT EMPTINESS, and the first version of this
    // test got that wrong: it looked `CoreHeldItems` up and panicked in the
    // lookup, because a set no plugin has ever named is not IN the graph at all.
    // That failure was the right answer arriving through the wrong door — the
    // guard now states it directly.
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
