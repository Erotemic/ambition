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
//! [`peaceful_config`] (the peaceful mind a catalog switch restores),
//! both applied by `provoke_actor_in_place` and `brain_command`.

use ambition_characters::brain::Brain;
use ambition_combat::actor_tuning::BrainProfile;
use ambition_entity_catalog::placements::CharacterBrain;



/// What RELEASING a provocation restores: the mind, and nothing else.
///
/// ⛔⛤ THIS WAS A WHOLE-BODY MIRROR OF THE PEACEFUL-NPC SEED — tuning,
/// capabilities, `is_aerial` — and every field of it was found wrong in turn
/// (`is_aerial` from the silhouette, `max_run_speed` flat, a health pool) and
/// patched one at a time, because a mirror of construction is a second answer
/// to what construction already answered. The last one was measured in the
/// shipped app (2026-09-23): a character-first NPC built at `patrol_speed 105`
/// / `chase_speed 210` came back from `<<restore_brain>>` at the seed's flat
/// 60 / 60. Provocation changes who is deciding and nothing about the body, so
/// release has only the mind to undo and writes nothing else.
pub(crate) struct PeacefulConfig {
    pub(crate) brain_profile: BrainProfile,
    pub(crate) config_brain: CharacterBrain,
}

pub(crate) fn peaceful_config(resolved_brain: &Brain) -> PeacefulConfig {
    PeacefulConfig {
        brain_profile: BrainProfile::default(),
        // `config.brain` (the integrator read-model) is DERIVED from the
        // resolved autonomous brain through the SHARED helper the spawn plan and
        // runtime switch both use, so the classification can never disagree with
        // the actual brain.
        config_brain: ambition_platformer2d_actor_spawn::brain_builders::config_brain_for(
            resolved_brain,
        ),
    }
}

// `fresh_health_pool(max_health)` stood here. Deleted with the write; a body's pool is now
// settled at construction and nothing re-rolls it because somebody got angry.

#[cfg(test)]
mod tests {
    use super::*;
    // ⚠ TEST-ONLY: the production half of this module stopped naming `ActorConfig`
    // when `provoked_projection` moved to `actor_spawn::conversion`, so importing
    // it at file scope is an unused import in a release build.
    use ambition_combat::actor_tuning::{ActorConfig, ActorTuning};

    fn config_fixture() -> ActorConfig {
        ActorConfig {
            tuning: ActorTuning::default(),
            brain_profile: BrainProfile::default(),
            brain: CharacterBrain::Passive,
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
        let proj = ambition_platformer2d_actor_spawn::brain_builders::provoked_projection(
            ambition_platformer2d_actor_spawn::brain_builders::default_provoked_policy(),
            &config,
            &ambition_combat::components::ActorIdentity::new("npc", "Npc"),
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
            ambition_platformer2d_actor_spawn::brain_builders::default_provoked_policy(),
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
            ambition_platformer2d_actor_spawn::brain_builders::config_brain_for(&proj.brain),
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
        let ambition_platformer2d_actor_spawn::brain_builders::ProvokedArchetype {
            brain_profile: _,
            config_brain: _,
            brain: _,
        } = proj;
    }
}

#[cfg(test)]
mod peaceful_shape_tests {
    use super::*;

    /// The release projection is a MIND. Every field of it is named here in an
    /// exhaustive destructure, so a body fact cannot be added back without
    /// editing this line — the provoke side pins the same shape.
    #[test]
    fn releasing_restores_a_mind_and_no_body_fact() {
        let PeacefulConfig {
            brain_profile: _,
            config_brain: _,
        } = peaceful_config(&Brain::stand_still());
    }
}
