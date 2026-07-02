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

use bevy::ecs::system::SystemParam;
use bevy::prelude::{
    Commands, Entity, MessageReader, MessageWriter, Query, Res, ResMut, With, Without,
};

use super::super::util::{approximately_same_aabb, midpoint};
use super::damage_drops::drop_currency_coin;
use super::{
    sync_actor_components_from_cluster, ActorCooldowns, ActorDisposition, ActorIdentity,
    ActorIntent, BodyCombat, BreakableFeature, CenteredAabb, FeatureId, FeatureName,
    FeatureSimEntity, GameplayBanner, HitEvent, HitSource, SetFlagRequested,
};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
#[cfg(test)]
use super::damage_drops::EXPLODER_BLAST_DAMAGE;
use super::damage_predicates::target_is_ignored;
#[cfg(test)]
use super::PickupFeature;
use ambition_sfx::SfxMessage;
use crate::features::ActorStimulus;
use crate::world::physics::DebrisBurstMessage;
use ambition_vfx::vfx::VfxMessage;

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
    // Knockback feel for struck actors (§A2 step 6). `Option` so minimal
    // headless test worlds that don't stand up the tuning resource still run
    // (they get the default feel).
    feel_tuning: Option<Res<crate::time::feel::SandboxFeelTuning>>,
    mut breakables: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            &mut BreakableFeature,
        ),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            Entity,
            &FeatureId,
            &CenteredAabb,
            &mut ActorIdentity,
            &ActorDisposition,
            &mut BodyCombat,
            &mut ActorIntent,
            &mut ActorCooldowns,
            // Provoke accumulator (shared aggression component). `Option` so
            // minimal test fixtures that spawn a bare actor without it still
            // match; production actors always carry it.
            Option<&mut super::super::components::ActorAggression>,
            // Dialogue payload — present on talkable actors (drives barks).
            Option<&super::super::components::ActorInteraction>,
            super::actor_clusters::ActorClusterQueryData,
        ),
        // Bosses are handled by the disjoint `bosses` query; both take
        // `&mut BodyKinematics` (the unified component), so exclude bosses
        // here to keep the two queries provably non-aliasing. `Without<PlayerEntity>`
        // keeps this `&mut BodyCombat` actor query disjoint from the player
        // `&mut BodyCombat` query below, now that both share the unified component.
        (
            With<FeatureSimEntity>,
            Without<super::boss_clusters::BossConfig>,
            Without<crate::actor::PlayerEntity>,
        ),
    >,
    mut bosses: Query<
        (
            &FeatureId,
            &CenteredAabb,
            super::boss_clusters::BossClusterQueryData,
            // The boss's shared body components (§A1): HP authority + the
            // hit-flash the damage path arms. `Without<PlayerEntity>` keeps
            // this `&mut BodyCombat` provably disjoint from the player query
            // below (the actor query is already `Without<BossConfig>`).
            &mut crate::actor::BodyHealth,
            &mut crate::actor::BodyCombat,
            &ambition_characters::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        (With<FeatureSimEntity>, Without<crate::actor::PlayerEntity>),
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
        (
            bevy::prelude::Entity,
            &mut crate::actor::BodyCombat,
            // The attacker's live swing, so a multi-active-frame slash records
            // which targets it has already struck and never double-hits them.
            Option<&mut crate::player::BodyMelee>,
        ),
        bevy::prelude::With<crate::actor::PlayerEntity>,
    >,
    primary_q: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<crate::actor::PlayerEntity>,
            bevy::prelude::With<crate::actor::PrimaryPlayer>,
        ),
    >,
    mut writers: FeatureHitWriters,
    // R3: boss damage mutates the boss ENTITY directly (`apply_boss_hit` →
    // `apply_entity_boss_damage`), so this system no longer needs the boss
    // encounter resources — death save/quest/music resolution lives in
    // `update_boss_encounters`.
) {
    let feel = feel_tuning.map(|r| *r).unwrap_or_default();
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
                if !approximately_same_aabb(aabb.aabb(), event.volume.bounds()) {
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
        // Relational actor-vs-actor (S3e): an event pre-resolved to a single
        // non-player actor victim (`HitTarget::Actor`) is applied to exactly that
        // body, whatever its source direction — this is how an Enemy/Boss swing
        // damages another actor without flowing through the player path.
        let actor_target = match event.target {
            crate::combat::events::HitTarget::Actor(entity) => Some(entity),
            _ => None,
        };
        // Victim-side sources (enemy touch, enemy swings, boss body
        // contact, hazards) are consumed by the player-damage path.
        // The feature drain only applies attacker-side player hits
        // here (plus the pre-resolved actor-vs-actor hits above);
        // otherwise an `EnemyBody` event would damage the same enemy
        // that emitted it when the volume overlaps its own AABB.
        if actor_target.is_none() && !event.source.is_attacker_side() {
            continue;
        }
        // Ignore-keys (`prefix:id`) of every target struck by THIS event, folded
        // back into the attacker's per-swing `hit_targets` below so a slash that
        // emits on every active frame only damages each target once.
        let mut landed_keys: Vec<String> = Vec::new();
        let mut actor_hit_this_event = false;
        for (
            actor_entity,
            id,
            aabb,
            mut identity,
            disposition,
            mut combat,
            mut intent,
            mut cooldowns,
            mut aggression,
            interaction,
            mut cq,
        ) in &mut actors
        {
            // Pre-resolved actor victim: apply ONLY to that entity.
            if let Some(target_entity) = actor_target {
                if actor_entity != target_entity {
                    continue;
                }
            }
            let prefix = if disposition.is_hostile() {
                "enemy"
            } else {
                "npc"
            };
            if target_is_ignored(&event.ignored_targets, prefix, id.as_str()) {
                continue;
            }
            if !event.volume.intersects_aabb(aabb.aabb()) {
                continue;
            }
            let interactable = interaction.map(|i| &i.interactable);
            let mut em = cq.as_actor_mut();
            if apply_actor_hit(
                &event,
                actor_entity,
                *disposition,
                &mut em,
                &mut combat,
                aggression.as_deref_mut(),
                interactable,
                &mut banner,
                combat_banter.as_deref(),
                feel,
                &mut writers,
            ) {
                actor_hit_this_event = true;
                landed_keys.push(format!("{prefix}:{}", id.as_str()));
                sync_actor_components_from_cluster(
                    &em,
                    *disposition,
                    &mut identity,
                    &mut combat,
                    &mut intent,
                    &mut cooldowns,
                );
            }
        }
        let mut boss_hit_this_event = false;
        // A pre-resolved actor-vs-actor hit never spills onto bosses / breakables.
        for (id, _aabb, mut feature, mut health, mut combat, attack_state, animation_frame) in
            bosses.iter_mut().filter(|_| actor_target.is_none())
        {
            if target_is_ignored(&event.ignored_targets, "boss", id.as_str()) {
                continue;
            }
            if apply_boss_hit(
                &event,
                feature.as_boss_mut(),
                &mut health,
                &mut combat,
                attack_state,
                animation_frame,
                &mut banner,
                combat_banter.as_deref(),
                &mut writers,
            ) {
                boss_hit_this_event = true;
                landed_keys.push(format!("boss:{}", id.as_str()));
            }
        }

        if actor_hit_this_event || boss_hit_this_event {
            let target_attacker = event.attacker.or_else(|| primary_q.single().ok());
            if let Some(attacker) = target_attacker {
                let record_dedup = matches!(event.source, HitSource::PlayerSlash { .. });
                for (entity, mut combat, active_attack) in &mut player_combat_q {
                    if entity != attacker {
                        continue;
                    }
                    combat.hitstop_timer = combat.hitstop_timer.max(0.06);
                    combat.hit_flash = combat.hit_flash.max(0.10);
                    // Record the targets this slash just struck so the next active
                    // frame's emit ignores them (one hit per target per swing).
                    if record_dedup {
                        if let Some(mut active) = active_attack {
                            if let Some(state) = active.swing.as_mut() {
                                state.hit_targets.extend(landed_keys.iter().cloned());
                            }
                        }
                    }
                    break;
                }
            }
            writers.sfx.write(SfxMessage::Hit {
                pos: event.volume.center(),
            });
        }

        for (entity, id, name, aabb, mut feature) in
            breakables.iter_mut().filter(|_| actor_target.is_none())
        {
            if target_is_ignored(&event.ignored_targets, "breakable", id.as_str()) {
                continue;
            }
            if feature.broken() || !feature.breakable.trigger.allows_hit() {
                continue;
            }
            if feature.breakable.pogo_refresh {
                continue;
            }
            if !event.volume.intersects_aabb(aabb.aabb()) {
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

mod actor_hit;
mod boss_hit;
use actor_hit::*;
use boss_hit::*;

#[cfg(test)]
mod tests;
