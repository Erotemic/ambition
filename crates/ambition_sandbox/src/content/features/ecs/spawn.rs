//! ECS-feature spawn facade.
//!
//! Room-level orchestration and public dynamic-mob entry points stay here, while
//! the concrete family-specific spawn helpers live in smaller sibling modules.
//! This keeps the active ECS path readable without changing the entity shapes
//! or scheduling surfaces that callers use.

use crate::content::features::util::room_spec_paths;
use bevy::prelude::{Commands, Entity, Query};

pub(crate) use super::spawn_actors::spawn_runtime_minion;

/// Spawn ECS-native feature entities for every authored static
/// feature in a room. One loop per family.
pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let paths = room_spec_paths(room);
    for hazard in &room.hazards {
        super::spawn_static::spawn_hazard(commands, hazard, &paths);
    }
    for boss in &room.boss_spawns {
        super::spawn_actors::spawn_boss(commands, boss);
    }
    for pickup in &room.pickups {
        super::spawn_static::spawn_pickup(commands, pickup);
    }
    for chest in &room.chests {
        super::spawn_static::spawn_chest(commands, chest);
    }
    for breakable in &room.breakables {
        super::spawn_static::spawn_breakable(commands, breakable);
    }
    for enemy in &room.enemy_spawns {
        super::spawn_actors::spawn_enemy(commands, enemy, &paths);
    }
    for interactable in &room.interactables {
        super::spawn_actors::spawn_interactable(commands, interactable, &paths);
    }
    // DebugLabel and DestinationLabel are presentation-only and don't
    // spawn ECS feature entities today. The presentation layer reads
    // them off `RoomSpec` directly.
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: crate::actor::EnemyBrain,
    pos: crate::engine_core::Vec2,
    size: crate::engine_core::Vec2,
) {
    super::spawn_actors::spawn_encounter_mob(commands, encounter_id, id, brain, pos, size);
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(
        Entity,
        &super::EncounterMob,
        &super::FeatureId,
        &super::ActorCombatState,
    )>,
    encounter_id: &str,
) {
    super::spawn_actors::despawn_encounter_mobs(commands, mobs, encounter_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::brain_builders::{enemy_default_action_set, enemy_default_brain};
    use super::super::spawn_actors::spawn_boss;
    use super::super::spawn_mounts::spawn_composite_mount_rider;
    use crate::brain::{
        ActionSet, ActorControl, Brain, MeleeActionSpec, MoveStyleSpec, StateMachineCfg,
    };
    use crate::content::features::{EnemyArchetype, EnemyRuntime, MountSlot};
    use crate::engine_core as ae;
    use bevy::prelude::{App, Commands, Update};

    fn make_enemy(archetype: EnemyArchetype) -> EnemyRuntime {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0));
        let mut enemy = EnemyRuntime::new(
            "test".to_string(),
            "test".to_string(),
            aabb,
            crate::actor::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.archetype = archetype;
        enemy
    }

    /// Regression net: spawning an encounter mob attaches a
    /// per-archetype Brain. `medium_striker` migrated from
    /// `MeleeBrute` to `Smash` in `enemy_archetypes.ron`; the test
    /// follows that and pins the Smash variant instead.
    #[test]
    fn encounter_mob_brain_is_per_archetype_melee_brute() {
        use crate::brain::{Brain, StateMachineCfg};
        let mut app = App::new();
        app.add_systems(Update, |mut commands: Commands| {
            spawn_encounter_mob(
                &mut commands,
                "test_encounter",
                "test_mob".to_string(),
                crate::actor::EnemyBrain::Custom("medium_striker".into()),
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 30.0),
            );
        });
        app.update();
        let mut q = app.world_mut().query::<&Brain>();
        let brain = q.iter(app.world()).next().expect("encounter mob exists");
        // medium_striker is a hostile archetype with Smash brain.
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::Smash { .. })
        ));
    }

    /// Regression net: spawn_boss attaches Brain (BossPattern) +
    /// ActionSet + ActorControl alongside BossFeature. Pins the
    /// parallel-shape invariant.
    #[test]
    fn boss_spawn_attaches_brain_components() {
        use crate::brain::{ActionSet, ActorControl, Brain, StateMachineCfg};
        let mut app = App::new();
        app.add_systems(Update, |mut commands: Commands| {
            let authored = crate::rooms::Authored {
                id: "test_boss".to_string(),
                name: "Test Warden".to_string(),
                aabb: ae::Aabb::new(ae::Vec2::new(200.0, 100.0), ae::Vec2::new(40.0, 50.0)),
                payload: crate::actor::BossBrain::Dormant,
            };
            spawn_boss(&mut commands, &authored);
        });
        app.update();
        let mut q = app
            .world_mut()
            .query::<(&Brain, &ActionSet, &ActorControl)>();
        let count = q.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "boss should carry Brain + ActionSet + ActorControl"
        );
        let (brain, action_set, _) = q.iter(app.world()).next().expect("boss exists");
        // Brain is BossPattern with the real encounter id derived
        // from the boss name.
        match brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) => {
                assert_eq!(cfg.encounter_id, "test_warden");
            }
            other => panic!("expected BossPattern brain, got {:?}", other),
        }
        // ActionSet carries an offensive baseline: Bolt ranged + no
        // special slot. The special slot is intentionally `None`
        // because boss specials are now emitted directly by
        // `tick_boss_brains_system` via `boss_special_for_profile`
        // (see `content/features/bosses.rs`) — the generic resolver
        // would otherwise fire a duplicate Special message with a
        // stale or wrong spec. The spawn default must be
        // hostile-capable for ranged so a brain-driven boss can act.
        assert!(
            matches!(
                action_set.ranged,
                Some(crate::brain::RangedActionSpec::Bolt { .. })
            ),
            "boss ActionSet should default to Bolt ranged",
        );
        assert!(
            action_set.special.is_none(),
            "boss ActionSet.special should be None — multi-special bosses \
             route through tick_boss_brains_system's direct-write path; got {:?}",
            action_set.special,
        );
    }

    /// Regression net: every encounter-spawned hostile actor lands
    /// with the universal-brain components attached. Pins the
    /// parallel-shape invariant so a future spawn-site refactor
    /// can't silently lose the brain.
    #[test]
    fn encounter_mob_spawns_with_brain_components() {
        let mut app = App::new();
        app.add_systems(Update, |mut commands: Commands| {
            spawn_encounter_mob(
                &mut commands,
                "test_encounter",
                "test_mob".to_string(),
                crate::actor::EnemyBrain::Custom("medium_striker".into()),
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 30.0),
            );
        });
        app.update();
        let mut q = app
            .world_mut()
            .query::<(&Brain, &ActionSet, &ActorControl)>();
        let count = q.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "encounter mob should carry Brain + ActionSet + ActorControl"
        );
    }

    /// enemy_default_brain picks a per-archetype template — pins
    /// the mapping so a future refactor that re-keys archetypes
    /// can't silently lose the Wanderer/StandStill assignments
    /// PuppySlug and Sandbag rely on.
    #[test]
    fn enemy_default_brain_picks_per_archetype_template() {
        let slug = make_enemy(EnemyArchetype::PuppySlug);
        assert!(matches!(
            enemy_default_brain(&slug),
            Brain::StateMachine(StateMachineCfg::Wanderer { .. })
        ));

        let sandbag = make_enemy(EnemyArchetype::InfiniteSandbag);
        assert!(matches!(
            enemy_default_brain(&sandbag),
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));

        let shark = make_enemy(EnemyArchetype::BurningFlyingShark);
        assert!(matches!(
            enemy_default_brain(&shark),
            Brain::StateMachine(StateMachineCfg::Shark { .. })
        ));

        // `MediumStriker` was migrated to the Smash brain template in
        // `enemy_archetypes.ron` — assert against the live data path
        // rather than reverting to MeleeBrute. The chase_speed pin
        // moves over to the `SmashCfg` row.
        let striker = make_enemy(EnemyArchetype::MediumStriker);
        match enemy_default_brain(&striker) {
            Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) => {
                assert!(cfg.aggro_radius > 0.0);
                assert!(
                    (cfg.chase_speed - EnemyArchetype::MediumStriker.chase_speed()).abs() < 0.01
                );
            }
            other => panic!("expected Smash for MediumStriker, got {:?}", other),
        }
    }

    /// Coverage lint: every EnemyArchetype in COMBAT_ALL maps to a
    /// usable Brain (no panic, non-empty per design). Catches a
    /// future archetype addition that forgets to update
    /// enemy_default_brain.
    #[test]
    fn enemy_default_brain_covers_every_combat_archetype() {
        for archetype in EnemyArchetype::COMBAT_ALL {
            let enemy = make_enemy(archetype);
            let brain = enemy_default_brain(&enemy);
            // Aggressiveness should match archetype.attacks_player.
            // (Wanderer / StandStill / peaceful Patrol all return
            // !is_hostile; everyone else returns is_hostile.)
            assert_eq!(
                brain.is_hostile(),
                archetype.attacks_player(),
                "{:?} brain.is_hostile mismatch with archetype.attacks_player",
                archetype,
            );
        }
    }

    /// Regression net: the riderless shark gets the new Shark brain
    /// while the mounted shark composite keeps the orbiting
    /// Skirmisher-style mount brain on purpose.
    #[test]
    fn shark_composite_mount_brain_stays_skirmisher() {
        use crate::brain::{Brain, StateMachineCfg};
        let mut app = App::new();
        app.add_systems(Update, |mut commands: Commands| {
            let authored = crate::rooms::Authored {
                id: "test_shark_on_shark".to_string(),
                name: "Test Shark on Shark".to_string(),
                aabb: ae::Aabb::new(ae::Vec2::new(200.0, 120.0), ae::Vec2::new(40.0, 32.0)),
                payload: crate::actor::EnemyBrain::Custom("pirate_on_shark".into()),
            };
            spawn_composite_mount_rider(
                &mut commands,
                &authored,
                &[],
                EnemyArchetype::PirateOnShark,
            );
        });
        app.update();
        let mut q = app.world_mut().query::<(&Brain, &MountSlot)>();
        let (brain, _) = q
            .iter(app.world())
            .next()
            .expect("composite mount should exist");
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::Skirmisher { .. })
        ));
    }

    /// Coverage lint: every EnemyArchetype gets a non-None
    /// ActionSet that respects its peaceful/hostile flag — hostile
    /// archetypes have a melee or ranged spec, peaceful ones don't.
    /// `attacks_player()` returns false only for `PuppySlug` and
    /// `PirateHeavy`; every other archetype (including sandbags,
    /// which have a `PunchWeak` counter-attack) is hostile by this
    /// gate.
    #[test]
    fn enemy_default_action_set_covers_every_combat_archetype() {
        for archetype in EnemyArchetype::COMBAT_ALL {
            let enemy = make_enemy(archetype);
            let set = enemy_default_action_set(&enemy);
            if archetype.attacks_player() {
                assert!(
                    set.melee.is_some() || set.ranged.is_some(),
                    "{:?} attacks_player but ActionSet has no melee or ranged",
                    archetype,
                );
            } else {
                // Only PuppySlug + PirateHeavy reach this branch —
                // both peaceful, both expected to have no melee.
                assert!(
                    set.melee.is_none(),
                    "{:?} is peaceful but has melee",
                    archetype,
                );
            }
        }
    }

    /// enemy_default_action_set picks a per-archetype concrete
    /// attack spec — the EFFECTS consumers read these to spawn
    /// distinct hitboxes / projectiles per archetype.
    #[test]
    fn enemy_default_action_set_picks_per_archetype_specs() {
        let slug = make_enemy(EnemyArchetype::PuppySlug);
        let set = enemy_default_action_set(&slug);
        assert!(set.melee.is_none(), "peaceful PuppySlug has no melee");
        assert!(matches!(set.move_style, MoveStyleSpec::Slither));

        let brute = make_enemy(EnemyArchetype::LargeBrute);
        let set = enemy_default_action_set(&brute);
        assert!(matches!(set.melee, Some(MeleeActionSpec::Lunge(_))));
        assert!(matches!(set.move_style, MoveStyleSpec::WalkHeavy));

        let striker = make_enemy(EnemyArchetype::MediumStriker);
        let set = enemy_default_action_set(&striker);
        assert!(matches!(set.melee, Some(MeleeActionSpec::Swipe(_))));

        let pirate_shark = make_enemy(EnemyArchetype::PirateOnShark);
        let set = enemy_default_action_set(&pirate_shark);
        assert!(set.ranged.is_some(), "PirateOnShark has ranged");
        assert!(matches!(set.move_style, MoveStyleSpec::Float));
    }
}
