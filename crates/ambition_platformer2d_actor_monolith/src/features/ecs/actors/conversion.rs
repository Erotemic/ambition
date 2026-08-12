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

/// **What a peaceful actor with NO authored provoked policy becomes.** One
/// answer, for everybody.
///
/// ⭐⭐ **it used to be a string matcher over identity and dialogue, and every
/// arm is now deleted** (ledger D84 and D89, 2026-08-11). It read a body's id,
/// display name and encounter dialogue node looking for `"pirate"`,
/// `"iron mary"`, `"cellular automaton"` — guessing at how content spells itself
/// — and handed back a whole ARCHETYPE. Eleven characters answered those arms and
/// every one of them publishes its own `provoked_profile_ref` now, so a provoked
/// creature rebuilds the mind IT authored rather than one recognised by its name.
///
/// ⚠ **the parameters are gone with the arms.** A function that takes an identity
/// and cannot use it invites the next matcher; taking nothing says what is true —
/// this is the fallback for a body whose character said nothing.
pub(crate) fn hostile_brain_id_for_actor() -> &'static str {
    // ⛔⛔ **THE CELLULAR-AUTOMATON ARM IS DELETED TOO (2026-08-11, ledger D89),
    // and it was the LAST one.** It asked whether an id, a display name or a
    // dialogue node contained "cellular automaton" — three spellings, because a
    // matcher on prose has to guess how content spells itself — and handed the
    // body a whole archetype.
    //
    // ⭐ both automatons publish `cellular_duelist` as their PROVOKED profile
    // now, so a creature that is attacked rebuilds the mind it authored instead
    // of being recognised by its name. That is the whole of what this function
    // was for.
    // ⛔ **THE TWO PIRATE ARMS ARE DELETED (2026-08-11, ledger D84), with the
    // rows they pointed at.** They asked whether an id, a display name or a
    // dialogue node contained `"pirate"` — or one of `"broadside bess"` /
    // `"iron mary"` / `"salt annet"` — and handed the body a whole archetype.
    //
    // All nine characters that answered them now state their own
    // `provoked_profile_ref`, and the branch above takes it. Measured before
    // deleting, twice: every pirate-named placement in every world carries a
    // `character_id` (so none can fall through to here), and a body built from a
    // named character keeps that id through construction (so the branch above can
    // find it).
    // Generic provoked NPC = a melee brawler (`combatant`: Smash + melee Swipe,
    // NO ranged), matching how the pirates fight. Deliberately NOT
    // `medium_striker` — that archetype carries a ranged Rock, which turned every
    // provoked NPC (kernel guide, merchant, ...) into a rock-thrower instead of a
    // melee attacker like the pirates.
    "combatant"
}

/// Resolve the hostile archetype spec a peaceful actor would become when
/// provoked. Spawn-time use: feeds the actor's stored `CombatKit` so a provoked
/// NPC fights with the right weapon. Generalized from `hostile_enemy_spec_for_npc`.
/// ⚠ its identity parameters went with the matcher's (see
/// [`hostile_brain_id_for_actor`]): there is one fallback spec, and a signature
/// that still asked who the body was would imply otherwise.
pub(crate) fn hostile_spec_for_actor(
    roster: &super::super::super::enemies::CharacterRoster,
) -> super::super::super::enemies::ArchetypeSpec {
    let brain = ambition_entity_catalog::placements::CharacterBrain::Custom(
        hostile_brain_id_for_actor().into(),
    );
    roster.spec_for_brain(&brain)
}

/// Build the read-model mirror components for an actor cluster seed at the given
/// disposition. Peaceful actors get a peaceful `BodyCombat`; hostile actors
/// the full hostile combat state.
pub fn actor_component_snapshot(
    seed: &super::super::actor_clusters::ActorClusterSeed,
    disposition: ActorDisposition,
) -> (
    ActorIdentity,
    ActorDisposition,
    BodyCombat,
    ActorIntent,
    ActorCooldowns,
) {
    // A freshly-seeded body has no damage-blink; the reaction timers (hit_flash /
    // i-frame) live on the spawned `BodyCombat` and start at 0.
    let combat = if disposition.is_hostile() {
        BodyCombat::hostile(
            seed.health.alive(),
            0.0,
            seed.attack.windup_remaining(),
            seed.attack.active_remaining(),
            seed.config.tuning.is_sandbag,
        )
    } else {
        BodyCombat::peaceful(0, 0.0)
    };
    (
        ActorIdentity::new(seed.config.id.clone(), seed.config.name.clone())
            .with_sprite_override(seed.config.sprite_override_npc_name.clone()),
        disposition,
        combat,
        ActorIntent::new(seed.status.ai_mode),
        ActorCooldowns {
            attack_cooldown: seed.attack.cooldown,
            respawn_timer: seed.status.respawn_timer,
        },
    )
}

/// Hostile spawn read-models (the common case for authored enemies).
pub fn enemy_component_snapshot(
    enemy: &super::super::actor_clusters::ActorClusterSeed,
) -> (
    ActorIdentity,
    ActorDisposition,
    BodyCombat,
    ActorIntent,
    ActorCooldowns,
) {
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
/// **Rebuild the driver from the policy the config now carries**, for a body
/// whose CHARACTER answered the provocation question.
///
/// ⚠ **the action set is untouched, and that is the difference.** The archetype
/// path swaps a body's kit because the archetype IS the kit; a character-first
/// body already fights with what its character authored, and provocation has no
/// business editing it. All that changes is who is deciding.
///
/// ⛔ **and never for a player-driven body.** Inserting a brain over
/// `Brain::Player(slot)` is a silent seizure — the defect the archetype path
/// below documents at length, which cost a couch session before anybody read the
/// brain component. One rule, both paths.
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
            .get::<ambition_characters::brain::Brain>(entity)
            .is_some_and(ambition_characters::brain::Brain::is_player);
        if driven {
            return;
        }
        if let Ok(mut em) = world.get_entity_mut(entity) {
            em.insert(brain);
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn provoke_actor_in_place(
    roster: &super::super::super::enemies::CharacterRoster,
    commands: &mut Commands,
    entity: Entity,
    em: &mut super::super::actor_clusters::ActorMut<'_>,
    disposition: &mut ActorDisposition,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    // ⚠ **the DIALOGUE NODE is gone with the matcher it fed** (ledger D89). It
    // existed so a provoked body could be recognised by its encounter's dialogue
    // id — one of three prose spellings `hostile_brain_id_for_actor` guessed at —
    // and a creature that publishes its own provoked policy needs none of them.
    // ⭐⭐ **WHICH CHARACTER THIS BODY IS — the GAMEPLAY identity.**
    //
    // ⛔ this read `em.config.sprite_character_id`, which is the identity its ART
    // resolves through, and Jon's second redirect (P1) named it as the exact
    // inversion D73 already removed from authored enemy spawning: presentation
    // deciding a gameplay question. It happened to agree today because the sprite
    // id and the worn id are the same string for every migrated body — which is
    // the kind of agreement that stops holding the first time a character borrows
    // another's art, and then a provoked body adopts a stranger's policy.
    worn_character: Option<&str>,
    // **WHAT THE BODY'S OWN CHARACTER SAYS ABOUT BEING PROVOKED**, if it says
    // anything — see `CharacterDefinition::provoked_profile_ref`. `Option`
    // because most compositions register no cast, and no character today states
    // one.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    chase: bool,
) {
    // ⭐⭐ **THE CREATURE'S OWN ANSWER, when it has one** (ledger D84).
    //
    // ⛔ what it replaces is `hostile_brain_id_for_actor`: provocation picks a
    // hostile ARCHETYPE by substring-matching an id, a display name or a
    // dialogue node — *does any of them contain "pirate"* — and then hands the
    // body that archetype's tuning, HP pool and capabilities. A peaceful pirate
    // that gets struck is given a different BODY rather than a different
    // attitude, which is the fused ontology at its most literal, and it is the
    // only thing keeping three archetype rows alive that no level places.
    //
    // ⭐ provocation is one body, a different driver, a changed relationship.
    // The body stays exactly as its character built it.
    // ⚠ the ID travels with the value: the value drives the body NOW and the id
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
        // ⭐⭐ **RECORD THE POLICY AS THE AUTONOMOUS SOURCE** (Jon's second
        // redirect, P1). Without this a rewind rereads `AutonomousSource`, finds
        // whatever the body carried before it was provoked, and rebuilds the
        // peaceful mind — so a provoke would not survive a rollback at all.
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
        // ⚠ the BRAIN is rebuilt from the new policy by the shared writer below,
        // which is also what protects a player-driven body from a silent
        // seizure — see the note further down.
        rebuild_provoked_brain(commands, entity, em, combat_kit, held_item, chase);
        return;
    }
    if disposition.is_peaceful() {
        let hostile_id = hostile_brain_id_for_actor();
        let spec = roster.spec_for_brain(
            &ambition_entity_catalog::placements::CharacterBrain::Custom(hostile_id.into()),
        );
        // The ONE definition of "what provocation produces" — shared verbatim with
        // the post-GGRS-load reconstruction (`autonomous_reconcile`), so a provoked
        // actor is identical whether it was just challenged or rebuilt from a
        // snapshot. It builds the hostile brain from the archetype's HOSTILE tuning
        // / brain-spec (an already-hostile actor is NOT re-derived here — that would
        // zero its accumulated fire/footsies/mode cadence every stimulus; escalation
        // that needs a different brain flows through the flip's archetype swap).
        let proj = super::super::autonomous_reconcile::project_provoked_archetype(
            &spec,
            hostile_id,
            em.config,
            combat_kit,
            held_item,
            em.abilities.abilities,
        );
        em.config.tuning = proj.tuning;
        // Re-sync gravity to the hostile archetype's locomotion mode — the same
        // invariant `reset_to_spawn` enforces. Without it a peaceful *Floating* NPC
        // (gravity 0) provoking into a grounded archetype would freeze mid-air (the
        // grounded brain never sets `velocity_target`). The Perfect Cell-ular
        // Automaton hits exactly this: floats peacefully, then descends to brawl.
        em.surface.gravity_scale = proj.gravity_scale;
        em.config.brain_profile = proj.brain_profile;
        em.config.brain = proj.config_brain;
        // Take on the hostile archetype's HP pool (the peaceful seed spawned at
        // health=1; a provoked actor fights at full archetype HP).
        *em.health = super::super::autonomous_reconcile::fresh_health_pool(proj.max_health);
        em.config.sprite_override_npc_name = proj.sprite_override_npc_name;
        commands.entity(entity).insert(proj.capabilities);
        *disposition = ActorDisposition::Hostile;
        // The provoked actor KEEPS its `ActorFaction` identity (no in-place flip to
        // `Enemy`). It hunts + hits its attacker through the per-actor GRUDGE
        // (`ActorAggression::grudge`, set by `apply_actor_stimuli`): targeting treats
        // the grudge entity as a foe, and the victim-side damage gate is `can_damage`
        // (different-faction), which an Npc-vs-Player hit already passes.
        // ⛔ **PROVOCATION CHANGES WHAT A BODY IS, NEVER WHO DRIVES IT.**
        //
        // This inserted the archetype's brain unconditionally, and for a body
        // under player control that is a silent seizure: the first hit a SEATED
        // FIGHTER took replaced its `Brain::Player(slot)` with the Smash state
        // machine, in place, permanently — activation is one-shot and never
        // rebinds — so a human's fighter became a CPU mid-fight and the couch
        // test read it as input crosstalk. Measured: both seats opened as
        // `Player(0)`/`Player(1)` and seat one flipped 28 frames after its pad
        // went quiet, which is when it traded its first blows.
        //
        // ⚠ **every other brain writer already knew this** — `brain_command`,
        // `reconcile_autonomous_actors` and `reconcile_brain_bindings` all open
        // with `if brain.is_player() { .. }` and update the SOURCE that resumes
        // instead of the live brain. This was the one path that did not, and it
        // was unreachable until a player-driven body could also be a provokable
        // actor. Seating one is what made that ordinary.
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
                .get::<ambition_characters::brain::Brain>(entity)
                .is_some_and(ambition_characters::brain::Brain::is_player);
            let Ok(mut em) = world.get_entity_mut(entity) else {
                return;
            };
            if driven {
                em.insert(provoked_action_set);
            } else {
                em.insert((provoked_brain, provoked_action_set));
            }
        });
        // Record the provoked ARCHETYPE in the autonomous binding. The stable
        // archetype id is all a rewind needs: the whole provoked config above is a
        // deterministic function of it (via `project_provoked_archetype`), so a
        // snapshot reconcile RERUNS that construction in either rewind direction
        // rather than rebuilding the catalog default over it. Deferred so it lands
        // with the `(brain, action_set)` insert; a no-op for anonymous NPCs/enemies
        // that carry no binding.
        commands.queue(move |world: &mut bevy::prelude::World| {
            if let Some(mut binding) =
                world.get_mut::<ambition_characters::actor::character_catalog::BrainBinding>(entity)
            {
                binding.provoke(
                    ambition_characters::actor::character_catalog::HostileArchetypeId::new(
                        hostile_id,
                    ),
                );
            }
        });
    }
    if chase {
        em.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Chase;
    }
}
