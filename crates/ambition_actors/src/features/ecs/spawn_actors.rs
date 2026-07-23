//! Actor spawn helpers for ECS feature entities.
//!
//! This module covers bosses, hostile enemies, peaceful NPC actors, dynamic
//! boss minions, and encounter mobs. Static pickups/chests/breakables live in
//! `spawn_static.rs`; composite mount/rider fan-out lives in `spawn_mounts.rs`.

use super::super::enemies::CharacterRoster;
use super::brain_builders::{
    enemy_combat_kit_for_spec, enemy_default_action_set, enemy_default_brain,
};
use super::*;
use crate::boss_encounter::BossCatalog;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
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
        brain: ambition_entity_catalog::placements::BossBrain,
        overrides: BossOverrides,
    },
    /// A hostile enemy, resolved through `spec_for_brain` (brain key →
    /// `CharacterArchetypeSpec`) — the same path a room `EnemySpawn` takes.
    Enemy {
        brain: ambition_entity_catalog::placements::CharacterBrain,
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
    character_catalog: bevy::prelude::Res<CharacterCatalog>,
    character_roster: bevy::prelude::Res<CharacterRoster>,
    boss_catalog: bevy::prelude::Res<BossCatalog>,
    active_session: Option<bevy::prelude::Res<ActiveSessionScope>>,
) {
    // Collect (feature id, entity, grudge-target id) for the Enemy spawns this batch
    // so a mutual grudge (a staged duel pair) can be cross-wired once both entities
    // exist — `grudge_against` names a foe by id, resolvable only after the whole
    // batch has reserved its entities.
    let mut staged: Vec<(String, bevy::prelude::Entity, Option<String>)> = Vec::new();
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        requests.clear();
        return;
    };
    for req in requests.read() {
        let entity = spawn_staged_actor(
            &mut commands,
            &character_catalog,
            &character_roster,
            &boss_catalog,
            session_scope,
            req,
        );
        if matches!(req.kind, SpawnActorKind::Enemy { .. }) {
            staged.push((req.id.clone(), entity, req.grudge_against.clone()));
        }
    }
    wire_staged_grudges(&mut commands, &staged);
}

/// Materialize one staged actor.
///
/// **The one staged-actor constructor.** Both the message-driven applier above
/// and the `ambition.staged-actor` construction recipe
/// ([`crate::construction`]) call it, so a room built through the planner and a
/// programmatic scene-setup request cannot drift into different entity shapes.
pub(crate) fn spawn_staged_actor(
    commands: &mut Commands,
    character_catalog: &CharacterCatalog,
    character_roster: &CharacterRoster,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    req: &SpawnActorRequest,
) -> bevy::ecs::entity::Entity {
    let root = commands.spawn_empty().id();
    spawn_staged_actor_into(
        commands,
        character_catalog,
        character_roster,
        boss_catalog,
        session_scope,
        root,
        req,
    );
    root
}

/// Populate a staged actor onto a root the construction executor allocated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_staged_actor_into(
    commands: &mut Commands,
    character_catalog: &CharacterCatalog,
    character_roster: &CharacterRoster,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    req: &SpawnActorRequest,
) {
    let aabb = ae::Aabb::new(req.pos, req.half_size);
    match &req.kind {
        SpawnActorKind::Boss { brain, overrides } => {
            let authored =
                crate::rooms::Authored::new(req.id.clone(), req.name.clone(), aabb, brain.clone());
            spawn_boss_with_overrides_into(
                commands,
                boss_catalog,
                session_scope,
                root,
                &authored,
                overrides,
            );
        }
        SpawnActorKind::Enemy { brain } => {
            let authored =
                crate::rooms::Authored::new(req.id.clone(), req.name.clone(), aabb, brain.clone());
            // Staged outside the authored RoomSpec lists: mark it so the
            // renderer's runtime-visual discovery gives it a sprite, the same as
            // any authored enemy.
            spawn_enemy_with_faction_into(
                commands,
                character_catalog,
                character_roster,
                session_scope,
                root,
                &authored,
                &[],
                req.faction,
            );
            commands.entity(root).insert(super::RuntimeStagedActor);
        }
    }
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
        // The character's signature moves AND its basic melee/ranged are authored on
        // its archetype (data); `build_actor_moveset` folds them into ONE moveset —
        // the melee subsumption (§A1 / §3a): a plain swing is a `"attack"`-verb move
        // run by the SAME moveset runtime as the specials. Every hostile spawn path
        // (authored rooms, encounter mobs, runtime minions) carries them without a
        // per-path branch.
        //
        // The melee/ranged SOURCE is the resolved `action_set` (kit + held item),
        // the SAME capability the brain's `emit_brain_action_messages` gate reads —
        // NOT the raw `spec.melee`. Now that the flat melee driver is gone, a body
        // that can emit a melee (its `ActionSet.melee` is `Some`, e.g. granted by a
        // held weapon) MUST have a moveset `"attack"` move, or it could never swing.
        // Building from `action_set` closes that gap definitionally: capability and
        // moveset share one source.
        let moveset = crate::combat::moveset::build_actor_moveset(
            enemy.spec.signature_move.as_ref(),
            action_set.melee.as_ref(),
            action_set.ranged.as_ref(),
            // Enemy/boss specials are AUTHORED in the archetype `signature_move`
            // (real hitboxes/timelines); the `ActionSet.special` marker only
            // mirrors it, so do NOT re-fold it here or it would clobber the
            // authored move with the generic capability shell.
            None,
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

    /// Spawn onto a freshly allocated entity. Kept for the room loops that have
    /// not moved onto the construction planner yet.
    pub(super) fn spawn(self, commands: &mut Commands, session_scope: SessionSpawnScope) -> Entity {
        let root = commands.spawn_empty().id();
        self.spawn_into(commands, session_scope, root);
        root
    }

    /// Populate a root someone else allocated — the shape the construction
    /// executor needs, since it owns authoritative-root allocation.
    pub(super) fn spawn_into(
        self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        entity: Entity,
    ) {
        let facing = self.enemy.kin.facing;
        let motion_model = self.enemy.config.tuning.motion_model();
        let (identity, disposition, combat, intent, cooldowns) =
            enemy_component_snapshot(&self.enemy);
        let cluster_bundle = self.enemy.into_components();
        let entity = commands
            .insert_session_scoped(
                session_scope,
                entity,
                (
                    Name::new(self.entity_name),
                    EnemyActorBundle::new(
                        FeatureBaseBundle::new(
                            &self.feature_id,
                            &self.feature_name,
                            self.feature_aabb,
                        ),
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
                    )
                    .with_motion_model(motion_model),
                    cluster_bundle,
                    self.brain,
                    self.action_set,
                    ambition_characters::brain::ActorControl::default(),
                ),
            )
            .id();
        if let Some(item) = self.held_item {
            commands.entity(entity).insert(super::HeldItem::new(item));
        }
        // Data-driven signature moves: the body carries its authored repertoire as
        // an `ActorMoveset`; `trigger_moveset_moves` starts a move on a control verb
        // edge through the shared moveset runtime (§A1, Path B).
        if let Some(moveset) = self.moveset {
            // A body whose moveset carries the `"attack"` verb melees through the
            // moveset (the only melee path): mark it `MovesetMelee` so its
            // `BodyMelee` read-model is projected from the live move.
            let has_attack = moveset
                .verbs
                .contains_key(crate::combat::moveset::ATTACK_VERB);
            // Likewise a body whose moveset carries the `"ranged"` verb has its shot
            // subsumed: mark it so the flat `frame.fire → Ranged` emission is skipped
            // (the move's fire event spawns the shot instead — no double-fire).
            let has_ranged = moveset
                .verbs
                .contains_key(crate::combat::moveset::RANGED_VERB);
            commands
                .entity(entity)
                .insert(crate::combat::moveset::ActorMoveset(moveset));
            if has_attack {
                commands
                    .entity(entity)
                    .insert(crate::combat::moveset::MovesetMelee);
            }
            if has_ranged {
                commands
                    .entity(entity)
                    .insert(ambition_characters::brain::MovesetRanged);
            }
        }
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
    /// The explicit brain binding (default preset + current selection) and the
    /// authored build context (patrol home + radius) for a catalog-backed NPC.
    /// `None` for anonymous NPCs (no catalog identity). When present, both are
    /// attached so the actor's brain can be switched at runtime (`BrainCommand`),
    /// rebuilt around its authored home (`RestoreDefault`), and its selection +
    /// context survive snapshot/restore.
    brain_binding: Option<(
        ambition_characters::actor::character_catalog::BrainBinding,
        ambition_characters::actor::character_catalog::AuthoredBrainContext,
    )>,
    action_set: ambition_characters::brain::ActionSet,
    combat_kit: crate::combat::CombatKit,
    aggression: super::ActorAggression,
}

impl NpcActorSpawnPlan {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn peaceful(
        catalog: &CharacterCatalog,
        roster: &CharacterRoster,
        entity_name: impl Into<String>,
        feature_aabb: CenteredAabb,
        id: impl Into<String>,
        name: impl Into<String>,
        spawn_aabb: ae::Aabb,
        interactable: ambition_interaction::Interactable,
        paths: &[(String, ambition_engine_core::KinematicPath)],
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
        // An NPC is by construction a UNIQUE named placement: its death is
        // permanent (ADR 0022 "Morrowind rules") regardless of the mob-tier
        // respawn policy the borrowed combat archetype authors. The policy is a
        // property of the PLACEMENT, and this placement is a person.
        //
        // That pin lives on the SEED's tuning (`ActorClusterSeed::new_peaceful_npc_in`)
        // and survives provocation via `ActorTuning::adopting_archetype`. It used
        // to be written onto this local `hostile_spec` instead, whose only
        // consumer is the combat-kit builder below — which never reads `respawn`.
        // So the pin was a no-op, provocation replaced the policy with the
        // borrowed archetype's `OnRoomReenter`, and killed NPCs came back.
        let hostile_spec = super::actors::hostile_spec_for_actor(roster, &id, &name, dialogue_id);
        let combat_kit = super::brain_builders::enemy_combat_kit_for_spec(&hostile_spec);
        let (mut seed, render_size) = super::actor_clusters::ActorClusterSeed::new_peaceful_npc_in(
            catalog,
            roster,
            id.clone(),
            name.clone(),
            spawn_aabb,
            &interactable,
            paths,
        );
        // Explicit brain authority: the placement's `brain_override` (else the
        // character's catalog `default_brain`) selects the brain; `patrol_radius`
        // only PARAMETERIZES a selected patrol preset. No radius/motion/hostility
        // inference. A catalog-backed NPC also gets a `BrainBinding` +
        // `AuthoredBrainContext` so its brain can be switched at runtime, rebuilt
        // around its authored home, and its selection survives snapshot.
        let (brain, brain_binding) =
            super::super::npcs::resolve_npc_brain(catalog, &interactable, seed.config.spawn.pos.x);
        // Derive the `CharacterBrain` read-model (patrol-stall intent) from the
        // RESOLVED autonomous brain, not from `patrol_radius`: a body patrol-stalls
        // iff its actual brain is a Patrol brain. Any other resolved brain (wanderer,
        // stand_still, hostile default) is `Passive` — a wanderer reverses at walls
        // through the integrator's own wall-stop, not this read-model.
        seed.config.brain = if matches!(
            brain,
            ambition_characters::brain::Brain::StateMachine(
                ambition_characters::brain::StateMachineCfg::Patrol { .. }
            )
        ) {
            let path_id = match &interactable.kind {
                ambition_interaction::InteractionKind::Npc { patrol_path_id, .. } => {
                    patrol_path_id.clone()
                }
                _ => None,
            };
            ambition_entity_catalog::placements::CharacterBrain::Patrol { path_id }
        } else {
            ambition_entity_catalog::placements::CharacterBrain::Passive
        };
        Self {
            entity_name: entity_name.into(),
            feature_id: id,
            feature_name: name,
            feature_aabb,
            seed,
            render_size,
            interactable,
            brain,
            brain_binding,
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

    pub(super) fn spawn(self, commands: &mut Commands, session_scope: SessionSpawnScope) -> Entity {
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
        let npc_moveset = crate::combat::moveset::build_actor_moveset(
            None,
            self.action_set.melee.as_ref(),
            self.action_set.ranged.as_ref(),
            // Peaceful NPC specials, like hostiles, are archetype-authored; the
            // marker is not re-folded (see the hostile path).
            None,
        );
        let motion_model = self.seed.config.tuning.motion_model();
        let cluster_bundle = self.seed.into_components();
        let mut entity = commands.spawn_session_scoped(
            session_scope,
            (
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
                )
                .with_motion_model(motion_model),
                cluster_bundle,
                self.brain,
                self.action_set,
                ambition_characters::brain::ActorControl::default(),
            ),
        );
        entity.insert(interaction);
        // The explicit brain binding + authored context travel with the actor so
        // runtime brain switches (`BrainCommand`), authored-home rebuilds
        // (`RestoreDefault`), and snapshot/restore all read the same authoritative
        // state. Anonymous NPCs (no catalog identity) carry neither.
        if let Some((binding, authored_context)) = self.brain_binding {
            // The autonomous body also carries its temporary-control state (starts
            // `Autonomous`): possession / mount record their controller here by
            // stable id, so a snapshot restores the control mode across a rewind.
            entity.insert((
                binding,
                authored_context,
                crate::features::TemporaryControl::Autonomous,
            ));
        }
        if let Some(moveset) = npc_moveset {
            let has_attack = moveset
                .verbs
                .contains_key(crate::combat::moveset::ATTACK_VERB);
            let has_ranged = moveset
                .verbs
                .contains_key(crate::combat::moveset::RANGED_VERB);
            entity.insert(crate::combat::moveset::ActorMoveset(moveset));
            if has_attack {
                entity.insert(crate::combat::moveset::MovesetMelee);
            }
            if has_ranged {
                entity.insert(ambition_characters::brain::MovesetRanged);
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
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
) {
    spawn_boss_with_overrides(
        commands,
        boss_catalog,
        session_scope,
        authored,
        &BossOverrides::default(),
    );
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
    crate::features::MotionModel,
    crate::combat::CombatCapabilities,
    crate::combat::CombatTuning,
) {
    // A boss floats: its movement kit grants flight, unioned into the body's
    // `AbilitySet` below. Death/weapon consequence traits stay default.
    let caps = crate::combat::CombatCapabilities::default();
    let movement_kit = ae::AbilitySet {
        fly: true,
        ..ae::AbilitySet::NONE
    };
    // Body-contact is now the SHARED `apply_actor_contact_damage` path (fable AD2):
    // drive the boss's contact tuning from its `behavior.body_damage` so a boss body
    // hazard (the Smirking Behemoth run-you-down) flows through the one contact
    // system instead of the deleted `boss_attack_damage` poll. `2.6` matches the old
    // boss body-contact push. STRIKE offense is the frame-driven Boss hitboxes
    // (`sync_boss_strike_hitboxes`), so `attacks_player` (actor melee) stays off.
    let body_damage = config.behavior.body_damage;
    let tuning = crate::features::ecs::actor_tuning::ActorTuning {
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
    let weight = tuning.weight;
    let actor_config = super::actor_clusters::ActorConfig {
        id: config.id.clone(),
        name: config.name.clone(),
        tuning,
        brain_spec: crate::features::ecs::actor_tuning::CharacterBrainSpec::default(),
        // The boss's REAL brain is its `BossPattern` `Brain` component. This
        // integrator-facing `CharacterBrain` only feeds patrol-stall intent, which
        // a free-flying boss never uses, so it takes the inert `Passive` row.
        brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
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
            super::actor_clusters::ActorBody::from_kit(movement_kit, true).0,
        ),
        // Every integrated body carries an explicit policy from spawn — the
        // boss is axis-swept (its direct-velocity flight rides the per-tick
        // axis-parameter refresh in `integrate_body`).
        crate::features::MotionModel::default(),
        caps,
        // Project the boss's weight onto the combat-owned carrier at spawn
        // (E2 verdict b); default `1.0` here since bosses don't author weight.
        crate::combat::CombatTuning {
            weight,
            // Bosses pace strikes via their move scripts, and carry no sprite
            // catalog id (their strike volumes are frame-authored).
            attack_cooldown_mult: 1.0,
            sprite_character_id: None,
        },
    )
}

/// Spawn a boss applying the per-spawn "tweaks Z" ([`BossOverrides`]). The
/// overrides are attached as a component and applied at SEED time by
/// `update_boss_encounters` (so the profile-application there can't clobber
/// them); the encounter opt-out is honored by `sync_boss_encounter_entities`.
pub(super) fn spawn_boss_with_overrides(
    commands: &mut Commands,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
    overrides: &BossOverrides,
) -> bevy::ecs::entity::Entity {
    let root = commands.spawn_empty().id();
    spawn_boss_with_overrides_into(
        commands,
        boss_catalog,
        session_scope,
        root,
        authored,
        overrides,
    );
    root
}

/// Populate a boss onto a root the construction executor allocated.
pub(super) fn spawn_boss_with_overrides_into(
    commands: &mut Commands,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
    overrides: &BossOverrides,
) {
    let mut boss = BossClusterScratch::new(
        boss_catalog,
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
    let boss_sheet_key = encounter_id.to_ascii_lowercase().replace('-', "_");
    let boss_anim_frame = crate::boss_encounter::sprites::BossAnimFrame::new(
        boss_catalog.sheet_for_key(&boss_sheet_key),
    );
    let combat_tuning = crate::time::feel::SandboxFeelTuning::default().feature_combat_tuning();
    let cycle_attack_active = boss
        .config
        .behavior
        .attack_active
        .max(combat_tuning.boss_attack_active)
        .max(0.01);
    // A self-dodging boss side-steps during its strike window (GNU-ton weaves
    // out of its own apple rain); the amplitude/frequency are authored boss
    // DATA (`self_dodge` in `boss_profiles.ron`), so the engine names no boss.
    let (self_dodge_amp, self_dodge_freq) = boss.config.behavior.self_dodge.unwrap_or((0.0, 0.0));
    let brain_cfg = ambition_characters::brain::BossPatternCfg {
        aggressiveness: 1.0,
        encounter_id: encounter_id.clone(),
        pattern: boss.config.behavior.attack_pattern.clone(),
        movement: boss.config.behavior.movement.clone(),
        movement_phase2: boss.config.behavior.movement_phase2.clone(),
        movement_enrage: boss.config.behavior.movement_enrage.clone(),
        spawn: boss.config.spawn,
        combat_size: boss.as_ref().combat_size(),
        cycle_attack_windup: boss.config.behavior.attack_windup.max(0.01),
        cycle_attack_active,
        cycle_attack_cooldown: boss.config.behavior.attack_cooldown.max(0.05),
        cycle_attacks: boss.config.behavior.attacks.clone(),
        self_dodge_amp,
        self_dodge_freq,
        macro_tuning: boss.config.behavior.macro_tuning,
    };
    // Authored special repertoire as body CAPABILITY (persists across a brain
    // swap): both the autonomous pattern and a possessing human drive these same
    // profiles. Derived before `brain_cfg` is moved into the brain.
    let boss_capability = ambition_characters::brain::BossCapability::from_cfg(&brain_cfg);
    // First-seen telegraph window per profile — lets each strike move span the whole
    // telegraph→strike as one timeline (E53). Derived before `brain_cfg` is moved.
    let boss_telegraph_windows = brain_cfg.telegraph_windows();
    // Captured before the scratch is consumed (`into_components` below), for the
    // boss attack moveset: each strike profile → a geometry / special move.
    let boss_attack_behavior = boss.config.behavior.clone();
    let boss_attack_combat_size = boss.as_ref().combat_size();
    let brain = ambition_characters::brain::Brain::StateMachine(
        ambition_characters::brain::StateMachineCfg::BossPattern {
            cfg: brain_cfg,
            state: ambition_characters::brain::BossPatternState::default(),
        },
    );
    // Bosses keep the ordinary ranged baseline, but their profile-driven
    // strikes and content techniques live in the per-profile ActorMoveset built
    // below. The boss brain publishes BossAttackIntent directly, so the generic
    // one-slot ActionSet special route stays empty and cannot double-trigger it.
    let _ = encounter_id; // resolved upstream via `boss.behavior`
    let boss_action_set = ambition_characters::brain::ActionSet {
        ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(380.0, 1)),
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
    // The boss's coarse render/composite footprint (R1.1 envelope split): the
    // body-generic `BodyEnvelope` the ONE shared integrator publishes the
    // `CenteredAabb` from, so the boss no longer needs a bespoke render-sized
    // publish. Captured before `into_components` consumes the scratch.
    let boss_render_envelope = crate::combat::BodyEnvelope(boss.as_ref().render_size());
    let boss_components = boss.into_components();
    let mut entity = commands.insert_session_scoped(
        session_scope,
        root,
        (
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
            boss_anim_frame,
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
        ),
    );
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
    // Data-driven attack MOVESET: EVERY boss strike — geometry AND content-technique
    // special — runs through the SHARED moveset runtime (one move per profile), so the
    // boss's melee/special path is the actor's, retiring both `sync_boss_strike_hitboxes`
    // and `dispatch_boss_special` (§A1). Built from the capability repertoire.
    let boss_attack_moves = crate::features::boss_attack_moveset(
        &boss_capability,
        &boss_attack_behavior,
        boss_attack_combat_size,
        &boss_telegraph_windows,
    );
    entity.insert((
        // The brain bundle stays grouped because each piece is required
        // for the boss tick chain.
        brain,
        boss_action_set,
        ambition_characters::brain::ActorControl::default(),
        ambition_characters::brain::BossAttackState::default(),
        // §A1 intent/projection split: the driver-written fire INTENT the moveset
        // trigger reads (BossAttackState is now the projected read-model).
        ambition_characters::brain::BossAttackIntent::default(),
        boss_capability,
    ));
    if let Some(moveset) = boss_attack_moves {
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
    // The coarse render footprint the shared integrator publishes the CenteredAabb
    // from (R1.1). Required by `integrate_boss_bodies`' query, so a boss without it
    // simply would not move — a loud failure the boss suites catch, not a silent
    // footprint shrink.
    entity.insert(boss_render_envelope);
    // Per-spawn tweaks Z: read at seed time by `update_boss_encounters`
    // (hp / size / phase triggers) + `sync_boss_encounter_entities`
    // (encounter opt-out). Default for room-authored bosses ⇒ no-op.
    entity.insert(overrides.clone());
    // ADR 0020: a boss authored as a would-be RIDER (non-empty
    // `pilotable_mount_classes`) becomes a `CanPilot` — the SAME mount-role tag
    // the enemy path attaches in `attach_mount_role`, so `spawn_boss` and
    // `spawn_solo_enemy` stay symmetric (a boss can board a `giant_gnu` mount).
    // `boss_attack_behavior` is a pre-`into_components` clone, still live here.
    // The `RidingOn`/`MountSlot` link is installed later by
    // `resolve_pending_mount_links` from the room's authored `mounted_on` refs.
    if !boss_attack_behavior.pilotable_mount_classes.is_empty() {
        entity.insert(super::CanPilot {
            classes: boss_attack_behavior
                .pilotable_mount_classes
                .iter()
                .map(|c| super::MountClass(c.clone()))
                .collect(),
        });
    }
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
/// to `Combatant` via `spec_for_brain`. `half_size` is
/// the spawn AABB half-extent (the archetype spec's `default_size`
/// usually overrides this anyway). `id` should be unique per spawn
/// so per-entity systems don't collide on identity. `encounter_id`
/// scopes the minion to a parent encounter so room reset / boss
/// despawn cleans it up alongside the boss.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_runtime_minion(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
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
    let root = commands.spawn_empty().id();
    spawn_runtime_minion_into(
        commands,
        catalog,
        roster,
        session_scope,
        root,
        id,
        name,
        world_pos,
        half_size,
        archetype_id,
        encounter_id,
        faction,
        aggression,
    );
    root
}

/// Populate a summoned minion onto a root the construction executor allocated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_runtime_minion_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    entity: bevy::ecs::entity::Entity,
    id: impl Into<String>,
    name: impl Into<String>,
    world_pos: ae::Vec2,
    half_size: ae::Vec2,
    archetype_id: &str,
    encounter_id: impl Into<String>,
    faction: super::ActorFaction,
    aggression: super::ActorAggression,
) {
    let id = id.into();
    let name = name.into();
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(world_pos, half_size);
    let brain = ambition_entity_catalog::placements::CharacterBrain::Custom(archetype_id.into());
    let mut enemy = super::actor_clusters::ActorClusterSeed::new_in(
        catalog,
        roster,
        id.clone(),
        name.clone(),
        aabb,
        brain,
        &[],
    );
    // `ActorClusterSeed::new_in` already sets HP from the resolved spec.
    // Boss-spawned minions shouldn't auto-respawn — they're part of
    // the encounter, not a static sandbag.
    enemy.status.respawn_timer = 999_999.0;
    let feature_aabb = CenteredAabb::from_aabb(aabb);
    EnemyActorSpawnPlan::hostile(
        format!("Runtime minion: {name}"),
        id.clone(),
        name.clone(),
        feature_aabb,
        enemy,
    )
    .with_faction(faction)
    .with_aggression(aggression)
    .spawn_into(commands, session_scope, entity);
    commands
        .entity(entity)
        .insert(super::EncounterMob::new(encounter_id));
    if let Some(rs) = super::actor_clusters::sprite_render_size_for_name_in(
        catalog,
        &name,
        aabb.half_size() * 2.0,
    ) {
        commands
            .entity(entity)
            .insert(crate::features::ActorRenderSize(rs));
    }
}

pub(super) fn spawn_enemy(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    paths: &[(String, ambition_engine_core::KinematicPath)],
) {
    let _ = spawn_enemy_with_faction(
        commands,
        catalog,
        roster,
        session_scope,
        authored,
        paths,
        super::ActorFaction::Enemy,
    );
}

/// Like [`spawn_enemy`] but the spawned body takes `faction` (the duel/arena path
/// puts its two fighters on DIFFERENT factions so they can damage each other under
/// the physical damage rule). Composite mounts ignore the override (they fan out
/// their own factions); the duel fighters are solo. Returns the spawned solo
/// body's entity so a caller (the duel staging) can attach extra markers; `None`
/// for the composite mount/rider path (it fans out two of its own entities).
pub(super) fn spawn_enemy_with_faction(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    paths: &[(String, ambition_engine_core::KinematicPath)],
    faction: super::ActorFaction,
) -> bevy::ecs::entity::Entity {
    let root = commands.spawn_empty().id();
    spawn_enemy_with_faction_into(
        commands,
        catalog,
        roster,
        session_scope,
        root,
        authored,
        paths,
        faction,
    );
    root
}

/// Populate an enemy onto a root the construction executor allocated.
///
/// **This no longer spawns a giant's hand limbs.** They were minted here as two
/// authoritative roots no plan row named — the last `KNOWN_LEGACY_FAMILIES`
/// entry. Giant hands are explicit construction rows now
/// ([`crate::construction::authored_giant_requests`]); a `"giant"`-class host is
/// built by [`populate_giant_host_into`] (this plus the host-side rig state) and
/// each hand by [`populate_giant_hand_into`], with the two joined by
/// `ambition.limb` relations. This function stays the path for every ordinary,
/// unlimbed enemy.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_enemy_with_faction_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    paths: &[(String, ambition_engine_core::KinematicPath)],
    faction: super::ActorFaction,
) {
    let spec = roster.spec_for_brain(&authored.payload);
    let enemy = super::actor_clusters::ActorClusterSeed::new_in(
        catalog,
        roster,
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    spawn_solo_enemy_into(
        commands,
        catalog,
        session_scope,
        root,
        enemy,
        authored,
        faction,
    );
    attach_mount_role(commands, root, &spec);
}

/// Populate a `"giant"`-class LIMBED HOST onto an executor-allocated root: the
/// ordinary enemy body, plus the host-side rig state the limb router reads.
///
/// The rig MEMBERSHIP (`LimbRig`) is installed by the `ambition.limb` relation
/// wiring, one entry per hand; the two host-owned scratch components
/// ([`super::LimbIntents`], [`super::LimbRouteState`]) belong to the host itself
/// and are inserted here. `spawn_giant_hand_limbs` used to insert all three
/// together while also minting the hand bodies; now the hands are their own plan
/// rows and only their back-link is a relation.
pub(crate) fn populate_giant_host_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    paths: &[(String, ambition_engine_core::KinematicPath)],
    faction: super::ActorFaction,
) {
    spawn_enemy_with_faction_into(
        commands,
        catalog,
        roster,
        session_scope,
        root,
        authored,
        paths,
        faction,
    );
    commands.entity(root).insert((
        super::LimbIntents::default(),
        super::LimbRouteState::default(),
    ));
}

/// Populate one giant hand body onto an executor-allocated root. The `Limb`
/// component and the host's rig entry are installed by the `ambition.limb`
/// relation, not here — this builds the ordinary actor body and marks it
/// non-hostile so targeting ignores it (the fan-out is its only driver).
pub(crate) fn populate_giant_hand_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
) {
    let enemy = super::actor_clusters::ActorClusterSeed::new_in(
        catalog,
        roster,
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        &[],
    );
    spawn_solo_enemy_into(
        commands,
        catalog,
        session_scope,
        root,
        enemy,
        authored,
        super::ActorFaction::Enemy,
    );
    commands
        .entity(root)
        .insert(super::ActorDisposition::Peaceful);
}

/// One giant hand's fully-resolved construction facts, computed at PLAN time
/// from the giant's authored box — no `Entity`, no live world.
pub struct GiantHandPlan {
    pub slot: super::LimbSlot,
    /// Stable spawned identity under the giant, deterministic across runs.
    pub ordinal: u64,
    /// Stable feature id, derived from the giant's authored id and the fixed side.
    pub feature_id: String,
    /// Where the hand body starts, in world space.
    pub aabb: ae::Aabb,
    /// Host-local idle anchor (the `Limb::home_offset`), stated relative to the
    /// giant's center.
    pub home_offset: ae::Vec2,
}

/// Whether an archetype is a limbed `"giant"`-class host. The one predicate the
/// request builder and any remaining loop share, so they cannot disagree about
/// which enemies are planned as hosts.
pub(crate) fn spec_is_limbed_host(spec: &super::super::enemies::CharacterArchetypeSpec) -> bool {
    mount_has_hand_limbs(spec)
}

/// Resolve the two hand plans for a giant, at plan time. The geometry that used
/// to live inside `spawn_giant_hand_limbs` is pure and moves here so the hands
/// can be prepared as rows before anything is spawned.
pub(crate) fn giant_hand_plans(
    giant_id: &str,
    giant_aabb: ae::Aabb,
    spec: &super::super::enemies::CharacterArchetypeSpec,
) -> Vec<GiantHandPlan> {
    let giant_half = spec
        .default_size
        .map(|s| s * 0.5)
        .unwrap_or_else(|| giant_aabb.half_size());
    let giant_center = giant_aabb.center();
    let hand_size = ae::Vec2::new(giant_half.x * 0.7, giant_half.y * 0.7);
    let home_l = ae::Vec2::new(-giant_half.x * 0.55, giant_half.y * 0.15);
    let home_r = ae::Vec2::new(giant_half.x * 0.55, giant_half.y * 0.15);
    [
        (super::LimbSlot::HandLeft, home_l, "left"),
        (super::LimbSlot::HandRight, home_r, "right"),
    ]
    .into_iter()
    .enumerate()
    .map(|(ordinal, (slot, home, tag))| GiantHandPlan {
        slot,
        ordinal: ordinal as u64,
        feature_id: giant_hand_feature_id(giant_id, tag),
        aabb: ae::Aabb::new(giant_center + home, hand_size * 0.5),
        home_offset: home,
    })
    .collect()
}

/// v1 predicate (Q18): which mount archetypes carry driven hand limbs. Scoped to
/// the `"giant"` class — the only limbed mount today. Generalizing to a
/// data-driven archetype flag is deferred until a second limbed mount exists.
fn mount_has_hand_limbs(spec: &super::super::enemies::CharacterArchetypeSpec) -> bool {
    spec.mount_class.as_deref() == Some("giant")
}

/// A giant hand's stable `FeatureId`, derived from the giant's AUTHORED id and the
/// hand's fixed side — an entity-free game fact, so two sims that spawn the same
/// giant give its hands the same identity. It deliberately takes `giant_id: &str`,
/// never an `Entity`: the old form used `giant.index()` (an allocator slot), which
/// handed the hands a different `SimId` every run and broke snapshot/replay
/// determinism (netcode.md N3.2 boss-hand residual).
fn giant_hand_feature_id(giant_id: &str, side: &str) -> String {
    format!("giant_gnu_hand_{side}_{giant_id}")
}

/// ADR 0020: give a standalone actor its mount role from its archetype. A
/// `mount_class` archetype becomes [`Mountable`] (a rideable platform); a
/// `pilotable_mount_classes` archetype becomes a would-be rider ([`CanPilot`]).
/// The `RidingOn`/`MountSlot` link itself is installed later by
/// [`super::resolve_pending_mount_links`] from the room's authored `mounted_on`
/// refs — this only tags the two roles.
fn attach_mount_role(
    commands: &mut Commands,
    entity: bevy::ecs::entity::Entity,
    spec: &super::super::enemies::CharacterArchetypeSpec,
) {
    if let Some(class) = &spec.mount_class {
        // Saddle offset heuristic: the rider sits just above the mount's top.
        // Feel-tunable; a mount that wants a precise saddle can grow a field.
        let mount_size = spec.default_size.unwrap_or(ae::Vec2::new(64.0, 64.0));
        let rider_offset = ae::Vec2::new(0.0, -(mount_size.y * 0.5 + 40.0));
        commands.entity(entity).insert((
            super::Mountable {
                rider_offset,
                class: super::MountClass(class.clone()),
                control_grant: super::ControlGrant::Total,
                death_impact: match spec.mount_death_splash {
                    Some(amount) => super::MountDeathImpact::Splash(amount),
                    None => super::MountDeathImpact::Dismount,
                },
            },
            // A heavy mount keeps the pair's center of gravity near itself, so
            // the lighter rider orbits it under a gravity flip (sync reads Mass).
            super::Mass(spec.mass),
        ));
    }
    if !spec.pilotable_mount_classes.is_empty() {
        commands.entity(entity).insert((
            super::CanPilot {
                classes: spec
                    .pilotable_mount_classes
                    .iter()
                    .cloned()
                    .map(super::MountClass)
                    .collect(),
            },
            super::Mass(spec.mass),
        ));
    }
}

/// Single-entity hostile spawn — the common path after composite
/// mount/rider fan-out has been handled. Returns the spawned body entity.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_solo_enemy_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    session_scope: SessionSpawnScope,
    entity: bevy::ecs::entity::Entity,
    enemy: super::actor_clusters::ActorClusterSeed,
    authored: &crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    faction: super::ActorFaction,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    EnemyActorSpawnPlan::hostile(
        format!("Feature actor enemy: {}", authored.name),
        authored.id.clone(),
        authored.name.clone(),
        feature_aabb,
        enemy,
    )
    .with_faction(faction)
    .spawn_into(commands, session_scope, entity);
    // A named catalog character carries its authored sprite render size on the
    // shared `ActorRenderSize` (the same component the peaceful-NPC path sets), so
    // the sprite draws at the authored scale and matches the body the per-frame
    // `CenteredAabb` sync derives from the sprite-sized collision.
    if let Some(rs) = super::actor_clusters::sprite_render_size_for_name_in(
        catalog,
        &authored.name,
        authored.aabb.half_size() * 2.0,
    ) {
        commands
            .entity(entity)
            .insert(crate::features::ActorRenderSize(rs));
    }
}
/// Human label for an authored NPC: the catalog `display_name` for the
/// spawn's `character_id`, falling back to the authored world-IR name.
///
/// The fallback is the honest answer for an NPC with no catalog row
/// (synthetic/legacy spawns, where `character_id` is `None`) — but a spawn
/// that DOES name a character and fails to resolve is a content bug, not a
/// naming preference, so that case is worth seeing rather than papering over.
fn npc_display_label(
    catalog: &CharacterCatalog,
    interactable: &ambition_interaction::Interactable,
    authored_name: &str,
) -> String {
    let ambition_interaction::InteractionKind::Npc { character_id, .. } = &interactable.kind else {
        return authored_name.to_string();
    };
    let Some(character_id) = character_id.as_deref() else {
        return authored_name.to_string();
    };
    match catalog.display_name(character_id) {
        Some(display_name) => display_name.to_string(),
        None => {
            warn!(
                character_id,
                authored_name,
                "NPC spawn names a character with no catalog row; \
                 falling back to the authored name for its label"
            );
            authored_name.to_string()
        }
    }
}

pub(super) fn spawn_interactable(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    authored: &crate::rooms::Authored<crate::rooms::InteractableSpec>,
    paths: &[(String, ambition_engine_core::KinematicPath)],
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let interactable = super::spawn_static::interactable_from_authored(authored);
    let interactable = &interactable;
    if matches!(
        interactable.kind,
        ambition_interaction::InteractionKind::Npc { .. }
    ) {
        // Every LDtk `NpcSpawn` shares the identifier "NpcSpawn", so the world
        // IR's `Authored.name` is never the character's label — the human label
        // lives in the catalog, keyed by the spawn's `character_id`. The LDtk
        // crate deliberately has no catalog dependency, so this is the first
        // seam that can resolve it. Everything reading `ActorIdentity.name`
        // (nameplates, interaction banner, dialogue speaker fallback, speech
        // SFX keying, and the `id_for_display_name` sprite-size lookup) depends
        // on this being the display name.
        let label = npc_display_label(catalog, interactable, &authored.name);
        NpcActorSpawnPlan::peaceful(
            catalog,
            roster,
            format!("Feature actor npc: {label}"),
            feature_aabb,
            authored.id.clone(),
            label,
            authored.aabb,
            interactable.clone(),
            paths,
        )
        .spawn(commands, session_scope);
    } else if let ambition_interaction::InteractionKind::Custom(payload) = &interactable.kind {
        if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload) {
            commands.spawn_session_scoped(
                session_scope,
                (
                    Name::new(format!("Feature switch: {}", authored.name)),
                    FeatureSimEntity,
                    RoomVisual,
                    FeatureId::new(authored.id.clone()),
                    FeatureName::new(authored.name.clone()),
                    feature_aabb,
                    SwitchFeature::new(activation),
                    SwitchOn(false),
                ),
            );
        }
    }
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub(super) fn spawn_encounter_mob(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    encounter_id: impl Into<String>,
    id: String,
    brain: ambition_entity_catalog::placements::CharacterBrain,
    pos: ae::Vec2,
    size: ae::Vec2,
) {
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy = super::actor_clusters::ActorClusterSeed::new_in(
        catalog,
        roster,
        id.clone(),
        id.clone(),
        aabb,
        brain,
        &[],
    );
    // `ActorClusterSeed::new_in` already sets HP from the resolved spec.
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
    .spawn(commands, session_scope);
    commands
        .entity(entity)
        .insert(EncounterMob::new(encounter_id));
    if let Some(rs) =
        super::actor_clusters::sprite_render_size_for_name_in(catalog, &id, size * 0.5 * 2.0)
    {
        commands
            .entity(entity)
            .insert(crate::features::ActorRenderSize(rs));
    }
}

/// Lib-side executor for `Effect::Summon`: the runtime-dynamic origin of the
/// three the construction planner covers.
///
/// Lives next to the spawner (not in `effects::apply_effects`) so the
/// `ambition_vfx` crate stays free of the enemy-roster substrate.
///
/// ## Why a summon is planned at all
///
/// One minion is a small plan, and running it through the same planner as a
/// room's contents is the point rather than an overhead: it is what gives a
/// summoned body a real dynamic identity (`SimId::spawned` under its summoner,
/// taken from the summoner's own `SimIdCounter`) and an explicit
/// [`SpawnOrigin::Dynamic`] naming its parent. Before this, a minion carried a
/// `FeatureId` and nothing else, so `ensure_sim_id` filed it under the AUTHORED
/// `placement:` namespace — the one namespace a summoned add categorically is
/// not in — and two summons sharing an authored id collided outright.
///
/// A summon that cannot name its summoner's identity is skipped: the spawner is
/// mid-migration to `SimId` (an unidentified body cannot lend an identity), and
/// minting a parentless dynamic id would reintroduce exactly the ambiguity this
/// replaces.
/// One summoner's reserved stretch of its own identity sequence.
///
/// Carries the value planning READ as well as the value it wants to write, so
/// applying the reservation can tell "nothing moved" from "someone else spent
/// these ids while this batch was in flight".
struct SummonerSequenceReservation {
    summoner: ambition_platformer_primitives::sim_id::SimId,
    /// What the counter held when this batch planned against it.
    expected: u64,
    /// What it must hold afterwards — `expected` plus one per summon reserved.
    next: u64,
}

impl SummonerSequenceReservation {
    /// Whether this summoner's counter still holds what planning assumed.
    fn still_valid(
        &self,
        counter: Option<&ambition_platformer_primitives::sim_id::SimIdCounter>,
    ) -> bool {
        counter.is_some_and(|counter| counter.0 == self.expected)
    }
}

pub fn apply_summon_effects(
    mut commands: bevy::prelude::Commands,
    mut requests: bevy::prelude::MessageReader<ambition_vfx::EffectRequest>,
    character_catalog: bevy::prelude::Res<CharacterCatalog>,
    character_roster: bevy::prelude::Res<CharacterRoster>,
    boss_catalog: bevy::prelude::Res<BossCatalog>,
    recipes: bevy::prelude::Res<crate::construction::ActorConstructionRegistry>,
    active_session: Option<bevy::prelude::Res<ActiveSessionScope>>,
    identities: bevy::prelude::Query<&ambition_platformer_primitives::sim_id::SimId>,
    // Read-only: the advance is a queued command, not a direct write.
    counters: bevy::prelude::Query<&ambition_platformer_primitives::sim_id::SimIdCounter>,
) {
    use ambition_platformer_primitives::construction::{ConstructionPlan, ConstructionScope};

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        requests.clear();
        return;
    };

    // Sequence numbers are RESERVED here and applied only as part of the commit.
    // `SimIdCounter` is snapshot-registered authoritative state, so advancing it
    // while assembling requests would mean a rejected batch had already consumed
    // dynamic identities that no entity was ever built for — a mutation that
    // survives into the next snapshot.
    //
    // Each reservation records the value it read, so applying it can verify the
    // counter is still what planning assumed rather than blindly overwriting.
    // ⚠ This is an ordered command, NOT rollback atomicity: the commands are
    // applied in sequence at the next flush, and nothing un-applies the earlier
    // ones if a later one finds its precondition violated. What it buys is that
    // a REFUSAL costs nothing and a violation is loud instead of silent.
    let mut reservations: std::collections::BTreeMap<
        bevy::prelude::Entity,
        SummonerSequenceReservation,
    > = std::collections::BTreeMap::new();
    let mut planned = Vec::new();
    for req in requests.read() {
        let ambition_vfx::Effect::Summon(s) = &req.effect else {
            continue;
        };
        let (Ok(summoner), Ok(counter)) = (identities.get(req.owner), counters.get(req.owner))
        else {
            // Loud, not silent: every body carrying a `FeatureId` is identified
            // at the head of the tick, so reaching this means the emitter is
            // outside the identity migration and its summons would have no
            // reconstructable provenance.
            bevy::log::warn!(
                target: "ambition::construction",
                "summon `{}` skipped: its emitter has no simulation identity to descend from",
                s.id,
            );
            continue;
        };
        // Successive summons from one summoner in a single batch each advance
        // the reserved value, so two adds never claim one identity.
        let reservation =
            reservations
                .entry(req.owner)
                .or_insert_with(|| SummonerSequenceReservation {
                    summoner: summoner.clone(),
                    expected: counter.0,
                    next: counter.0,
                });
        let taken = reservation.next;
        reservation.next += 1;
        planned.push(crate::construction::summoned_minion_request(
            summoner,
            taken,
            crate::construction::SummonedMinionParams {
                feature_id: s.id.clone(),
                name: s.name.clone(),
                pos: s.pos,
                half_size: s.half_size,
                archetype_id: s.archetype_id.clone(),
                encounter_id: s.encounter_id.clone(),
                faction: crate::combat::actor_faction_from_hit_side(s.faction),
            },
        ));
    }
    if planned.is_empty() {
        return;
    }

    let scope = ConstructionScope {
        // A summon is not a content artifact. It says so explicitly rather than
        // by writing the same zero epoch a reset and a fixture also wrote, which
        // is what made the three indistinguishable to a commit boundary.
        binding: ambition_platformer_primitives::construction::ContentBinding::RuntimeDynamic,
        room: None,
    };
    let services = crate::construction::ActorConstructionServices {
        context: crate::world::placements::ActorPlacementContext::new(
            &character_catalog,
            &character_roster,
        ),
        boss_catalog: boss_catalog.clone(),
    };

    // Planning stays out here, against the App's own registry, and stays pure:
    // a rejected batch has spent nothing and built nothing.
    let live: std::collections::BTreeSet<_> = identities.iter().cloned().collect();
    let plan = match ConstructionPlan::prepare(scope.clone(), planned, &live, &recipes) {
        Ok(plan) => plan,
        Err(error) => {
            bevy::log::error!(
                target: "ambition::construction",
                "summon batch rejected before mutation: {error}"
            );
            return;
        }
    };

    // The counter check, the construction, and the advance then happen inside
    // ONE exclusive-world command, so nothing can spend this summoner's
    // identities between the check and the spawn.
    //
    // ⚠ Atomicity of DECISION, not rollback. Bevy commands do not un-apply.
    // What this buys is that the counters are verified while holding exclusive
    // access, immediately before the construction that depends on them, with no
    // window in between — so a refusal happens with nothing built, and a commit
    // is never followed by the discovery that its reservation was already
    // stale. There is consequently no `max()` recovery path: by the time the
    // advance runs, the value it is replacing has just been read under the same
    // lock.
    commands.queue(move |world: &mut bevy::prelude::World| {
        use ambition_platformer_primitives::sim_id::SimIdCounter;

        for (owner, reservation) in &reservations {
            let counter = world.get::<SimIdCounter>(*owner);
            if !reservation.still_valid(counter) {
                bevy::log::error!(
                    target: "ambition::construction",
                    "summon batch refused: summoner `{}` no longer holds the counter value {} \
                     this batch reserved against (now {:?}). Nothing was built.",
                    reservation.summoner,
                    reservation.expected,
                    counter.map(|counter| counter.0),
                );
                return;
            }
        }

        {
            let mut commands = world.commands();
            let mut ctx = ambition_platformer_primitives::construction::ConstructionExecCtx {
                commands: &mut commands,
                scope: &scope,
                session: session_scope,
                services: &services,
            };
            plan.commit(&mut ctx);
        }
        world.flush();

        for (owner, reservation) in reservations {
            if let Some(mut counter) = world.get_mut::<SimIdCounter>(owner) {
                counter.0 = reservation.next;
            }
        }
    });
}

#[cfg(test)]
mod giant_hand_identity_tests {
    use super::giant_hand_feature_id;
    use ambition_platformer_primitives::sim_id::SimId;

    /// The hand's identity is a pure function of the giant's AUTHORED id + its
    /// fixed side — no `Entity`, so it is the same across two sims fed the same
    /// inputs. The old form derived the `_N` suffix from `giant.index()`, an
    /// allocator slot: this pins that the suffix is now the authored id instead.
    #[test]
    fn a_giant_hands_feature_id_is_deterministic_from_the_authored_id() {
        assert_eq!(
            giant_hand_feature_id("gnu-42", "left"),
            "giant_gnu_hand_left_gnu-42"
        );
        assert_eq!(
            giant_hand_feature_id("gnu-42", "right"),
            "giant_gnu_hand_right_gnu-42"
        );
        // Two different giants → two different hand ids (no live collision);
        // the SAME giant id → the SAME hand id (determinism across sims).
        assert_ne!(
            giant_hand_feature_id("gnu-42", "left"),
            giant_hand_feature_id("gnu-99", "left")
        );
        assert_eq!(
            giant_hand_feature_id("gnu-42", "left"),
            giant_hand_feature_id("gnu-42", "left")
        );
    }

    /// A spawned hand lands in the SPAWNED namespace parented to the giant —
    /// `SimId::spawned(giant_placement, ordinal)` — not the authored `placement:`
    /// namespace. The ordinal is the fixed loop order (left=0, right=1), so the
    /// pair is deterministic and legible as the giant's children.
    #[test]
    fn a_giant_hand_sim_id_is_a_spawned_child_of_the_giant() {
        let giant = SimId::placement("gnu-42");
        let left = SimId::spawned(&giant, 0);
        let right = SimId::spawned(&giant, 1);
        assert_eq!(left.as_str(), "placement:gnu-42/0");
        assert_eq!(right.as_str(), "placement:gnu-42/1");
        // It is a child of the giant, not a sibling authored placement.
        assert!(left.as_str().starts_with(giant.as_str()));
        assert_ne!(left, giant);
    }
}
