//! ECS-feature spawn paths.
//!
//! Both static room features (authored entities from `RoomSpec`) and
//! dynamic encounter mobs land here. The static path is per-family —
//! one loop per `RoomSpec.{pickups,chests,…}` — so adding a new
//! authored entity type is "add a new Vec on RoomSpec + a new loop
//! here" rather than "edit a match arm somewhere."

use super::*;
use super::variation::{five_f32s_from_seed, seed_from_id};
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
    authored: &crate::rooms::Authored<crate::combat::DamageVolume>,
    paths: &[(String, crate::actor::KinematicPath)],
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

fn spawn_boss(commands: &mut Commands, authored: &crate::rooms::Authored<crate::actor::BossBrain>) {
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

fn spawn_pickup(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Pickup>,
) {
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

fn spawn_chest(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Chest>,
) {
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

fn spawn_breakable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Breakable>,
) {
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
        spawn_composite_mount_rider(commands, authored, paths, probe.archetype);
        return;
    }
    spawn_solo_enemy(commands, probe, authored);
}

/// Single-entity hostile spawn — the common path. Mirrors the
/// legacy `spawn_enemy` body.
fn spawn_solo_enemy(
    commands: &mut Commands,
    enemy: EnemyRuntime,
    authored: &crate::rooms::Authored<crate::actor::EnemyBrain>,
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
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

/// Fan a composite "X on Shark" spawn into a mount entity + a rider
/// entity. Both are spawned at the authored position; the per-tick
/// [`super::sync_riders_to_mounts`] system snaps the rider to the
/// mount's offset from frame one.
///
/// Mount: `BurningFlyingShark` archetype with an explicit orbiting
/// Skirmisher-style mount brain so the fused shark+pirate encounter
/// keeps its aerial height changes and spreads out instead of
/// clumping. Health comes from the composite spec (PirateOnShark = 6,
/// PirateHeavyOnShark = 7) so the body HP pool stays as authored. The
/// riderless shark now has its own dedicated Shark brain via
/// `enemy_default_brain`.
///
/// Rider: `PirateRaider` for the light composite, `PirateHeavy` for
/// the heavy composite. Brain is explicitly built as Skirmisher with
/// the composite's `Bolt` ranged spec — the rider's STANDALONE brain
/// (Smash for raider, MeleeBrute for heavy) is what gets restored
/// after the mount dies; the bolt-firing behavior lives only while
/// mounted. Aggressiveness is forced ON regardless of the rider
/// archetype's `attacks_player()` default so a dismounted PirateHeavy
/// (which is normally peaceful Cove crew) keeps fighting after the
/// shark is killed under her.
fn spawn_composite_mount_rider(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::actor::EnemyBrain>,
    paths: &[(String, crate::actor::KinematicPath)],
    composite_archetype: EnemyArchetype,
) {
    use crate::brain::{
        ActionSet, Brain, RangedActionSpec, SkirmisherCfg, SkirmisherState, StateMachineCfg,
    };

    // Spawn both at the authored center. The mount's standalone
    // size is its `default_size`; the rider rides at
    // `pirate_on_shark_rider_offset(mount.size, rider.size)`.
    let center = authored.aabb.center();
    let mount_archetype = EnemyArchetype::BurningFlyingShark;
    let mount_size = mount_archetype
        .default_size()
        .expect("BurningFlyingShark has a default_size in archetype_specs");
    let mount_aabb = ae::Aabb::new(center, mount_size * 0.5);

    // Mount HP: take the composite spec's body HP rather than the
    // standalone BurningFlyingShark's default, so tuning the
    // composite's HP in the RON works as expected.
    let composite_hp = composite_archetype.max_health();
    // Mount keeps the authored id so the room-side FeatureVisual
    // entity (spawned by `spawn_room_visuals`) matches and resolves
    // its sprite via the standard upgrade path. The rider takes a
    // suffixed id (`<authored>:rider`) and gets its own FeatureVisual
    // entity from `spawn_composite_visuals` in
    // `presentation::rendering::world`.
    let mount_id = authored.id.clone();
    let mount_name = "Burning Flying Shark".to_string();
    let mut mount_enemy = EnemyRuntime::new(
        mount_id.clone(),
        mount_name.clone(),
        mount_aabb,
        crate::actor::EnemyBrain::Custom("burning_flying_shark".into()),
        paths,
    );
    mount_enemy.health = crate::actor::Health::new(composite_hp);

    // Rider archetype + variant name. The light composite is always
    // a Pirate Raider; the heavy composite parses the authored name
    // (e.g. "Iron Mary on Shark") to pick the variant for the sprite
    // layer.
    let (rider_archetype, rider_variant_name) = match composite_archetype {
        EnemyArchetype::PirateHeavyOnShark => {
            let base = authored
                .name
                .strip_suffix(" on Shark")
                .unwrap_or("Broadside Bess")
                .to_string();
            (EnemyArchetype::PirateHeavy, base)
        }
        _ => (EnemyArchetype::PirateRaider, "Pirate Raider".to_string()),
    };
    // Standalone size = full cove-pirate hitbox (44x78 for the
    // raider; 72x110 for a heavy). Mounted size = half that, so the
    // rider visually fits ON the shark instead of dwarfing it. The
    // dissolve path restores standalone size to `EnemyRuntime.size`.
    let standalone_size = rider_archetype
        .default_size()
        .expect("rider archetype has a default_size");
    let mounted_size = standalone_size * 0.5;
    // Rider starts at the mounted footprint so its initial AABB
    // matches the visual that will resolve through
    // `upgrade_enemy_sprites` (which reads `view.size`).
    let rider_offset = super::mount::pirate_on_shark_rider_offset(mount_size, mounted_size);
    let rider_pos = center + rider_offset;
    let rider_aabb = ae::Aabb::new(rider_pos, mounted_size * 0.5);
    let rider_id = format!("{}:rider", authored.id);
    let rider_brain_payload = match rider_archetype {
        EnemyArchetype::PirateHeavy => crate::actor::EnemyBrain::Custom("pirate_heavy".into()),
        _ => crate::actor::EnemyBrain::Custom("pirate_raider".into()),
    };
    let mut rider_enemy = EnemyRuntime::new(
        rider_id.clone(),
        rider_variant_name.clone(),
        rider_aabb,
        rider_brain_payload,
        paths,
    );
    // Override `spawn_size` so `reset_to_spawn` restores STANDALONE
    // size (not the mounted-half size we just constructed with).
    // `EnemyRuntime::new` initializes `spawn_size = size`, but the
    // dissolve path (and reset's restore-to-standalone semantics)
    // wants the full cove-pirate footprint when the rider hits the
    // ground.
    rider_enemy.spawn_size = standalone_size;
    rider_enemy.size = mounted_size;
    // Rider HP from the composite spec's `rider_max_health`.
    if let Some(rider_hp) = composite_archetype.rider_max_health() {
        rider_enemy.health = crate::actor::Health::new(rider_hp);
    }
    rider_enemy.gravity_scale = 0.0;

    // Build the rider's MOUNTED brain: Skirmisher with the
    // composite's Bolt ranged spec + per-actor jitter so a squadron
    // of riders doesn't fire in unison. Aggressiveness forced to 1.0
    // so a PirateHeavy variant (peaceful standalone) still engages
    // while riding.
    let ranged = composite_archetype.ranged_spec();
    let bolt = match ranged {
        Some(RangedActionSpec::Bolt { speed, damage }) => Some((speed, damage)),
        _ => None,
    };
    // Per-rider jitter — same generator the solo-Skirmisher spawn
    // path uses (see `enemy_default_brain`). Without this, every
    // rider seeded with `cooldown_remaining = 0.0` ticks the same
    // dt and fires on the same beat ("all riders shoot at once").
    let seed = seed_from_id(&rider_id);
    let jitters = five_f32s_from_seed(seed);
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    let standoff_base = (composite_archetype.attack_range() * 0.35).max(120.0);
    let standoff_px = standoff_base * (0.8 + 0.4 * jitters.2);
    let orbit_phase = jitters.3 * std::f32::consts::TAU;
    let orbit_drift_rad_s = 0.4 + 0.8 * jitters.4;
    let mounted_skirmisher_cfg = SkirmisherCfg {
        aggressiveness: 1.0,
        aggro_radius: composite_archetype.aggro_radius(),
        standoff_px,
        strafe_speed: composite_archetype.chase_speed(),
        fire_cooldown_s,
        orbit_drift_rad_s,
    };
    let rider_brain = Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg: mounted_skirmisher_cfg,
        state: SkirmisherState {
            cooldown_remaining: initial_cooldown_s,
            orbit_phase,
            ..Default::default()
        },
    });
    let rider_action_set = ActionSet {
        melee: None,
        ranged: bolt.map(|(speed, damage)| RangedActionSpec::Bolt { speed, damage }),
        move_style: rider_archetype.move_style(),
        ..Default::default()
    };

    // Build mount-side bundles and reserve the entity. We need both
    // entity IDs to link MountSlot ↔ RidingOn, so we spawn each and
    // then attach the link components. The mount should keep the
    // orbiting aerial brain so the shark still changes height while
    // the rider stays visually welded to it.
    let mount_brain = skirmisher_brain_for_enemy(&mount_enemy);
    let mount_action_set = enemy_default_action_set(&mount_enemy);
    let mount_actor = ActorRuntime::Hostile(mount_enemy);
    let (m_identity, m_disposition, m_health, m_combat, m_intent, m_cooldowns) =
        actor_component_snapshot(&mount_actor);
    let mount_feature_aabb = FeatureAabb::from_aabb(mount_aabb);
    let mount_entity = commands
        .spawn((
            Name::new(format!("Feature actor mount: {mount_name}")),
            EnemyActorBundle {
                base: FeatureBaseBundle::new(&mount_id, &mount_name, mount_feature_aabb),
                identity: m_identity,
                disposition: m_disposition,
                faction: super::ActorFaction::Enemy,
                target: super::ActorTarget::default(),
                health: m_health,
                combat: m_combat,
                intent: m_intent,
                cooldowns: m_cooldowns,
            },
            mount_actor,
            mount_brain,
            mount_action_set,
            crate::brain::ActorControl::default(),
            bevy::transform::components::Transform::from_xyz(
                mount_feature_aabb.center.x,
                mount_feature_aabb.center.y,
                0.0,
            ),
            super::Mountable { rider_offset },
            super::MountSlot::default(),
        ))
        .id();

    // Rider-side bundles, with the RidingOn link pointing at the
    // mount we just spawned.
    let rider_actor = ActorRuntime::Hostile(rider_enemy);
    let (r_identity, r_disposition, r_health, r_combat, r_intent, r_cooldowns) =
        actor_component_snapshot(&rider_actor);
    let rider_feature_aabb = FeatureAabb::from_aabb(rider_aabb);
    // Cache the mounted brain on the rider so the same-room reset
    // path can restore it after a mount-death-then-reset cycle
    // (without the cache, the rider would keep their solo brain
    // after reset and the gun-sword would go silent).
    let mounted_brain_cache = super::MountedBrainCache {
        brain: rider_brain.clone(),
        action_set: rider_action_set.clone(),
    };
    let rider_entity = commands
        .spawn((
            Name::new(format!("Feature actor rider: {rider_variant_name}")),
            EnemyActorBundle {
                base: FeatureBaseBundle::new(&rider_id, &rider_variant_name, rider_feature_aabb),
                identity: r_identity,
                disposition: r_disposition,
                faction: super::ActorFaction::Enemy,
                target: super::ActorTarget::default(),
                health: r_health,
                combat: r_combat,
                intent: r_intent,
                cooldowns: r_cooldowns,
            },
            rider_actor,
            rider_brain,
            rider_action_set,
            crate::brain::ActorControl::default(),
            bevy::transform::components::Transform::from_xyz(
                rider_feature_aabb.center.x,
                rider_feature_aabb.center.y,
                0.0,
            ),
            mounted_brain_cache,
            super::Mounted,
            super::MountedSize(mounted_size),
            super::RidingOn {
                mount: mount_entity,
            },
        ))
        .id();

    // Wire MountSlot.rider on the mount so death-side dissolution
    // can reach back from mount → rider.
    commands.entity(mount_entity).insert(super::MountSlot {
        rider: Some(rider_entity),
    });
}

fn melee_brute_brain_for_enemy(enemy: &EnemyRuntime) -> crate::brain::Brain {
    use crate::brain::{Brain, MeleeBruteCfg, MeleeBruteState, StateMachineCfg};

    let archetype = enemy.archetype;
    let seed = seed_from_id(&enemy.id);
    let jitters = five_f32s_from_seed(seed);
    let aggro_radius = archetype.aggro_radius() * (0.8 + 0.4 * jitters.0);
    let chase_speed = archetype.chase_speed() * (0.85 + 0.3 * jitters.1);
    let attack_range = archetype.attack_range() * (0.9 + 0.2 * jitters.2);
    Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg {
            aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
            aggro_radius,
            attack_range,
            chase_speed,
        },
        state: MeleeBruteState::default(),
    })
}

fn skirmisher_brain_for_enemy(enemy: &EnemyRuntime) -> crate::brain::Brain {
    use crate::brain::{Brain, SkirmisherCfg, SkirmisherState, StateMachineCfg};

    let archetype = enemy.archetype;
    let seed = seed_from_id(&enemy.id);
    let jitters = five_f32s_from_seed(seed);
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    let standoff_base = (archetype.attack_range() * 0.35).max(120.0);
    let standoff_px = standoff_base * (0.8 + 0.4 * jitters.2);
    let orbit_phase = jitters.3 * std::f32::consts::TAU;
    let orbit_drift_rad_s = 0.4 + 0.8 * jitters.4;
    Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg {
            aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
            aggro_radius: archetype.aggro_radius(),
            standoff_px,
            strafe_speed: archetype.chase_speed(),
            fire_cooldown_s,
            orbit_drift_rad_s,
        },
        state: SkirmisherState {
            cooldown_remaining: initial_cooldown_s,
            orbit_phase,
            ..Default::default()
        },
    })
}

fn shark_brain_for_enemy(enemy: &EnemyRuntime) -> crate::brain::Brain {
    use crate::brain::{Brain, SharkCfg, SharkState, StateMachineCfg};

    let archetype = enemy.archetype;
    let seed = seed_from_id(&enemy.id);
    let jitters = five_f32s_from_seed(seed);
    let aggro_radius = archetype.aggro_radius() * (0.85 + 0.3 * jitters.0);
    let cruise_speed = archetype.chase_speed() * (0.85 + 0.25 * jitters.1);
    let charge_speed = (cruise_speed * (2.0 + 0.4 * jitters.2)).max(360.0);
    let bite_range = archetype.attack_range() * (0.85 + 0.15 * jitters.3);
    let charge_duration_s = 0.38 + 0.18 * jitters.4;
    let charge_cooldown_s = 0.75 + 0.55 * jitters.1;
    let standoff_px = (archetype.attack_range() * 0.40).max(140.0) * (0.8 + 0.4 * jitters.2);
    let vertical_wobble_px = (archetype.attack_range() * 0.12).max(20.0) * (0.8 + 0.4 * jitters.3);
    let orbit_drift_rad_s = 0.55 + 0.7 * jitters.4;
    Brain::StateMachine(StateMachineCfg::Shark {
        cfg: SharkCfg {
            aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
            aggro_radius,
            cruise_speed,
            charge_speed,
            bite_range,
            charge_duration_s,
            charge_cooldown_s,
            standoff_px,
            vertical_wobble_px,
            orbit_drift_rad_s,
        },
        state: SharkState {
            charge_cooldown_remaining: charge_cooldown_s * (0.25 + 0.75 * jitters.0),
            ..Default::default()
        },
    })
}

/// Map an `EnemyRuntime` to a Brain template. Reads the archetype's
/// actual tunings (chase_speed / aggro_radius / attack_range) so
/// the brain's MeleeBrute cfg matches what the existing AI loop
/// uses. PuppySlug gets a Wanderer; sandbags get StandStill;
/// sharks get a dedicated charge brain; everyone else gets a
/// MeleeBrute keyed to their archetype.
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
pub(in crate::content::features) fn enemy_default_brain(
    enemy: &EnemyRuntime,
) -> crate::brain::Brain {
    use super::super::enemies::EnemyBrainTemplate;
    use crate::brain::{
        Brain, SmashState, SniperCfg, SniperState, StateMachineCfg, WandererCfg, WandererState,
    };
    let archetype = enemy.archetype;
    match archetype.brain_template() {
        EnemyBrainTemplate::StandStill => Brain::StateMachine(StateMachineCfg::StandStill),
        EnemyBrainTemplate::Wanderer => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        }),
        EnemyBrainTemplate::MeleeBrute => melee_brute_brain_for_enemy(enemy),
        EnemyBrainTemplate::Shark => shark_brain_for_enemy(enemy),
        EnemyBrainTemplate::Skirmisher => skirmisher_brain_for_enemy(enemy),
        EnemyBrainTemplate::Sniper => {
            let seed = seed_from_id(&enemy.id);
            let jitters = five_f32s_from_seed(seed);
            let base_cooldown_s = 1.5;
            let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
            let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
            Brain::StateMachine(StateMachineCfg::Sniper {
                cfg: SniperCfg {
                    aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
                    aggro_radius: archetype.aggro_radius(),
                    fire_cooldown_s,
                },
                state: SniperState {
                    cooldown_remaining: initial_cooldown_s,
                },
            })
        }
        EnemyBrainTemplate::Smash => Brain::StateMachine(StateMachineCfg::Smash {
            cfg: smash_cfg_for_archetype(archetype),
            state: SmashState {
                rng_seed: seed_from_id(&enemy.id) as u64,
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
        let mut q = app.world_mut().query::<(&Brain, &super::MountSlot)>();
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
