//! Damage event application for ECS-owned feature entities.
//!
//! Drains [`DamageEvent`] and [`PogoBounceEvent`] messages and applies them to
//! actors (peaceful + hostile), bosses, and breakables — including the side
//! effects (banners, VFX, SFX, debris, gameplay effects, hit-stop) those hits
//! produce. Read-only `ecs_damage_event_hits_*` predicates live here too so the
//! attack/projectile systems can pre-check whether a queued damage event will
//! actually land before kicking off cues.

use ambition_engine::AabbExt;
use bevy::prelude::{Commands, Entity, MessageReader, MessageWriter, Query, Res, ResMut, With};

use super::super::{
    util::{approximately_same_aabb, midpoint},
    NPC_HOSTILE_STRIKE_THRESHOLD,
};
use super::{
    ae, sync_actor_components_from_runtime, ActorCombatState, ActorCooldowns, ActorDisposition,
    ActorHealth, ActorIdentity, ActorIntent, ActorRuntime, BossFeature, BreakableFeature,
    DamageEvent, DamageSource, EnemyArchetype, FeatureAabb, FeatureId, FeatureName,
    FeatureSimEntity, GameplayBanner, GameplayEffect, PogoBounceEvent, RespawnTimer,
};
use crate::audio::SfxMessage;
use crate::fx::{ParticleKind, VfxMessage};
use crate::physics::{DebrisBurstMessage, PhysicsDebrisCue};

/// Apply typed slash/projectile/pogo damage messages to ECS feature targets.
pub fn apply_feature_damage_events(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut pogo_bounces: MessageReader<PogoBounceEvent>,
    mut banner: ResMut<GameplayBanner>,
    combat_banter: Option<Res<crate::banter::CombatBanterRegistry>>,
    mut breakables: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &FeatureAabb,
            &mut BreakableFeature,
        ),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            &FeatureId,
            &FeatureAabb,
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
    mut bosses: Query<(&FeatureId, &FeatureAabb, &mut BossFeature), With<FeatureSimEntity>>,
    mut player_combat_q: Query<
        &mut crate::player::PlayerCombatState,
        With<crate::player::PlayerEntity>,
    >,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    for event in damage_events.read().cloned() {
        let mut actor_hit_this_event = false;
        for (
            id,
            aabb,
            mut actor,
            mut identity,
            mut disposition,
            mut health,
            mut combat,
            mut intent,
            mut cooldowns,
        ) in &mut actors
        {
            let key = match *disposition {
                ActorDisposition::Peaceful => format!("npc:{}", id.as_str()),
                ActorDisposition::Hostile => format!("enemy:{}", id.as_str()),
            };
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            if !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            match &mut *actor {
                ActorRuntime::Peaceful(npc) => {
                    npc.hit_flash = 0.18;
                    npc.strikes = npc.strikes.saturating_add(1);
                    let impact = midpoint(event.volume.center(), npc.pos);
                    vfx.write(VfxMessage::Impact { pos: impact });
                    gameplay_effects.write(GameplayEffect::StrikeNpc {
                        npc_id: npc.id.clone(),
                        pos: npc.pos,
                    });
                    actor_hit_this_event = true;
                    if npc.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD {
                        let hostile = ActorRuntime::hostile_from_npc(npc);
                        gameplay_effects.write(GameplayEffect::SetFlag {
                            id: npc.flag_id(),
                            on: true,
                        });
                        vfx.write(VfxMessage::SpeechBubble {
                            pos: npc.bark_anchor(),
                            text: npc.hostile_bark().to_string(),
                        });
                        vfx.write(VfxMessage::Burst {
                            pos: npc.pos,
                            count: 16,
                            speed: 230.0,
                            color: [0.84, 0.95, 1.0, 0.82],
                            kind: ParticleKind::Spark,
                        });
                        banner.show(format!("{} turns hostile", npc.name), 2.6);
                        *actor = ActorRuntime::Hostile(hostile);
                    } else {
                        vfx.write(VfxMessage::SpeechBubble {
                            pos: npc.bark_anchor(),
                            text: npc.hit_bark().to_string(),
                        });
                    }
                }
                ActorRuntime::Hostile(enemy) => {
                    if !enemy.alive {
                        continue;
                    }
                    // Combat banter — fire a speech bubble only on
                    // the first non-overlapping hit (hit_flash near
                    // zero before we re-set it below). The line
                    // rotates per hit so repeated strikes don't loop
                    // the same line. Skipped silently if no registry
                    // is loaded (e.g. headless / sandbox-only build)
                    // or this enemy name has no authored lines.
                    let should_bark = enemy.hit_flash < 0.05;
                    enemy.hit_flash = 0.16;
                    if should_bark {
                        if let Some(reg) = combat_banter.as_deref() {
                            let strikes = enemy.health.max - enemy.health.current;
                            if let Some(line) =
                                reg.pick_hit_bark(&enemy.name, strikes.max(0) as u32)
                            {
                                vfx.write(VfxMessage::SpeechBubble {
                                    pos: enemy.bark_anchor(),
                                    text: line.to_string(),
                                });
                            }
                        }
                    }
                    if let DamageSource::PlayerSlash { knock_x } = &event.source {
                        enemy.vel.x += *knock_x;
                        enemy.vel.y = (enemy.vel.y - 90.0).max(-280.0);
                    }
                    let damage_amount = event.damage.max(1);
                    // Fused pirate-on-shark routes through apply_damage_at
                    // so hits on the top half damage the rider and hits
                    // on the bottom half damage the shark. A rider /
                    // shark death triggers the dismount morph (actor
                    // stays alive in its new form — no death banner).
                    let (killed, archetype_changed) =
                        if enemy.archetype == EnemyArchetype::PirateOnShark {
                            match enemy.apply_damage_at(event.volume, damage_amount) {
                                super::super::enemies::EnemyDamageOutcome::Damaged {
                                    killed,
                                    archetype_changed,
                                } => (killed, archetype_changed),
                                super::super::enemies::EnemyDamageOutcome::NoOp => (false, false),
                            }
                        } else if enemy.archetype == EnemyArchetype::InfiniteSandbag {
                            (false, false)
                        } else {
                            (enemy.health.damage(damage_amount), false)
                        };
                    let impact = midpoint(event.volume.center(), enemy.pos);
                    vfx.write(VfxMessage::Impact { pos: impact });
                    actor_hit_this_event = true;
                    if archetype_changed {
                        // Dismount cue — small banner so the player
                        // sees the morph happened. Avoid death banner.
                        banner.show(format!("{} dismounted", enemy.name), 1.8);
                    }
                    if killed {
                        enemy.alive = false;
                        if enemy.archetype == EnemyArchetype::FiniteSandbag {
                            enemy.respawn_timer = 0.85;
                            banner.show(format!("{} dropped; respawning", enemy.name), 2.6);
                        } else {
                            banner.show(format!("defeated {}", enemy.name), 2.6);
                            if !enemy.id.starts_with("encounter:")
                                && enemy.archetype != EnemyArchetype::InfiniteSandbag
                                && enemy.archetype != EnemyArchetype::FiniteSandbag
                            {
                                gameplay_effects.write(GameplayEffect::SetFlag {
                                    id: format!("enemy_{}_dead", enemy.id),
                                    on: true,
                                });
                            }
                        }
                        vfx.write(VfxMessage::Burst {
                            pos: enemy.pos,
                            count: 16,
                            speed: 230.0,
                            color: [0.84, 0.95, 1.0, 0.82],
                            kind: ParticleKind::Spark,
                        });
                        debris.write(DebrisBurstMessage {
                            pos: enemy.pos,
                            cue: PhysicsDebrisCue::EnemyRagdoll,
                        });
                        sfx.write(SfxMessage::Death { pos: enemy.pos });
                    }
                }
            }
            sync_actor_components_from_runtime(
                &*actor,
                &mut *identity,
                &mut *disposition,
                &mut *health,
                &mut *combat,
                &mut *intent,
                &mut *cooldowns,
            );
        }
        let mut boss_hit_this_event = false;
        for (id, aabb, mut feature) in &mut bosses {
            let key = format!("boss:{}", id.as_str());
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            let boss = &mut feature.boss;
            if !boss.alive || !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            // Speech bubble bark when player lands a hit, debounced by hit_flash.
            let should_bark = boss.hit_flash < 0.05;
            boss.hit_flash = 0.18;
            if should_bark {
                if let Some(reg) = combat_banter.as_deref() {
                    let strikes = boss.health.max - boss.health.current;
                    if let Some(line) = reg.pick_hit_bark(&boss.name, strikes.max(0) as u32) {
                        vfx.write(VfxMessage::SpeechBubble {
                            pos: boss.bark_anchor(),
                            text: line.to_string(),
                        });
                    }
                }
            }
            let amount = event.damage.max(1);
            let killed = boss.health.damage(amount);
            let impact = midpoint(event.volume.center(), boss.pos);
            vfx.write(VfxMessage::Impact { pos: impact });
            gameplay_effects.write(GameplayEffect::DamageBoss {
                boss_id: boss.id.clone(),
                amount,
            });
            boss_hit_this_event = true;
            if killed {
                boss.alive = false;
                banner.show(format!("defeated boss {}", boss.name), 2.6);
                vfx.write(VfxMessage::Burst {
                    pos: boss.pos,
                    count: 16,
                    speed: 230.0,
                    color: [0.84, 0.95, 1.0, 0.82],
                    kind: ParticleKind::Spark,
                });
                debris.write(DebrisBurstMessage {
                    pos: boss.pos,
                    cue: PhysicsDebrisCue::BossRagdoll,
                });
                sfx.write(SfxMessage::Death { pos: boss.pos });
            }
        }

        if actor_hit_this_event || boss_hit_this_event {
            if let Ok(mut combat) = player_combat_q.single_mut() {
                combat.hitstop_timer = combat.hitstop_timer.max(0.06);
                combat.flash_timer = combat.flash_timer.max(0.10);
            }
            sfx.write(SfxMessage::Hit {
                pos: event.volume.center(),
            });
        }

        for (entity, id, name, aabb, mut feature) in &mut breakables {
            let key = format!("breakable:{}", id.as_str());
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            if feature.broken() || !feature.breakable.trigger.allows_hit() {
                continue;
            }
            if feature.breakable.pogo_refresh {
                continue;
            }
            if !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            let broke = feature.breakable.apply_damage(event.damage.max(1));
            vfx.write(VfxMessage::Impact {
                pos: midpoint(event.volume.center(), aabb.center),
            });
            if broke {
                begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                banner.show(format!("broke {}", name.0.as_str()), 2.6);
                emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
            }
        }
    }

    for event in pogo_bounces.read() {
        let orb_aabb = event.orb_aabb;
        let damage = event.damage;
        for (entity, _id, name, aabb, mut feature) in &mut breakables {
            if feature.broken() || !feature.breakable.pogo_refresh {
                continue;
            }
            if !approximately_same_aabb(aabb.aabb(), orb_aabb) {
                continue;
            }
            let broke = feature.breakable.apply_damage(damage.max(1));
            vfx.write(VfxMessage::Impact { pos: aabb.center });
            if broke {
                begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                banner.show(format!("shattered {}", name.0.as_str()), 2.6);
                emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
            }
        }
    }
}

/// Read-only hit test used by systems that need immediate projectile / attack
/// feedback while damage application is still drained through
/// typed Bevy messages.
pub fn ecs_damage_event_hits_breakable(
    event: &DamageEvent,
    breakables: &Query<(&FeatureId, &FeatureAabb, &BreakableFeature), With<FeatureSimEntity>>,
) -> bool {
    breakables.iter().any(|(id, aabb, feature)| {
        let key = format!("breakable:{}", id.as_str());
        !event.ignored_targets.iter().any(|ignored| ignored == &key)
            && !feature.broken()
            && feature.breakable.trigger.allows_hit()
            && !feature.breakable.pogo_refresh
            && event.volume.strict_intersects(aabb.aabb())
    })
}

pub fn ecs_damage_event_hits_actor(
    event: &DamageEvent,
    actors: &Query<
        (
            &FeatureId,
            &FeatureAabb,
            &ActorDisposition,
            &ActorCombatState,
        ),
        With<FeatureSimEntity>,
    >,
) -> bool {
    actors.iter().any(|(id, aabb, disposition, combat)| {
        let key = match *disposition {
            ActorDisposition::Peaceful => format!("npc:{}", id.as_str()),
            ActorDisposition::Hostile => format!("enemy:{}", id.as_str()),
        };
        !event.ignored_targets.iter().any(|ignored| ignored == &key)
            && combat.alive
            && event.volume.strict_intersects(aabb.aabb())
    })
}

pub fn ecs_damage_event_hits_boss(
    event: &DamageEvent,
    bosses: &Query<(&FeatureId, &FeatureAabb, &BossFeature), With<FeatureSimEntity>>,
) -> bool {
    bosses.iter().any(|(id, aabb, feature)| {
        let key = format!("boss:{}", id.as_str());
        !event.ignored_targets.iter().any(|ignored| ignored == &key)
            && feature.boss.alive
            && event.volume.strict_intersects(aabb.aabb())
    })
}

/// Schedule a broken breakable for respawn if its policy allows.
///
/// Called from both `apply_feature_damage_events` (typed damage path) and
/// `update_ecs_breakables` (stand-to-break path), so it lives here as a
/// `pub(super)` helper rather than duplicating the policy check.
pub(super) fn begin_ecs_breakable_respawn(
    commands: &mut Commands,
    entity: Entity,
    breakable: &ae::Breakable,
) {
    if let ae::RespawnPolicy::AfterSeconds(seconds) = breakable.respawn {
        commands.entity(entity).insert(RespawnTimer(seconds));
    }
}

/// Common VFX/SFX/debris emission when a breakable is destroyed by any path.
pub(super) fn emit_breakable_destroyed(
    pos: ae::Vec2,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
) {
    vfx.write(VfxMessage::Burst {
        pos,
        count: 16,
        speed: 230.0,
        color: [0.84, 0.95, 1.0, 0.82],
        kind: ParticleKind::Spark,
    });
    debris.write(DebrisBurstMessage {
        pos,
        cue: PhysicsDebrisCue::Breakable,
    });
    sfx.write(SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_CRATE_BREAK,
        pos,
    });
}
