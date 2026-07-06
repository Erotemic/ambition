//! GNU-ton arena environment gating.
//!
//! Two arena hooks live here, both driven by the same "is the GNU-ton
//! boss alive?" check so they stay in lockstep:
//!
//! 1. **Ladder reveal.** The arena's retreat ladder is authored as a
//!    Climbable IntGrid column in `gnu_ton_arena_area.yaml`, so by
//!    default it's painted into `world.climbable_regions` the moment
//!    the room loads — which would let the player skip the fight by
//!    climbing right back out. This module hides the ladder while the
//!    boss is alive and re-adds it the frame the boss is defeated.
//!
//! 2. **Floor-gate above the ladder.** The entry ledge has a 48-px
//!    gap punched out above the ladder column; a named Solid
//!    (`ladder_floor_gate`) authored in LDtk fills that gap while the
//!    boss is alive and is removed from `world.blocks` on defeat, so
//!    the player can climb up the ladder and walk back to the exit
//!    door. The floor-gate uses the *opposite* polarity from the
//!    ladder (present-when-alive instead of absent-when-alive); both
//!    are intentionally driven from the same boss-alive check so a
//!    single gating system maintains both invariants.
//!
//! Gating is current-state-driven (any ECS boss with `is_gnu_ton() &&
//! !alive`) rather than persisted-encounter-driven. Dying mid-fight
//! resets the boss to alive, which correctly re-hides the ladder for
//! the next attempt. Cross-session persistence inherits whatever
//! state the boss runtime restores on respawn — no extra hookup here.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_engine_core::RoomGeometry;
use ambition_gameplay_core::features::{BossClusterRef, FeatureEcsWorldOverlay};

/// LDtk level identifier of the arena room whose ladder this system
/// gates. Held as a constant so it's grep-able alongside the matching
/// yaml at `tools/ambition_ldtk_tools/specs/gnu_ton_arena_area.yaml`.
const ARENA_ROOM_NAME: &str = "gnu_ton_arena";

/// Authored name of the named Solid block that fills the gap above
/// the ladder while the fight is live. Defined in the LDtk file as a
/// `Solid` entity with `fields.name = "ladder_floor_gate"`. Must
/// match `specs/gnu_ton/add_ladder_floor_gate.yaml`.
const FLOOR_GATE_BLOCK_NAME: &str = "ladder_floor_gate";

/// GNU-ton recognizer (id or authored display name). Lives content-side:
/// the generic cluster views no longer carry named-boss predicates.
///
/// The arena (ADR 0020 / G4) now spawns the SPLIT pair: the encounter boss is
/// the `gnu_ton_rider` scholar riding the `giant_gnu` mount, so the gate must
/// recognize the rider id. The fused `gnu_ton` id is still matched so the gate
/// keeps working against the (still-authored) fused profile and its regression
/// tests until the fused teardown lands.
fn boss_is_gnu_ton(boss: &ambition_gameplay_core::features::BossRef<'_>) -> bool {
    boss.config.behavior.id == "gnu_ton"
        || boss.config.behavior.id == "gnu_ton_rider"
        || boss.config.name.eq_ignore_ascii_case("gnu_ton")
        || boss.config.name.eq_ignore_ascii_case("gnu-ton")
}

/// Stateless arena-gate contributor. The authored base ALWAYS carries the
/// retreat ladders + the `ladder_floor_gate` Solid (immutable mid-room); this
/// system derives, each frame, which of them the collision *view* should hide,
/// from the current boss state — instead of mutating `RoomGeometry`:
///
/// - **Boss alive (or not yet spawned):** carve out the arena's Ladder regions
///   (so the player can't climb back out and skip the fight) and leave the
///   floor-gate solid.
/// - **Boss defeated:** stop carving the ladders (they reappear from the base)
///   and add the floor-gate block to `removed_block_names` so the gap opens and
///   the player can climb up to the exit.
///
/// Runs in `WorldPrep` after `rebuild_feature_ecs_world_overlay` clears the
/// overlay (same clean-slate-per-frame contract as the encounter / intro lock
/// walls). No per-visit `Local` state: a fresh room load swaps the immutable
/// base and the derive recomputes from scratch; dying mid-fight (boss back to
/// alive) re-hides the ladders automatically.
pub fn gate_gnu_ton_arena_ladder(
    world: Res<RoomGeometry>,
    bosses: Query<(BossClusterRef, &ambition_characters::actor::BodyHealth)>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    if world.0.name != ARENA_ROOM_NAME {
        return;
    }
    // Defeat = an ECS gnu_ton boss observed `alive = false`. An empty query
    // (boss not yet spawned) is NOT defeat — the ladder stays hidden.
    let boss_defeated = bosses.iter().any(|(feature, health)| {
        let boss = feature.as_boss_ref();
        boss_is_gnu_ton(&boss) && !health.alive()
    });

    if boss_defeated {
        // Open the gap above the ladder so the player can climb back to the exit.
        overlay
            .removed_block_names
            .push(FLOOR_GATE_BLOCK_NAME.to_string());
        // Ladders: contribute no carve → they reappear from the immutable base.
    } else {
        // Hide every authored Ladder region while the fight is live.
        for region in &world.0.climbable_regions {
            if region.kind == ae::ClimbableKind::Ladder {
                overlay.climbable_carves.push(region.aabb);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_gameplay_core::features::{
        rebuild_feature_ecs_world_overlay, world_with_sandbox_solids, BossBehaviorProfile,
        BossClusterScratch,
    };

    /// The composited collision view (immutable base + this frame's overlay).
    /// The gate is now a derived overlay contributor, so the arena assertions
    /// read the VIEW — what player/actor collision actually sees — not the base.
    fn arena_view(app: &App) -> ae::World {
        let base = &app.world().resource::<RoomGeometry>().0;
        let overlay = app.world().resource::<FeatureEcsWorldOverlay>();
        world_with_sandbox_solids(base, &[], overlay)
    }

    fn make_game_world(
        name: &str,
        ladders: Vec<ae::ClimbableRegion>,
    ) -> ambition_engine_core::RoomGeometry {
        let world = ae::World::new(
            name,
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
        .with_climbable_regions(ladders);
        ambition_engine_core::RoomGeometry(world)
    }

    fn make_game_world_with_floor_gate(
        name: &str,
        ladders: Vec<ae::ClimbableRegion>,
    ) -> ambition_engine_core::RoomGeometry {
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
        ambition_engine_core::RoomGeometry(world)
    }

    fn floor_gate_count(app: &App) -> usize {
        arena_view(app)
            .blocks
            .iter()
            .filter(|b| b.name == FLOOR_GATE_BLOCK_NAME)
            .count()
    }

    fn spawn_gnu_ton_runtime() -> BossClusterScratch {
        let behavior = BossBehaviorProfile::from_data("gnu_ton");
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut scratch = BossClusterScratch::new(
            "boss_gnu_ton",
            "GNU-ton",
            aabb,
            ambition_entity_catalog::placements::BossBrain::Dormant,
        );
        scratch.config.behavior = behavior;
        scratch
    }

    /// The ADR 0020 / G4 encounter boss: the `gnu_ton_rider` scholar (the boss
    /// that actually spawns in the reauthored arena — it rides the `giant_gnu`
    /// mount). The gate must treat this the same as the fused `gnu_ton`.
    fn spawn_gnu_ton_rider_runtime() -> BossClusterScratch {
        let behavior = BossBehaviorProfile::from_data("gnu_ton_rider");
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(54.0, 96.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut scratch = BossClusterScratch::new(
            "boss_gnu_ton_rider",
            "GNU-ton",
            aabb,
            ambition_entity_catalog::placements::BossBrain::Dormant,
        );
        scratch.config.behavior = behavior;
        scratch
    }

    /// Regression guard for the GNU-ton head-hurtbox alignment concern
    /// (TODO #30): after the sprite-metrics derivation, the
    /// `damageable_volumes` head hurtbox must actually overlap the boss
    /// body envelope (`boss.aabb()`, which the debug overlay documents as
    /// lining up with the visible body) rather than floating off to the
    /// side / below it. This pins that the hurtbox tracks the body even
    /// though it is computed in frame space (frame-center → `boss.pos`)
    /// while the body envelope carries `combat_offset`.
    #[test]
    fn gnu_ton_head_hurtbox_overlaps_the_body_envelope() {
        use ambition_engine_core::AabbExt;

        crate::bosses::install_boss_roster();
        let mut app = App::new();
        app.add_plugins(ambition_gameplay_core::character_sprites::SheetRegistryPlugin);
        let entity = app
            .world_mut()
            .spawn((
                ambition_gameplay_core::features::FeatureSimEntity,
                spawn_gnu_ton_runtime().into_components(),
                ambition_characters::brain::BossAttackState::default(),
            ))
            .id();
        app.add_systems(
            Update,
            ambition_gameplay_core::features::derive_boss_sprite_metrics,
        );
        // First update runs Startup (loads the baked sprite registry)
        // then Update (derives the boss's sprite metrics from it).
        app.update();

        let status = app
            .world()
            .get::<ambition_gameplay_core::features::BossEncounter>(entity)
            .unwrap();
        assert!(
            status.sprite_metrics.is_some(),
            "gnu_ton sprite metrics should derive from the baked sheet registry"
        );
        let attack = app
            .world()
            .get::<ambition_characters::brain::BossAttackState>(entity)
            .unwrap();
        let kin = app
            .world()
            .get::<ambition_gameplay_core::features::BodyKinematics>(entity)
            .unwrap();
        let config = app
            .world()
            .get::<ambition_gameplay_core::features::BossConfig>(entity)
            .unwrap();
        let status = app
            .world()
            .get::<ambition_gameplay_core::features::BossEncounter>(entity)
            .unwrap();
        let boss_ref = ambition_gameplay_core::features::BossRef {
            kin,
            config,
            status,
        };
        let ctx = ambition_gameplay_core::features::BossVolumeContext::from_ref(boss_ref, attack);
        let hurtboxes = ambition_gameplay_core::features::damageable_volumes(&ctx);
        assert!(
            !hurtboxes.is_empty(),
            "gnu_ton should expose at least one damageable hurtbox at rest"
        );
        let kin = app
            .world()
            .get::<ambition_gameplay_core::features::BodyKinematics>(entity)
            .unwrap();
        let config = app
            .world()
            .get::<ambition_gameplay_core::features::BossConfig>(entity)
            .unwrap();
        let status = app
            .world()
            .get::<ambition_gameplay_core::features::BossEncounter>(entity)
            .unwrap();
        let body = ambition_gameplay_core::features::BossRef {
            kin,
            config,
            status,
        }
        .aabb();
        for hb in &hurtboxes {
            assert!(
                body.strict_intersects(*hb),
                "hurtbox {hb:?} does not overlap the boss body envelope {body:?} \
                 — the head hurtbox has drifted off the visible body (TODO #30)"
            );
        }
    }

    fn make_app(world: ambition_engine_core::RoomGeometry) -> App {
        crate::bosses::install_boss_roster();
        let mut app = App::new();
        app.insert_resource(world);
        app.init_resource::<FeatureEcsWorldOverlay>();
        // Mirror the production WorldPrep order: the overlay rebuild clears the
        // per-frame contributions, then the gate re-derives them this frame.
        app.add_systems(
            Update,
            (rebuild_feature_ecs_world_overlay, gate_gnu_ton_arena_ladder).chain(),
        );
        app
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
        use ambition_gameplay_core::ldtk_world::LdtkProject;

        // `to_room_set` reads the world manifest + resolves spawn display names
        // through the character roster; install both content seams before
        // composing (first install wins, so this is safe to call in any test).
        crate::worlds::install();
        crate::character_catalog::install();
        let mut project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
        // Compose ONLY the arena area. The full sandbox composes portal rooms
        // whose entities need the `portal_ldtk` feature (off in this test build);
        // the arena itself has no portal entities, so scoping to it keeps the
        // conversion-level assertion (`convert_boss_spawn` → `mount_links`) light.
        project
            .levels
            .retain(|level| level.identifier == ARENA_ROOM_NAME);
        let room_set = project.to_room_set().expect("gnu_ton_arena composes");
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
            matches!(&mount.payload, CharacterBrain::Custom(id) if id == "giant_gnu"),
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
    fn ladder_is_hidden_when_rider_boss_is_alive() {
        // G4: the reauthored arena spawns the `gnu_ton_rider` (the scholar on
        // the giant), not the fused `gnu_ton`. The gate must still hide the
        // retreat ladder while THAT boss is alive, or the split boss would let
        // the player climb out and skip the fight.
        let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
        let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
        app.world_mut()
            .spawn(spawn_gnu_ton_rider_runtime().into_components());
        app.update();
        assert_eq!(
            climbable_regions_len(&app),
            0,
            "ladder must be hidden while the gnu_ton_rider encounter boss is alive"
        );
    }

    #[test]
    fn ladder_appears_when_rider_boss_dies() {
        // The rider-boss defeat must reveal the retreat ladder, exactly like the
        // fused boss does — the gate keys on the encounter boss being defeated.
        let ladder = ae::ClimbableRegion::ladder(ladder_aabb());
        let mut app = make_app(make_game_world(ARENA_ROOM_NAME, vec![ladder]));
        let boss_entity = app
            .world_mut()
            .spawn(spawn_gnu_ton_rider_runtime().into_components())
            .id();
        app.update();
        assert_eq!(climbable_regions_len(&app), 0);
        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyHealth>(boss_entity)
            .unwrap()
            .health
            .current = 0;
        app.update();
        let regions = &app
            .world()
            .resource::<ambition_engine_core::RoomGeometry>()
            .0
            .climbable_regions;
        assert_eq!(
            regions.len(),
            1,
            "ladder should be back after the rider boss is defeated"
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
        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyHealth>(boss_entity)
            .unwrap()
            .health
            .current = 0;
        app.update();
        let regions = &app
            .world()
            .resource::<ambition_engine_core::RoomGeometry>()
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
        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyHealth>(boss_entity)
            .unwrap()
            .health
            .current = 0;
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
        let mut app = make_app(make_game_world("some_other_room", vec![ladder]));
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
        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyHealth>(boss_entity)
            .unwrap()
            .health
            .current = 0;
        app.update();
        assert_eq!(climbable_regions_len(&app), 1);

        // Leave the arena (room change → world wholesale replaced,
        // boss entity despawned in the real flow but irrelevant
        // here since the room-name check fires first).
        app.world_mut()
            .resource_mut::<ambition_engine_core::RoomGeometry>()
            .0 = ae::World::new(
            "some_other_room",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        app.update();

        // Re-enter arena with fresh ladder + fresh (alive) boss.
        app.world_mut()
            .resource_mut::<ambition_engine_core::RoomGeometry>()
            .0 = ae::World::new(
            ARENA_ROOM_NAME,
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
        .with_climbable_regions(vec![ladder]);
        // Replace the existing boss component (still alive=false) with
        // a fresh-spawn boss so the gate sees a live boss again.
        app.world_mut().despawn(boss_entity);
        app.world_mut()
            .spawn(spawn_gnu_ton_runtime().into_components());
        app.update();

        assert_eq!(
            climbable_regions_len(&app),
            0,
            "ladder must be hidden again on re-entry with a live boss"
        );
    }
}
