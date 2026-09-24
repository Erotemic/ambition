//! Provocation: a live peaceful body flips hostile IN PLACE.
//!
//! ⚠ ONE CONCERN: what happens to an ALREADY-SPAWNED actor when something
//! provokes it. A person the save remembers as provoked is not flipped here:
//! construction builds them provoked from the same answer
//! (`brain_builders::provoked_mind`, via `NpcActorSpawnPlan::provoke`).
//! It reads and mutates the live cluster view (`crate::actor_clusters::ActorMut`),
//! which is why it lives in the actor kernel and not in the spawn capability:
//! the spawn crate builds brains and bodies from data; it never touches a live
//! entity. The brain BUILDERS it calls (`aggressive_brain_for_enemy`)
//! stay in `ambition_platformer2d_actor_spawn::brain_builders`, because the same
//! builder answers a spawn and a provocation.

use super::*;
use ambition_combat::components::ActorDisposition;
use ambition_platformer2d_actor_spawn::brain_builders::{authored_provoked_policy, provoked_mind};

/// Flip a PEACEFUL actor hostile in place — no cluster swap, no entity churn —
/// installing the mind [`provoked_mind`] answers. An already-hostile actor is
/// left as it is.
#[allow(clippy::too_many_arguments)]
pub fn provoke_actor_in_place(
    commands: &mut Commands,
    entity: Entity,
    em: &mut crate::actor_clusters::ActorMut<'_>,
    disposition: &mut ActorDisposition,
    // WHAT THE BODY CAN DO, read rather than rebuilt: provocation changes who
    // is deciding, never the body's repertoire, so the brain choice consumes
    // the live projection instead of folding a second answer from a subset of
    // its inputs.
    repertoire: Option<&ambition_characters::brain::ActionSet>,
    // It existed so a provoked body could be recognised by its encounter's dialogue id — one of
    // three prose spellings `hostile_brain_id_for_actor` guessed at — and a creature that publishes
    // its own provoked policy needs none of them. WHICH CHARACTER THIS BODY IS — the GAMEPLAY
    // identity.
    worn_character: Option<&str>,
    // WHAT THE BODY'S OWN CHARACTER SAYS ABOUT BEING PROVOKED, if it says
    // anything — see `CharacterDefinition::provoked_profile_ref`. `Option`
    // because most compositions register no cast, and no character today states
    // one.
    prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
) {
    // THE CREATURE'S OWN ANSWER, when it has one.
    //
    // provocation is one body, a different driver, a changed relationship.
    // The body stays exactly as its character built it.
    let authored = prepared
        .zip(worn_character)
        .and_then(|(registry, character)| authored_provoked_policy(registry, character));
    // ⛔ AN ALREADY-HOSTILE BODY IS NOT RE-DERIVED: that would zero its
    // accumulated fire/footsies/mode cadence on every stimulus. Only the first
    // flip installs a mind, and the binding records the policy only when it is
    // the one installed.
    if disposition.is_peaceful() {
        // The ONE definition of "what provocation produces", shared with
        // construction from a save's provocation fact
        // (`NpcActorSpawnPlan::provoke`): the character's own provoked policy
        // when it authors one, else the ENGINE's default
        // (`default_provoked_policy`), stated where a session ruleset will
        // eventually override it. The binding records the MODE — payloadless
        // `ProvokedDefault`, or `ProvokedProfile` by canonical id — never a
        // roster key to look up (P2.21).
        let mind = provoked_mind(
            authored,
            em.config,
            em.identity,
            repertoire,
            em.abilities.abilities,
        );
        // THE MIND CHANGES. THE BODY DOES NOT.
        //
        // The engine's default provoked policy is `CharacterBrainTemplate::Smash`,
        // and the Smash brain branches on `obs.self_aerial` with no `can_fly`
        // gate — a flyer's grounded motor outputs are discarded and it steers a
        // 2D `velocity_target` instead. `cfg.can_fly` gates only the hybrid
        // take-off/landing toggle, and it is read off THIS body's `AbilitySet`,
        // so the driver a flying body is handed already knows it flies. A
        // provoked parrot is an angry parrot.
        //
        // A body's pool is settled at construction and provocation has no
        // opinion about it.
        em.config.brain_profile = mind.projection.brain_profile;
        *disposition = ActorDisposition::Hostile;
        // The provoked actor KEEPS its `ActorFaction` identity (no in-place flip to
        // `Enemy`). It hunts + hits its attacker through the per-actor GRUDGE
        // (`ActorAggression::grudge`, set by `apply_actor_stimuli`): targeting treats
        // the grudge entity as a foe, and the victim-side damage gate is `can_damage`
        // (different-faction), which an Npc-vs-Player hit already passes.
        //
        // PROVOCATION CHANGES WHAT A BODY IS, NEVER WHO DRIVES IT: a body a
        // participant is driving keeps its seat, and its own brain waits.
        //
        // ONLY THE BRAIN LANDS. What a body fights with is a projection of its
        // identity, its worn equipment and its hand, and getting angry moves
        // none of those.
        //
        // The binding is written whether or not the body is driven, so
        // releasing control later resumes the provoked mode rather than the
        // peaceful one; a no-op for anonymous bodies that carry no binding.
        let (provoked_brain, source) = (mind.projection.brain, mind.source);
        commands.queue(move |world: &mut bevy::prelude::World| {
            let driven = world
                .get::<ambition_characters::control::DrivingParticipant>(entity)
                .is_some();
            let Ok(mut em) = world.get_entity_mut(entity) else {
                return;
            };
            if !driven {
                em.insert(provoked_brain);
            }
            if let Some(mut binding) =
                em.get_mut::<ambition_characters::actor::character_catalog::BrainBinding>()
            {
                binding.source = source;
            }
        });
    }
}
