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
use super::{
    ae, sync_actor_components_from_enemy, ActorCombatState,
    ActorCooldowns, ActorDisposition, ActorHealth, ActorIdentity, ActorIntent, ActorRuntime,
    BossConfig, BreakableFeature, EnemyArchetype, FeatureAabb, FeatureId, FeatureName,
    FeatureSimEntity, GameplayBanner, HitEvent, HitSource, PickupFeature, RespawnTimer,
    SetFlagRequested,
};
use crate::audio::SfxMessage;
use crate::boss_encounter::{record_boss_damage, BossEncounterRegistry};
use crate::encounter::BossEncounterMusicRequest;
use crate::features::ActorStimulus;
use crate::presentation::cutscene::CutsceneTriggerQueue;
use crate::presentation::fx::{ParticleKind, VfxMessage};
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};

#[derive(SystemParam)]
pub struct FeatureHitWriters<'w> {
    pub set_flag: MessageWriter<'w, SetFlagRequested>,
    pub actor_stimuli: MessageWriter<'w, ActorStimulus>,
    pub sfx: MessageWriter<'w, SfxMessage>,
    pub vfx: MessageWriter<'w, VfxMessage>,
    pub debris: MessageWriter<'w, DebrisBurstMessage>,
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

/// Deterministic (FNV-1a over the id) gate so ~1 in 4 enemy *kinds* drops a heart.
/// Deterministic, not random, so the headless sim stays reproducible — the same
/// enemy always drops or always doesn't.
pub fn id_drops_health(id: &str) -> bool {
    let h = id
        .bytes()
        .fold(2166136261u32, |a, b| (a ^ b as u32).wrapping_mul(16777619));
    h % 4 == 0
}

/// Spawn a collectible currency coin at `pos` — an enemy's death drop. Reuses the
/// exact pickup entity shape that LDtk-placed coins use, so the already-registered
/// [`super::collect_ecs_pickups`] grants it (and plays `WORLD_COIN_PICKUP`) when a
/// player overlaps it. The coin sits where the enemy fell and never respawns
/// (`Pickup::new` defaults to [`crate::interaction::RespawnPolicy::Never`]).
pub fn drop_currency_coin(commands: &mut Commands, id: &str, pos: ae::Vec2, amount: i32) {
    commands.spawn((
        FeatureSimEntity,
        FeatureId::new(format!("coin:{id}")),
        FeatureName::new("Coin"),
        FeatureAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(crate::interaction::Pickup::new(
            format!("coin:{id}"),
            crate::interaction::PickupKind::Currency { amount },
        )),
    ));
}

/// Spawn a collectible health heart at `pos` (a sometimes-drop on enemy defeat),
/// same pickup path as the coin so `collect_ecs_pickups` heals the player on
/// overlap via `PlayerHealRequested`.
pub fn drop_health_pickup(commands: &mut Commands, id: &str, pos: ae::Vec2, amount: i32) {
    commands.spawn((
        FeatureSimEntity,
        FeatureId::new(format!("heart:{id}")),
        FeatureName::new("Health"),
        FeatureAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(crate::interaction::Pickup::new(
            format!("heart:{id}"),
            crate::interaction::PickupKind::Health { amount },
        )),
    ));
}

/// The ability a defeated boss bestows, keyed by boss profile id (catalog
/// `dialog_id`), or `None` for bosses that grant nothing (puzzle bosses, etc.).
/// Each pairing reads as "this boss embodies that theorem".
fn boss_reward_ability(boss_id: &str) -> Option<&'static str> {
    match boss_id {
        // The false-god FSM flits through the air — it drops Blink.
        "flying_spaghetti_monster_boss" => Some("blink"),
        // The grounded T-Rex lunges and anchors — it drops Grapple.
        "trex_boss" => Some("grapple"),
        // GNU-Ton hurls apples in arcs — it drops the Fireball projectile.
        "gnu_ton" => Some("fireball"),
        // The Clockwork Warden rewinds and repositions — it drops Mark/Recall.
        "clockwork_warden" => Some("markrecall"),
        // Mockingbird (mimic) + Smirking Behemoth (environmental puzzle) grant
        // nothing on their own.
        _ => None,
    }
}

/// Spawn a collectible ability pickup at `pos` — a defeated boss's reward. Reuses
/// the standard pickup entity shape so [`super::collect_ecs_pickups`] grants the
/// ability to the player's catalog ([`crate::items::OwnedItems`]) on overlap.
pub fn drop_ability_pickup(
    commands: &mut Commands,
    boss_id: &str,
    pos: ae::Vec2,
    ability_id: &str,
    ability_name: &str,
) {
    commands.spawn((
        FeatureSimEntity,
        FeatureId::new(format!("ability_drop:{boss_id}")),
        FeatureName::new(ability_name.to_string()),
        FeatureAabb::from_center_size(pos, ae::Vec2::new(16.0, 16.0)),
        PickupFeature::new(crate::interaction::Pickup::new(
            format!("ability_drop:{boss_id}"),
            crate::interaction::PickupKind::Ability {
                ability_id: ability_id.to_string(),
            },
        )),
    ));
}

/// Apply typed slash / projectile / pogo hit messages to ECS feature targets.
pub fn apply_feature_hit_events(
    mut commands: Commands,
    mut hit_events: MessageReader<HitEvent>,
    mut banner: ResMut<GameplayBanner>,
    combat_banter: Option<Res<crate::content::banter::CombatBanterRegistry>>,
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
        With<FeatureSimEntity>,
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
                    begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
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
                &mut commands,
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
                        if let (Some(status), Some(config)) =
                            (npc_status.as_deref(), npc_config)
                        {
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
            let key = format!("boss:{}", id.as_str());
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            if apply_boss_hit(
                &event,
                feature.as_boss_mut(),
                attack_state,
                animation_frame,
                &mut banner,
                combat_banter.as_deref(),
                &mut commands,
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
            writers.vfx.write(VfxMessage::Impact {
                pos: midpoint(event.volume.center(), aabb.center),
            });
            if broke {
                begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                banner.show(format!("broke {}", name.0.as_str()), 2.6);
                // Loot: a smashed crate/pot drops a small coin (same collectible
                // pickup path as enemy drops).
                drop_currency_coin(&mut commands, id.as_str(), aabb.center, BREAKABLE_BOUNTY);
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
    combat_banter: Option<&crate::content::banter::CombatBanterRegistry>,
    commands: &mut Commands,
    writers: &mut FeatureHitWriters,
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
            let killed = if em.config.archetype == EnemyArchetype::InfiniteSandbag {
                false
            } else {
                em.status.health.damage(damage_amount)
            };
            let impact = midpoint(event.volume.center(), em.kin.pos);
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            if killed {
                em.status.alive = false;
                if em.config.archetype == EnemyArchetype::FiniteSandbag {
                    em.status.respawn_timer = 0.85;
                    banner.show(format!("{} dropped; respawning", em.config.name), 2.6);
                } else {
                    banner.show(format!("defeated {}", em.config.name), 2.6);
                    // Earn-side: a defeated enemy drops a collectible coin so the
                    // player can fund the merchant / ability shop from combat, and
                    // ~1 in 4 enemy kinds also drops a heart (combat sustain).
                    drop_currency_coin(commands, &em.config.id, em.kin.pos, ENEMY_BOUNTY);
                    if id_drops_health(&em.config.id) {
                        drop_health_pickup(
                            commands,
                            &em.config.id,
                            em.kin.pos + ae::Vec2::new(18.0, 0.0),
                            ENEMY_HEALTH_DROP,
                        );
                    }
                    // Steal the enemy's weapon: a defeated enemy that was wielding
                    // a held item drops it as a `GroundItem` the player can grab +
                    // wield (e.g. a pirate's gun-sword), via the existing pickup path.
                    if let Some(spec) = em.config.archetype.held_item_spec() {
                        commands.spawn((
                            crate::item_pickup::GroundItem {
                                spec,
                                pos: em.kin.pos + ae::Vec2::new(-14.0, 0.0),
                                vel: ae::Vec2::ZERO,
                                half_extent: ae::Vec2::splat(16.0),
                            },
                            bevy::prelude::Name::new("Dropped weapon"),
                        ));
                    }
                    if !em.config.id.starts_with("encounter:")
                        && em.config.archetype != EnemyArchetype::InfiniteSandbag
                        && em.config.archetype != EnemyArchetype::FiniteSandbag
                    {
                        use crate::features::EnemyRespawnPolicy as P;
                        let flag_id = match em.config.archetype.respawn_policy() {
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
    combat_banter: Option<&crate::content::banter::CombatBanterRegistry>,
    commands: &mut Commands,
    writers: &mut FeatureHitWriters,
    boss_registry: Option<&mut BossEncounterRegistry>,
    music_request: Option<&mut BossEncounterMusicRequest>,
    cutscene_queue: Option<&mut CutsceneTriggerQueue>,
) -> bool {
    if !boss.status.alive {
        return false;
    }
    if crate::boss_encounter::is_cut_rope_boss(&boss.config.behavior.id)
        && matches!(
            event.source,
            HitSource::PlayerSlash { .. } | HitSource::PlayerProjectile { .. }
        )
    {
        // Smirking Behemoth is an environmental puzzle boss:
        // ordinary player hits should give honest local feedback only
        // when they overlap the body hurtbox, but they must not damage
        // the boss. The LDtk-authored rope/anvil system owns the only
        // kill condition. Keep this before the generic damage branch
        // so harmless feedback cannot accidentally route through
        // `record_boss_damage`.
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
        // A jackpot of coins for the hardest fight, on top of the ability reward.
        drop_currency_coin(commands, &boss.config.behavior.id, boss.kin.pos, BOSS_BOUNTY);
        // North star: "every boss a failed objective function, every upgrade a
        // theorem" — a defeated boss drops the ability it embodies, so combat
        // (not just the merchant) teaches the player new verbs.
        if let Some(ability_id) = boss_reward_ability(&boss.config.behavior.id) {
            if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                drop_ability_pickup(
                    commands,
                    &boss.config.behavior.id,
                    boss.kin.pos,
                    ability_id,
                    item.display_name(),
                );
            }
        }
    }
    true
}

/// Read-only hit test used by systems that need immediate projectile / attack
/// feedback while damage application is still drained through
/// typed Bevy messages.
pub fn ecs_hit_event_hits_breakable(
    event: &HitEvent,
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

pub fn ecs_hit_event_hits_actor(
    event: &HitEvent,
    actors: &Query<
        (
            &FeatureId,
            &FeatureAabb,
            &ActorDisposition,
            &ActorCombatState,
        ),
        (With<FeatureSimEntity>, Without<BossConfig>),
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

pub fn ecs_hit_event_hits_boss(
    event: &HitEvent,
    bosses: &Query<
        (
            &FeatureId,
            &FeatureAabb,
            super::boss_clusters::BossClusterRef,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
) -> bool {
    // Check against `damageable_volumes` so the hit-check matches
    // what `apply_feature_hit_events` will actually apply damage
    // to. Multi-part bosses (e.g. GNU-ton) have a gross
    // `FeatureAabb` covering the whole creature but only the head
    // is actually damageable — checking against the gross AABB
    // would over-trigger projectile termination on the body without
    // ever applying damage. `damageable_volumes` reads the brain's
    // `BossAttackState` to decide head-descent vs rest position, and
    // the live `BossAnimationFrameSample` (same component
    // `apply_boss_hit` consumes) so the projectile's hit/terminate
    // check locks to the exact rendered frame instead of an
    // elapsed-time estimate — otherwise the projectile could
    // register a hit a few frames off from where the head is drawn
    // and where damage actually lands.
    bosses
        .iter()
        .any(|(id, _aabb, feature, attack_state, animation_frame)| {
            let key = format!("boss:{}", id.as_str());
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                return false;
            }
            if !feature.status.alive {
                return false;
            }
            crate::features::damageable_volumes(
                &crate::features::BossVolumeContext::from_ref(feature.as_boss_ref(), attack_state)
                    .with_animation_frame(animation_frame),
            )
            .iter()
            .any(|part| event.volume.strict_intersects(*part))
        })
}

/// Schedule a broken breakable for respawn if its policy allows.
///
/// Called from both `apply_feature_hit_events` (typed damage path) and
/// `update_ecs_breakables` (stand-to-break path), so it lives here as a
/// `pub(super)` helper rather than duplicating the policy check.
pub(super) fn begin_ecs_breakable_respawn(
    commands: &mut Commands,
    entity: Entity,
    breakable: &crate::interaction::Breakable,
) {
    if let crate::actor::RespawnPolicy::AfterSeconds(seconds) = breakable.respawn {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::ecs::enemy_component_snapshot;
    use crate::features::{HitMode, HitTarget};
    use bevy::prelude::{App, Update};

    fn spawn_hostile_actor(app: &mut App) -> bevy::prelude::Entity {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        let mut enemy = crate::content::features::ecs::enemy_clusters::EnemyClusterScratch::new(
            "kernel_guide".to_string(),
            "Kernel Guide".to_string(),
            aabb,
            crate::actor::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.status.health = crate::actor::Health::new(5);
        let (identity, disposition, health, combat, intent, cooldowns) =
            enemy_component_snapshot(&enemy);
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("kernel_guide"),
                FeatureAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
                ActorRuntime::Enemy,
                enemy.into_components(),
                identity,
                disposition,
                health,
                combat,
                intent,
                cooldowns,
            ))
            .id()
    }

    #[test]
    fn victim_side_enemy_body_hit_does_not_damage_features() {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<HitEvent>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_feature_hit_events);

        let actor_entity = spawn_hostile_actor(&mut app);
        let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        app.world_mut().write_message(HitEvent {
            volume: event_volume,
            damage: 1,
            source: HitSource::EnemyBody,
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });

        app.update();

        let health = app
            .world()
            .get::<ActorHealth>(actor_entity)
            .expect("hostile actor exists");
        assert_eq!(
            health.health.current, 5,
            "enemy body contact should not damage the enemy that emitted it"
        );
    }

    #[test]
    fn enemy_charge_crash_is_processed_as_enemy_damage() {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<HitEvent>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_feature_hit_events);

        let actor_entity = spawn_hostile_actor(&mut app);
        let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        app.world_mut().write_message(HitEvent {
            volume: event_volume,
            damage: 10,
            source: HitSource::EnemyChargeCrash,
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });

        app.update();

        let health = app
            .world()
            .get::<ActorHealth>(actor_entity)
            .expect("hostile actor exists");
        assert_eq!(
            health.health.current, 0,
            "enemy charge crash should damage and kill the crashing enemy"
        );
        let status = app
            .world()
            .get::<super::super::enemy_clusters::EnemyStatus>(actor_entity)
            .expect("hostile actor cluster status exists");
        assert!(
            !status.alive,
            "charge crash should mark the enemy dead through the normal kill path"
        );
    }

    #[test]
    fn player_slash_damages_and_can_kill_a_hostile_actor() {
        // The core attack loop through the unified HitEvent path: a
        // player slash (attacker-side source) reduces a hostile
        // actor's HP, and enough damage routes through the normal kill
        // path. Complements the enemy-side tests above.
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<HitEvent>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_feature_hit_events);

        let actor_entity = spawn_hostile_actor(&mut app); // HP 5
        let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));

        // First slash: 2 damage → 3 HP, still alive.
        app.world_mut().write_message(HitEvent {
            volume: event_volume,
            damage: 2,
            source: HitSource::PlayerSlash { knock_x: 120.0 },
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        assert_eq!(
            app.world().get::<ActorHealth>(actor_entity).unwrap().health.current,
            3,
            "a 2-damage player slash should bring the 5-HP enemy to 3"
        );
        assert!(
            app.world()
                .get::<super::super::enemy_clusters::EnemyStatus>(actor_entity)
                .unwrap()
                .alive,
            "the enemy should still be alive after one slash"
        );

        // Lethal slash: 5 damage → dead through the normal kill path.
        app.world_mut().write_message(HitEvent {
            volume: event_volume,
            damage: 5,
            source: HitSource::PlayerSlash { knock_x: 120.0 },
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();
        assert_eq!(
            app.world().get::<ActorHealth>(actor_entity).unwrap().health.current,
            0,
            "a lethal slash should bring the enemy to 0 HP"
        );
        assert!(
            !app.world()
                .get::<super::super::enemy_clusters::EnemyStatus>(actor_entity)
                .unwrap()
                .alive,
            "the killed enemy should be marked dead"
        );
    }

    #[test]
    fn player_slash_shatters_a_breakable() {
        // Completes the attacker-side hit matrix: a player slash on a
        // 1-HP breakable shatters it through apply_feature_hit_events.
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<HitEvent>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_feature_hit_events);

        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 20.0));
        let breakable = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("crate"),
                FeatureName::new("crate"),
                FeatureAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
                BreakableFeature::new(crate::interaction::Breakable::new("crate", 1)),
            ))
            .id();
        assert!(!app.world().get::<BreakableFeature>(breakable).unwrap().broken());

        app.world_mut().write_message(HitEvent {
            volume: aabb,
            damage: 2,
            source: HitSource::PlayerSlash { knock_x: 0.0 },
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        app.update();

        assert!(
            app.world().get::<BreakableFeature>(breakable).unwrap().broken(),
            "a player slash should shatter a 1-HP breakable"
        );

        // Shattering a crate drops one collectible coin.
        let mut q = app.world_mut().query::<&PickupFeature>();
        let coins = q
            .iter(app.world())
            .filter(|p| matches!(p.kind(), crate::interaction::PickupKind::Currency { .. }))
            .count();
        assert_eq!(coins, 1, "shattering a crate drops one coin");
    }

    #[test]
    fn enemy_defeat_drops_a_collectible_currency_coin() {
        let mut app = App::new();
        app.add_systems(Update, |mut c: Commands| {
            drop_currency_coin(&mut c, "goblin_1", ae::Vec2::new(40.0, 50.0), ENEMY_BOUNTY);
        });
        app.update();
        let mut q = app.world_mut().query::<(&PickupFeature, &FeatureId)>();
        let rows: Vec<(crate::interaction::PickupKind, String)> = q
            .iter(app.world())
            .map(|(p, id)| (p.kind().clone(), id.as_str().to_string()))
            .collect();
        assert_eq!(rows.len(), 1, "exactly one coin dropped");
        assert_eq!(rows[0].1, "coin:goblin_1", "coin id is keyed to the enemy");
        assert_eq!(
            rows[0].0,
            crate::interaction::PickupKind::Currency {
                amount: ENEMY_BOUNTY
            },
            "the drop is a currency coin worth the bounty",
        );
    }

    #[test]
    fn defeated_boss_drops_its_signature_ability() {
        // Each boss is paired with the ability it embodies; others grant nothing.
        assert_eq!(
            boss_reward_ability("flying_spaghetti_monster_boss"),
            Some("blink")
        );
        assert_eq!(boss_reward_ability("trex_boss"), Some("grapple"));
        assert_eq!(boss_reward_ability("gnu_ton"), Some("fireball"));
        assert_eq!(boss_reward_ability("clockwork_warden"), Some("markrecall"));
        assert_eq!(boss_reward_ability("mockingbird"), None);
        assert_eq!(boss_reward_ability("smirking_behemoth_boss"), None);
        // Every mapped reward resolves to a real catalog item.
        for boss in ["flying_spaghetti_monster_boss", "trex_boss", "gnu_ton", "clockwork_warden"] {
            let id = boss_reward_ability(boss).unwrap();
            assert!(
                crate::items::Item::from_dialog_id(id).is_some(),
                "boss {boss} -> ability {id} must be a real catalog item",
            );
        }

        // The drop spawns a single collectible Ability pickup.
        let mut app = App::new();
        app.add_systems(Update, |mut c: Commands| {
            drop_ability_pickup(&mut c, "trex_boss", ae::Vec2::new(10.0, 20.0), "grapple", "Grapple");
        });
        app.update();
        let mut q = app.world_mut().query::<&PickupFeature>();
        let kinds: Vec<crate::interaction::PickupKind> =
            q.iter(app.world()).map(|p| p.kind().clone()).collect();
        assert_eq!(kinds.len(), 1, "one ability pickup dropped");
        assert_eq!(
            kinds[0],
            crate::interaction::PickupKind::Ability {
                ability_id: "grapple".to_string()
            },
        );
    }

    #[test]
    fn enemy_health_drop_is_deterministic_and_spawns_a_heart() {
        // The gate is a pure function of the id, so the headless sim is reproducible.
        assert_eq!(id_drops_health("goblin_42"), id_drops_health("goblin_42"));
        // The drop spawns one collectible Health pickup.
        let mut app = App::new();
        app.add_systems(Update, |mut c: Commands| {
            drop_health_pickup(&mut c, "any", ae::Vec2::ZERO, ENEMY_HEALTH_DROP);
        });
        app.update();
        let mut q = app.world_mut().query::<&PickupFeature>();
        let kinds: Vec<crate::interaction::PickupKind> =
            q.iter(app.world()).map(|p| p.kind().clone()).collect();
        assert_eq!(kinds.len(), 1, "one heart dropped");
        assert!(
            matches!(kinds[0], crate::interaction::PickupKind::Health { .. }),
            "the drop is a health pickup",
        );
    }

    #[test]
    fn an_armed_enemy_archetype_resolves_a_weapon_to_drop() {
        // The defeat branch's weapon drop keys off `held_item_spec()`; the pirate
        // carries a gun-sword, so a defeated pirate drops one.
        let spec = EnemyArchetype::PirateOnShark.held_item_spec();
        assert!(spec.is_some(), "PirateOnShark carries a weapon");
        assert_eq!(spec.unwrap().id.as_str(), "gun_sword");
    }
}
