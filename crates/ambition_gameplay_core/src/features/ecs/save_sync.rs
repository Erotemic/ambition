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
    save: Res<crate::persistence::save::SandboxSave>,
    mut actors: Query<
        (
            Entity,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut BodyHealth,
            &mut ActorCombatState,
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
    for (
        entity,
        mut identity,
        mut disposition,
        mut health,
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
            aggression.mode = AggressionMode::HostileToPlayer;
            let mut em = cq.as_actor_mut();
            super::actors::provoke_actor_in_place(
                &mut commands,
                entity,
                &mut em,
                &mut disposition,
                combat_kit,
                held_item,
                dialogue_id,
                false,
            );
            if dead_on_load {
                em.status.alive = false;
                em.status.health.current = 0;
            }
        } else if interaction.is_none() {
            // Authored enemy: respect both `_dead` (Never policy) and
            // `_dead_until_rest` (OnRest policy) flags so an enemy killed in a
            // previous session/room visit stays dead. OnRoomReenter enemies
            // never write a flag, so they spawn alive.
            let em = cq.as_actor_mut();
            if !em.config.id.starts_with("encounter:")
                && !em.config.tuning.is_sandbag
                && dead_on_load
            {
                em.status.alive = false;
                em.status.health.current = 0;
            }
        }

        let em = cq.as_actor_mut();
        sync_actor_components_from_cluster(
            &em,
            *disposition,
            &mut identity,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

/// Mirror persisted boss-cleared state onto ECS-owned boss actors.
pub fn sync_ecs_bosses_with_save(
    save: Res<crate::persistence::save::SandboxSave>,
    mut bosses: Query<
        (
            super::boss_clusters::BossClusterQueryData,
            Option<&mut BossDeathAnimation>,
            Option<&mut BossPhase>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (mut feature, death_anim, phase) in &mut bosses {
        // R4: "cleared" is keyed to this PLACEMENT, not the archetype. Shared
        // predicate (`boss_is_cleared`) with the per-tick encounter driver so
        // they can't drift.
        if super::boss_clusters::boss_is_cleared(&save, &feature.config) {
            feature.status.alive = false;
            feature.status.health.current = 0;
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
    save: Res<crate::persistence::save::SandboxSave>,
    mut switches: Query<(&FeatureId, &mut SwitchOn), With<SwitchFeature>>,
) {
    for (id, mut switch_on) in &mut switches {
        switch_on.0 = save.data().switch(id.as_str());
    }
}

#[cfg(test)]
mod switch_save_tests {
    //! sync_ecs_switches_from_save authoritatively restores each switch's
    //! on/off from the save flag keyed by its FeatureId — so a save load
    //! (or a reset that rewrote flags) re-derives switch visuals/state.
    use super::*;
    use crate::encounter::SwitchActivation;
    use crate::persistence::save::SandboxSave;
    use bevy::prelude::{App, Update};

    #[test]
    fn switches_restore_their_on_state_from_the_save() {
        let mut app = App::new();
        let mut save = SandboxSave::default();
        save.data_mut().set_switch("on_switch", true);
        app.insert_resource(save);
        app.add_systems(Update, sync_ecs_switches_from_save);

        // Start each switch at the OPPOSITE of its saved value to prove the
        // restore is authoritative, not an OR.
        let on = app
            .world_mut()
            .spawn((
                FeatureId::new("on_switch"),
                SwitchOn(false),
                SwitchFeature::new(SwitchActivation::default()),
            ))
            .id();
        let off = app
            .world_mut()
            .spawn((
                FeatureId::new("off_switch"), // never set in the save -> false
                SwitchOn(true),
                SwitchFeature::new(SwitchActivation::default()),
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<SwitchOn>(on).unwrap().0,
            "a saved-on switch restores to on"
        );
        assert!(
            !app.world().get::<SwitchOn>(off).unwrap().0,
            "an unsaved switch is authoritatively set off"
        );
    }
}
