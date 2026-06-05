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
    save: Res<crate::persistence::save::SandboxSave>,
    mut actors: Query<
        (
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            super::enemy_clusters::EnemyClusterQueryData,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let data = save.data();
    for (
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        mut cq,
    ) in &mut actors
    {
        // Respect both `_dead` (Never policy) and `_dead_until_rest`
        // (OnRest policy) flags so an enemy killed in a previous
        // session/room visit doesn't spring back to life when the room
        // loads. OnRoomReenter enemies never write a flag in the first
        // place, so they spawn alive by default.
        let em = cq.as_enemy_mut();
        if !em.config.id.starts_with("encounter:")
            && em.config.archetype != EnemyArchetype::InfiniteSandbag
            && em.config.archetype != EnemyArchetype::FiniteSandbag
            && (data.flag(&format!("enemy_{}_dead", em.config.id))
                || data.flag(&format!(
                    "enemy_{}{}",
                    em.config.id,
                    crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
                )))
        {
            em.status.alive = false;
            em.status.health.current = 0;
        }
        sync_actor_components_from_enemy(
            &em,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

/// Mirror save-derived provoke state onto peaceful NPC actors: an NPC
/// whose hostility flag is persisted loads as a hostile enemy. Split
/// from [`sync_ecs_actors_with_save`] because the NPC cluster query
/// borrows the shared kinematics/surface the enemy query holds mutably.
pub fn sync_ecs_npc_actors_with_save(
    mut commands: Commands,
    save: Res<crate::persistence::save::SandboxSave>,
    mut actors: Query<
        (
            Entity,
            &mut ActorRuntime,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            &mut ActorAggression,
            &CombatKit,
            Option<&HeldItem>,
            super::npc_clusters::NpcClusterQueryData,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let data = save.data();
    for (
        entity,
        mut actor,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        mut aggression,
        combat_kit,
        held_item,
        npc,
    ) in &mut actors
    {
        if !data.flag(&super::super::npcs::npc_flag_id(&npc.config)) {
            let (i, d, h, c, it, cd) =
                super::actors::npc_component_snapshot(&npc.config, &npc.status);
            *identity = i;
            *disposition = d;
            *health = h;
            *combat = c;
            *intent = it;
            *cooldowns = cd;
            continue;
        }
        let mut hostile = enemy_runtime_for_npc_combat(&npc.config, &npc.kin, &npc.surface);
        if data.flag(&format!("enemy_{}_dead", hostile.config.id))
            || data.flag(&format!(
                "enemy_{}{}",
                hostile.config.id,
                crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
            ))
        {
            hostile.status.alive = false;
            hostile.status.health.current = 0;
        }
        aggression.mode = AggressionMode::HostileToPlayer;
        let (new_brain, new_action_set) =
            super::brain_builders::aggressive_brain_and_action_set_for_enemy(
                &hostile.config,
                combat_kit,
                held_item,
            );
        make_entity_enemy(
            &mut commands,
            entity,
            &mut actor,
            &hostile,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
        commands.entity(entity).insert((new_brain, new_action_set));
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
    let data = save.data();
    for (mut feature, death_anim, phase) in &mut bosses {
        // Use the canonical behavior id (resolved at spawn from the
        // brain's `PhaseScript:` payload) so an LDtk BossSpawn with
        // flavor name "System Boss" + brain
        // `PhaseScript:clockwork_warden` still hits the
        // `clockwork_warden` save slot. `boss.id` (runtime entity
        // id) also wins as a legacy fallback.
        let encounter_id = feature.config.behavior.id.clone();
        if matches!(
            data.boss(&encounter_id),
            crate::save::PersistedEncounterState::Cleared
        ) || matches!(
            data.boss(&feature.config.id),
            crate::save::PersistedEncounterState::Cleared
        ) {
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
