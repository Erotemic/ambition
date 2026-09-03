//! Actor read-model snapshots + the in-place hostile flip.
//!
//! Provoking a peaceful actor (an NPC struck past its retaliation threshold, or
//! a persisted-hostile NPC on load) no longer swaps clusters or churns the
//! entity: every actor is the SAME cluster, so the flip just re-resolves the
//! hostile archetype, overwrites the cluster `config` in place, swaps the
//! `Brain`/`ActionSet`, and flips `ActorDisposition` (the single source of
//! truth for hostility — "enemy" is a state, not a class).

use super::super::*;
use super::*;
use ambition_combat::components::{ActorDisposition, ActorIdentity, CombatKit};

// It began as a matcher over an id, a display name and a dialogue node — *does any of them
// contain "pirate"* — and handed the struck body a whole archetype.
//
// Both branches read their `BrainProfile` now — the policy that always owned both numbers — so
// `provoked_projection` derives the read-model with `config_brain_for` like every other road,
// and provocation names no roster key.
//
// its test-only twin `hostile_spec_for_actor` went with it: its whole purpose
// was to be the roster's side of an equivalence test against this function.

/// Build the read-model mirror components for an actor cluster seed at the given
/// disposition. Peaceful actors get a peaceful `BodyCombat`; hostile actors
/// the full hostile combat state.
pub fn actor_component_snapshot(
    seed: &ambition_body_seed::ActorClusterSeed,
    disposition: ActorDisposition,
) -> (ActorIdentity, ActorDisposition, BodyCombat) {
    // THE SEED'S OWN, not a rebuild (AC6.2). This constructed a fresh
    // `BodyCombat` and filled its one authored flag from
    // `ActorTuning::is_sandbag` — a copy of the character's `practice_target`
    // made at spawn so it could be copied AGAIN here. The seed decides a body's
    // components; this reads the one it decided.
    //
    // AC3.1.A: a fresh body's `BodyCombat` is its reaction history at rest plus
    // one authored flag. Liveness is `BodyHealth`'s, so a seed does not state it
    // here and cannot state it wrongly. AC3.1.D: the flag is authored, so it is
    // written once at construction rather than re-derived every frame by the
    // read-model sync — and the disposition gate that sync applied is
    // deliberately gone: a body authored as a training dummy is one whether or
    // not it currently reads as hostile.
    let combat = seed.combat.clone();
    (
        ActorIdentity::new(seed.config.id.clone(), seed.config.name.clone())
            .with_sprite_override(seed.config.sprite_override_npc_name.clone()),
        disposition,
        combat,
    )
}

/// Hostile spawn read-models (the common case for authored enemies).
pub fn enemy_component_snapshot(
    enemy: &ambition_body_seed::ActorClusterSeed,
) -> (ActorIdentity, ActorDisposition, BodyCombat) {
    actor_component_snapshot(enemy, ActorDisposition::Hostile)
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
    em: &mut super::super::actor_clusters::ActorMut<'_>,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    chase: bool,
) {
    let (brain, _) = super::super::brain_builders::aggressive_brain_and_action_set_for_enemy(
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
pub(crate) fn provoke_actor_in_place(
    commands: &mut Commands,
    entity: Entity,
    em: &mut super::super::actor_clusters::ActorMut<'_>,
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
        let proj = super::super::autonomous_reconcile::provoked_projection(
            super::super::brain_builders::default_provoked_policy(),
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
