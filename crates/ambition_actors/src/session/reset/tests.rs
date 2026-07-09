//! Unit tests for the sandbox reset flow: idle-by-default request resource,
//! request/consume edge behavior, and the post-reset gameplay-state rebuild.

use super::*;
use crate::player::PlayerBlinkCameraState;
use ambition_dev_tools::dev_tools::EditableMovementTuning;
use ambition_engine_core::RoomGeometry;

/// Pin the request resource's defaults: a fresh app starts with
/// no reset queued. Important because the reset processor must
/// be a no-op when nothing has been requested.
#[test]
fn request_default_is_idle() {
    let req = SandboxResetRequested::default();
    assert!(!req.request);
}

/// `request()` sets the flag; the processor consumes it.
#[test]
fn request_helper_sets_the_flag() {
    let mut req = SandboxResetRequested::default();
    req.request();
    assert!(req.request);
}

#[test]
fn sandbox_reset_clears_portals_held_items_and_summons() {
    let mut app = App::new();
    app.insert_resource(SandboxResetRequested::default());
    app.add_systems(Update, clear_transient_on_sandbox_reset);

    let ground = app
        .world_mut()
        .spawn(crate::items::pickup::GroundItem {
            spec: crate::items::pickup::axe_spec(),
            pos: ae::Vec2::ZERO,
            vel: ae::Vec2::ZERO,
            half_extent: ae::Vec2::splat(18.0),
        })
        .id();
    let ally = app
        .world_mut()
        .spawn(crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly)
        .id();
    let player =
        app.world_mut()
            .spawn((
                crate::actor::PlayerEntity,
                ambition_characters::brain::ActionSet::default(),
                crate::items::pickup::StashedActionSet(
                    ambition_characters::brain::ActionSet::default(),
                ),
                crate::features::HeldItem::new(crate::items::pickup::axe_spec()),
            ))
            .id();
    #[cfg(feature = "portal")]
    app.world_mut()
        .entity_mut(player)
        .insert(ambition_portal::PortalGun::default());

    // No reset queued → nothing changes.
    app.update();
    assert!(app
        .world()
        .get::<crate::items::pickup::GroundItem>(ground)
        .is_some());
    assert!(app
        .world()
        .get::<crate::features::HeldItem>(player)
        .is_some());

    // Reset requested → transient entities despawn + player held-state stripped.
    app.world_mut()
        .resource_mut::<SandboxResetRequested>()
        .request = true;
    app.update();
    assert!(
        app.world()
            .get::<crate::items::pickup::GroundItem>(ground)
            .is_none(),
        "ground item despawned on reset"
    );
    assert!(
        app.world()
            .get::<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>(ally)
            .is_none(),
        "summoned ally despawned on reset"
    );
    assert!(
        app.world()
            .get::<crate::features::HeldItem>(player)
            .is_none(),
        "held item removed from player"
    );
    #[cfg(feature = "portal")]
    assert!(
        app.world()
            .get::<ambition_portal::PortalGun>(player)
            .is_none(),
        "portal gun removed from player"
    );
    assert!(
        app.world()
            .get::<crate::items::pickup::StashedActionSet>(player)
            .is_none(),
        "stashed action set cleared"
    );
}

fn dummy_world() -> ae::World {
    ae::World::new(
        "test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 1000.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 1500.0),
            ae::Vec2::new(2000.0, 32.0),
        )],
    )
}

/// Build a minimal Bevy app wired with the reset processor and
/// just enough resources for it to run: the request resource,
/// the save, the three registries it clears, the music request,
/// runtime + world + room set + tuning, and the relevant entity
/// queries (empty here — no controllers / no room visuals to
/// despawn in this synthetic harness).
fn min_app() -> App {
    let mut app = App::new();
    let world = dummy_world();
    app.insert_resource(SandboxResetRequested::default());
    app.insert_resource(SandboxSave::default());
    app.insert_resource(EncounterRegistry::default());
    app.insert_resource(BossEncounterRegistry::default());
    app.insert_resource(QuestRegistry::default());
    app.insert_resource(EncounterMusicRequest::default());
    app.insert_resource(crate::features::GameplayBanner::default());
    // Spawn the player entity so process_sandbox_reset_request can query it.
    // Uses the full simulation bundle so every cluster component lands
    // — the reset path queries `BodyClusterQueryData` which needs all
    // of them present.
    {
        let mut initial =
            crate::player::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all());
        ae::refresh_movement_resources_clusters(
            &initial.abilities,
            &mut initial.dash,
            &mut initial.jump,
            ae::DEFAULT_TUNING,
        );
        let health = ambition_characters::actor::Health::new(20);
        app.world_mut()
            .spawn(crate::player::PlayerSimulationBundle::from_scratch(
                initial, health,
            ));
        let _ = PlayerBlinkCameraState::default();
    }
    app.insert_resource(crate::world::physics::PhysicsSandboxSettings::default());
    app.insert_resource(crate::MovingPlatformSet::default());
    app.insert_resource(crate::SandboxSimState::default());
    app.insert_resource(ambition_time::ClockState::default());
    app.insert_resource(ambition_dev_tools::SandboxDevState::default());
    app.insert_resource(RoomGeometry(world.clone()));
    // Construct a minimal RoomSet with one room so `start` and
    // `active` are both valid indices.
    let room_spec = crate::rooms::RoomSpec {
        id: "test".into(),
        world: world.clone(),
        loading_zones: Vec::new(),
        metadata: crate::rooms::RoomMetadata::default(),
        camera_zones: Vec::new(),
        kinematic_paths: Vec::new(),
        moving_platforms: Vec::new(),
        props: Vec::new(),
        ground_items: Vec::new(),
        portal_gun_spawns: Vec::new(),
        portals: Vec::new(),
        shrines: Vec::new(),
        gravity_zones: Vec::new(),
        hazards: Vec::new(),
        enemy_spawns: Vec::new(),
        boss_spawns: Vec::new(),
        debug_labels: Vec::new(),
        mount_links: Vec::new(),
        placements: Vec::new(),
    };
    app.insert_resource(crate::rooms::RoomSet::from_parts(
        "test",
        vec![room_spec],
        Vec::new(),
    ));
    app.insert_resource(EditableMovementTuning::default());
    // The processor now emits `RespawnRoomVisualsRequested` instead of spawning
    // visuals inline (the render layer consumes it); register the message so the
    // headless test app can run the system. Restaging the start room also
    // emits the `RoomLoaded` staging fact (JD4).
    app.add_message::<crate::session::RespawnRoomVisualsRequested>();
    app.add_message::<crate::rooms::RoomLoaded>();
    app.add_message::<crate::time::time_control::ClockResetRequest>();
    app.add_systems(Update, process_sandbox_reset_request);
    app
}

/// Sanity: with no request, the processor leaves state alone.
/// Set a save flag, run a tick, confirm it's still set.
#[test]
fn processor_is_a_noop_without_request() {
    let mut app = min_app();
    {
        let mut save = app.world_mut().resource_mut::<SandboxSave>();
        save.data_mut().set_flag("npc_test_hostile", true);
    }
    app.update();
    let save = app.world().resource::<SandboxSave>();
    assert!(save.data().flag("npc_test_hostile"));
}

/// The headline behavior: a queued request wipes the save flags
/// (the thing the user noticed — NPCs persisting as dead) and
/// flips registries back to "specs not loaded" so the populate
/// systems repopulate on the next frame.
#[test]
fn processor_wipes_save_flags_and_clears_registries() {
    let mut app = min_app();
    // Pre-populate the state the user is trying to reset:
    // - a save flag remembering an NPC turned hostile
    // - a save flag remembering an encounter chest was looted
    // - "specs already loaded" on the registries
    {
        let mut save = app.world_mut().resource_mut::<SandboxSave>();
        save.data_mut().set_flag("npc_kira_hostile", true);
        save.data_mut()
            .set_flag("encounter_goblin_encounter_reward_dropped", true);
        save.data_mut().set_encounter(
            "goblin_encounter",
            ambition_persistence::save_data::PersistedEncounterState::Cleared,
        );
    }
    {
        let mut reg = app.world_mut().resource_mut::<EncounterRegistry>();
        reg.specs_loaded = true;
    }
    {
        let mut reg = app.world_mut().resource_mut::<BossEncounterRegistry>();
        reg.specs_loaded = true;
    }
    {
        let mut reg = app.world_mut().resource_mut::<QuestRegistry>();
        reg.initialized = true;
    }
    // Queue the reset.
    {
        let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
        req.request();
    }
    app.update();

    // Save is wiped.
    let save = app.world().resource::<SandboxSave>();
    assert!(!save.data().flag("npc_kira_hostile"));
    assert!(!save
        .data()
        .flag("encounter_goblin_encounter_reward_dropped"));
    assert_eq!(
        save.data().encounter("goblin_encounter"),
        ambition_persistence::save_data::PersistedEncounterState::Untouched
    );
    // Registries flag-flipped back so populate Update systems
    // will re-run on the next frame.
    let enc = app.world().resource::<EncounterRegistry>();
    assert!(!enc.specs_loaded);
    let boss = app.world().resource::<BossEncounterRegistry>();
    assert!(!boss.specs_loaded);
    let quest = app.world().resource::<QuestRegistry>();
    assert!(!quest.initialized);
    // Banner surfaces the action so the player can see it.
    assert_eq!(
        app.world()
            .resource::<crate::features::GameplayBanner>()
            .text,
        "SANDBOX RESET"
    );
    // Request consumed.
    let req = app.world().resource::<SandboxResetRequested>();
    assert!(!req.request);
}

/// After reset, the player is warped to the start room's spawn
/// regardless of where they were before the reset. This is the
/// "back to a fresh game" guarantee.
#[test]
fn processor_warps_player_to_start_spawn() {
    let mut app = min_app();
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut crate::actor::BodyKinematics, With<crate::actor::PlayerEntity>>(
            );
        if let Ok(mut kin) = q.single_mut(app.world_mut()) {
            kin.pos = ae::Vec2::new(1234.0, 1234.0);
        }
    }
    {
        let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
        req.request();
    }
    app.update();
    let world = app.world().resource::<RoomGeometry>();
    let expected_spawn = world.0.spawn;
    let mut q = app
        .world_mut()
        .query_filtered::<&crate::actor::BodyKinematics, With<crate::actor::PlayerEntity>>();
    let player_pos = q.single(app.world()).map(|k| k.pos).unwrap();
    assert_eq!(player_pos, expected_spawn);
}

/// Reset must restore the moving platform from the start room's
/// authored LDtk platform, not from the old procedural fallback.
#[test]
fn processor_restores_authored_start_room_platform() {
    let mut app = min_app();
    let authored = crate::world::platforms::MovingPlatformState::from_authored(
        ae::Vec2::new(512.0, 900.0),
        ae::Vec2::new(128.0, 16.0),
        192.0,
        75.0,
    );
    {
        let mut room_set = app.world_mut().resource_mut::<RoomSet>();
        room_set.rooms[0].moving_platforms = vec![authored.clone()];
    }
    {
        let mut platform_set = app.world_mut().resource_mut::<crate::MovingPlatformSet>();
        platform_set.0 = vec![crate::world::platforms::MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            64.0,
            10.0,
        )];
    }
    {
        let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
        req.request();
    }
    app.update();
    let platform_set = app.world().resource::<crate::MovingPlatformSet>();
    assert_eq!(platform_set.0[0].pos, authored.pos);
    assert_eq!(platform_set.0[0].size, authored.size);
}
