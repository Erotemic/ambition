//! Prospective script capabilities run through the same body services.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::combat::extension::status::{BodyStatuses, StatusClocks};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults};
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::time::ProperTimeScale;
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("capability_probe", "status.clock", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "capability_probe", key: "status",
       schemas: [#{ key: "status.clock", fields: [
           #{ tag: 1, name: "ticks", kind: "u32" },
           #{ tag: 2, name: "credit", kind: "u64" },
           #{ tag: 3, name: "seed", kind: "u64", initial: "18446744073709551615" },
       ] }],
       entries: [#{ key: "tick", phase: "technique_execution",
           trigger: #{ port: "status_tick", selector: "capability_probe.slow" },
           reads: ["credit", "in_play"], writes: ["status.clock"],
           requests: ["time_scale", "status_clock", "blast"], limit: 4 }],
    }
}
fn tick() {
    if !this.observe.in_play { return; }
    let state = this.state.clock;
    state.ticks += 1;
    state.credit = this.observe.credit;
    this.requests.push(request("time_scale", #{ scale: 0.5 }));
    this.requests.push(request("status_clock", #{ kind: "capability_probe.slow", remaining: 0.75 }));
    this.requests.push(request("blast", #{
        center: [5000.0, 5000.0], half_extent: [4.0, 4.0], damage: 1,
        lifetime_s: 0.1, credit: this.observe.credit,
    }));
    if state.ticks == 3 { this.requests.push(request("time_scale", #{ scale: 0.0 })); }
    this.state.clock = state;
}
"#;

#[test]
fn scripts_can_compose_status_clocks_credit_and_area_strikes_for_any_controller() {
    let dir = std::env::temp_dir().join(format!("ambition_status_ports_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("status.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room").with_save(save)
            .with_extension_module_files(vec![file]).with_sync_test_rollback_settings(4, 12),
    ).unwrap();
    sim.step_n(AgentAction::default(), 30);
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let teacher = world.query::<(Entity, &CharacterForm)>().iter(world)
        .find(|(_, form)| form.id() == "npc_creator").unwrap().0;
    for body in [player, teacher] {
        world.entity_mut(body).insert(BodyStatuses(vec!["capability_probe.slow".into()]));
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 12);
    for body in [player, teacher] {
        let world = sim.world();
        assert_eq!(world.get::<ProperTimeScale>(body).unwrap().0, 0.5, "zero scale was refused");
        assert!(world.get::<StatusClocks>(body).is_some());
        assert!(matches!(world.get::<BodyRecords>(body).and_then(|r| r.get(&RECORD))
            .and_then(|r| r.get(FieldRef(0)).ok()), Some(Value::U32(ticks)) if *ticks > 3));
        let record = world.get::<BodyRecords>(body).unwrap().get(&RECORD).unwrap();
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U64(
            ambition_platformer2d::combat::extension::blast::credit_of_scope(world, body).unwrap()));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U64(u64::MAX));
        assert!(world.resource::<ambition_platformer2d::sim_view::CombatGeometryView>().strikes.iter()
            .any(|strike| strike.owner == body), "the token credits the scope body's real strike");
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
