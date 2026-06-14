//! Hit event application for ECS-owned feature entities.
//!
//! Drains [`HitEvent`] messages and applies them to actors (peaceful +
//! hostile), bosses, and breakables — including the side effects
//! (banners, VFX, SFX, debris, gameplay effects, hit-stop) those hits
//! produce. Pogo-orb resolution lives in this same drain loop and
//! branches on `HitSource::PogoBounce` to do orb-AABB matching rather
//! than broadcast volume overlap. Read-only `ecs_hit_event_hits_*`
//! predicates live here too so the attack / projectile systems can
//! pre-check whether a queued hit will actually land before kicking off
//! cues.

use crate::engine_core::AabbExt;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{
    Commands, Entity, MessageReader, MessageWriter, Query, Res, ResMut, With, Without,
};

use super::super::{
    util::{approximately_same_aabb, midpoint},
    NPC_HOSTILE_STRIKE_THRESHOLD,
};
use super::damage_drops::{
    drop_ability_pickup, drop_currency_coin, drop_health_pickup, id_drops_health,
    spawn_death_explosion, spawn_split_offspring,
};
use super::{
    ae, sync_actor_components_from_enemy, ActorCombatState, ActorCooldowns, ActorDisposition,
    ActorHealth, ActorIdentity, ActorIntent, ActorRuntime, BreakableFeature, FeatureAabb,
    FeatureId, FeatureName, FeatureSimEntity, GameplayBanner, HitEvent, HitSource,
    SetFlagRequested,
};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
#[cfg(test)]
use super::damage_drops::EXPLODER_BLAST_DAMAGE;
use super::damage_predicates::target_is_ignored;
#[cfg(test)]
use super::PickupFeature;
use crate::audio::SfxMessage;
use crate::boss_encounter::{record_boss_damage, BossEncounterRegistry};
use crate::cutscene_trigger::CutsceneTriggerQueue;
use crate::encounter::BossEncounterMusicRequest;
use crate::features::ActorStimulus;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_effects::vfx::{ParticleKind, VfxMessage};

#[derive(SystemParam)]
pub struct FeatureHitWriters<'w, 's> {
    pub set_flag: MessageWriter<'w, SetFlagRequested>,
    pub actor_stimuli: MessageWriter<'w, ActorStimulus>,
    pub sfx: MessageWriter<'w, SfxMessage>,
    pub vfx: MessageWriter<'w, VfxMessage>,
    pub debris: MessageWriter<'w, DebrisBurstMessage>,
    /// Refactor 3: spawning loot/respawns on a hit is a one-liner
    /// (`writers.commands.spawn(...)`) instead of hand-threading a separate
    /// `&mut Commands` through every helper that already takes `writers`.
    pub commands: Commands<'w, 's>,
}

/// Coins a defeated standard enemy drops. A flat amount — a *working* earn-side
/// (kill -> coin -> wallet -> merchant), not a balanced economy.
pub const ENEMY_BOUNTY: i32 = 5;

/// Coins a smashed crate/pot drops — smaller than an enemy kill, so combat still
/// pays best, but the environment is worth poking.
pub const BREAKABLE_BOUNTY: i32 = 2;

/// Coins a defeated boss drops, beyond its ability reward — a jackpot for the
/// hardest fights (one boss kill ~= ten standard enemies).
pub const BOSS_BOUNTY: i32 = 50;

/// Health a dropped heart restores when the enemy drops one.
pub const ENEMY_HEALTH_DROP: i32 = 1;

/// Apply typed slash / projectile / pogo hit messages to ECS feature targets.
pub fn apply_feature_hit_events(
    mut hit_events: MessageReader<HitEvent>,
    mut banner: ResMut<GameplayBanner>,
    combat_banter: Option<Res<crate::features::banter::CombatBanterRegistry>>,
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
            Entity,
            &FeatureId,
            &FeatureAabb,
            &mut ActorRuntime,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
            // NPC status/config read & mutated directly; position comes
            // from `FeatureAabb` above so we never borrow the shared
            // kinematics the enemy cluster query already holds mutably.
            Option<&mut super::npc_clusters::NpcStatus>,
            Option<&super::npc_clusters::NpcConfig>,
        ),
        // Bosses are handled by the disjoint `bosses` query; both take
        // `&mut BodyKinematics` (the unified component), so exclude bosses
        // here to keep the two queries provably non-aliasing.
        (
            With<FeatureSimEntity>,
            Without<super::boss_clusters::BossConfig>,
        ),
    >,
    mut bosses: Query<
        (
            &FeatureId,
            &FeatureAabb,
            super::boss_clusters::BossClusterQueryData,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
    // Hitstop / flash on a successful player attack apply to the
    // attacker that landed the hit. Iterates every player and uses
    // `HitEvent::attacker` (now stamped by every player-attacker
    // emit site — slash, pogo, and player projectile). Events with
    // `attacker = None` are environmental / anonymous hits (hazards,
    // enemy strikes) and fall back to primary on the rare path
    // where a feature-target consumer ever needs to apply hitstop
    // for them.
    mut player_combat_q: Query<
        (bevy::prelude::Entity, &mut crate::player::PlayerCombatState),
        bevy::prelude::With<crate::player::PlayerEntity>,
    >,
    primary_q: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<crate::player::PlayerEntity>,
            bevy::prelude::With<crate::player::PrimaryPlayer>,
        ),
    >,
    mut writers: FeatureHitWriters,
    // OVERNIGHT-TODO #8 — boss encounter authoritative state. The
    // engine `BossEncounterState` is now the source of truth for boss
    // HP; the sandbox `BossRuntime.health` is a one-way mirror updated
    // by `update_boss_encounters`. `record_boss_damage` applies the
    // damage delta to engine state and returns the outcome (post-hit
    // HP + killed flag) so this system can drive death VFX / banner
    // on the same tick the kill landed instead of one frame late.
    //
    // The boss-encounter resources are `Option<ResMut<…>>` so unit
    // tests that exercise only the actor / breakable / pickup damage
    // paths (e.g. projectile tests) don't have to install the boss
    // encounter machine. When the resources are absent the boss
    // damage branch falls back to the pre-inversion direct mutation
    // path, so behavior is identical to the legacy code on tests.
    mut boss_registry: Option<ResMut<BossEncounterRegistry>>,
    mut music_request: Option<ResMut<BossEncounterMusicRequest>>,
    mut cutscene_queue: Option<ResMut<CutsceneTriggerQueue>>,
) {
    for event in hit_events.read().cloned() {
        // PogoBounce hits target only the breakable whose AABB
        // approximately matches the orb volume the engine reported.
        // Skip the actor / boss / broadcast-breakable scans entirely;
        // jump straight to the orb-match loop at the bottom.
        if matches!(event.source, HitSource::PogoBounce) {
            for (entity, _id, name, aabb, mut feature) in &mut breakables {
                if feature.broken() || !feature.breakable.pogo_refresh {
                    continue;
                }
                if !approximately_same_aabb(aabb.aabb(), event.volume) {
                    continue;
                }
                let broke = feature.breakable.apply_damage(event.damage.max(1));
                writers.vfx.write(VfxMessage::Impact { pos: aabb.center });
                if broke {
                    begin_ecs_breakable_respawn(&mut writers.commands, entity, &feature.breakable);
                    banner.show(format!("shattered {}", name.0.as_str()), 2.6);
                    emit_breakable_destroyed(
                        aabb.center,
                        &mut writers.sfx,
                        &mut writers.vfx,
                        &mut writers.debris,
                    );
                }
            }
            continue;
        }
        // Victim-side sources (enemy touch, enemy swings, boss body
        // contact, hazards) are consumed by the player-damage path.
        // The feature drain only applies attacker-side player hits
        // here; otherwise an `EnemyBody` event would damage the same
        // enemy that emitted it when the volume overlaps its own
        // AABB.
        if !event.source.is_attacker_side() {
            continue;
        }
        let mut actor_hit_this_event = false;
        for (
            actor_entity,
            id,
            aabb,
            mut actor,
            mut identity,
            mut disposition,
            mut health,
            mut combat,
            mut intent,
            mut cooldowns,
            mut clusters,
            mut npc_status,
            npc_config,
        ) in &mut actors
        {
            let prefix = match *disposition {
                ActorDisposition::Peaceful => "npc",
                ActorDisposition::Hostile => "enemy",
            };
            if target_is_ignored(&event.ignored_targets, prefix, id.as_str()) {
                continue;
            }
            if !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            let mut em_opt = clusters.as_mut().map(|cq| cq.as_enemy_mut());
            let npc_target = match (npc_status.as_deref_mut(), npc_config) {
                (Some(status), Some(config)) => Some(NpcHitTarget {
                    status,
                    config,
                    aabb: aabb.aabb(),
                }),
                _ => None,
            };
            if apply_actor_hit(
                &event,
                actor_entity,
                &mut actor,
                em_opt.as_mut(),
                npc_target,
                &mut banner,
                combat_banter.as_deref(),
                &mut writers,
            ) {
                actor_hit_this_event = true;
                match &*actor {
                    ActorRuntime::Enemy => {
                        if let Some(em) = em_opt.as_ref() {
                            sync_actor_components_from_enemy(
                                em,
                                &mut identity,
                                &mut disposition,
                                &mut health,
                                &mut combat,
                                &mut intent,
                                &mut cooldowns,
                            );
                        }
                    }
                    ActorRuntime::Npc => {
                        if let (Some(status), Some(config)) = (npc_status.as_deref(), npc_config) {
                            let (i, d, h, c, it, cd) =
                                super::actors::npc_component_snapshot(config, status);
                            *identity = i;
                            *disposition = d;
                            *health = h;
                            *combat = c;
                            *intent = it;
                            *cooldowns = cd;
                        }
                    }
                }
            }
        }
        let mut boss_hit_this_event = false;
        for (id, _aabb, mut feature, attack_state, animation_frame) in &mut bosses {
            if target_is_ignored(&event.ignored_targets, "boss", id.as_str()) {
                continue;
            }
            if apply_boss_hit(
                &event,
                feature.as_boss_mut(),
                attack_state,
                animation_frame,
                &mut banner,
                combat_banter.as_deref(),
                &mut writers,
                boss_registry.as_deref_mut(),
                music_request.as_deref_mut(),
                cutscene_queue.as_deref_mut(),
            ) {
                boss_hit_this_event = true;
            }
        }

        if actor_hit_this_event || boss_hit_this_event {
            let target_attacker = event.attacker.or_else(|| primary_q.single().ok());
            if let Some(attacker) = target_attacker {
                for (entity, mut combat) in &mut player_combat_q {
                    if entity != attacker {
                        continue;
                    }
                    combat.hitstop_timer = combat.hitstop_timer.max(0.06);
                    combat.flash_timer = combat.flash_timer.max(0.10);
                    break;
                }
            }
            writers.sfx.write(SfxMessage::Hit {
                pos: event.volume.center(),
            });
        }

        for (entity, id, name, aabb, mut feature) in &mut breakables {
            if target_is_ignored(&event.ignored_targets, "breakable", id.as_str()) {
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
            writers.vfx.write(VfxMessage::Impact {
                pos: midpoint(event.volume.center(), aabb.center),
            });
            if broke {
                begin_ecs_breakable_respawn(&mut writers.commands, entity, &feature.breakable);
                banner.show(format!("broke {}", name.0.as_str()), 2.6);
                // Loot: a smashed crate/pot drops a small coin (same collectible
                // pickup path as enemy drops).
                drop_currency_coin(
                    &mut writers.commands,
                    id.as_str(),
                    aabb.center,
                    BREAKABLE_BOUNTY,
                );
                emit_breakable_destroyed(
                    aabb.center,
                    &mut writers.sfx,
                    &mut writers.vfx,
                    &mut writers.debris,
                );
            }
        }
    }
}

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
/// `FeatureAabb` instead).
struct NpcHitTarget<'a> {
    status: &'a mut super::npc_clusters::NpcStatus,
    config: &'a super::npc_clusters::NpcConfig,
    aabb: ae::Aabb,
}

fn apply_actor_hit(
    event: &HitEvent,
    actor_entity: Entity,
    actor: &mut ActorRuntime,
    enemy: Option<&mut super::enemy_clusters::EnemyMut<'_>>,
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
            let bark_anchor = super::super::npcs::npc_bark_anchor_from_aabb(npc.aabb);
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
                    id: super::super::npcs::npc_flag_id(npc.config),
                    on: true,
                });
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: bark_anchor,
                    text: super::super::npcs::npc_hostile_bark_line(npc.config).to_string(),
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
                    text: super::super::npcs::npc_hit_bark_line(npc.config, npc.status).to_string(),
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
                            kind: ambition_effects::vfx::ExplosionKind::ClassicBurst,
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

/// Apply one landed attacker-side hit to a single boss and emit its
/// feedback. Routes damage through `record_boss_damage` (engine
/// `BossEncounterState` is the source of truth) when the encounter
/// resources are present, falling back to direct runtime mutation for
/// test fixtures without the encounter machine. Cut-rope puzzle bosses
/// give honest local impact feedback but take no HP damage from
/// ordinary player hits.
///
/// Returns `true` when the boss took the hit (so the caller drives the
/// shared landed-hit feedback). Early-returns `false` for a dead boss,
/// a miss against the live damageable volumes, or an invulnerable-phase
/// swallow — matching the previous `continue` behavior.
///
/// Extracted from `apply_feature_hit_events` per ecs-cleanup-plan.md #4.
#[allow(clippy::too_many_arguments)]
fn apply_boss_hit(
    event: &HitEvent,
    boss: super::boss_clusters::BossMut<'_>,
    attack_state: &crate::brain::BossAttackState,
    animation_frame: Option<&crate::features::BossAnimationFrameSample>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&crate::features::banter::CombatBanterRegistry>,
    writers: &mut FeatureHitWriters<'_, '_>,
    boss_registry: Option<&mut BossEncounterRegistry>,
    music_request: Option<&mut BossEncounterMusicRequest>,
    cutscene_queue: Option<&mut CutsceneTriggerQueue>,
) -> bool {
    if !boss.status.alive {
        return false;
    }
    if boss.config.behavior.environmental_kill_only
        && matches!(
            event.source,
            HitSource::PlayerSlash { .. } | HitSource::PlayerProjectile { .. }
        )
    {
        // Environmental puzzle bosses (e.g. the Smirking Behemoth) take
        // no HP from ordinary player hits; those should give honest local
        // feedback only when they overlap the body hurtbox. The authored
        // environmental rule (the rope/anvil trap in
        // `ambition_content::bosses::cut_rope`) owns the only kill
        // condition. This is data-driven via `environmental_kill_only`
        // so core never names a specific boss. Keep this before the
        // generic damage branch so harmless feedback cannot accidentally
        // route through `record_boss_damage`.
        let damageable = crate::features::damageable_volumes(
            &crate::features::BossVolumeContext::from_ref(boss.as_ref(), attack_state)
                .with_animation_frame(animation_frame),
        );
        if let Some(hit_aabb) = damageable
            .iter()
            .find(|part| event.volume.strict_intersects(**part))
        {
            boss.status.hit_flash = 0.18;
            let impact = midpoint(event.volume.center(), hit_aabb.center());
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            return true;
        }
        return false;
    }
    // Damageable volumes read from BossAttackState (the
    // brain's source of truth for which strike profile is
    // live) so GNU-ton's head-descent vulnerability window
    // and the standard whole-body hurtbox agree on a single
    // attack-state source.
    let damageable = crate::features::damageable_volumes(
        &crate::features::BossVolumeContext::from_ref(boss.as_ref(), attack_state)
            .with_animation_frame(animation_frame),
    );
    let Some(hit_aabb) = damageable
        .iter()
        .find(|part| event.volume.strict_intersects(**part))
    else {
        return false;
    };
    // Speech bubble bark when player lands a hit, debounced by hit_flash.
    let should_bark = boss.status.hit_flash < 0.05;
    boss.status.hit_flash = 0.18;
    if should_bark {
        if let Some(reg) = combat_banter {
            let strikes = boss.status.health.max - boss.status.health.current;
            if let Some(line) = reg.pick_hit_bark(&boss.config.name, strikes.max(0) as u32) {
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: boss.bark_anchor(),
                    text: line.to_string(),
                });
            }
        }
    }
    let amount = event.damage.max(1);
    // Boss encounter authoritative state (OVERNIGHT-TODO #8).
    // Apply damage to engine state via the registry; the
    // outcome tells us whether the hit actually landed (false
    // during invulnerable phases) and whether it killed the
    // boss. Mirror the new HP back to the runtime so
    // downstream readers (HUD health bar, bark count, etc.)
    // see it on the same tick instead of one frame late.
    //
    // If any of the boss-encounter resources is missing (test
    // fixtures that don't install the encounter machine) we
    // fall back to the pre-inversion direct mutation so the
    // runtime still takes damage and the test exercises the
    // hit path.
    let outcome = match (boss_registry, music_request, cutscene_queue) {
        (Some(registry), Some(music), Some(cutscene)) => record_boss_damage(
            registry,
            music,
            cutscene,
            banner,
            boss.config.id.as_str(),
            amount,
        ),
        _ => None,
    };
    let (applied, killed) = match outcome {
        Some(outcome) => {
            boss.status.health.current = outcome.hp_remaining;
            (outcome.applied, outcome.killed)
        }
        // No engine encounter / missing test resource. Fall
        // back to the pre-inversion direct mutation so the
        // runtime still takes damage.
        None => {
            let died = boss.status.health.damage(amount);
            (true, died)
        }
    };
    if !applied {
        // Invulnerable phase swallowed the damage. Skip the
        // hit VFX / GameplayEffect signal so the player sees
        // the boss as a hard wall during the beat instead of
        // a fake impact.
        return false;
    }
    let impact = midpoint(event.volume.center(), hit_aabb.center());
    writers.vfx.write(VfxMessage::Impact { pos: impact });
    // Boss HP is applied directly via `record_boss_damage`
    // above, so the engine BossEncounterState is the source of
    // truth; the old no-op GameplayEffect::DamageBoss bus hook
    // has been removed.
    if killed {
        boss.status.alive = false;
        banner.show(format!("defeated boss {}", boss.config.name), 2.6);
        writers.vfx.write(VfxMessage::Burst {
            pos: boss.kin.pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        writers.debris.write(DebrisBurstMessage {
            pos: boss.kin.pos,
            cue: PhysicsDebrisCue::BossRagdoll,
        });
        writers.sfx.write(SfxMessage::Death { pos: boss.kin.pos });
        // A jackpot of coins + a heal for the hardest fight, on top of the ability.
        drop_currency_coin(
            &mut writers.commands,
            &boss.config.behavior.id,
            boss.kin.pos,
            BOSS_BOUNTY,
        );
        drop_health_pickup(
            &mut writers.commands,
            &boss.config.behavior.id,
            boss.kin.pos + ae::Vec2::new(24.0, 0.0),
            3,
        );
        // North star: "every boss a failed objective function, every upgrade a
        // theorem" — a defeated boss drops the ability it embodies, so combat
        // (not just the merchant) teaches the player new verbs.
        if let Some(ability_id) = boss.config.behavior.reward_ability.as_deref() {
            if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                drop_ability_pickup(
                    &mut writers.commands,
                    &boss.config.behavior.id,
                    boss.kin.pos,
                    ability_id,
                    item.display_name(),
                );
            }
        }
        // …and its signature wielded attack drops as a ground-item gauntlet the
        // player picks up + uses (the player literally wields the boss's move).
        if let Some(gauntlet_id) = boss.config.behavior.signature_gauntlet.as_deref() {
            if let Some(spec) = crate::brain::held_item_by_id(gauntlet_id) {
                writers.commands.spawn((
                    crate::items::pickup::GroundItem {
                        spec,
                        // Offset from the ability pickup so the two drops don't stack.
                        pos: boss.kin.pos + ae::Vec2::new(36.0, 0.0),
                        vel: ae::Vec2::ZERO,
                        half_extent: ae::Vec2::splat(18.0),
                    },
                    bevy::prelude::Name::new("Boss signature gauntlet"),
                ));
            }
        }
    }
    true
}

// `begin_ecs_breakable_respawn` / `emit_breakable_destroyed` moved to
// the combat kit (`crate::mechanics::combat::breakables`) — they are
// generic breakable side-effect helpers shared by the typed-damage
// path here and the kit's stand-to-break path.
pub(crate) use crate::mechanics::combat::breakables::{
    begin_ecs_breakable_respawn, emit_breakable_destroyed,
};

#[cfg(test)]
mod tests;
