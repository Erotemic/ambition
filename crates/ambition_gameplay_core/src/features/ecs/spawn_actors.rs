//! Actor spawn helpers for ECS feature entities.
//!
//! This module covers bosses, hostile enemies, peaceful NPC actors, dynamic
//! boss minions, and encounter mobs. Static pickups/chests/breakables live in
//! `spawn_static.rs`; composite mount/rider fan-out lives in `spawn_mounts.rs`.

use super::brain_builders::{
    enemy_combat_kit_for_spec, enemy_default_action_set, enemy_default_brain,
};
use super::*;
use bevy::prelude::Name;

/// Declarative seed for the common hostile-actor spawn bundle.
///
/// Authored enemies, encounter mobs, runtime minions, mounts, and riders all
/// share the same core entity shape: feature identity + generic actor combat
/// read models + enemy ECS cluster + brain/action/control.  Keeping that shape
/// here prevents each spawn path from rebuilding the same bundle by hand and
/// makes the mount/rider special cases read as small overrides.
pub(super) struct EnemyActorSpawnPlan {
    entity_name: String,
    feature_id: String,
    feature_name: String,
    feature_aabb: CenteredAabb,
    enemy: super::enemy_clusters::EnemyClusterSeed,
    faction: super::ActorFaction,
    aggression: super::ActorAggression,
    brain: crate::brain::Brain,
    action_set: crate::brain::ActionSet,
    combat_kit: crate::combat::CombatKit,
    held_item: Option<crate::brain::HeldItemSpec>,
}

impl EnemyActorSpawnPlan {
    pub(super) fn hostile(
        entity_name: impl Into<String>,
        feature_id: impl Into<String>,
        feature_name: impl Into<String>,
        feature_aabb: CenteredAabb,
        enemy: super::enemy_clusters::EnemyClusterSeed,
    ) -> Self {
        let brain = enemy_default_brain(&enemy.config);
        let action_set = enemy_default_action_set(&enemy.spec);
        let combat_kit = enemy_combat_kit_for_spec(&enemy.spec);
        let held_item = super::brain_builders::held_item_for_spec(&enemy.spec);
        Self {
            entity_name: entity_name.into(),
            feature_id: feature_id.into(),
            feature_name: feature_name.into(),
            feature_aabb,
            enemy,
            faction: super::ActorFaction::Enemy,
            aggression: super::ActorAggression::hostile_to_player(),
            brain,
            action_set,
            combat_kit,
            held_item,
        }
    }

    pub(super) fn with_faction(mut self, faction: super::ActorFaction) -> Self {
        self.faction = faction;
        self
    }

    pub(super) fn with_aggression(mut self, aggression: super::ActorAggression) -> Self {
        self.aggression = aggression;
        self
    }

    pub(super) fn with_brain(mut self, brain: crate::brain::Brain) -> Self {
        self.brain = brain;
        self
    }

    pub(super) fn with_action_set(mut self, action_set: crate::brain::ActionSet) -> Self {
        self.action_set = action_set;
        self
    }

    pub(super) fn with_combat_kit(mut self, combat_kit: crate::combat::CombatKit) -> Self {
        self.combat_kit = combat_kit;
        self
    }

    pub(super) fn with_held_item(mut self, held_item: Option<crate::brain::HeldItemSpec>) -> Self {
        self.held_item = held_item;
        self
    }

    pub(super) fn without_held_item(mut self) -> Self {
        self.held_item = None;
        self
    }

    pub(super) fn spawn(self, commands: &mut Commands) -> Entity {
        let facing = self.enemy.kin.facing;
        let (identity, disposition, health, combat, intent, cooldowns) =
            enemy_component_snapshot(&self.enemy);
        let cluster_bundle = self.enemy.into_components();
        let entity = commands
            .spawn((
                Name::new(self.entity_name),
                EnemyActorBundle::new(
                    FeatureBaseBundle::new(&self.feature_id, &self.feature_name, self.feature_aabb),
                    identity,
                    disposition,
                    self.faction,
                    ActorPose::from_parts(
                        self.feature_aabb.center,
                        self.feature_aabb.half_size,
                        facing,
                    ),
                    self.combat_kit,
                    self.aggression,
                    health,
                    combat,
                    intent,
                    cooldowns,
                ),
                ActorRuntime::Enemy,
                cluster_bundle,
                self.brain,
                self.action_set,
                crate::brain::ActorControl::default(),
            ))
            .id();
        if let Some(item) = self.held_item {
            commands.entity(entity).insert(super::HeldItem::new(item));
        }
        entity
    }
}

/// Declarative seed for the common peaceful-NPC actor spawn bundle.
///
/// Peaceful NPCs share the same actor read-model shape as enemies, but spawn
/// with NPC clusters, peaceful actions, and retaliation-only aggression. Keeping
/// that shape here makes NPC spawning the sibling of [`EnemyActorSpawnPlan`]
/// instead of another hand-built `EnemyActorBundle` tuple.
pub(super) struct NpcActorSpawnPlan {
    entity_name: String,
    feature_id: String,
    feature_name: String,
    feature_aabb: CenteredAabb,
    npc: super::npc_clusters::NpcClusterScratch,
    brain: crate::brain::Brain,
    action_set: crate::brain::ActionSet,
    combat_kit: crate::combat::CombatKit,
    aggression: super::ActorAggression,
}

impl NpcActorSpawnPlan {
    pub(super) fn peaceful(
        entity_name: impl Into<String>,
        feature_id: impl Into<String>,
        feature_name: impl Into<String>,
        feature_aabb: CenteredAabb,
        npc: super::npc_clusters::NpcClusterScratch,
    ) -> Self {
        let mut npc = npc;
        let brain = npc.as_mut().build_brain();
        let hostile_spec = super::actors::hostile_enemy_spec_for_npc(&npc.config);
        let combat_kit = super::brain_builders::enemy_combat_kit_for_spec(&hostile_spec);
        Self {
            entity_name: entity_name.into(),
            feature_id: feature_id.into(),
            feature_name: feature_name.into(),
            feature_aabb,
            npc,
            brain,
            action_set: crate::brain::ActionSet::peaceful(),
            combat_kit,
            aggression: super::ActorAggression::retaliates_when_hit(
                super::super::NPC_HOSTILE_STRIKE_THRESHOLD as u8,
            ),
        }
    }

    pub(super) fn spawn(self, commands: &mut Commands) -> Entity {
        let facing = self.npc.kin.facing;
        // Sprite-metadata render size lives on the SHARED `ActorRenderSize`
        // component (not the NPC-only cluster) so it survives a hostile flip —
        // otherwise the migrated enemy loses it and the body-sized collision
        // gets `collision_scale` re-applied, ballooning the sprite.
        let render_size = self.npc.render_size;
        let (identity, disposition, health, combat, intent, cooldowns) =
            super::actors::npc_component_snapshot(&self.npc.config, &self.npc.status);
        let cluster_bundle = self.npc.into_components();
        let mut entity = commands.spawn((
            Name::new(self.entity_name),
            EnemyActorBundle::new(
                FeatureBaseBundle::new(&self.feature_id, &self.feature_name, self.feature_aabb),
                identity,
                disposition,
                super::ActorFaction::Npc,
                ActorPose::from_parts(self.feature_aabb.center, self.feature_aabb.half_size, facing),
                self.combat_kit,
                self.aggression,
                health,
                combat,
                intent,
                cooldowns,
            ),
            ActorRuntime::Npc,
            cluster_bundle,
            self.brain,
            self.action_set,
            crate::brain::ActorControl::default(),
        ));
        if let Some(size) = render_size {
            entity.insert(crate::features::ActorRenderSize(size));
        }
        entity.id()
    }
}

pub(super) fn spawn_boss(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::actor::BossBrain>,
) {
    let boss = BossClusterScratch::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
    );
    bevy::log::info!(
        target: "ambition::boss_spawn",
        "spawn_boss id={} name={:?} brain={:?} → behavior.id={} combat_size={:?}",
        boss.config.id,
        boss.config.name,
        authored.payload,
        boss.config.behavior.id,
        boss.as_ref().combat_size(),
    );
    let initial_phase = BossPhase::from_alive(boss.status.alive);
    let feature_aabb = CenteredAabb::from_center_size(boss.kin.pos, boss.as_ref().render_size());
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
    let encounter_id = boss.config.behavior.id.clone();
    let combat_tuning = crate::time::feel::SandboxFeelTuning::default().feature_combat_tuning();
    let cycle_attack_active = boss
        .config
        .behavior
        .attack_active
        .max(combat_tuning.boss_attack_active)
        .max(0.01);
    // GNU-ton dodges its own apple rain by side-stepping during the
    // strike window. Other bosses don't have a self-dodge.
    let (apple_rain_dodge_amp, apple_rain_dodge_freq) =
        if encounter_id == crate::features::bosses::GNU_TON_ENCOUNTER_ID {
            (70.0, 1.6)
        } else {
            (0.0, 0.0)
        };
    let brain_cfg = crate::brain::BossPatternCfg {
        aggressiveness: 1.0,
        encounter_id: encounter_id.clone(),
        pattern: boss.config.behavior.attack_pattern.clone(),
        movement: boss.config.behavior.movement.clone(),
        movement_phase2: boss.config.behavior.movement_phase2.clone(),
        movement_enrage: boss.config.behavior.movement_enrage.clone(),
        strike_speed_scale: boss.config.behavior.strike_speed_scale,
        spawn: boss.config.spawn,
        combat_size: boss.as_ref().combat_size(),
        cycle_attack_windup: boss.config.behavior.attack_windup.max(0.01),
        cycle_attack_active,
        cycle_attack_cooldown: boss.config.behavior.attack_cooldown.max(0.05),
        cycle_attacks: boss.config.behavior.attacks.clone(),
        apple_rain_dodge_amp,
        apple_rain_dodge_freq,
        macro_tuning: boss.config.behavior.macro_tuning,
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
    let boss_combat_kit = CombatKit::from_action_set(&boss_action_set);
    let (boss_identity, boss_disposition, boss_health, boss_combat, boss_intent, boss_cooldowns) =
        boss_component_snapshot(boss.as_ref(), &crate::brain::BossAttackState::default());
    let boss_facing = boss.kin.facing;
    let boss_components = boss.into_components();
    let mut entity = commands.spawn((
        Name::new(format!("Feature boss: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        feature_aabb,
        // BossPatternTimer is a presentation-side mirror of the brain's
        // `BossPatternState.pattern_timer`; updated each tick by
        // `update_ecs_bosses`. Initial value is 0.0 because the brain
        // state defaults to a fresh `BossPatternState`.
        BossPatternTimer(0.0),
        BossDeathAnimation::default(),
        initial_phase,
        super::ActorFaction::Boss,
        super::ActorTarget::default(),
        ActorPose::from_parts(feature_aabb.center, feature_aabb.half_size, boss_facing),
        (
            DamageableVolumes::default(),
            PogoPolicy::FromDamageable,
            PogoTargetVolumes::default(),
            boss_components,
        ),
    ));
    entity.insert((
        // Shared actor combat read models. Boss-specific encounter
        // phase / music / rewards stay on BossFeature + boss
        // encounter systems, but generic combat/targeting code can
        // now reason about bosses through the same pieces as other
        // actors.
        boss_identity,
        boss_disposition,
        boss_health,
        boss_combat,
        boss_intent,
        boss_cooldowns,
        boss_combat_kit,
        ActorAggression::hostile_to_player(),
    ));
    entity.insert((
        // The brain bundle stays grouped because each piece is required
        // for the boss tick chain.
        brain,
        boss_action_set,
        crate::brain::ActorControl::default(),
        crate::brain::BossAttackState::default(),
    ));
    // Per-boss special-technique state (apple-rain accumulator, overfit-volley
    // samples, pit/cross/cascade gates, eye-beam lock) is now content-owned
    // (`ambition_content::bosses::specials`), attached to every boss via
    // `register_required_components::<BossConfig, _>()` in the content plugin —
    // the engine spawn names no boss special.
}
/// Runtime minion spawner — used by boss EFFECTS consumers (e.g.
/// PitTrap puppy_slug spawn, MinionCascade slop adds). Mirrors
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_runtime_minion(
    commands: &mut Commands,
    id: impl Into<String>,
    name: impl Into<String>,
    world_pos: ae::Vec2,
    half_size: ae::Vec2,
    archetype_id: &str,
    encounter_id: impl Into<String>,
    // Allegiance of the spawned minion. Boss adds pass `Enemy` +
    // `hostile_to_player`; the puppy-slug-gun passes `Player` + `passive` so the
    // summon damages the player's enemies (via the `can_damage` matrix) but never
    // the player, and just wanders rather than targeting.
    faction: super::ActorFaction,
    aggression: super::ActorAggression,
) -> bevy::ecs::entity::Entity {
    let id = id.into();
    let name = name.into();
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(world_pos, half_size);
    let brain = crate::actor::EnemyBrain::Custom(archetype_id.into());
    let mut enemy =
        super::enemy_clusters::EnemyClusterSeed::new(id.clone(), name.clone(), aabb, brain, &[]);
    // `EnemyClusterSeed::new` already sets HP from the resolved spec.
    // Boss-spawned minions shouldn't auto-respawn — they're part of
    // the encounter, not a static sandbag.
    enemy.status.respawn_timer = 999_999.0;
    let feature_aabb = CenteredAabb::from_aabb(aabb);
    let entity = EnemyActorSpawnPlan::hostile(
        format!("Runtime minion: {name}"),
        id.clone(),
        name.clone(),
        feature_aabb,
        enemy,
    )
    .with_faction(faction)
    .with_aggression(aggression)
    .spawn(commands);
    commands
        .entity(entity)
        .insert(super::EncounterMob::new(encounter_id));
    entity
}

pub(super) fn spawn_enemy(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::actor::EnemyBrain>,
    paths: &[(String, crate::actor::KinematicPath)],
) {
    let spec = super::super::enemies::spec_for_brain(&authored.payload);
    if spec.is_composite() {
        super::spawn_mounts::spawn_composite_mount_rider(commands, authored, paths, &spec);
        return;
    }
    let enemy = super::enemy_clusters::EnemyClusterSeed::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    spawn_solo_enemy(commands, enemy, authored);
}

/// Single-entity hostile spawn — the common path after composite
/// mount/rider fan-out has been handled.
pub(super) fn spawn_solo_enemy(
    commands: &mut Commands,
    enemy: super::enemy_clusters::EnemyClusterSeed,
    authored: &crate::rooms::Authored<crate::actor::EnemyBrain>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    EnemyActorSpawnPlan::hostile(
        format!("Feature actor enemy: {}", authored.name),
        authored.id.clone(),
        authored.name.clone(),
        feature_aabb,
        enemy,
    )
    .spawn(commands);
}
pub(super) fn spawn_interactable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Interactable>,
    paths: &[(String, crate::actor::KinematicPath)],
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let interactable = &authored.payload;
    if matches!(
        interactable.kind,
        crate::interaction::InteractionKind::Npc { .. }
    ) {
        let npc = super::npc_clusters::NpcClusterScratch::new_with_paths(
            authored.id.clone(),
            authored.name.clone(),
            authored.aabb,
            interactable.clone(),
            paths,
        );
        NpcActorSpawnPlan::peaceful(
            format!("Feature actor npc: {}", authored.name),
            authored.id.clone(),
            authored.name.clone(),
            feature_aabb,
            npc,
        )
        .spawn(commands);
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
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy =
        super::enemy_clusters::EnemyClusterSeed::new(id.clone(), id.clone(), aabb, brain, &[]);
    // `EnemyClusterSeed::new` already sets HP from the resolved spec.
    // Encounter mobs should not auto-respawn like training sandbags.
    enemy.status.respawn_timer = 999_999.0;
    let feature_aabb = CenteredAabb::from_center_size(pos, size);
    let entity = EnemyActorSpawnPlan::hostile(
        format!("Encounter mob: {id}"),
        id.clone(),
        id.clone(),
        feature_aabb,
        enemy,
    )
    .spawn(commands);
    commands
        .entity(entity)
        .insert(EncounterMob::new(encounter_id));
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

/// Lib-side executor for `Effect::Summon`: materializes each summon via
/// `spawn_runtime_minion`. Lives next to the spawner (not in
/// `effects::apply_effects`) so the `ambition_vfx` crate stays free of the
/// enemy-roster substrate. Summons authored so far are all hostile-to-player.
pub fn apply_summon_effects(
    mut commands: bevy::prelude::Commands,
    mut requests: bevy::prelude::MessageReader<crate::effects::EffectRequest>,
) {
    for req in requests.read() {
        if let crate::effects::Effect::Summon(s) = &req.effect {
            spawn_runtime_minion(
                &mut commands,
                s.id.clone(),
                s.name.clone(),
                s.pos,
                s.half_size,
                &s.archetype_id,
                s.encounter_id.clone(),
                s.faction,
                super::ActorAggression::hostile_to_player(),
            );
        }
    }
}
