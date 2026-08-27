//! Volley — a player-wielded ranged boss attack: a fan of bolts that damage
//! enemies, fired through the shared projectile request/materialization road.
//!
//! This is the ranged counterpart to `crate::abilities::ranged::shockwave` (the wielded AOE). Now
//! damage routes off the FIRER's real `ActorFaction` (looked up from the projectile's owner
//! entity): a player-owned shot damages enemies/bosses and expires on contact, an enemy-owned shot
//! still hits the player. Same pool, same step system — the projectile analog of the shockwave's
//! faction-tagged `Hitbox`.

use bevy::prelude::*;

use crate::features::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::BodyMana;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};

/// Held-item id of the volley gauntlet.
pub const VOLLEY_ID: &str = "volley";

/// Mana the volley spends per fan (out of 100). Cheaper than the shockwave slam.
const VOLLEY_MANA_COST: f32 = 18.0;

/// Bolts per volley.
const VOLLEY_SHOT_COUNT: usize = 5;
/// Total fan spread (degrees), centered on the aim direction.
const VOLLEY_SPREAD_DEG: f32 = 40.0;
const VOLLEY_SPEED: f32 = 460.0;
const VOLLEY_DAMAGE: i32 = 2;
const VOLLEY_LIFETIME: f32 = 1.6;
const VOLLEY_HALF: ae::Vec2 = ae::Vec2::new(8.0, 8.0);

fn volley_origin_local_offset(aim_local: ae::Vec2, body_size: ae::Vec2) -> ae::Vec2 {
    let dir = aim_local.normalize_or_zero();
    if dir == ae::Vec2::ZERO {
        return ae::Vec2::ZERO;
    }
    let half = body_size * 0.5;
    let body_extent_along_aim = half.x * dir.x.abs() + half.y * dir.y.abs();
    dir * (body_extent_along_aim + 8.0)
}

fn volley_origin_world(
    player_pos: ae::Vec2,
    body_size: ae::Vec2,
    aim_local: ae::Vec2,
    frame: ae::AccelerationFrame,
) -> ae::Vec2 {
    player_pos + frame.to_world(volley_origin_local_offset(aim_local, body_size))
}

/// `Attack` while holding the volley gauntlet fires a fan of player-faction
/// bolts along the body-semantic aim direction (`ActorControl` aim / locomotion /
/// facing). Plain Attack only — `Shield + Attack` drops the item
/// (the id is excluded from throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_volley_system(
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
        &mut BodyMana,
    )>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((entity, control, kin, resolved_frame, held, mut mana)) = players.get_mut(subject)
    else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != VOLLEY_ID {
        return;
    }
    // Costs mana — out of mana, no volley.
    if !mana.meter.try_spend(VOLLEY_MANA_COST) {
        return;
    }
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let frame = resolved_frame.basis();
    let aim_local = crate::items::pickup::ability_aim_local(&c, kin.facing);
    let aim = frame.to_world(aim_local).normalize_or_zero();
    if aim == ae::Vec2::ZERO {
        return;
    }
    let base_angle = aim.y.atan2(aim.x);
    let origin = volley_origin_world(kin.pos, kin.size, aim_local, frame);
    let spread = VOLLEY_SPREAD_DEG.to_radians();
    for i in 0..VOLLEY_SHOT_COUNT {
        // Centered fan: t in [-0.5, 0.5].
        let t = if VOLLEY_SHOT_COUNT > 1 {
            i as f32 / (VOLLEY_SHOT_COUNT - 1) as f32 - 0.5
        } else {
            0.0
        };
        let angle = base_angle + t * spread;
        let dir = ae::Vec2::new(angle.cos(), angle.sin());
        projectiles.write(ProjectileSpawnRequest::open(
            // The firing actor owns every bolt, so a kill attributes back to the
            // player (materialization stamps `ProjectileOwner` from this entity).
            entity,
            ProjectileSpawn {
                origin,
                dir,
                speed: VOLLEY_SPEED,
                damage: VOLLEY_DAMAGE,
                max_lifetime: VOLLEY_LIFETIME,
                half_extent: VOLLEY_HALF,
                gravity: 0.0,
                visual_id: String::new(),
                // Straight volley: this ability authors no bounce.
                bounces: 0,
                bounce_on_world_contact: false,
            },
            ProjectileStart::StepThisTick,
        ));
    }
    sfx.write_for(
        entity,
        ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: kin.pos,
        },
    );
}

#[cfg(test)]
mod tests;
