//! Loaded scripts receive shared physical contact facts for any controller.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::CharacterForm,
    combat::{
        extension::{contact::ContactFactsLimits, status::BodyStatuses},
        targeting::MatchTeam,
    },
    engine_core::BodyKinematics,
    extension::{
        BodyRecords, ExtensionFaults,
        sdk::{FieldRef, SchemaKey, Value},
    },
    platformer::{markers::PrimaryPlayerOnly, sim_id::SimId},
    world::scoped_rules::{DeclaredRules, RulesScope},
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("contact_probe", "contact", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "contact_probe", key: "contacts",
       schemas: [#{ key: "contact", fields: [
           #{ tag: 1, name: "samples", kind: "u32" },
           #{ tag: 2, name: "contacts", kind: "u32" },
           #{ tag: 3, name: "empty", kind: "u32" },
           #{ tag: 4, name: "target", kind: "opt body" },
       ] }],
       entries: [
           #{ key: "activate", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "contact_probe.seed" },
              requests: ["hold_status"], limit: 1 },
           #{ key: "observe", phase: "body_contact",
           trigger: #{ port: "body_contact", selector: "contact_probe.observe" },
           writes: ["contact"], requests: ["contact_bounce"], limit: 1 }],
    }
}
fn activate() {
    this.requests.push(request("hold_status", #{ kind: "contact_probe.observe", held: true }));
}
fn observe() {
    let sample = this.trigger;
    if sample.incomplete { throw "the fixture has refused geometry"; }
    this.state.contact.samples += 1;
    let met = false;
    for index in 0..sample.touchers.len() {
        let toucher = sample.touchers[index];
        if toucher.relation != "foe" { continue; }
        met = true;
        if toucher.id == sample.body_id { throw "a body touches itself"; }
        if toucher.surfaces.len() != 1 { throw "the surface budget was ignored"; }
        if !toucher.surfaces_truncated { throw "the contact patch was not reported"; }
        this.state.contact.target = toucher.id;
        if this.state.contact.contacts == 0 {
            this.requests.push(request("contact_bounce", #{ toucher: index, speed: 250.0 }));
        }
        this.state.contact.contacts += 1;
    }
    if !met { this.state.contact.empty += 1; }
}
"#;

#[test]
fn scripts_read_bounded_native_contacts_and_bounce_either_controller_through_replay() {
    let dir = std::env::temp_dir().join(format!("ambition_body_contacts_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("contacts.rhai");
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
    let teacher_body = *world.get::<BodyKinematics>(teacher).unwrap();
    let player_body = *world.get::<BodyKinematics>(player).unwrap();
    for (body, team) in [(player, "red"), (teacher, "blue")] {
        world.entity_mut(body).insert((
            MatchTeam::new(team),
            BodyStatuses(vec!["contact_probe.seed".into()]),
        ));
    }
    world
        .get_resource_or_insert_with(DeclaredRules::<ContactFactsLimits>::default)
        .declare(
            RulesScope::EveryRoom,
            ContactFactsLimits {
                max_touchers: 1,
                max_surfaces: 1,
            },
        );
    let x = teacher_body.pos.x + teacher_body.half_extent().x + player_body.half_extent().x - 5.0;
    sim.teleport_player((x, teacher_body.pos.y));
    sim.step_n(AgentAction::default(), 4);
    for (body, target) in [(player, teacher), (teacher, player)] {
        let world = sim.world();
        let record = world
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert!(matches!(record.get(FieldRef(1)).unwrap(), Value::U32(count) if *count > 0));
        assert_eq!(
            record.get(FieldRef(3)).unwrap(),
            &Value::Opt(Some(Box::new(Value::Ref(
                world.get::<SimId>(target).unwrap().as_str().into()
            ))))
        );
        assert!(world.get::<BodyKinematics>(body).unwrap().vel.y < 0.0);
    }
    sim.teleport_player((x + 300.0, teacher_body.pos.y));
    sim.step_n(AgentAction::default(), 8);
    for body in [player, teacher] {
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert!(matches!(record.get(FieldRef(2)).unwrap(), Value::U32(count) if *count > 0));
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
