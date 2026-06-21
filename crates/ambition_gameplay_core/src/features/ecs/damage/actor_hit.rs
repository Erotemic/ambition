//! Applying a hit to an actor (enemy/NPC): damage, knockback, aggression, death.

use crate::engine_core::AabbExt;
use bevy::prelude::Entity;

use super::super::super::{util::midpoint, NPC_HOSTILE_STRIKE_THRESHOLD};
use super::super::damage_drops::{
    drop_currency_coin, drop_health_pickup, id_drops_health, spawn_death_explosion,
    spawn_split_offspring,
};
use super::super::{ae, ActorRuntime, GameplayBanner, HitEvent, HitSource, SetFlagRequested};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
use crate::audio::SfxMessage;
use crate::features::ActorStimulus;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

use super::*;

/// Peel-off speed (px/s) applied along the surface normal when a struck
/// surface-walker loses its cling. Enough to visibly pop off a wall/ceiling
/// before gravity takes over; tuned well under the patrol speed's order so it
/// reads as a knock, not a launch.
const CLING_DETACH_POP_SPEED: f32 = 180.0;

/// Apply one landed attacker-side hit to a single actor (peaceful NPC
/// or hostile enemy) and emit its per-actor feedback (impact VFX,
/// retaliation stimulus, banter, kill banner / debris / death SFX,
/// hostility flags).
///
/// Returns `true` when the actor took the hit, so the caller drives the
/// shared landed-hit feedback (hitstop + Hit SFX) and re-syncs the
/// ECS read-model components. A dead enemy returns `false` (no-op),
/// matching the previous `continue`-before-sync behavior.
///
/// Extracted from `apply_feature_hit_events` per ecs-cleanup-plan.md #4
/// so the per-target families are testable helpers instead of one god
/// loop body; the scheduled system is unchanged.
/// NPC-side hit target: the mutated status, read-only config, and the
/// actor AABB (position source — the shared kinematics is borrowed
/// mutably by the enemy cluster query, so NPC positions come from
/// `CenteredAabb` instead).
pub(crate) struct NpcHitTarget<'a> {
    pub(crate) status: &'a mut super::super::npc_clusters::NpcStatus,
    pub(crate) config: &'a super::super::npc_clusters::NpcConfig,
    pub(crate) aabb: ae::Aabb,
}

pub(crate) fn apply_actor_hit(
    event: &HitEvent,
    actor_entity: Entity,
    actor: &mut ActorRuntime,
    enemy: Option<&mut super::super::enemy_clusters::EnemyMut<'_>>,
    npc: Option<NpcHitTarget<'_>>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&crate::features::banter::CombatBanterRegistry>,
    writers: &mut FeatureHitWriters<'_, '_>,
) -> bool {
    match actor {
        ActorRuntime::Npc => {
            let Some(npc) = npc else {
                return false;
            };
            let pos = npc.aabb.center();
            let bark_anchor = super::super::super::npcs::npc_bark_anchor_from_aabb(npc.aabb);
            npc.status.hit_flash = 0.18;
            npc.status.strikes = npc.status.strikes.saturating_add(1);
            let impact = midpoint(event.volume.center(), pos);
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            // Retaliation/hostility is driven by ActorStimulus
            // below; the old GameplayEffect::StrikeNpc trace
            // hook was a no-op and has been removed.
            writers.actor_stimuli.write(ActorStimulus::DamagedBy {
                actor: actor_entity,
                source: event.attacker,
                damage: event.damage,
            });
            if npc.status.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD {
                writers.set_flag.write(SetFlagRequested {
                    id: super::super::super::npcs::npc_flag_id(npc.config),
                    on: true,
                });
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: bark_anchor,
                    text: super::super::super::npcs::npc_hostile_bark_line(npc.config).to_string(),
                });
                writers.vfx.write(VfxMessage::Burst {
                    pos,
                    count: 16,
                    speed: 230.0,
                    color: [0.84, 0.95, 1.0, 0.82],
                    kind: ParticleKind::Spark,
                });
                banner.show(format!("{} turns hostile", npc.config.name), 2.6);
            } else {
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: bark_anchor,
                    text: super::super::super::npcs::npc_hit_bark_line(npc.config, npc.status)
                        .to_string(),
                });
            }
            true
        }
        ActorRuntime::Enemy => {
            let Some(em) = enemy else {
                return false;
            };
            if !em.status.alive {
                return false;
            }
            // Combat banter — fire a speech bubble only on
            // the first non-overlapping hit (hit_flash near
            // zero before we re-set it below).
            let should_bark = em.status.hit_flash < 0.05;
            em.status.hit_flash = 0.16;
            if should_bark {
                if let Some(reg) = combat_banter {
                    let strikes = em.status.health.max - em.status.health.current;
                    if let Some(line) = reg.pick_hit_bark(&em.config.name, strikes.max(0) as u32) {
                        writers.vfx.write(VfxMessage::SpeechBubble {
                            pos: em.bark_anchor(),
                            text: line.to_string(),
                        });
                    }
                }
            }
            if let HitSource::PlayerSlash { knock_x } = &event.source {
                em.kin.vel.x += *knock_x;
                em.kin.vel.y = (em.kin.vel.y - 90.0).max(-280.0);
            }
            let damage_amount = event.damage.max(1);
            let caps = em.caps.clone();
            let killed = if caps.never_dies {
                false
            } else {
                em.status.health.damage(damage_amount)
            };
            let impact = midpoint(event.volume.center(), em.kin.pos);
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            // Cling-break: a struck surface-walker (puppy-slug) is knocked off
            // its surface — it peels away along the surface normal and falls with
            // gravity until it lands and re-attaches (handled by the surface-walk
            // integration's `fall_until_landed`). Archetypes authored with
            // `cling_breaks_on_hit: false` hold on when hit. Keep the last surface
            // normal while airborne; `fall_until_landed` reorients it relative to
            // the active acceleration frame on the next support contact.
            if !killed && em.config.tuning.surface_walker && em.config.tuning.cling_breaks_on_hit {
                let peel = em.surface.surface_normal * CLING_DETACH_POP_SPEED;
                em.surface.on_ground = false;
                em.kin.vel += peel;
            }
            if killed {
                em.status.alive = false;
                if let Some(respawn_s) = caps.respawn_in_place_seconds {
                    em.status.respawn_timer = respawn_s;
                    banner.show(format!("{} dropped; respawning", em.config.name), 2.6);
                } else {
                    banner.show(format!("defeated {}", em.config.name), 2.6);
                    // Earn-side: a defeated enemy drops a collectible coin so the
                    // player can fund the merchant / ability shop from combat, and
                    // ~1 in 4 enemy kinds also drops a heart (combat sustain).
                    drop_currency_coin(
                        &mut writers.commands,
                        &em.config.id,
                        em.kin.pos,
                        ENEMY_BOUNTY,
                    );
                    // Volatile archetypes detonate on death — a sizable
                    // Enemy-faction blast at the corpse, so a point-blank kill is
                    // punished (the read: kill it at range / sidestep the body).
                    if caps.explodes_on_death {
                        spawn_death_explosion(&mut writers.commands, actor_entity, em.kin.pos);
                        writers.vfx.write(VfxMessage::Explosion {
                            pos: em.kin.pos,
                            kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
                            scale: 0.85,
                        });
                    }
                    // Replicating blobs divide on death into two fast offspring.
                    if caps.divides_on_death {
                        spawn_split_offspring(&mut writers.commands, &em.config.id, em.kin.pos);
                    }
                    if id_drops_health(&em.config.id) {
                        drop_health_pickup(
                            &mut writers.commands,
                            &em.config.id,
                            em.kin.pos + ae::Vec2::new(18.0, 0.0),
                            ENEMY_HEALTH_DROP,
                        );
                    }
                    // Steal the enemy's weapon: a defeated enemy that was wielding
                    // a held item drops it as a `GroundItem` the player can grab +
                    // wield (e.g. a pirate's gun-sword), via the existing pickup path.
                    if let Some(spec) = caps.drops_held_item.clone() {
                        writers.commands.spawn((
                            crate::items::pickup::GroundItem {
                                spec,
                                pos: em.kin.pos + ae::Vec2::new(-14.0, 0.0),
                                vel: ae::Vec2::ZERO,
                                half_extent: ae::Vec2::splat(16.0),
                            },
                            bevy::prelude::Name::new("Dropped weapon"),
                        ));
                    }
                    if !em.config.id.starts_with("encounter:") && !em.config.tuning.is_sandbag {
                        use crate::features::EnemyRespawnPolicy as P;
                        let flag_id = match caps.respawn_policy {
                            P::OnRoomReenter => None,
                            P::OnRest => Some(format!(
                                "enemy_{}{}",
                                em.config.id,
                                crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
                            )),
                            P::Never => Some(format!("enemy_{}_dead", em.config.id)),
                        };
                        if let Some(id) = flag_id {
                            writers.set_flag.write(SetFlagRequested { id, on: true });
                        }
                    }
                }
                writers.vfx.write(VfxMessage::Burst {
                    pos: em.kin.pos,
                    count: 16,
                    speed: 230.0,
                    color: [0.84, 0.95, 1.0, 0.82],
                    kind: ParticleKind::Spark,
                });
                writers.debris.write(DebrisBurstMessage {
                    pos: em.kin.pos,
                    cue: PhysicsDebrisCue::EnemyRagdoll,
                });
                writers.sfx.write(SfxMessage::Death { pos: em.kin.pos });
            }
            true
        }
    }
}
