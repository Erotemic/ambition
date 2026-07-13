//! EFFECTS-stage consumers for `ActorActionMessage`.
//!
//! Hitboxes, projectiles, SFX, VFX, and recoil are driven from resolved
//! action messages rather than from per-actor integration loops.
//!
//! This module owns the consumer Bevy systems that read
//! `MessageReader<ActorActionMessage>` and produce effects. Each
//! system is one variant of `ActionRequest`; the upstream
//! `emit_brain_action_messages` resolver translates the actor's
//! `ActorControl` frame + `ActionSet` into the per-request stream
//! these systems consume.
//!
//! Schedule:
//! - `emit_brain_action_messages` runs first
//! - these systems run after, reading the same message stream
//! - the `BrainActionCounter` observer is unaffected (it counts but
//!   doesn't consume)

use ambition_engine_core as ae;
use bevy::prelude::*;

use crate::enemy_projectile::EnemyProjectileSpawn;
#[cfg(test)]
use crate::time::feel::SandboxFeelTuning;
use ambition_characters::brain::{action_set::ActionRequest, ActorActionMessage};
use ambition_sfx::{SfxMessage, SfxWriter};

/// Recoil applied to the firing enemy along the negative fire
/// direction. Per-archetype because PirateOnShark visibly knocks
/// back the rider+shark combo.
const RANGED_RECOIL_PIRATE: f32 = 380.0;
const RANGED_RECOIL_DEFAULT: f32 = 60.0;

/// Projectile envelope shared by every ranged enemy. Future
/// per-archetype overrides (slower arrows, gravity-arc rocks)
/// will move this into an `ActionSet`-derived parameter.
const PROJECTILE_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(10.0, 8.0);
const PROJECTILE_MAX_LIFETIME: f32 = 2.4;

/// Body-side ranged refire interval (s) — the floor on every ranged-capable
/// body's fire rate (invariant I3). The controller (AI brain, possessing human,
/// or future RL policy) may attempt `fire` every tick; the body accepts a shot
/// at most once per this interval. This was previously a *brain*-side cadence
/// (`SmashState::ranged_cooldown_remaining`), which leaked the physical limit
/// into the controller — a human could spam past it. It now lives on the body.
/// Per-archetype tempos will move this onto an `ActionSet`-derived parameter,
/// like the projectile envelope above.
const RANGED_REFIRE_S: f32 = 1.1;

/// How long the actor's post-fire Shoot overlay pose holds — matches the player's
/// `SHOOT_ANIM_HOLD_SECS` (`projectile::systems`) so a possessed body and an
/// autonomous one pulse the same, short enough that rapid fire stutters
/// Shoot↔locomotion rather than locking the read (§A9 follow-up).
const SHOOT_ANIM_HOLD_SECS: f32 = 0.18;

/// Read every `ActorActionMessage::Ranged` and spawn the matching
/// enemy projectile. Applies recoil to the firing actor's velocity.
///
/// Only handles **hostile** actors today — player projectiles still
/// flow through the legacy `update_player` path. Player migration is
/// the next slice in the mandate.
pub fn spawn_enemy_projectiles_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: SfxWriter,
    mut actors: Query<Option<super::actor_clusters::ActorClusterQueryData>>,
    // Disjoint from `actors` — `ActorClusterQueryData` carries no `BodyAnimFacts`,
    // so this second view borrows the firing body's overlay-pose facts without
    // aliasing. Arms the Shoot pose on the frame the body accepts a shot.
    mut anim_facts: Query<&mut crate::actor::BodyAnimFacts>,
    held_items: Query<&super::HeldItem>,
) {
    for msg in messages.read() {
        let ActionRequest::Ranged {
            spec,
            origin,
            dir,
            dir_policy,
        } = msg.request
        else {
            continue;
        };
        let Ok(clusters) = actors.get_mut(msg.actor) else {
            // Message references an actor that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        // Capability, not AI policy: the actor fires because it OWNS a ranged
        // `ActionSet` slot (the upstream resolver only emits `Ranged` for a body
        // whose `ActionSet.ranged.is_some()`). A player possessing a peaceful NPC
        // fires its authored weapon; an autonomous peaceful NPC has no ranged
        // slot, so it emits nothing. Disposition (attack-or-not while autonomous)
        // is the BRAIN's business, not this effect consumer's.
        let Some(mut cq) = clusters else {
            continue;
        };
        let enemy = cq.as_actor_mut();
        if !enemy.health.alive() {
            continue;
        }
        // Body-side fire-rate enforcement (invariant I3): the controller attempts
        // a shot every time it emits `fire`; the body accepts it only when the
        // ranged weapon is off cooldown, re-arming on each accepted shot. A
        // blocked attempt simply spawns nothing this tick. This is the single
        // place the weapon rate is enforced, identical for an AI spam controller,
        // a tactical brain, and a possessing human.
        if !enemy.attack.try_fire_ranged(RANGED_REFIRE_S).accepted() {
            continue;
        }
        // The shot is committed — arm the firing body's Shoot overlay pose (the
        // actor analogue of the player's post-fire pulse in `projectile::systems`).
        // The pick reads `shoot_anim_timer`; the pose shows for whatever body owns
        // a Shoot row, autonomous or possessed (§A9 follow-up).
        if let Ok(mut anim) = anim_facts.get_mut(msg.actor) {
            anim.shoot_anim_timer = SHOOT_ANIM_HOLD_SECS;
        }
        // Held-item muzzle: a gun-sword shot should originate at the actor's
        // hand whether the pirate is still mounted or has fallen off the shark.
        // Future items can extend this routing by id without changing the brain.
        let held_item_id = held_items.get(msg.actor).ok().map(|item| item.id());
        let uses_gun_sword = held_item_id == Some("gun_sword");
        // The projectile's APPEARANCE is chosen by KIND, set here at the fire
        // site: a gun-sword discharge is a spinning lasersword; otherwise the
        // archetype's authored ranged visual (e.g. the PCA's Conway glider),
        // defaulting to the generic hostile shot. The render layer reads this
        // kind — never the owner-id string.
        let visual_kind = if uses_gun_sword {
            crate::projectile::ProjectileVisualKind::Lasersword
        } else {
            enemy.config.tuning.ranged_visual
        };
        let gravity_dir = -enemy
            .surface
            .surface_normal
            .normalize_or(ae::Vec2::new(0.0, -1.0));
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let request = ambition_characters::actor::control::ActorFireRequest {
            dir,
            dir_policy,
            speed: spec.speed(),
        };
        let world_dir = request.dir_to_world(frame).normalize_or_zero();
        // owner_id is the firing actor's id ONLY — used for self / friendly-fire
        // filtering and traces. It no longer encodes the projectile's look
        // (that's `visual_kind`), so a gun-sword shot carries the plain actor id
        // while still originating at the hand muzzle.
        let owner_id = enemy.config.id.clone();
        let spawn_origin = if uses_gun_sword {
            let hand = crate::features::rider_hand_world_pos_in_frame(
                enemy.kin.pos,
                enemy.kin.facing,
                enemy.kin.size.y,
                gravity_dir,
            );
            hand + world_dir * 18.0
        } else {
            origin + frame.to_world(ae::Vec2::new(0.0, -8.0))
        };
        let spawn = EnemyProjectileSpawn {
            origin: spawn_origin,
            dir: world_dir,
            speed: spec.speed(),
            damage: spec.damage(),
            max_lifetime: PROJECTILE_MAX_LIFETIME,
            half_extent: PROJECTILE_HALF_EXTENT,
            owner_id: owner_id.clone(),
            gravity: 0.0,
            visual_tag: visual_kind.to_tag(),
        };
        if uses_gun_sword {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::from_static("weapon.lasersword.fire"),
                pos: spawn.origin,
            });
        }
        effects.write(ambition_vfx::EffectRequest {
            owner: msg.actor,
            effect: ambition_vfx::Effect::Projectiles { shots: vec![spawn] },
        });
        // Recoil: push the firing actor backward along the negative
        // fire direction.
        let recoil_strength = if uses_gun_sword {
            RANGED_RECOIL_PIRATE
        } else {
            RANGED_RECOIL_DEFAULT
        };
        let kick = world_dir * -recoil_strength;
        enemy.kin.vel += kick;
    }
}

// Melee START is no longer an actor-specific consumer. `ActorActionMessage::Melee`
// is turned into a swing by the body-generic `combat::attack::start_body_melee`
// phase (which runs for EVERY body — player, possessed actor, autonomous hostile),
// and the active-edge strike is spawned by `combat::attack::advance_body_melee`.
// The old `start_enemy_melee_from_brain_actions` / `ActorMut::begin_melee_attack`
// actor-only pair is deleted — one melee lifecycle, not a player driver plus an
// actor driver.

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `SandboxFeelTuning` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> crate::features::events::FeatureCombatTuning {
    SandboxFeelTuning::default().feature_combat_tuning()
}

#[cfg(test)]
mod tests;
