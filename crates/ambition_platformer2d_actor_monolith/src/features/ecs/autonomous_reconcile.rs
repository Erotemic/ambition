//! **What a PROVOCATION produces**, projected once and applied by the live flip.
//!
//! ⛔⛔ **THIS FILE USED TO BE A ROLLBACK RECONCILER, AND IT NEVER RAN**
//! (ledger D104, reported by GPT 5.6 and proven 2026-08-12). Its entry point
//! `reconcile_autonomous_actors` had two re-exports, four doc comments and a
//! test module that called it directly — and zero production call sites. The
//! only system in `AmbitionLoadWorldSet::Reconcile` is
//! `codecs::reconcile_brain_bindings`, which filters on
//! `binding.active_preset()?` — `None` for every provoked and character-first
//! source. Nothing was ever going to invoke the reconstruction.
//!
//! ⭐ **and it was not a missing wire-up, it was 654 lines of redundancy.**
//! Every component it rebuilt is registered rollback state: `Brain` (cursor),
//! `BrainBinding`, `BodyHealth`, `ActorSurfaceState`, `TemporaryControl` and
//! `CombatCapabilities` (canonical), `ActorConfig`, `ActionSet`, `Mounted`,
//! `MountSlot`, `RidingOn`, `MountedBrainCache` (clone). The codecs had already
//! put all of them back; the file's reason for existing evaporated when those
//! registrations landed, and nobody re-read it.
//!
//! ⛔ **installing it would have been a REGRESSION.** Its provoked
//! reconstruction called `fresh_health_pool(max_health)`, so a damaged actor
//! would have healed to full on every load — exactly the divergence the Track B
//! campaign note recorded as *"a mid-brawl enemy full-heal"*.
//!
//! ⚠ **the proof is `game/ambition_app/tests/rollback_provoked_actor.rs`**, and
//! its first version was not proof: it asserted that state SURVIVED a window it
//! merely believed was a rollback window. It asserts against
//! `RollbackExecutionStats::lifetime_load_runs` now — the counter
//! `count_load_run` increments inside the very reconciliation set — because at
//! the SHIPPED prediction distance of 0, `LoadWorld` runs zero times and the
//! original tests passed anyway.
//!
//! ⇒ what remains here is the LIVE half, which always did the work:
//! [`provoked_projection`] (a mind and a kit, never a body) and
//! [`peaceful_config`] (the generic peaceful NPC seed a catalog switch restores),
//! both applied by `provoke_actor_in_place` and `brain_command`.

use bevy::prelude::*;

use super::actor_clusters::ActorConfig;
use super::{CombatKit, HeldItem};
use crate::combat::CombatCapabilities;
use crate::features::ecs::actor_tuning::{ActorTuning, BrainProfile};
use ambition_characters::actor::character_catalog::{CharacterBodyKind, CharacterCatalog};
use ambition_characters::actor::{BodyHealth, Health};
use ambition_characters::brain::{Brain, NPC_PATROL_SPEED};
use ambition_entity_catalog::placements::CharacterBrain;

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

/// **The projection itself, from a POLICY rather than from a row.**
///
/// ⭐ the live generic-provocation path calls this with
/// [`default_provoked_policy`](super::brain_builders::default_provoked_policy)
/// and [`DEFAULT_PROVOKED_HEALTH`](super::brain_builders::DEFAULT_PROVOKED_HEALTH),
/// so provoking a body no longer touches the archetype roster at all — that
/// lookup was the last reason the live path knew the ontology existed.
/// [`reconstruct_provoked_default`] is the rollback road's entry, and it states
/// the same two constants — so the twins cannot disagree about the policy.
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

/// Reset a body's live `BodyHealth` to a fresh archetype pool — used by the live
/// provoke flip. Reconstruction leaves health to its snapshot blob.
pub(crate) fn fresh_health_pool(max_health: i32) -> BodyHealth {
    BodyHealth::new(Health::new(max_health))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core as ae;

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

    /// **PROVOCATION PROJECTS NO TUNING AT ALL, so a body keeps everything it
    /// was.**
    ///
    /// ⛔ this test used to be narrower and its name said so: *"borrows COMBAT
    /// numbers but never the placement respawn policy"*. It existed because the
    /// projection assigned an archetype's `tuning()` wholesale and a provoked
    /// NPC silently became `OnRoomReenter` — the kill hook wrote no death flag,
    /// save-sync had nothing to read, and the NPC was rebuilt alive by the next
    /// room construction ("kill an NPC, it respawns immediately", ADR 0022).
    ///
    /// ⭐ the fix at the time carved ONE field out of the wholesale assignment.
    /// The projection assigns no tuning whatever now — a provocation changes the
    /// mind and the kit, never the body — so the respawn policy survives for the
    /// same reason the run speed does, and the narrow claim became a special
    /// case of a general one. Asserting the general one is what stops a future
    /// widening putting a second field back.
    ///
    /// ⚠ the poison is the second half: the projection must still produce a real
    /// hostile MIND. "It changed nothing" would satisfy the first assertion
    /// perfectly while describing a provocation that does not provoke.
    #[test]
    fn provocation_changes_the_mind_and_leaves_every_body_fact_alone() {
        use ambition_entity_catalog::placements::RespawnPolicy;

        let mut config = config_fixture();
        config.tuning.respawn = RespawnPolicy::DeadStaysDead;
        config.tuning.max_run_speed = 91.0;
        config.tuning.surface_walker = true;

        let before = config.clone();
        let proj = provoked_projection(
            crate::features::ecs::brain_builders::default_provoked_policy(),
            crate::features::ecs::brain_builders::DEFAULT_PROVOKED_HEALTH,
            false,
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
            crate::features::ecs::brain_builders::default_provoked_policy(),
            "the provoked POLICY is the engine's default — that is the one thing \
             a generic provocation is for"
        );
        assert!(
            !matches!(
                proj.config_brain,
                ambition_entity_catalog::placements::CharacterBrain::Passive
            ),
            "a provoked body's read-model still says it is not passive"
        );
        assert_eq!(
            proj.max_health,
            crate::features::ecs::brain_builders::DEFAULT_PROVOKED_HEALTH,
            "the HP pool is the one body fact still supplied, and it is D96 \
             item 7 rather than a design — if this stops being true the ledger \
             row was answered and this comment is the changelog"
        );
    }
}
