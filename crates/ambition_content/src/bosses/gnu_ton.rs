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

use ambition_gameplay_core::features::BossClusterRef;

/// LDtk level identifier of the arena room whose ladder this system
/// gates. Held as a constant so it's grep-able alongside the matching
/// yaml at `tools/ambition_ldtk_tools/specs/gnu_ton_arena_area.yaml`.
const ARENA_ROOM_NAME: &str = "gnu_ton_arena";

/// Authored name of the named Solid block that fills the gap above
/// the ladder while the fight is live. Defined in the LDtk file as a
/// `Solid` entity with `fields.name = "ladder_floor_gate"`. Must
/// match `specs/gnu_ton/add_ladder_floor_gate.yaml`.
const FLOOR_GATE_BLOCK_NAME: &str = "ladder_floor_gate";

/// Per-visit gating state. Lives in a `Local` on the gating system so
/// the resource graph stays clean (no global resource for a single
/// arena's environment hook).
#[derive(Default)]
pub struct GnuTonLadderGate {
    /// The Ladder-kind ClimbableRegions we pulled out of the world on
    /// arena entry. `None` means we haven't observed the arena yet
    /// this visit; `Some(_)` (even if empty) means we have, and won't
    /// re-scan. Reset to `None` on leaving the arena so a fresh entry
    /// re-runs the stash pass against the newly loaded world.
    stashed: Option<Vec<ae::ClimbableRegion>>,
    /// True once we've re-added the stashed ladders on boss death
    /// this visit. Stops us from re-adding every frame after defeat.
    revealed: bool,
    /// True once the named `ladder_floor_gate` Solid has been removed
    /// from `world.blocks` on defeat this visit. Stops us from
    /// re-scanning every frame after the gate is open.
    floor_gate_opened: bool,
}

/// Stash the arena's Climbable Ladder regions while the boss is alive,
/// then add them back to `world.climbable_regions` the frame the boss
/// dies. Cheap in non-arena rooms (single name comparison) and in
/// arena rooms after stash/reveal completes.
/// GNU-ton recognizer (id or authored display name). Lives content-side:
/// the generic cluster views no longer carry named-boss predicates.
fn boss_is_gnu_ton(boss: &ambition_gameplay_core::features::BossRef<'_>) -> bool {
    boss.config.behavior.id == "gnu_ton"
        || boss.config.name.eq_ignore_ascii_case("gnu_ton")
        || boss.config.name.eq_ignore_ascii_case("gnu-ton")
}

pub fn gate_gnu_ton_arena_ladder(
    mut world: ResMut<ambition_gameplay_core::RoomGeometry>,
    bosses: Query<BossClusterRef>,
    mut state: Local<GnuTonLadderGate>,
) {
    if world.0.name != ARENA_ROOM_NAME {
        // Leaving the arena (or never entered). Drop visit-scoped
        // state so re-entry re-stashes against the freshly loaded
        // world — `world.0 = spec.world.clone()` on room change
        // restores the ladder cells that we removed last visit, and
        // re-emits the `ladder_floor_gate` Solid block.
        state.stashed = None;
        state.revealed = false;
        state.floor_gate_opened = false;
        return;
    }

    // First-frame-in-arena: pull all Ladder-kind regions out of the
    // world and stash them. Empty stash is allowed (e.g. a future
    // arena revision with no authored ladder); we still mark
    // `stashed = Some(...)` so we don't keep re-scanning.
    if state.stashed.is_none() {
        let ladders: Vec<ae::ClimbableRegion> = world
            .0
            .climbable_regions
            .iter()
            .filter(|r| r.kind == ae::ClimbableKind::Ladder)
            .cloned()
            .collect();
        if !ladders.is_empty() {
            world
                .0
                .climbable_regions
                .retain(|r| r.kind != ae::ClimbableKind::Ladder);
        }
        state.stashed = Some(ladders);
        state.revealed = false;
    }

    if state.revealed {
        return;
    }

    // Reveal condition: any ECS boss is a defeated gnu_ton. Empty
    // query (boss not yet spawned, or already despawned) does NOT
    // count as defeat — we only reveal on observed `alive = false`.
    let boss_defeated = bosses.iter().any(|feature| {
        let boss = feature.as_boss_ref();
        boss_is_gnu_ton(&boss) && !boss.status.alive
    });

    if boss_defeated {
        if let Some(stashed) = state.stashed.as_ref() {
            for region in stashed {
                world.0.climbable_regions.push(region.clone());
            }
        }
        state.revealed = true;
        if !state.floor_gate_opened {
            // Drop the named Solid that fills the gap above the ladder
            // so the player can climb back up. The block is restored
            // automatically on re-entry by the LDtk room-load path.
            let before = world.0.blocks.len();
            world.0.blocks.retain(|b| b.name != FLOOR_GATE_BLOCK_NAME);
            if world.0.blocks.len() != before {
                state.floor_gate_opened = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_gameplay_core::features::{BossBehaviorProfile, BossClusterScratch};

    fn make_game_world(
        name: &str,
        ladders: Vec<ae::ClimbableRegion>,
    ) -> ambition_gameplay_core::RoomGeometry {
        let world = ae::World::new(
            name,
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
        .with_climbable_regions(ladders);
        ambition_gameplay_core::RoomGeometry(world)
    }

    fn make_game_world_with_floor_gate(
        name: &str,
        ladders: Vec<ae::ClimbableRegion>,
    ) -> ambition_gameplay_core::RoomGeometry {
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
        ambition_gameplay_core::RoomGeometry(world)
    }

    fn floor_gate_count(app: &App) -> usize {
        app.world()
            .resource::<ambition_gameplay_core::RoomGeometry>()
            .0
            .blocks
            .iter()
            .filter(|b| b.name == FLOOR_GATE_BLOCK_NAME)
            .count()
    }

    fn spawn_gnu_ton_runtime() -> BossClusterScratch {
        let behavior = BossBehaviorProfile::gnu_ton();
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut scratch = BossClusterScratch::new(
            "boss_gnu_ton",
            "GNU-ton",
            aabb,
            ambition_characters::actor::BossBrain::Dormant,
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
            .get::<ambition_gameplay_core::features::BossStatus>(entity)
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
            .get::<ambition_gameplay_core::features::BossStatus>(entity)
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
            .get::<ambition_gameplay_core::features::BossStatus>(entity)
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

    fn make_app(world: ambition_gameplay_core::RoomGeometry) -> App {
        crate::bosses::install_boss_roster();
        let mut app = App::new();
        app.insert_resource(world);
        app.add_systems(Update, gate_gnu_ton_arena_ladder);
        app
    }

    fn ladder_aabb() -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(136.0, 736.0), ae::Vec2::new(16.0, 512.0))
    }

    fn climbable_regions_len(app: &App) -> usize {
        app.world()
            .resource::<ambition_gameplay_core::RoomGeometry>()
            .0
            .climbable_regions
            .len()
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
            .get_mut::<ambition_gameplay_core::features::BossStatus>(boss_entity)
            .unwrap()
            .alive = false;
        app.update();
        let regions = &app
            .world()
            .resource::<ambition_gameplay_core::RoomGeometry>()
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
            .get_mut::<ambition_gameplay_core::features::BossStatus>(boss_entity)
            .unwrap()
            .alive = false;
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
            .get_mut::<ambition_gameplay_core::features::BossStatus>(boss_entity)
            .unwrap()
            .alive = false;
        app.update();
        assert_eq!(climbable_regions_len(&app), 1);

        // Leave the arena (room change → world wholesale replaced,
        // boss entity despawned in the real flow but irrelevant
        // here since the room-name check fires first).
        app.world_mut()
            .resource_mut::<ambition_gameplay_core::RoomGeometry>()
            .0 = ae::World::new(
            "some_other_room",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        app.update();

        // Re-enter arena with fresh ladder + fresh (alive) boss.
        app.world_mut()
            .resource_mut::<ambition_gameplay_core::RoomGeometry>()
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
