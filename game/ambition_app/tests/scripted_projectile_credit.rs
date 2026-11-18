//! Loaded rules exercise reflected credit and native rollback after retirement.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::CharacterForm,
    combat::{extension::status::BodyStatuses, targeting::MatchTeam},
    engine_core::BodyKinematics,
    extension::{
        sdk::{FieldRef, SchemaKey, Value},
        BodyRecords, ExtensionFaults,
    },
    platformer::{construction::SpawnOrigin, markers::PrimaryPlayerOnly, sim_id::SimId},
    projectiles::{ownership::ProjectileCredit, LiveProjectile, ProjectileOwner},
};
use bevy::prelude::*;

const CHECK_DISTANCE: usize = 4;
const RECORD: SchemaKey = SchemaKey::new("credit_probe", "flight", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "credit_probe", key: "flight",
       schemas: [#{ key: "flight", fields: [#{ tag: 1, name: "fired", kind: "bool" }] }],
       entries: [
           #{ key: "fire", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "credit_probe.fire" },
              writes: ["flight"], requests: ["projectile"], limit: 1 },
           #{ key: "guard", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "credit_probe.guard" },
              requests: ["parry_window"], limit: 1 },
           #{ key: "retire", phase: "body_contact",
              trigger: #{ port: "body_contact", selector: "credit_probe.retire" },
              requests: ["defeat_body"], limit: 1 },
       ],
    }
}
fn fire() {
    if this.state.flight.fired { return; }
    this.state.flight.fired = true;
    this.requests.push(request("projectile", #{ origin: this.trigger.position,
        dir: [-1.0, 0.0], speed: 200.0, damage: 1, max_lifetime: 3.0,
        half_extent: [4.0, 4.0], visual_id: "" }));
}
fn guard() {
    this.requests.push(request("parry_window", #{ window_s: 0.2 }));
}
fn retire() {
    this.requests.push(request("defeat_body", #{}));
}
"#;

#[test]
fn a_scripted_reflection_keeps_current_credit_when_the_deflector_retires_and_replay_runs() {
    let dir =
        std::env::temp_dir().join(format!("ambition_projectile_credit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("credit.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(CHECK_DISTANCE, 12),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let defender = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, form)| form.id() == "npc_creator")
        .unwrap()
        .0;
    let source_id = world.get::<SimId>(player).unwrap().clone();
    let defender_id = world.get::<SimId>(defender).unwrap().clone();
    let target = world.get::<BodyKinematics>(defender).unwrap().pos;
    world.entity_mut(player).insert((
        MatchTeam::new("source"),
        BodyStatuses(vec!["credit_probe.fire".into()]),
    ));
    world.entity_mut(defender).insert((
        MatchTeam::new("defender"),
        BodyStatuses(vec!["credit_probe.guard".into()]),
    ));
    sim.teleport_player((target.x + 100.0, target.y));
    sim.rebase_rollback_history().unwrap();
    let mut reflected = None;
    for _ in 0..60 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        reflected = world
            .query_filtered::<(Entity, &ProjectileOwner), With<LiveProjectile>>()
            .iter(world)
            .find(|(_, owner)| owner.0 == defender)
            .map(|(shot, _)| shot);
        if reflected.is_some() {
            break;
        }
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    let shot = reflected.expect("the authored native parry reflects the script's projectile");
    let world = sim.world();
    assert_eq!(
        world.get::<ProjectileCredit>(shot).unwrap().body.as_ref(),
        Some(&defender_id)
    );
    let SpawnOrigin::Dynamic { parent, sequence } = world.get::<SpawnOrigin>(shot).unwrap() else {
        panic!("a shot has dynamic creation provenance");
    };
    assert_eq!(parent, &source_id);
    let shot_id = SimId::spawned(&source_id, *sequence);
    assert_eq!(world.get::<SimId>(shot), Some(&shot_id));
    assert_eq!(
        world
            .get::<BodyRecords>(player)
            .unwrap()
            .get(&RECORD)
            .unwrap()
            .get(FieldRef(0))
            .unwrap(),
        &Value::Bool(true)
    );
    sim.step_n(AgentAction::default(), CHECK_DISTANCE as u32);
    sim.rollback_health().unwrap();
    sim.world_mut()
        .get_mut::<BodyStatuses>(defender)
        .unwrap()
        .0
        .push("credit_probe.retire".into());
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 3);
    let world = sim.world_mut();
    assert!(!world
        .query::<&SimId>()
        .iter(world)
        .any(|id| *id == defender_id));
    let shot = world
        .query::<(Entity, &SimId)>()
        .iter(world)
        .find(|(_, id)| **id == shot_id)
        .unwrap()
        .0;
    assert!(
        world.get::<ProjectileOwner>(shot).is_none(),
        "the original shooter cannot regain reflected credit"
    );
    assert_eq!(
        world.get::<ProjectileCredit>(shot).unwrap().body.as_ref(),
        Some(&defender_id)
    );
    sim.step_n(AgentAction::default(), 30);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    let _ = std::fs::remove_dir_all(dir);
}
