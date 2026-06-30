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
    // in enemy_archetypes.ron). Matches the catalog id, the display name, or
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
    id: &str,
    name: &str,
    dialogue_id: Option<&str>,
) -> super::super::super::enemies::EnemyArchetypeSpec {
    let brain = ambition_characters::actor::EnemyBrain::Custom(
        hostile_brain_id_for_actor(id, name, dialogue_id).into(),
    );
    super::super::super::enemies::spec_for_brain(&brain)
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
        let spec = super::super::super::enemies::spec_for_brain(
            &ambition_characters::actor::EnemyBrain::Custom(hostile_id.into()),
        );
        em.config.tuning = spec.tuning();
        // Re-sync the body's gravity to match the hostile archetype's locomotion
        // mode — the same invariant `reset_to_spawn` enforces
        // (`gravity_scale = if is_aerial { 0 } else { 1 }`). Without this, a
        // peaceful *Floating* NPC (gravity_scale 0 at spawn) that provokes into a
        // grounded archetype would keep `is_aerial`, so the integrator reads
        // `velocity_target` (which the grounded Smash brain never sets) and the
        // actor freezes in mid-air. The Perfect Cell-ular Automaton hits exactly
        // this path: it floats peacefully, then descends to fight as a grounded
        // brawler. (Aerial archetypes stay aerial; their brains drive
        // `velocity_target`.)
        em.surface.gravity_scale = if em.config.tuning.is_aerial { 0.0 } else { 1.0 };
        em.config.brain_spec = spec.brain_spec();
        em.config.brain = ambition_characters::actor::EnemyBrain::Custom(hostile_id.into());
        // Take on the hostile archetype's HP pool (the peaceful seed spawned with
        // health=1; a provoked actor should fight at full archetype HP).
        *em.health =
            crate::actor::BodyHealth::new(ambition_characters::actor::Health::new(spec.max_health));
        // Keep the actor's own sprite sheet (its NPC name) when hostile — except
        // the Kernel Guide, which uses the default enemy sheet (legacy quirk).
        if em.config.name != "Kernel Guide NPC" {
            em.config.sprite_override_npc_name = Some(em.config.name.clone());
        }
        commands.entity(entity).insert(spec.combat_capabilities());
        *disposition = ActorDisposition::Hostile;
        // A provoked NPC is now a COMBATANT: flip its faction to `Enemy`, not just
        // its disposition. Leaving it `Npc` was a hostility-model bifurcation —
        // `FactionRelations` marks Player↔Enemy hostile but NOT Npc↔Player, so the
        // provoked actor's hits were RELATIONALLY FILTERED in `apply_player_hit_events`:
        // its body-contact FX fired every frame at emission (gated only by the
        // player's vulnerability) yet no damage ever landed, so the player's post-hit
        // i-frame was never set and nothing gated the stream (the "Kernel Guide hits
        // me continuously, no knockback" bug). Faction `Enemy` makes the hit real →
        // damage + knockback + the 0.75 s i-frame that gates re-hits.
        commands
            .entity(entity)
            .insert(crate::combat::components::ActorFaction::Enemy);
        // Build the hostile brain ONCE — on the peaceful→hostile flip. Re-deriving
        // it on every later stimulus (each hit, each re-challenge) would overwrite
        // the brain component with a FRESH one, zeroing all of its accumulated
        // state every tick: the ranged/melee fire cadence, the footsies / dash /
        // blink / neutral-jump timers, mode-dwell hysteresis. That is exactly what
        // turned the Perfect Cell-ular Automaton from a varied duelist into a
        // per-tick glider spammer — a continuously-struck (or re-challenged) actor
        // never got to advance any cadence. An already-hostile actor keeps its
        // live brain and its dueling state; only its target / chase mode update
        // below. (Escalation that needs a genuinely different brain — e.g. a
        // peaceful pirate forced into a Brute — flows through the archetype swap
        // above, which only runs on the flip.)
        let (brain, action_set) =
            super::super::brain_builders::aggressive_brain_and_action_set_for_enemy(
                em.config, combat_kit, held_item,
            );
        commands.entity(entity).insert((brain, action_set));
    }
    if chase {
        em.status.ai_mode = ambition_characters::actor::ai::CharacterAiMode::Chase;
    }
}
