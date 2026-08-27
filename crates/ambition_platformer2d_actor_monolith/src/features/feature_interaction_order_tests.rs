//! The `FeatureInteraction` cross-domain order, asserted as the app composes it.
//!
//! This phase was ONE anonymous `.chain()` of ten systems spanning four domains
//! (`conversation`, the interaction features, the NPC cast, `encounter`), and
//! every interleave in it was load-bearing. The order is now
//! [`FeatureInteractionSet`], which is what lets `conversation` register its own
//! systems without naming anything in `features` — but a vocabulary only helps
//! if the edges are really there.
//!
//! these read the SCHEDULE GRAPH the real plugin built, not a list retyped
//! from the plugin. A test that hand-lists a chain pins the function, not the
//! wiring: if it asserted "these ten systems appear in this order" by observing
//! an execution, an unordered pair could satisfy it by luck of the topological
//! sort. `.chain()` between sets materialises directed edges in
//! `Schedule::graph().dependency()` eagerly, at `configure_sets` time, so each
//! edge can be asserted individually and each one FAILS the moment it is
//! dropped — regardless of executor or tie-break.

use bevy::ecs::schedule::{NodeId, Schedules, SystemKey, SystemSet as _};
use bevy::prelude::App;

use ambition_platformer2d_shared_tangle::schedule::FeatureInteractionSet;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

/// The phase order, head to tail. Each boundary's justification lives on the
/// [`FeatureInteractionSet`] variant it separates.
const PHASES: [FeatureInteractionSet; 7] = [
    FeatureInteractionSet::NarrativeIntake,
    FeatureInteractionSet::Actuate,
    FeatureInteractionSet::Continuity,
    FeatureInteractionSet::CutBarkCast,
    FeatureInteractionSet::HoldProjection,
    FeatureInteractionSet::WorldObjects,
    FeatureInteractionSet::SwitchIndex,
];

/// Every system the phase schedules, and the phase it must belong to. Four
/// domains: `conversation`, `features::ecs`, `features::npcs`, `encounter`.
const MEMBERSHIP: [(&str, FeatureInteractionSet); 10] = [
    (
        "close_conversation_on_narrative_end",
        FeatureInteractionSet::NarrativeIntake,
    ),
    (
        "interact_ecs_actors_and_switches",
        FeatureInteractionSet::Actuate,
    ),
    (
        "break_dialogue_on_hit_or_separation",
        FeatureInteractionSet::Continuity,
    ),
    (
        "speak_conversation_cut_barks",
        FeatureInteractionSet::CutBarkCast,
    ),
    (
        "project_conversation_hold",
        FeatureInteractionSet::HoldProjection,
    ),
    ("open_ecs_chests", FeatureInteractionSet::WorldObjects),
    ("update_ecs_breakables", FeatureInteractionSet::WorldObjects),
    (
        "update_ecs_falling_chests",
        FeatureInteractionSet::WorldObjects,
    ),
    (
        "sync_ecs_switches_from_save",
        FeatureInteractionSet::WorldObjects,
    ),
    (
        "rebuild_encounter_switch_index",
        FeatureInteractionSet::SwitchIndex,
    ),
];

/// An `App` carrying exactly the composition under test: the real plugin, doing
/// its real `build`.
fn composed_app() -> App {
    let mut app = App::new();
    app.add_plugins(super::FeatureInteractionSchedulePlugin);
    app
}

/// Assert every consecutive phase edge; `(A, B).before(C)` would not order
/// `A` relative to `B` and therefore would not prove the required total order.
#[test]
fn the_feature_interaction_phases_are_chained_head_to_tail() {
    let mut app = composed_app();
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let schedule = schedules
        .get(sim)
        .expect("FeatureInteractionSchedulePlugin must have created the sim schedule");
    let graph = schedule.graph();

    for pair in PHASES.windows(2) {
        let (before, after) = (pair[0], pair[1]);
        let before_key = graph
            .system_sets
            .get_key(before.intern())
            .unwrap_or_else(|| panic!("{before:?} must be a registered SystemSet"));
        let after_key = graph
            .system_sets
            .get_key(after.intern())
            .unwrap_or_else(|| panic!("{after:?} must be a registered SystemSet"));
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(before_key), NodeId::Set(after_key)),
            "the sim schedule must carry a dependency edge {before:?} -> {after:?}. \
             Without it the cross-domain contract that boundary records is unstated: \
             see the doc on FeatureInteractionSet::{after:?} for what breaks."
        );
    }
}

/// Every phase is nested inside
/// [`Platformer2dSimulationPhaseMonolith::FeatureInteraction`].
///
/// The chain above orders the phases relative to each other; this is what keeps
/// the whole group where the rest of the frame expects it. A phase that fell out
/// of the containing set would still be internally ordered and would run at an
/// arbitrary point in the sim frame.
#[test]
fn every_feature_interaction_phase_is_nested_in_the_containing_set() {
    let mut app = composed_app();
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let schedule = schedules.get(sim).expect("sim schedule must exist");
    let graph = schedule.graph();

    let parent_key = graph
        .system_sets
        .get_key(Platformer2dSimulationPhaseMonolith::FeatureInteraction.intern())
        .expect("FeatureInteraction must be a registered SystemSet");
    for phase in PHASES {
        let child_key = graph
            .system_sets
            .get_key(phase.intern())
            .unwrap_or_else(|| panic!("{phase:?} must be a registered SystemSet"));
        assert!(
            graph
                .hierarchy()
                .graph()
                .contains_edge(NodeId::Set(parent_key), NodeId::Set(child_key)),
            "{phase:?} must be nested inside \
             Platformer2dSimulationPhaseMonolith::FeatureInteraction"
        );
    }
}

/// Each of the ten systems is a member of the phase whose doc explains why it
/// runs there.
///
/// The chain is only a real ordering if the members are where they claim to be:
/// a system that quietly left its set would keep the edges above green while
/// running anywhere in the phase. this is also the assertion that catches the
/// carve going wrong — `conversation` places three of these from its OWN plugin
/// now, and `features` places the other seven.
#[test]
fn every_feature_interaction_system_is_in_its_named_phase() {
    let mut app = composed_app();
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let schedule = schedules.get(sim).expect("sim schedule must exist");
    let graph = schedule.graph();

    // Resolve each expected system to its graph key by name. `DebugName`
    // renders a full path, so match the leaf.
    let key_of = |leaf: &str| -> SystemKey {
        let mut found: Option<SystemKey> = None;
        for (key, system, _) in graph.systems.iter() {
            let name = format!("{}", system.name());
            if name.rsplit("::").next() == Some(leaf) {
                assert!(
                    found.is_none(),
                    "{leaf} resolved to more than one system in the sim schedule; \
                     the membership assertion below would be ambiguous"
                );
                found = Some(key);
            }
        }
        found.unwrap_or_else(|| {
            panic!("{leaf} must be scheduled by FeatureInteractionSchedulePlugin")
        })
    };

    for (leaf, phase) in MEMBERSHIP {
        let system_key = key_of(leaf);
        let set_key = graph
            .system_sets
            .get_key(phase.intern())
            .unwrap_or_else(|| panic!("{phase:?} must be a registered SystemSet"));
        assert!(
            graph
                .hierarchy()
                .graph()
                .contains_edge(NodeId::Set(set_key), NodeId::System(system_key)),
            "{leaf} must be in FeatureInteractionSet::{phase:?}; \
             that variant's doc is the reason it runs there"
        );
    }
}

/// the plugin schedules NOTHING into the phase outside a named set.
///
/// The whole point of the vocabulary is that no interleave is positional any
/// more. A system added straight to
/// [`Platformer2dSimulationPhaseMonolith::FeatureInteraction`] would be ordered
/// against nothing in particular and would re-introduce exactly the "documented
/// only in prose at the call site" state this replaced — and it would do so
/// silently, because the chain assertions above would stay green.
#[test]
fn no_system_sits_in_the_phase_without_a_named_set() {
    let mut app = composed_app();
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let schedule = schedules.get(sim).expect("sim schedule must exist");
    let graph = schedule.graph();

    let parent_key = graph
        .system_sets
        .get_key(Platformer2dSimulationPhaseMonolith::FeatureInteraction.intern())
        .expect("FeatureInteraction must be a registered SystemSet");
    let direct_systems: Vec<String> = graph
        .hierarchy()
        .graph()
        .neighbors_directed(NodeId::Set(parent_key), bevy::ecs::schedule::graph::Direction::Outgoing)
        .filter_map(|node| match node {
            NodeId::System(key) => graph
                .systems
                .get(key)
                .map(|system| format!("{}", system.system.name())),
            NodeId::Set(_) => None,
        })
        .collect();
    assert!(
        direct_systems.is_empty(),
        "these systems are in FeatureInteraction but in no FeatureInteractionSet \
         phase, so their position in the phase is unstated: {direct_systems:?}"
    );
}
