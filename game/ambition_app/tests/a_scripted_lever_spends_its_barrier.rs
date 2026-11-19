//! A playable room composes native activation, private methods and one atomic spend.
#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    breakables::BreakableFeature,
    characters::actor::BodyWallet,
    combat::components::FeatureId,
    extension::{BodyRecords, ExtensionFaults, ExtensionGeneration, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

#[test]
fn native_activation_opens_the_authored_barrier_only_after_an_applied_exchange() { exercise(false); }

#[test]
fn retained_activation_reconciles_dropped_notifications_and_code_reload_keeps_objects() { exercise(true); }

fn exercise(drop_notifications: bool) {
    let source = include_str!("../../ambition_content/rhai/switch_lab.rhai");
    let path = std::env::temp_dir().join(format!("ambition_switch_lab_{}_{}.rhai", std::process::id(), drop_notifications));
    std::fs::write(&path, source).unwrap();
    let options = crate::common::fixed_60hz_room_options("switch_lab").with_extension_module_files(vec![path.clone()]);
    // The sync-test caller owns its timeline. Live edits use the fixed host.
    let options = if drop_notifications { options } else { options.with_sync_test_rollback_settings(4, 12) };
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    if drop_notifications {
        let mut rules = ambition_platformer2d::world::scoped_rules::DeclaredRules::default();
        rules.declare(ambition_platformer2d::world::scoped_rules::RulesScope::EveryRoom,
            ambition_platformer2d::extension::ObjectBehaviorLimits {max_event_history: 0, ..Default::default()});
        sim.world_mut().insert_resource(rules);
    }
    sim.step_n(crate::common::base(), 24);
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let barrier = world.query::<(Entity, &FeatureId)>().iter(world)
        .find(|(_, id)| id.as_str() == "switch_lab_barrier").unwrap().0;
    world.entity_mut(player).insert(BodyWallet {balance: 2});
    let lever_position = world.query::<(&FeatureId, &ambition_platformer2d::engine_core::CenteredAabb)>().iter(world)
        .find(|(id, _)| id.as_str() == "switch_lab_toll").unwrap().1.center;
    assert!(ambition_platformer2d::platformer::lifecycle::object_reference(world, player).is_some());
    sim.teleport_player((lever_position.x, lever_position.y));
    sim.rebase_rollback_history().unwrap();
    let press = |sim: &mut Platformer2dSimHarness| {
        sim.step(crate::common::base());
        sim.step(AgentAction {interact: true, ..crate::common::base()});
        sim.step_n(crate::common::base(), if drop_notifications { 40 } else { 16 });
    };
    press(&mut sim);
    assert!(!sim.world().get::<BreakableFeature>(barrier).unwrap().broken());
    assert!(collider_present(&mut sim, barrier));
    assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 2);
    sim.world_mut().get_mut::<BodyWallet>(player).unwrap().balance = 6;
    sim.rebase_rollback_history().unwrap();
    press(&mut sim);
    assert!(sim.world().get::<BreakableFeature>(barrier).unwrap().broken(), "faults: {:?}", sim.world().resource::<ExtensionFaults>());
    assert!(!collider_present(&mut sim, barrier));
    assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 3);
    press(&mut sim);
    assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 3, "an already consumed barrier cannot charge twice");
    let key = SchemaKey::new("ambition", "switch_lab.lever", 1);
    let world = sim.world_mut();
    let records: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&key)).collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get(FieldRef(2)).unwrap(), &Value::U32(1));
    assert_eq!(records[0].get(FieldRef(3)).unwrap(), &Value::U32(2));
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    if drop_notifications {
        let before = sim.world().resource::<ExtensionGeneration>().clone();
        let reference = ambition_platformer2d::platformer::lifecycle::object_reference(sim.world(), barrier).unwrap();
        std::fs::write(&path, source.replace("pay_coins: this.observe.behavior.params.price", "pay_coins: 0")).unwrap();
        sim.step_n(crate::common::base(), 400);
        assert!(*sim.world().resource::<ExtensionGeneration>() != before,
            "the local host did not admit the changed script; files: {:?}",
            sim.world().get_resource::<ambition_platformer2d::extension::ExtensionModuleFiles>());
        assert_eq!(ambition_platformer2d::platformer::lifecycle::object_reference(sim.world(), barrier), Some(reference));
        assert!(!sim.world().get::<BreakableFeature>(barrier).unwrap().broken());
        assert!(collider_present(&mut sim, barrier));
        press(&mut sim);
        assert!(sim.world().get::<BreakableFeature>(barrier).unwrap().broken());
        assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 3);
        assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
        sim.rollback_health().unwrap();
    }
    let _ = std::fs::remove_file(path);
}


fn collider_present(sim: &mut Platformer2dSimHarness, entity: Entity) -> bool {
    use ambition_platformer2d::{engine_core as ae, platformer::lifecycle::InRoomInstance};
    let id = sim.world().get::<FeatureId>(entity).unwrap().as_str().to_owned();
    let room = *sim.world().get::<InRoomInstance>(entity).unwrap();
    let mut collision = bevy::ecs::system::SystemState::<ambition_platformer2d::world::collision::CollisionWorld>::new(sim.world_mut());
    collision.get(sim.world()).unwrap().room(Some(&room)).unwrap().solids().unwrap().blocks.iter()
        .any(|block| block.id == ae::GeoId::placement(ae::PlacementId::new(&id), 0))
}
