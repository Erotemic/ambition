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
            let _ = built.initialize(world);
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
