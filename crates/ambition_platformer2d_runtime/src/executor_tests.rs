//! Every main-world schedule runs on the single-threaded executor.
//!
//! Bevy 0.19 does not say which executor a schedule has, so this reads what
//! the executor does: the single-threaded one runs each system on the thread
//! that runs the schedule, and the multi-threaded one gives systems that do not
//! conflict to the compute pool's threads.
//!
//! The compute pool has more than one thread only with Bevy's `multi_threaded`
//! feature, which only the visible builds turn on. Without it both executors
//! run on one thread and this test cannot tell them apart, so it says so and
//! passes. Run it as the visible builds have it:
//! `cargo test -p ambition_platformer2d_runtime --lib --features bevy/multi_threaded -- executor_tests`.

use super::serialize_main_schedules;
use bevy::app::{
    First, FixedFirst, FixedLast, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Last, PostUpdate, PreUpdate, SpawnScene,
    Update,
};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::state::state::StateTransition;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

/// The threads that ran the probes of each schedule.
#[derive(Resource, Clone, Default)]
struct Ran(Arc<Mutex<Vec<(&'static str, ThreadId)>>>);

#[test]
fn every_main_schedule_runs_its_systems_on_the_calling_thread() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    let ran = Ran::default();
    app.insert_resource(ran.clone());
    // Each probe reads only `Ran`, so the probes of a schedule do not conflict
    // and a multi-threaded executor can give them to other threads.
    fn add_probes(app: &mut App, schedule: impl ScheduleLabel + Clone, name: &'static str) {
        for _ in 0..16 {
            app.add_systems(schedule.clone(), move |ran: Res<Ran>| {
                ran.0.lock().unwrap().push((name, std::thread::current().id()));
            });
        }
    }
    add_probes(&mut app, First, "First");
    add_probes(&mut app, PreUpdate, "PreUpdate");
    add_probes(&mut app, StateTransition, "StateTransition");
    add_probes(&mut app, Update, "Update");
    add_probes(&mut app, SpawnScene, "SpawnScene");
    add_probes(&mut app, PostUpdate, "PostUpdate");
    add_probes(&mut app, Last, "Last");
    add_probes(&mut app, FixedFirst, "FixedFirst");
    add_probes(&mut app, FixedPreUpdate, "FixedPreUpdate");
    add_probes(&mut app, FixedUpdate, "FixedUpdate");
    add_probes(&mut app, FixedPostUpdate, "FixedPostUpdate");
    add_probes(&mut app, FixedLast, "FixedLast");
    serialize_main_schedules(&mut app);
    // The fixed schedules run when a whole step of time has passed.
    app.insert_resource(Time::<Fixed>::from_hz(2_000.0));
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(3));
        app.update();
    }

    let threads = bevy::tasks::ComputeTaskPool::get().thread_num();
    if threads < 2 {
        eprintln!("NOT RUN: the compute pool has {threads} thread; build with bevy/multi_threaded to compare the executors");
        return;
    }
    let here = std::thread::current().id();
    let ran = ran.0.lock().unwrap();
    for name in [
        "First", "PreUpdate", "StateTransition", "Update", "SpawnScene", "PostUpdate", "Last", "FixedFirst",
        "FixedPreUpdate", "FixedUpdate", "FixedPostUpdate", "FixedLast",
    ] {
        let runs: Vec<ThreadId> = ran.iter().filter(|(of, _)| *of == name).map(|(_, thread)| *thread).collect();
        assert!(!runs.is_empty(), "the premise: {name} ran its probes");
        let away = runs.iter().filter(|thread| **thread != here).count();
        assert_eq!(away, 0, "{away} of {} probe runs of {name} were on another thread", runs.len());
    }
}

#[derive(bevy::ecs::schedule::ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct MadeLater;

/// A schedule a plugin makes after the policy call runs serially too, once
/// the app is complete (`finish`, then `cleanup`, as `App::run` does).
#[test]
fn a_schedule_made_after_the_policy_runs_on_the_calling_thread() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    serialize_main_schedules(&mut app);
    app.init_schedule(MadeLater);
    let ran = Ran::default();
    app.insert_resource(ran.clone());
    for _ in 0..16 {
        app.add_systems(MadeLater, |ran: Res<Ran>| {
            ran.0.lock().unwrap().push(("MadeLater", std::thread::current().id()));
        });
    }
    app.finish();
    app.cleanup();
    for _ in 0..5 {
        app.world_mut().run_schedule(MadeLater);
    }
    let threads = bevy::tasks::ComputeTaskPool::get().thread_num();
    if threads < 2 {
        eprintln!("NOT RUN: the compute pool has {threads} thread; build with bevy/multi_threaded to compare the executors");
        return;
    }
    let here = std::thread::current().id();
    let ran = ran.0.lock().unwrap();
    assert!(!ran.is_empty(), "the premise: the schedule ran its probes");
    let away = ran.iter().filter(|(_, thread)| *thread != here).count();
    assert_eq!(away, 0, "{away} of {} probe runs were on another thread", ran.len());
}
