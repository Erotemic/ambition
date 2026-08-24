//! Verify schedule-sensitive pins against the host-selected simulation schedule.
//!
//! `CoreSimulation` lives in `app.sim_schedule()`: `Update`, `FixedUpdate`, or
//! `GgrsSchedule` depending on the host. A pin installed in that selected
//! schedule is effective in every composition; a pin installed in a literal
//! schedule constrains only hosts whose simulation uses that schedule.

use bevy::ecs::schedule::{ScheduleLabel, Schedules, SystemSet};
use bevy::prelude::*;

use ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith as Phase;
use ambition_platformer2d::rollback::GgrsSchedule;

/// Number of systems in a set, initializing the schedule first because Bevy
/// builds its graph lazily.
fn systems_in(app: &mut App, schedule: impl ScheduleLabel, set: impl SystemSet) -> Option<usize> {
    let label = schedule.intern();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let built = schedules.get_mut(label)?;
            // ⛔ THE BUILD ERROR IS NOT SWALLOWED. A failed `initialize` leaves a
            // partial graph, and every query below then answers `None` — which
            // reads as "this set has no systems" when what happened is that the
            // schedule did not build at all.
            built
                .initialize(world)
                .unwrap_or_else(|e| panic!("the sim schedule failed to build: {e:?}"));
            built
                .graph()
                .systems_in_set(set.intern())
                .ok()
                .map(|systems| systems.len())
        })
}

fn shipped_app() -> App {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..4 {
        app.update();
    }
    app
}

/// In the shipped GGRS composition, `CoreSimulation` must contain systems in
/// `GgrsSchedule` while the literal-`Update` references create only an empty set
/// node there.
#[test]
fn the_shipped_apps_core_simulation_is_in_the_ggrs_schedule_and_update_holds_an_empty_husk() {
    let mut app = shipped_app();

    let in_ggrs = systems_in(&mut app, GgrsSchedule, Phase::CoreSimulation)
        .expect("the shipped app hosts its sim in GgrsSchedule, so the set has a node there");
    assert!(
        in_ggrs > 100,
        "the whole core simulation should be in GgrsSchedule; found only {in_ggrs} systems, \
         which means the composition changed shape rather than that this pin moved"
    );

    let in_update = systems_in(&mut app, Update, Phase::CoreSimulation);
    assert_eq!(
        in_update,
        Some(0),
        "`Update` should hold a CoreSimulation node with NO members — the husk that the \
         literal-`Update` `.before(CoreSimulation)` pins create by naming the set. Getting \
         `None` means even those pins are gone; getting a positive count means the sim moved \
         into `Update` and those pins JUST BECAME LOAD-BEARING — go read them, they were \
         written when they constrained nothing."
    );
}

/// The same fact from the other side, so neither statement can drift alone: the
/// sub-phases inside `CoreSimulation` are in the sim schedule too, and are not in
/// `Update` at all. A pin naming one of THOSE from `Update` would not even create
/// a husk to notice later.
#[test]
fn the_core_simulation_sub_phases_live_where_core_simulation_does() {
    let mut app = shipped_app();

    for phase in [Phase::PlayerInput, Phase::WorldPrep, Phase::Combat] {
        let ggrs = systems_in(&mut app, GgrsSchedule, phase);
        assert!(
            ggrs.is_some_and(|count| count > 0),
            "{phase:?} should own systems in the sim schedule; found {ggrs:?}"
        );
        assert_eq!(
            systems_in(&mut app, Update, phase),
            None,
            "{phase:?} should have no node in `Update` at all — nothing pins against it from \
             there, so unlike `CoreSimulation` there is not even an empty husk"
        );
    }
}

/// ⛔⛔ EVERY RESTRICTION OVER PUBLISHED CONTROL IS REGISTERED EXACTLY ONCE.
///
/// It was not. Control is published twice — a possessed body's in
/// `PlayerInputSet::Brain`, an autonomous body's in the actor decision chain a
/// phase later — and `PlayerInputSet::ControlGate` sat in `PlayerInput`, before
/// the second one. The answer had been a SECOND copy of each restriction later
/// in the frame, and the pair was correct only by an invariant nothing enforced:
/// the first blank was what stopped the second sampler crediting the same human
/// press. Delete either blank and a held human escapes at double rate, silently.
///
/// ⭐ THIS IS A COUNT, NOT A PRESENCE — the failure was two, not zero. And it
/// runs against the SHIPPED app, which is the only composition where both
/// registration sites exist: the monolith's own phase-membership test builds a
/// world with just `WorldPrepSchedulePlugin` and never saw the second copy.
#[test]
fn each_restriction_over_published_control_is_registered_exactly_once() {
    use bevy::ecs::schedule::Schedules;

    let mut app = shipped_app();
    let label = GgrsSchedule.intern();
    // ⛔ NOT INITIALIZED FIRST, deliberately. A built schedule MOVES its systems
    // out of the graph into the executable, so counting after `initialize` reads
    // zero for everything — which would look like "this system is gone" for the
    // whole list. The registration count is a fact about the graph as it was
    // ASSEMBLED, and that is what this asks for.
    let (total, counts) =
        app.world_mut()
            .resource_scope(|_world, mut schedules: Mut<Schedules>| {
                let built = schedules.get_mut(label).expect("the sim schedule exists");
                let graph = built.graph();
                let count_of = |leaf: &str| {
                    graph
                        .systems
                        .iter()
                        .filter(|(_, system, _)| {
                            format!("{}", system.name()).rsplit("::").next() == Some(leaf)
                        })
                        .count()
                };
                (
                    graph.systems.iter().count(),
                    [
                        "sample_capture_escape",
                        "blank_scripted_control_frames",
                        "gate_worn_player_control",
                        "sustain_bubble_shield",
                        "update_body_mode",
                    ]
                    .map(|leaf| (leaf, count_of(leaf))),
                )
            });
    assert!(
        total > 100,
        "the sim schedule's graph holds {total} systems, so it has already been \
         built and every count below would read zero for the wrong reason"
    );
    for (leaf, n) in counts {
        assert_eq!(
            n, 1,
            "`{leaf}` is registered {n} times in the shipped sim schedule. Two means \
             control is being restricted once per publication phase again, and the \
             copies are load-bearing on each other (D202); zero means it stopped \
             running at all"
        );
    }
}

/// ... AND THE ONE COPY RUNS AFTER BOTH PUBLICATIONS.
///
/// The count above is satisfied by a single copy in the WRONG place — back in
/// `PlayerInput`, gating the human frame and no AI frame in the world — which is
/// the state the second copy existed to paper over. So the placement is asserted
/// beside it: `ControlGate` is a child of `WorldPrep`, and NOT of `PlayerInput`,
/// whatever the enum it belongs to is called.
#[test]
fn the_one_control_gate_lives_after_the_later_publication() {
    use ambition_platformer2d::platformer::schedule::PlayerInputSet;

    let mut app = shipped_app();
    for (set, phase, expected) in [
        (PlayerInputSet::ControlGate, Phase::WorldPrep, true),
        (PlayerInputSet::ControlGate, Phase::PlayerInput, false),
        (PlayerInputSet::BodyMode, Phase::WorldPrep, true),
        (PlayerInputSet::BodyMode, Phase::PlayerInput, false),
        // The poison: the publication the gate is measured against did NOT move.
        (PlayerInputSet::Brain, Phase::PlayerInput, true),
    ] {
        let count = systems_in(&mut app, GgrsSchedule, set)
            .expect("the set is registered in the shipped app");
        assert!(count > 0, "{set:?} has no members at all");
        let in_phase = systems_in(&mut app, GgrsSchedule, phase).unwrap_or(0);
        assert!(in_phase > 0, "{phase:?} has no members at all");
        let nested = set_is_inside(&mut app, GgrsSchedule, set, phase);
        assert_eq!(
            nested, expected,
            "{set:?} inside {phase:?} should be {expected}"
        );
    }
}

/// Is `set` a descendant of `parent` in the schedule's hierarchy?
fn set_is_inside(
    app: &mut App,
    schedule: impl ScheduleLabel,
    set: impl SystemSet,
    parent: impl SystemSet,
) -> bool {
    use bevy::ecs::schedule::NodeId;

    let label = schedule.intern();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let built = schedules.get_mut(label).expect("the schedule exists");
            let _ = built.initialize(world);
            let graph = built.graph();
            let (Some(child), Some(parent)) = (
                graph.system_sets.get_key(set.intern()),
                graph.system_sets.get_key(parent.intern()),
            ) else {
                return false;
            };
            graph
                .hierarchy()
                .graph()
                .contains_edge(NodeId::Set(parent), NodeId::Set(child))
        })
}
