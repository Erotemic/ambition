//! sync_boss_reward_chests_ecs drops a boss's reward chest once the
//! boss reads Cleared in the save and a spawn anchor is known. The
//! non-ECS world/anchors params are carried in test-only resources so
//! a normal wrapper system can drive the helper.
use super::*;
use crate::{BossProfile, test_boss_catalog};
use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::PersistedEncounterState;
use bevy::prelude::{App, Res, Resource, Update};

#[derive(Resource)]
struct TestWorld(ae::World);
#[derive(Resource)]
struct TestAnchors(Vec<BossRewardAnchor>);
#[derive(Resource)]
struct TestRooms(Vec<Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>>);

fn run_boss_sync(
    mut commands: Commands,
    save: Res<AmbitionGameSave>,
    world: Res<TestWorld>,
    anchors: Res<TestAnchors>,
    rooms: Res<TestRooms>,
    chests: Query<
        (
            Entity,
            &BossRewardChest,
            &FeatureId,
            Option<&Opened>,
            Option<&FallingChest>,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
        With<ChestFeature>,
    >,
) {
    for room in &rooms.0 {
        sync_boss_reward_chests_ecs(
            &mut commands,
            SessionSpawnScope::UNSCOPED.in_room(*room),
            save.data(),
            &world.0,
            &anchors.0,
            &chests,
        );
    }
}

fn app() -> App {
    let mut app = App::new();
    let mut save = AmbitionGameSave::default();
    save.data_mut()
        .set_boss("test_boss", PersistedEncounterState::Cleared);
    app.insert_resource(save);
    app.insert_resource(TestWorld(ae::World::new(
        "t",
        ae::Vec2::new(400.0, 400.0),
        ae::Vec2::new(50.0, 50.0),
        Vec::new(),
    )));
    app.insert_resource(TestAnchors(vec![BossRewardAnchor {
        placement_id: "test_boss".into(),
        spawn: ae::Vec2::new(200.0, 100.0),
        reward: BossProfile::from_id(test_boss_catalog(), "mockingbird")
            .expect("mockingbird is authored")
            .reward,
    }]));
    app.insert_resource(TestRooms(vec![None]));
    app.add_systems(Update, run_boss_sync);
    app
}

#[test]
fn cleared_boss_drops_its_reward_chest() {
    let mut app = app();
    app.update();
    let mut q = app.world_mut().query::<&BossRewardChest>();
    let ids: Vec<String> = q
        .iter(app.world())
        .map(|r| r.encounter_id.clone())
        .collect();
    assert_eq!(
        ids,
        vec!["test_boss".to_string()],
        "a cleared boss drops one reward chest"
    );
}

#[test]
fn boss_reward_sync_is_idempotent() {
    let mut app = app();
    app.update();
    app.update();
    let mut q = app.world_mut().query::<&BossRewardChest>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "no duplicate boss chest on re-tick"
    );
}

#[test]
fn a_reward_has_one_chest_per_live_room_and_rewinds_only_its_room() {
    use crate::retraction::{
        BossDefeatRetracted, BossDefeatSinceCheckpoint, BossDefeatsSinceCheckpoint,
    };
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    let mut app = app();
    let first = LiveRoomInstance::from_ordinal(1);
    let second = LiveRoomInstance::from_ordinal(2);
    app.world_mut().resource_mut::<TestRooms>().0 = vec![Some(first)];
    app.update();
    app.world_mut()
        .resource_mut::<TestRooms>()
        .0
        .push(Some(second));
    app.update();
    app.update();
    let chests: Vec<_> = app
        .world_mut()
        .query::<(Entity, &BossRewardChest, &InRoomInstance)>()
        .iter(app.world())
        .map(|(entity, _, room)| (entity, room.0))
        .collect();
    assert_eq!(chests.len(), 2, "each live copy has its own reward");
    let first_chest = chests.iter().find(|(_, room)| *room == first).unwrap().0;
    let second_chest = chests.iter().find(|(_, room)| *room == second).unwrap().0;

    app.init_resource::<ambition_persistence::quest::QuestRegistry>();
    app.init_resource::<BossDefeatsSinceCheckpoint>();
    app.add_message::<BossDefeatRetracted>();
    app.world_mut()
        .resource_mut::<BossDefeatsSinceCheckpoint>()
        .record(
            "test_boss",
            BossDefeatSinceCheckpoint {
                room: Some(first),
                definition: "t".into(),
                boss: None,
                present: vec![],
                tick: Some(10),
            },
        );
    app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag(
            ambition_encounter::encounter_reward_looted_flag("test_boss"),
            true,
        );
    app.update();
    assert!(app.world().get::<Opened>(first_chest).is_some());
    assert!(app.world().get::<Opened>(second_chest).is_some());
    crate::retraction::retract_boss_defeats_on_room_rewind(app.world_mut(), first, 9);
    assert!(app.world().get_entity(first_chest).is_err());
    assert!(app.world().get::<Opened>(second_chest).is_some());
    let save = app.world().resource::<AmbitionGameSave>().data();
    assert_eq!(save.boss("test_boss"), PersistedEncounterState::Untouched);
    assert!(
        !save.flag(&ambition_encounter::encounter_reward_looted_flag(
            "test_boss"
        ))
    );
}
