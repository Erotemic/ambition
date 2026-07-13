//! Meteor — a player-wielded **overhead area-strike**: call down a short volley
//! of falling player-faction projectiles onto a zone ahead of the player. It
//! fills a real gap in the wielded kit — every other ability strikes *forward*
//! or *centered* (gun_sword, shockwave, beam, volley, vortex, dive); the meteor
//! is the only one that hits a **zone from above**, so it answers a different
//! question: not "what's in front of me" but "clear that patch of ground over
//! there" (chip a cluster, zone a doorway, rain on a grounded mob you don't want
//! to walk into).
//!
//! It is **GNU-ton's** signature gauntlet — the giant whose phase-2 tell is a
//! rain of apples from its descending head. Defeat it, wield its apple-rain
//! yourself ("every boss a failed objective function, learn its attack").
//!
//! Mechanically it reuses the faction-aware projectile pool the sentry/volley
//! use (`EnemyProjectileState::spawn_with_faction(..., Player)`), spawning each
//! meteor high above the strike zone with a downward heading + gravity so it
//! accelerates into the ground — a readable rain, not a hitscan. Player faction,
//! so the meteors damage enemies/bosses and spare the player.

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;
use ambition_platformer_primitives::markers::ControlledSubject;

/// Held-item id of the meteor gauntlet.
pub const METEOR_ID: &str = "meteor";

/// Mana the meteor spends per cast (out of 100) — the priciest wielded attack
/// (a multi-hit zone strike), so it's gated hardest.
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
/// Damage per meteor (the AOE comes from the count + spread, not big single hits).
const METEOR_DAMAGE: i32 = 2;
const METEOR_LIFETIME: f32 = 2.0;
const METEOR_HALF: ae::Vec2 = ae::Vec2::new(9.0, 9.0);

/// Resolve the spawn origins of one cast: `METEOR_COUNT` points spread evenly
/// across `METEOR_SPREAD`, centered `METEOR_RANGE` ahead of the player along the
/// aim's horizontal (defaulting to `facing`), all `METEOR_DROP_HEIGHT` above the
/// player's level so they fall *down* onto the zone. Pure so the geometry is
/// unit-testable without the projectile pool.
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

/// `Attack` while holding the meteor gauntlet rains [`METEOR_COUNT`] falling
/// `Player`-faction projectiles onto the zone ahead. Plain Attack only — `Shield
/// + Attack` drops the item (the id is `UseSystem`).
pub fn fire_meteor_system(
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        &BodyKinematics,
        &crate::physics::ResolvedMotionFrame,
        &HeldItem,
        &mut BodyMana,
    )>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: ambition_sfx::SfxWriter,
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
    if held.spec.id != METEOR_ID {
        return;
    }
    if !mana.meter.try_spend(METEOR_MANA_COST) {
        return;
    }
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let gravity_dir = resolved_frame.down();
    let aim = crate::items::pickup::ability_aim_local(&c, kin.facing);
    for origin in meteor_strike_origins(kin.pos, aim, kin.facing, gravity_dir) {
        effects.write(ambition_vfx::EffectRequest {
            // The firing actor owns every meteor, so a kill attributes back to
            // the player (the executor stamps `ProjectileOwner` from this entity).
            owner: entity,
            effect: ambition_vfx::Effect::Projectiles {
                shots: vec![EnemyProjectileSpawn {
                    origin,
                    // Straight toward local feet/down; gravity accelerates it in the same frame.
                    dir: gravity_dir,
                    speed: METEOR_SPEED,
                    damage: METEOR_DAMAGE,
                    max_lifetime: METEOR_LIFETIME,
                    half_extent: METEOR_HALF,
                    owner_id: "player_meteor".into(),
                    gravity: METEOR_GRAVITY,
                    visual_tag: 0,
                }],
            },
        });
    }
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

#[cfg(test)]
mod tests;
