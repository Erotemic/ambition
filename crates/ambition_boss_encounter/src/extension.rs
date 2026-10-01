//! The boss domain's extension trigger adapter.
//!
//! For each special key that an admitted module entry is bound to, this
//! adapter queues one [`BossSpecialCast`] invocation for each boss, each tick,
//! with the boss's settled facts and whether it pressed the key. See the port card in
//! `ambition_boss_special_port`.

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::{ActorActionMessage, BossAttackProfile, BossAttackState};
use ambition_combat::components::ActorTarget;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core::{AabbExt, BodyKinematics};
use bevy::prelude::*;

use crate::BossClusterRef;

/// Install the trigger port in `technique_execution`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<BossSpecialCast, _>(
        TECHNIQUE_EXECUTION,
        "ambition_boss_encounter",
        queue_boss_special_casts,
    );
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
        for key in &keys {
            let press = presses
                .iter()
                .find(|(actor, pressed, _)| *actor == entity && pressed == key);
            invocations.trigger::<BossSpecialCast>(
                &TECHNIQUE_EXECUTION,
                key.to_string(),
                entity,
                press.and_then(|p| p.2),
                BossCaster {
                    pressed: press.is_some(),
                    telegraphing: telegraphed == Some(*key),
                    alive: health.alive(),
                    position: [pos.x, pos.y],
                    facing: boss.kin.facing.signum(),
                    projectile_offset: [offset.x, offset.y],
                    target,
                },
            );
        }
    }
}
