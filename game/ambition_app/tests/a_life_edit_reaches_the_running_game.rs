//! RADIANT-NPC-HTN slice 6: change a policy without a host compile.
//!
//! The shipped composition plays the coil lab. An edit to the lives file
//! (Nikobolt lunches at eleven, not noon), compiled into a candidate pack in
//! memory and published by a content reload, reaches his next plan in the
//! running game: the hour that was a working one is a lunch hour now. No Rust
//! is built for the edit, and the tree is not touched.
//!
//! The reload prepares the session again, so the lab and Nikobolt are built
//! anew from the edited pack (measured: another entity, with no plan yet).

use ambition_platformer2d::actors::body::life::{Life, LifeBook, Value};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::time::{DayClock, DayCycle};
use bevy::prelude::*;

const LAB: &str = "eureka_coil_lab";
const NIKOBOLT: &str = "npc_science_nikobolt";

fn nikobolt(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == NIKOBOLT)
        .map(|(entity, _)| entity)
        .expect("the coil lab stages Nikobolt")
}

/// Put the clock just before `hour` of the next day: the next plan is a new
/// day's plan for that hour.
fn just_before(app: &mut App, hour: f32) {
    let world = app.world_mut();
    let cycle = *world.resource::<DayCycle>();
    let day = world.resource::<DayClock>().day;
    *world.resource_mut::<DayClock>() = DayClock { day: day + 1, ..DayClock::at_hour(&cycle, hour - 0.01) };
}

/// The plan Nikobolt makes at `hour`: the first reason its planner gave (the
/// method his day was lived by: `live_the_day: <method>`).
fn the_plan_at(app: &mut App, hour: f32) -> String {
    just_before(app, hour);
    let body = nikobolt(app);
    let started = app.world().get::<Life>(body).map(|life| life.revision);
    for _ in 0..120 {
        app.update();
        let body = nikobolt(app);
        if let Some(life) = app.world().get::<Life>(body) {
            if Some(life.revision) != started && life.planned_in.map(|(_, at)| at) == Some(hour as i64) {
                if let Some(step) = life.task() {
                    return step.because.first().cloned().unwrap_or_default();
                }
            }
        }
    }
    let body = nikobolt(app);
    panic!("Nikobolt made no plan at {hour}: {:?}", app.world().get::<Life>(body));
}

fn lunch_hour_in_the_book(app: &App) -> Option<Value> {
    app.world().resource::<LifeBook>().lives.get(NIKOBOLT).and_then(|spec| spec.authored.get("lunch_hour").cloned())
}

#[test]
fn an_edited_lunch_hour_reaches_his_next_plan_in_the_running_game() {
    let mut app = crate::shell_host_rendered::rendered_app_in_room(ambition_app::app::VisibleRenderMode::NoWindow, Some(LAB));
    crate::shell_host_rendered::settle(&mut app);
    app.world_mut().write_message(ShellCommand::GoTo(ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into()));
    crate::shell_host_rendered::settle(&mut app);

    // Control: as shipped, eleven o'clock is a working hour.
    assert_eq!(lunch_hour_in_the_book(&app), Some(Value::Int(12)), "premise: the shipped lunch hour");
    assert_eq!(the_plan_at(&mut app, 11.0), "live_the_day: a working day", "as shipped, eleven is not his lunch hour");

    let before = nikobolt(&mut app);
    let revision = app.world().get::<Life>(before).map(|life| life.revision);

    // The edit: lunch at eleven. Compiled in memory; the tree is not touched.
    let base = ambition_content::pack::selected(app.world()).expect("a selection").fingerprint;
    let mut edited = false;
    let candidate = std::sync::Arc::new(
        ambition_content::pack::compile_pack_with(|declared, text| {
            if declared == "data/npc_lives.ron" {
                let out = text.replacen("\"lunch_hour\": Int(12)", "\"lunch_hour\": Int(11)", 1);
                edited = out != text;
                return out;
            }
            text
        })
        .expect("the edited pack compiles"),
    );
    assert!(edited, "the lives file no longer states Nikobolt's lunch at twelve");
    let outcome = ambition_content::reload::request_reload(
        app.world_mut(),
        ambition_content::CandidateGeneration::prepared_against(candidate, Some(base)),
    );
    assert!(matches!(outcome, ambition_content::reload::ReloadRequest::Requested { .. }), "{outcome:?}");
    let started = std::time::Instant::now();
    while app.world().contains_resource::<ambition_content::reload::PendingGeneration>() {
        assert!(started.elapsed() < crate::common::ACTIVATION_CEILING, "the reload is still pending");
        app.update();
    }
    assert_eq!(lunch_hour_in_the_book(&app), Some(Value::Int(11)), "the reload did not publish the edited lives");

    let after = nikobolt(&mut app);
    println!(
        "Nikobolt before the reload {before:?} (plan revision {revision:?}), after {after:?} (revision {:?})",
        app.world().get::<Life>(after).map(|life| life.revision)
    );

    // The same hour is a lunch hour now.
    assert_eq!(the_plan_at(&mut app, 11.0), "live_the_day: lunch", "after the reload, eleven is still not his lunch hour");
}
