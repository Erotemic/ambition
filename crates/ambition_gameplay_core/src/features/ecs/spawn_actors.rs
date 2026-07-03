//! Actor spawn helpers for ECS feature entities.
//!
//! This module covers bosses, hostile enemies, peaceful NPC actors, dynamic
//! boss minions, and encounter mobs. Static pickups/chests/breakables live in
//! `spawn_static.rs`; composite mount/rider fan-out lives in `spawn_mounts.rs`.

use super::brain_builders::{
    enemy_combat_kit_for_spec, enemy_default_action_set, enemy_default_brain,
};
use super::*;
use bevy::prelude::{Message, Name};

/// Programmatic actor-spawn request — the public seam for dropping a specific
/// actor into a live sim at an arbitrary position WITHOUT authoring an LDtk room.
///
/// Room load is the only other way an actor reaches the world, and it needs a
/// fully-built [`crate::rooms::RoomSpec`] — too heavy for scenario tests and
/// RL/agent scene setup, which both want "put this boss here, step, observe".
/// Writers emit this as a Bevy message; [`apply_spawn_actor_requests`] drains it
/// each frame and materializes the entity through the SAME `spawn_boss` /
/// `spawn_enemy` paths room load uses, so a programmatically-spawned actor is
/// indistinguishable from an authored one (it targets, ticks, takes damage, and
/// resets identically).
///
/// Today's variants cover bosses and hostile enemies — the families with a
/// trivial value-only spawn path. Peaceful NPCs need an
/// [`ambition_interaction::Interactable`] payload, so they stay room-authored
/// until a programmatic use case lands (the "add knobs when use cases land"
/// rule).
#[derive(Message, Clone, Debug)]
pub struct SpawnActorRequest {
    /// Stable feature id. Must be unique per live spawn so per-entity systems
    /// (targeting, encounter bookkeeping, save sync) don't collide on identity.
    pub id: String,
    /// Display name. For bosses this also seeds the behavior-profile lookup when
    /// the brain doesn't pin a `PhaseScript:` id — e.g. name `"mockingbird"`
    /// resolves the mockingbird profile via `canonical_boss_id_from`.
    pub name: String,
    /// World-space spawn center.
    pub pos: ae::Vec2,
    /// World-space collision HALF-extent at spawn. A boss whose profile defines
    /// `combat_size` (most do) overrides this for its combat/contact box, and an
    /// enemy archetype's `default_size` usually overrides it too — but it always
    /// seeds the kinematic body size.
    pub half_size: ae::Vec2,
    /// Faction the spawned body takes. Applies to the [`SpawnActorKind::Enemy`]
    /// path; the room-authored path uses `Enemy`. Ignored for [`SpawnActorKind::Boss`],
    /// which is always `Boss`. A spectator duel stages both fighters as plain `Npc`
    /// and lets a mutual `grudge_against` (below) — not a hostile faction — drive the
    /// fight.
    pub faction: super::ActorFaction,
    /// Feature id of another actor in the SAME spawn batch this body should hold a
    /// personal grudge against. Resolved post-spawn (once both entities exist) into
    /// an [`ActorAggression::grudge`](crate::combat::components::ActorAggression),
    /// which drives relational targeting AND authorizes same-faction damage
    /// (`damage_lands`) — the mechanism behind two `Npc` duelists feuding without a
    /// hostile faction. `None` ⇒ no grudge (fights on faction lines only).
    pub grudge_against: Option<String>,
    /// Which actor family to materialize.
    pub kind: SpawnActorKind,
}

/// The actor family a [`SpawnActorRequest`] materializes.
#[derive(Clone, Debug)]
pub enum SpawnActorKind {
    /// A boss, resolved through the same behavior-profile lookup as a room
    /// `BossSpawn`. `brain` pins the encounter (`PhaseScript { script_id }`) or
    /// falls back to the request `name` (`Dormant` / `Custom` both defer to it).
    /// `overrides` applies the spawn "tweaks Z" (hp / size / phase triggers /
    /// encounter opt-out) — see [`BossOverrides`].
    Boss {
        brain: ambition_characters::actor::BossBrain,
        overrides: BossOverrides,
    },
    /// A hostile enemy, resolved through `CharacterArchetype::from_brain` — the same
    /// path a room `EnemySpawn` takes.
    Enemy {
        brain: ambition_characters::actor::CharacterBrain,
    },
}

/// Per-spawn boss "tweaks Z" — the data that makes "spawn boss X (with tweaks Z)
/// at position Y and it just works" true (the refactor's one-line goal, R6).
///
/// Carried on the spawned boss entity as a `Component` and read at SEED time by
/// `update_boss_encounters` (hp / size / phase triggers) and by
/// `sync_boss_encounter_entities` (the encounter opt-out). `Default` = no
/// tweaks (use the archetype profile), so a room-authored boss is unaffected.
#[derive(bevy::prelude::Component, Clone, Debug, Default)]
pub struct BossOverrides {
    /// Override max HP (also the starting HP). `None` ⇒ the profile's `max_hp`.
    pub max_hp: Option<i32>,
    /// Override the combat/contact box half-extent → full size. `None` ⇒ the
    /// profile's `combat_size`.
    pub combat_size: Option<ae::Vec2>,
    /// Override the intrinsic phase triggers as DATA. `Some(vec![])` ⇒ the boss
    /// never phases up (fights to death — a boss reused as a plain tough enemy);
    /// `None` ⇒ the profile-derived triggers. Proves phases are trivially
    /// flippable data, no code change.
    pub phase_triggers: Option<Vec<crate::boss_encounter::PhaseTrigger>>,
    /// Spawn the boss WITHOUT an encounter wrapper — a plain tough enemy: no
    /// HUD, no lock-walls, no win/lose. (`sync_boss_encounter_entities` skips
    /// it.) The creature still fights + dies normally.
    pub no_encounter: bool,
}

/// Drain [`SpawnActorRequest`]s and materialize each actor.
///
/// Intentionally UNGATED by `gameplay_allowed`: programmatic scene setup (an RL
/// episode reset, a scenario-test fixture) must apply regardless of the coarse
/// `GameMode`, unlike the in-gameplay `apply_summon_effects`. The spawned
/// entity's own systems are still gameplay-gated, so an actor placed during a
/// transition just waits inert until play resumes.
pub fn apply_spawn_actor_requests(
    mut commands: bevy::prelude::Commands,
    mut requests: bevy::prelude::MessageReader<SpawnActorRequest>,
) {
    // Collect (feature id, entity, grudge-target id) for the Enemy spawns this batch
    // so a mutual grudge (a staged duel pair) can be cross-wired once both entities
    // exist — `grudge_against` names a foe by id, resolvable only after the whole
    // batch has reserved its entities.
    let mut staged: Vec<(String, bevy::prelude::Entity, Option<String>)> = Vec::new();
    for req in requests.read() {
        let aabb = ae::Aabb::new(req.pos, req.half_size);
        match &req.kind {
            SpawnActorKind::Boss { brain, overrides } => {
                let authored = crate::rooms::Authored::new(
                    req.id.clone(),
                    req.name.clone(),
                    aabb,
                    brain.clone(),
                );
                spawn_boss_with_overrides(&mut commands, &authored, overrides);
            }
            SpawnActorKind::Enemy { brain } => {
                let authored = crate::rooms::Authored::new(
                    req.id.clone(),
                    req.name.clone(),
                    aabb,
                    brain.clone(),
                );
                // Runtime spawn (outside the authored RoomSpec lists): mark it so
                // the renderer's runtime-visual discovery gives it a sprite, the
                // same as any authored enemy.
                if let Some(entity) =
                    spawn_enemy_with_faction(&mut commands, &authored, &[], req.faction)
                {
                    commands.entity(entity).insert(super::RuntimeStagedActor);
                    staged.push((req.id.clone(), entity, req.grudge_against.clone()));
                }
            }
        }
    }
    wire_staged_grudges(&mut commands, &staged);
}

/// Cross-wire mutual grudges for a freshly-staged feuding set. `staged` pairs each
/// new entity with the feature id of the foe it should grudge (from
/// [`SpawnActorRequest::grudge_against`]). Each id is resolved against the SAME batch
/// and that fighter's [`ActorAggression`](super::ActorAggression) is stamped with a
/// grudge against its rival — so two same-faction `Npc` duelists hunt AND damage each
/// other (relational targeting + the `damage_lands` override) without either being
/// re-tagged a hostile faction. An unresolved id is skipped (grudge stays `None` → the
/// actor fights on faction lines only). Re-inserting `ActorAggression` is safe: the
/// fighters spawn `hostile()` already, so this only adds the grudge.
pub(super) fn wire_staged_grudges(
    commands: &mut bevy::prelude::Commands,
    staged: &[(String, bevy::prelude::Entity, Option<String>)],
) {
    use std::collections::HashMap;
    let by_id: HashMap<&str, bevy::prelude::Entity> =
        staged.iter().map(|(id, e, _)| (id.as_str(), *e)).collect();
    for (_id, entity, foe_id) in staged {
        let Some(foe_id) = foe_id else { continue };
        let Some(&foe) = by_id.get(foe_id.as_str()) else {
            continue;
        };
        commands.entity(*entity).insert(super::ActorAggression {
            grudge: Some(foe),
            ..super::ActorAggression::hostile()
        });
    }
}

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
    enemy: super::actor_clusters::ActorClusterSeed,
    faction: super::ActorFaction,
    aggression: super::ActorAggression,
    brain: ambition_characters::brain::Brain,
    action_set: ambition_characters::brain::ActionSet,
    combat_kit: crate::combat::CombatKit,
    held_item: Option<ambition_characters::brain::HeldItemSpec>,
    /// The archetype's data-driven signature move repertoire, if any (§A1, Path B).
    moveset: Option<ambition_entity_catalog::MovesetContract>,
}

impl EnemyActorSpawnPlan {
    pub(super) fn hostile(
        entity_name: impl Into<String>,
        feature_id: impl Into<String>,
        feature_name: impl Into<String>,
        feature_aabb: CenteredAabb,
        enemy: super::actor_clusters::ActorClusterSeed,
    ) -> Self {
        let brain = enemy_default_brain(&enemy.config);
        let action_set = enemy_default_action_set(&enemy.spec);
        let combat_kit = enemy_combat_kit_for_spec(&enemy.spec);
        let held_item = super::brain_builders::held_item_for_spec(&enemy.spec);
        // The character's signature moves AND its basic melee swing are authored on
        // its archetype (data); `build_actor_moveset` folds both into ONE moveset —
        // the melee subsumption (§A1 / §3a): a plain swing is a `"attack"`-verb move
        // run by the SAME moveset runtime as the specials. Every hostile spawn path
        // (authored rooms, encounter mobs, runtime minions) carries them without a
        // per-path branch.
        let moveset = crate::combat::moveset::build_actor_moveset(
            enemy.spec.signature_move.as_ref(),
            enemy.spec.melee.as_ref(),
        );
        Self {
            entity_name: entity_name.into(),
            feature_id: feature_id.into(),
            feature_name: feature_name.into(),
            feature_aabb,
            enemy,
            faction: super::ActorFaction::Enemy,
            aggression: super::ActorAggression::hostile(),
            brain,
            action_set,
            combat_kit,
            held_item,
            moveset,
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

    pub(super) fn with_brain(mut self, brain: ambition_characters::brain::Brain) -> Self {
        self.brain = brain;
        self
    }

    pub(super) fn with_action_set(
        mut self,
        action_set: ambition_characters::brain::ActionSet,
    ) -> Self {
        self.action_set = action_set;
        self
    }

    pub(super) fn with_combat_kit(mut self, combat_kit: crate::combat::CombatKit) -> Self {
        self.combat_kit = combat_kit;
        self
    }

    pub(super) fn with_held_item(
        mut self,
        held_item: Option<ambition_characters::brain::HeldItemSpec>,
    ) -> Self {
        self.held_item = held_item;
        self
    }

    pub(super) fn without_held_item(mut self) -> Self {
        self.held_item = None;
        self
    }

    pub(super) fn spawn(self, commands: &mut Commands) -> Entity {
        let facing = self.enemy.kin.facing;
        let (identity, disposition, combat, intent, cooldowns) =
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
                    combat,
                    intent,
                    cooldowns,
                ),
                cluster_bundle,
                self.brain,
                self.action_set,
                ambition_characters::brain::ActorControl::default(),
            ))
            .id();
        if let Some(item) = self.held_item {
            commands.entity(entity).insert(super::HeldItem::new(item));
        }
        // Data-driven signature moves: the body carries its authored repertoire as
        // an `ActorMoveset`; `trigger_moveset_moves` starts a move on a control verb
        // edge through the shared moveset runtime (§A1, Path B).
        if let Some(moveset) = self.moveset {
            // A body whose moveset carries the `"attack"` verb has its basic melee
            // subsumed by the moveset: mark it so the flat-melee phases skip it and
            // its `BodyMelee` read-model is projected from the live move.
            let has_attack = moveset
                .verbs
                .contains_key(crate::combat::moveset::ATTACK_VERB);
            commands
                .entity(entity)
                .insert(crate::combat::moveset::ActorMoveset(moveset));
            if has_attack {
                commands
                    .entity(entity)
                    .insert(crate::combat::moveset::MovesetMelee);
            }
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
    /// Peaceful actors are the SAME unified cluster as enemies, built with
    /// peaceful tuning + a `Passive`/`Patrol` AI brain.
    seed: super::actor_clusters::ActorClusterSeed,
    render_size: Option<ae::Vec2>,
    interactable: ambition_interaction::Interactable,
    brain: ambition_characters::brain::Brain,
    action_set: ambition_characters::brain::ActionSet,
    combat_kit: crate::combat::CombatKit,
    aggression: super::ActorAggression,
}

impl NpcActorSpawnPlan {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn peaceful(
        entity_name: impl Into<String>,
        feature_aabb: CenteredAabb,
        id: impl Into<String>,
        name: impl Into<String>,
        spawn_aabb: ae::Aabb,
        interactable: ambition_interaction::Interactable,
        paths: &[(String, ambition_characters::actor::KinematicPath)],
    ) -> Self {
        let id = id.into();
        let name = name.into();
        let dialogue_id = match &interactable.kind {
            ambition_interaction::InteractionKind::Npc { dialogue_id, .. } => {
                dialogue_id.as_deref()
            }
            _ => None,
        };
        // The hostile archetype this actor becomes when provoked: feeds its
        // stored CombatKit (so a provoked NPC fights with the right weapon) and
        // the seed's inert reconstruction spec.
        let hostile_spec = super::actors::hostile_spec_for_actor(&id, &name, dialogue_id);
        let combat_kit = super::brain_builders::enemy_combat_kit_for_spec(&hostile_spec);
        let (seed, render_size) = super::actor_clusters::ActorClusterSeed::new_peaceful_npc(
            id.clone(),
            name.clone(),
            spawn_aabb,
            &interactable,
            paths,
        );
        let patrol_radius = match &interactable.kind {
            ambition_interaction::InteractionKind::Npc { patrol_radius, .. } => {
                patrol_radius.max(0.0)
            }
            _ => 0.0,
        };
        let brain = super::super::npcs::npc_brain_from_catalog(
            &interactable,
            seed.config.spawn.pos.x,
            patrol_radius,
            super::super::npcs::NPC_TALK_RADIUS,
            seed.motion.0.is_some(),
        );
        Self {
            entity_name: entity_name.into(),
            feature_id: id,
            feature_name: name,
            feature_aabb,
            seed,
            render_size,
            interactable,
            brain,
            // Body CAPABILITY, not AI POLICY: a peaceful NPC carries its authored
            // combat kit as its `ActionSet` (the same kit it fights with when
            // provoked), so the SAME body can throw its authored punch/swing when a
            // player DRIVES it — while its peaceful autonomous brain simply never
            // presses attack, so it still ambles harmlessly on its own. (Was
            // `ActionSet::peaceful()` — empty — which erased body capability behind
            // peaceful disposition: the "possessed peaceful NPC can't attack" bug.)
            action_set: combat_kit.to_action_set(None),
            combat_kit,
            aggression: super::ActorAggression::retaliates_when_hit(
                super::super::NPC_HOSTILE_STRIKE_THRESHOLD as u8,
            ),
        }
    }

    pub(super) fn spawn(self, commands: &mut Commands) -> Entity {
        let facing = self.seed.kin.facing;
        // Sprite-metadata render size lives on the SHARED `ActorRenderSize`
        // component so it survives a hostile flip (otherwise the body-sized
        // collision would get `collision_scale` re-applied, ballooning the sprite).
        let render_size = self.render_size;
        // Dialogue is a SHARED actor capability (`ActorInteraction`).
        let interaction = super::ActorInteraction {
            interactable: self.interactable,
            talk_radius: super::super::npcs::NPC_TALK_RADIUS,
        };
        let (identity, disposition, combat, intent, cooldowns) =
            super::actors::actor_component_snapshot(&self.seed, super::ActorDisposition::Peaceful);
        // Uniform melee subsumption (§A1/§3a): a peaceful NPC carries its combat
        // kit's melee as body CAPABILITY (for possession / provocation), so fold it
        // into a moveset `"attack"` move like every hostile — a possessed peaceful
        // NPC's swing runs through the SAME moveset runtime, not the flat path.
        let npc_moveset =
            crate::combat::moveset::build_actor_moveset(None, self.action_set.melee.as_ref());
        let cluster_bundle = self.seed.into_components();
        let mut entity = commands.spawn((
            Name::new(self.entity_name),
            EnemyActorBundle::new(
                FeatureBaseBundle::new(&self.feature_id, &self.feature_name, self.feature_aabb),
                identity,
                disposition,
                super::ActorFaction::Npc,
                ActorPose::from_parts(
                    self.feature_aabb.center,
                    self.feature_aabb.half_size,
                    facing,
                ),
                self.combat_kit,
                self.aggression,
                combat,
                intent,
                cooldowns,
            ),
            cluster_bundle,
            self.brain,
            self.action_set,
            ambition_characters::brain::ActorControl::default(),
        ));
        entity.insert(interaction);
        if let Some(moveset) = npc_moveset {
            let has_attack = moveset
                .verbs
                .contains_key(crate::combat::moveset::ATTACK_VERB);
            entity.insert(crate::combat::moveset::ActorMoveset(moveset));
            if has_attack {
                entity.insert(crate::combat::moveset::MovesetMelee);
            }
        }
        if let Some(size) = render_size {
            entity.insert(crate::features::ActorRenderSize(size));
        }
        entity.id()
    }
}

/// Spawn a boss with no spawn-time tweaks (room-load + the default seam path).
pub(super) fn spawn_boss(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ambition_characters::actor::BossBrain>,
) {
    spawn_boss_with_overrides(commands, authored, &BossOverrides::default());
}

/// The flight ceiling a boss body steers under. A boss's `BossPattern` brain
/// commands its full 2D velocity each tick (a free-mover), so the shared flight
/// limb's terminal clamp (`velocity_target / flight_speed`) must sit well above
/// any authored boss pattern speed or a telegraphed lunge would be throttled.
/// Deliberately generous — bosses author velocities in the low hundreds of px/s.
const BOSS_FLIGHT_SPEED: f32 = 1200.0;

/// Build the actor movement cluster a boss carries so its body can integrate
/// through the SHARED body pipeline like every other actor (archetype swap AS2 —
/// "a boss IS just an aerial actor"). These are exactly the components an aerial
/// enemy carries MINUS the [`BodyKinematics`] + [`ambition_characters::actor::BodyHealth`]
/// the boss already owns (§A1), so the boss's authoritative kin/HP stay the single
/// source of truth and the encounter wrapper (`BossConfig` / `BossEncounter` /
/// `BossAttackState`) layers on top unchanged.
///
/// The boss is AERIAL (a gravity-free free-mover): it spawns flight-enabled so it
/// steers through the shared flight limb (archetype swap AS4). `attacks_player` /
/// `body_contact_damage` are false — boss offense flows through `BossAttackState`
/// + `boss_attack_damage`, never the actor melee/contact path; the boss is a
/// victim-side body here (the vulnerability trio rides in via the bundle below).
fn boss_actor_cluster(
    config: &BossConfig,
    kin: &BodyKinematics,
    hp_max: i32,
) -> (
    super::actor_clusters::ActorStatus,
    super::actor_clusters::ActorConfig,
    super::actor_clusters::ActorMotionPath,
    super::super::enemies::ActorSurfaceState,
    super::super::components::BodyMelee,
    crate::actor::AncillaryMovementBundle,
    crate::combat::CombatCapabilities,
) {
    let caps = crate::combat::CombatCapabilities {
        can_fly: true,
        ..Default::default()
    };
    // Body-contact is now the SHARED `apply_actor_contact_damage` path (fable AD2):
    // drive the boss's contact tuning from its `behavior.body_damage` so a boss body
    // hazard (the Smirking Behemoth run-you-down) flows through the one contact
    // system instead of the deleted `boss_attack_damage` poll. `2.6` matches the old
    // boss body-contact push. STRIKE offense is the frame-driven Boss hitboxes
    // (`sync_boss_strike_hitboxes`), so `attacks_player` (actor melee) stays off.
    let body_damage = config.behavior.body_damage;
    let tuning = crate::combat::ActorTuning {
        max_health: hp_max,
        chase_speed: BOSS_FLIGHT_SPEED,
        max_run_speed: BOSS_FLIGHT_SPEED,
        is_aerial: true,
        // The BossPattern brain commands an exact per-tick velocity, so the flight
        // limb takes it verbatim (AS4c) — byte-identical to the old SNAP float.
        flight_direct_velocity: true,
        attacks_player: false,
        body_contact_damage: body_damage > 0,
        damage_amount: body_damage,
        contact_strength: 2.6,
        ..Default::default()
    };
    let actor_config = super::actor_clusters::ActorConfig {
        id: config.id.clone(),
        name: config.name.clone(),
        tuning,
        brain_spec: crate::combat::CharacterBrainSpec::default(),
        // The boss's REAL brain is its `BossPattern` `Brain` component. This
        // integrator-facing `CharacterBrain` only feeds patrol-stall intent, which
        // a free-flying boss never uses, so it takes the inert `Passive` row.
        brain: ambition_characters::actor::CharacterBrain::Passive,
        spawn: super::super::enemies::ActorSpawnState {
            pos: kin.pos,
            size: kin.size,
        },
        sprite_override_npc_name: None,
        sprite_character_id: None,
    };
    (
        super::actor_clusters::ActorStatus {
            respawn_timer: 0.0,
            ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
        },
        actor_config,
        super::actor_clusters::ActorMotionPath::default(),
        super::super::enemies::ActorSurfaceState {
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale: 0.0,
        },
        super::super::components::BodyMelee::default(),
        crate::actor::AncillaryMovementBundle::from_scratch(
            super::actor_clusters::ActorBody::from_caps(&caps, true).0,
        ),
        caps,
    )
}

/// Spawn a boss applying the per-spawn "tweaks Z" ([`BossOverrides`]). The
/// overrides are attached as a component and applied at SEED time by
/// `update_boss_encounters` (so the profile-application there can't clobber
/// them); the encounter opt-out is honored by `sync_boss_encounter_entities`.
pub(super) fn spawn_boss_with_overrides(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ambition_characters::actor::BossBrain>,
    overrides: &BossOverrides,
) {
    let mut boss = BossClusterScratch::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
    );
    // Apply a combat-size override to the initial scratch so the first-frame
    // AABB/render size are right; `update_boss_encounters` re-applies it at seed
    // (after the profile application that would otherwise overwrite it).
    if let Some(size) = overrides.combat_size {
        boss.config.behavior.combat_size = Some(size);
        // AS4b: `kin.size` IS the collision envelope, so keep it in lock-step with an
        // overridden combat size (the render basis stays in `status.render_size`).
        boss.kin.size = size;
    }
    bevy::log::info!(
        target: "ambition::boss_spawn",
        "spawn_boss id={} name={:?} brain={:?} → behavior.id={} combat_size={:?}",
        boss.config.id,
        boss.config.name,
        authored.payload,
        boss.config.behavior.id,
        boss.as_ref().combat_size(),
    );
    let initial_phase = BossPhase::from_alive(boss.health.alive());
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
    let brain_cfg = ambition_characters::brain::BossPatternCfg {
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
    // Authored special repertoire as body CAPABILITY (persists across a brain
    // swap): both the autonomous pattern and a possessing human drive these same
    // profiles. Derived before `brain_cfg` is moved into the brain.
    let boss_capability = ambition_characters::brain::BossCapability::from_cfg(&brain_cfg);
    let brain = ambition_characters::brain::Brain::StateMachine(
        ambition_characters::brain::StateMachineCfg::BossPattern {
            cfg: brain_cfg,
            state: ambition_characters::brain::BossPatternState::default(),
        },
    );
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
    let boss_action_set = ambition_characters::brain::ActionSet {
        ranged: Some(ambition_characters::brain::RangedActionSpec::Bolt {
            speed: 380.0,
            damage: 1,
        }),
        special: None,
        move_style: ambition_characters::brain::MoveStyleSpec::Walk,
        ..Default::default()
    };
    let boss_combat_kit = CombatKit::from_action_set(&boss_action_set);
    // §A1: the boss's `BodyHealth` HP authority spawns from the scratch
    // (`into_components` below); the snapshot builds only the read-models.
    let (boss_identity, boss_disposition, boss_combat, boss_intent, boss_cooldowns) =
        boss_component_snapshot(
            boss.as_ref(),
            &ambition_characters::brain::BossAttackState::default(),
            &boss.health,
            &ambition_characters::actor::BodyCombat::default(),
        );
    let boss_facing = boss.kin.facing;
    // Archetype swap AS2: the boss carries the same aerial actor movement cluster
    // every other actor does (built here BEFORE the scratch is consumed), so the
    // shared body pipeline can integrate it (AS4). Kin/HP are NOT in this bundle —
    // the boss owns those directly (§A1).
    let boss_actor_cluster = boss_actor_cluster(&boss.config, &boss.kin, boss.health.max());
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
        boss_combat,
        boss_intent,
        boss_cooldowns,
        boss_combat_kit,
        ActorAggression::hostile(),
    ));
    // Data-driven special MOVESET: the boss's content-technique specials run through
    // the SHARED moveset runtime (a sustain-move per key) instead of the boss-only
    // `dispatch_boss_special`, so the boss's special path is the actor's (§A1). Built
    // from the capability repertoire before it's moved into the brain bundle.
    let boss_special_moves = crate::features::boss_special_moveset(&boss_capability);
    entity.insert((
        // The brain bundle stays grouped because each piece is required
        // for the boss tick chain.
        brain,
        boss_action_set,
        ambition_characters::brain::ActorControl::default(),
        ambition_characters::brain::BossAttackState::default(),
        boss_capability,
    ));
    if let Some(moveset) = boss_special_moves {
        entity.insert(moveset);
    }
    // Archetype swap AS2: the aerial actor movement cluster (18 ancillary body
    // clusters + status/config/surface/melee/caps). This is what lets the boss
    // integrate through the shared body pipeline (AS4) instead of its bespoke
    // float. It ALSO supplies the victim-side vulnerability trio (`BodyOffense` /
    // `BodyDodgeState` / `BodyShieldState`) the boss used to carry standalone
    // (§A1 slice 3a) — so `apply_hitbox_damage`'s non-`Option` victim query still
    // matches, now via the one bundle every body shares.
    entity.insert(boss_actor_cluster);
    // Per-spawn tweaks Z: read at seed time by `update_boss_encounters`
    // (hp / size / phase triggers) + `sync_boss_encounter_entities`
    // (encounter opt-out). Default for room-authored bosses ⇒ no-op.
    entity.insert(overrides.clone());
    // Per-boss special-technique state (apple-rain accumulator, overfit-volley
    // samples, pit/cross/cascade gates, eye-beam lock) is now content-owned
    // (`ambition_content::bosses::specials`), attached to every boss via
    // `register_required_components::<BossConfig, _>()` in the content plugin —
    // the engine spawn names no boss special.
}
/// Runtime minion spawner — used by boss EFFECTS consumers (e.g.
/// PitTrap puppy_slug spawn, MinionCascade slop adds). Mirrors
/// `spawn_encounter_mob` but takes plain values from a Bevy system
/// so callers don't have to wrap them in an `Authored<CharacterBrain>`.
/// The resulting entity carries the same component set as authored
/// encounter mobs — crucially including the `EncounterMob` marker
/// so `spawn_dynamic_feature_visuals` picks it up next frame and
/// attaches the right sprite. Without that marker the minion would
/// spawn invisibly (ECS-only).
///
/// `archetype_id` matches one of the strings in `BRAIN_NAME_TO_ARCHETYPE`
/// (`"puppy_slug"`, `"small_lurker"`, …); unknown strings fall back
/// to `Combatant` via `CharacterArchetype::from_brain`. `half_size` is
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
    let brain = ambition_characters::actor::CharacterBrain::Custom(archetype_id.into());
    let mut enemy =
        super::actor_clusters::ActorClusterSeed::new(id.clone(), name.clone(), aabb, brain, &[]);
    // `ActorClusterSeed::new` already sets HP from the resolved spec.
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
    if let Some(rs) =
        super::actor_clusters::sprite_render_size_for_name(&name, aabb.half_size() * 2.0)
    {
        commands
            .entity(entity)
            .insert(crate::features::ActorRenderSize(rs));
    }
    entity
}

pub(super) fn spawn_enemy(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ambition_characters::actor::CharacterBrain>,
    paths: &[(String, ambition_characters::actor::KinematicPath)],
) {
    let _ = spawn_enemy_with_faction(commands, authored, paths, super::ActorFaction::Enemy);
}

/// Like [`spawn_enemy`] but the spawned body takes `faction` (the duel/arena path
/// puts its two fighters on DIFFERENT factions so they can damage each other under
/// the physical damage rule). Composite mounts ignore the override (they fan out
/// their own factions); the duel fighters are solo. Returns the spawned solo
/// body's entity so a caller (the duel staging) can attach extra markers; `None`
/// for the composite mount/rider path (it fans out two of its own entities).
pub(super) fn spawn_enemy_with_faction(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ambition_characters::actor::CharacterBrain>,
    paths: &[(String, ambition_characters::actor::KinematicPath)],
    faction: super::ActorFaction,
) -> Option<bevy::ecs::entity::Entity> {
    let spec = super::super::enemies::spec_for_brain(&authored.payload);
    if spec.is_composite() {
        super::spawn_mounts::spawn_composite_mount_rider(commands, authored, paths, &spec);
        return None;
    }
    let enemy = super::actor_clusters::ActorClusterSeed::new(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    Some(spawn_solo_enemy(commands, enemy, authored, faction))
}

/// Single-entity hostile spawn — the common path after composite
/// mount/rider fan-out has been handled. Returns the spawned body entity.
pub(super) fn spawn_solo_enemy(
    commands: &mut Commands,
    enemy: super::actor_clusters::ActorClusterSeed,
    authored: &crate::rooms::Authored<ambition_characters::actor::CharacterBrain>,
    faction: super::ActorFaction,
) -> bevy::ecs::entity::Entity {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let entity = EnemyActorSpawnPlan::hostile(
        format!("Feature actor enemy: {}", authored.name),
        authored.id.clone(),
        authored.name.clone(),
        feature_aabb,
        enemy,
    )
    .with_faction(faction)
    .spawn(commands);
    // A named catalog character carries its authored sprite render size on the
    // shared `ActorRenderSize` (the same component the peaceful-NPC path sets), so
    // the sprite draws at the authored scale and matches the body the per-frame
    // `CenteredAabb` sync derives from the sprite-sized collision.
    if let Some(rs) = super::actor_clusters::sprite_render_size_for_name(
        &authored.name,
        authored.aabb.half_size() * 2.0,
    ) {
        commands
            .entity(entity)
            .insert(crate::features::ActorRenderSize(rs));
    }
    entity
}
pub(super) fn spawn_interactable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<ambition_interaction::Interactable>,
    paths: &[(String, ambition_characters::actor::KinematicPath)],
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let interactable = &authored.payload;
    if matches!(
        interactable.kind,
        ambition_interaction::InteractionKind::Npc { .. }
    ) {
        NpcActorSpawnPlan::peaceful(
            format!("Feature actor npc: {}", authored.name),
            feature_aabb,
            authored.id.clone(),
            authored.name.clone(),
            authored.aabb,
            interactable.clone(),
            paths,
        )
        .spawn(commands);
    } else if let ambition_interaction::InteractionKind::Custom(payload) = &interactable.kind {
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
    brain: ambition_characters::actor::CharacterBrain,
    pos: ae::Vec2,
    size: ae::Vec2,
) {
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy =
        super::actor_clusters::ActorClusterSeed::new(id.clone(), id.clone(), aabb, brain, &[]);
    // `ActorClusterSeed::new` already sets HP from the resolved spec.
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
    if let Some(rs) = super::actor_clusters::sprite_render_size_for_name(&id, size * 0.5 * 2.0) {
        commands
            .entity(entity)
            .insert(crate::features::ActorRenderSize(rs));
    }
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub(super) fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(Entity, &EncounterMob, &FeatureId, &BodyCombat)>,
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
    mut requests: bevy::prelude::MessageReader<ambition_vfx::EffectRequest>,
) {
    for req in requests.read() {
        if let ambition_vfx::Effect::Summon(s) = &req.effect {
            spawn_runtime_minion(
                &mut commands,
                s.id.clone(),
                s.name.clone(),
                s.pos,
                s.half_size,
                &s.archetype_id,
                s.encounter_id.clone(),
                s.faction,
                super::ActorAggression::hostile(),
            );
        }
    }
}
