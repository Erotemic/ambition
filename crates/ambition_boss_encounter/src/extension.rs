//! The boss domain's extension adapters.
//!
//! The trigger: for each special key that an admitted module entry is bound
//! to, one [`BossSpecialCast`] invocation for each boss, each tick, with the
//! boss's settled facts and whether it pressed the key.
//!
//! The request: a boss module's [`BossSummonPort`] minion enters the summon
//! executor's one road (`EffectRequest` → `Effect::Summon`), in the boss's
//! encounter. See the port cards in `ambition_boss_special_port`.

use ambition_boss_special_port::{BossCaster, BossSpecialCast, BossSummonPort};
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::{ActorActionMessage, BossAttackProfile, BossAttackState};
use ambition_combat::components::ActorTarget;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core::{AabbExt, BodyKinematics};
use bevy::prelude::*;

use crate::{BossClusterRef, BossConfig};

/// Install the trigger port in `technique_execution`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<BossSpecialCast, _>(
        TECHNIQUE_EXECUTION,
        "ambition_boss_encounter",
        queue_boss_special_casts,
    );
}

/// Install the summon request port in `technique_execution`. The host
/// lowers a phase's request ports in install order; the composition installs
/// this one after the damage box, as the native techniques wrote them.
pub fn install_summons(app: &mut App) {
    app.install_extension_request::<BossSummonPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_boss_encounter",
        lower_boss_summons,
    );
}

fn lower_boss_summons(
    mut outbox: ResMut<ExtensionOutbox>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    bosses: Query<&BossConfig>,
) {
    for submitted in outbox.drain::<BossSummonPort>() {
        // ⛔ SUBMITTED IS NOT APPLIED. Only a boss has an encounter to put a
        // minion in.
        let Ok(boss) = bosses.get(submitted.scope) else {
            warn!(
                "extension entry {} asked for a summon for {:?}, which is not a boss; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let summon = submitted.value;
        let Some(id) = summon.id(&boss.id) else {
            warn!(
                "extension entry {} asked for a summon with the label {:?}; refused",
                submitted.entry, summon.label
            );
            continue;
        };
        effects.write(ambition_vfx::EffectRequest {
            owner: submitted.scope,
            effect: ambition_vfx::Effect::Summon(ambition_vfx::SummonSpec {
                id,
                pos: ambition_platformer2d_core::Vec2::from(summon.position),
                half_size: ambition_platformer2d_core::Vec2::from(summon.half_size),
                character_id: summon.character_id,
                encounter_id: boss.behavior.id.clone(),
                faction: ambition_vfx::HitSide::Enemy,
                ridden_by_summoner: None,
                health: summon.health,
                keeps_contact_damage: summon.keeps_contact_damage,
            }),
        });
    }
}

/// One invocation for each boss and each bound key, every tick, in boss
/// query order. `pressed` says whether the boss pressed the key this tick.
/// A second press of one key in one tick replaces the move use: the last
/// press is the one that counts.
pub fn queue_boss_special_casts(
    admitted: Res<AdmittedExtensions>,
    mut messages: MessageReader<ActorActionMessage>,
    mut invocations: ResMut<ExtensionInvocations>,
    bosses: Query<(
        Entity,
        BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        Option<&ActorTarget>,
        Option<&BossAttackState>,
    )>,
    bodies: Query<&BodyKinematics>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
) {
    let keys: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == BossSpecialCast::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .fold(Vec::new(), |mut keys, key| {
            if !keys.contains(&key) {
                keys.push(key);
            }
            keys
        });
    let mut presses: Vec<(Entity, &str, Option<u32>)> = Vec::new();
    for msg in messages.read() {
        let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        else {
            continue;
        };
        if let Some(press) = presses
            .iter_mut()
            .find(|(actor, pressed, _)| *actor == msg.actor && *pressed == key.as_str())
        {
            press.2 = msg.move_instance;
        } else {
            presses.push((msg.actor, key.as_str(), msg.move_instance));
        }
    }
    if keys.is_empty() {
        return;
    }
    for (entity, boss, health, target, attack) in &bosses {
        let boss = boss.as_boss_ref();
        let pos = boss.kin.pos;
        let offset = boss.config.behavior.projectile_origin_offset;
        let body = boss.aabb();
        let (body_center, body_half) = (body.center(), body.half_size());
        let telegraphed = attack.and_then(|a| match &a.telegraph_profile {
            Some(BossAttackProfile::Special(key)) => Some(key.as_str()),
            _ => None,
        });
        // The tracked body's centre, whoever it is; the tracked point when the
        // target is not a live body.
        let target = target.map(|t| {
            let at = t
                .entity
                .and_then(|e| bodies.get(e).ok())
                .map(|kin| kin.aabb().center())
                .unwrap_or(t.pos);
            [at.x, at.y]
        });
        // The boss's OWN live room (OW1 cut 7n), never a sole room.
        let room_size = rooms.of(entity).map(|g| [g.0.size.x, g.0.size.y]);
        for key in &keys {
            let press = presses
                .iter()
                .find(|(actor, pressed, _)| *actor == entity && pressed == key);
            invocations.trigger::<BossSpecialCast>(
                &TECHNIQUE_EXECUTION,
                key.to_string(),
                entity,
                press.and_then(|p| p.2),
                // The port's IDLE: the key is neither pressed nor telegraphed.
                press.is_none() && telegraphed != Some(*key),
                BossCaster {
                    pressed: press.is_some(),
                    telegraphing: telegraphed == Some(*key),
                    alive: health.alive(),
                    position: [pos.x, pos.y],
                    facing: boss.kin.facing.signum(),
                    projectile_offset: [offset.x, offset.y],
                    body_center: [body_center.x, body_center.y],
                    body_half_size: [body_half.x, body_half.y],
                    target,
                    room_size,
                },
            );
        }
    }
}
