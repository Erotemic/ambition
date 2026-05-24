//! ECS-feature spawn paths.
//!
//! Both static room features (authored entities from `RoomSpec`) and
//! dynamic encounter mobs land here. The static path is per-family —
//! one loop per `RoomSpec.{pickups,chests,…}` — so adding a new
//! authored entity type is "add a new Vec on RoomSpec + a new loop
//! here" rather than "edit a match arm somewhere."

use super::*;
use crate::content::features::util::room_spec_paths;
use bevy::prelude::Name;

/// Spawn ECS-native feature entities for every authored static
/// feature in a room. One loop per family.
pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let paths = room_spec_paths(room);
    for hazard in &room.hazards {
        spawn_hazard(commands, hazard, &paths);
    }
    for boss in &room.boss_spawns {
        spawn_boss(commands, boss);
    }
    for pickup in &room.pickups {
        spawn_pickup(commands, pickup);
    }
    for chest in &room.chests {
        spawn_chest(commands, chest);
    }
    for breakable in &room.breakables {
        spawn_breakable(commands, breakable);
    }
    for enemy in &room.enemy_spawns {
        spawn_enemy(commands, enemy, &paths);
    }
    for interactable in &room.interactables {
        spawn_interactable(commands, interactable, &paths);
    }
    // DebugLabel and DestinationLabel are presentation-only and don't
    // spawn ECS feature entities today. The presentation layer reads
    // them off `RoomSpec` directly.
}

fn spawn_hazard(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ae::DamageVolume>,
    paths: &[(String, ae::KinematicPath)],
) {
    let hazard = HazardRuntime::new_with_paths(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    commands.spawn((
        Name::new(format!("Feature hazard: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        FeatureAabb::from_center_size(hazard.pos, hazard.size),
        HazardFeature::new(hazard),
    ));
}

fn spawn_boss(commands: &mut Commands, authored: &crate::rooms::Authored<ae::BossBrain>) {
    let boss = BossRuntime::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
    );
    let initial_phase = BossPhase::from_alive(boss.alive);
    // Parallel-shape Brain attachment for bosses, same pattern as
    // enemies. The BossPattern template threads through the real
    // encounter id (matches the registry lookup the boss-encounter
    // system already uses); the ActorControl frame is dormant for
    // bosses today — BossRuntime still drives behavior.
    let encounter_id = crate::boss_encounter::encounter_id_from_name(&boss.name);
    let brain = crate::brain::Brain::StateMachine(
        crate::brain::StateMachineCfg::BossPattern {
            cfg: crate::brain::BossPatternCfg {
                aggressiveness: 1.0,
                encounter_id,
            },
            state: crate::brain::BossPatternState::default(),
        },
    );
    commands.spawn((
        Name::new(format!("Feature boss: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        FeatureAabb::from_center_size(boss.pos, boss.render_size()),
        BossPatternTimer(boss.pattern_timer),
        initial_phase,
        super::ActorFaction::Boss,
        super::ActorTarget::default(),
        BossFeature::new(boss),
        brain,
        crate::brain::ActionSet::peaceful(),
        crate::brain::ActorControl::default(),
    ));
}

fn spawn_pickup(commands: &mut Commands, authored: &crate::rooms::Authored<ae::Pickup>) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    commands.spawn((
        Name::new(format!("Feature pickup: {}", authored.name)),
        PickupBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

fn spawn_chest(commands: &mut Commands, authored: &crate::rooms::Authored<ae::Chest>) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    commands.spawn((
        Name::new(format!("Feature chest: {}", authored.name)),
        ChestBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

fn spawn_breakable(commands: &mut Commands, authored: &crate::rooms::Authored<ae::Breakable>) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let breakable = &authored.payload;
    let mut entity = commands.spawn((
        Name::new(format!("Feature breakable: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        feature_aabb,
        BreakableFeature::new(breakable.clone()),
        StandTimer(0.0),
    ));
    if breakable.collision.blocks_movement() {
        entity.insert(SandboxSolidContributor);
    }
    if breakable.pogo_refresh
        || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
    {
        entity.insert(PogoTargetContributor);
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ae::EnemyBrain>,
    paths: &[(String, ae::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let enemy = EnemyRuntime::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    // Attach a parallel-shape Brain + ActionSet to the enemy entity
    // so the universal-brain seam is wired even though EnemyRuntime
    // still drives behavior. Today no system reads these on hostile
    // actors — daytime work flips consumers off the choreography +
    // EnemyRuntime AI loop and onto Brain.tick() + ActionSet
    // resolution.
    let brain = enemy_default_brain(&enemy);
    let action_set = enemy_default_action_set(&enemy);
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    commands.spawn((
        Name::new(format!("Feature actor enemy: {}", authored.name)),
        EnemyActorBundle {
            base: FeatureBaseBundle::new(&authored.id, &authored.name, feature_aabb),
            identity,
            disposition,
            faction: super::ActorFaction::Enemy,
            target: super::ActorTarget::default(),
            health,
            combat,
            intent,
            cooldowns,
        },
        actor,
        brain,
        action_set,
        crate::brain::ActorControl::default(),
    ));
}

/// Map an `EnemyRuntime` to a Brain template. Reads the archetype's
/// actual tunings (chase_speed / aggro_radius / attack_range) so
/// the brain's MeleeBrute cfg matches what the existing AI loop
/// uses. PuppySlug gets a Wanderer; sandbags get StandStill;
/// everyone else gets a MeleeBrute keyed to their archetype.
///
/// When daytime work flips the EFFECTS consumer to read the brain's
/// ActorControl frame, the brain output will match the archetype's
/// pre-flip behavior — no per-archetype tuning gap to retune.
/// Map an `EnemyRuntime` to a default ActionSet keyed off its
/// archetype. Sandbags + peaceful archetypes get
/// [`ActionSet::peaceful`]; striker / brute archetypes get a Swipe
/// (or Lunge for brutes); ranged archetypes get an Arrow / Pistol
/// / Bolt where appropriate. Today no system consumes the resulting
/// ActionRequests for hostile actors — daytime work flips combat
/// spawners onto the resolver stream.
fn enemy_default_action_set(enemy: &EnemyRuntime) -> crate::brain::ActionSet {
    use crate::brain::{
        ActionSet, BiteSpec, LungeSpec, MeleeActionSpec, MoveStyleSpec, PunchSpec,
        RangedActionSpec, SwipeSpec,
    };
    let archetype = enemy.archetype;
    if !archetype.attacks_player() {
        // PuppySlug / PirateHeavy in peaceful disposition.
        return ActionSet {
            move_style: match archetype {
                EnemyArchetype::PuppySlug => MoveStyleSpec::Slither,
                _ => MoveStyleSpec::Walk,
            },
            ..ActionSet::default()
        };
    }
    match archetype {
        EnemyArchetype::LargeBrute | EnemyArchetype::LargeColossus => ActionSet {
            melee: Some(MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT)),
            move_style: MoveStyleSpec::WalkHeavy,
            ..Default::default()
        },
        EnemyArchetype::InfiniteSandbag | EnemyArchetype::FiniteSandbag => ActionSet {
            melee: Some(MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT)),
            ..Default::default()
        },
        EnemyArchetype::BurningFlyingShark => ActionSet {
            melee: Some(MeleeActionSpec::Bite(BiteSpec {
                windup_s: 0.18,
                active_s: 0.10,
                recover_s: 0.30,
                damage: archetype.damage_amount(),
                reach_px: 42.0,
            })),
            move_style: MoveStyleSpec::Float,
            ..Default::default()
        },
        EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark => ActionSet {
            ranged: Some(RangedActionSpec::Bolt {
                speed: 500.0,
                damage: archetype.damage_amount(),
            }),
            move_style: MoveStyleSpec::Float,
            ..Default::default()
        },
        // Default melee striker — covers Combatant / SmallSkitter /
        // MediumStriker / AggressiveSeeker / SmallLurker /
        // PirateRaider.
        _ => ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        },
    }
}

fn enemy_default_brain(enemy: &EnemyRuntime) -> crate::brain::Brain {
    use crate::brain::{
        Brain, MeleeBruteCfg, MeleeBruteState, StateMachineCfg, WandererCfg, WandererState,
    };
    let archetype = enemy.archetype;
    match archetype {
        EnemyArchetype::InfiniteSandbag | EnemyArchetype::FiniteSandbag => {
            Brain::StateMachine(StateMachineCfg::StandStill)
        }
        EnemyArchetype::PuppySlug => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        }),
        _ => Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg {
                aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
                aggro_radius: archetype.aggro_radius(),
                attack_range: archetype.attack_range(),
                chase_speed: archetype.chase_speed(),
            },
            state: MeleeBruteState::default(),
        }),
    }
}

fn spawn_interactable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ae::Interactable>,
    paths: &[(String, ae::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let interactable = &authored.payload;
    if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
        let npc = NpcRuntime::new_with_paths(
            authored.id.clone(),
            authored.name.clone(),
            authored.aabb,
            interactable.clone(),
            paths,
        );
        // Build the brain from the authored NPC fields before
        // wrapping into the ActorRuntime variant. Patrol-radius > 0
        // or an authored motion path → Patrol brain; otherwise
        // StandStill. ActionSet stays peaceful by default.
        let brain = npc.build_brain();
        let actor = ActorRuntime::Peaceful(npc);
        let (identity, disposition, health, combat, intent, cooldowns) =
            actor_component_snapshot(&actor);
        commands.spawn((
            Name::new(format!("Feature actor npc: {}", authored.name)),
            EnemyActorBundle {
                base: FeatureBaseBundle::new(&authored.id, &authored.name, feature_aabb),
                identity,
                disposition,
                faction: super::ActorFaction::Npc,
                target: super::ActorTarget::default(),
                health,
                combat,
                intent,
                cooldowns,
            },
            actor,
            brain,
            crate::brain::ActionSet::peaceful(),
            crate::brain::ActorControl::default(),
        ));
    } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
        if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload) {
            commands.spawn((
                Name::new(format!("Feature switch: {}", authored.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(authored.id.clone()),
                FeatureName::new(authored.name.clone()),
                feature_aabb,
                SwitchFeature::new(activation),
                SwitchOn(false),
            ));
        }
    }
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: ae::EnemyBrain,
    pos: ae::Vec2,
    size: ae::Vec2,
) {
    let encounter_id = encounter_id.into();
    let archetype = EnemyArchetype::from_brain(&brain);
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy = EnemyRuntime::new(id.clone(), id.clone(), aabb, brain, &[]);
    enemy.archetype = archetype;
    enemy.health = ae::Health::new(archetype.max_health());
    // Encounter mobs should not auto-respawn like training sandbags.
    enemy.respawn_timer = 999_999.0;
    let brain = enemy_default_brain(&enemy);
    let action_set = enemy_default_action_set(&enemy);
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    let feature_aabb = FeatureAabb::from_center_size(pos, size);
    commands.spawn((
        Name::new(format!("Encounter mob: {id}")),
        EnemyActorBundle {
            base: FeatureBaseBundle::new(&id, &id, feature_aabb),
            identity,
            disposition,
            faction: super::ActorFaction::Enemy,
            target: super::ActorTarget::default(),
            health,
            combat,
            intent,
            cooldowns,
        },
        actor,
        EncounterMob::new(encounter_id),
        brain,
        action_set,
        crate::brain::ActorControl::default(),
    ));
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(Entity, &EncounterMob, &FeatureId, &ActorCombatState)>,
    encounter_id: &str,
) {
    for (entity, mob, _, _) in mobs.iter() {
        if mob.encounter_id == encounter_id {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::{
        ActionSet, ActorControl, Brain, MeleeActionSpec, MoveStyleSpec, StateMachineCfg,
    };
    use bevy::prelude::*;

    fn make_enemy(archetype: EnemyArchetype) -> EnemyRuntime {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 30.0));
        let mut enemy = EnemyRuntime::new(
            "test".to_string(),
            "test".to_string(),
            aabb,
            ae::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.archetype = archetype;
        enemy
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
                ae::EnemyBrain::Custom("medium_striker".into()),
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(20.0, 30.0),
            );
        });
        app.update();
        let mut q = app
            .world_mut()
            .query::<(&Brain, &ActionSet, &ActorControl)>();
        let count = q.iter(app.world()).count();
        assert_eq!(count, 1, "encounter mob should carry Brain + ActionSet + ActorControl");
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

        let striker = make_enemy(EnemyArchetype::MediumStriker);
        match enemy_default_brain(&striker) {
            Brain::StateMachine(StateMachineCfg::MeleeBrute { cfg, .. }) => {
                assert!(cfg.aggressiveness > 0.0);
                // Brain's chase_speed mirrors the archetype tuning.
                assert!((cfg.chase_speed - EnemyArchetype::MediumStriker.chase_speed()).abs() < 0.01);
            }
            other => panic!("expected MeleeBrute for MediumStriker, got {:?}", other),
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

    /// Coverage lint: every EnemyArchetype gets a non-None
    /// ActionSet that respects its peaceful/hostile flag — hostile
    /// archetypes have a melee or ranged spec, peaceful ones don't.
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
                // Peaceful archetypes (PuppySlug, PirateHeavy) have
                // no melee. PirateHeavy has no ranged either.
                if !matches!(
                    archetype,
                    EnemyArchetype::InfiniteSandbag | EnemyArchetype::FiniteSandbag
                ) {
                    // Sandbags are an exception: they have a weak
                    // counter-punch (PunchWeak), so peaceful=false
                    // but they still have melee.
                    assert!(
                        set.melee.is_none(),
                        "{:?} is peaceful but has melee",
                        archetype,
                    );
                }
            }
        }
    }

    /// enemy_default_action_set picks a per-archetype concrete
    /// attack spec — daytime EFFECTS-flip code reads this to spawn
    /// distinct hitboxes per archetype.
    #[test]
    fn enemy_default_action_set_picks_per_archetype_specs() {
        let slug = make_enemy(EnemyArchetype::PuppySlug);
        let set = enemy_default_action_set(&slug);
        assert!(set.melee.is_none(), "peaceful PuppySlug has no melee");
        assert!(matches!(set.move_style, MoveStyleSpec::Slither));

        let brute = make_enemy(EnemyArchetype::LargeBrute);
        let set = enemy_default_action_set(&brute);
        assert!(matches!(
            set.melee,
            Some(MeleeActionSpec::Lunge(_))
        ));
        assert!(matches!(set.move_style, MoveStyleSpec::WalkHeavy));

        let striker = make_enemy(EnemyArchetype::MediumStriker);
        let set = enemy_default_action_set(&striker);
        assert!(matches!(
            set.melee,
            Some(MeleeActionSpec::Swipe(_))
        ));

        let pirate_shark = make_enemy(EnemyArchetype::PirateOnShark);
        let set = enemy_default_action_set(&pirate_shark);
        assert!(set.ranged.is_some(), "PirateOnShark has ranged");
        assert!(matches!(set.move_style, MoveStyleSpec::Float));
    }
}
