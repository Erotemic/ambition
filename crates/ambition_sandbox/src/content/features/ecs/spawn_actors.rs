//! Actor spawn helpers for ECS feature entities.
//!
//! This module covers bosses, hostile enemies, peaceful NPC actors, dynamic
//! boss minions, and encounter mobs. Static pickups/chests/breakables live in
//! `spawn_static.rs`; composite mount/rider fan-out lives in `spawn_mounts.rs`.

use super::brain_builders::{enemy_default_action_set, enemy_default_brain};
use super::*;
use bevy::prelude::Name;

pub(super) fn spawn_boss(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::actor::BossBrain>,
) {
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
        BossDeathAnimation::default(),
        initial_phase,
        super::ActorFaction::Boss,
        super::ActorTarget::default(),
        (
            DamageableVolumes::default(),
            PogoPolicy::FromDamageable,
            PogoTargetVolumes::default(),
            BossFeature::new(boss),
        ),
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
            super::EyeBeamState::default(),
            super::MinimaTrapState::default(),
            super::SaddlePointState::default(),
            super::GradientCascadeState::default(),
        ),
    ));
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
    let brain = crate::actor::EnemyBrain::Custom(archetype_id.into());
    let archetype = EnemyArchetype::from_brain(&brain);
    let mut enemy = EnemyRuntime::new(id.clone(), name.clone(), aabb, brain, &[]);
    enemy.archetype = archetype;
    enemy.health = crate::actor::Health::new(archetype.max_health());
    // Boss-spawned minions shouldn't auto-respawn — they're part of
    // the encounter, not a static sandbag.
    enemy.respawn_timer = 999_999.0;
    let feature_aabb = FeatureAabb::from_aabb(aabb);
    let brain_component = enemy_default_brain(&enemy);
    let action_set = enemy_default_action_set(&enemy);
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    let held_item = super::brain_builders::held_item_for_archetype(archetype);
    let entity = commands
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
                damageable_volumes: DamageableVolumes::default(),
                pogo_policy: PogoPolicy::FromDamageable,
                pogo_target_volumes: PogoTargetVolumes::default(),
            },
            actor,
            super::EncounterMob::new(encounter_id),
            brain_component,
            action_set,
            crate::brain::ActorControl::default(),
        ))
        .id();
    if let Some(item) = held_item {
        commands.entity(entity).insert(super::HeldItem::new(item));
    }
    entity
}

pub(super) fn spawn_enemy(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::actor::EnemyBrain>,
    paths: &[(String, crate::actor::KinematicPath)],
) {
    // Build a probe runtime to inspect the resolved archetype. The
    // composite "X on Shark" archetypes fan out into a mount entity +
    // a rider entity linked via [`super::Mountable`] /
    // [`super::RidingOn`]; everything else goes through the standard
    // single-entity spawn.
    let probe = EnemyRuntime::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    if super::mount::is_composite_spawn(probe.archetype) {
        super::spawn_mounts::spawn_composite_mount_rider(
            commands,
            authored,
            paths,
            probe.archetype,
        );
        return;
    }
    spawn_solo_enemy(commands, probe, authored);
}

/// Single-entity hostile spawn — the common path. Mirrors the
/// legacy `spawn_enemy` body.
pub(super) fn spawn_solo_enemy(
    commands: &mut Commands,
    enemy: EnemyRuntime,
    authored: &crate::rooms::Authored<crate::actor::EnemyBrain>,
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let brain = enemy_default_brain(&enemy);
    let action_set = enemy_default_action_set(&enemy);
    let held_item = super::brain_builders::held_item_for_archetype(enemy.archetype);
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    let entity = commands
        .spawn((
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
                damageable_volumes: DamageableVolumes::default(),
                pogo_policy: PogoPolicy::FromDamageable,
                pogo_target_volumes: PogoTargetVolumes::default(),
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
        ))
        .id();
    if let Some(item) = held_item {
        commands.entity(entity).insert(super::HeldItem::new(item));
    }
}
pub(super) fn spawn_interactable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Interactable>,
    paths: &[(String, crate::actor::KinematicPath)],
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let interactable = &authored.payload;
    if matches!(
        interactable.kind,
        crate::interaction::InteractionKind::Npc { .. }
    ) {
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
                damageable_volumes: DamageableVolumes::default(),
                pogo_policy: PogoPolicy::FromDamageable,
                pogo_target_volumes: PogoTargetVolumes::default(),
            },
            actor,
            brain,
            crate::brain::ActionSet::peaceful(),
            crate::brain::ActorControl::default(),
            // The sim entity owns a lightweight Transform because the
            // universal ActionSet resolver queries Transform for the
            // action origin. Peaceful NPCs can become hostile in-place;
            // without this component they chase via `update_ecs_actors`
            // but their melee/ranged intents are silently skipped by
            // `emit_brain_action_messages`.
            bevy::transform::components::Transform::from_xyz(
                feature_aabb.center.x,
                feature_aabb.center.y,
                0.0,
            ),
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
pub(super) fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: crate::actor::EnemyBrain,
    pos: ae::Vec2,
    size: ae::Vec2,
) {
    let encounter_id = encounter_id.into();
    let archetype = EnemyArchetype::from_brain(&brain);
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy = EnemyRuntime::new(id.clone(), id.clone(), aabb, brain, &[]);
    enemy.archetype = archetype;
    enemy.health = crate::actor::Health::new(archetype.max_health());
    // Encounter mobs should not auto-respawn like training sandbags.
    enemy.respawn_timer = 999_999.0;
    let brain = enemy_default_brain(&enemy);
    let action_set = enemy_default_action_set(&enemy);
    let held_item = super::brain_builders::held_item_for_archetype(enemy.archetype);
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
            damageable_volumes: DamageableVolumes::default(),
            pogo_policy: PogoPolicy::FromDamageable,
            pogo_target_volumes: PogoTargetVolumes::default(),
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
pub(super) fn despawn_encounter_mobs(
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
