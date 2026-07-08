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

use ambition_platformer_primitives::markers::ControlledSubject;
use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;

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
    gravity: crate::physics::GravityCtx,
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        &BodyKinematics,
        &HeldItem,
        &mut BodyMana,
    )>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((entity, control, kin, held, mut mana)) = players.get_mut(subject) else {
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
    let gravity_dir = gravity.dir_at(kin.pos);
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
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;
    use crate::enemy_projectile::test_support::enemy_projectile_bodies;
    use crate::enemy_projectile::EnemyProjectileState;
    use crate::projectile::ProjectileSeqCounter;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::SfxMessage>();
        app.add_message::<ambition_vfx::EffectRequest>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // fire emits Effect::Projectiles; apply_projectile_effects spawns the entity.
        app.add_systems(
            Update,
            (
                fire_meteor_system,
                crate::enemy_projectile::apply_projectile_effects,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn attack_rains_player_faction_meteors() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, METEOR_ID);
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        let bodies = enemy_projectile_bodies(&mut app);
        assert_eq!(
            bodies.len(),
            METEOR_COUNT,
            "one volley = METEOR_COUNT meteors"
        );
    }

    #[test]
    fn no_meteor_without_attack_or_item() {
        let mut app = test_app();
        spawn_primary_player_holding(&mut app, METEOR_ID);
        app.update(); // no attack pressed
        assert!(enemy_projectile_bodies(&mut app).is_empty());
    }

    #[test]
    fn meteor_costs_mana_and_is_blocked_when_empty() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, METEOR_ID);
        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .current = 5.0;
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        assert!(
            enemy_projectile_bodies(&mut app).is_empty(),
            "no meteors when mana < cost"
        );
        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .current = 100.0;
        app.update();
        assert_eq!(
            enemy_projectile_bodies(&mut app).len(),
            METEOR_COUNT,
            "fires once there's mana"
        );
    }

    #[test]
    fn meteors_spawn_above_the_player_and_spread_horizontally() {
        let player_pos = ae::Vec2::new(100.0, 100.0);
        let origins = meteor_strike_origins(
            player_pos,
            ae::Vec2::new(1.0, 0.0),
            1.0,
            ae::Vec2::new(0.0, 1.0),
        );
        // All spawn above the player (smaller y, engine y-down).
        assert!(
            origins.iter().all(|o| o.y < player_pos.y),
            "meteors spawn above the player to fall down: {origins:?}"
        );
        // The zone centers ahead of the player (+x for a rightward aim).
        let mean_x = origins.iter().map(|o| o.x).sum::<f32>() / METEOR_COUNT as f32;
        assert!(mean_x > player_pos.x, "strike zone is ahead of the player");
        // They spread horizontally (not a single column).
        let min_x = origins.iter().map(|o| o.x).fold(f32::INFINITY, f32::min);
        let max_x = origins
            .iter()
            .map(|o| o.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            (max_x - min_x) > 100.0,
            "meteors are spread across a band: {min_x}..{max_x}"
        );
    }

    #[test]
    fn meteor_origins_are_frame_equivalent() {
        let player_pos = ae::Vec2::new(100.0, 100.0);
        let local_aim = ae::Vec2::new(1.0, 0.0);
        let down = meteor_strike_origins(player_pos, local_aim, 1.0, ae::Vec2::new(0.0, 1.0));
        for gravity_dir in [
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let rotated = meteor_strike_origins(player_pos, local_aim, 1.0, gravity_dir);
            for (reference, actual) in down.iter().zip(rotated.iter()) {
                let expected_local = ae::AccelerationFrame::new(ae::Vec2::new(0.0, 1.0))
                    .to_local(*reference - player_pos);
                let actual_local = frame.to_local(*actual - player_pos);
                assert!((expected_local - actual_local).length() < 1e-3);
            }
        }
    }

    #[test]
    fn meteor_aims_with_the_left_stick_facing_on_a_null_aim() {
        // Aiming left (negative facing, no directional hold) puts the zone to the left.
        let left = meteor_strike_origins(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::ZERO,
            -1.0,
            ae::Vec2::new(0.0, 1.0),
        );
        let mean_x = left.iter().map(|o| o.x).sum::<f32>() / METEOR_COUNT as f32;
        assert!(
            mean_x < 100.0,
            "a left-facing null-aim cast strikes to the left"
        );
    }
}
