//! Cut-rope boss victory NPC — spawns the Smirking Behemoth's defeated-form
//! NPC the player talks to after winning.
//!
//! Split out of the former 793-line `cut_rope.rs` (2026-06-15).

use super::*;

/// Spawn the post-Smirking-Behemoth NPC after the boss encounter has fully resolved.
///
/// The NPC is runtime-spawned rather than LDtk-authored so the room layout stays stable and the
/// entity can feel like it crawled out of the dead boss body. It is still a normal peaceful NPC
/// actor with a Yarn dialogue id, so interaction, sprite fallback, pogo/damage volumes, and reset
/// behavior use the existing ECS actor path.
pub fn spawn_cut_rope_victory_npc(
    mut commands: Commands,
    room_set: Res<RoomSet>,
    registry: Res<BossEncounterRegistry>,
    save: Res<ambition_gameplay_core::persistence::save::SandboxSave>,
    existing: Query<&FeatureId, With<SmirkingBehemothVictoryNpc>>,
    bosses: Query<(&FeatureId, &CenteredAabb, BossClusterRef), With<FeatureSimEntity>>,
) {
    if room_set.active_spec().id != CUT_ROPE_ROOM_ID {
        return;
    }
    if existing
        .iter()
        .any(|id| id.as_str() == CUT_ROPE_VICTORY_NPC_ID)
    {
        return;
    }
    let Some((_boss_id, boss_aabb, boss_feature)) = bosses.iter().find(|(id, _, feature)| {
        id.as_str() == CUT_ROPE_BOSS_ID
            || is_cut_rope_boss(feature.as_boss_ref().config.behavior.id.as_str())
    }) else {
        return;
    };
    let boss = boss_feature.as_boss_ref();
    let encounter_death_complete =
        registry
            .encounters
            .get(CUT_ROPE_BOSS_ID)
            .is_some_and(|encounter| {
                matches!(
                    encounter.phase,
                    ambition_gameplay_core::boss_encounter::BossEncounterPhase::Death
                ) && encounter.death_complete()
            });
    let boss_persisted_cleared = {
        let data = save.data();
        matches!(
            data.boss(CUT_ROPE_BOSS_ID),
            ambition_gameplay_core::persistence::save_data::PersistedEncounterState::Cleared
        ) || matches!(
            data.boss(&boss.config.behavior.id),
            ambition_gameplay_core::persistence::save_data::PersistedEncounterState::Cleared
        ) || matches!(
            data.boss(&boss.config.id),
            ambition_gameplay_core::persistence::save_data::PersistedEncounterState::Cleared
        )
    };
    if !encounter_death_complete && !boss_persisted_cleared {
        return;
    }
    let boss_bottom_y = boss_aabb.center.y + boss_aabb.half_size.y;
    let spawn_pos = ae::Vec2::new(boss.kin.pos.x, boss_bottom_y - CUT_ROPE_VICTORY_NPC_H * 0.5);
    spawn_victory_npc_entity(&mut commands, spawn_pos);
}

fn victory_npc_size() -> ae::Vec2 {
    ae::Vec2::new(CUT_ROPE_VICTORY_NPC_W, CUT_ROPE_VICTORY_NPC_H)
}

fn spawn_victory_npc_entity(commands: &mut Commands, pos: ae::Vec2) -> Entity {
    let size = victory_npc_size();
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let interactable = ambition_gameplay_core::interaction::Interactable {
        id: CUT_ROPE_VICTORY_NPC_ID.to_string(),
        prompt: "Talk".to_string(),
        aabb,
        kind: ambition_gameplay_core::interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: Some(CUT_ROPE_VICTORY_NPC_DIALOGUE_ID.to_string()),
            patrol_radius: 0.0,
            patrol_path_id: None,
        },
        requires_facing: false,
        enabled: true,
    };
    let mut npc = ambition_gameplay_core::features::NpcClusterScratch::new_with_paths(
        CUT_ROPE_VICTORY_NPC_ID,
        CUT_ROPE_VICTORY_NPC_NAME,
        ae::Aabb::new(pos, size * 0.5),
        interactable,
        &[],
    );
    npc.kin.facing = -1.0;
    let brain = npc.as_mut().build_brain();
    let combat_kit = ambition_gameplay_core::features::CombatKit::default();
    let cluster_bundle = npc.into_components();
    let facing = cluster_bundle.0.facing;
    let (identity, disposition, health, combat, intent, cooldowns) =
        ambition_gameplay_core::features::npc_component_snapshot(&cluster_bundle.3, &cluster_bundle.4);
    commands
        .spawn((
            Name::new("Post-boss NPC: Smirking Behemoth victory"),
            SmirkingBehemothVictoryNpc,
            PostBossNpc,
            EnemyActorBundle {
                base: FeatureBaseBundle::new(
                    CUT_ROPE_VICTORY_NPC_ID,
                    CUT_ROPE_VICTORY_NPC_NAME,
                    CenteredAabb::from_aabb(aabb),
                ),
                identity,
                disposition,
                faction: ambition_gameplay_core::features::ActorFaction::Npc,
                target: ambition_gameplay_core::features::ActorTarget::default(),
                pose: ActorPose::from_parts(aabb.center(), aabb.half_size(), facing),
                combat_kit,
                aggression: ambition_gameplay_core::features::ActorAggression::passive(),
                health,
                combat,
                intent,
                cooldowns,
                damageable_volumes: DamageableVolumes::default(),
                pogo_policy: PogoPolicy::FromDamageable,
                pogo_target_volumes: PogoTargetVolumes::default(),
            },
            ActorRuntime::Npc,
            cluster_bundle,
            brain,
            ambition_gameplay_core::brain::ActionSet::peaceful(),
            ActorControl::default(),
        ))
        .id()
}
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmirkingBehemothVictoryNpc;
