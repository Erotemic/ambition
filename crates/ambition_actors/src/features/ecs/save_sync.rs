//! Mirror persisted save state onto ECS-owned feature actors, bosses,
//! and switches.
//!
//! These systems run at room-load time so authored entities reflect
//! flags carried in the SandboxSave (provoked NPCs, dead enemies,
//! cleared bosses, flipped switches) before gameplay resumes.

use super::*;

/// Mirror save-derived actor state onto ECS-owned authored NPC/enemy actors.
///
/// Provoked NPCs load as hostile actors, and persisted non-respawning enemy
/// deaths stay dead across room reloads. Dynamic encounter mobs are ignored
/// because their lifecycle belongs to encounter state.
pub fn sync_ecs_actors_with_save(
    mut commands: Commands,
    roster: Res<crate::features::CharacterRoster>,
    save: Res<ambition_persistence::save::SandboxSave>,
    // A persisted-hostile NPC re-establishes its grudge against a stable player
    // slot on load (the original attacker entity doesn't survive a save round-trip;
    // single-player has exactly one slot to be angry at).
    players: Query<(Entity, Option<&crate::control::PlayerSlot>), With<crate::actor::PlayerEntity>>,
    mut actors: Query<
        (
            Entity,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut BodyCombat,
            &mut ActorIntent,
            &mut ActorCooldowns,
            &mut ActorAggression,
            &CombatKit,
            Option<&HeldItem>,
            // Talkable actors (NPCs) carry the interaction payload + a persisted
            // `npc_<id>_hostile` provoke flag.
            Option<&ActorInteraction>,
            super::actor_clusters::ActorClusterQueryData,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let data = save.data();
    // AMBITION_REVIEW(determinism): Bevy query iteration order is not a stable
    // multiplayer fallback. When the persisted attacker entity cannot be restored,
    // anchor hostility to the lowest PlayerSlot so save-load behavior is replay-safe.
    let stable_player_grudge = players
        .iter()
        .min_by_key(|(_, slot)| slot.copied().unwrap_or(crate::control::PlayerSlot::PRIMARY))
        .map(|(entity, _)| entity);
    for (
        entity,
        mut identity,
        mut disposition,
        mut combat,
        mut intent,
        mut cooldowns,
        mut aggression,
        combat_kit,
        held_item,
        interaction,
        mut cq,
    ) in &mut actors
    {
        let id = cq.as_actor_mut().config.id.clone();
        let dead_on_load = data.flag(&format!("enemy_{id}_dead"))
            || data.flag(&format!(
                "enemy_{id}{}",
                crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
            ));

        if interaction.is_some() && data.flag(&super::super::npcs::npc_flag_id(&id)) {
            // Persisted-hostile NPC: flip it hostile IN PLACE on load (no cluster
            // swap), keeping its entity + sprite.
            let dialogue_id = interaction.and_then(|i| match &i.interactable.kind {
                ambition_interaction::InteractionKind::Npc { dialogue_id, .. } => {
                    dialogue_id.as_deref()
                }
                _ => None,
            });
            aggression.mode = AggressionMode::Hostile;
            aggression.grudge = stable_player_grudge;
            let mut em = cq.as_actor_mut();
            super::actors::provoke_actor_in_place(
                &roster,
                &mut commands,
                entity,
                &mut em,
                &mut disposition,
                combat_kit,
                held_item,
                dialogue_id,
                false,
            );
        }

        // Liveness applies to EVERY persistent actor, hostile-flagged or not
        // (ADR 0022). The old shape only reached the dead-flag for provoked
        // NPCs and bare enemies — a killed UNPROVOKED peaceful NPC fell
        // through both branches and respawned alive on every room load.
        // Zeroing HP is the single liveness authority — `alive()` reads it.
        // (`encounter:*` keeps its own state machine; sandbags are InPlace
        // and never write flags, so the guards are belt-and-suspenders.)
        {
            let em = cq.as_actor_mut();
            if !em.config.id.starts_with("encounter:")
                && !em.config.tuning.is_sandbag
                && dead_on_load
            {
                em.health.health.current = 0;
            }
        }

        let em = cq.as_actor_mut();
        sync_actor_components_from_cluster(
            &em,
            *disposition,
            &mut identity,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

/// Mirror persisted boss-cleared state onto ECS-owned boss actors.
pub fn sync_ecs_bosses_with_save(
    save: Res<ambition_persistence::save::SandboxSave>,
    mut bosses: Query<
        (
            super::boss_clusters::BossClusterQueryData,
            &mut ambition_characters::actor::BodyHealth,
            Option<&mut BossDeathAnimation>,
            Option<&mut BossPhase>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (feature, mut health, death_anim, phase) in &mut bosses {
        // R4: "cleared" is keyed to this PLACEMENT, not the archetype. Shared
        // predicate (`boss_is_cleared`) with the per-tick encounter driver so
        // they can't drift.
        if super::boss_clusters::boss_is_cleared(&save, &feature.config) {
            health.health.current = 0;
            if let Some(mut death_anim) = death_anim {
                death_anim.clear();
            }
            if let Some(mut phase) = phase {
                *phase = BossPhase::Defeated;
            }
        }
    }
}

/// Mirror persisted save switch state onto ECS switch components.
///
/// Encounter arming now reads `EncounterSwitchIndex`, which is rebuilt from
/// these ECS components.
pub fn sync_ecs_switches_from_save(
    save: Res<ambition_persistence::save::SandboxSave>,
    mut switches: Query<(&FeatureId, &mut SwitchOn), With<SwitchFeature>>,
) {
    for (id, mut switch_on) in &mut switches {
        switch_on.0 = save.data().switch(id.as_str());
    }
}

#[cfg(test)]
mod actor_liveness_tests;
#[cfg(test)]
mod switch_save_tests;
