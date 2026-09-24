//! Meteor: a player-wielded overhead area strike. It calls down a short
//! volley of falling player-faction projectiles onto a zone ahead of the
//! player. Other abilities strike forward or centered (gun_sword, shockwave,
//! beam, volley, vortex, dive); the meteor hits a zone from above, to clear a
//! patch of ground, zone a doorway, or hit a grounded mob from a distance.
//!
//! It is GNU-ton's signature (its phase-2 tell is a rain of apples): defeat it
//! and wield the rain.
//!
//! It uses the projectile pool the sentry and volley use (a
//! `ProjectileSpawnRequest` owned by the wielder). Each meteor spawns high
//! above the zone with a downward heading and gravity, so it is a readable
//! rain, not a hitscan. Player faction: it damages enemies and bosses and
//! spares the player.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};

/// Held-item id of the meteor gauntlet.
pub const METEOR_ID: &str = "meteor";

/// Mana per cast (out of 100): the most expensive wielded attack (a multi-hit
/// zone strike).
const METEOR_MANA_COST: f32 = 32.0;

/// How many meteors fall per cast.
const METEOR_COUNT: usize = 5;
/// How far ahead of the player (along the aim's horizontal) the strike zone centers.
const METEOR_RANGE: f32 = 190.0;
/// Horizontal width (px) the meteors are spread across.
const METEOR_SPREAD: f32 = 220.0;
/// How far above the player's level each meteor spawns (it falls from here).
const METEOR_DROP_HEIGHT: f32 = 270.0;
/// Initial downward speed (px/s); gravity accelerates it from there.
const METEOR_SPEED: f32 = 140.0;
/// Downward acceleration (px/s^2) — a fast, readable fall.
const METEOR_GRAVITY: f32 = 950.0;
/// Damage per meteor (the area comes from count and spread, not large hits).
const METEOR_DAMAGE: i32 = 2;
const METEOR_LIFETIME: f32 = 2.0;
const METEOR_HALF: ae::Vec2 = ae::Vec2::new(9.0, 9.0);

/// The spawn origins of one cast: `METEOR_COUNT` points spread evenly across
/// `METEOR_SPREAD`, centered `METEOR_RANGE` ahead along the aim's horizontal
/// (default `facing`), all `METEOR_DROP_HEIGHT` above the player so they fall
/// onto the zone. Pure, so the geometry is testable without the projectile
/// pool.
fn meteor_strike_origins(
    player_pos: ae::Vec2,
    aim_local: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> [ae::Vec2; METEOR_COUNT] {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let dir_x = if aim_local.x.abs() > 0.001 {
        aim_local.x.signum()
    } else {
        facing.signum()
    };
    let zone = player_pos + frame.to_world(ae::Vec2::new(dir_x * METEOR_RANGE, 0.0));
    let spawn_center = zone + frame.to_world(ae::Vec2::new(0.0, -METEOR_DROP_HEIGHT));
    let mut origins = [ae::Vec2::ZERO; METEOR_COUNT];
    for (i, slot) in origins.iter_mut().enumerate() {
        // Spread evenly across [-0.5, 0.5] * SPREAD along local side.
        let frac = (i as f32) / ((METEOR_COUNT - 1) as f32) - 0.5;
        *slot = spawn_center + frame.to_world(ae::Vec2::new(frac * METEOR_SPREAD, 0.0));
    }
    origins
}

/// `Attack` while holding the meteor gauntlet drops [`METEOR_COUNT`] falling
/// `Player`-faction projectiles onto the zone ahead. Plain Attack only;
/// `Shield + Attack` drops the item (the id is `UseSystem`).
pub fn fire_meteor_system(
    // Every driven body, not only the primary seat's `ControlledSubject`, so
    // a possessed body or a second seat can use it.
    driven: ambition_held_items::DrivenBodies,
    mut players: Query<(
        Entity,
        &ActorControl,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
        Option<&mut ambition_platformer2d_core::resources::ActorResources>,
    )>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for subject in driven.entities() {
        let Ok((entity, control, kin, resolved_frame, held, mut mana)) = players.get_mut(subject)
        else {
            continue;
        };
        let c = control.0;
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        if held.spec.id != METEOR_ID {
            continue;
        }
        if !crate::mana::spend(mana.as_deref_mut(), METEOR_MANA_COST) {
            continue;
        }
        // The body's per-tick resolved frame (ADR 0024 frame law).
        let gravity_dir = resolved_frame.down();
        let aim = ambition_held_items::ability_aim_local(&c, kin.facing);
        for origin in meteor_strike_origins(kin.pos, aim, kin.facing, gravity_dir) {
            projectiles.write(ProjectileSpawnRequest::open(
                // The firing actor owns every meteor, so a kill is credited to
                // it (materialization stamps `ProjectileOwner` from this).
                entity,
                ProjectileSpawn {
                    origin,
                    // Toward local down; gravity accelerates it the same way.
                    dir: gravity_dir,
                    speed: METEOR_SPEED,
                    damage: METEOR_DAMAGE,
                    max_lifetime: METEOR_LIFETIME,
                    half_extent: METEOR_HALF,
                    gravity: METEOR_GRAVITY,
                    visual_id: String::new(),
                    // Straight volley: this ability authors no bounce.
                    bounces: 0,
                    bounce_on_world_contact: false,
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
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
}

#[cfg(test)]
mod tests;
