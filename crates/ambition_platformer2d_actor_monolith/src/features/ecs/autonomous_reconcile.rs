//! What a PROVOCATION produces, projected once and applied by the live flip.
//!
//! Its entry point `reconcile_autonomous_actors` had two re-exports, four doc comments and a test
//! module that called it directly — and zero production call sites. The only system in
//! `AmbitionLoadWorldSet::Reconcile` is `codecs::reconcile_brain_bindings`, which filters on
//! `binding.active_preset()?` — `None` for every provoked and character-first source. Nothing was
//! ever going to invoke the reconstruction.
//!
//! It asserts against `RollbackExecutionStats::lifetime_load_runs` now — the counter
//! `count_load_run` increments inside the very reconciliation set — because at the SHIPPED
//! prediction distance of 0, `LoadWorld` runs zero times and the original tests passed anyway.
//!
//!  what remains here is the LIVE half, which always did the work:
//! [`provoked_projection`] (a mind and a kit, never a body) and
//! [`peaceful_config`] (the generic peaceful NPC seed a catalog switch restores),
//! both applied by `provoke_actor_in_place` and `brain_command`.

use super::HeldItem;
use ambition_characters::actor::character_catalog::{CharacterBodyKind, CharacterCatalog};
use ambition_characters::brain::{Brain, NPC_PATROL_SPEED};
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::actor_tuning::{ActorTuning, BrainProfile};
use ambition_combat::components::CombatKit;
use ambition_combat::CombatCapabilities;
use ambition_entity_catalog::placements::CharacterBrain;

/// What provocation produces: a MIND and a KIT. Never a body.
///
/// The comment three lines above the code that did it already stated the correct invariant:
/// *"provocation is one body, a different driver, a changed relationship. The body stays exactly as
/// its character built it."* It was describing the OTHER branch.
///
///  what a provocation may change is the POLICY the body is driven by, the KIT
/// it swings if it has none of its own, and its relationship to whoever struck
/// it. Its speed, its locomotion, its capabilities and its silhouette are facts
/// about the creature, and being hit is not an argument about any of them.
///
/// do not add a third. Every field on this struct is now a MIND or a KIT;
/// a body fact reappearing here is the ontology growing back.
///
/// and the brain is lowered against the BODY's tuning now, not the
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

/// The projection itself, from a POLICY rather than from a row.
///
/// the policy is pinned equal to the `combatant` row while that row survives
/// (`an_engine_default_provoked_policy_matches_the_combatant_row`); when the row
/// goes, this signature is already the one that stays.
pub(crate) fn provoked_projection(
    brain_profile: BrainProfile,
    current_config: &ActorConfig,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    body: ambition_platformer2d_core::AbilitySet,
) -> ProvokedArchetype {
    // the POLICY is the provoked one; the BODY is the one that was struck.
    let mut hostile_config = current_config.clone();
    hostile_config.brain_profile = brain_profile;
    let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
        &hostile_config,
        combat_kit,
        held_item,
        body,
    );
    // that read-model is a SILHOUETTE, and it was being used as a hostility
    // flag. `evaluate_enemy_ai_output` branched `Passive => aggro 0.0` and
    // `patrol_enabled = !Passive`, so a provoked body needed a NON-`Passive`
    // value to read correctly — and the only one to hand was an archetype name.
    // Both branches ask their `BrainProfile` now, so nothing needs the name.
    //
    //  derived like every other road derives it (`config_brain_for`), which
    // answers `Patrol` for a patrol brain and `Passive` otherwise. The live
    // provoke and the reconstruction agreed on `Custom("combatant")` before and
    // agree on the derived value now, which is this module's central claim.
    let config_brain = crate::features::brain_command::config_brain_for(&brain);

    ProvokedArchetype {
        config_brain,
        brain,
        action_set,
        brain_profile,
    }
}

/// Mirrors `ActorClusterSeed:new_peaceful_npc_in`: an undescribed-pool stroller with default
/// brain-spec / capabilities, its authored combat kit as body-capability action set, and
/// `is_aerial` from the CHARACTER's own locomotion — the catalog's silhouette only for a
/// character nobody prepared (; see the body of [`peaceful_config`] for why that distinction is
/// not cosmetic).
pub(crate) struct PeacefulConfig {
    pub(crate) tuning: ActorTuning,
    pub(crate) brain_profile: BrainProfile,
    pub(crate) capabilities: CombatCapabilities,
    pub(crate) action_set: ambition_characters::brain::ActionSet,
    pub(crate) config_brain: CharacterBrain,
}

pub(crate) fn peaceful_config(
    catalog: &CharacterCatalog,
    // THE PREPARED CAST, asked FIRST — see below.
    prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    character_id: Option<&str>,
    combat_kit: &CombatKit,
    resolved_brain: &Brain,
) -> PeacefulConfig {
    // THIS READ `body_kind: Floating` AND NOTHING ELSE, which is the one rule the invariant
    // list forbids by name: *do not reintroduce `body_kind => is_aerial` as authority*.
    //
    // The only Floating row with no prepared definition is `npc_snakes_on_a_cartesian_plane`,
    // for which the catalog IS the right authority.
    //
    //  it now mirrors `new_peaceful_npc_in` for real, which is what this
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
    // THE SAME TRAP THE `is_aerial` NOTE ABOVE DESCRIBES, IN THE TWO
    // FIELDS BESIDE IT.
    //
    // This installed `max_health: DEFAULT_UNAUTHORED_BODY_HEALTH` and `max_run_speed:
    // MAX_RUN_SPEED` flat, with a comment claiming it was *"the same undescribed-body pool the
    // seed this mirrors installs"*. It is not: `new_peaceful_npc_in` reads the PREPARED
    // character's blueprint for both (P1.10), and falls back to those constants only for a body
    // nobody authored.
    //
    // So the population that could reach it is EMPTY today and springs the day somebody authors a
    // body without a policy — which is an ordinary thing to author.
    let authored_body = character_id
        .and_then(|cid| prepared.and_then(|registry| registry.get(cid)))
        .and_then(|prepared| prepared.body_blueprint().ok());
    let tuning = ActorTuning {
        // STILL FLAT, and that is not an oversight. How fast a body
        // AMBLES is the controller's fact, not the body's — `new_peaceful_npc_in`
        // hard-codes these two for the same reason. A character authoring
        // `run_speed: 400.0` must not make its idle stroll a sprint.
        patrol_speed: NPC_PATROL_SPEED,
        chase_speed: NPC_PATROL_SPEED,
        max_run_speed: authored_body
            .as_ref()
            .map_or(ambition_platformer2d_core::MAX_RUN_SPEED, |body| {
                body.locomotion.run_speed
            }),
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

// `fresh_health_pool(max_health)` stood here. Deleted with the write; a body's pool is now
// settled at construction and nothing re-rolls it because somebody got angry.

#[cfg(test)]
mod tests {
    use super::*;

    fn config_fixture() -> ActorConfig {
        ActorConfig {
            id: "npc".into(),
            name: "Npc".into(),
            tuning: ActorTuning::default(),
            brain_profile: BrainProfile::default(),
            brain: CharacterBrain::Passive,
            sprite_override_npc_name: None,
            sprite_character_id: Some("npc_x".into()),
            // A fixture body, not a seated CPU twin.
            preserves_mirror_symmetry: false,
        }
    }

    /// PROVOCATION PROJECTS NO TUNING AT ALL, so a body keeps everything it
    /// was.
    ///
    /// It existed because the projection assigned an archetype's `tuning()` wholesale and a
    /// provoked NPC silently became `OnRoomReenter` — the kill hook wrote no death flag, save-sync
    /// had nothing to read, and the NPC was rebuilt alive by the next room construction ("kill an
    /// NPC, it respawns immediately", ADR 0022).
    ///
    /// The projection assigns no tuning whatever now — a provocation changes the mind and the
    /// kit, never the body — so the respawn policy survives for the same reason the run speed
    /// does, and the narrow claim became a special case of a general one. Asserting the general
    /// one is what stops a future widening putting a second field back.
    ///
    /// the poison is the second half: the projection must still produce a real
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

        // THE POISON. Without this, deleting the whole projection passes.
        assert_eq!(
            proj.brain_profile,
            crate::features::ecs::brain_builders::default_provoked_policy(),
            "the provoked POLICY is the engine's default — that is the one thing \
             a generic provocation is for"
        );
        //  both of those branches read the `BrainProfile` now, so `Passive` is
        // the CORRECT read-model for a provoked wanderer: hostility is
        // `ActorDisposition`'s and the policy is the profile's, and the
        // integrator-facing silhouette is neither. What must hold is that the
        // value is DERIVED rather than authored — no roster key may reappear here.
        assert!(
            !matches!(
                proj.config_brain,
                ambition_entity_catalog::placements::CharacterBrain::Custom(_)
            ),
            "a provoked body's read-model names an archetype ({:?}) — provocation \
             is spelling a roster key again, which is the whole of what P2.20 \
             deleted",
            proj.config_brain
        );
        assert_eq!(
            proj.config_brain,
            crate::features::brain_command::config_brain_for(&proj.brain),
            "the read-model disagrees with what deriving it from the actual brain \
             gives, so provocation has a second answer to a question one function \
             owns"
        );
        // The endpoint is that there is no such field, so the claim worth pinning is the SHAPE:
        // every field on this projection is a mind or a kit. A new body fact cannot be added
        // without editing this list, which is the point.
        //
        // an EXHAUSTIVE destructure rather than a field read: adding a field
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
    fn grounded_floater() -> ambition_characters::prepared::PreparedCharacterRegistry {
        let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
        let definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "pca",
            "Automaton",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 120.0,
            baseline_free_flight: Some(false),
            ..Default::default()
        });
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    }

    /// A SILHOUETTE IS NOT A CLAIM ABOUT FLIGHT — the one rule the invariant
    /// list forbids reintroducing by name.
    ///
    /// `peaceful_config` read `body_kind: Floating` and nothing else, so a catalog switch back
    /// to peaceful would have made the Perfect Cellular Automaton fly — a character whose own
    /// definition says `baseline_free_flight: Some(false)`.
    #[test]
    fn a_prepared_characters_refusal_to_fly_outranks_a_floating_silhouette() {
        let catalog = catalog();
        let cast = grounded_floater();
        let brain = Brain::StateMachine(ambition_characters::brain::StateMachineCfg::StandStill);

        // THE POISON FIRST, because it is what makes the assertion below about
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

#[cfg(test)]
mod peaceful_body_authority_tests {
    use super::*;

    /// A BODY RELEASED BACK TO PEACEFUL KEEPS THE BODY ITS CHARACTER
    /// AUTHORED.
    ///
    /// the trap this closes had an EMPTY population, and that is the reason to write the test
    /// rather than a reason not to. `peaceful_config` hard-coded `max_run_speed:
    /// MAX_RUN_SPEED` and the undescribed health pool while claiming to install *"the same
    /// undescribed-body pool the seed this mirrors installs"* — and the seed it mirrors reads
    /// the prepared character's blueprint for both (P1.10). A body without a policy is an
    /// ordinary thing to author, and the day one exists the calm-down would have handed it the
    /// player's top speed.
    ///
    /// two terms, both observed. The character's numbers survive, AND an
    /// unauthored body still gets the shared defaults — otherwise "reads the
    /// blueprint" could be satisfied by a projection that reads it for
    /// everything and quietly changes what a catalog-only NPC becomes.
    #[test]
    fn calming_down_restores_the_characters_body_and_not_a_generic_one() {
        use ambition_characters::actor::definition::CharacterDefinition;
        use ambition_characters::actor::CharacterLocomotion;

        const AUTHORED_RUN_SPEED: f32 = 63.0;
        const AUTHORED_HEALTH: i32 = 9;

        use crate::character_runtime::CharacterDefinitionAppExt;
        let mut definition = CharacterDefinition::new("wanderer", "Wanderer", "test")
            .with_locomotion(CharacterLocomotion {
                run_speed: AUTHORED_RUN_SPEED,
                ..Default::default()
            });
        definition.vitals.max_health = Some(AUTHORED_HEALTH);
        let mut app = bevy::prelude::App::new();
        app.register_character(definition);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .clone();

        let calmed = peaceful_config(
            &CharacterCatalog::empty(),
            Some(&prepared),
            Some("wanderer"),
            &CombatKit::default(),
            &ambition_characters::brain::Brain::stand_still(),
        );
        assert_eq!(
            calmed.tuning.max_run_speed, AUTHORED_RUN_SPEED,
            "a body that calmed down was handed the shared top speed instead of \
             the one its character authored — a silent downgrade wearing a \
             controller change, which is what the provoke side was split to stop"
        );
        // THE HEALTH HALF OF THIS TEST IS NOW STRUCTURAL (AC6.2). It
        // asserted `calmed.tuning.max_health == AUTHORED_HEALTH`: the projection
        // restored a pool onto `ActorConfig`, and `apply_catalog_mode` copied the
        // whole tuning back over the live one. That copy never touched
        // `BodyHealth`, so the number it restored was a mirror the respawn path
        // read instead of the body's own pool. The projection has no pool to
        // state now — a controller change cannot reach a body's health because
        // there is nothing on this road that carries it.

        // THE OTHER TERM: a body nobody authored still gets the shared
        // defaults, so this is "ask the character" and not "ask anything".
        let stranger = peaceful_config(
            &CharacterCatalog::empty(),
            Some(&prepared),
            Some("nobody_registered_this"),
            &CombatKit::default(),
            &ambition_characters::brain::Brain::stand_still(),
        );
        assert_eq!(
            stranger.tuning.max_run_speed,
            ambition_platformer2d_core::MAX_RUN_SPEED,
            "a body no character describes stopped getting the undescribed \
             default, so this projection is now answering for creatures nobody \
             authored"
        );
    }
}
