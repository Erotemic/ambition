//! Post-GGRS-load reconciliation of an actor's **autonomous configuration** from
//! its restored [`BrainBinding`] source.
//!
//! The rollback contract registers the small, stable facts (the [`BrainBinding`] source,
//! disposition, health, gravity) but NOT the whole archetype config an actor
//! carries — its `ActorConfig` tuning / brain-spec, `CombatCapabilities`, and
//! `ActionSet`. That config is a deterministic function of the autonomous source
//! plus the actor's durable combat kit, so rather than duplicate it, a rewind
//! RECONSTRUCTS it here, the same way spawn / provocation build it live. This is
//! what makes provocation rollback-correct in BOTH directions:
//!
//! - Rewind INTO a provoked snapshot ([`AutonomousSource::Provoked`]): rerun
//!   the roster archetype construction ([`project_provoked_archetype`], shared
//!   with the live provoke flip) to rebuild the hostile brain / action set /
//!   tuning / capabilities from the archetype id the binding retained.
//! - Rewind to BEFORE a challenge (a catalog source over a still-hostile config):
//!   restore the peaceful catalog config the character spawned with.
//!
//! The live `Brain` for a CATALOG source is rebuilt by the catalog pass
//! (`ambition_platformer2d_runtime::rollback::reconcile_brain_bindings`, which runs first); this pass
//! owns the coupled CONFIG for catalog sources and the whole autonomous state for
//! provoked sources. Registered facts (disposition, health, gravity) are restored by
//! their own GGRS strategies — this pass never overwrites them.
//!
//! Bodies under temporary control (player possession / mount) are skipped; their
//! control is reconciled separately (see the temporary-control reconcile pass).

use bevy::prelude::*;

use super::actor_clusters::ActorConfig;
use super::mount::{MountSlot, Mounted, MountedBrainCache, RidingOn};
use super::{CombatKit, HeldItem};
use crate::abilities::traversal::possession::PossessionState;
use crate::combat::CombatCapabilities;
use crate::features::ecs::actor_tuning::{ActorTuning, BrainProfile};
use crate::features::enemies::ArchetypeSpecExt;
use crate::features::enemies::{ArchetypeSpec, CharacterRoster};
use crate::features::TemporaryControl;
use ambition_characters::actor::character_catalog::{
    AuthoredBrainContext, AutonomousSource, BrainBinding, BrainBuildContext, CharacterBodyKind,
    CharacterCatalog,
};
use ambition_characters::actor::pose::ActorPose;
use ambition_characters::actor::{BodyHealth, Health};
use ambition_characters::brain::{ActorControl, Brain, PlayerSlot, NPC_PATROL_SPEED};
use ambition_entity_catalog::placements::CharacterBrain;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// **What provocation produces: a MIND and a KIT. Never a body.**
///
/// ⛔⛔ **THIS USED TO REPLACE THE CREATURE.** A peaceful body that got struck
/// had its tuning, its gravity scale, its HP pool, its combat capabilities and
/// its sprite override overwritten from the `combatant` archetype row — so a
/// provoked villager did not become an angry villager, it became a `combatant`
/// wearing a villager's name. The comment three lines above the code that did it
/// already stated the correct invariant: *"provocation is one body, a different
/// driver, a changed relationship. The body stays exactly as its character built
/// it."* It was describing the OTHER branch.
///
/// ⇒ what a provocation may change is the POLICY the body is driven by, the KIT
/// it swings if it has none of its own, and its relationship to whoever struck
/// it. Its speed, its locomotion, its capabilities and its silhouette are facts
/// about the creature, and being hit is not an argument about any of them.
///
/// ⚠ **and the brain is lowered against the BODY's tuning now**, not the
/// archetype's — §4.7, a policy states normalized effort and the body states the
/// speed. A provoked villager chases at a villager's top speed, which is the
/// same sentence as the paragraph above with the consequence attached.
///
/// Both the live provoke flip (`provoke_actor_in_place`) and the post-restore
/// reconstruction apply this exact projection, so a provoked actor is identical
/// whether it was just challenged or rebuilt after a GGRS load.
pub(crate) struct ProvokedArchetype {
    pub brain_profile: BrainProfile,
    /// **THE ONE BODY FACT STILL PROJECTED, and it is a decision rather than a
    /// design.**
    ///
    /// A peaceful NPC placement spawns at `max_health: 1` — the generic
    /// stroller seed — so a provoked one that kept its own pool would die to a
    /// single hit. What a provoked villager's health pool should be is **D96
    /// item 7**, open and Jon's. Until it is answered this borrows the archetype
    /// row's number, and it is the last thing generic provocation takes from
    /// one.
    ///
    /// ⛔ do not quietly widen this back out. Every other field that used to sit
    /// beside it is gone on purpose; a second one reappearing is the ontology
    /// growing back.
    pub max_health: i32,
    /// **THE SECOND, and it is a MISMATCH being patched rather than a fact.**
    ///
    /// A body at `gravity_scale: 0` driven by a GROUNDED policy freezes: the
    /// aerial integrator reads a `velocity_target` the grounded brain never
    /// sets. So provocation re-grounds a floating body, which is a body change,
    /// and `a_floating_npc_grounds_when_provoked_into_a_grounded_archetype`
    /// pins it.
    ///
    /// ⛔ **the guard was probed rather than dropped.** GPT 5.6's redirect says
    /// no gravity replacement, and it is right about the direction — but the
    /// freeze is real, and deleting the write on the strength of an argument
    /// would ship it. The actual defect is one level up: a generic provocation
    /// hands every body the same GROUNDED policy, so a flying creature is given
    /// a mind that cannot drive it. Fixing THAT deletes this field; forcing the
    /// body to match the policy is the ontology arguing back.
    ///
    /// ⚠ the example this used to cite is already gone: the Perfect Cellular
    /// Automaton "floats peacefully, then descends to brawl" and now authors
    /// `baseline_free_flight: Some(false)` itself (D89). What remains reachable
    /// is a body whose CHARACTER says it flies — the parrot, the burning shark.
    pub gravity_scale: f32,
    /// The `ActorConfig.brain` read-model marker for a provoked actor.
    pub config_brain: CharacterBrain,
    pub brain: Brain,
    pub action_set: ambition_characters::brain::ActionSet,
}

/// Project a hostile roster archetype's POLICY onto an actor. Pure: no ECS, no
/// mutation — the single definition of "what provocation produces", so the live
/// flip and a snapshot rebuild can never drift.
///
/// `current_config` is the actor's config at call time, and it is the body the
/// policy is lowered against. The projection clones it to swap in the hostile
/// `brain_profile` and nothing else.
pub(crate) fn project_provoked_archetype(
    spec: &ArchetypeSpec,
    archetype: &str,
    current_config: &ActorConfig,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    // The provoked body's LIVE verbs. Provocation swaps the driver, never the
    // body: a peaceful pirate that gets struck becomes hostile with exactly the
    // abilities it already had, and the driver may only reach for those.
    body: ambition_platformer2d_core::AbilitySet,
) -> ProvokedArchetype {
    provoked_projection(
        spec.brain_profile(),
        spec.max_health,
        spec.tuning().is_aerial,
        archetype,
        current_config,
        combat_kit,
        held_item,
        body,
    )
}

/// **The projection itself, from a POLICY rather than from a row.**
///
/// ⭐ the live generic-provocation path calls this with
/// [`default_provoked_policy`](super::brain_builders::default_provoked_policy)
/// and [`DEFAULT_PROVOKED_HEALTH`](super::brain_builders::DEFAULT_PROVOKED_HEALTH),
/// so provoking a body no longer touches the archetype roster at all — that
/// lookup was the last reason the live path knew the ontology existed.
/// [`project_provoked_archetype`] above is the rollback road's entry, which
/// still resolves the archetype id a binding recorded.
///
/// ⚠ the two are pinned equal while `combatant` survives
/// (`an_engine_default_provoked_policy_matches_the_combatant_row`); when the row
/// goes, this signature is already the one that stays.
#[allow(clippy::too_many_arguments)]
pub(crate) fn provoked_projection(
    brain_profile: BrainProfile,
    max_health: i32,
    aerial: bool,
    archetype: &str,
    current_config: &ActorConfig,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    body: ambition_platformer2d_core::AbilitySet,
) -> ProvokedArchetype {
    let config_brain = CharacterBrain::Custom(archetype.to_string());

    // ⭐ the POLICY is the archetype's; the BODY is the one that was struck.
    let mut hostile_config = current_config.clone();
    hostile_config.brain_profile = brain_profile;
    hostile_config.brain = config_brain.clone();
    let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
        &hostile_config,
        combat_kit,
        held_item,
        body,
    );

    ProvokedArchetype {
        // See the field doc: the POLICY is grounded, so a floating body has to
        // be grounded to be drivable by it. The mismatch is the bug.
        gravity_scale: if aerial { 0.0 } else { 1.0 },
        max_health,
        config_brain,
        brain,
        action_set,
        brain_profile,
    }
}

/// The fixed peaceful catalog config a catalog-backed NPC spawns with. Mirrors
/// `ActorClusterSeed::new_peaceful_npc_in`: a health-1 stroller with default
/// brain-spec / capabilities, its authored combat kit as body-capability action
/// set, and `is_aerial` from the character's catalog body kind. The only
/// non-constant input is `character_id` (for `is_aerial` + the resolved
/// `config.brain` read-model).
pub(crate) struct PeacefulConfig {
    pub(crate) tuning: ActorTuning,
    pub(crate) brain_profile: BrainProfile,
    pub(crate) capabilities: CombatCapabilities,
    pub(crate) action_set: ambition_characters::brain::ActionSet,
    pub(crate) config_brain: CharacterBrain,
}

pub(crate) fn peaceful_config(
    catalog: &CharacterCatalog,
    character_id: Option<&str>,
    combat_kit: &CombatKit,
    resolved_brain: &Brain,
) -> PeacefulConfig {
    let is_aerial = character_id
        .map(|cid| matches!(catalog.body_kind(cid), Some(CharacterBodyKind::Floating)))
        .unwrap_or(false);
    let tuning = ActorTuning {
        max_health: 1,
        patrol_speed: NPC_PATROL_SPEED,
        chase_speed: NPC_PATROL_SPEED,
        max_run_speed: ambition_platformer2d_core::MAX_RUN_SPEED,
        is_aerial,
        // STATED, matching the spawn seed this mirrors: an NPC placement is a
        // person, so its death is permanent (ADR 0022). Rewinding a provoked
        // actor back to peaceful must restore that policy, not a default that
        // happens to agree.
        respawn: ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
        ..Default::default()
    };
    // `config.brain` (the integrator read-model) is DERIVED from the resolved
    // autonomous brain through the SHARED helper the spawn plan and runtime switch
    // both use, so the classification can never disagree with the actual brain.
    let config_brain = crate::features::brain_command::config_brain_for(resolved_brain);
    PeacefulConfig {
        tuning,
        brain_profile: BrainProfile::default(),
        capabilities: CombatCapabilities::default(),
        // Body CAPABILITY: the peaceful autonomous brain never presses attack, but
        // a possessing player can still throw the kit's punch/swing — the same
        // action set the spawn plan installs (`combat_kit.to_action_set(None)`).
        action_set: combat_kit.to_action_set(None),
        config_brain,
    }
}

/// Rebuild each autonomous catalog-backed actor's CONFIG from its restored
/// [`BrainBinding`] source. Skips gracefully when the world lacks the roster /
/// catalog (headless fixtures) or an actor lacks an `ActorConfig` (the catalog
/// pass already handled its live `Brain`).
pub fn reconcile_autonomous_actors(world: &mut World) {
    // Phase A: temporary control. Restore the live control mode (player possession
    // / mount) from the snapshot-persisted `TemporaryControl`, so a rewind that
    // crossed a possess/release boundary lands the body — and the player's home
    // avatar — in the correct mode. Runs first so phase B skips controlled bodies.
    reconcile_temporary_control(world);

    struct Job {
        entity: Entity,
        source: AutonomousSource,
        character_id: Option<String>,
    }

    // Collect the autonomous, config-bearing actors. `query` (not `try_query`) so
    // the optional `Mounted` component type is initialized even in a world that
    // never spawned one (a `try_query` returns `None` there and silently skips).
    let jobs: Vec<Job> = {
        let mut q = world.query::<(
            Entity,
            &BrainBinding,
            &Brain,
            Option<&ActorConfig>,
            bevy::ecs::query::Has<crate::features::Mounted>,
        )>();
        q.iter(world)
            .filter_map(|(entity, binding, brain, config, mounted)| {
                // Temporary control is untouchable and owned by its own pass.
                if brain.is_player() || mounted {
                    return None;
                }
                let config = config?;
                Some(Job {
                    entity,
                    source: binding.source.clone(),
                    character_id: config.sprite_character_id.clone(),
                })
            })
            .collect()
    };
    if jobs.is_empty() {
        return;
    }

    for job in jobs {
        match &job.source {
            AutonomousSource::Provoked { archetype } => {
                reconstruct_provoked(world, job.entity, archetype.as_str());
            }
            // ⭐ **A PROVOKED CHARACTER KEEPS ITS BODY.** The archetype arm above
            // rebuilds a whole body — tuning, HP pool, capabilities — because an
            // archetype IS the creature. A character that states what it becomes
            // when struck changed only its mind, so only its mind is rebuilt.
            AutonomousSource::ProvokedProfile { profile } => {
                reconstruct_provoked_profile(world, job.entity, profile);
            }
            AutonomousSource::CatalogDefault | AutonomousSource::CatalogPreset(_) => {
                restore_peaceful_config(world, job.entity, job.character_id.as_deref());
            }
            // ⭐⭐ **A CHARACTER-FIRST BODY IS NOT A GENERIC PEACEFUL NPC.**
            //
            // ⛔ this shared the arm above, and the comment justifying that said
            // the difference was "WHO stated the policy, not what a rewind has to
            // put back". That is false in both directions.
            // [`peaceful_config`] is not controller reconstruction: it is BODY
            // reconstruction — `max_health: 1`, `max_run_speed: MAX_RUN_SPEED`,
            // default capabilities, and `brain_profile: BrainProfile::default()`.
            // Running it over a body whose character authored its own run speed,
            // health and locomotion replaces those with the generic stroller
            // seed, and the profile it zeroes is the very field the old
            // `CharacterProfile` restoration then READ back as the character's
            // default.
            //
            // ⭐ restoring an autonomous source updates the MIND. The body a
            // character built is the body it comes back to.
            AutonomousSource::CharacterProfile => {
                restore_character_default_policy(world, job.entity);
            }
            // A boss's autonomous BossPattern brain is snapshotted by the ordinary
            // brain codec (it is a `Brain` variant), and a boss carries no
            // `ActorConfig` — so it is filtered out of this config-reconstruction
            // loop above. Its temporary-control resumption is handled by the
            // suspended-autonomous-runtime pass; nothing to reconstruct here.
            AutonomousSource::Boss { .. } => {}
        }
    }
}

/// Rerun the roster archetype construction for a provoked actor and install the
/// coupled config (tuning / brain-spec / capabilities / sprite / read-model brain
/// / live brain / action set). Leaves the registered disposition / health /
/// gravity to their own restored blobs.
/// **Rebuild a provoked CHARACTER's mind, and nothing else.**
///
/// ⭐ the counterpart to [`reconstruct_provoked`], which rebuilds a whole body
/// because an archetype IS the creature. A character that states what it becomes
/// when struck changed only its controller policy and its relationship, so a
/// rewind restores only those — the body it came back to is the one its
/// character built.
///
/// ⚠ a profile the registry no longer publishes leaves the live brain alone,
/// which is the same answer the archetype path gives for a roster it cannot
/// resolve: a headless fixture publishes nothing, and refusing to guess is
/// better than installing a default somebody's rewind did not ask for.
fn reconstruct_provoked_profile(
    world: &mut World,
    entity: Entity,
    profile: &ambition_entity_catalog::BrainProfileId,
) {
    let Some(resolved) = world
        .get_resource::<ambition_characters::actor::character_catalog::BrainProfileRegistry>()
        .and_then(|profiles| profiles.get(profile).copied())
    else {
        return;
    };
    let Some(config) = world.get::<ActorConfig>(entity).cloned() else {
        return;
    };
    let abilities = world
        .get::<ambition_platformer2d_core::BodyAbilities>(entity)
        .map(|abilities| abilities.abilities)
        .unwrap_or_default();
    let mut hostile = config;
    hostile.brain_profile = resolved;
    let brain = crate::features::ecs::enemy_default_brain(&hostile, abilities);
    let Ok(mut em) = world.get_entity_mut(entity) else {
        return;
    };
    if let Some(mut config) = em.get_mut::<ActorConfig>() {
        config.brain_profile = resolved;
    }
    // ⛔ never over a player-driven body — the silent-seizure rule the live
    // provoke path documents at length. One rule, every writer.
    let driven = em
        .get::<ambition_characters::brain::Brain>()
        .is_some_and(ambition_characters::brain::Brain::is_player);
    if !driven {
        em.insert(brain);
    }
    em.insert(crate::combat::components::ActorDisposition::Hostile);
}

fn reconstruct_provoked(world: &mut World, entity: Entity, archetype: &str) {
    let Some(spec) = world
        .get_resource::<CharacterRoster>()
        .map(|roster| roster.spec_for_brain(&CharacterBrain::Custom(archetype.to_string())))
    else {
        // Headless fixture without a roster: leave the live brain to its authority.
        return;
    };
    let (Some(config), Some(kit)) = (
        world.get::<ActorConfig>(entity).cloned(),
        world.get::<CombatKit>(entity).cloned(),
    ) else {
        return;
    };
    let held = world.get::<HeldItem>(entity).cloned();
    // The body's live verbs. A body with no `BodyAbilities` has none to reach
    // for, which is the honest answer rather than a borrowed default.
    let body = world
        .get::<ambition_platformer2d_core::BodyAbilities>(entity)
        .map(|abilities| abilities.abilities)
        .unwrap_or_default();
    let proj = project_provoked_archetype(&spec, archetype, &config, &kit, held.as_ref(), body);

    let Ok(mut em) = world.get_entity_mut(entity) else {
        return;
    };
    // ⭐ the twin of the live flip, and it lost the same four writes: a rewind
    // into a provoked snapshot restores the MIND and the KIT, because those are
    // all the provocation ever changed.
    if let Some(mut config) = em.get_mut::<ActorConfig>() {
        config.brain_profile = proj.brain_profile;
        config.brain = proj.config_brain;
    }
    // ⚠ **the twins must write the SAME set**, which is this module's whole
    // contract: a provoked actor is identical whether it was just challenged or
    // rebuilt from a snapshot. Both write the mind, the kit, and the two body
    // facts that are still projected — see `ProvokedArchetype`, where each of
    // the two says why it is there and what deletes it.
    em.insert(fresh_health_pool(proj.max_health));
    if let Some(mut surface) = em.get_mut::<crate::features::enemies::ActorSurfaceState>() {
        surface.gravity_scale = proj.gravity_scale;
    }
    em.insert((proj.brain, proj.action_set));
}

/// **Put a character-first body back on its own character's policy** — and touch
/// nothing else about the body.
///
/// The counterpart to [`restore_peaceful_config`]: that one reconstructs the
/// generic peaceful NPC seed because a catalog-default body IS that seed, and
/// this one restores a mind because a character-first body's tuning, health,
/// capabilities and kit came from its character and no rewind of a CONTROLLER
/// decision has any business editing them.
///
/// ⚠ `config.brain` is the integrator read-model and is derived from the live
/// brain through the shared helper, exactly as every other restoration site
/// derives it, so the classification cannot disagree with the actual brain.
/// A body with no resolvable policy (no cast, or a character that states none)
/// keeps the one it has — the fixture road, see
/// [`character_policy::default_policy_for`](crate::features::ecs::character_policy::default_policy_for).
fn restore_character_default_policy(world: &mut World, entity: Entity) {
    let Some(profile) = crate::features::ecs::character_policy::default_policy_in(world, entity)
    else {
        return;
    };
    let config_brain = world
        .get::<Brain>(entity)
        .map(crate::features::brain_command::config_brain_for);
    let Ok(mut em) = world.get_entity_mut(entity) else {
        return;
    };
    if let Some(mut config) = em.get_mut::<ActorConfig>() {
        config.brain_profile = profile;
        if let Some(config_brain) = config_brain {
            config.brain = config_brain;
        }
    }
}

/// Restore the peaceful catalog config a catalog-backed NPC spawned with —
/// reverting a config left hostile by a provocation the rewind undid. Idempotent
/// for an NPC that was never provoked (it re-sets the same fixed peaceful values).
/// The live catalog brain is rebuilt by the catalog reconcile pass; this only
/// owns the coupled config, and `config.brain` is derived from that live brain.
fn restore_peaceful_config(world: &mut World, entity: Entity, character_id: Option<&str>) {
    let Some(kit) = world.get::<CombatKit>(entity).cloned() else {
        return;
    };
    let Some(brain) = world.get::<Brain>(entity).cloned() else {
        return;
    };
    let Some(peaceful) = world
        .get_resource::<CharacterCatalog>()
        .map(|catalog| peaceful_config(catalog, character_id, &kit, &brain))
    else {
        return;
    };

    let Ok(mut em) = world.get_entity_mut(entity) else {
        return;
    };
    if let Some(mut config) = em.get_mut::<ActorConfig>() {
        config.tuning = peaceful.tuning;
        config.brain_profile = peaceful.brain_profile;
        config.brain = peaceful.config_brain;
        config.sprite_override_npc_name = None;
    }
    em.insert((peaceful.action_set, peaceful.capabilities));
}

/// Reset a body's live `BodyHealth` to a fresh archetype pool — used by the live
/// provoke flip. Reconstruction leaves health to its snapshot blob.
pub(crate) fn fresh_health_pool(max_health: i32) -> BodyHealth {
    BodyHealth::new(Health::new(max_health))
}

/// The autonomous `Brain` an actor resumes when no controller masks it — rebuilt
/// from its [`BrainBinding`] source (catalog preset, or a rerun of the provoked
/// roster construction). The single seam possession/mount RELEASE resumes from
/// and reconciliation rebuilds a Player→Autonomous transition with, so "resume
/// the selected autonomous source" means the *current* source, never a stale
/// cache. Returns `None` in a fixture missing the catalog / roster / config.
pub(crate) fn autonomous_brain_for_source(world: &World, entity: Entity) -> Option<Brain> {
    let binding = world.get::<BrainBinding>(entity)?;
    match &binding.source {
        // ⭐ the character's own combat policy, resolved from the published
        // registry — the same shape as the archetype arm below it, one authority
        // over.
        AutonomousSource::ProvokedProfile { profile } => {
            let profiles = world.get_resource::<
                ambition_characters::actor::character_catalog::BrainProfileRegistry,
            >()?;
            let resolved = *profiles.get(profile)?;
            let config = world.get::<ActorConfig>(entity)?;
            let mut hostile = config.clone();
            hostile.brain_profile = resolved;
            let abilities = world
                .get::<ambition_platformer2d_core::BodyAbilities>(entity)
                .map(|abilities| abilities.abilities)
                .unwrap_or_default();
            Some(crate::features::ecs::enemy_default_brain(
                &hostile, abilities,
            ))
        }
        // ⭐⭐ **ASK THE CHARACTER, BY IDENTITY.**
        //
        // ⛔ this read the body's own `ActorConfig::brain_profile` and called it
        // "the policy its character authored". It is the policy the body is
        // running NOW, and `provoke_actor_in_place` overwrites it — so resuming
        // a provoked-then-released body from this arm rebuilt the PROVOKED mind
        // and labelled it the character's default. See
        // [`character_policy`](crate::features::ecs::character_policy), which is
        // the one place the durable answer is recovered.
        AutonomousSource::CharacterProfile => {
            crate::features::ecs::character_policy::character_default_brain_in(world, entity)
        }
        AutonomousSource::Provoked { archetype } => {
            let roster = world.get_resource::<CharacterRoster>()?;
            let spec =
                roster.spec_for_brain(&CharacterBrain::Custom(archetype.as_str().to_string()));
            let config = world.get::<ActorConfig>(entity)?;
            let kit = world.get::<CombatKit>(entity)?;
            let held = world.get::<HeldItem>(entity);
            let body = world
                .get::<ambition_platformer2d_core::BodyAbilities>(entity)
                .map(|abilities| abilities.abilities)
                .unwrap_or_default();
            Some(
                project_provoked_archetype(&spec, archetype.as_str(), config, kit, held, body)
                    .brain,
            )
        }
        AutonomousSource::CatalogDefault | AutonomousSource::CatalogPreset(_) => {
            let catalog = world.get_resource::<CharacterCatalog>()?;
            let preset = binding.active_preset()?.as_str().to_string();
            let ctx = world
                .get::<AuthoredBrainContext>(entity)
                .map(AuthoredBrainContext::build_context)
                .unwrap_or_else(|| {
                    BrainBuildContext::at(
                        world
                            .get::<ActorPose>(entity)
                            .map(|pose| pose.origin().x)
                            .unwrap_or(0.0),
                    )
                });
            catalog.build_brain_from_preset(&preset, &ctx)
        }
        // A boss's autonomous brain is not rebuilt from a catalog preset: it is
        // the live `BossPattern` captured into the suspended-autonomous-runtime at
        // possession and resumed from there, so this catalog-preset resolver
        // returns `None` for a boss source (the caller resumes from the captured
        // runtime instead).
        AutonomousSource::Boss { .. } => None,
    }
}

/// Restore the live temporary-control mode (player possession / mount) from each
/// body's snapshot-persisted [`TemporaryControl`].
///
/// The `Brain` cursor cannot restore a `Brain::Player`, and possession/mount
/// relationships were re-derived from live components, so after the registered
/// blobs land the live control can disagree with the restored `TemporaryControl`.
/// This rebuilds it — both the controlled body's live `Brain`/`Mounted` AND the
/// coupled relationships (the vacated home avatar, `PossessionState`, `RidingOn`/
/// `MountSlot`) — from the stable ids, in BOTH rewind directions.
///
/// Order: possession first (there is exactly one player), then mounts. A body
/// whose control ended resumes its autonomous brain from its binding source (via
/// [`autonomous_brain_for_source`]); a body that gained control gets the player /
/// mounted brain installed.
fn reconcile_temporary_control(world: &mut World) {
    struct Body {
        entity: Entity,
        control: TemporaryControl,
        live_is_player: bool,
        live_mounted: bool,
        /// **This body is player-brained BY CONSTRUCTION, not by possession.**
        /// See the autonomous-resume arm below for what mistaking the two costs.
        seated: bool,
    }

    let bodies: Vec<Body> = {
        let mut q = world.query::<(
            Entity,
            &TemporaryControl,
            &Brain,
            bevy::ecs::query::Has<Mounted>,
            bevy::ecs::query::Has<crate::character_runtime::MatchSeat>,
        )>();
        q.iter(world)
            .map(|(entity, control, brain, mounted, seated)| Body {
                entity,
                control: control.clone(),
                live_is_player: brain.is_player(),
                live_mounted: mounted,
                seated,
            })
            .collect()
    };
    if bodies.is_empty() {
        return;
    }

    // The player's home avatar (keeps `PrimaryPlayer` even while its brain is
    // vacated onto a possessed body). A defensive fallback only — the possessed
    // body's snapshotted controller id is the authority (see below).
    let primary_player_home = {
        let mut q = world.query_filtered::<Entity, bevy::ecs::query::With<PrimaryPlayer>>();
        q.iter(world).next()
    };
    // Stable-id → entity, to rebuild raw-`Entity` relationships (mount links) and
    // to resolve the authoritative possession controller id.
    let by_sim_id: std::collections::BTreeMap<String, Entity> = {
        let mut q = world.query::<(Entity, &SimId)>();
        q.iter(world)
            .map(|(entity, id)| (id.as_str().to_string(), entity))
            .collect()
    };

    // ── Possession ──────────────────────────────────────────────────────────
    // Exactly one body may be player-controlled. More than one is a corrupt
    // snapshot (two vacated homes, a double-assigned slot); surface it rather than
    // silently picking one.
    let player_bodies: Vec<&Body> = bodies
        .iter()
        .filter(|b| matches!(b.control, TemporaryControl::Player { .. }))
        .collect();
    if player_bodies.len() > 1 {
        error!(
            target: "ambition_platformer2d_actor_monolith::rollback_reconcile",
            "restore: {} bodies are player-controlled (expected <= 1); using the first",
            player_bodies.len(),
        );
    }
    let possessed = player_bodies.first().map(|b| (b.entity, b.control.clone()));

    if let Some((target, control)) = possessed {
        // The controller is authoritative: resolve the stable id the snapshot
        // stored, NOT whichever body happens to carry `PrimaryPlayer`. Diagnose a
        // missing controller or a disagreement rather than silently diverging.
        let controller_id = match &control {
            TemporaryControl::Player { controller } => Some(controller.as_str().to_string()),
            _ => None,
        };
        let home = match controller_id
            .as_deref()
            .and_then(|id| by_sim_id.get(id).copied())
        {
            Some(resolved) => {
                if let Some(pp) = primary_player_home {
                    if pp != resolved {
                        warn!(
                            target: "ambition_platformer2d_actor_monolith::rollback_reconcile",
                            "restore: possession controller id {:?} resolves to a different body \
                             than PrimaryPlayer; trusting the stored controller id",
                            controller_id,
                        );
                    }
                }
                Some(resolved)
            }
            None => {
                warn!(
                    target: "ambition_platformer2d_actor_monolith::rollback_reconcile",
                    "restore: possession controller id {:?} did not resolve to any body; \
                     falling back to PrimaryPlayer",
                    controller_id,
                );
                primary_player_home
            }
        };

        // A possessed body: install the player brain, vacate the home avatar, and
        // rebuild the possession bookkeeping. `restore_brain` is the CURRENT
        // autonomous source (so a source changed during possession resumes on
        // release), never a stale cache.
        let restore_brain = autonomous_brain_for_source(world, target);
        if let Ok(mut em) = world.get_entity_mut(target) {
            em.insert((Brain::Player(PlayerSlot::PRIMARY), ActorControl::default()));
        }
        if let Some(home) = home {
            if let Ok(mut em) = world.get_entity_mut(home) {
                em.remove::<Brain>();
                em.insert(ActorControl::default());
            }
        }
        if let Some(mut possession) = world.get_resource_mut::<PossessionState>() {
            possession.possessed = Some(target);
            possession.home = home;
            possession.restore_brain = restore_brain;
        }
    } else {
        // No possession: the home avatar drives, and any body left player-brained
        // by the abandoned future resumes its autonomous source.
        for body in &bodies {
            // ⛔ **A SEATED FIGHTER IS NOT AN ABANDONED FUTURE.**
            //
            // This arm resumes the autonomous brain of any body that is
            // player-brained while its `TemporaryControl` says autonomous — a
            // correct repair when the ONLY way to be player-brained is to be
            // possessed, which was true until a match could seat one.
            //
            // A local seat's fighter carries `Brain::Player(slot)` because
            // `activate_the_prepared_match` gave it one, and `TemporaryControl`
            // never says otherwise because nobody possessed anything. So the
            // first rollback restore after the match opened handed every human's
            // fighter to the CPU, in place, on the same entity — and the couch
            // test read it as crosstalk, because the AI then walked the "wrong"
            // fighter around. Measured: seat 1 flipped 28 frames after its pad
            // was released, seat 0 shortly after.
            //
            // ⭐ **arbitrate by IDENTITY.** A seat is who this body is, not a
            // state it is passing through; possession is the thing that is
            // temporary. This function reconciles the control it OWNS.
            if body.seated {
                continue;
            }
            if body.live_is_player && body.control.is_autonomous() {
                if let Some(brain) = autonomous_brain_for_source(world, body.entity) {
                    if let Ok(mut em) = world.get_entity_mut(body.entity) {
                        em.insert((brain, ActorControl::default()));
                    }
                }
            }
        }
        if let Some(home) = primary_player_home {
            let home_drives = world
                .get::<Brain>(home)
                .map(Brain::is_player)
                .unwrap_or(false);
            if !home_drives {
                if let Ok(mut em) = world.get_entity_mut(home) {
                    em.insert((Brain::Player(PlayerSlot::PRIMARY), ActorControl::default()));
                }
            }
        }
        if let Some(mut possession) = world.get_resource_mut::<PossessionState>() {
            possession.possessed = None;
            possession.home = None;
            possession.restore_brain = None;
        }
    }

    // ── Seated fighters ─────────────────────────────────────────────────────
    //
    // ⛔ **A PLAYER BRAIN IS NEVER SNAPSHOTTED, and a match seat is the one
    // player brain nothing here knew how to rebuild.** `TemporaryControl`'s own
    // doc states the contract: *"the `Brain` cursor is a no-op for
    // `Brain::Player` … reconciliation rebuilds the live control from the
    // restored id"*. That works because, until a match could seat one, the only
    // way to be player-brained was to BE the home avatar or to be possessing
    // something — both recorded in `TemporaryControl`.
    //
    // A seated fighter is player-brained by CONSTRUCTION. Its `TemporaryControl`
    // says `Autonomous`, because nobody possessed anything, so every rollback
    // restore put the archetype's brain back and the human's fighter became a
    // CPU — in place, on the same entity, permanently, because activation is
    // one-shot and never rebinds. Measured on the couch fixture: both seats
    // opened as `Player(0)`/`Player(1)` and seat one flipped to the Smash state
    // machine 28 frames after its pad went quiet.
    //
    // ⭐ **the plan is what activation replays FROM, and it is what this replays
    // from too.** `PreparedMatch` is deliberately not rollback state — a
    // decision made before the session — so it is still standing after any
    // rewind and still says which seats hold a local channel. Re-deriving the
    // authority from the plan is the same act as binding it in the first place,
    // which is the property that makes a rewind reconstruct the SAME match
    // rather than a similar one.
    let channels: Vec<Option<ambition_input::ParticipantId>> = world
        .get_resource::<crate::character_runtime::PreparedMatch>()
        .map(|plan| {
            plan.seats()
                .iter()
                .map(|seat| seat.authority.local_channel())
                .collect()
        })
        .unwrap_or_default();
    let seated_authorities: Vec<(Entity, ambition_input::ParticipantId)> = if channels.is_empty() {
        Vec::new()
    } else {
        let mut q = world.query::<(Entity, &crate::character_runtime::MatchSeat)>();
        q.iter(world)
            .filter_map(|(entity, seat)| {
                channels
                    .get(seat.0)
                    .copied()
                    .flatten()
                    .map(|channel| (entity, channel))
            })
            .collect()
    };
    for (entity, channel) in seated_authorities {
        // ⭐ through the one correspondence, exactly as construction does — a
        // rebuild that spelled the projection itself would be the second place
        // that decides which seat a channel reads.
        let seat = crate::participant_seat::player_slot_of(channel);
        let wanted = Brain::Player(seat);
        // `is_player` + slot rather than a whole-value compare: `Brain` is not
        // `PartialEq`, and the slot is the only part of a player brain that
        // carries meaning here.
        let already = world
            .get::<Brain>(entity)
            .is_some_and(|brain| matches!(brain, Brain::Player(slot) if *slot == seat));
        if already {
            continue;
        }
        if let Ok(mut em) = world.get_entity_mut(entity) {
            em.insert((wanted, ActorControl::default()));
        }
    }

    // ── Mounts ──────────────────────────────────────────────────────────────
    for body in &bodies {
        match &body.control {
            TemporaryControl::Mounted { mount } => {
                // Install the mounted mode (BOTH the mounted brain AND its action
                // set, from the rider's cache) and rebuild the rider↔mount link
                // from the stable mount id. Installing the brain without the action
                // set would leave a mounted rider with a mismatched pair.
                let cached = world
                    .get::<MountedBrainCache>(body.entity)
                    .map(|cache| (cache.brain.clone(), cache.action_set.clone()));
                let mount_entity = by_sim_id.get(mount.as_str()).copied();
                if let Ok(mut em) = world.get_entity_mut(body.entity) {
                    if !em.contains::<Mounted>() {
                        em.insert(Mounted);
                    }
                    if let Some((brain, action_set)) = cached {
                        em.insert((brain, action_set));
                    }
                    if let Some(mount_entity) = mount_entity {
                        em.insert(RidingOn {
                            mount: mount_entity,
                        });
                    }
                }
                if let Some(mount_entity) = mount_entity {
                    // INSERT, not `get_mut`. This was
                    // `if let Some(mut slot) = world.get_mut::<MountSlot>(..)`,
                    // which silently did nothing when the mount had no
                    // `MountSlot` — and a mount can easily lack one, because the
                    // component is installed by the pair wiring rather than by
                    // the mount's own construction. The rider-side `RidingOn`
                    // above is inserted unconditionally, so the two disagreed:
                    // the rider pointed at a mount that did not point back, and
                    // `steer_mount_from_rider` queries `With<MountSlot>`, so the
                    // mount stopped obeying while every rider-side assertion
                    // still passed.
                    //
                    // Writing the whole relation side rather than mutating a
                    // field also makes the restored link identical whether or not
                    // a slot survived the rewind.
                    if let Ok(mut mount_ref) = world.get_entity_mut(mount_entity) {
                        mount_ref.insert(MountSlot {
                            rider: Some(body.entity),
                        });
                    }
                }
            }
            // Not mounted per the snapshot, but a stale `Mounted` marker survived
            // the rewind (and it is not player-possessed): dismount it back to its
            // autonomous brain.
            TemporaryControl::Autonomous if body.live_mounted => {
                let brain = autonomous_brain_for_source(world, body.entity);
                if let Ok(mut em) = world.get_entity_mut(body.entity) {
                    em.remove::<Mounted>();
                    if let Some(brain) = brain {
                        em.insert(brain);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::enemies::test_roster;
    use ambition_characters::actor::character_catalog::{
        parse_catalog, BrainPresetId, HostileArchetypeId,
    };
    use ambition_platformer2d_core as ae;

    const CATALOG: &str = r#"(
        brain_presets: { "wanderer_x": Wanderer(speed: 40.0, aggressiveness: 0.0) },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_x": (
                display_name: "X", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "wanderer_x", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn catalog() -> CharacterCatalog {
        CharacterCatalog::from_data(parse_catalog(CATALOG))
    }

    fn config_fixture() -> ActorConfig {
        ActorConfig {
            id: "npc".into(),
            name: "Npc".into(),
            tuning: ActorTuning::default(),
            brain_profile: BrainProfile::default(),
            brain: CharacterBrain::Passive,
            spawn: crate::features::enemies::ActorSpawnState {
                pos: ae::Vec2::ZERO,
                size: ae::Vec2::splat(8.0),
            },
            sprite_override_npc_name: None,
            sprite_character_id: Some("npc_x".into()),
        }
    }

    fn wanderer(world: &World) -> Brain {
        world
            .resource::<CharacterCatalog>()
            .build_brain_from_preset("wanderer_x", &BrainBuildContext::at(0.0))
            .expect("wanderer builds")
    }

    /// **PROVOCATION PROJECTS NO TUNING AT ALL, so a body keeps everything it
    /// was.**
    ///
    /// ⛔ this test used to be narrower and its name says so: *"borrows COMBAT
    /// numbers but never the placement respawn policy"*. It existed because
    /// `project_provoked_archetype` assigned `spec.tuning()` wholesale and a
    /// provoked NPC silently became `OnRoomReenter` — the kill hook wrote no
    /// death flag, save-sync had nothing to read, and the NPC was rebuilt alive
    /// by the next room construction ("kill an NPC, it respawns immediately",
    /// ADR 0022).
    ///
    /// ⭐ the fix at the time carved ONE field out of the wholesale assignment.
    /// The projection now assigns no tuning whatever — a provocation changes the
    /// mind and the kit, never the body — so the respawn policy survives for the
    /// same reason the run speed and the gait do, and the narrow claim became a
    /// special case of a general one. Asserting the general one is what stops a
    /// future widening putting a second field back.
    ///
    /// ⚠ the poison is the second half: the projection must still produce a real
    /// hostile MIND. "It changed nothing" would satisfy the first assertion
    /// perfectly while describing a provocation that does not provoke.
    #[test]
    fn provocation_changes_the_mind_and_leaves_every_body_fact_alone() {
        use ambition_entity_catalog::placements::RespawnPolicy;

        let roster = test_roster();
        let spec = roster.spec_for_brain(&CharacterBrain::Custom("combatant".into()));
        // The fixture body disagrees with the archetype on every field checked
        // below, so "preserved" cannot be read as "coincidentally equal".
        assert_eq!(
            spec.tuning().respawn,
            RespawnPolicy::OnRoomReenter,
            "fixture assumption: the archetype respawns per room"
        );

        let mut config = config_fixture();
        config.tuning.respawn = RespawnPolicy::DeadStaysDead;
        config.tuning.max_run_speed = 91.0;
        config.tuning.surface_walker = true;
        assert_ne!(
            config.tuning.max_run_speed,
            spec.tuning().max_run_speed,
            "fixture assumption: the body is not already running at the archetype's speed"
        );

        let before = config.clone();
        let proj = project_provoked_archetype(
            &spec,
            "combatant",
            &config,
            &CombatKit::default(),
            None,
            ambition_platformer2d_core::AbilitySet::default(),
        );

        // The projection is pure, so the only way a body fact could change is
        // through a field on the result. There is none — this asserts the input
        // is untouched AND names what the result is allowed to carry.
        assert_eq!(
            before.tuning, config.tuning,
            "the projection mutated its input"
        );

        // ⭐ THE POISON. Without this, deleting the whole projection passes.
        assert_eq!(
            proj.brain_profile,
            spec.brain_profile(),
            "the provoked POLICY is the archetype's — that is the one thing a \
             generic provocation is for"
        );
        assert!(
            !matches!(
                proj.config_brain,
                ambition_entity_catalog::placements::CharacterBrain::Passive
            ),
            "a provoked body's read-model still says it is not passive"
        );
        assert_eq!(
            proj.max_health, spec.max_health,
            "the HP pool is the one body fact still borrowed, and it is D96 \
             item 7 rather than a design — if this stops being true the ledger \
             row was answered and this comment is the changelog"
        );
    }

    /// **The mount reconcile INSERTS a missing `MountSlot` rather than skipping
    /// it.**
    ///
    /// The bug: the restore path re-established the rider→mount link with
    /// `world.get_mut::<MountSlot>(mount)`, which silently does nothing when the
    /// mount has no `MountSlot` — and a mount easily lacks one, because the
    /// component is installed by the pair wiring, not by the mount's own
    /// construction. `RidingOn` was inserted unconditionally, so the two ends
    /// disagreed: the rider pointed at a mount that did not point back, and
    /// `steer_mount_from_rider` (which queries `With<MountSlot>`) quietly stopped
    /// obeying while every rider-side assertion still passed.
    ///
    /// The fixture starts exactly there: a restored `Mounted` rider naming its
    /// mount by id, the mount present, and NO `MountSlot` on it.
    #[test]
    fn the_mount_reconcile_inserts_a_missing_mount_slot() {
        use crate::features::TemporaryControl;

        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());

        let mount = w.spawn((SimId::placement("shark"), config_fixture())).id();
        let rider = w
            .spawn((
                SimId::placement("rider"),
                config_fixture(),
                Brain::stand_still(),
                Mounted,
                MountedBrainCache {
                    brain: Brain::stand_still(),
                    action_set: ambition_characters::brain::ActionSet::default(),
                },
                TemporaryControl::Mounted {
                    mount: SimId::placement("shark"),
                },
            ))
            .id();
        assert!(
            w.get::<MountSlot>(mount).is_none(),
            "fixture: the mount begins with no MountSlot, the exact half-write shape"
        );

        reconcile_autonomous_actors(&mut w);

        assert_eq!(
            w.get::<RidingOn>(rider).map(|r| r.mount),
            Some(mount),
            "the rider still points at its mount"
        );
        assert_eq!(
            w.get::<MountSlot>(mount).and_then(|slot| slot.rider),
            Some(rider),
            "and the mount now points back — both ends agree after reconciliation"
        );
    }

    /// The cast a character-first fixture rewinds inside — published through the
    /// production registration seam, so the identity lookup under test is the
    /// one production performs rather than a hand-built map.
    fn cast_with_villager(
        policy: BrainProfile,
    ) -> crate::character_runtime::PreparedCharacterRegistry {
        use crate::character_runtime::CharacterDefinitionAppExt;
        let mut app = bevy::prelude::App::new();
        app.register_character(
            crate::character_runtime::CharacterDefinition::new(
                "npc_villager",
                "Villager",
                "reconcile_tests",
            )
            .with_autonomous_profile(policy),
        );
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        app.world()
            .resource::<crate::character_runtime::PreparedCharacterRegistry>()
            .clone()
    }

    /// **A REWIND RESTORES A MIND, NOT A GENERIC BODY.**
    ///
    /// ⛔⛔ the `CharacterProfile` source shared an arm with the catalog-default
    /// one, on the stated reasoning that "the difference is WHO stated the
    /// policy, not what a rewind has to put back". Both halves of that are
    /// false. [`peaceful_config`] is BODY reconstruction — `max_health: 1`,
    /// `max_run_speed: MAX_RUN_SPEED`, default capabilities — so every rewind
    /// replaced a character-authored body with the generic stroller seed. And
    /// the `brain_profile: BrainProfile::default()` it wrote was the very field
    /// the old `CharacterProfile` resume road then read back as *the policy its
    /// character authored*: the restoration destroyed its own source.
    ///
    /// ⭐ the poison is the config going IN: its live policy is a MeleeBrute and
    /// its character authors a Wanderer, so a reconstruction that reads the
    /// field instead of the character gives a visibly different answer.
    #[test]
    fn a_rewound_character_first_body_gets_its_characters_policy_and_keeps_its_body() {
        let authored = BrainProfile {
            template: ambition_characters::brain::CharacterBrainTemplate::Wanderer,
            ..Default::default()
        };
        let mut w = World::new();
        w.insert_resource(catalog());
        w.insert_resource(cast_with_villager(authored));

        let mut config = config_fixture();
        config.sprite_character_id = Some("npc_villager".into());
        config.tuning.max_health = 7;
        config.tuning.max_run_speed = 91.0;
        // ⭐⭐ **NOT `MeleeBrute`, AND THAT IS THE INSTRUMENT.** `BrainProfile`'s
        // own `Default` is a MeleeBrute, so a provoked fixture using it makes
        // "left provoked" and "zeroed to a default nobody authored" print the
        // SAME failure — two different bugs wearing one message. Three distinct
        // templates keep the three outcomes three.
        config.brain_profile = BrainProfile {
            template: ambition_characters::brain::CharacterBrainTemplate::Skirmisher,
            aggro_radius: 220.0,
            ..Default::default()
        };
        let e = w
            .spawn((
                SimId::placement("villager"),
                config,
                CombatKit::default(),
                Brain::stand_still(),
                BrainBinding {
                    default_preset: ambition_characters::actor::character_catalog::AutonomousDefault::CharacterProfile,
                    source: AutonomousSource::CharacterProfile,
                },
                ambition_characters::actor::WornCharacter::new("npc_villager"),
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        let config = w.get::<ActorConfig>(e).expect("still configured");
        assert_eq!(
            config.brain_profile.template,
            ambition_characters::brain::CharacterBrainTemplate::Wanderer,
            "the restored policy is the one its CHARACTER authors"
        );
        assert_eq!(config.tuning.max_health, 7, "its own health pool survives");
        assert_eq!(
            config.tuning.max_run_speed, 91.0,
            "and its own top speed — a rewind of a controller decision is not a              licence to rebuild the body"
        );

        assert_eq!(
            autonomous_brain_for_source(&w, e).map(|brain| brain.label()),
            Some("wanderer"),
            "and the mind it resumes into is that same policy"
        );
    }

    /// A rewind INTO a provoked snapshot reruns the roster construction: the
    /// hostile mind, kit and HP pool are reconstructed from the stable archetype
    /// id alone.
    ///
    /// ⛔⛔ **THE TWINS DISAGREED ABOUT WHERE THE HP POOL LIVES, and this test
    /// was asserting the reconstruct's side of it.** The live flip writes
    /// `BodyHealth` (`*em.health = fresh_health_pool(..)`) and never touched
    /// `config.tuning.max_health`; the reconstruct wrote `config.tuning`
    /// wholesale and so set the second. They looked identical because the
    /// wholesale write happened to carry the archetype's number into both.
    ///
    /// `tuning.max_health` is read at SPAWN only — `ActorClusterSeed` sizes the
    /// initial `BodyHealth` from it — so after construction it is a stale
    /// record, and the LIVE pool is the authority. Now that provocation projects
    /// no tuning at all, the two twins write the one place that matters, and
    /// this asserts that place.
    #[test]
    fn reconstructs_a_provoked_actor_from_its_archetype_id() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        let mut config = config_fixture();
        config.tuning.max_health = 1; // peaceful HP left over from the present.
        let e = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::Provoked {
                        archetype: HostileArchetypeId::new("combatant"),
                    },
                ),
                config,
                CombatKit::default(),
                Brain::stand_still(),
                TemporaryControl::Autonomous,
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        let config = w.get::<ActorConfig>(e).unwrap();
        assert!(
            matches!(&config.brain, CharacterBrain::Custom(id) if id == "combatant"),
            "config.brain marks the provoked archetype"
        );
        assert_eq!(
            config.tuning.max_health, 1,
            "the placement's own tuning is untouched: provocation projects no \
             body facts, so the spawn-time record still says what it said"
        );
        assert_eq!(
            w.get::<BodyHealth>(e).map(|health| health.max()),
            Some(4),
            "the combatant HP pool is reconstructed from the roster, into the \
             LIVE pool the live provoke flip writes — the two twins must reach \
             the same field or a rewind changes a fighter's health"
        );
        assert_ne!(
            w.get::<Brain>(e).unwrap().label(),
            "stand_still",
            "the live brain is rebuilt to the hostile archetype, not left peaceful"
        );
    }

    /// A rewind to BEFORE a challenge (a catalog source over a config the present
    /// left hostile) restores the peaceful catalog config.
    #[test]
    fn reverts_a_catalog_actor_to_its_peaceful_config() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        let mut config = config_fixture();
        // The present is still hostile (provoked, then rewound past the challenge).
        config.tuning.max_health = 100;
        config.brain = CharacterBrain::Custom("combatant".into());
        let brain = wanderer(&w);
        let e = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config,
                CombatKit::default(),
                brain,
                TemporaryControl::Autonomous,
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        let config = w.get::<ActorConfig>(e).unwrap();
        assert_eq!(
            config.tuning.max_health, 1,
            "the peaceful HP pool is restored"
        );
        assert!(
            matches!(config.brain, CharacterBrain::Passive),
            "config.brain is derived from the live wanderer brain: Passive"
        );
    }

    /// Possession rollback — rewind INTO a possessed snapshot: the NPC becomes
    /// player-controlled, the home avatar is vacated, and `PossessionState` is
    /// rebuilt from the stable ids.
    #[test]
    fn restores_possession_across_a_rewind() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        w.init_resource::<PossessionState>();
        let home = w
            .spawn((
                SimId::player_slot(0),
                Brain::Player(PlayerSlot::PRIMARY),
                PrimaryPlayer,
            ))
            .id();
        let brain = wanderer(&w);
        let npc = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config_fixture(),
                CombatKit::default(),
                brain, // present: autonomous (released)
                TemporaryControl::Player {
                    controller: SimId::player_slot(0),
                },
                AuthoredBrainContext::from_placement(0.0, 0.0),
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        assert!(
            w.get::<Brain>(npc).unwrap().is_player(),
            "the possessed NPC carries the player brain again"
        );
        assert!(
            w.get::<Brain>(home).is_none(),
            "the home avatar is vacated (its player brain moved onto the NPC)"
        );
        let possession = w.resource::<PossessionState>();
        assert_eq!(possession.possessed, Some(npc));
        assert_eq!(possession.home, Some(home));
    }

    /// Possession rollback — rewind to an AUTONOMOUS snapshot from a possessed
    /// present: the NPC resumes its autonomous brain and the home avatar drives
    /// again (exactly one player brain).
    #[test]
    fn releases_possession_across_a_rewind() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        w.init_resource::<PossessionState>();
        // Home vacated in the present (possessing).
        let home = w.spawn((SimId::player_slot(0), PrimaryPlayer)).id();
        let npc = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config_fixture(),
                CombatKit::default(),
                Brain::Player(PlayerSlot::PRIMARY), // present: possessed
                TemporaryControl::Autonomous,       // snapshot: autonomous
                AuthoredBrainContext::from_placement(0.0, 0.0),
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        assert!(
            !w.get::<Brain>(npc).unwrap().is_player(),
            "the NPC resumes its autonomous brain"
        );
        assert!(
            w.get::<Brain>(home).unwrap().is_player(),
            "the home avatar drives again"
        );
        assert_eq!(w.resource::<PossessionState>().possessed, None);
    }

    /// The stored controller `SimId` is authoritative: reconcile resolves the home
    /// avatar by that stable id, NOT by whichever body carries `PrimaryPlayer`.
    /// Here the home has the controller id but no `PrimaryPlayer` marker, yet
    /// possession still vacates and rebinds to it.
    #[test]
    fn possession_resolves_the_home_by_controller_id_not_primary_player() {
        let mut w = World::new();
        w.insert_resource(test_roster());
        w.insert_resource(catalog());
        w.init_resource::<PossessionState>();
        // Home carries the controller id and a live player brain, but NOT the
        // `PrimaryPlayer` marker — so a `PrimaryPlayer` lookup would find nothing.
        let home = w
            .spawn((SimId::player_slot(0), Brain::Player(PlayerSlot::PRIMARY)))
            .id();
        let brain = wanderer(&w);
        let npc = w
            .spawn((
                SimId::placement("npc"),
                BrainBinding::new(
                    BrainPresetId::new("wanderer_x"),
                    AutonomousSource::CatalogDefault,
                ),
                config_fixture(),
                CombatKit::default(),
                brain,
                TemporaryControl::Player {
                    controller: SimId::player_slot(0),
                },
                AuthoredBrainContext::from_placement(0.0, 0.0),
            ))
            .id();

        reconcile_autonomous_actors(&mut w);

        assert!(
            w.get::<Brain>(npc).unwrap().is_player(),
            "the possessed NPC carries the player brain"
        );
        assert!(
            w.get::<Brain>(home).is_none(),
            "the home avatar, resolved via the stored controller id, is vacated"
        );
        let possession = w.resource::<PossessionState>();
        assert_eq!(possession.possessed, Some(npc));
        assert_eq!(
            possession.home,
            Some(home),
            "home resolved via the stored controller id, not PrimaryPlayer"
        );
    }
}
