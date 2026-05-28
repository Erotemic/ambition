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
    bevy::log::info!(
        target: "ambition::boss_spawn",
        "spawn_boss id={} name={:?} brain={:?} → behavior.id={} combat_size={:?}",
        boss.id,
        boss.name,
        authored.payload,
        boss.behavior.id,
        boss.combat_size(),
    );
    let initial_phase = BossPhase::from_alive(boss.alive);
    // BossPattern brain owns boss intent. The cfg snapshots the
    // authored behavior profile's pattern + movement at spawn
    // time, plus the per-boss spawn anchor and combat collision
    // size the movement / dodge math reads. The brain's
    // `tick_boss_pattern` (driven by `tick_boss_brains_system`)
    // is the single intent producer; `BossRuntime::integrate_body`
    // only consumes the resulting `desired_vel`.
    // Canonical encounter id from the boss runtime's behavior
    // (which `BossRuntime::new` resolved via the brain's
    // `PhaseScript:` payload). Using the runtime-resolved id
    // instead of `encounter_id_from_name(boss.name)` ensures an
    // LDtk BossSpawn with a flavor display name still wires the
    // apple-rain self-dodge (and any future per-encounter
    // overrides) to the right boss.
    let encounter_id = boss.behavior.id.clone();
    let combat_tuning = crate::time::feel::SandboxFeelTuning::default().feature_combat_tuning();
    let cycle_attack_active = boss
        .behavior
        .attack_active
        .max(combat_tuning.boss_attack_active)
        .max(0.01);
    // GNU-ton dodges its own apple rain by side-stepping during the
    // strike window. Other bosses don't have a self-dodge.
    let (apple_rain_dodge_amp, apple_rain_dodge_freq) =
        if encounter_id == crate::content::features::bosses::GNU_TON_ENCOUNTER_ID {
            (70.0, 1.6)
        } else {
            (0.0, 0.0)
        };
    let brain_cfg = crate::brain::BossPatternCfg {
        aggressiveness: 1.0,
        encounter_id: encounter_id.clone(),
        pattern: boss.behavior.attack_pattern.clone(),
        movement: boss.behavior.movement.clone(),
        movement_phase2: boss.behavior.movement_phase2.clone(),
        movement_enrage: boss.behavior.movement_enrage.clone(),
        strike_speed_scale: boss.behavior.strike_speed_scale,
        spawn: boss.spawn,
        combat_size: boss.combat_size(),
        cycle_attack_windup: boss.behavior.attack_windup.max(0.01),
        cycle_attack_active,
        cycle_attack_cooldown: boss.behavior.attack_cooldown.max(0.05),
        cycle_attacks: boss.behavior.attacks.clone(),
        apple_rain_dodge_amp,
        apple_rain_dodge_freq,
        macro_tuning: boss.behavior.macro_tuning,
    };
    let brain = crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::BossPattern {
        cfg: brain_cfg,
        state: crate::brain::BossPatternState::default(),
    });
    // Bosses spawn with an offensive ActionSet — Bolt ranged +
    // empty special slot. Per-boss specials (including GNU-ton's
    // apple rain) are now emitted by `tick_boss_brains_system` via
    // direct `MessageWriter<ActorActionMessage>` writes, looking up
    // the spec through `boss_special_for_profile`. Keeping
    // `special: None` here prevents the generic
    // `emit_brain_action_messages` resolver from emitting a
    // duplicate Special message that would double-fire the
    // consumer.
    let _ = encounter_id; // resolved upstream via `boss.behavior`
    let boss_action_set = crate::brain::ActionSet {
        ranged: Some(crate::brain::RangedActionSpec::Bolt {
            speed: 380.0,
            damage: 1,
        }),
        special: None,
        move_style: crate::brain::MoveStyleSpec::Walk,
        ..Default::default()
    };
    commands.spawn((
        Name::new(format!("Feature boss: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        FeatureAabb::from_center_size(boss.pos, boss.render_size()),
        // BossPatternTimer is a presentation-side mirror of the brain's
        // `BossPatternState.pattern_timer`; updated each tick by
        // `update_ecs_bosses`. Initial value is 0.0 because the brain
        // state defaults to a fresh `BossPatternState`.
        BossPatternTimer(0.0),
        initial_phase,
        super::ActorFaction::Boss,
        super::ActorTarget::default(),
        BossFeature::new(boss),
        (
            // Sub-tuple keeps the outer bundle under Bevy's
            // 15-tuple Bundle arity limit. The brain bundle stays
            // grouped because each piece is required for the boss
            // tick chain. Per-special state components live in a
            // second sub-tuple alongside `AppleRainSpawnState` — see
            // `content/features/ecs/brain_effects.rs` for the
            // consumers that drive each one.
            brain,
            boss_action_set,
            crate::brain::ActorControl::default(),
            crate::brain::BossAttackState::default(),
            super::AppleRainSpawnState::default(),
        ),
        (
            // Gradient Sentinel special state. Defaulted-attached to
            // every boss so a future encounter can adopt the same
            // attacks without re-touching the spawn wiring.
            super::OverfitVolleyState::default(),
            super::MinimaTrapState::default(),
            super::SaddlePointState::default(),
            super::GradientCascadeState::default(),
        ),
    ));
}

fn spawn_pickup(commands: &mut Commands, authored: &crate::rooms::Authored<crate::interaction::Pickup>) {
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

fn spawn_chest(commands: &mut Commands, authored: &crate::rooms::Authored<crate::interaction::Chest>) {
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

fn spawn_breakable(commands: &mut Commands, authored: &crate::rooms::Authored<crate::interaction::Breakable>) {
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

/// Runtime minion spawner — used by boss EFFECTS consumers (e.g.
/// MinimaTrap puppy_slug spawn, GradientCascade slop adds). Mirrors
/// `spawn_encounter_mob` but takes plain values from a Bevy system
/// so callers don't have to wrap them in an `Authored<EnemyBrain>`.
/// The resulting entity carries the same component set as authored
/// encounter mobs — crucially including the `EncounterMob` marker
/// so `spawn_dynamic_feature_visuals` picks it up next frame and
/// attaches the right sprite. Without that marker the minion would
/// spawn invisibly (ECS-only).
///
/// `archetype_id` matches one of the strings in `BRAIN_NAME_TO_ARCHETYPE`
/// (`"puppy_slug"`, `"small_lurker"`, …); unknown strings fall back
/// to `Combatant` via `EnemyArchetype::from_brain`. `half_size` is
/// the spawn AABB half-extent (the archetype spec's `default_size`
/// usually overrides this anyway). `id` should be unique per spawn
/// so per-entity systems don't collide on identity. `encounter_id`
/// scopes the minion to a parent encounter so room reset / boss
/// despawn cleans it up alongside the boss.
pub(crate) fn spawn_runtime_minion(
    commands: &mut Commands,
    id: impl Into<String>,
    name: impl Into<String>,
    world_pos: ae::Vec2,
    half_size: ae::Vec2,
    archetype_id: &str,
    encounter_id: impl Into<String>,
) -> bevy::ecs::entity::Entity {
    let id = id.into();
    let name = name.into();
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(world_pos, half_size);
    let brain = ae::EnemyBrain::Custom(archetype_id.into());
    let archetype = EnemyArchetype::from_brain(&brain);
    let mut enemy = EnemyRuntime::new(id.clone(), name.clone(), aabb, brain, &[]);
    enemy.archetype = archetype;
    enemy.health = ae::Health::new(archetype.max_health());
    // Boss-spawned minions shouldn't auto-respawn — they're part of
    // the encounter, not a static sandbag.
    enemy.respawn_timer = 999_999.0;
    let feature_aabb = FeatureAabb::from_aabb(aabb);
    let brain_component = enemy_default_brain(&enemy);
    let action_set = enemy_default_action_set(&enemy);
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    commands
        .spawn((
            Name::new(format!("Runtime minion: {name}")),
            EnemyActorBundle {
                base: FeatureBaseBundle::new(&id, &name, feature_aabb),
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
            super::EncounterMob::new(encounter_id),
            brain_component,
            action_set,
            crate::brain::ActorControl::default(),
        ))
        .id()
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
    // Attach Brain + ActionSet + ActorControl to the enemy entity.
    // `update_ecs_actors` runs the runtime's per-tick frame and
    // writes it into `ActorControl` — `EnemyRuntime` is the single
    // intent producer for hostile actors today. The resolver
    // translates the frame into `ActorActionMessage`s consumed by
    // EFFECTS systems (see `spawn_enemy_projectiles_from_brain_actions`
    // for the ranged case, `start_enemy_melee_from_brain_actions`
    // for melee). The Brain component stays attached as a future
    // migration handle for when the legacy AI lifts into the brain.
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
        // `emit_brain_action_messages` requires a `Transform` on the
        // sim entity to compute the action `origin`. Without this,
        // the resolver silently skips enemies (the visual entity has
        // a Transform but the sim entity does not), and every brain
        // intent — melee, ranged, special — gets dropped. Position
        // is kept fresh by `update_ecs_actors` via FeatureAabb; the
        // Transform here is just the schema requirement.
        bevy::transform::components::Transform::from_xyz(
            feature_aabb.center.x,
            feature_aabb.center.y,
            0.0,
        ),
    ));
}

/// Map an `EnemyRuntime` to a Brain template. Reads the archetype's
/// actual tunings (chase_speed / aggro_radius / attack_range) so
/// the brain's MeleeBrute cfg matches what the existing AI loop
/// uses. PuppySlug gets a Wanderer; sandbags get StandStill;
/// everyone else gets a MeleeBrute keyed to their archetype.
///
/// Today the hostile-actor pipeline writes the LEGACY AI's frame
/// into `ActorControl` (the brain's shadow output is overwritten);
/// see `update_ecs_actors`. The resolver still produces correct
/// `ActorActionMessage`s because the legacy frame carries the same
/// intent (melee_pressed / fire) that the brain would emit.
/// Future work moves the brain's frame into the authority position
/// for hostile actors too.
///
/// Map an `EnemyRuntime` to a default ActionSet keyed off its
/// archetype. Sandbags + peaceful archetypes get
/// [`ActionSet::peaceful`]; striker / brute archetypes get a Swipe
/// (or Lunge for brutes); ranged archetypes get an Arrow / Pistol
/// / Bolt where appropriate. The resolver consumes these to
/// produce ActorActionMessages for the EFFECTS-stage consumers.
/// Build the enemy's default `ActionSet` from its archetype spec.
/// Reads `melee_spec()` / `ranged_spec()` / `move_style()` straight
/// off the data-driven `EnemyArchetypeSpec` — every spec value
/// (timings, damage, reach) lives in `enemy_archetypes.ron`. Adding
/// a new archetype is a single RON row + a new `EnemyArchetype` enum
/// variant.
pub(super) fn enemy_default_action_set(enemy: &EnemyRuntime) -> crate::brain::ActionSet {
    use crate::brain::ActionSet;
    let archetype = enemy.archetype;
    let move_style = archetype.move_style();
    // PuppySlug / PirateHeavy stay peaceful by default (no melee /
    // no ranged). The brain still ticks; the resolver sees an empty
    // ActionSet and emits no messages. Provoking-into-hostility
    // (Patrol → MeleeBrute swap, ActionSet swap) is handled by the
    // hostile-flip path in `content/features/ecs/damage.rs`.
    if !archetype.attacks_player() {
        return ActionSet {
            move_style,
            ..ActionSet::default()
        };
    }
    ActionSet {
        melee: archetype.melee_spec(),
        ranged: archetype.ranged_spec(),
        move_style,
        ..Default::default()
    }
}

/// Build the enemy's default `Brain` from its archetype spec.
/// Reads `brain_template()` off the consolidated `EnemyArchetypeSpec`
/// so adding a new archetype is a single row, not a parallel match.
pub(super) fn enemy_default_brain(enemy: &EnemyRuntime) -> crate::brain::Brain {
    use super::super::enemies::EnemyBrainTemplate;
    use crate::brain::{
        Brain, MeleeBruteCfg, MeleeBruteState, SmashState, StateMachineCfg, WandererCfg,
        WandererState,
    };
    let archetype = enemy.archetype;
    match archetype.brain_template() {
        EnemyBrainTemplate::StandStill => Brain::StateMachine(StateMachineCfg::StandStill),
        EnemyBrainTemplate::Wanderer => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        }),
        EnemyBrainTemplate::MeleeBrute => Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg {
                aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
                aggro_radius: archetype.aggro_radius(),
                attack_range: archetype.attack_range(),
                chase_speed: archetype.chase_speed(),
            },
            state: MeleeBruteState::default(),
        }),
        EnemyBrainTemplate::Smash => Brain::StateMachine(StateMachineCfg::Smash {
            cfg: smash_cfg_for_archetype(archetype),
            state: SmashState {
                rng_seed: crate::attack_choreography::seed_from_id(&enemy.id) as u64,
                ..Default::default()
            },
        }),
    }
}

/// Build a `SmashCfg` from the archetype's tuning row. Heavier
/// archetypes (Brute) get a longer attack reach + slower chase;
/// lighter archetypes (Skitter / Lurker) get a tighter engage band.
///
/// IMPORTANT: the archetype's `attack_range` in `enemy_archetypes.ron`
/// is the AI-decision aggro distance (~150 px for goblins). That's
/// the radius at which the brain commits to "I'm attacking this
/// target", NOT the distance at which the swing actually hits. The
/// melee swing's reach is in the `SwipeSpec::reach_px` (~28 px); the
/// brain needs to close to roughly `body_half_width + swing_reach`
/// before emitting MeleeAttack, otherwise the windup fires from too
/// far away and the player walks out of the active window.
fn smash_cfg_for_archetype(arch: super::super::enemies::EnemyArchetype) -> crate::brain::SmashCfg {
    use super::super::enemies::EnemyArchetype::*;
    use crate::brain::SmashCfg;
    let base = match arch {
        LargeBrute | LargeColossus => SmashCfg::BRUTE_DEFAULT,
        _ => SmashCfg::STRIKER_DEFAULT,
    };
    // Per-archetype hit-band sizing. Mirrors the legacy
    // `MeleeBruteCfg` defaults (Striker = 36 px, Brute = 44 px) so
    // the actor closes the gap before swinging instead of windup-
    // committing at the AI aggro distance.
    let hit_band = match arch {
        LargeBrute | LargeColossus => 44.0,
        SmallSkitter | SmallLurker => 32.0,
        _ => 36.0,
    };
    SmashCfg {
        aggro_radius: arch.aggro_radius(),
        attack_range: hit_band,
        // Engage band: the brain holds position once inside this
        // radius even if the swing is on cooldown. Slightly larger
        // than `attack_range` so the actor doesn't bob in/out of
        // engage as it inches forward through approach.
        engage_distance: hit_band * 1.6,
        // Retreat threshold — well inside the hit band so a player
        // dashing into the goblin's space pushes it back rather
        // than getting eaten.
        too_close_distance: (hit_band * 0.5).max(18.0),
        chase_speed: arch.chase_speed(),
        retreat_speed: arch.chase_speed() * 0.75,
        ..base
    }
}

fn spawn_interactable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Interactable>,
    paths: &[(String, ae::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let interactable = &authored.payload;
    if matches!(interactable.kind, crate::interaction::InteractionKind::Npc { .. }) {
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
    } else if let crate::interaction::InteractionKind::Custom(payload) = &interactable.kind {
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
        // Same Transform requirement as `spawn_enemy` — see that
        // path for the rationale. Without it, encounter-spawned mobs
        // are silently skipped by `emit_brain_action_messages`.
        bevy::transform::components::Transform::from_xyz(pos.x, pos.y, 0.0),
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

    /// Regression net: spawning an encounter mob attaches a
    /// per-archetype Brain (MeleeBrute by default since
    /// medium_striker is hostile). Verifies the spawn_encounter_mob
    /// path threads the brain through end-to-end.
    #[test]
    fn encounter_mob_brain_is_per_archetype_melee_brute() {
        use crate::brain::{Brain, StateMachineCfg};
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
        let mut q = app.world_mut().query::<&Brain>();
        let brain = q.iter(app.world()).next().expect("encounter mob exists");
        // medium_striker is a hostile archetype → MeleeBrute brain.
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::MeleeBrute { .. })
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
                payload: ae::BossBrain::Dormant,
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

        let striker = make_enemy(EnemyArchetype::MediumStriker);
        match enemy_default_brain(&striker) {
            Brain::StateMachine(StateMachineCfg::MeleeBrute { cfg, .. }) => {
                assert!(cfg.aggressiveness > 0.0);
                // Brain's chase_speed mirrors the archetype tuning.
                assert!(
                    (cfg.chase_speed - EnemyArchetype::MediumStriker.chase_speed()).abs() < 0.01
                );
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
