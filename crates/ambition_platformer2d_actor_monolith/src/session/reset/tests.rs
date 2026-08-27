//! Unit tests for the sandbox reset flow: idle-by-default request resource,
//! request/consume edge behavior, and the post-reset gameplay-state rebuild.

use super::*;
use ambition_dev_tools::dev_tools::EditableMovementTuning;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState;

/// Pin the request resource's defaults: a fresh app starts with
/// no reset queued. Important because the reset processor must
/// be a no-op when nothing has been requested.
#[test]
fn request_default_is_idle() {
    let req = NewGameResetRequested::default();
    assert!(!req.request);
}

/// `request()` sets the flag; the processor consumes it.
#[test]
fn request_helper_sets_the_flag() {
    let mut req = NewGameResetRequested::default();
    req.request();
    assert!(req.request);
}

/// The transient clear follows the COMMITMENT, and a bare request — which is
/// what a reset whose preflight refuses leaves behind — clears nothing.
#[test]
fn sandbox_reset_clears_portals_held_items_and_summons() {
    let mut app = App::new();
    app.add_message::<NewGameResetCommitted>();
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
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
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
        .insert(ambition_portal2d::PortalGun::default());

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

    // A reset that was ASKED FOR but refused: the request resource is set and
    // no commitment was announced. Nothing may be taken away.
    app.insert_resource(NewGameResetRequested { request: true });
    app.update();
    assert!(
        app.world()
            .get::<crate::features::HeldItem>(player)
            .is_some(),
        "a refused reset emptied the player's hands. The decline path promises \
         the running session is untouched; this is the system that has to make \
         that true."
    );
    assert!(
        app.world()
            .get::<crate::items::pickup::GroundItem>(ground)
            .is_some(),
        "a refused reset despawned a dropped item"
    );

    // Committed → transient entities despawn + player held-state stripped.
    app.world_mut().write_message(NewGameResetCommitted);
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
            .get::<ambition_portal2d::PortalGun>(player)
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

/// THE ROOM IS ALREADY REBUILT WHEN THIS SYSTEM RUNS, SO IT MAY NOT SWEEP THE
/// ROOM.
///
/// `process_new_game_reset_request` retires every `RoomScopedEntity` and commits
/// a fresh start-room plan in the same call, and the `.chain()` between the two
/// systems carries an auto-inserted `ApplyDeferred` — so every room-scoped
/// ground item this system can see is one the reset AUTHORED a sync point ago
/// from the room's own records. A blanket `With<GroundItem>` sweep despawned
/// exactly those, and a reset taken in a room with an authored pickup rebuilt
/// that room permanently one pickup short of itself.
///
/// ROOM scope is the line, and it is the line precisely because `retire_outgoing` already owns that
/// side.
#[test]
fn the_transient_clear_spares_the_rebuilt_rooms_own_items() {
    let mut app = App::new();
    app.add_message::<NewGameResetCommitted>();
    app.add_systems(Update, clear_transient_on_sandbox_reset);

    // What the reset's room plan just authored: room-scoped, exactly as
    // `spawn_ground_item_resolved_into` builds it (`insert_room_in_session`).
    let authored = app
        .world_mut()
        .spawn((
            RoomScopedEntity,
            crate::items::pickup::GroundItem {
                spec: crate::items::pickup::axe_spec(),
                pos: ae::Vec2::new(64.0, 0.0),
                vel: ae::Vec2::ZERO,
                half_extent: ae::Vec2::splat(18.0),
            },
        ))
        .id();
    // Residue of the session that is being thrown away: a weapon dropped by a
    // defeated body is `spawn_session_scoped`, so no room sweep will ever take
    // it back — this system is its only retirement.
    let dropped = app
        .world_mut()
        .spawn(crate::items::pickup::GroundItem {
            spec: crate::items::pickup::axe_spec(),
            pos: ae::Vec2::new(-64.0, 0.0),
            vel: ae::Vec2::ZERO,
            half_extent: ae::Vec2::splat(18.0),
        })
        .id();

    app.world_mut().write_message(NewGameResetCommitted);
    app.update();

    assert!(
        app.world()
            .get::<crate::items::pickup::GroundItem>(authored)
            .is_some(),
        "the reset despawned the pickup it had just authored from the room's \
         own records, so the rebuilt room came back short of itself"
    );
    assert!(
        app.world()
            .get::<crate::items::pickup::GroundItem>(dropped)
            .is_none(),
        "a session-scoped dropped weapon survived the sandbox reset — nothing \
         else retires one, so sparing it leaks the old attempt into the new game"
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
    app.insert_resource(NewGameResetRequested::default());
    app.insert_resource(AmbitionGameSave::default());
    app.insert_resource(EncounterRegistry::default());
    app.insert_resource(BossEncounterRegistry::default());
    app.insert_resource(QuestRegistry::default());
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        EncounterMusicRequest::default(),
    );
    app.insert_resource(ambition_combat::events::GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    // Explicit content-free boss authority: the reset processor reads
    // `Res<BossCatalog>` (required, not optional) to rebuild encounter state.
    app.insert_resource(ambition_boss_encounter::BossCatalog::default());
    // Spawn the player entity so process_new_game_reset_request can query it.
    // Uses the full simulation bundle so every cluster component lands
    // — the reset path queries `BodyClusterQueryData` which needs all
    // of them present.
    {
        let mut initial =
            crate::avatar::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all());
        ae::refresh_movement_resources_clusters(
            &initial.abilities,
            &mut initial.dash,
            &mut initial.jump,
            &mut initial.dodge,
            ae::DEFAULT_TUNING.air_jumps,
        );
        let health = ambition_characters::actor::Health::new(20);
        app.world_mut()
            .spawn(crate::avatar::PlayerSimulationBundle::from_scratch(
                initial, health,
            ));
        let _ = PlayerBlinkCameraState::default();
    }
    app.insert_resource(crate::world::physics::PhysicsSandboxSettings::default());
    // The reset processor re-stages the start room through the App-installed
    // placement-lowering authority (7d972b6); the minimal app must provide it.
    app.insert_resource(crate::world::placements::PlacementLoweringRegistry::default());
    app.insert_resource(crate::construction::engine_construction_registry());
    app.insert_resource(crate::features::RoomContentStagingRegistry::default());
    app.insert_resource(ambition_platformer2d_world::collision::MovingPlatformSet::default());
    app.insert_resource(
        ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default(),
    );
    app.insert_resource(ambition_time::ClockState::default());
    app.insert_resource(ambition_dev_tools::DeveloperRuntimeState::default());
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        RoomGeometry(world.clone()),
    );
    // Construct a minimal RoomSet with one room so `start` and
    // `active` are both valid indices.
    let room_spec = ambition_platformer2d_world::rooms::RoomSpec {
        id: "test".into(),
        world: world.clone(),
        loading_zones: Vec::new(),
        metadata: ambition_platformer2d_world::rooms::RoomMetadata::default(),
        camera_zones: Vec::new(),
        kinematic_paths: Vec::new(),
        moving_platforms: Vec::new(),
        props: Vec::new(),
        ground_items: Vec::new(),
        portal_gun_spawns: Vec::new(),
        shrines: Vec::new(),
        gravity_zones: Vec::new(),
        enemy_spawns: Vec::new(),
        boss_spawns: Vec::new(),
        debug_labels: Vec::new(),
        mount_links: Vec::new(),
        placements: Vec::new(),
        encounter_triggers: Vec::new(),
        lock_walls: Vec::new(),
        switch_commands: Vec::new(),
    };
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "test",
            vec![room_spec],
            Vec::new(),
        ),
    );
    app.insert_resource(EditableMovementTuning::default());
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    // The processor now emits `RespawnRoomVisualsRequested` instead of spawning
    // visuals inline (the render layer consumes it); register the message so the
    // headless test app can run the system. Restaging the start room also
    // emits the `RoomLoaded` staging fact (JD4).
    app.add_message::<crate::session::RespawnRoomVisualsRequested>();
    app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
    app.add_message::<ambition_time::time_control::ClockResetRequest>();
    app.add_message::<NewGameResetCommitted>();
    app.add_systems(Update, process_new_game_reset_request);
    app
}

/// Sanity: with no request, the processor leaves state alone.
/// Set a save flag, run a tick, confirm it's still set.
#[test]
fn processor_is_a_noop_without_request() {
    let mut app = min_app();
    {
        let mut save = app.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut().set_flag("npc_test_hostile", true);
    }
    app.update();
    let save = app.world().resource::<AmbitionGameSave>();
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
        let mut save = app.world_mut().resource_mut::<AmbitionGameSave>();
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
        let mut req = app.world_mut().resource_mut::<NewGameResetRequested>();
        req.request();
    }
    app.update();

    // Save is wiped.
    let save = app.world().resource::<AmbitionGameSave>();
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
            .resource::<ambition_combat::events::GameplayBanner>()
            .text,
        "SANDBOX RESET"
    );
    // Request consumed.
    let req = app.world().resource::<NewGameResetRequested>();
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
            .query_filtered::<&mut ambition_platformer2d_core::BodyKinematics, With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>>(
            );
        if let Ok(mut kin) = q.single_mut(app.world_mut()) {
            kin.pos = ae::Vec2::new(1234.0, 1234.0);
        }
    }
    {
        let mut req = app.world_mut().resource_mut::<NewGameResetRequested>();
        req.request();
    }
    app.update();
    let world = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        RoomGeometry,
    >(app.world())
    .expect("session room geometry");
    let expected_spawn = world.0.spawn;
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d_core::BodyKinematics, With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>>();
    let player_pos = q.single(app.world()).map(|k| k.pos).unwrap();
    assert_eq!(player_pos, expected_spawn);
}

/// Reset must restore the moving platform from the start room's
/// authored LDtk platform, not from the old procedural fallback.
#[test]
fn processor_restores_authored_start_room_platform() {
    let mut app = min_app();
    let authored = ambition_platformer2d_world::platforms::MovingPlatformState::from_authored(
        ae::Vec2::new(512.0, 900.0),
        ae::Vec2::new(128.0, 16.0),
        192.0,
        75.0,
    );
    {
        let mut room_set =
            ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<RoomSet>(
                app.world_mut(),
            )
            .expect("session room set");
        room_set.rooms[0].moving_platforms = vec![authored.clone()];
    }
    {
        let mut platform_set =
            app.world_mut()
                .resource_mut::<ambition_platformer2d_world::collision::MovingPlatformSet>();
        platform_set.0 = vec![
            ambition_platformer2d_world::platforms::MovingPlatformState::from_authored(
                ae::Vec2::new(10.0, 20.0),
                ae::Vec2::new(32.0, 8.0),
                64.0,
                10.0,
            ),
        ];
    }
    {
        let mut req = app.world_mut().resource_mut::<NewGameResetRequested>();
        req.request();
    }
    app.update();
    let platform_set = app
        .world()
        .resource::<ambition_platformer2d_world::collision::MovingPlatformSet>();
    assert_eq!(platform_set.0[0].pos, authored.pos);
    assert_eq!(platform_set.0[0].size, authored.size);
}

/// A DECLINED reset leaves the session exactly as it was.
///
/// Its completion criterion in as many words: *"a failed preparation leaves the
/// current session byte-for-byte semantically unchanged except for
/// diagnostics."* The decline path is documented in the processor and one test
/// pins that teardown waits for the COMMIT rather than the request — but nothing
/// asserted the criterion, and "the running session is untouched" was a comment.
#[test]
fn a_declined_reset_leaves_the_running_session_untouched() {
    let mut app = min_app();
    {
        let mut save = app.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut().set_flag("npc_kira_hostile", true);
    }
    {
        let mut reg = app.world_mut().resource_mut::<EncounterRegistry>();
        reg.specs_loaded = true;
    }
    // Point the session at a room that does not exist. Preparation must refuse.
    //
    // the `RoomSet` is a session-world COMPONENT, not a resource — it belongs
    // to the session root so it dies with the session rather than outliving it
    // as a global.
    {
        let mut rooms =
            ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<
                ambition_platformer2d_world::rooms::RoomSet,
            >(app.world_mut())
            .expect("the fixture staged a room set");
        rooms.start = 999;
    }
    {
        let mut req = app.world_mut().resource_mut::<NewGameResetRequested>();
        req.request();
    }
    app.update();

    assert!(
        app.world()
            .resource::<AmbitionGameSave>()
            .data()
            .flag("npc_kira_hostile"),
        "a reset that could not be prepared still wiped the save. The preflight \
         runs before the wipe precisely so a refusal costs nothing."
    );
    assert!(
        app.world().resource::<EncounterRegistry>().specs_loaded,
        "a declined reset cleared the registries anyway"
    );
    let committed = app.world().resource::<Messages<NewGameResetCommitted>>();
    assert!(
        committed.is_empty(),
        "a declined reset announced a COMMIT, so every teardown system keyed on \
         it would have run against a session that was never replaced"
    );
}
