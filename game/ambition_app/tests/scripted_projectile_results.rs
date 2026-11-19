//! Loaded rules observe actual projectile creation, refusal, and retry identity.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::CharacterForm,
    combat::extension::status::BodyStatuses,
    extension::{
        BodyRecords, ExtensionFaults,
        sdk::{FieldRef, SchemaKey, Value},
    },
    platformer::markers::PrimaryPlayerOnly,
    projectiles::ProjectileSeqCounter,
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("projectile_probe", "shots", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "projectile_probe", key: "shots",
       schemas: [#{ key: "shots", fields: [
           #{ tag: 1, name: "sent", kind: "bool" },
           #{ tag: 2, name: "created", kind: "u32" },
           #{ tag: 3, name: "refused", kind: "u32" },
           #{ tag: 4, name: "shot", kind: "opt body" },
       ] }],
       entries: [
           #{ key: "fire", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "projectile_probe.fire" },
              writes: ["shots"], requests: ["projectile"], limit: 3 },
           #{ key: "result", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "result" }, writes: ["shots"] },
       ],
    }
}
fn fire() {
    if this.state.shots.sent { return; }
    this.state.shots.sent = true;
    let shot = #{ origin: [320.0, 80.0], dir: [1.0, 0.0], speed: 100.0,
        damage: 1, max_lifetime: 3.0, half_extent: [2.0, 2.0],
        reply: #{ request: 1, selector: "projectile_probe::shots/result" } };
    this.requests.push(request("projectile", shot));
    this.requests.push(request("projectile", shot));
    shot.reply.request = 2;
    shot.half_extent = [-2.0, 2.0];
    this.requests.push(request("projectile", shot));
}
fn result() {
    let result = this.trigger;
    if result.stage == "applied" {
        if !result.terminal || result.code != "projectile_created" || result.subjects.len() != 1 {
            throw "creation did not identify the applied shot";
        }
        this.state.shots.created += 1;
        this.state.shots.shot = result.subjects[0].key;
    } else if result.stage == "rejected" {
        if result.code != "invalid_projectile_shape" { throw "wrong refusal"; }
        this.state.shots.refused += 1;
    }
}
"#;

#[test]
fn both_controllers_receive_created_identity_and_refusal_without_duplicate_births_under_replay() {
    let dir = std::env::temp_dir().join(format!(
        "ambition_projectile_results_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("shots.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let teacher = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, form)| form.id() == "npc_creator")
        .unwrap()
        .0;
    for body in [player, teacher] {
        world
            .entity_mut(body)
            .insert(BodyStatuses(vec!["projectile_probe.fire".into()]));
    }
    let before = world.resource::<ProjectileSeqCounter>().0;
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 20);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    assert_eq!(sim.world().resource::<ProjectileSeqCounter>().0, before + 2);
    let mut ids = Vec::new();
    for body in [player, teacher] {
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(1));
        let Value::Opt(Some(id)) = record.get(FieldRef(3)).unwrap() else {
            panic!("creation has an identity");
        };
        let Value::Ref(id) = &**id else {
            panic!("creation supplies a semantic reference");
        };
        ids.push(id.clone());
    }
    assert_ne!(ids[0], ids[1]);
    let _ = std::fs::remove_dir_all(dir);
}
