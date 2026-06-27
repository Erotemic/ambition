//! Hitbox-entity lifecycle: spawn → overlap-check → despawn.
//!
//! Per the actor/brain follow-up plan
//! (`dev/journals/actor-brain-migration-followups-plan.md`, Task A):
//! enemy melee strikes were resolved by per-tick polling inside
//! `update_ecs_actors` (calling `enemy.player_damage(player_body)`
//! every frame the attack_timer was hot). That bypass made melee
//! the only attack family that didn't flow through the actor/brain
//! → ActorActionMessage → EFFECTS-consumer seam.
//!
//! This module replaces the poll with explicit entities:
//!
//! - `update_ecs_actors` detects the windup → active edge and
//!   spawns one `(Hitbox, HitboxLifetime, HitboxHits)` entity per
//!   strike using the strike's per-archetype AABB.
//! - `apply_hitbox_damage` (this module) tests overlap against the
//!   target faction's hurtboxes each tick, emits the matching
//!   damage event, and inserts hit targets into `HitboxHits` so a
//!   long active window can't double-hit the same target.
//! - `tick_and_despawn_hitboxes` (this module) advances every
//!   hitbox's lifetime and despawns expired ones.
//!
//! `HitboxAnchor::FollowOwner` re-resolves the hitbox AABB each
//! tick from the owner entity's position, so a moving attacker's
//! swing tracks the actor without a per-frame component update.
//! `HitboxAnchor::World` (Task B groundwork) is a fixed
//! world-space rectangle for hazards / boss specials.

use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, With};

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

use super::components::ActorFaction;
use super::targeting::can_damage;
use super::events::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
use super::util::midpoint;
use crate::audio::SfxMessage;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::WorldTime;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

// The hitbox COMPONENTS moved to the reusable `ambition_vfx` crate (the
// damage-box primitive). Re-exported here so `combat::hitbox::Hitbox`
// (and `features::Hitbox`) paths are unchanged; the SYSTEMS below (damage
// resolution, melee spawn, lifecycle) stay in the lib.
pub use ambition_vfx::{Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime};

/// Apply each live hitbox's damage to the right faction's targets.
///
/// Enemy / Boss hitboxes hit the player and emit `HitEvent` with a
/// victim-side `HitSource`. Player / Npc / Neutral hitboxes are
/// routed through other paths (player slash flows as
/// `HitSource::PlayerSlash`); this system is the catch-all for
/// hostile melee.
pub fn apply_hitbox_damage(
    mut hitboxes: Query<(Entity, &Hitbox, &mut HitboxHits)>,
    owners: Query<&super::components::CenteredAabb>,
    // Friendly-fire policy (the DAMAGE side; targeting is `FactionRelations`).
    // Optional so minimal headless tests that don't stand up the plugin still run
    // (fall back to the default: friendly fire OFF — same-faction allies safe).
    friendly_fire: Option<Res<crate::features::FriendlyFire>>,
    // Non-player actor victims for the actor-vs-actor melee path: an Enemy/Boss
    // swing damages any DIFFERENT-faction actor it overlaps (e.g. a Boss vs an
    // Enemy in a duel); same-faction allies are spared unless friendly fire is on.
    actor_victims: Query<(Entity, &super::components::CenteredAabb, &ActorFaction)>,
    // Iterate every player so a multi-player build hits each
    // overlapping player independently. Single-player behavior is
    // preserved because the iterator has exactly one entity today.
    player_query: Query<
        (
            Entity,
            &crate::actor::BodyKinematics,
            &crate::actor::BodyOffense,
            &crate::actor::BodyDodgeState,
            &crate::actor::BodyShieldState,
            &crate::actor::BodyCombat,
        ),
        bevy::prelude::With<crate::actor::PlayerEntity>,
    >,
    // Orient the player's hurtbox to its (zone-aware) gravity frame — the same
    // box the debug overlay draws and enemies/bosses resolve through
    // `collision_aabb`. Identity under vertical gravity.
    gravity: crate::physics::GravityCtx,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
) {
    let friendly_fire = friendly_fire.map(|r| *r).unwrap_or_default();
    for (_hitbox_entity, hitbox, mut hits) in &mut hitboxes {
        let owner_pos = match owners.get(hitbox.owner) {
            Ok(aabb) => aabb.center,
            // Owner despawned this frame — leave the hitbox as a
            // ghost; `tick_and_despawn_hitboxes` will clean it up
            // when its lifetime expires. Don't apply damage from
            // an owner-less hitbox; the source position can't be
            // resolved sensibly.
            Err(_) => continue,
        };
        let world_volume = hitbox.world_volume(owner_pos);

        match hitbox.source {
            ActorFaction::Enemy | ActorFaction::Boss => {
                let source_kind = if matches!(hitbox.source, ActorFaction::Boss) {
                    HitSource::BossAttack
                } else {
                    HitSource::EnemyAttack
                };
                // Actor-vs-actor: an Enemy/Boss swing damages any DIFFERENT-faction
                // actor it overlaps (e.g. a Boss vs an Enemy in a duel). Same-faction
                // allies are spared unless friendly fire is on; the attacker never
                // hits itself (owner check). Stamped `HitTarget::Actor` so the
                // actor-damage consumer applies it to exactly that body.
                for (victim_entity, victim_aabb, victim_faction) in &actor_victims {
                    if victim_entity == hitbox.owner {
                        continue;
                    }
                    if !can_damage(hitbox.source, *victim_faction, friendly_fire) {
                        continue;
                    }
                    if hits.hit.contains(&victim_entity) {
                        continue;
                    }
                    if !world_volume.intersects_aabb(victim_aabb.aabb()) {
                        continue;
                    }
                    let impact = midpoint(victim_aabb.center, world_volume.center());
                    vfx.write(VfxMessage::Impact { pos: impact });
                    hit_events.write(HitEvent {
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: source_kind.clone(),
                        attacker: Some(hitbox.owner),
                        target: HitTarget::Actor(victim_entity),
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                    hits.hit.insert(victim_entity);
                }
                // Damage is physical: an Enemy/Boss swing that overlaps the player
                // hits them — even a neutral observer caught in a duel's crossfire
                // (Player is a different faction, so not an ally). Targeting is
                // separate (`FactionRelations` decides whether they AIM at the
                // player); this is just "the hit landed". Same-faction would be
                // spared (co-op), unless friendly fire is on.
                if !can_damage(hitbox.source, ActorFaction::Player, friendly_fire) {
                    continue;
                }
                // Iterate every player and emit one HitEvent per
                // overlapping vulnerable player. `HitboxHits`
                // tracks which players this hitbox has already
                // damaged so a long active window doesn't double-
                // tap a stationary player.
                for (player_entity, kin, offense, dodge, shield, combat) in &player_query {
                    // Player hurtbox via the one shared combat-geometry path,
                    // oriented to its gravity frame (matches the gizmo + the
                    // box enemies/bosses resolve through `collision_aabb`).
                    let down = gravity.dir_at(kin.pos);
                    let player_body =
                        crate::features::collision_aabb(&crate::features::SimpleActorGeometry {
                            pos: kin.pos,
                            size: kin.size,
                            facing: kin.facing,
                            frame_down: down,
                        });
                    let dodge_rolling = dodge.roll_timer > 0.0;
                    let player_vulnerable = !offense.invincible
                        && !dodge_rolling
                        && !shield.parrying()
                        && combat.vulnerable();
                    if !player_vulnerable {
                        continue;
                    }
                    if hits.hit.contains(&player_entity) {
                        continue;
                    }
                    if !world_volume.intersects_aabb(player_body) {
                        continue;
                    }
                    let impact = midpoint(player_body.center(), world_volume.center());
                    let knockback_dir = if player_body.center().x >= owner_pos.x {
                        1.0
                    } else {
                        -1.0
                    };
                    sfx.write(SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_DAMAGE,
                        pos: impact,
                    });
                    vfx.write(VfxMessage::Impact { pos: impact });
                    vfx.write(VfxMessage::Burst {
                        pos: impact,
                        count: 14,
                        speed: 300.0,
                        color: [1.0, 0.34, 0.28, 0.88],
                        kind: ParticleKind::Shard,
                    });
                    debris.write(DebrisBurstMessage {
                        pos: impact,
                        cue: PhysicsDebrisCue::Impact,
                    });
                    hit_events.write(HitEvent {
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: source_kind.clone(),
                        // Enemy / boss hitboxes know their owner — the
                        // entity that spawned the hitbox is the
                        // attacker. Read on the player side to
                        // attribute hitstun to the right attacker.
                        attacker: Some(hitbox.owner),
                        // Stamp the victim so the player-damage
                        // reader doesn't fall back to primary.
                        target: HitTarget::Player(player_entity),
                        mode: HitMode::Knockback,
                        knockback: Some(HitKnockback {
                            dir: knockback_dir,
                            strength: hitbox.knockback_strength.max(0.0),
                            source_pos: owner_pos,
                            impact_pos: impact,
                        }),
                        ignored_targets: Vec::new(),
                    });
                    hits.hit.insert(player_entity);
                }
            }
            // Player-faction hitbox (a wielded boss-style AOE — see
            // `crate::abilities::ranged::shockwave`): damage the enemies/bosses it overlaps by
            // emitting ONE attacker-side Volume `HitEvent` that
            // `apply_feature_hit_events` resolves against every overlapping
            // actor + boss. This is the player end of the same primitive a boss
            // AOE uses through the Enemy/Boss branch above — the faction is the
            // only difference. Fires once per strike (the owner doubles as a
            // "already fired" sentinel in `HitboxHits`, harmless since a hitbox
            // never targets its own owner).
            ActorFaction::Player => {
                if hits.hit.insert(hitbox.owner) {
                    vfx.write(VfxMessage::Impact {
                        pos: world_volume.center(),
                    });
                    hit_events.write(HitEvent {
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: HitSource::PlayerSlash { knock_x: 0.0 },
                        attacker: Some(hitbox.owner),
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                }
            }
            // Peaceful NPC / neutral factions don't spawn damaging hitboxes.
            ActorFaction::Npc | ActorFaction::Neutral => {}
        }
    }
}

/// Advance every hitbox's lifetime by `world_time.sim_dt()` and
/// despawn the ones that hit zero. Sim-clock so bullet-time freezes
/// in-flight hitboxes alongside the rest of combat (ADR 0010).
pub fn tick_and_despawn_hitboxes(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    mut hitboxes: Query<(Entity, &mut HitboxLifetime), With<Hitbox>>,
) {
    let dt = world_time.sim_dt();
    for (entity, mut lifetime) in &mut hitboxes {
        lifetime.remaining_s -= dt;
        if lifetime.remaining_s <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn helper: emit a fresh hitbox entity for a melee strike. The
/// caller picks the local offset / half-extent / damage / faction
/// based on the strike's archetype + facing.
pub fn spawn_melee_hitbox(
    commands: &mut Commands,
    owner: Entity,
    source: ActorFaction,
    local_offset: ae::Vec2,
    half_extent: ae::Vec2,
    damage: i32,
    knockback_strength: f32,
    active_s: f32,
) -> Entity {
    commands
        .spawn((
            Hitbox {
                owner,
                source,
                anchor: HitboxAnchor::FollowOwner { local_offset },
                half_extent,
                shape: None,
                facing: 1.0,
                damage,
                knockback_strength,
            },
            HitboxLifetime {
                remaining_s: active_s.max(0.0),
            },
            HitboxHits::default(),
        ))
        .id()
}

#[cfg(test)]
mod tests;
