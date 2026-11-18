//! Loaded rules exercise reflected credit and native rollback after retirement.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::CharacterForm,
    combat::{extension::status::BodyStatuses, targeting::MatchTeam},
    engine_core::BodyKinematics,
    extension::{
        BodyRecords, ExtensionFaults,
        sdk::{FieldRef, SchemaKey, Value},
    },
    platformer::{construction::SpawnOrigin, markers::PrimaryPlayerOnly, sim_id::SimId},
    projectiles::{LiveProjectile, ProjectileOwner, ownership::ProjectileCredit},
};
use bevy::prelude::*;

const CHECK_DISTANCE: usize = 4;
const RECORD: SchemaKey = SchemaKey::new("credit_probe", "flight", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "credit_probe", key: "flight",
       schemas: [#{ key: "flight", fields: [
           #{ tag: 1, name: "fired", kind: "bool" },
           #{ tag: 2, name: "receptions", kind: "u32" },
           #{ tag: 3, name: "shot", kind: "opt body" },
           #{ tag: 4, name: "credit", kind: "opt body" },
           #{ tag: 5, name: "damage", kind: "u32" },
           #{ tag: 6, name: "births", kind: "u32" },
           #{ tag: 7, name: "reflections", kind: "u32" },
           #{ tag: 8, name: "retirements", kind: "u32" },
           #{ tag: 9, name: "old_credit", kind: "opt body" },
           #{ tag: 10, name: "new_credit", kind: "opt body" },
           #{ tag: 11, name: "last_shot", kind: "opt body" },
       ] }],
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
           #{ key: "lifecycle", phase: "hit_reaction",
              trigger: #{ port: "projectile_outcomes", selector: "credit_probe.lifecycle" },
              writes: ["flight"] },
           #{ key: "observe", phase: "hit_reaction",
              trigger: #{ port: "hit_receptions", selector: "credit_probe.observe" },
              writes: ["flight"] },
       ],
    }
}
fn fire() {
    if this.state.flight.fired { return; }
    this.state.flight.fired = true;
    this.requests.push(request("projectile", #{ origin: this.trigger.position,
        dir: [-1.0, 0.0], speed: 200.0, damage: 1, max_lifetime: 3.0,
        half_extent: [4.0, 4.0], splash_half_extent: 16.0, visual_id: "" }));
}
fn guard() {
    this.requests.push(request("parry_window", #{ window_s: 0.2 }));
}
fn retire() {
    this.requests.push(request("defeat_body", #{}));
}
fn lifecycle() {
    for fact in this.trigger.events {
        if fact.projectile.id == () || fact.credit == () || fact.credit.id == () { throw "lost native shot or credit"; }
        if fact.kind == "created" {
            if fact.terminal { throw "birth retired the shot"; }
            this.state.flight.births += 1;
        } else if fact.kind == "reflected" {
            if fact.terminal || fact.previous_credit == () || fact.interceptor == () { throw "lost reflection context"; }
            if fact.credit.id != fact.interceptor.id { throw "reflection did not commit new credit"; }
            this.state.flight.reflections += 1;
            this.state.flight.old_credit = fact.previous_credit.id;
            this.state.flight.new_credit = fact.credit.id;
        } else if fact.kind == "body_contact" {
            if !fact.terminal { throw "a spent shot stayed live"; }
            this.state.flight.retirements += 1;
            this.state.flight.last_shot = fact.projectile.id;
        } else { throw "unexpected native lifecycle"; }
    }
}
fn observe() {
    for hit in this.trigger.events {
        if hit.source != "projectile" || !hit.incoming { continue; }
        if hit.projectile == () || hit.projectile.id == () || hit.attacker == () || hit.attacker.id == () {
            throw "an accepted projectile hit lost its captured provenance";
        }
        this.state.flight.receptions += 1;
        this.state.flight.shot = hit.projectile.id;
        this.state.flight.credit = hit.attacker.id;
        this.state.flight.damage += hit.damage;
    }
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
    sim.teleport_player((target.x + 100.0, target.y));
    sim.rebase_rollback_history().unwrap();
    for _ in 0..60 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(player)
            .unwrap()
            .on_ground
        {
            break;
        }
    }
    assert!(
        sim.world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(player)
            .unwrap()
            .on_ground
    );
    let world = sim.world_mut();
    world.entity_mut(player).insert((
        MatchTeam::new("source"),
        BodyStatuses(vec![
            "credit_probe.fire".into(),
            "credit_probe.observe".into(),
            "credit_probe.lifecycle".into(),
        ]),
    ));
    world.entity_mut(defender).insert((
        MatchTeam::new("defender"),
        BodyStatuses(vec![
            "credit_probe.guard".into(),
            "credit_probe.lifecycle".into(),
        ]),
    ));
    sim.world_mut()
        .get_resource_or_insert_with(
            ambition_platformer2d::world::scoped_rules::DeclaredRules::<
                ambition_platformer2d::combat::rules::ProjectileOutcomeRules,
            >::default,
        )
        .declare(
            ambition_platformer2d::world::scoped_rules::RulesScope::EveryRoom,
            ambition_platformer2d::combat::rules::ProjectileOutcomeRules {
                max_events: 8,
                room_observers: vec!["credit_probe.lifecycle".into()],
            },
        );
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
    assert!(
        !world
            .query::<&SimId>()
            .iter(world)
            .any(|id| *id == defender_id)
    );
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
    for _ in 0..120 {
        sim.step(AgentAction::default());
        let records = sim.world().get::<BodyRecords>(player).unwrap();
        if records.get(&RECORD).unwrap().get(FieldRef(1)).unwrap() == &Value::U32(2) {
            break;
        }
    }
    sim.step_n(AgentAction::default(), CHECK_DISTANCE as u32);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    let record = sim
        .world()
        .get::<BodyRecords>(player)
        .unwrap()
        .get(&RECORD)
        .unwrap();
    assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(2));
    assert_eq!(
        record.get(FieldRef(2)).unwrap(),
        &Value::Opt(Some(Box::new(Value::Ref(shot_id.as_str().into()))))
    );
    assert_eq!(
        record.get(FieldRef(3)).unwrap(),
        &Value::Opt(Some(Box::new(Value::Ref(defender_id.as_str().into()))))
    );
    assert!(matches!(record.get(FieldRef(4)).unwrap(), Value::U32(damage) if *damage > 0));
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>>().single(world).unwrap();
    let record = world
        .get::<BodyRecords>(root)
        .unwrap()
        .get(&RECORD)
        .unwrap();
    for field in [5, 6, 7] {
        assert_eq!(record.get(FieldRef(field)).unwrap(), &Value::U32(1));
    }
    for (field, id) in [(8, source_id), (9, defender_id), (10, shot_id)] {
        assert_eq!(
            record.get(FieldRef(field)).unwrap(),
            &Value::Opt(Some(Box::new(Value::Ref(id.as_str().into()))))
        );
    }
    let player_record = world
        .get::<BodyRecords>(player)
        .unwrap()
        .get(&RECORD)
        .unwrap();
    assert_eq!(player_record.get(FieldRef(5)).unwrap(), &Value::U32(1));
    assert_eq!(player_record.get(FieldRef(6)).unwrap(), &Value::U32(1));
    assert_eq!(player_record.get(FieldRef(7)).unwrap(), &Value::U32(0));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn creation_waiting_at_a_snapshot_is_delivered_once_without_the_native_message_transport() {
    use ambition_platformer2d::projectiles::{
        ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart, ProjectileWriter,
        materialize_projectiles_for_next_tick,
    };
    use bevy::ecs::system::RunSystemOnce;
    let dir = std::env::temp_dir().join(format!(
        "ambition_projectile_pending_{}",
        std::process::id()
    ));
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
    sim.step_n(AgentAction::default(), 2);
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
            .insert(BodyStatuses(vec!["credit_probe.lifecycle".into()]));
        let request = ProjectileSpawnRequest::open(
            body,
            ProjectileSpawn {
                reply: None,
                properties: Default::default(),
                guard: Default::default(),
                origin: Vec2::new(320.0, 80.0),
                dir: Vec2::X,
                speed: 10.0,
                damage: 1,
                max_lifetime: 3.0,
                half_extent: Vec2::splat(2.0),
                gravity: 0.0,
                visual_id: String::new(),
                bounces: 0,
                bounce_on_world_contact: false,
                splash_half_extent: 0.0,
                boomerang_return_s: None,
            },
            ProjectileStart::StepNextTick,
        );
        assert!(
            world
                .run_system_once_with(
                    |In(request), mut writer: ProjectileWriter| writer.write(request),
                    request
                )
                .unwrap()
        );
    }
    world
        .run_system_once(materialize_projectiles_for_next_tick)
        .unwrap();
    world.resource_mut::<Messages<ambition_platformer2d::combat::projectile_outcomes::ProjectileOutcome>>().clear();
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 12);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    for body in [player, teacher] {
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert_eq!(record.get(FieldRef(5)).unwrap(), &Value::U32(1));
    }
    let _ = std::fs::remove_dir_all(dir);
}
