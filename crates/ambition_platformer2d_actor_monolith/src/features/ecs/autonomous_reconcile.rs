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
/// ⭐⭐ **AND AS OF 2026-08-12 THAT SENTENCE IS LITERALLY TRUE** (ledger D101,
/// GPT 5.6's redirect). Two body facts outlived the tuning/sprite/capability
/// purge and this struct carried both as fields:
/// - `gravity_scale`, which re-grounded a flying body so a "grounded" policy
///   could drive it. The premise was stale — the default provoked policy is
///   `Smash`, and the Smash brain steers aerially off `obs.self_aerial` with no
///   `can_fly` gate, so a flyer's driver already knew it flies.
/// - `max_health`, which replaced the body's whole `BodyHealth` with a fresh
///   4-point pool because a peaceful placement spawned at `1`. The `1` was the
///   defect: an undescribed body is undescribed before anybody hits it, so the
///   number moved to `DEFAULT_UNAUTHORED_BODY_HEALTH` at the two spawn seeds
///   and provocation stopped writing health.
///
/// ⛔ **do not add a third.** Every field on this struct is now a MIND or a KIT;
/// a body fact reappearing here is the ontology growing back.
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
    /// The `ActorConfig.brain` read-model marker for a provoked actor.
    pub config_brain: CharacterBrain,
    pub brain: Brain,
    pub action_set: ambition_characters::brain::ActionSet,
}

/// **The projection itself, from a POLICY rather than from a row.**
///
/// ⭐ the live generic-provocation path calls this with
/// [`default_provoked_policy`](super::brain_builders::default_provoked_policy),
/// so provoking a body no longer touches the archetype roster at all — that
/// lookup was the last reason the live path knew the ontology existed.
///
/// ⚠ the policy is pinned equal to the `combatant` row while that row survives
/// (`an_engine_default_provoked_policy_matches_the_combatant_row`); when the row
/// goes, this signature is already the one that stays.
pub(crate) fn provoked_projection(
    brain_profile: BrainProfile,
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
        config_brain,
        brain,
        action_set,
        brain_profile,
    }
}

/// The fixed peaceful catalog config a catalog-backed NPC spawns with. Mirrors
/// `ActorClusterSeed::new_peaceful_npc_in`: an undescribed-pool stroller with default
/// brain-spec / capabilities, its authored combat kit as body-capability action
/// set, and `is_aerial` from the CHARACTER's own locomotion — the catalog's
/// silhouette only for a character nobody prepared (D89; see the body of
/// [`peaceful_config`] for why that distinction is not cosmetic).
pub(crate) struct PeacefulConfig {
    pub(crate) tuning: ActorTuning,
    pub(crate) brain_profile: BrainProfile,
    pub(crate) capabilities: CombatCapabilities,
    pub(crate) action_set: ambition_characters::brain::ActionSet,
    pub(crate) config_brain: CharacterBrain,
}

pub(crate) fn peaceful_config(
    catalog: &CharacterCatalog,
    // **THE PREPARED CAST, asked FIRST** — see below.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    character_id: Option<&str>,
    combat_kit: &CombatKit,
    resolved_brain: &Brain,
) -> PeacefulConfig {
    // ⛔⛔ **THIS READ `body_kind: Floating` AND NOTHING ELSE**, which is the one
    // rule the invariant list forbids by name: *do not reintroduce
    // `body_kind => is_aerial` as authority*. D89 settled that a body kind
    // describes a SHAPE — `CharacterLocomotion::baseline_free_flight` is
    // `Option<bool>` precisely so a character can refuse flight out loud, which a
    // silhouette cannot express — and the Perfect Cellular Automaton is the
    // standing example: `body_kind: Floating` in its catalog row,
    // `baseline_free_flight: Some(false)` in its own definition.
    //
    // ⚠ **the wrong answer was UNREACHABLE, and that is why it survived.** Six
    // catalog rows say `Floating`; every one of them that is PREPARED also
    // authors an autonomous profile, and `apply_catalog_mode` returns before
    // this call when a character states its own policy. The only Floating row
    // with no prepared definition is `npc_snakes_on_a_cartesian_plane`, for
    // which the catalog IS the right authority. So this was a trap rather than a
    // bug — and a trap that springs the day somebody deletes a profile.
    //
    // ⇒ it now mirrors `new_peaceful_npc_in` for real, which is what this
    // function's own doc has always claimed: the PREPARED character answers, and
    // the catalog is the fallback for a character nobody registered.
    let is_aerial = character_id
        .map(|cid| {
            prepared
                .and_then(|registry| registry.get(cid))
                .and_then(|prepared| prepared.locomotion)
                .and_then(|locomotion| locomotion.baseline_free_flight)
                .unwrap_or_else(|| {
                    matches!(catalog.body_kind(cid), Some(CharacterBodyKind::Floating))
                })
        })
        .unwrap_or(false);
    let tuning = ActorTuning {
        // The same undescribed-body pool the seed this mirrors installs; a
        // catalog switch back to peaceful must not resize the body either.
        max_health: ambition_characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH,
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

// ⛔ `fresh_health_pool(max_health)` stood here. Its one caller was the live
// provoke flip, which used it to swap a struck body's whole `BodyHealth` for a
// fresh 4-point pool — the last body mutation in provocation (D101). Deleted
// with the write; a body's pool is now settled at construction and nothing
// re-rolls it because somebody got angry.

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
        // ⭐⭐ **AND THE STRUCTURAL ASSERTION THAT REPLACED THE HP ONE.** This
        // used to read `proj.max_health == DEFAULT_PROVOKED_HEALTH`, pinning the
        // last body fact a provocation supplied. The endpoint is that there is
        // no such field, so the claim worth pinning is the SHAPE: every field on
        // this projection is a mind or a kit. A new body fact cannot be added
        // without editing this list, which is the point.
        //
        // ⚠ an EXHAUSTIVE destructure rather than a field read: adding a field
        // breaks this line, where reading four fields would silently ignore a
        // fifth.
        let ProvokedArchetype {
            brain_profile: _,
            config_brain: _,
            brain: _,
            action_set: _,
        } = proj;
    }
}

#[cfg(test)]
mod peaceful_flight_tests {
    use super::*;

    const FLOATING_CATALOG: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {
        "pca": (
            display_name: "Automaton", spritesheet: "x.png", manifest: "x_spritesheet.ron",
            tier: MainHall, body_kind: Floating, composition: None,
            default_brain: "stand_still", default_action_set: "peaceful", tags: [],
        ),
    },
)"#;

    fn catalog() -> CharacterCatalog {
        CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(FLOATING_CATALOG),
        )
    }

    /// A character with the PCA's exact disagreement: a floating silhouette and
    /// an authored refusal to fly.
    fn grounded_floater() -> crate::character_runtime::PreparedCharacterRegistry {
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        let definition =
            crate::character_runtime::CharacterDefinition::new("pca", "Automaton", "test")
                .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                    run_speed: 120.0,
                    baseline_free_flight: Some(false),
                    ..Default::default()
                });
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// **A SILHOUETTE IS NOT A CLAIM ABOUT FLIGHT** — the one rule the invariant
    /// list forbids reintroducing by name.
    ///
    /// ⛔⛔ `peaceful_config` read `body_kind: Floating` and nothing else, so a
    /// catalog switch back to peaceful would have made the Perfect Cellular
    /// Automaton fly — a character whose own definition says
    /// `baseline_free_flight: Some(false)`. D89 settled this: a body kind
    /// describes a SHAPE, and `Option<bool>` exists precisely so a character can
    /// refuse flight out loud.
    ///
    /// ⚠ **the wrong answer was UNREACHABLE, which is why it survived.** Six
    /// catalog rows say `Floating`; every PREPARED one also authors an autonomous
    /// profile, and `apply_catalog_mode` returns before this call when a
    /// character states its own policy. It was a trap that springs the day
    /// somebody deletes a profile — so it is fixed at the rule rather than left
    /// resting on a reachability argument nobody would re-derive.
    #[test]
    fn a_prepared_characters_refusal_to_fly_outranks_a_floating_silhouette() {
        let catalog = catalog();
        let cast = grounded_floater();
        let brain = Brain::StateMachine(ambition_characters::brain::StateMachineCfg::StandStill);

        // ⭐ THE POISON FIRST, because it is what makes the assertion below about
        // PRECEDENCE. With no prepared cast the catalog is the only authority and
        // it really does say this body floats — so an empty-catalog fixture, or a
        // resolver that answered `false` for everything, could not fake this pair.
        let unprepared =
            peaceful_config(&catalog, None, Some("pca"), &CombatKit::default(), &brain);
        assert!(
            unprepared.tuning.is_aerial,
            "the fixture catalog must genuinely say `Floating`, or the test below \
             passes for the wrong reason"
        );

        let prepared = peaceful_config(
            &catalog,
            Some(&cast),
            Some("pca"),
            &CombatKit::default(),
            &brain,
        );
        assert!(
            !prepared.tuning.is_aerial,
            "the character authored `baseline_free_flight: Some(false)` and the \
             catalog's silhouette overruled it — `body_kind => is_aerial` is the \
             authority D89 deleted, and this is the PCA exactly"
        );
    }
}
