//! Player → ECS feature interaction (peaceful NPC dialogue, switches).
//!
//! Chests stay in `open_ecs_chests` because they have their own
//! reward/persistence path; this system covers the conversational
//! and switch-activation interactions that share the
//! `PlayerInteractionState` buffered-press contract.

use super::*;

/// Handle interactions with ECS switches and peaceful NPCs. Chests stay in
/// `open_ecs_chests` because they have their own reward/persistence path.
pub fn interact_ecs_actors_and_switches(
    mut dialogue: ResMut<crate::dialog::DialogState>,
    mut next_mode: ResMut<NextState<crate::GameMode>>,
    mut banner: ResMut<GameplayBanner>,
    mut player: Query<
        (
            &crate::player::PlayerKinematics,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerAnimState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    actors: Query<
        (
            &FeatureAabb,
            &ActorRuntime,
            Option<&super::npc_clusters::NpcConfig>,
            Option<&super::npc_clusters::NpcStatus>,
        ),
        With<FeatureSimEntity>,
    >,
    mut switches: Query<
        (
            &FeatureId,
            &FeatureName,
            &FeatureAabb,
            &SwitchFeature,
            &mut SwitchOn,
        ),
        With<FeatureSimEntity>,
    >,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    // Iterate every player's buffered-interact state so each player
    // can talk to an NPC or activate a switch independently. The NPC
    // dialogue gate is global (one DialogState resource), so when one
    // player engages dialogue the loop short-circuits — a future
    // per-player dialogue surface (OVERNIGHT-TODO #17) would let
    // simultaneous NPC interactions land per-player. Single-player
    // behavior preserved because the iterator has one entity today.
    // How long the player's `Interact` pose holds after the interaction
    // commits. Short enough that the gesture clears before dialogue UI
    // or the room transition takes camera focus.
    const INTERACT_ANIM_HOLD_SECS: f32 = 0.28;
    for (player_kin, mut interaction, mut anim) in &mut player {
        if !interaction.buffered() {
            continue;
        }
        let player_aabb = player_kin.aabb();
        let mut consumed = false;
        for (aabb, actor, npc_config, npc_status) in &actors {
            if !matches!(actor, ActorRuntime::Npc) {
                continue;
            }
            let (Some(config), Some(status)) = (npc_config, npc_status) else {
                continue;
            };
            if !aabb.aabb().strict_intersects(player_aabb) {
                continue;
            }
            interaction.clear();
            anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
            banner.show(super::super::npcs::npc_message(config, status), 2.6);
            let request = super::super::npcs::npc_dialogue_request(config);
            dialogue.start(&request.dialogue_id, &request.npc_name);
            next_mode.set(crate::GameMode::Dialogue);
            gameplay_effects.write(GameplayEffect::AdvanceQuest(
                crate::quest::QuestAdvanceEvent::NpcTalked(config.id.clone()),
            ));
            gameplay_effects.write(GameplayEffect::SetFlag {
                id: "met_any_hub_npc".into(),
                on: true,
            });
            gameplay_effects.write(GameplayEffect::SetFlag {
                id: format!("npc_{}_talked", request.dialogue_id),
                on: true,
            });
            vfx.write(VfxMessage::Burst {
                pos: aabb.center,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            // Dialogue is a global mode flip; once one player engages
            // it the loop short-circuits — no other player can also
            // start a dialogue this frame. Return entirely (skip the
            // switch loop too) to match the prior single() semantic.
            return;
        }
        for (_id, name, aabb, switch, mut on) in &mut switches {
            if !aabb.aabb().strict_intersects(player_aabb) {
                continue;
            }
            interaction.clear();
            anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
            banner.show(format!("activated {}", name.0.as_str()), 2.6);
            on.0 = true;
            gameplay_effects.write(GameplayEffect::ActivateSwitch {
                activation: switch.activation.clone(),
                pos: aabb.center,
            });
            vfx.write(VfxMessage::Burst {
                pos: aabb.center,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            consumed = true;
            break;
        }
        if consumed {
            // Switch activation is per-target; once this player
            // flipped a switch we don't keep checking actors / other
            // switches for them this tick (matches the prior
            // single-`return`-after-switch semantic).
            continue;
        }
    }
}
