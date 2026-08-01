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

/// Resolve the spawn-brain key of the hostile archetype a peaceful actor turns
/// into when provoked, from its identity + dialogue id (string matching only —
/// no roster enum). Generalized from the old `hostile_enemy_brain_for_npc`.
pub(crate) fn hostile_brain_id_for_actor(
    id: &str,
    name: &str,
    dialogue_id: Option<&str>,
) -> &'static str {
    let id = id.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let dialogue = dialogue_id.unwrap_or("").to_ascii_lowercase();
    // The Perfect Cell-ular Automaton boss: a dedicated reactive Smash
    // archetype with boss HP + a quick jab (see `cellular_automaton_fighter`
    // in character_archetypes.ron). Matches the catalog id, the display name, or
    // the encounter's dialogue node so any of the three placements resolves.
    let looks_like_cellular_automaton = id.contains("cellular_automaton")
        || name.contains("cell-ular automaton")
        || name.contains("cellular automaton")
        || dialogue.contains("cellular_automaton");
    if looks_like_cellular_automaton {
        return "cellular_automaton_fighter";
    }
    let looks_like_pirate_heavy = id.contains("pirate_heavy")
        || name.contains("broadside bess")
        || name.contains("iron mary")
        || name.contains("salt annet")
        || dialogue.contains("pirate_heavy");
    if looks_like_pirate_heavy {
        return "pirate_heavy";
    }
    let looks_like_pirate = id.contains("pirate")
        || name.contains("pirate")
        || name.contains("quartermaster")
        || name.contains("lookout")
        || name.contains("navigator")
        || dialogue.contains("pirate");
    if looks_like_pirate {
        return "pirate_raider";
    }
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
pub(crate) fn hostile_spec_for_actor(
    roster: &super::super::super::enemies::CharacterRoster,
    id: &str,
    name: &str,
    dialogue_id: Option<&str>,
) -> super::super::super::enemies::CharacterArchetypeSpec {
    let brain = ambition_entity_catalog::placements::CharacterBrain::Custom(
        hostile_brain_id_for_actor(id, name, dialogue_id).into(),
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
/// archetype, overwrites the cluster `config` (tuning / brain_spec / brain /
/// caps) so the actor fights as that archetype, keeps its own sprite, resets HP
/// to the hostile pool, and flips `ActorDisposition::Hostile` (the single source
/// of truth — "enemy" is just hostile disposition now). An already-hostile actor
/// just re-derives its aggressive brain (escalation). Shared by the runtime
/// stimulus and save-load provoke paths.
#[allow(clippy::too_many_arguments)]
pub(crate) fn provoke_actor_in_place(
    roster: &super::super::super::enemies::CharacterRoster,
    commands: &mut Commands,
    entity: Entity,
    em: &mut super::super::actor_clusters::ActorMut<'_>,
    disposition: &mut ActorDisposition,
    combat_kit: &CombatKit,
    held_item: Option<&HeldItem>,
    dialogue_id: Option<&str>,
    chase: bool,
) {
    if disposition.is_peaceful() {
        let hostile_id = hostile_brain_id_for_actor(&em.config.id, &em.config.name, dialogue_id);
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
            &spec, hostile_id, em.config, combat_kit, held_item,
        );
        em.config.tuning = proj.tuning;
        // Re-sync gravity to the hostile archetype's locomotion mode — the same
        // invariant `reset_to_spawn` enforces. Without it a peaceful *Floating* NPC
        // (gravity 0) provoking into a grounded archetype would freeze mid-air (the
        // grounded brain never sets `velocity_target`). The Perfect Cell-ular
        // Automaton hits exactly this: floats peacefully, then descends to brawl.
        em.surface.gravity_scale = proj.gravity_scale;
        em.config.brain_spec = proj.brain_spec;
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
        commands
            .entity(entity)
            .insert((proj.brain, proj.action_set));
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
