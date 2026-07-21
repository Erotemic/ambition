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
use ambition_characters::brain::{
    action_set::{ActionRequest, ProjectileFlight},
    ActorActionMessage,
};
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

/// Read every `ActorActionMessage::Ranged` and spawn the matching projectile.
/// Applies recoil to the firing body's velocity.
///
/// BODY-GENERIC. This used to demand the full actor cluster, so a body without an
/// `ActorConfig` — every home/player body — silently fell through and its ranged
/// move spawned nothing, which is why player shots needed their own path. The
/// query now names only what firing actually needs: kinematics, the body's melee
/// state (which owns the shared refire floor), its surface frame, and an OPTIONAL
/// archetype config for the per-archetype default look. Any body that emits
/// `ActionRequest::Ranged` now fires through this one consumer.
pub fn spawn_enemy_projectiles_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: SfxWriter,
    mut actors: Query<(
        &mut ae::BodyKinematics,
        &mut crate::actor::BodyMelee,
        Option<&super::ActorSurfaceState>,
        Option<&super::ActorConfig>,
        Option<&ambition_characters::actor::BodyHealth>,
    )>,
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
        } = msg.request.clone()
        else {
            continue;
        };
        let Ok((mut kin, mut melee, surface, config, health)) = actors.get_mut(msg.actor) else {
            // Message references a body that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        // Capability, not AI policy: the body fires because it OWNS a ranged
        // `ActionSet` slot (the upstream resolver only emits `Ranged` for a body
        // whose `ActionSet.ranged.is_some()`). A player possessing a peaceful NPC
        // fires its authored weapon; an autonomous peaceful NPC has no ranged
        // slot, so it emits nothing. Disposition (attack-or-not while autonomous)
        // is the BRAIN's business, not this effect consumer's.
        // A dead body fires nothing. `None` (a headless test body with no health
        // pool) is treated as alive, matching the shared hit resolver.
        if health.is_some_and(|h| !h.alive()) {
            continue;
        }
        // Body-side fire-rate enforcement (invariant I3): the controller attempts
        // a shot every time it emits `fire`; the body accepts it only when the
        // ranged weapon is off cooldown, re-arming on each accepted shot. A
        // blocked attempt simply spawns nothing this tick. This is the single
        // place the weapon rate is enforced, identical for an AI spam controller,
        // a tactical brain, and a possessing human.
        if !melee.try_fire_ranged(RANGED_REFIRE_S).accepted() {
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
        // The projectile's APPEARANCE is chosen by an OPEN visual id, set here at
        // the fire site: a gun-sword discharge is the spinning `"lasersword"`;
        // otherwise the archetype's authored ranged visual (e.g. the PCA's
        // `"glider"`), defaulting to the empty/generic hostile shot. The render
        // layer resolves this id through the content catalog — never the owner-id
        // string. The held-item id → projectile-visual id mapping is game policy;
        // when a second item needs its own discharge look this table can move to
        // a content-owned held-item→projectile registration.
        // Precedence: a held item's discharge, else the ACTION's own authored
        // visual (an equipment-granted verb brings its look with it), else the
        // archetype's default ranged look.
        let visual_id = if uses_gun_sword {
            "lasersword".to_string()
        } else if let Some(authored) = spec.visual.clone() {
            authored
        } else {
            config
                .map(|c| c.tuning.ranged_visual.clone())
                .unwrap_or_default()
        };
        // Flight is the ACTION's to author; the shared envelope is the fallback
        // for every ranged verb that doesn't care.
        let flight = spec.flight.unwrap_or(ProjectileFlight {
            gravity: 0.0,
            bounces: 0,
            bounce_on_world_contact: false,
            max_lifetime: PROJECTILE_MAX_LIFETIME,
            half_extent: PROJECTILE_HALF_EXTENT,
        });
        let gravity_dir = -surface
            .map(|s| s.surface_normal)
            .unwrap_or(ae::Vec2::new(0.0, -1.0))
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
        // Used for self / friendly-fire ignore lists and traces only; the
        // authoritative owner is the `ProjectileOwner` entity stamped at spawn, so
        // a body with no archetype row still owns its shot correctly.
        let owner_id = config.map(|c| c.id.clone()).unwrap_or_default();
        let spawn_origin = if uses_gun_sword {
            let hand = crate::features::rider_hand_world_pos_in_frame(
                kin.pos,
                kin.facing,
                kin.size.y,
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
            max_lifetime: flight.max_lifetime,
            half_extent: flight.half_extent,
            owner_id: owner_id.clone(),
            gravity: flight.gravity,
            visual_id,
            bounces: flight.bounces,
            bounce_on_world_contact: flight.bounce_on_world_contact,
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
        kin.vel += kick;
    }
}

// Melee is a moveset move for EVERY body — there is no actor-specific (or
// player-specific) melee driver. A body's melee capability (`ActionSet.melee`)
// is folded into a `"attack"`-verb move at spawn (`build_actor_moveset`); the
// brain's `melee_pressed` edge starts it via `combat::moveset::trigger_moveset_moves`
// and `advance_move_playback` spawns the active-window strike. The old
// `start_enemy_melee_from_brain_actions` / `ActorMut::begin_melee_attack` actor
// pair AND the flat `start_body_melee` / `advance_body_melee` are all deleted —
// one melee lifecycle.

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `SandboxFeelTuning` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> crate::features::events::FeatureCombatTuning {
    SandboxFeelTuning::default().feature_combat_tuning()
}

#[cfg(test)]
mod tests;
