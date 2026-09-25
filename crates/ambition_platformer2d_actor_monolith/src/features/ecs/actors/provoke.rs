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
use ambition_platformer2d_actor_spawn::brain_builders::provoked_mind;

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
    // WHAT THE BODY'S CHARACTER RESOLVED ABOUT BEING PROVOKED
    // (`brain_builders::authored_provoked_policy`: its own policy, else its provider's
    // declared default), asked once by the caller, which also gates the
    // aggression flip on it.
    provoked_policy: Option<(
        ambition_characters::brain::BrainProfile,
        ambition_entity_catalog::BrainProfileId,
    )>,
) {
    // provocation is one body, a different driver, a changed relationship.
    // The body stays exactly as its character built it.
    //
    // ⛔ NO POLICY, NO PROVOCATION. A body whose character and provider state
    // no provoked policy (or that wears no prepared character at all) is not
    // handed an engine-invented fighter: it stays as it was.
    let Some(policy) = provoked_policy else {
        return;
    };
    // ⛔ AN ALREADY-HOSTILE BODY IS NOT RE-DERIVED: that would zero its
    // accumulated fire/footsies/mode cadence on every stimulus. Only the first
    // flip installs a mind, and the binding records the policy only when it is
    // the one installed.
    if disposition.is_peaceful() {
        // The ONE definition of "what provocation produces", shared with
        // construction from a save's provocation fact
        // (`NpcActorSpawnPlan::provoke`). The binding records the policy by
        // canonical id (`ProvokedProfile`), never a roster key (P2.21).
        let mind = provoked_mind(
            policy,
            em.config,
            em.identity,
            repertoire,
            em.abilities.abilities,
        );
        // THE MIND CHANGES. THE BODY DOES NOT.
        //
        // A provoked policy is usually `CharacterBrainTemplate::Smash`, and the
        // Smash brain branches on `obs.self_aerial` with no `can_fly`
        // gate — a flyer's grounded motor outputs are discarded and it steers a
        // 2D `velocity_target` instead. `cfg.can_fly` gates only the hybrid
        // take-off/landing toggle, and it is read off THIS body's `AbilitySet`,
        // so the driver a flying body is handed already knows it flies. A
        // provoked parrot is an angry parrot.
        //
        // A body's pool is settled at construction and provocation has no
        // opinion about it.
        *disposition = ActorDisposition::Hostile;
        // The provoked actor KEEPS its `ActorFaction` identity (no in-place flip to
        // `Enemy`). It hunts + hits its attacker through the per-actor GRUDGE
        // (`ActorAggression::grudge`, set by `apply_actor_stimuli`): targeting treats
        // the grudge entity as a foe, and the victim-side damage gate is `can_damage`
        // (different-faction), which an Npc-vs-Player hit already passes.
        //
        // PROVOCATION CHANGES WHAT A BODY IS, NEVER WHO DRIVES IT: a driven
        // body keeps its `DrivingParticipant`, which suppresses the brain's
        // execution without replacing it. So the provoked brain is installed
        // under the driver too, and it is what resumes when control is released.
        //
        // ONLY THE BRAIN LANDS. What a body fights with is a projection of its
        // identity, its worn equipment and its hand, and getting angry moves
        // none of those.
        //
        // The binding records the provoked source; a no-op for anonymous
        // bodies that carry none.
        //
        // The policy lands in the SAME command as the brain lowered from it, so
        // no tick sees one without the other.
        let (provoked_brain, policy, source) = (
            mind.projection.brain,
            ambition_combat::actor_tuning::ActorPolicy(mind.projection.brain_profile),
            mind.source,
        );
        commands.queue(move |world: &mut bevy::prelude::World| {
            let Ok(mut em) = world.get_entity_mut(entity) else {
                return;
            };
            em.insert((provoked_brain, policy));
            if let Some(mut binding) =
                em.get_mut::<ambition_characters::actor::character_catalog::BrainBinding>()
            {
                binding.source = source;
            }
        });
    }
}
