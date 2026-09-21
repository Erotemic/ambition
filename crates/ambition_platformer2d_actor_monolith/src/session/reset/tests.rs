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
        .spawn(ambition_held_items::GroundItem::at_rest(
            ambition_held_items::axe_spec(),
            ae::Vec2::ZERO,
            ae::Vec2::splat(18.0),
        ))
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
                ambition_held_items::StashedActionSet(
                    ambition_characters::brain::ActionSet::default(),
                ),
                ambition_combat::held_items::HeldItem::new(ambition_held_items::axe_spec()),
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
        .get::<ambition_held_items::GroundItem>(ground)
        .is_some());
    assert!(app
        .world()
        .get::<ambition_combat::held_items::HeldItem>(player)
        .is_some());

    // A reset that was ASKED FOR but refused: the request resource is set and
    // no commitment was announced. Nothing may be taken away.
    app.insert_resource(NewGameResetRequested { request: true });
    app.update();
    assert!(
        app.world()
            .get::<ambition_combat::held_items::HeldItem>(player)
            .is_some(),
        "a refused reset emptied the player's hands. The decline path promises \
         the running session is untouched; this is the system that has to make \
         that true."
    );
    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(ground)
            .is_some(),
        "a refused reset despawned a dropped item"
    );

    // Committed → transient entities despawn + player held-state stripped.
    app.world_mut().write_message(NewGameResetCommitted);
    app.update();
    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(ground)
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
            .get::<ambition_combat::held_items::HeldItem>(player)
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
            .get::<ambition_held_items::StashedActionSet>(player)
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
/// ROOM scope is the line, and it is the line precisely because
/// [`process_new_game_reset_request`] — named two paragraphs up — already owns
/// that side. ⚠ This said `retire_outgoing` until 2026-09-19, a method A10  <!-- cite-ok: names the method A10 DELETED on 2026-09-14; this sentence RECORDS the dead name, so a resolvable citation here would mean the deletion did not happen -->
/// deleted on 2026-09-14; the sweep this paragraph is about is the reset's, not
/// the room transition's.
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
            ambition_held_items::GroundItem::at_rest(
                ambition_held_items::axe_spec(),
                ae::Vec2::new(64.0, 0.0),
                ae::Vec2::splat(18.0),
            ),
        ))
        .id();
    // Residue of the session that is being thrown away: a weapon dropped by a
    // defeated body is `spawn_session_scoped`, so no room sweep will ever take
    // it back — this system is its only retirement.
    let dropped = app
        .world_mut()
        .spawn(ambition_held_items::GroundItem::at_rest(
            ambition_held_items::axe_spec(),
            ae::Vec2::new(-64.0, 0.0),
            ae::Vec2::splat(18.0),
        ))
        .id();

    app.world_mut().write_message(NewGameResetCommitted);
    app.update();

    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(authored)
            .is_some(),
        "the reset despawned the pickup it had just authored from the room's \
         own records, so the rebuilt room came back short of itself"
    );
    assert!(
        app.world()
            .get::<ambition_held_items::GroundItem>(dropped)
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
    min_app_that_can_hide_a_candidate(true)
}

/// As [`min_app`], choosing whether the world can hide an inactive candidate.
///
/// ⛔ THE COMPOSITION PRODUCTION BUILDS IS `true`. Room construction mints every
/// root hidden — unconditionally, since 2026-09-15 — and `transaction::open`
/// REFUSES a world that cannot
/// hide one, rather than validating candidates in plain sight. `false` is that
/// refusal — a real production one, on the road production uses, and the only
/// injection this harness can make without a second content generation.
fn min_app_that_can_hide_a_candidate(filter: bool) -> App {
    let mut app = App::new();
    if filter {
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
    }
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
            ae::RecoveryRefresh::Answered,
        );
        let health = ambition_characters::actor::Health::new(20);
        app.world_mut()
            .spawn(crate::avatar::PlayerSimulationBundle::from_scratch(
                initial, health,
            ));
        let _ = PlayerBlinkCameraState::default();
    }
    app.insert_resource(
        ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings::default(),
    );
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
    // ⛔⛤ **THE SCOPED CONSTRUCTION AUTHORITY, DECLARED, because a reset
    // rebuilds a LIVE room and there is no longer an App-registry road to
    // rebuild it from.** This fixture used to get one for free: the processor
    // read five App resources whenever no generation had been activated, which
    // is the anonymous fallback the 2026-09-19 composition ruling deleted
    // (*"explicit direct/headless/test compositions may hold scoped
    // fixture/direct-entry authority where needed, but no anonymous App-global
    // fallback state returns"*). Stating it is the fixture doing what a
    // composition is now required to do.
    //
    // ⚠ AND IT IS LOAD-BEARING FOR THE REFUSAL ARMS TOO: without it
    // `a_declined_reset_leaves_the_running_session_untouched` would decline at
    // the mechanics gate and pass without ever reaching the missing room it is
    // about.
    app.init_resource::<crate::session::mechanics::SessionMechanics>();
    app.insert_resource(EditableMovementTuning::default());
    app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
    // The processor now emits `RespawnRoomVisualsRequested` instead of spawning
    // visuals inline (the render layer consumes it); register the message so the
    // headless test app can run the system. Restaging the start room also
    // emits the `RoomLoaded` staging fact (JD4).
    app.add_message::<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>();
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

/// ⛔⛤ **A10: A RESET WHOSE START ROOM IS REFUSED WIPES NOTHING.**
///
/// The reset used to say *"past the point of refusal"* and then destroy the save,
/// the registries, the remembered occurrences and the player's position — before
/// anything had verified that the start room could be built at all. The room
/// transaction CAN refuse, and under that order the refusal arrived after there
/// was nothing left to go back to.
///
/// ⚠ **THE REFUSAL IS A PRODUCTION ONE.** A composition that cannot hide an
/// inactive candidate is refused by `transaction::open` rather than building
/// candidates in plain sight; this harness simply does not register the filter.
#[test]
fn a_reset_whose_start_room_is_refused_wipes_nothing() {
    let mut app = min_app_that_can_hide_a_candidate(false);
    let platform = ambition_platformer2d_world::platforms::MovingPlatformState::from_authored(
        ae::Vec2::new(10.0, 20.0),
        ae::Vec2::new(32.0, 8.0),
        64.0,
        10.0,
    );
    {
        let mut platform_set = app
            .world_mut()
            .resource_mut::<ambition_platformer2d_world::collision::MovingPlatformSet>();
        platform_set.0 = vec![platform.clone()];
    }
    // ⛔ THE PREMISE: the save must have something in it, or "the save survived"
    // is true of an empty one.
    {
        let mut save = app.world_mut().resource_mut::<AmbitionGameSave>();
        save.data_mut()
            .set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint {
                room_id: "somewhere".into(),
                x: 12,
                y: 34,
            });
    }
    let before_player = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d_core::BodyKinematics,
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
        >();
        q.single(world).expect("the harness has a player").pos
    };

    app.world_mut()
        .resource_mut::<NewGameResetRequested>()
        .request();
    app.update();

    assert!(
        !app.world()
            .resource::<crate::features::LastConstructionVerification>()
            .published,
        "the start room PUBLISHED, so this arm is not about a refused reset"
    );
    assert!(
        app.world()
            .resource::<AmbitionGameSave>()
            .data()
            .checkpoint()
            .is_some(),
        "⛔ a reset whose room was REFUSED wiped the save anyway: the player has \
         lost their run and gained no world to play it in"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d_world::collision::MovingPlatformSet>()
            .0
            .len(),
        1,
        "the live platform state moved under a refused reset"
    );
    let after_player = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d_core::BodyKinematics,
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
        >();
        q.single(world).expect("the harness has a player").pos
    };
    assert_eq!(
        after_player, before_player,
        "a refused reset warped the player to the spawn of a room that was never \
         built"
    );
    // ⛔⛤ **AND THE SIGNAL FOUR OTHER DOMAINS ACT ON, WHICH NOTHING ON THIS ROAD
    // CHECKED.** Every assertion above reads a value `process_new_game_reset_request`
    // writes itself, so all of them are satisfied by a system that refuses
    // correctly and ANNOUNCES the commit anyway. Gravity, `items/persist`,
    // `durable_horizon` and the session teardown bundle all key on
    // `NewGameResetCommitted` rather than on the request, so that announcement is
    // what actually reaches them.
    //
    // ⚠ `a_declined_reset_leaves_the_running_session_untouched` asserts the same
    // emptiness and CANNOT stand in for this: it refuses at preparation, before
    // the staged closure runs, so it never reaches the verification this arm is
    // about. Measured by poison 2026-09-18 — writing the message above the
    // `publication_succeeded` check left all ten arms in this module green.
    assert!(
        app.world()
            .resource::<Messages<NewGameResetCommitted>>()
            .is_empty(),
        "the start room was REFUSED and the reset announced a COMMIT anyway. \
         Nothing was wiped, and every teardown domain keyed on that message just \
         tore down a session that is still running"
    );
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
    // Point the session at a room that does not exist. Preparation must refuse
    // with `RoomConstructionError::UnknownRoom`, which is the decline this test
    // is about — NOT the mechanics gate above it, which the fixture declares a
    // generation specifically to get past.
    //
    // ⛔⛤ **THE MISSING ROOM IS A SET WITH NO ROOMS, BECAUSE `start = 999` IS
    // NO LONGER A STATE THAT EXISTS — 2026-09-20.** This used to reach into the
    // component and write an out-of-range index by hand. `RoomSet::start` is
    // private now and both roads that write it resolve an authored id first, so
    // an index past the end is unconstructible and a fixture that injected one
    // was rehearsing an impossible world. An EMPTY set is reachable through the
    // ordinary constructor and lands on the same refusal: index 0 of no rooms.
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
        *rooms = ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "a_room_this_session_does_not_have",
            Vec::new(),
            Vec::new(),
        );
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
