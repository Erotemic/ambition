use super::*;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;
use ambition_boss_encounter::pattern::profile::BossBehaviorProfile;
use ambition_boss_encounter::BossBehaviorProfileExt;
use ambition_boss_encounter::BossClusterScratch;
use ambition_platformer2d_runtime::world_overlay::rebuild_feature_ecs_world_overlay;
use ambition_platformer2d_world::collision::world_with_sandbox_solids;

/// The composited collision view (immutable base + this frame's overlay).
/// The gate is now a derived overlay contributor, so the arena assertions
/// read the VIEW — what player/actor collision actually sees — not the base.
fn arena_view(app: &App) -> ae::World {
    let base = &ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
        RoomGeometry,
    >(app.world())
    .expect("session room geometry")
    .0;
    let overlay = ambition_platformer2d::session::sole_live_room_component::<FeatureEcsWorldOverlay>(app.world()).expect("the live room has a collision overlay");
    world_with_sandbox_solids(base, &[], overlay)
}

fn make_game_world(
    name: &str,
    ladders: Vec<ae::ClimbableRegion>,
) -> ambition_platformer2d_core::RoomGeometry {
    let world = ae::World::new(
        name,
        ae::Vec2::new(2_000.0, 2_000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
    .with_climbable_regions(ladders);
    ambition_platformer2d_core::RoomGeometry(world)
}

fn make_game_world_with_floor_gate(
    name: &str,
    ladders: Vec<ae::ClimbableRegion>,
) -> ambition_platformer2d_core::RoomGeometry {
    let gate_block = ae::Block::solid(
        FLOOR_GATE_BLOCK_NAME,
        ae::Vec2::new(112.0, 208.0),
        ae::Vec2::new(48.0, 16.0),
    );
    let world = ae::World::new(
        name,
        ae::Vec2::new(2_000.0, 2_000.0),
        ae::Vec2::ZERO,
        vec![gate_block],
    )
    .with_climbable_regions(ladders);
    ambition_platformer2d_core::RoomGeometry(world)
}

fn floor_gate_count(app: &App) -> usize {
    arena_view(app)
        .blocks
        .iter()
        .filter(|b| b.name == FLOOR_GATE_BLOCK_NAME)
        .count()
}

/// The arena's encounter boss: the `gnu_ton_rider` scholar aboard the
/// `giant_gnu` mount (ADR 0020 / G4). GNU-ton IS this boss — the fused
/// single profile was torn down in the E6 teardown.
fn spawn_gnu_ton_runtime() -> BossClusterScratch {
    let behavior =
        BossBehaviorProfile::from_data(&crate::bosses::shipped_boss_catalog(), "gnu_ton_rider");
    let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(54.0, 96.0));
    let pos = ae::Vec2::new(500.0, 400.0);
    let aabb = ae::Aabb::new(pos, combat_size * 0.5);
    let catalog = crate::bosses::shipped_boss_catalog();
    let mut scratch = BossClusterScratch::new(
        &catalog,
        "boss_gnu_ton_rider",
        "GNU-ton",
        aabb,
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    scratch.config.behavior = behavior;
    scratch
}

/// A boss wearing the GIANT's sheet — the only sheet whose body geometry is
/// authored PER ANIMATION. Since the ADR-0020 split the giant is a mount
/// ACTOR, so no live boss targets it; but the boss sprite-metrics machinery
/// still has to handle such a sheet (the fused GNU-ton was exactly this
/// shape), and the head-hurtbox alignment guard below is what pins it.
fn spawn_giant_bodied_boss_runtime() -> BossClusterScratch {
    let mut behavior =
        BossBehaviorProfile::from_data(&crate::bosses::shipped_boss_catalog(), "gnu_ton_rider");
    behavior.sprite_target = Some("giant_gnu".to_string());
    let combat_size = ae::Vec2::new(220.0, 220.0);
    behavior.combat_size = Some(combat_size);
    let pos = ae::Vec2::new(500.0, 400.0);
    let aabb = ae::Aabb::new(pos, combat_size * 0.5);
    let catalog = crate::bosses::shipped_boss_catalog();
    let mut scratch = BossClusterScratch::new(
        &catalog,
        "boss_giant_gnu",
        "Giant GNU",
        aabb,
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    scratch.config.behavior = behavior;
    scratch.resolve_sheet_body(&catalog);
    scratch
}

/// This pins that the hurtbox tracks the body even though it is computed in frame space
/// (frame-center → `boss.pos`) while the body envelope carries `combat_offset`.
#[test]
fn giant_head_hurtbox_overlaps_the_body_envelope() {
    use ambition_platformer2d_core::AabbExt;

    let boss = spawn_giant_bodied_boss_runtime();
    assert!(
        boss.status.sprite_metrics.is_some(),
        "the giant's sprite metrics should resolve from the baked sheet registry"
    );
    let attack = ambition_characters::brain::BossAttackState::default();
    let catalog = crate::bosses::shipped_boss_catalog();
    let ctx = ambition_boss_encounter::attack_geometry::BossVolumeContext::from_ref(
        &catalog,
        boss.as_ref(),
        &attack,
    );
    let hurtboxes = ambition_combat::body_geometry::damageable_volumes(&ctx);
    assert!(
        !hurtboxes.is_empty(),
        "the giant should expose at least one damageable hurtbox at rest"
    );
    let body = boss.as_ref().aabb();
    for hb in &hurtboxes {
        assert!(
            body.strict_intersects(hb.bounds()),
            "hurtbox {hb:?} does not overlap the boss body envelope {body:?} \
             — the head hurtbox has drifted off the visible body (TODO #30)"
        );
    }
}

fn make_app(world: ambition_platformer2d_core::RoomGeometry) -> App {
    make_app_authoring(world, &[&gnu_ton_placement()])
}

/// The session room set of these tests: the arena, whose spec authors a
/// GNU-ton placement for each id (as the shipped arena authors
/// `hall.gnu_ton`), and one other room. A live room is the room its
/// `LiveRoomDefinition` names, as in the game.
fn room_set(placements: &[&str]) -> ambition_platformer2d_world::rooms::RoomSet {
    let mut arena = ambition_platformer2d_world::rooms::RoomSpec::new(
        ARENA_ROOM_NAME,
        make_game_world(ARENA_ROOM_NAME, Vec::new()).0,
    );
    for id in placements {
        arena.boss_spawns.push(ambition_platformer2d_world::rooms::Authored::new(
            *id,
            "GNU-ton",
            ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::splat(40.0)),
            ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id: GNU_TON_ID.to_string() },
        ));
    }
    let other = ambition_platformer2d_world::rooms::RoomSpec::new(
        OTHER_ROOM,
        make_game_world(OTHER_ROOM, Vec::new()).0,
    );
    ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(ARENA_ROOM_NAME, vec![arena, other], Vec::new())
}

const OTHER_ROOM: &str = "some_other_room";

fn definition(id: &str) -> ambition_platformer2d_world::rooms::LiveRoomDefinition {
    room_set(&[]).definition_by_id(id).expect("a room of the test set")
}

fn seat_room_set(app: &mut App, placements: &[&str]) {
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(
            ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(1),
        ),
        room_set(placements),
    ));
}

/// The app of one live room, the room `world` is named for.
fn make_app_authoring(world: ambition_platformer2d_core::RoomGeometry, placements: &[&str]) -> App {
    let mut app = App::new();
    seat_room_set(&mut app, placements);
    let seated = definition(&world.0.name);
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        world,
    );
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(app.world_mut(), seated);
    ambition_platformer2d::session::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    // Mirror the production WorldPrep order: the overlay rebuild clears the
    // per-frame contributions, then the gate re-derives them this frame.
    app.add_systems(
        Update,
        (rebuild_feature_ecs_world_overlay, gate_gnu_ton_arena_ladder).chain(),
    );
    app
}

/// The boss dies, and the boss domain records its placement `Cleared`.
fn kill(app: &mut App, boss: Entity) {
    app.world_mut().get_mut::<ambition_characters::actor::BodyHealth>(boss).unwrap().health.current = 0;
    record_defeat(app, &gnu_ton_placement());
}

/// The save's record of a placement, as the boss domain writes it on the
/// tick a death settles (and a replay retracts it).
fn record(app: &mut App, placement: &str, state: ambition_persistence::save_data::PersistedEncounterState) {
    app.world_mut()
        .get_resource_or_insert_with(ambition_persistence::save::AmbitionGameSave::default)
        .data_mut()
        .set_boss(placement, state);
}

fn record_defeat(app: &mut App, placement: &str) {
    record(app, placement, ambition_persistence::save_data::PersistedEncounterState::Cleared);
}

fn gnu_ton_placement() -> String {
    spawn_gnu_ton_runtime().config.id.clone()
}

fn ladder_aabb() -> ae::Aabb {
    ae::Aabb::new(ae::Vec2::new(136.0, 736.0), ae::Vec2::new(16.0, 512.0))
}

fn climbable_regions_len(app: &App) -> usize {
    arena_view(app).climbable_regions.len()
}

#[test]
fn ladder_is_hidden_on_entry_when_boss_is_alive() {
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
    // Spawn an ECS boss in alive state, just like the real
    // populate-bosses pass would.
    app.world_mut()
        .spawn(spawn_gnu_ton_runtime().into_components());
    app.update();
    assert_eq!(
        climbable_regions_len(&app),
        0,
        "ladder must be removed on first frame in arena while boss is alive"
    );
}

/// G4 (ADR 0020): the reauthored arena spawns the LINKED PAIR — the
/// `gnu_ton_rider` BossSpawn welded to the `giant_gnu` EnemySpawn mount via
/// a `mounted_on` EntityRef — not the fused single `gnu_ton` boss. This
/// pins the full authoring→conversion chain (`convert_boss_spawn` emitting a
/// `mount_links` entry) end-to-end off the embedded sandbox.ldtk.
#[test]
fn arena_spawns_the_adr0020_linked_pair() {
    use ambition_entity_catalog::placements::{BossBrain, CharacterBrain};
    use ambition_platformer2d_ldtk::LdtkProject;

    // `to_room_set` reads the world manifest + resolves spawn display names
    // through stable authored ids; this test names the provider-owned manifest
    // it composes the partial project against.
    let manifest = crate::worlds::world_manifest();
    let mut project =
        LdtkProject::load_default_for_dev(&manifest).expect("sandbox LDtk should load");
    // Compose ONLY the arena area. The full sandbox composes portal rooms
    // whose entities need the `portal_ldtk` feature (off in this test build);
    // the arena itself has no portal entities, so scoping to it keeps the
    // conversion-level assertion (`convert_boss_spawn` → `mount_links`) light.
    project
        .levels
        .retain(|level| level.identifier == ARENA_ROOM_NAME);
    let room_set = project
        .to_room_set(
            &manifest,
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .expect("gnu_ton_arena composes");
    let arena = room_set
        .rooms
        .iter()
        .find(|r| r.id == ARENA_ROOM_NAME)
        .expect("gnu_ton_arena room exists");

    // Exactly one authored mount link: the rider BossSpawn → the giant mount.
    assert_eq!(
        arena.mount_links.len(),
        1,
        "the reauthored arena must emit exactly one mount link (rider → mount)"
    );
    let (rider_id, mount_id) = &arena.mount_links[0];

    // The mount side is the brainless `giant_gnu` EnemySpawn (the carried giant).
    let mount = arena
        .enemy_spawns
        .iter()
        .find(|e| &e.id == mount_id)
        .expect("mount-link target resolves to an authored EnemySpawn");
    assert!(
        matches!(&mount.payload.brain, CharacterBrain::Custom(id) if id == "giant_gnu"),
        "the mount is the giant_gnu archetype, got {:?}",
        mount.payload
    );

    // The rider side is the `gnu_ton_rider` BossSpawn (the encounter boss).
    let rider = arena
        .boss_spawns
        .iter()
        .find(|b| &b.id == rider_id)
        .expect("mount-link source resolves to an authored BossSpawn");
    assert!(
        matches!(
            &rider.payload,
            BossBrain::PhaseScript { script_id } if script_id == "gnu_ton_rider"
        ),
        "the rider is the gnu_ton_rider phase-script boss, got {:?}",
        rider.payload
    );

    // The fused single-boss authoring is gone: nothing spawns `gnu_ton`.
    assert!(
        !arena.boss_spawns.iter().any(|b| matches!(
            &b.payload,
            BossBrain::PhaseScript { script_id } if script_id == "gnu_ton"
        )),
        "the arena must no longer spawn the fused gnu_ton boss — it is the split pair now"
    );
}

#[test]
fn ladder_appears_when_boss_dies() {
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
    let boss_entity = app
        .world_mut()
        .spawn(spawn_gnu_ton_runtime().into_components())
        .id();

    // First tick: boss alive → ladder hidden.
    app.update();
    assert_eq!(climbable_regions_len(&app), 0);

    // Kill the boss; next tick should add the ladder back.
    kill(&mut app, boss_entity);
    app.update();
    let regions = &ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world())
    .expect("session room geometry")
    .0
    .climbable_regions;
    assert_eq!(regions.len(), 1, "ladder should be back after defeat");
    assert_eq!(regions[0].kind, ae::ClimbableKind::Ladder);
}

#[test]
fn floor_gate_block_stays_while_boss_is_alive() {
    // While the boss is alive the ladder_floor_gate block must
    // remain in world.blocks — that's what physically blocks the
    // player from climbing back up to the exit door mid-fight.
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world_with_floor_gate(
        ARENA_ROOM_NAME,
        vec![ladder],
    ));
    app.world_mut()
        .spawn(spawn_gnu_ton_runtime().into_components());
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        floor_gate_count(&app),
        1,
        "floor gate must stay while the boss is alive — otherwise \
         the player can climb back out to skip the fight"
    );
}

#[test]
fn floor_gate_block_is_removed_on_boss_defeat() {
    // The frame the boss dies, the named ladder_floor_gate block
    // should be removed from world.blocks so the player can climb
    // up the ladder and walk back to the exit door.
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world_with_floor_gate(
        ARENA_ROOM_NAME,
        vec![ladder],
    ));
    let boss_entity = app
        .world_mut()
        .spawn(spawn_gnu_ton_runtime().into_components())
        .id();
    app.update();
    assert_eq!(floor_gate_count(&app), 1);
    kill(&mut app, boss_entity);
    app.update();
    assert_eq!(
        floor_gate_count(&app),
        0,
        "floor gate must be dropped on defeat so the player can climb out"
    );
}

#[test]
fn ladder_is_not_added_when_no_boss_present() {
    // Empty boss query ≠ defeat. The gate must NOT reveal the
    // ladder just because the boss runtime hasn't spawned yet.
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        climbable_regions_len(&app),
        0,
        "no boss in world ≠ defeat; ladder must stay hidden"
    );
}

#[test]
fn non_arena_rooms_pass_climbables_through_untouched() {
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(OTHER_ROOM, vec![ladder]));
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        climbable_regions_len(&app),
        1,
        "non-arena ladders must be left alone"
    );
}

#[test]
fn leaving_arena_resets_state_for_next_visit() {
    // Visit 1: enter arena, boss dies, ladder revealed.
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder.clone()]));
    let boss_entity = app
        .world_mut()
        .spawn(spawn_gnu_ton_runtime().into_components())
        .id();
    app.update();
    kill(&mut app, boss_entity);
    app.update();
    assert_eq!(climbable_regions_len(&app), 1);

    // Leave the arena: the live room is another room now.
    *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<
        ambition_platformer2d_world::rooms::LiveRoomDefinition,
    >(app.world_mut())
    .expect("the live room's definition") = definition(OTHER_ROOM);
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world_mut())
    .expect("session room geometry")
    .0 = ae::World::new(
        OTHER_ROOM,
        ae::Vec2::new(2_000.0, 2_000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    );
    app.update();

    // Re-enter the arena.
    *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<
        ambition_platformer2d_world::rooms::LiveRoomDefinition,
    >(app.world_mut())
    .expect("the live room's definition") = definition(ARENA_ROOM_NAME);
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world_mut())
    .expect("session room geometry")
    .0 = ae::World::new(
        ARENA_ROOM_NAME,
        ae::Vec2::new(2_000.0, 2_000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
    .with_climbable_regions(vec![ladder]);
    // The body is gone (a cleared placement is not built again). The
    // record still says the fight is won.
    app.world_mut().despawn(boss_entity);
    app.update();
    assert_eq!(climbable_regions_len(&app), 1, "re-entry after a won fight: the ladder is open");

    // A replay retracts the defeat and builds him again: the fight is on.
    record(&mut app, &gnu_ton_placement(), ambition_persistence::save_data::PersistedEncounterState::Untouched);
    app.world_mut().spawn(spawn_gnu_ton_runtime().into_components());
    app.update();
    assert_eq!(
        climbable_regions_len(&app),
        0,
        "ladder must be hidden again on re-entry when the defeat was retracted"
    );
}

/// ⭐ DEBT 13: THE ARENA OPENS ON THE RECORDED OUTCOME OF EVERY GNU-TON IN
/// IT. Two GNU-tons in one arena: one dead body opened it while the other
/// lived, and a corpse that went closed it again.
#[test]
fn two_gnu_tons_in_one_arena_open_it_only_when_both_are_beaten_and_it_stays_open() {
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app_authoring(
        make_game_world_with_floor_gate(ARENA_ROOM_NAME, vec![ladder]),
        &["boss_gnu_ton_a", "boss_gnu_ton_b"],
    );
    let bosses: Vec<(Entity, String)> = ["boss_gnu_ton_a", "boss_gnu_ton_b"]
        .into_iter()
        .map(|id| {
            let mut scratch = spawn_gnu_ton_runtime();
            scratch.config.id = id.to_string();
            (app.world_mut().spawn(scratch.into_components()).id(), id.to_string())
        })
        .collect();
    app.update();
    assert_eq!((climbable_regions_len(&app), floor_gate_count(&app)), (0, 1), "the premise: the fight is on");

    // A dies and is recorded. B lives.
    app.world_mut().get_mut::<ambition_characters::actor::BodyHealth>(bosses[0].0).unwrap().health.current = 0;
    record_defeat(&mut app, &bosses[0].1);
    app.update();
    assert_eq!(
        (climbable_regions_len(&app), floor_gate_count(&app)),
        (0, 1),
        "one GNU-ton of two is beaten: the arena stays shut"
    );

    app.world_mut().get_mut::<ambition_characters::actor::BodyHealth>(bosses[1].0).unwrap().health.current = 0;
    record_defeat(&mut app, &bosses[1].1);
    app.update();
    assert_eq!((climbable_regions_len(&app), floor_gate_count(&app)), (1, 0), "both are beaten: open");

    // Both bodies go. Their records stay, and so does the outcome.
    for (entity, _) in &bosses {
        app.world_mut().despawn(*entity);
    }
    app.update();
    assert_eq!(
        (climbable_regions_len(&app), floor_gate_count(&app)),
        (1, 0),
        "the outcome is the record, not the corpses"
    );
}

/// The arena is the room whose spec id is the arena's, whatever its
/// geometry is called: the shipped arena's geometry is "Ambition: gnu ton
/// arena", and a gate that compared that name never ran in the game.
#[test]
fn the_arena_is_known_by_its_room_id_not_by_its_display_name() {
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world_mut())
    .expect("session room geometry")
    .0
    .name = "Ambition: gnu ton arena".to_string();
    app.world_mut().spawn(spawn_gnu_ton_runtime().into_components());
    app.update();
    assert_eq!(climbable_regions_len(&app), 0, "the fight is on in the arena: the ladder is hidden");
}

/// A dead body with no record is not a won fight (a body a test or a script
/// zeroed): the boss domain records the defeat when the death settles.
#[test]
fn a_dead_body_with_no_record_does_not_open_the_arena() {
    let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
    let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
    let boss = app.world_mut().spawn(spawn_gnu_ton_runtime().into_components()).id();
    app.world_mut().get_mut::<ambition_characters::actor::BodyHealth>(boss).unwrap().health.current = 0;
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(climbable_regions_len(&app), 0);
}

/// OW1 cut 7: each live arena gates its ladder by its own boss. Two live
/// rooms are the arena: in #0 the boss lives, in #1 it is dead. The subject:
/// #0's overlay hides the ladder and #1's opens it and its floor gate. When
/// the gate read the sole live room, it did not run while two rooms were
/// live, and neither room's ladder was hidden.
#[test]
fn each_live_arena_gates_its_ladder_by_its_own_boss() {
    use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    let (first, second) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
    let mut app = App::new();
    seat_room_set(&mut app, &[]);
    app.add_systems(
        Update,
        (rebuild_feature_ecs_world_overlay, gate_gnu_ton_arena_ladder).chain(),
    );
    for (room, alive) in [(first, true), (second, false)] {
        app.world_mut().spawn((
            RoomInstanceRoot,
            room,
            definition(ARENA_ROOM_NAME),
            make_game_world_with_floor_gate(ARENA_ROOM_NAME, vec![ae::ClimbableRegion::ladder(ladder_aabb())]),
            FeatureEcsWorldOverlay::default(),
        ));
        // Two placements: the record of one is not the outcome of the other.
        let mut scratch = spawn_gnu_ton_runtime();
        scratch.config.id = format!("boss_gnu_ton_{alive}");
        let placement = scratch.config.id.clone();
        let boss = app
            .world_mut()
            .spawn((scratch.into_components(), InRoomInstance(room)))
            .id();
        if !alive {
            app.world_mut()
                .get_mut::<ambition_characters::actor::BodyHealth>(boss)
                .unwrap()
                .health
                .current = 0;
            record_defeat(&mut app, &placement);
        }
    }
    app.update();
    let mut overlays: Vec<_> = app
        .world_mut()
        .query::<(&LiveRoomInstance, &FeatureEcsWorldOverlay)>()
        .iter(app.world())
        .map(|(room, overlay)| {
            (
                *room,
                overlay.climbable_carves.len(),
                overlay.removed_block_names.iter().any(|name| name == FLOOR_GATE_BLOCK_NAME),
            )
        })
        .collect();
    overlays.sort();
    assert_eq!(
        overlays,
        vec![(first, 1, false), (second, 0, true)],
        "(room, ladder carves, floor gate removed): each arena did not follow its own boss"
    );
}

/// The choreography's rows against the sheets the game ships: the
/// choreography is in the module crate, and the catalogs are here.
mod pinned_rows {
    use ambition_content_modules::gnu_ton::choreography::*;
    use ambition_platformer2d_core::Vec2;

    fn stage() -> Stage {
        Stage {
            hall: Hall { floor: 1248.0, left: 16.0, right: 1776.0 },
            homes: [Vec2::new(600.0, 1000.0), Vec2::new(1200.0, 1000.0)],
            scholar: Vec2::new(850.0, 970.0),
            fist: Vec2::new(130.0, 100.0),
        }
    }

    fn latch(aim_x: f32) -> Latch {
        Latch { aim: Vec2::new(aim_x, 1225.0), lead: Some(Fist::Left), from: stage().homes, curve_phase: 0.0 }
    }

    fn cue(mv: Move, striking: bool, t: f32, dur: f32) -> Cue {
        Cue { mv, striking, t, dur, tel_dur: 1.0, beat_t: if striking { 1.0 + t } else { t }, clock: 0.0 }
    }

    /// Every row a pin can name is on the sheet that body draws with, at every
    /// quality tier. A pin naming a row the sheet lacks draws the body's slot
    /// row instead — silently — so a renamed or misspelled row would put the
    /// scholar back in his resting pose with nothing red. The sheet keys come
    /// from where the game resolves them (the boss sprite table, the character
    /// catalog), not a copy here.
    #[test]
    fn every_row_a_pin_names_is_on_the_sheet_that_draws_it() {
        use ambition_sprite_sheet::boss::boss_ron_target;
        use ambition_sprite_sheet::character::sheets::{available_sheet_keys, record_for_sheet_key};
        use std::collections::BTreeSet;

        let mut scholar = BTreeSet::new();
        let mut gnu = BTreeSet::new();
        let mut fist = BTreeSet::new();
        let s = stage();
        let l = latch(900.0);
        for mv in Move::ALL {
            for striking in [false, true] {
                let dur = 3.4;
                for step in 0..=68 {
                    let c = cue(mv, striking, dur * step as f32 / 68.0, dur);
                    scholar.extend(rows::scholar(Some(&c)).into_iter().flat_map(|p| p.0.iter().copied()));
                    gnu.extend(rows::gnu(Some(&c)).into_iter().flat_map(|p| p.0.iter().copied()));
                    for f in [Fist::Left, Fist::Right] {
                        fist.extend(rows::fist(&perform(&s, &l, f, &c), 0.0).0.iter().copied());
                    }
                }
            }
        }
        // The sample reaches every pose each body is choreographed into.
        for (body, named, expected) in [
            ("scholar", &scholar, &["point", "conduct", "invoke", "brace", "tumble", "dazed", "climb"][..]),
            ("gnu", &gnu, &["buck", "stomp"][..]),
            ("fist", &fist, &["rest", "fall", "stuck"][..]),
        ] {
            assert_eq!(named, &expected.iter().copied().collect::<BTreeSet<_>>(), "{body}");
        }

        let catalog = crate::character_catalog::shipped_catalog();
        let sheet_of_character = |id: &str| {
            boss_ron_target(&catalog.get(id).expect(id).spritesheet()).expect(id).to_string()
        };
        let rider = crate::bosses::shipped_boss_catalog()
            .sprite_filenames()
            .find(|(sheet, _)| *sheet == crate::bosses::gnu_ton::conductor::GNU_TON_ID)
            .map(|(_, file)| file.to_owned())
            .expect("the rider's sheet has a file");
        let bodies = [
            (boss_ron_target(&rider).expect("rider sheet").to_string(), &scholar),
            (sheet_of_character("npc_giant_gnu"), &gnu),
            (sheet_of_character("npc_giant_gnu_hands"), &fist),
        ];
        for (key, named) in bodies {
            let tiers: Vec<&str> = available_sheet_keys()
                .into_iter()
                .filter(|k| *k == key || k.strip_prefix(key.as_str()).is_some_and(|rest| rest.starts_with('.')))
                .collect();
            assert!(tiers.len() >= 2, "`{key}` should ship with its quality tiers: {tiers:?}");
            for tier in tiers {
                let record = record_for_sheet_key(tier).expect("a listed key has a record");
                for row in named.iter() {
                    assert!(
                        record.first_bound_row([*row]).is_some(),
                        "`{tier}` has no `{row}` row; the pin would silently draw the slot row"
                    );
                }
            }
        }
    }
}

/// ⭐ THE LOOK LINES ARE WHERE THE ARMS ARE CUT. The giant's sheet publishes
/// the cut of each arm in its line (anchors `seam:<slot>:a` / `:b`, measured
/// on the drawn arm by the renderer), and the lone fist's sheet the cut of
/// its fore arm (`seam:self`). The presentation lays the conductor's lines
/// on those cuts; where the conductor puts them (`ch::arms::SHOULDER_LINES`,
/// `WRIST_LINE`) is where they are while no cut is published, so the two
/// must agree at rest or a line slides when the cut comes. Each cut is as
/// wide as the arm (about 55 world units: the arm is 57 wide in the drawing
/// with its ink).
#[test]
fn the_conductors_look_lines_are_on_the_published_cuts_at_rest() {
    use ambition_content_modules::gnu_ton::choreography as ch;
    use ambition_sprite_sheet::character::sheets::record_for_sheet_key;
    let world_per_pixel = ch::blows::DESIGN * 2.0;
    // Each cut of the row `rest`'s first frame: its name, its middle from the
    // middle of the body (world units), its length, and its way.
    let cuts = |key: &str| {
        let record = record_for_sheet_key(key).unwrap_or_else(|| panic!("the sheet `{key}` is published"));
        let body = record.body_metrics.as_ref().and_then(|m| m.body_pixel_bbox).expect("an authored body");
        let (cx, cy) = body.center();
        let row = record.rows.iter().find(|row| row.animation == "rest").expect("a row `rest`");
        let anchors = &row.rects[0].anchors;
        let world = |p: &ambition_sprite_sheet::NormPoint| {
            ae::Vec2::new(p.x * record.frame_width as f32 - cx, p.y * record.frame_height as f32 - cy) * world_per_pixel
        };
        anchors
            .iter()
            .filter_map(|(key, a)| {
                let name = key.strip_prefix("seam:")?.strip_suffix(":a")?;
                let b = anchors.get(&format!("seam:{name}:b"))?;
                let (a, b) = (world(a), world(b));
                Some((name.to_owned(), (a + b) * 0.5, a.distance(b), (b - a).normalize()))
            })
            .collect::<Vec<_>>()
    };
    let giant = cuts("giant_gnu");
    for (shoulder, limb) in ["hand_left", "hand_right"].into_iter().enumerate() {
        let (_, middle, length, along) =
            giant.iter().find(|cut| cut.0 == limb).unwrap_or_else(|| panic!("no cut of {limb}: {giant:?}"));
        let line = ch::arms::SHOULDER_LINES[shoulder];
        assert!(middle.distance(line) <= 3.0, "{limb}: cut at {middle:?}, line at {line:?}");
        assert!((length - 55.0).abs() <= 6.0, "{limb}: the cut is {length} wide");
        assert!(along.x.abs() < 0.05, "{limb}: the arm is cut on an upright line: {along:?}");
    }
    // The right fist, as drawn: its wrist to the left of its middle.
    let fist = cuts("giant_gnu_fist");
    let (_, middle, ..) = fist.iter().find(|cut| cut.0 == "self").expect("the fist publishes its own cut");
    assert!(
        middle.distance(ae::Vec2::new(-ch::arms::WRIST_LINE, 0.0)) <= 4.0,
        "the fist's cut is at {middle:?}, its wrist line at {}",
        -ch::arms::WRIST_LINE
    );
}
