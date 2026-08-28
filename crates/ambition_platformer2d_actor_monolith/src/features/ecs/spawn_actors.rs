//! Actor spawn helpers for ECS feature entities.
//!
//! This module covers bosses, hostile enemies, peaceful NPC actors, dynamic
//! boss minions, and encounter mobs. Static pickups/chests/breakables live in
//! `spawn_static.rs`; composite mount/rider fan-out lives in `spawn_mounts.rs`.

use super::brain_builders::enemy_default_brain;
use super::*;
use ambition_boss_encounter::{BossCatalog, BossClusterScratch, BossConfig, BossOverrides};
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::limb::LimbSlot;
use ambition_combat::components::BossPatternTimer;
use ambition_combat::components::{
    ActorAggression, ActorPose, BossDeathAnimation, BossPhase, CenteredAabb, CombatKit,
    DamageableVolumes, EncounterMob, FeatureId, FeatureName, PogoPolicy, PogoTargetVolumes,
};
use ambition_encounter::switches::{SwitchFeature, SwitchOn};
use ambition_platformer2d_core::body_clusters::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::prelude::{Message, Name};

/// Programmatic actor-spawn request — the public seam for dropping a specific
/// actor into a live sim at an arbitrary position WITHOUT authoring an LDtk room.
///
/// Room load is the only other way an actor reaches the world, and it needs a
/// fully-built [`ambition_platformer2d_world::rooms::RoomSpec`] — too heavy for scenario tests and
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
    pub faction: ambition_combat::components::ActorFaction,
    /// Feature id of another actor in the SAME spawn batch this body should hold a
    /// personal grudge against. Resolved post-spawn (once both entities exist) into
    /// an [`ActorAggression::grudge`](ambition_combat::components::ActorAggression),
    /// which drives relational targeting AND authorizes same-faction damage
    /// (`damage_lands`) — the mechanism behind two `Npc` duelists feuding without a
    /// hostile faction. `None`  no grudge (fights on faction lines only).
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
    /// A hostile enemy — the same path a room `EnemySpawn` takes.
    Enemy {
        brain: ambition_entity_catalog::placements::CharacterBrain,
        /// WHICH CHARACTER this instantiates, when the caller knows.
        ///
        /// a programmatic spawn may name a character now. It could only name a brain key, so
        /// code that wanted a specific creature had to name the ARCHETYPE that happened to describe
        /// it — and when that creature migrated, the request silently resolved the `combatant`
        /// fallback instead.
        character: ambition_entity_catalog::CharacterId,
    },
}

/// Drain [`SpawnActorRequest`]s and materialize each actor.
///
/// Phase-4 scope ruling: this is the ONE sanctioned out-of-plan spawn path,
/// and it is for PROGRAMMATIC scene setup only — an RL episode reset, a
/// scenario-test fixture, a dev command. Authored room content must never
/// route through it: room occupants are construction plan rows (provider
/// stagers included, via `RoomContentStagingRegistry`), stamped and verified
/// at the room boundary. A body spawned here carries no plan identity and is
/// invisible to boundary verification by design; the moment such a body needs
/// identity, reconstruction, or relations, it has outgrown this path and
/// belongs in the planner.
///
/// Intentionally UNGATED by `gameplay_allowed`: programmatic scene setup must
/// apply regardless of the coarse `GameMode`, unlike the in-gameplay
/// `apply_summon_effects`. The spawned entity's own systems are still
/// gameplay-gated, so an actor placed during a transition just waits inert
/// until play resumes.
pub fn apply_spawn_actor_requests(
    mut commands: bevy::prelude::Commands,
    mut requests: bevy::prelude::MessageReader<SpawnActorRequest>,
    character_catalog: bevy::prelude::Res<CharacterCatalog>,
    authored_sheets: bevy::prelude::Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    // `Option`, matching its sibling on the authored path, and the absence is MEANINGFUL rather
    // than defensive: a composition that never registered a character has no such resource at
    // all, and that is exactly the state is about.
    prepared: Option<bevy::prelude::Res<crate::character_runtime::PreparedCharacterRegistry>>,
    boss_catalog: bevy::prelude::Res<BossCatalog>,
    active_session: Option<bevy::prelude::Res<ActiveSessionScope>>,
) {
    // Collect (feature id, entity, grudge-target id) for the Enemy spawns this batch
    // so a mutual grudge (a staged duel pair) can be cross-wired once both entities
    // exist — `grudge_against` names a foe by id, resolvable only after the whole
    // batch has reserved its entities.
    let mut staged: Vec<(String, bevy::prelude::Entity, Option<String>)> = Vec::new();
    // The stand-in for a composition that registered nothing. Named rather than
    // inlined so the two readings — "no cast published" and "this character is
    // not in the cast" — stay distinguishable at the call site.
    let empty_cast = crate::character_runtime::PreparedCharacterRegistry::default();
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        requests.clear();
        return;
    };
    for req in requests.read() {
        // A refused request (`None`) produced NO entity and must not join the
        // grudge batch either — otherwise a phantom id resolves and stamps
        // `ActorAggression` onto nothing.
        let Some(entity) = spawn_staged_actor(
            &mut commands,
            &character_catalog,
            &authored_sheets,
            prepared.as_deref().unwrap_or(&empty_cast),
            &boss_catalog,
            session_scope,
            req,
        ) else {
            continue;
        };
        if matches!(req.kind, SpawnActorKind::Enemy { .. }) {
            staged.push((req.id.clone(), entity, req.grudge_against.clone()));
        }
    }
    wire_staged_grudges(&mut commands, &staged);
}

/// Materialize one staged actor, or refuse WITHOUT allocating anything.
///
/// The one staged-actor constructor for the programmatic path. The
/// message-driven applier above calls it; the `ambition.staged-actor`
/// construction recipe calls [`spawn_staged_actor_into`] directly with a root
/// the plan executor owns. `None` means the request was refused — validation
/// runs BEFORE `spawn_empty`, because a refused request must produce no
/// entity at all: an empty leaked root would still be recorded as a spawned
/// enemy by the caller's batch grudge map and could receive `ActorAggression`
/// through `wire_staged_grudges` (: an in-recipe refusal is too
/// late once the allocation belongs to the caller).
pub(crate) fn spawn_staged_actor(
    commands: &mut Commands,
    character_catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    req: &SpawnActorRequest,
) -> Option<bevy::ecs::entity::Entity> {
    // The programmatic path does not lower through the planner, so it cannot
    // mint a giant's host + two hand rows — refuse a giant-class spec like
    // every other runtime origin, instead of silently producing a handless
    // host.
    if let SpawnActorKind::Enemy { character, .. } = &req.kind {
        //  ASK THE CHARACTER, because this path HAS one. The doc on
        // `spec_is_limbed_host` says the runtime refusals cannot be
        // character-aware because *"making the refusal character-aware means
        // giving those paths a placement, which they do not have"* — true of
        // summons and encounter waves, and NOT true here: a staged request has
        // carried a `character` since P1.12. A character that authors a
        // non-giant mount was still being refused by whatever archetype its
        // brain key happened to name.
        if reject_runtime_giant(
            //  ONE question, asked of the CHARACTER. This resolved the
            // character AND the placement's brain key against the roster and
            // merged the two answers, so a body whose character said nothing
            // inherited whatever archetype its brain key happened to name — and a
            // misspelled key inherited the reserved `combatant` row's limb answer.
            // A character is the only thing that states limbs now.
            is_limbed_host(prepared.get(character.as_str())),
            "programmatic staged actor",
            &req.id,
        ) {
            return None;
        }
    }
    let root = commands.spawn_empty().id();
    spawn_staged_actor_into(
        commands,
        character_catalog,
        authored_sheets,
        prepared,
        boss_catalog,
        session_scope,
        root,
        req,
    );
    Some(root)
}

/// Populate a staged actor onto a root the construction executor allocated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_staged_actor_into(
    commands: &mut Commands,
    character_catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    req: &SpawnActorRequest,
) {
    let aabb = ae::Aabb::new(req.pos, req.half_size);
    match &req.kind {
        SpawnActorKind::Boss { brain, overrides } => {
            let authored = ambition_platformer2d_world::rooms::Authored::new(
                req.id.clone(),
                req.name.clone(),
                aabb,
                brain.clone(),
            );
            spawn_boss_with_overrides_into(
                commands,
                boss_catalog,
                session_scope,
                root,
                &authored,
                overrides,
            );
        }
        SpawnActorKind::Enemy { brain, character } => {
            // The programmatic path already refused BEFORE allocating the root
            // (`spawn_staged_actor`), so leaving the caller-owned root empty here is deliberate — a
            // plan row left unbuilt is exactly what the room transaction's roster verification
            // exists to flag. Same character-first question as `spawn_staged_actor`'s — the two
            // refusals must agree, or the programmatic path would allocate a root the recipe then
            // refuses to fill.
            if reject_runtime_giant(
                // See the twin above: the character is the only authority on
                // limbs, so the two refusals cannot drift apart.
                is_limbed_host(prepared.get(character.as_str())),
                "programmatic staged actor",
                &req.id,
            ) {
                return;
            }
            let payload = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                brain.clone(),
                character.clone(),
            );
            let authored = ambition_platformer2d_world::rooms::Authored::new(
                req.id.clone(),
                req.name.clone(),
                aabb,
                payload,
            );
            // Staged outside the authored RoomSpec lists: mark it so the
            // renderer's runtime-visual discovery gives it a sprite, the same as
            // any authored enemy.
            spawn_enemy_with_faction_into(
                commands,
                character_catalog,
                authored_sheets,
                prepared,
                //  no placement, so no placement-authored policy. A staged
                // actor is built from a REQUEST rather than from a level, and
                // `EnemySpawnSpec::new` leaves `brain_profile` absent — so an
                // empty registry is the honest value here rather than a
                // borrowed one, and nothing can name into it.
                &ambition_characters::actor::character_catalog::BrainProfileRegistry::default(),
                session_scope,
                root,
                &authored,
                &[],
                req.faction,
            );
            commands
                .entity(root)
                .insert(ambition_combat::components::RuntimeStagedActor);
        }
    }
}

/// Cross-wire mutual grudges for a freshly-staged feuding set. `staged` pairs each
/// new entity with the feature id of the foe it should grudge (from
/// [`SpawnActorRequest::grudge_against`]). Each id is resolved against the SAME batch
/// and that fighter's [`ActorAggression`](ambition_combat::components::ActorAggression) is stamped with a
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
        commands
            .entity(*entity)
            .insert(ambition_combat::components::ActorAggression {
                grudge: Some(foe),
                ..ambition_combat::components::ActorAggression::hostile()
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
    faction: ambition_combat::components::ActorFaction,
    aggression: ambition_combat::components::ActorAggression,
    brain: ambition_characters::brain::Brain,
    action_set: ambition_characters::brain::ActionSet,
    combat_kit: ambition_combat::CombatKit,
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
        let brain = enemy_default_brain(&enemy.config, enemy.body.0.abilities.abilities);
        // A CHARACTER-FIRST BODY HAS NO ARCHETYPE TO ASK — and as of AC6 there is no other kind
        // of body. The kit that actually reaches such a body arrives from
        // `grant_prepared_character_body` moments later, so nothing at all is both the honest
        // answer and the only one.
        let action_set = ambition_characters::brain::ActionSet::peaceful();
        let combat_kit = ambition_combat::CombatKit::default();
        let held_item = None;
        // A character's signature moves AND its basic melee/ranged fold into ONE
        // moveset — the melee subsumption (§A1 / §3a): a plain swing is an
        // `"attack"`-verb move run by the SAME moveset runtime as the specials.
        //
        // The melee/ranged SOURCE is the resolved `action_set` (kit + held item),
        // the SAME capability the brain's `emit_brain_action_messages` gate reads.
        // A body that can emit a melee (its `ActionSet.melee` is `Some`, e.g.
        // granted by a held weapon) MUST have a moveset `"attack"` move or it
        // could never swing; building from `action_set` closes that gap
        // definitionally, because capability and moveset share one source.
        let moveset = ambition_combat::moveset::build_actor_moveset(
            None,
            action_set.melee.as_ref(),
            action_set.ranged.as_ref(),
            None,
        );
        Self {
            entity_name: entity_name.into(),
            feature_id: feature_id.into(),
            feature_name: feature_name.into(),
            feature_aabb,
            enemy,
            faction: ambition_combat::components::ActorFaction::Enemy,
            aggression: ambition_combat::components::ActorAggression::hostile(),
            brain,
            action_set,
            combat_kit,
            held_item,
            moveset,
        }
    }

    pub(super) fn with_faction(
        mut self,
        faction: ambition_combat::components::ActorFaction,
    ) -> Self {
        self.faction = faction;
        self
    }

    pub(super) fn with_aggression(
        mut self,
        aggression: ambition_combat::components::ActorAggression,
    ) -> Self {
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
        let (identity, disposition, combat) = enemy_component_snapshot(&self.enemy);
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
                    )
                    .with_motion_model(motion_model),
                    cluster_bundle,
                    self.brain,
                    self.action_set,
                    ambition_characters::control::ActorControl::default(),
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
                .contains_key(ambition_combat::moveset::ATTACK_VERB);
            // Likewise a body whose moveset carries the `"ranged"` verb has its shot
            // subsumed: mark it so the flat `frame.fire → Ranged` emission is skipped
            // (the move's fire event spawns the shot instead — no double-fire).
            let has_ranged = moveset
                .verbs
                .contains_key(ambition_combat::moveset::RANGED_VERB);
            commands
                .entity(entity)
                .insert(ambition_combat::moveset::ActorMoveset(moveset));
            if has_attack {
                commands
                    .entity(entity)
                    .insert(ambition_combat::moveset::MovesetMelee);
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
    combat_kit: ambition_combat::CombatKit,
    aggression: ambition_combat::components::ActorAggression,
}

impl NpcActorSpawnPlan {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn peaceful(
        catalog: &CharacterCatalog,
        authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        prepared: &crate::character_runtime::PreparedCharacterRegistry,
        entity_name: impl Into<String>,
        feature_aabb: CenteredAabb,
        id: impl Into<String>,
        name: impl Into<String>,
        spawn_aabb: ae::Aabb,
        interactable: ambition_interaction::Interactable,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> Self {
        let id = id.into();
        let name = name.into();
        // The hostile archetype this actor becomes when provoked: feeds its
        // stored CombatKit (so a provoked NPC fights with the right weapon) and
        // the seed's inert reconstruction spec.
        // An NPC is by construction a UNIQUE named placement: its death is
        // permanent (ADR 0022 "Morrowind rules") regardless of the mob-tier
        // respawn policy the borrowed combat archetype authors. The policy is a
        // property of the PLACEMENT, and this placement is a person.
        //
        // That pin lives on the SEED's tuning (`ActorClusterSeed::new_peaceful_npc_in`) and
        // survives provocation via `ActorTuning::adopting_archetype`. A peaceful NPC carries the
        // kit it will use if provoked, resolved at spawn — and it was resolved by handing an
        // ARCHETYPE the NPC's display name and taking whatever that matched. For a character that
        // authors its own repertoire, that is a borrowed weapon: the cove pirates state a bolt, a
        // swipe and a gun-sword, and were being handed `pirate_raider`'s instead.
        //
        //  the NPC road's fallback is narrower than the enemy road's, and so
        // is its refusal. An NPC's BODY comes from its catalog row, not from
        // the definition — so an unregistered-but-cataloged character still gets
        // the right body and only the KIT is borrowed. What has no fallback at
        // all is a character in NEITHER, where `new_peaceful_npc_in` drops to a
        // display-name match: a person built by resembling somebody.
        let authored_kit = match npc_character_id(&interactable) {
            None => None,
            Some(character) => match prepared.get(character) {
                Some(prepared) => prepared.kit.action_set(),
                None => {
                    super::spawn::report_unprepared_character(
                        character,
                        &format!("NPC `{}`", id),
                        prepared,
                        catalog
                            .display_name(character)
                            .is_some()
                            .then_some("its catalog row's body with a borrowed kit"),
                    );
                    None
                }
            },
        }
        .map(ambition_combat::components::CombatKit::from_action_set);
        let combat_kit = match authored_kit {
            Some(kit) => kit,
            //  it is what a body that authored NO kit fights with once
            // provoked. A Hall NPC authors `peaceful`, so without this it would
            // have nothing to swing — which is the same reason Smash grants
            // `smash_fighter_kit()`, and the reason both are one concept.
            None => super::brain_builders::default_fighting_kit(),
        };
        let (mut seed, render_size) = super::actor_clusters::ActorClusterSeed::new_peaceful_npc_in(
            authored_sheets,
            catalog,
            Some(prepared),
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
        let (brain, brain_binding) = super::super::npcs::resolve_npc_brain(
            catalog,
            prepared,
            &interactable,
            seed.spawn.pos.x,
            // The seed is already built, so the body a `BrainProfile` default
            // would be paced against is right here.
            &seed.config,
            seed.body.0.abilities.abilities,
        );
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
            // Body CAPABILITY, not AI POLICY: a peaceful NPC carries its authored combat kit as
            // its `ActionSet` (the same kit it fights with when provoked), so the SAME body can
            // throw its authored punch/swing when a player DRIVES it — while its peaceful
            // autonomous brain simply never presses attack, so it still ambles harmlessly on
            // its own.
            action_set: combat_kit.to_action_set(None),
            combat_kit,
            aggression: ambition_combat::components::ActorAggression::retaliates_when_hit(
                super::super::NPC_HOSTILE_STRIKE_THRESHOLD as u8,
            ),
        }
    }

    #[allow(dead_code)]
    pub(super) fn spawn(self, commands: &mut Commands, session_scope: SessionSpawnScope) -> Entity {
        let root = commands.spawn_empty().id();
        self.spawn_into(commands, session_scope, root);
        root
    }

    /// Populate onto a root someone else allocated — the construction
    /// executor's shape, mirroring `EnemyActorSpawnPlan::spawn_into`.
    pub(super) fn spawn_into(
        self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        root: Entity,
    ) -> Entity {
        let facing = self.seed.kin.facing;
        // Sprite-metadata render size lives on the SHARED `ActorRenderSize`
        // component so it survives a hostile flip (otherwise the body-sized
        // collision would get `collision_scale` re-applied, ballooning the sprite).
        let render_size = self.render_size;
        // Dialogue is a SHARED actor capability (`ActorInteraction`).
        let interaction = ambition_combat::components::ActorInteraction {
            interactable: self.interactable,
            talk_radius: super::super::npcs::NPC_TALK_RADIUS,
        };
        let (identity, disposition, combat) = super::actors::actor_component_snapshot(
            &self.seed,
            ambition_combat::components::ActorDisposition::Peaceful,
        );
        // Uniform melee subsumption (§A1/§3a): a peaceful NPC carries its combat
        // kit's melee as body CAPABILITY (for possession / provocation), so fold it
        // into a moveset `"attack"` move like every hostile — a possessed peaceful
        // NPC's swing runs through the SAME moveset runtime, not the flat path.
        let npc_moveset = ambition_combat::moveset::build_actor_moveset(
            None,
            self.action_set.melee.as_ref(),
            self.action_set.ranged.as_ref(),
            // Peaceful NPC specials, like hostiles, are archetype-authored; the
            // marker is not re-folded (see the hostile path).
            None,
        );
        let motion_model = self.seed.config.tuning.motion_model();
        let cluster_bundle = self.seed.into_components();
        let mut entity = commands.insert_session_scoped(
            session_scope,
            root,
            (
                Name::new(self.entity_name),
                EnemyActorBundle::new(
                    FeatureBaseBundle::new(&self.feature_id, &self.feature_name, self.feature_aabb),
                    identity,
                    disposition,
                    ambition_combat::components::ActorFaction::Npc,
                    ActorPose::from_parts(
                        self.feature_aabb.center,
                        self.feature_aabb.half_size,
                        facing,
                    ),
                    self.combat_kit,
                    self.aggression,
                    combat,
                )
                .with_motion_model(motion_model),
                cluster_bundle,
                self.brain,
                self.action_set,
                ambition_characters::control::ActorControl::default(),
            ),
        );
        let worn = npc_character_id(&interaction.interactable).map(str::to_string);
        entity.insert(interaction);
        //  A CATALOG-BACKED NPC WEARS ITS CHARACTER.
        //
        //  it did not, and that is what made provocation read the SPRITE id
        // . `provoke_actor_in_place` has to know which
        // character a body IS in order to ask what it becomes when struck, and
        // the only identity a peaceful NPC carried was the one its ART resolves
        // through. Threading the gameplay identity into that seam is worth
        // nothing if the body does not have one.
        //
        //  the ANONYMOUS case is the reason this is conditional: a
        // synthetic or legacy NPC placement names no character, and giving it a
        // made-up worn id would be inventing an identity to satisfy a lookup.
        // Absence stays the honest answer, and the legacy name-matcher still
        // covers it.
        if let Some(character) = worn {
            entity.insert(ambition_characters::actor::WornCharacter::new(character));
        }
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
                ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl::Autonomous,
            ));
        }
        if let Some(moveset) = npc_moveset {
            let has_attack = moveset
                .verbs
                .contains_key(ambition_combat::moveset::ATTACK_VERB);
            let has_ranged = moveset
                .verbs
                .contains_key(ambition_combat::moveset::RANGED_VERB);
            entity.insert(ambition_combat::moveset::ActorMoveset(moveset));
            if has_attack {
                entity.insert(ambition_combat::moveset::MovesetMelee);
            }
            if has_ranged {
                entity.insert(ambition_characters::brain::MovesetRanged);
            }
        }
        if let Some(size) = render_size {
            entity.insert(ambition_combat::components::ActorRenderSize(size));
        }
        entity.id()
    }
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
/// steers through the shared flight limb (archetype swap AS4). `is_hostile` /
/// `body_contact_damage` are false — boss offense flows through `BossAttackState`
/// + `boss_attack_damage`, never the actor melee/contact path; the boss is a
/// victim-side body here (the vulnerability trio rides in via the bundle below).
fn boss_actor_cluster(
    config: &BossConfig,
    kin: &BodyKinematics,
) -> (
    ambition_characters::actor::ai::ActorStatus,
    ambition_combat::actor_tuning::ActorConfig,
    ambition_platformer2d_shared_tangle::body::SpawnBaseline,
    super::actor_clusters::ActorMotionPath,
    ambition_platformer2d_core::body_clusters::ActorSurfaceState,
    ambition_combat::components::BodyMelee,
    ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle,
    ambition_platformer2d_core::movement::MotionModel,
    ambition_combat::CombatCapabilities,
    ambition_combat::CombatTuning,
) {
    // A boss floats: its movement kit grants flight, unioned into the body's
    // `AbilitySet` below. Death/weapon consequence traits stay default.
    let caps = ambition_combat::CombatCapabilities::default();
    let movement_kit = ae::AbilitySet {
        fly: true,
        //  PERMANENT flight, and the toggle was a lie that cost a boss its
        // legs. This said `fly_toggle: true` — "the toggled kind a boss has
        // always had" — but nothing ever toggles a boss: it spawns flying
        // (`ActorTuning::is_aerial` → `flight.fly_enabled`) and must stay
        // flying, because its `BossPattern` brain steers ONLY by commanding an
        // exact velocity, which only the flight limb reads.
        fly_toggle: false,
        ..ae::AbilitySet::NONE
    };
    // STRIKE offense is the frame-driven Boss hitboxes (`sync_boss_strike_hitboxes`), so
    // `is_hostile` (actor melee) stays off.
    let body_damage = config.behavior.body_damage;
    let tuning = ambition_combat::actor_tuning::ActorTuning {
        chase_speed: BOSS_FLIGHT_SPEED,
        max_run_speed: BOSS_FLIGHT_SPEED,
        is_aerial: true,
        // The BossPattern brain commands an exact per-tick velocity, so the flight
        // limb takes it verbatim (AS4c) — byte-identical to the old SNAP float.
        flight_direct_velocity: true,
        is_hostile: false,
        body_contact_damage: body_damage > 0,
        damage_amount: body_damage,
        contact_strength: 2.6,
        ..Default::default()
    };
    let weight = tuning.weight;
    let actor_config = ambition_combat::actor_tuning::ActorConfig {
        id: config.id.clone(),
        name: config.name.clone(),
        tuning,
        brain_profile: ambition_combat::actor_tuning::BrainProfile::default(),
        // The boss's REAL brain is its `BossPattern` `Brain` component. This
        // integrator-facing `CharacterBrain` only feeds patrol-stall intent, which
        // a free-flying boss never uses, so it takes the inert `Passive` row.
        brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
        sprite_override_npc_name: None,
        sprite_character_id: None,
        // A boss drives a `BossPattern`, never the fighter brain the trait picks
        // a stream for, and there is only ever one of it.
        preserves_mirror_symmetry: false,
    };
    (
        ambition_characters::actor::ai::ActorStatus {
            respawn_timer: 0.0,
            ai_mode: ambition_characters::actor::ai::CharacterAiMode::Idle,
        },
        actor_config,
        // A boss FLIES, so its authored gravity scale is 0.0 — the same value
        // its live surface state starts at, three lines down.
        ambition_platformer2d_shared_tangle::body::SpawnBaseline {
            pos: kin.pos,
            size: kin.size,
            gravity_scale: 0.0,
        },
        super::actor_clusters::ActorMotionPath::default(),
        ambition_platformer2d_core::body_clusters::ActorSurfaceState {
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale: 0.0,
        },
        ambition_combat::components::BodyMelee::default(),
        ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
            super::actor_clusters::ActorBody::from_kit(movement_kit, true, kin.size).0,
        ),
        // Every integrated body carries an explicit policy from spawn — the
        // boss is axis-swept (its direct-velocity flight rides the per-tick
        // axis-parameter refresh in `integrate_body`).
        ambition_platformer2d_core::movement::MotionModel::default(),
        caps,
        // Project the boss's weight onto the combat-owned carrier at spawn
        // (E2 verdict b); default `1.0` here since bosses don't author weight.
        ambition_combat::CombatTuning {
            weight,
            // Bosses pace strikes via their move scripts, and carry no sprite
            // catalog id (their strike volumes are frame-authored).
            attack_cooldown_mult: 1.0,
            sprite_character_id: None,
            // CM8: a struck boss reacts with the plain hurt profile (its death is
            // handled by the boss-death feedback, not this).
            hurt_feedback: ambition_vfx::HurtFeedback::ENEMY,
        },
    )
}

/// Populate a boss onto a root the construction executor allocated.
pub(crate) fn spawn_boss_with_overrides_into(
    commands: &mut Commands,
    boss_catalog: &BossCatalog,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_entity_catalog::placements::BossBrain,
    >,
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
        target: "ambition_platformer2d::boss_spawn",
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
    let boss_anim_frame = ambition_boss_encounter::sprites::BossAnimFrame::new(
        boss_catalog.sheet_for_key(&boss_sheet_key),
    );
    let combat_tuning =
        ambition_combat::feel::Platformer2dFeelTuningMonolith::default().feature_combat_tuning();
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
    let boss_capability = ambition_characters::brain::BossCapability::from_cfg(&brain_cfg);
    // First-seen telegraph window per profile — lets each strike move span the whole
    // telegraph→strike as one timeline (E53).
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
    let boss_combat = ambition_characters::actor::BodyCombat::default();
    let (boss_identity, boss_disposition) = boss_component_snapshot(boss.as_ref());
    let boss_facing = boss.kin.facing;
    // Kin/HP are NOT in this bundle — the boss owns those directly (§A1).  the pool is NOT
    // passed in (AC6.2). It was — as `boss.health.max()`, to fill an
    // `ActorTuning::max_health` that the boss's own `BodyHealth` already held. The boss owns
    // its health directly (§A1); handing it a copy of its own number was the duplicate this
    // slice removes.
    let boss_actor_cluster = boss_actor_cluster(&boss.config, &boss.kin);
    let boss_render_envelope = ambition_combat::BodyEnvelope(boss.as_ref().render_size());
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
            ambition_combat::components::ActorFaction::Boss,
            ambition_combat::components::ActorTarget::default(),
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
        boss_combat_kit,
        ActorAggression::hostile(),
    ));
    // Data-driven attack MOVESET: EVERY boss strike — geometry AND content-technique
    // special — runs through the SHARED moveset runtime (one move per profile), so the
    // boss's melee/special path is the actor's, retiring both `sync_boss_strike_hitboxes`
    // and `dispatch_boss_special` (§A1). Built from the capability repertoire.
    let boss_attack_moves = ambition_boss_encounter::attack_moveset::boss_attack_moveset(
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
        ambition_characters::control::ActorControl::default(),
        ambition_characters::brain::BossAttackState::default(),
        // §A1 intent/projection split: the driver-written fire INTENT the moveset
        // trigger reads (BossAttackState is now the projected read-model).
        ambition_characters::brain::BossAttackIntent::default(),
        boss_capability,
    ));
    if let Some(moveset) = boss_attack_moves {
        entity.insert(moveset);
    }
    entity.insert(boss_actor_cluster);
    // The coarse render footprint the shared integrator publishes the CenteredAabb
    // from (R1.1). Required by `integrate_boss_bodies`' query, so a boss without it
    // simply would not move — a loud failure the boss suites catch, not a silent
    // footprint shrink.
    entity.insert(boss_render_envelope);
    // Per-spawn tweaks Z: read at seed time by `update_boss_encounters`
    // (hp / size / phase triggers) + `sync_boss_encounter_entities`
    // (encounter opt-out). Default for room-authored bosses  no-op.
    entity.insert(overrides.clone());
    // ADR 0020: a boss authored as a would-be RIDER (non-empty
    // `pilotable_mount_classes`) becomes a `CanPilot` — the SAME mount-role tag
    // the enemy path attaches in `attach_mount_role`, so `spawn_boss` and
    // `spawn_solo_enemy` stay symmetric (a boss can board a `giant_gnu` mount).
    // `boss_attack_behavior` is a pre-`into_components` clone, still live here.
    // The `RidingOn`/`MountSlot` link is installed later by
    // the planned `ambition.mount` relation from the room's authored `mounted_on` refs.
    if !boss_attack_behavior.pilotable_mount_classes.is_empty() {
        entity.insert(ambition_mount::CanPilot {
            classes: boss_attack_behavior
                .pilotable_mount_classes
                .iter()
                .map(|c| ambition_mount::MountClass(c.clone()))
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
/// An id that names no prepared character fails construction, and fails it at PREPARATION
/// (`construction::preflight_planned_bodies`), so a refused summon batch has built nothing.
/// `half_size` is the spawn AABB half-extent. `id` should be unique per spawn so per-entity
/// systems don't collide on identity. `encounter_id` scopes the minion to a parent encounter so
/// room reset / boss despawn cleans it up alongside the boss.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_runtime_minion(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    session_scope: SessionSpawnScope,
    id: impl Into<String>,
    name: impl Into<String>,
    world_pos: ae::Vec2,
    half_size: ae::Vec2,
    character_id: &str,
    encounter_id: impl Into<String>,
    // Allegiance of the spawned minion. Boss adds pass `Enemy` +
    // `hostile_to_player`; the puppy-slug-gun passes `Player` + `passive` so the
    // summon damages the player's enemies (via the `can_damage` matrix) but never
    // the player, and just wanders rather than targeting.
    faction: ambition_combat::components::ActorFaction,
    aggression: ambition_combat::components::ActorAggression,
) -> bevy::ecs::entity::Entity {
    let root = commands.spawn_empty().id();
    spawn_runtime_minion_into(
        commands,
        catalog,
        authored_sheets,
        prepared,
        session_scope,
        root,
        id,
        name,
        world_pos,
        half_size,
        character_id,
        encounter_id,
        faction,
        aggression,
        // A boss minion keeps the vitals its character authored, hazard and all.
        None,
        true,
    );
    root
}

/// Populate a summoned minion onto a root the construction executor allocated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_runtime_minion_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    session_scope: SessionSpawnScope,
    entity: bevy::ecs::entity::Entity,
    id: impl Into<String>,
    name: impl Into<String>,
    world_pos: ae::Vec2,
    half_size: ae::Vec2,
    character_id: &str,
    encounter_id: impl Into<String>,
    faction: ambition_combat::components::ActorFaction,
    aggression: ambition_combat::components::ActorAggression,
    // Health for THIS occurrence, overriding the character's authored vitals.
    // See `ambition_vfx::SummonSpec::health`.
    health: Option<u32>,
    // Whether this occurrence keeps the character's authored contact hazard.
    keeps_contact_damage: bool,
) {
    let id = id.into();
    let name = name.into();
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(world_pos, half_size);
    let brain = ambition_entity_catalog::placements::CharacterBrain::Custom(character_id.into());
    // Summons name prepared characters. Preflight resolves the same registry
    // before reserving or constructing the batch, so reaching this failure means
    // preparation and construction disagreed.
    let Some(body) = prepared
        .get(character_id)
        .and_then(|prepared| prepared.body_blueprint().ok())
    else {
        panic!(
            "boss summon `{id}` names `{character_id}`, which is not a prepared \
             character that can build a body — and it reached construction, so \
             the row was never preflighted. A summon that resolved nothing used \
             to become a generic `combatant` silently; that is what cost the \
             Gradient Sentinel its minions, and the row it borrowed no longer \
             exists. Register the character."
        );
    };
    // ⛔⛔ THE MOUNT ROLE, ON THE THIRD ROAD THAT DROPPED IT.
    // `CharacterBodyBlueprint::mount` is right here and `new_character_in`
    // swallows it in a `..` — so a summoned `npc_burning_flying_shark`, whose
    // row authors `class: "shark"`, arrived with no `Mountable` and nobody could
    // board it. Placement reads the fact off its own definition and seating now
    // reads it off the prepared seat; this is the same fact on the road that
    // makes a body at runtime.
    //
    // ⚠ TAKEN BEFORE `body` IS MOVED into the seed below.
    let mount_role = body.mount.cloned();
    let mut enemy = super::actor_clusters::ActorClusterSeed::new_character_in(
        authored_sheets,
        catalog,
        id.clone(),
        body,
        aabb,
        brain,
        &[],
    );
    if reject_runtime_giant(
        is_limbed_host(prepared.get(character_id)),
        "runtime minion",
        &id,
    ) {
        return;
    }
    // `new_character_in` already set HP from the character's own vitals.
    //
    // ⭐ …AND THE SUMMONER MAY OVERRIDE IT, because the same creature is not the
    // same thing in two games. The burning flying shark authors 6 HP, which is
    // fair where it was written and is one connection in a platform fighter
    // whose move table runs 2–17 — and the pirate's up-B drops it exactly where
    // its rider is, which in a fight is exactly where the hits are. `None`
    // leaves the authored vitals alone, which is every summon that predates
    // this. See `ambition_vfx::SummonSpec::health`.
    if let Some(health) = health {
        enemy.health = ambition_characters::actor::BodyHealth::new(
            ambition_characters::actor::Health::new(health.max(1) as i32),
        );
    }
    // ⛔ AND THE SUMMONER MAY DECLINE THE CONTACT HAZARD. The shark's own
    // `ContactDamage` is right for the game it hunts in and wrong for a mount a
    // player rides through a fight. It has been inert only because a neutral
    // body acquires no target — a coincidence of the targeting rules, not a
    // statement — so a rule that says "no contact damage" says it here instead.
    if !keeps_contact_damage {
        enemy.config.tuning.body_contact_damage = false;
    }
    // Boss-spawned minions shouldn't auto-respawn — they're part of
    // the encounter, not a static sandbag.
    enemy.status.respawn_timer = 999_999.0;
    // ⛔⛔ AND ITS DEATH IS NOT WRITTEN DOWN. A summoned body inherits its
    // character's `RespawnPolicy`, which DEFAULTS to `DeadStaysDead` — and that
    // policy means "when this body dies, set the save flag `enemy_<id>_dead`
    // FOREVER". That is a statement about a PLACEMENT: one authored actor,
    // standing in one room, which the player killed. A summon is not a placement.
    // It has no room, it is made fresh every time somebody calls for it, and its
    // `config.id` is a FIXED STRING shared by every instance ever summoned.
    //
    // ⭐⭐ SO ONE DEATH POISONED EVERY LATER SUMMON, PERMANENTLY. The pirate's
    // recovery shark spawns as `smash_ride_shark`; the first one that died wrote
    // `enemy_smash_ride_shark_dead`, and `sync_ecs_actors_with_save` — which runs
    // EVERY SIM TICK, not on load — then zeroed `health.current` on the first
    // tick of every shark summoned afterwards, in that save, for good. The rider
    // boarded a body that was alive when it was built and dead one tick later,
    // and the log could only say "its health pool reached zero": no hit had
    // landed, so nothing upstream had anything to report. A fresh save rode
    // perfectly, which is why every test stayed green.
    //
    // ⭐ `OnRoomReenter` IS THE POLICY THAT MEANS "NOT PERSISTED": it writes no
    // flag on death and reads none on load. The two lines that made a summon
    // transient are now the two lines that say so — a timer that never fires and
    // a liveness nobody records.
    enemy.config.tuning.respawn = ambition_entity_catalog::placements::RespawnPolicy::OnRoomReenter;
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
        .insert(ambition_combat::components::EncounterMob::new(encounter_id));
    // The authored mount role captured above, on the body that now exists.
    if let Some(mount) = mount_role.as_ref() {
        attach_mount_role_from(
            commands,
            entity,
            mount.class.as_deref(),
            Some(aabb.half_size() * 2.0),
            mount.death_splash,
            1.0,
            &mount.pilotable_classes,
        );
    }
    if let Some(rs) = super::actor_clusters::sprite_render_size_for_name_in(
        authored_sheets,
        catalog,
        &name,
        aabb.half_size() * 2.0,
    ) {
        commands
            .entity(entity)
            .insert(ambition_combat::components::ActorRenderSize(rs));
    }
}

/// Populate an ordinary enemy onto a preallocated construction root. Giant
/// limbs are explicit construction rows and use the giant host/limb paths.
#[allow(clippy::too_many_arguments)]
/// Default for placements that do not author a respawn policy. Named actors use
/// their explicit policy; ordinary unspecified room bodies respawn on reentry.
pub(crate) const UNDESCRIBED_BODY_RESPAWN: ambition_entity_catalog::placements::RespawnPolicy =
    ambition_entity_catalog::placements::RespawnPolicy::OnRoomReenter;

pub(crate) fn spawn_enemy_with_faction_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    // The published controller policies, so this PLACEMENT may name one.
    // See `EnemySpawnSpec::brain_profile`.
    profiles: &ambition_characters::actor::character_catalog::BrainProfileRegistry,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::EnemySpawnSpec,
    >,
    paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    faction: ambition_combat::components::ActorFaction,
) {
    // The authored placement, lowered to the one plan every surface will
    // lower to (see `spawn::character_spawn_plan`). It owns the two questions
    // that are not the archetype's: WHICH character this instantiates, and who
    // drives it. Everything the seed still reads off `authored` below is what
    // phases 2–4 move onto the character.
    //  THE THREE OUTCOMES ARE TWO (AC6). This read *complete character →
    // built from it · unnamed or incomplete → the archetype builds it and the
    // character patches it · named but unprepared → a fault, and the archetype
    // keeps the body*. Two of those three arms end at an archetype, and there is
    // no archetype. So the question is no longer *which road builds this body*
    // but simply *which character is it*, and the answer is required.
    let character = authored.payload.gameplay_character_id();
    let plan = super::spawn::CharacterSpawnPlan::new(
        character,
        super::spawn::SpawnContext {
            feature_id: &authored.id,
            aabb: authored.aabb,
        },
    );
    //  the placement NAMED a character and construction cannot find it —
    // the Iron Mary case, and the reason this is a refusal rather than a
    // shrug: a body authored as Iron Mary and quietly built as a shark rider
    // looks exactly like a working spawn.
    let definition = plan.definition(prepared).unwrap_or_else(|missing| {
        panic!(
            "enemy `{}` names character `{missing}`, which this composition has \
             not registered — and it reached construction, so the row was never \
             preflighted (see `ActorConstructionError::BodyCharacterNotRegistered`)",
            authored.id,
        )
    });
    //  CHARACTER-FIRST, and there is no second road. A character's body is
    // built from its own facts and it WEARS itself, so its kit arrives through
    // the one writer every worn body uses.
    let body = definition.body_blueprint().unwrap_or_else(|missing| {
        panic!(
            "enemy `{}`: {missing}. It reached construction, so the row was \
             never preflighted (see \
             `ActorConstructionError::BodyCharacterIsIncomplete`)",
            authored.id,
        )
    });
    {
        let mut body = body;
        //  WHO DRIVES THIS ONE, if the placement said. The last of the
        // three authorities to become authorable at the placement: a character
        // states what a body IS, a `BrainProfile` states how a driver decides,
        // and until now only the character could name a profile — so one
        // creature had exactly one way to be played everywhere it appeared.
        //
        //  a name that resolves to nothing is a REFUSAL, the same contract
        // `CharacterDefinition::autonomous_profile_ref` carries. An explicit
        // reference that misses must never read as silence, or the level says
        // "guard this door" and the body patrols.
        if let Some(reference) = &authored.payload.brain_profile {
            let resolved = reference.resolve_in(definition.provider.as_str());
            match profiles.get(&resolved) {
                Some(profile) => body.autonomous_profile = Some(*profile),
                None => panic!(
                    "EnemySpawn `{}` names the controller profile `{reference}`, \
                     which resolves to `{resolved}` against character `{}`'s \
                     provider and is not published. Published: [{}]",
                    authored.id,
                    definition.id.as_str(),
                    profiles.ids().collect::<Vec<_>>().join(", "),
                ),
            }
        }
        let mut enemy = super::actor_clusters::ActorClusterSeed::new_character_in(
            authored_sheets,
            catalog,
            plan.context().feature_id.to_string(),
            body,
            plan.context().aabb,
            authored.payload.brain.clone(),
            paths,
        );
        // INITIAL ORIENTATION IS A PLACEMENT FACT. The character constructor
        // intentionally has no stage-direction opinion; seed the body's one
        // authoritative facing here, before the entity is spawned. Wanderer and
        // every other controller then consume ordinary body orientation rather
        // than learning a Mary-O/game-specific default.
        enemy.kin.facing = authored.payload.facing.sign();
        // The PLACEMENT's respawn policy — the one fact here that is neither
        // the character's nor the controller's (ADR 0022).
        //
        //  TWO authorities now, and the second is STATED rather than
        // borrowed. The PLACEMENT owns respawn policy (ADR 0022); a body whose
        // placement says nothing takes the engine's own answer for an undescribed
        // one. The middle authority — "whatever archetype row this brain key
        // happens to name" — is what AC6 deleted, and it was reached by a lookup
        // that could not fail. It answered `OnRoomReenter` for every body that
        // got this far, which is what `UNDESCRIBED_BODY_RESPAWN` says on purpose.
        enemy.config.tuning.respawn = authored.payload.respawn.unwrap_or(UNDESCRIBED_BODY_RESPAWN);
        // So the giant GNU, a mount whose authored profile states it never seeks anybody, was
        // handed its hostility back one line after construction resolved it correctly. A
        // placement may still overrule, which is what a disposition is for.
        enemy.config.tuning.is_hostile = authored
            .payload
            .disposition
            .map_or(enemy.config.tuning.is_hostile, |disposition| {
                disposition.is_hostile()
            });
        // What this body DOES when it dies, and what it may do — both the
        // character's, both already resolved on the definition.
        enemy.caps = ambition_combat::CombatCapabilities::from(
            &definition.death_traits.clone().unwrap_or_default(),
        );
        let body_size = enemy.kin.size;
        spawn_solo_enemy_into(
            commands,
            catalog,
            authored_sheets,
            session_scope,
            root,
            enemy,
            authored,
            faction,
        );
        //  IT WEARS ITSELF. The persona derive is the single writer for a
        // worn body's action set, moveset and identity baseline — the same one
        // that serves a match seat — so a migrated enemy's kit comes from its
        // character rather than from `enemy.spec.melee`.
        commands
            .entity(root)
            .insert(ambition_characters::actor::WornCharacter::new(
                definition.id.as_str(),
            ));
        // The body was built partial here, `WornCharacter` was attached, and
        // `project_prepared_character_definitions` noticed it a tick later and inserted the
        // action set, the moveset, the hurtboxes and the posed body onto a body that had
        // already begun simulating.
        //
        //  the memo goes on in the SAME batch (see
        // `grant_prepared_character_body`), so the re-template pass reads this
        // body as current and never touches it. That pass is now what it was
        // always for: a cast hot reload, or a deliberate runtime re-wear.
        crate::character_runtime::grant_prepared_character_body(
            commands,
            root,
            definition,
            prepared.generation(),
            crate::character_runtime::KitOwnership::Grant,
            // A room placement answers to no match: the character's own feel is
            // the whole answer here (see `MatchRules::body_over`).
            definition.movement_tuning,
        );
        // THE WEAPON THE CHARACTER CARRIES. The plan resolves its held item
        // from `enemy.spec`, which for a character-first body is inert — so a
        // migrated raider spawned empty-handed and dropped nothing when it died,
        // which is most of what a raider is. Inserted here for the same reason
        // the mount role is: the fact is the character's, and this is the road
        // that believes the character.
        //
        //  an id the registry does not know is a WARNING, not a refusal: the
        // body is fine without it, and a silent nothing is what made the archetype
        // path's typos invisible.
        if let Some(id) = definition.held_item.as_deref() {
            match ambition_characters::brain::held_item_by_id(id) {
                Some(spec) => {
                    commands.entity(root).insert(super::HeldItem::new(spec));
                }
                None => bevy::log::warn!(
                    "character `{}` holds `{id}`, which is not a registered held item",
                    definition.id.as_str()
                ),
            }
        }
        //  a body that states no mount gets no role, and that is the whole
        // rule now. The other arm read the archetype row this placement's brain
        // key happened to name — which for every migrated character was the
        // reserved `combatant` row, stating no mount class, so it was already
        // answering "nothing" by the longest available route.
        if let Some(mount) = definition.mount.as_ref() {
            attach_mount_role_from(
                commands,
                root,
                mount.class.as_deref(),
                Some(body_size),
                mount.death_splash,
                definition.vitals.mass.unwrap_or(1.0),
                &mount.pilotable_classes,
            );
        }
    }
    //  THE ARCHETYPE ROAD ENDED HERE. What stood below
    // was a SECOND constructor: resolve a row for the placement's brain key —
    // settling for the generic `combatant` when nothing matched — build the body
    // from it, then re-apply the placement's respawn and disposition because the
    // character-first road above had already done both on its own.
    //
    //  a body is built from a character.
}

/// One giant hand's fully-resolved construction facts, computed at PLAN time
/// from the giant's authored box — no `Entity`, no live world.
pub struct GiantHandPlan {
    pub slot: LimbSlot,
    /// Stable spawned identity under the giant, deterministic across runs.
    pub ordinal: u64,
    pub feature_id: String,
    /// Where the hand body starts, in world space.
    pub aabb: ae::Aabb,
    /// Host-local idle anchor (the `Limb::home_offset`), stated relative to the
    /// giant's center.
    pub home_offset: ae::Vec2,
}

/// Is this body a limbed `"giant"`-class host?
///
///  the roster-only form is what kept the giant chained to
/// `character_archetypes.ron`. A character may author
/// `CharacterMount { class: Some("giant") }` on its definition, and
/// `npc_giant_gnu` does; the planner could not see it, so deleting the row made
/// every giant a handless host (measured: 18 red tests, "host + two hands",
/// `left: 1, right: 3` — ).
///
///  AC6 removed the other half of the question rather than answering it.
/// This took an `Option<&ArchetypeSpec>` as well and fell back to it whenever the
/// character said nothing, with a companion `spec_is_limbed_host` for the runtime
/// paths that "hold a spec and no placement, so they have no character to ask".
/// Every one of those paths resolves a character now, so the fallback and the
/// companion are both gone and there is one predicate again.
///
///  still scoped to the `"giant"` string. A data-driven "which mounts have
/// limbs" flag waits for a SECOND limbed mount.
pub(crate) fn is_limbed_host(
    character: Option<&crate::character_runtime::PreparedCharacterDefinition>,
) -> bool {
    character
        .and_then(|definition| definition.mount.as_ref())
        .is_some_and(|mount| mount.class.as_deref() == Some("giant"))
}

/// Refuse a `"giant"`-class archetype on a runtime hostile-spawn path.
///
/// Summon effects, encounter waves, and runtime minions do NOT go through the
/// construction planner, so they cannot lower a giant into its host + two hand
/// rows (the only shape that gives a giant its rig — see [`giant_cluster_rows`]).
/// Rather than silently produce a handless giant, these origins refuse a
/// `"giant"`-class spec outright and log why. Authored and provider-staged giants
/// ARE supported; they lower through the planner. Returns `true` (having logged)
/// when the caller should skip the spawn.
pub(crate) fn reject_runtime_giant(
    //  the ANSWER, not the evidence. This took an
    // `Option<&ArchetypeSpec>` and asked the question itself, which forced every
    // caller to answer it the same way — and the two staged callers had a
    // CHARACTER available and no way to spend it. A refusal that only refuses
    // lets each origin ask in the terms it actually has: the staged paths ask
    // `is_limbed_host` with their character, the summon and encounter paths ask
    // `spec_is_limbed_host` because a spec is all they hold.
    //
    //  `false` is still the right answer for a character-first body with no
    // archetype at all: the giant's limbs are planned rows, and such a body
    // reaches the world through the planner, which is the road this protects.
    is_limbed_host: bool,
    origin: &str,
    id: &str,
) -> bool {
    if is_limbed_host {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "{origin} refuses `{id}`: a \"giant\"-class actor carries a limb rig and is only \
             constructible through the planner (authored or provider-staged); refusing rather than \
             spawning a handless giant"
        );
        return true;
    }
    false
}

pub(crate) fn giant_hand_plans(giant_id: &str, giant_aabb: ae::Aabb) -> Vec<GiantHandPlan> {
    //  the giant's own placement decides the hand geometry. This took an
    // `Option<&ArchetypeSpec>` and preferred that row's `default_size`; callers
    // handed it the reserved `combatant` fallback purely to satisfy the
    // signature, so the hands of a body with no archetype were sized by an
    // archetype. No surviving row authored one, so this is the same geometry
    // with the pretence removed.
    let giant_half = giant_aabb.half_size();
    let giant_center = giant_aabb.center();
    let hand_size = ae::Vec2::new(giant_half.x * 0.7, giant_half.y * 0.7);
    let home_l = ae::Vec2::new(-giant_half.x * 0.55, giant_half.y * 0.15);
    let home_r = ae::Vec2::new(giant_half.x * 0.55, giant_half.y * 0.15);
    [
        (LimbSlot::HAND_LEFT, home_l, "left"),
        (LimbSlot::HAND_RIGHT, home_r, "right"),
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

/// It deliberately takes `giant_id: &str`, never an `Entity`: the old form used `giant.index()`
/// (an allocator slot), which handed the hands a different `SimId` every run and broke
/// snapshot/replay determinism (netcode.md N3.2 boss-hand residual).
fn giant_hand_feature_id(giant_id: &str, side: &str) -> String {
    format!("giant_gnu_hand_{side}_{giant_id}")
}

/// The mount role, from values.
///
///  it had a sibling that took an `ArchetypeSpec` and unpacked the same seven values off it;
/// AC6 deleted the sibling with the rows, so a CHARACTER that states it is rideable is the only
/// thing that produces a `Mountable` ( group A: the shark family).
#[allow(clippy::too_many_arguments)]
fn attach_mount_role_from(
    commands: &mut Commands,
    entity: bevy::ecs::entity::Entity,
    mount_class: Option<&str>,
    default_size: Option<ae::Vec2>,
    death_splash: Option<i32>,
    mass: f32,
    pilotable: &[String],
) {
    if let Some(class) = mount_class {
        // Saddle offset heuristic: the rider sits just above the mount's top.
        // Feel-tunable; a mount that wants a precise saddle can grow a field.
        let mount_size = default_size.unwrap_or(ae::Vec2::new(64.0, 64.0));
        let rider_offset = ae::Vec2::new(0.0, -(mount_size.y * 0.5 + 40.0));
        commands.entity(entity).insert((
            ambition_mount::Mountable {
                rider_offset,
                class: ambition_mount::MountClass(class.to_string()),
                control_grant: ambition_mount::ControlGrant::Total,
                death_impact: match death_splash {
                    Some(amount) => ambition_mount::MountDeathImpact::Splash(amount),
                    None => ambition_mount::MountDeathImpact::Dismount,
                },
            },
            // A heavy mount keeps the pair's center of gravity near itself, so
            // the lighter rider orbits it under a gravity flip (sync reads Mass).
            ambition_platformer2d_shared_tangle::body::Mass(mass),
        ));
    }
    if !pilotable.is_empty() {
        commands.entity(entity).insert((
            ambition_mount::CanPilot {
                classes: pilotable
                    .iter()
                    .cloned()
                    .map(ambition_mount::MountClass)
                    .collect(),
            },
            ambition_platformer2d_shared_tangle::body::Mass(mass),
        ));
    }
}

/// Single-entity hostile spawn — the common path after composite
/// mount/rider fan-out has been handled. Returns the spawned body entity.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_solo_enemy_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    session_scope: SessionSpawnScope,
    entity: bevy::ecs::entity::Entity,
    enemy: super::actor_clusters::ActorClusterSeed,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::EnemySpawnSpec,
    >,
    faction: ambition_combat::components::ActorFaction,
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
        authored_sheets,
        catalog,
        &authored.name,
        authored.aabb.half_size() * 2.0,
    ) {
        commands
            .entity(entity)
            .insert(ambition_combat::components::ActorRenderSize(rs));
    }
}
/// Human label for an authored NPC: the catalog `display_name` for the
/// spawn's `character_id`, falling back to the authored world-IR name.
///
/// The character an NPC placement names, if it names one.
pub(crate) fn npc_character_id(interactable: &ambition_interaction::Interactable) -> Option<&str> {
    match &interactable.kind {
        ambition_interaction::InteractionKind::Npc { character_id, .. } => character_id.as_deref(),
        _ => None,
    }
}

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

pub(crate) fn spawn_interactable_into(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::InteractableSpec,
    >,
    paths: &[(String, ambition_platformer2d_core::KinematicPath)],
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
            authored_sheets,
            prepared,
            format!("Feature actor npc: {label}"),
            feature_aabb,
            authored.id.clone(),
            label,
            authored.aabb,
            interactable.clone(),
            paths,
        )
        .spawn_into(commands, session_scope, root);
    } else if let ambition_interaction::InteractionKind::Custom(payload) = &interactable.kind {
        if let Some(activation) = ambition_encounter::SwitchActivation::parse_custom(payload) {
            commands.insert_session_scoped(
                session_scope,
                root,
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
        } else {
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "interactable `{}` carries an unparseable Custom payload; the row will fail \
                 boundary verification",
                authored.id
            );
        }
    }
}

/// One encounter wave mob, as the wave director describes it.
///
///  the three questions a body's identity answers, and they are separate.
/// The vocabulary is deliberately [`ambition_platformer2d_world::rooms::EnemySpawnSpec`]'s, the
/// neighbouring spawn path, so the two structs read against each other:
///
/// | question | here | `EnemySpawnSpec` |
/// |---|---|---|
/// | what it LOOKS LIKE | `character` | `character_id` |
/// | what it DOES | `brain` | `brain` |
/// | which BODY | `id` | the authored placement's own id |
///
///  a struct rather than five more positional arguments, because the
/// interesting value here is `character: None` — and a bare `None` in argument
/// position 8 tells a reader nothing about which of three questions was
/// declined.
pub struct EncounterMobSeed<'a> {
    /// WHICH BODY. Minted per spawn by the wave director
    /// (`encounter:<trigger>:w<wave>:<n>`) so ids never collide across attempts,
    /// and the key the encounter's own `FeatureId` liveness refresh looks a mob
    /// up by.  never the character: two goblins in one wave are two bodies.
    pub id: String,
    /// WHAT IT LOOKS LIKE. A catalog character id — art only, exactly as far
    /// as [`ambition_platformer2d_world::rooms::EnemySpawnSpec::character_id`] reaches: the sheet, the
    /// sprite-derived collision box, hurt feedback, and the display label its
    /// banners and barks are keyed by.  it does NOT select the catalog's
    /// `default_brain` or `default_action_set` — `brain` below does that, and
    /// whether an enemy IS a character or merely WEARS one is an open design
    /// question, not something this field quietly answers.
    ///
    /// `None` is the older road and stays open: an encounter assembled from LDtk
    /// `EnemySpawn` markers that name no `character_id` has no character to give.
    pub character: Option<&'a str>,
    /// WHAT IT DOES. The roster archetype key, as
    /// `CharacterBrain::Custom(kind)` — health, speed, reach, melee/ranged kit.
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// Spawn centre, world space.
    pub pos: ae::Vec2,
    /// Body size. A HINT: a named character resizes to its authored sprite's
    /// collision, the same as a peaceful NPC of that character.
    pub size: ae::Vec2,
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub(super) fn spawn_encounter_mob(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    // The prepared cast — the body authority for every encounter mob that
    // names a character. `kind` is controller policy, not an alternate body key.
    prepared: &crate::character_runtime::PreparedCharacterRegistry,
    session_scope: SessionSpawnScope,
    encounter_id: impl Into<String>,
    mob: EncounterMobSeed<'_>,
) {
    let EncounterMobSeed {
        id,
        character,
        brain,
        pos,
        size,
    } = mob;
    let encounter_id = encounter_id.into();
    let aabb = ae::Aabb::new(pos, size * 0.5);
    //  the DISPLAY name is what the renderer binds a sheet from.
    // `rebuild_actor_render_index` publishes `ActorConfig::name`, and `upgrade_actor_sprites`
    // resolves name-first against the character registry — it never reads the seed's resolved
    // `sprite_character_id`.
    let label = character.map_or_else(
        || id.clone(),
        |character_id| match catalog.display_name(character_id) {
            Some(display_name) => display_name.to_string(),
            None => {
                warn!(
                    character_id,
                    mob_id = id.as_str(),
                    "encounter mob names a character with no catalog row; it will draw the \
                     unclaimed-body placeholder"
                );
                id.clone()
            }
        },
    );
    // The character is the body authority. A wave that names a character
    // must resolve a prepared definition whose body blueprint is complete. A
    // missing/unregistered/incomplete character is a content fault handled by the
    // refusal below; `kind` never substitutes another body.
    let prepared_character = match character {
        None => None,
        Some(character_id) => match prepared.get(character_id) {
            Some(definition) => Some(definition),
            None => {
                //  A `report_unprepared_character(.., Some("its archetype"))` STOOD HERE AND
                // WAS A LIE (AC6). It warned rather than refused, on the reasoning that *"a
                // wave always has an archetype to fall back to — `new_in` resolves one from the
                // roster"*. This road refuses in its own words instead; the shared rule is for
                // the one road that still HAS a fallback to name.
                None
            }
        },
    };
    let definition = prepared_character.filter(|definition| definition.body_blueprint().is_ok());
    // Nothing failed, because a fallback IS a body.
    //
    //  AC6 removed the fallback rather than the silence.
    let mut enemy = match definition {
        Some(definition) => {
            let mut enemy = super::actor_clusters::ActorClusterSeed::new_character_in(
                authored_sheets,
                catalog,
                // The instance identity.  NOT the character: two goblins in one
                // wave are two bodies, and every id-keyed index in the actor
                // runtime — the encounter's own `FeatureId` liveness lookup
                // included — would collapse them into one.
                id.clone(),
                definition
                    .body_blueprint()
                    .expect("the filter above kept only character-complete definitions"),
                aabb,
                brain,
                &[],
            );
            enemy.caps = ambition_combat::CombatCapabilities::from(
                &definition.death_traits.clone().unwrap_or_default(),
            );
            enemy
        }
        None => panic!(
            "encounter wave mob `{id}` is of kind `{brain:?}` and names no \
             character that can build a body — either the wave names none, or \
             the one it names is unregistered or cannot state how it moves. The \
             wave used to fill with generic `combatant` bodies and read as a \
             working encounter."
        ),
    };
    if reject_runtime_giant(
        is_limbed_host(character.and_then(|id| prepared.get(id))),
        "encounter wave",
        &id,
    ) {
        return;
    }
    // `new_character_in` already set HP from the character's own vitals.
    // Encounter mobs should not auto-respawn like training sandbags.
    enemy.status.respawn_timer = 999_999.0;
    let feature_aabb = CenteredAabb::from_center_size(pos, size);
    let entity = EnemyActorSpawnPlan::hostile(
        format!("Encounter mob: {id}"),
        id.clone(),
        label,
        feature_aabb,
        enemy,
    )
    .spawn(commands, session_scope);
    // The mob wears its authored character, so identity/kit state follows the
    // same character seam used by the other body-construction roads.
    if let Some(definition) = definition {
        commands
            .entity(entity)
            .insert(ambition_characters::actor::WornCharacter::new(
                definition.id.as_str(),
            ));
    }
    commands
        .entity(entity)
        .insert(EncounterMob::new(encounter_id));
    if let Some(rs) = super::actor_clusters::sprite_render_size_for_name_in(
        authored_sheets,
        catalog,
        character.unwrap_or(&id),
        size * 0.5 * 2.0,
    ) {
        commands
            .entity(entity)
            .insert(ambition_combat::components::ActorRenderSize(rs));
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
/// One minion is a small plan, and running it through the same planner as a room's contents is
/// the point rather than an overhead: it is what gives a summoned body a real dynamic identity
/// (`SimId::spawned` under its summoner, taken from the summoner's own `SimIdCounter`) and an
/// explicit [`SpawnOrigin::Dynamic`] naming its parent.
///
/// A summon without a summoner `SimId` is skipped because dynamic identities
/// require an explicit parent provenance.
/// One summoner's reserved stretch of its own identity sequence.
///
/// Carries the value planning READ as well as the value it wants to write, so
/// applying the reservation can tell "nothing moved" from "someone else spent
/// these ids while this batch was in flight".
struct SummonerSequenceReservation {
    summoner: ambition_platformer2d_shared_tangle::sim_id::SimId,
    /// What the counter held when this batch planned against it.
    expected: u64,
    /// What it must hold afterwards — `expected` plus one per summon reserved.
    next: u64,
}

impl SummonerSequenceReservation {
    /// Whether this summoner's counter still holds what planning assumed.
    fn still_valid(
        &self,
        counter: Option<&ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    ) -> bool {
        counter.is_some_and(|counter| counter.0 == self.expected)
    }
}

pub fn apply_summon_effects(
    mut commands: bevy::prelude::Commands,
    mut requests: bevy::prelude::MessageReader<ambition_vfx::EffectRequest>,
    character_catalog: bevy::prelude::Res<CharacterCatalog>,
    authored_sheets: bevy::prelude::Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    // `Option` like every other reader of it: a composition with no registered characters is
    // ordinary, not degraded.
    prepared_characters: Option<
        bevy::prelude::Res<crate::character_runtime::PreparedCharacterRegistry>,
    >,
    boss_catalog: bevy::prelude::Res<BossCatalog>,
    recipes: bevy::prelude::Res<crate::construction::ActorConstructionRegistry>,
    active_session: Option<bevy::prelude::Res<ActiveSessionScope>>,
    identities: bevy::prelude::Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    // Read-only: the advance is a queued command, not a direct write.
    counters: bevy::prelude::Query<&ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
) {
    use ambition_platformer2d_shared_tangle::construction::{ConstructionPlan, ConstructionScope};

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
    //  This is an ordered command, NOT rollback atomicity: the commands are
    // applied in sequence at the next flush, and nothing un-applies the earlier
    // ones if a later one finds its precondition violated. What it buys is that
    // a REFUSAL costs nothing and a violation is loud instead of silent.
    let mut reservations: std::collections::BTreeMap<
        bevy::prelude::Entity,
        SummonerSequenceReservation,
    > = std::collections::BTreeMap::new();
    let mut planned = Vec::new();
    // (rider, the mount's derived identity) for the summons that asked to be
    // ridden. Resolved to entities only after the commit flush.
    let mut board_after_commit: Vec<(
        bevy::prelude::Entity,
        ambition_platformer2d_shared_tangle::sim_id::SimId,
        ambition_vfx::SummonedRide,
    )> = Vec::new();
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
                target: "ambition_platformer2d::construction",
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
        // ⭐ THE MOUNT'S IDENTITY IS KNOWN BEFORE IT EXISTS. `SimId::spawned` is
        // what the request below derives, so a summon that asked to be ridden
        // can name its mount now and look it up after the commit — no channel,
        // no follow-up tick, and nothing that could name a DIFFERENT body.
        if let Some(ride) = s.ridden_by_summoner {
            board_after_commit.push((
                req.owner,
                ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(summoner, taken),
                ride,
            ));
        }
        planned.push(crate::construction::summoned_minion_request(
            summoner,
            taken,
            crate::construction::SummonedMinionParams {
                health: s.health,
                keeps_contact_damage: s.keeps_contact_damage,
                feature_id: s.id.clone(),
                name: s.name.clone(),
                pos: s.pos,
                half_size: s.half_size,
                character_id: s.character_id.clone(),
                encounter_id: s.encounter_id.clone(),
                faction: ambition_combat::actor_faction_from_hit_side(s.faction),
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
        binding: ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
        room: None,
    };
    let services = crate::construction::ActorConstructionServices {
        context: {
            let context = crate::world::placements::ActorPlacementContext::new(
                &character_catalog,
                &authored_sheets,
            );
            match prepared_characters.as_deref() {
                Some(prepared) => context.with_prepared(prepared),
                None => context,
            }
        },
        boss_catalog: boss_catalog.clone(),
    };

    // Every minion's body is proved buildable before the batch is planned.
    // A summon that resolves nothing REFUSES — and after AC6 that refusal is the
    // only outcome, because there is no generic body left to settle for. It
    // belongs here rather than inside the recipe: a rejected batch has spent
    // nothing, where a recipe-time refusal is a panic with rows already built.
    if let Err(error) =
        crate::construction::preflight_planned_bodies(&planned, prepared_characters.as_deref())
    {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "summon batch rejected before mutation: {error}"
        );
        return;
    }
    // Planning stays out here, against the App's own registry, and stays pure:
    // a rejected batch has spent nothing and built nothing.
    let live: std::collections::BTreeSet<_> = identities.iter().cloned().collect();
    let plan = match ConstructionPlan::prepare(scope.clone(), planned, &live, &recipes) {
        Ok(plan) => plan,
        Err(error) => {
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "summon batch rejected before mutation: {error}"
            );
            return;
        }
    };

    // The counter check, the construction, and the advance then happen inside
    // ONE exclusive-world command, so nothing can spend this summoner's
    // identities between the check and the spawn.
    //
    //  Atomicity of DECISION, not rollback. Bevy commands do not un-apply. There is
    // consequently no `max()` recovery path: by the time the advance runs, the value it is
    // replacing has just been read under the same lock.
    commands.queue(move |world: &mut bevy::prelude::World| {
        use ambition_platformer2d_shared_tangle::sim_id::SimIdCounter;

        for (owner, reservation) in &reservations {
            let counter = world.get::<SimIdCounter>(*owner);
            if !reservation.still_valid(counter) {
                bevy::log::error!(
                    target: "ambition_platformer2d::construction",
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
            let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
                commands: &mut commands,
                scope: &scope,
                session: session_scope,
                services: &services,
            };
            plan.commit(&mut ctx);
        }
        world.flush();

        // ⭐⭐ CONSTRUCTION RESERVES THE MOUNT; IT DOES NOT BOARD IT. This used
        // to weld the rider here, inside the same exclusive command that built
        // the body — the only moment both were in hand — and install the lease
        // in the same breath. That was right while a summoned mount appeared on
        // top of its summoner and wrong the moment it has to travel to them.
        //
        // ⛔ AND IT WAS NEVER THE ATOMIC TRANSACTION ITS COMMENT CLAIMED. A
        // refused board left the freshly-built mount standing in the world with
        // no `MountSlot`, which every cleanup path filters on, so nothing could
        // see it — a GPT review named this and Jon hit it in play. Now the
        // reservation is the whole of what construction owes: it either becomes
        // a ride or becomes a `RideRefused`, and `board_reserved_mounts` owns
        // both endings.
        //
        // ⚠ THE SUMMONER'S IDENTITY IS KNOWN BEFORE THE BODY EXISTS —
        // `SimId::spawned` is what the request below derives — so the
        // reservation can name its rider without a channel or a follow-up tick.
        for (rider, mount_id, ride) in board_after_commit {
            let mount = {
                let mut q = world.query::<(
                    bevy::prelude::Entity,
                    &ambition_platformer2d_shared_tangle::sim_id::SimId,
                )>();
                q.iter(world)
                    .find(|(_, id)| **id == mount_id)
                    .map(|(entity, _)| entity)
            };
            match mount {
                Some(mount) => {
                    bevy::log::info!(
                        target: "ambition::mount",
                        "summon built, reserved for its summoner: mount={mount:?} rider={rider:?}",
                    );
                    world
                        .entity_mut(mount)
                        .insert(ambition_mount::MountReservedFor {
                            rider,
                            lease_seconds: ride.seconds,
                            board_within: ride.board_within,
                            expires_in: ride.board_deadline_s,
                        });
                }
                None => bevy::log::warn!(
                    target: "ambition_platformer2d::construction",
                    "summon `{mount_id:?}` asked to be ridden but no body with that identity \
                     exists after the commit",
                ),
            }
        }

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
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// The old form derived the `_N` suffix from `giant.index()`, an allocator slot: this pins
    /// that the suffix is now the authored id instead.
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
    /// `SimId::spawned(giant_placement, ordinal)` — not the authored `placement:` namespace.
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

#[cfg(test)]
mod runtime_giant_refusal_tests {
    use super::*;
    use bevy::prelude::{App, Update};

    #[test]
    fn a_refused_programmatic_giant_allocates_no_entity() {
        let mut app = App::new();
        app.add_message::<SpawnActorRequest>();
        //  the giant is a CHARACTER that states its mount class — the
        // only authority on limbs since AC6, where this used to publish a
        // two-row roster whose `test_giant` declared `mount_class:
        // Some("giant")`.
        app.insert_resource(giant_cast());
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        app.init_resource::<ambition_boss_encounter::BossCatalog>();
        app.init_resource::<ActiveSessionScope>();
        app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.add_systems(Update, apply_spawn_actor_requests);

        let before = app.world().entities().len();
        app.world_mut().write_message(SpawnActorRequest {
            id: "giant_0".to_string(),
            name: "Runtime Giant".to_string(),
            pos: ae::Vec2::ZERO,
            half_size: ae::Vec2::new(16.0, 16.0),
            faction: ambition_combat::components::ActorFaction::Enemy,
            grudge_against: None,
            kind: SpawnActorKind::Enemy {
                brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                    "test_giant".to_string(),
                ),
                character: ambition_entity_catalog::CharacterId::from("test_giant"),
            },
        });
        app.update();
        assert_eq!(
            app.world().entities().len(),
            before,
            "a refused giant request must allocate NOTHING — an empty root \
             would rejoin the grudge map as a phantom spawned enemy"
        );
    }

    /// A cast of one `"giant"`-class limbed host, which is what the refusal
    /// above reads.
    fn giant_cast() -> crate::character_runtime::PreparedCharacterRegistry {
        let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "test_giant",
            "Test Giant",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 0.0,
            ..Default::default()
        });
        definition.vitals.max_health = Some(2);
        definition.mount = Some(ambition_characters::actor::CharacterMount {
            class: Some("giant".to_string()),
            ..Default::default()
        });
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        registry.insert_prepared(finalized.prepared);
        registry
    }
}
