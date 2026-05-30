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
            &mut ActorRuntime,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let data = save.data();
    for (
        mut actor,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
    ) in &mut actors
    {
        match &mut *actor {
            ActorRuntime::Peaceful(npc) => {
                if data.flag(&npc.flag_id()) {
                    let mut hostile = ActorRuntime::hostile_from_npc(npc);
                    if data.flag(&format!("enemy_{}_dead", hostile.id))
                        || data.flag(&format!(
                            "enemy_{}{}",
                            hostile.id,
                            crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
                        ))
                    {
                        hostile.alive = false;
                        hostile.health.current = 0;
                    }
                    *actor = ActorRuntime::Hostile(hostile);
                }
            }
            ActorRuntime::Hostile(enemy) => {
                // Respect both `_dead` (Never policy) and
                // `_dead_until_rest` (OnRest policy) flags so an
                // enemy killed in a previous session/room visit
                // doesn't spring back to life when the room loads.
                // OnRoomReenter enemies never write a flag in the
                // first place, so they spawn alive by default.
                if !enemy.id.starts_with("encounter:")
                    && enemy.archetype != EnemyArchetype::InfiniteSandbag
                    && enemy.archetype != EnemyArchetype::FiniteSandbag
                    && (data.flag(&format!("enemy_{}_dead", enemy.id))
                        || data.flag(&format!(
                            "enemy_{}{}",
                            enemy.id,
                            crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
                        )))
                {
                    enemy.alive = false;
                    enemy.health.current = 0;
                }
            }
        }
        sync_actor_components_from_runtime(
            &actor,
            &mut identity,
            &mut disposition,
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
    mut bosses: Query<&mut BossFeature, With<FeatureSimEntity>>,
) {
    let data = save.data();
    for mut feature in &mut bosses {
        let boss = &mut feature.boss;
        // Use the canonical behavior id (resolved at spawn from the
        // brain's `PhaseScript:` payload) so an LDtk BossSpawn with
        // flavor name "System Boss" + brain
        // `PhaseScript:clockwork_warden` still hits the
        // `clockwork_warden` save slot. `boss.id` (runtime entity
        // id) also wins as a legacy fallback.
        let encounter_id = boss.behavior.id.clone();
        if matches!(
            data.boss(&encounter_id),
            crate::save::PersistedEncounterState::Cleared
        ) || matches!(
            data.boss(&boss.id),
            crate::save::PersistedEncounterState::Cleared
        ) {
            boss.alive = false;
            boss.health.current = 0;
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
