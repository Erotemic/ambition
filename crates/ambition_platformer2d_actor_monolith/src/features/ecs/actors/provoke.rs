//! Provocation: a live peaceful body flips hostile IN PLACE.
//!
//! ⚠ ONE CONCERN: what happens to an ALREADY-SPAWNED actor when something
//! provokes it — the runtime stimulus road and the save-load road share it.
//! It reads and mutates the live cluster view (`crate::actor_clusters::ActorMut`),
//! which is why it lives in the actor kernel and not in the spawn capability:
//! the spawn crate builds brains and bodies from data; it never touches a live
//! entity. The brain BUILDERS it calls (`aggressive_brain_and_action_set_for_enemy`)
//! stay in `ambition_platformer2d_actor_spawn::brain_builders`, because the same
//! builder answers a spawn and a provocation.

use super::*;
use ambition_characters::brain::profile::BrainProfile;
use ambition_characters::brain::Brain;
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::{ActorDisposition, CombatKit};
use ambition_entity_catalog::placements::CharacterBrain;

/// **THE POLICY A BODY IS DRIVEN BY WHEN IT IS PROVOKED AND SAYS NOTHING.**
///
/// the twin of [`default_fighting_kit`] one authority over: that one answers
/// *what does it swing*, this one answers *how does it fight*. They were the two
/// halves the `combatant` archetype row was doing at once, and separating them
/// is what lets the row die — a body is not a policy, and neither is a kit.
///
/// `an_engine_default_provoked_policy_matches_the_combatant_row` pins the numbers against the
/// row while the row survives; when it goes, the constant stands alone and nothing has to
/// change.
///
/// A stage that wants provoked bodies to fight differently says so there; nothing says so yet.
///
/// deliberately NOT a ranged policy. `medium_striker` carried a thrown rock,
/// and using it here turned every provoked NPC — the kernel guide, a merchant —
/// into a rock-thrower instead of a melee attacker like the pirates.
pub fn default_provoked_policy() -> ambition_combat::actor_tuning::BrainProfile {
    ambition_combat::actor_tuning::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Smash,
        aggro_radius: 460.0,
        attack_range: 150.0,
        patrol_effort: 0.6774,
        chase_effort: 1.0,
        ..Default::default()
    }
}

/// Flip an actor hostile IN PLACE — no cluster swap, no entity churn.
///
/// On the first flip (the actor is still peaceful) this re-resolves the hostile
/// archetype, overwrites the cluster `config` (tuning / brain_profile / brain /
/// caps) so the actor fights as that archetype, keeps its own sprite, resets HP
/// to the hostile pool, and flips `ActorDisposition::Hostile` (the single source
/// of truth — "enemy" is just hostile disposition now). An already-hostile actor
/// just re-derives its aggressive brain (escalation). Shared by the runtime
/// stimulus and save-load provoke paths.
/// Rebuild the driver from the policy the config now carries, for a body
/// whose CHARACTER answered the provocation question.
///
/// the action set is untouched, and that is the difference. The archetype
/// path swaps a body's kit because the archetype IS the kit; a character-first
/// body already fights with what its character authored, and provocation has no
/// business editing it. All that changes is who is deciding.
///
/// One rule, both paths.
fn rebuild_provoked_brain(
    commands: &mut Commands,
    entity: Entity,
    em: &mut crate::actor_clusters::ActorMut<'_>,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    chase: bool,
) {
    let (brain, _) = ambition_platformer2d_actor_spawn::brain_builders::aggressive_brain_and_action_set_for_enemy(
        em.config,
        combat_kit,
        held_item,
        em.abilities.abilities,
    );
    if chase {
        // Nothing to seed: a policy that chases does so from its own aggro
        // radius, and a grudge is what points it at whoever struck.
    }
    commands.queue(move |world: &mut bevy::prelude::World| {
        let driven = world
            .get::<ambition_characters::control::DrivingParticipant>(entity)
            .is_some();
        if driven {
            return;
        }
        if let Ok(mut em) = world.get_entity_mut(entity) {
            em.insert(brain);
        }
    });
}

#[allow(clippy::too_many_arguments)]
/// The generic branch asked it for `combatant`'s policy and HP pool, and that was the last
/// thing on this path that knew the archetype ontology existed. Both come from the engine's own
/// defaults now — see `brain_builders::default_provoked_policy`.
pub fn provoke_actor_in_place(
    commands: &mut Commands,
    entity: Entity,
    em: &mut crate::actor_clusters::ActorMut<'_>,
    disposition: &mut ActorDisposition,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
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
    chase: bool,
) {
    // THE CREATURE'S OWN ANSWER, when it has one.
    //
    // A peaceful pirate that gets struck is given a different BODY rather than a different
    // attitude, which is the fused ontology at its most literal, and it is the only thing
    // keeping three archetype rows alive that no level places.
    //
    // provocation is one body, a different driver, a changed relationship.
    // The body stays exactly as its character built it.
    // the ID travels with the value: the value drives the body NOW and the id
    // is what a rewind resolves later, and taking both from one lookup is what
    // stops them disagreeing.
    let authored_provoked = prepared
        .zip(worn_character)
        .and_then(|(registry, character)| {
            let prepared = registry.get(character)?;
            Some((
                prepared.provoked_profile?,
                prepared.provoked_profile_id.clone()?,
            ))
        });
    if let Some((profile, profile_id)) = authored_provoked {
        if disposition.is_peaceful() {
            em.config.brain_profile = profile;
            *disposition = ActorDisposition::Hostile;
        }
        let recorded = profile_id.clone();
        commands.queue(move |world: &mut bevy::prelude::World| {
            if let Some(mut binding) = world
                .get_mut::<ambition_characters::actor::character_catalog::BrainBinding>(entity)
            {
                binding.source =
                    ambition_characters::actor::character_catalog::AutonomousSource::ProvokedProfile {
                        profile: recorded,
                    };
            }
        });
        // the BRAIN is rebuilt from the new policy by the shared writer below,
        // which is also what protects a player-driven body from a silent
        // seizure — see the note further down.
        rebuild_provoked_brain(commands, entity, em, combat_kit, held_item, chase);
        return;
    }
    if disposition.is_peaceful() {
        // THE LIVE PROVOKE PATH NO LONGER ASKS THE ROSTER.
        //
        // this looked `combatant` up with `spec_for_brain` to get a
        // `BrainProfile` and an HP pool — the last reason provocation knew the
        // archetype ontology existed. The policy is the ENGINE's default now
        // (`default_provoked_policy`), stated where a session ruleset will
        // eventually override it.
        //
        // nothing is recorded but the MODE: `binding.provoke()` sets the
        // payloadless `AutonomousSource::ProvokedDefault`, so a rewind resolves
        // the policy the engine states rather than a roster key it must look up
        // (P2.21). `an_engine_default_provoked_policy_matches_the_combatant_row`
        // pins the two equal while the row survives.
        // The ONE definition of "what provocation produces" — shared verbatim with
        // the post-GGRS-load reconstruction (`autonomous_reconcile`), so a provoked
        // actor is identical whether it was just challenged or rebuilt from a
        // snapshot. It builds the hostile brain from the archetype's HOSTILE tuning
        // / brain-spec (an already-hostile actor is NOT re-derived here — that would
        // zero its accumulated fire/footsies/mode cadence every stimulus; escalation
        // that needs a different brain flows through the flip's archetype swap).
        let proj = provoked_projection(
            default_provoked_policy(),
            em.config,
            combat_kit,
            held_item,
            em.abilities.abilities,
        );
        // THE MIND CHANGES. THE BODY DOES NOT.
        //
        // A struck villager did not become an angry villager, it became a `combatant` wearing a
        // villager's name, and the paragraph above this branch has always said otherwise.
        //
        // The premise had gone stale: the engine's default provoked policy is
        // `CharacterBrainTemplate::Smash`, and the Smash brain branches on `obs.self_aerial`
        // with no `can_fly` gate — a flyer's grounded motor outputs are discarded and it steers
        // a 2D `velocity_target` instead. `cfg.can_fly` gates only the hybrid take-off/landing
        // toggle, and it is read off THIS body's `AbilitySet`, so the driver a flying body is
        // handed already knows it flies. A provoked parrot is an angry parrot.
        em.config.brain_profile = proj.brain_profile;
        em.config.brain = proj.config_brain;
        // AND THE LAST BODY FACT WENT WITH IT. This was
        // `*em.health = fresh_health_pool(DEFAULT_PROVOKED_HEALTH)` — a struck
        // body's entire `BodyHealth` replaced by a fresh 4-point pool, current
        // damage and all, because a peaceful placement spawned at `max_health: 1`
        // and a provoked one that kept its own pool died to a single hit.
        //
        // The value is unchanged at 4 and still owns it; what changed is that a body's pool is
        // settled at construction and provocation no longer has an opinion.
        *disposition = ActorDisposition::Hostile;
        // The provoked actor KEEPS its `ActorFaction` identity (no in-place flip to
        // `Enemy`). It hunts + hits its attacker through the per-actor GRUDGE
        // (`ActorAggression::grudge`, set by `apply_actor_stimuli`): targeting treats
        // the grudge entity as a foe, and the victim-side damage gate is `can_damage`
        // (different-faction), which an Npc-vs-Player hit already passes.
        // PROVOCATION CHANGES WHAT A BODY IS, NEVER WHO DRIVES IT.
        //
        // Measured: both seats opened as `Player(0)`/`Player(1)` and seat one flipped 28 frames
        // after its pad went quiet, which is when it traded its first blows.
        //
        // The ACTION SET still lands: what a body fights with is part of what it
        // is, and a provoked fighter should swing the archetype's kit. Only the
        // driver is left alone. The archetype is recorded in `BrainBinding`
        // below either way, so releasing control later resumes the provoked mode
        // rather than the peaceful one.
        let provoked_brain = proj.brain;
        let provoked_action_set = proj.action_set;
        commands.queue(move |world: &mut bevy::prelude::World| {
            let driven = world
                .get::<ambition_characters::control::DrivingParticipant>(entity)
                .is_some();
            let Ok(mut em) = world.get_entity_mut(entity) else {
                return;
            };
            if driven {
                em.insert(provoked_action_set);
            } else {
                em.insert((provoked_brain, provoked_action_set));
            }
        });
        // Record that this body is provoked into the ENGINE's default policy.
        //
        // What actually carries the provoked mode across a rewind is this binding plus the
        // `Brain` cursor, proven end to end by
        // `game/ambition_app/tests/rollback_provoked_actor.rs`.
        //
        // `provoke()` carries nothing now.
        //
        // Deferred so it lands with the `(brain, action_set)` insert; a no-op
        // for anonymous NPCs/enemies that carry no binding.
        commands.queue(move |world: &mut bevy::prelude::World| {
            if let Some(mut binding) =
                world.get_mut::<ambition_characters::actor::character_catalog::BrainBinding>(entity)
            {
                binding.provoke();
            }
        });
    }
    if chase {
        em.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Chase;
    }
}

// ⚠ AND THE RETURN TYPE HAD TO COME TOO, which the first attempt missed: moving
// the function alone left it importing `ProvokedArchetype` from the module it had
// just left, so the edge count did not move at all. **A function and the type it
// returns are one unit for this purpose** — the second time in ten minutes that
// one hop was not far enough.
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
pub struct ProvokedArchetype {
    pub brain_profile: BrainProfile,
    /// The `ActorConfig.brain` read-model marker for a provoked actor.
    pub config_brain: CharacterBrain,
    pub brain: Brain,
    pub action_set: ambition_characters::brain::ActionSet,
}

// ⛔ THAT IS WHY THE FIRST MEASUREMENT SAID "NOT MECHANICAL". Reading
// `provoked_projection` alone showed a genuine feature-layer dependency and I
// recorded it as one; reading what THAT depended on showed a leaf. ⇒ A dependency
// is only real if the thing it depends on is, and ONE HOP IS NOT FAR ENOUGH TO
// TELL.
/// The `ActorConfig.brain` read-model derived from a live autonomous brain, shared
/// by the spawn plan, the runtime switch, and the post-restore reconcile so the
/// classification can never disagree with the actual brain.
pub fn config_brain_for(brain: &Brain) -> ambition_entity_catalog::placements::CharacterBrain {
    use ambition_characters::brain::StateMachineCfg;
    if matches!(brain, Brain::StateMachine(StateMachineCfg::Patrol { .. })) {
        // The `path_id` is cosmetic in the read-model (no read site inspects it —
        // the real path is a separate `ActorMotionPath`), so a derived one is None.
        ambition_entity_catalog::placements::CharacterBrain::Patrol { path_id: None }
    } else {
        ambition_entity_catalog::placements::CharacterBrain::Passive
    }
}

/// The projection itself, from a POLICY rather than from a row.
///
/// the policy is pinned equal to the `combatant` row while that row survives
/// (`an_engine_default_provoked_policy_matches_the_combatant_row`); when the row
/// goes, this signature is already the one that stays.
pub fn provoked_projection(
    brain_profile: BrainProfile,
    current_config: &ActorConfig,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    body: ambition_platformer2d_core::AbilitySet,
) -> ProvokedArchetype {
    // the POLICY is the provoked one; the BODY is the one that was struck.
    let mut hostile_config = current_config.clone();
    hostile_config.brain_profile = brain_profile;
    let (brain, action_set) = ambition_platformer2d_actor_spawn::brain_builders::aggressive_brain_and_action_set_for_enemy(
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
    let config_brain = config_brain_for(&brain);

    ProvokedArchetype {
        config_brain,
        brain,
        action_set,
        brain_profile,
    }
}
