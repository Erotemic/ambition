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
    room_set: ambition::platformer::lifecycle::SessionWorldRef<RoomSet>,
    save: Res<ambition_persistence::save::SandboxSave>,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<ambition_actors::features::CharacterRoster>,
    mut released: MessageReader<ambition_actors::boss_encounter::PayloadReleased>,
    existing: Query<&FeatureId, With<SmirkingBehemothVictoryNpc>>,
    bosses: Query<(Entity, &FeatureId, &CenteredAabb, BossClusterRef), With<FeatureSimEntity>>,
) {
    // Drain the release signal every frame (host = the dying behemoth). R5: the
    // swallowed victory NPC is freed via the generic `ReleaseOnDeath` capability
    // the instant the behemoth dies; the save-cleared poll below re-spawns it on
    // a later room re-entry. Both paths are guarded by "NPC not already present".
    let released_hosts: Vec<Entity> = released.read().map(|m| m.host).collect();
    if room_set.active_spec().id != CUT_ROPE_ROOM_ID {
        return;
    }
    if existing
        .iter()
        .any(|id| id.as_str() == CUT_ROPE_VICTORY_NPC_ID)
    {
        return;
    }
    let Some((boss_entity, _boss_id, boss_aabb, boss_feature)) =
        bosses.iter().find(|(_, id, _, feature)| {
            id.as_str() == CUT_ROPE_BOSS_ID
                || is_cut_rope_boss(feature.as_boss_ref().config.behavior.id.as_str())
        })
    else {
        return;
    };
    let boss = boss_feature.as_boss_ref();
    // R3/R4: the boss death is resolved entity-side; R4 keys the persisted
    // "cleared" record by PLACEMENT (`config.id`). Spawn the victory NPC when
    // EITHER the behemoth just released its payload this frame (fresh kill) OR
    // the placement reads cleared in the save (room re-entry).
    let boss_persisted_cleared = matches!(
        save.data().boss(&boss.config.id),
        ambition_persistence::save_data::PersistedEncounterState::Cleared
    );
    let released_now = released_hosts.contains(&boss_entity);
    if !boss_persisted_cleared && !released_now {
        return;
    }
    let boss_bottom_y = boss_aabb.center.y + boss_aabb.half_size.y;
    let spawn_pos = ae::Vec2::new(boss.kin.pos.x, boss_bottom_y - CUT_ROPE_VICTORY_NPC_H * 0.5);
    spawn_victory_npc_entity(
        &mut commands,
        &character_catalog,
        &character_roster,
        spawn_pos,
    );
}

fn victory_npc_size() -> ae::Vec2 {
    ae::Vec2::new(CUT_ROPE_VICTORY_NPC_W, CUT_ROPE_VICTORY_NPC_H)
}

fn spawn_victory_npc_entity(
    commands: &mut Commands,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    character_roster: &ambition_actors::features::CharacterRoster,
    pos: ae::Vec2,
) -> Entity {
    let size = victory_npc_size();
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let interactable = ambition_interaction::Interactable {
        id: CUT_ROPE_VICTORY_NPC_ID.to_string(),
        prompt: "Talk".to_string(),
        aabb,
        kind: ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: Some(CUT_ROPE_VICTORY_NPC_DIALOGUE_ID.to_string()),
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
        requires_facing: false,
        enabled: true,
    };
    // Peaceful actors are the SAME unified cluster as enemies now — build the
    // victory NPC through the shared peaceful seed.
    let (mut seed, _render) = ambition_actors::features::ActorClusterSeed::new_peaceful_npc_in(
        character_catalog,
        character_roster,
        CUT_ROPE_VICTORY_NPC_ID,
        CUT_ROPE_VICTORY_NPC_NAME,
        aabb,
        &interactable,
        &[],
    );
    seed.kin.facing = -1.0;
    let combat_kit = ambition_actors::features::CombatKit::default();
    let facing = seed.kin.facing;
    // Dialogue is a SHARED actor capability — carried on `ActorInteraction` so the
    // interact / proximity systems (which key off the component, not an NPC type
    // tag) still offer "Talk" on this runtime-spawned victory NPC.
    let interaction = ambition_actors::features::ActorInteraction {
        interactable: interactable.clone(),
        talk_radius: ambition_actors::features::NPC_TALK_RADIUS,
    };
    let (identity, disposition, combat, intent, cooldowns) =
        ambition_actors::features::actor_component_snapshot(
            &seed,
            ambition_actors::features::ActorDisposition::Peaceful,
        );
    let cluster_bundle = seed.into_components();
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
                faction: ambition_actors::features::ActorFaction::Npc,
                target: ambition_actors::features::ActorTarget::default(),
                pose: ActorPose::from_parts(aabb.center(), aabb.half_size(), facing),
                motion_model: ambition_actors::features::MotionModel::default(),
                combat_kit,
                aggression: ambition_actors::features::ActorAggression::passive(),
                combat,
                intent,
                cooldowns,
                damageable_volumes: DamageableVolumes::default(),
                pogo_policy: PogoPolicy::FromDamageable,
                pogo_target_volumes: PogoTargetVolumes::default(),
                anim: Default::default(),
            },
            cluster_bundle,
            ambition_characters::brain::Brain::stand_still(),
            ambition_characters::brain::ActionSet::peaceful(),
            ActorControl::default(),
            interaction,
        ))
        .id()
}
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmirkingBehemothVictoryNpc;
