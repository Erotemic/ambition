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

use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerMana, PrimaryPlayer};
use crate::projectile::ProjectileFaction;

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
    aim: ae::Vec2,
    facing: f32,
) -> [ae::Vec2; METEOR_COUNT] {
    let dir_x = if aim.x.abs() > 0.001 {
        aim.x.signum()
    } else {
        facing.signum()
    };
    let zone_x = player_pos.x + dir_x * METEOR_RANGE;
    // Engine y grows downward, so "above" is a *smaller* y.
    let spawn_y = player_pos.y - METEOR_DROP_HEIGHT;
    let mut origins = [ae::Vec2::ZERO; METEOR_COUNT];
    for (i, slot) in origins.iter_mut().enumerate() {
        // Spread evenly across [-0.5, 0.5] * SPREAD.
        let frac = (i as f32) / ((METEOR_COUNT - 1) as f32) - 0.5;
        *slot = ae::Vec2::new(zone_x + frac * METEOR_SPREAD, spawn_y);
    }
    origins
}

/// `Attack` while holding the meteor gauntlet rains [`METEOR_COUNT`] falling
/// `Player`-faction projectiles onto the zone ahead. Plain Attack only — `Shield
/// + Attack` drops the item (the id is `UseSystem`).
pub fn fire_meteor_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (&BodyKinematics, &HeldItem, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, held, mut mana)) = players.single_mut() else {
        return;
    };
    if held.spec.id != METEOR_ID {
        return;
    }
    if !mana.meter.try_spend(METEOR_MANA_COST) {
        return;
    }
    let aim = crate::items::pickup::held_shot_aim(&control, kin.facing);
    for origin in meteor_strike_origins(kin.pos, aim, kin.facing) {
        effects.write(crate::effects::EffectRequest {
            // Projectiles are self-describing (owner_id is on the shot); the
            // EffectRequest owner is unused by the projectile executor.
            owner: Entity::PLACEHOLDER,
            effect: crate::effects::Effect::Projectiles {
                faction: ProjectileFaction::Player,
                shots: vec![EnemyProjectileSpawn {
                    origin,
                    // Straight down (engine y grows downward); gravity accelerates it.
                    dir: ae::Vec2::new(0.0, 1.0),
                    speed: METEOR_SPEED,
                    damage: METEOR_DAMAGE,
                    max_lifetime: METEOR_LIFETIME,
                    half_extent: METEOR_HALF,
                    owner_id: "player_meteor".into(),
                    gravity: METEOR_GRAVITY,
                }],
            },
        });
    }
    sfx.write(crate::audio::SfxMessage::Play {
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
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.insert_resource(ControlFrame::default());
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
        spawn_primary_player_holding(&mut app, METEOR_ID);
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        let bodies = enemy_projectile_bodies(&mut app);
        assert_eq!(
            bodies.len(),
            METEOR_COUNT,
            "one volley = METEOR_COUNT meteors"
        );
        assert!(
            bodies
                .iter()
                .all(|b| b.body.game.faction == ProjectileFaction::Player),
            "meteors are player-faction (damage enemies, spare the player)"
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
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .current = 5.0;
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        assert!(
            enemy_projectile_bodies(&mut app).is_empty(),
            "no meteors when mana < cost"
        );
        app.world_mut()
            .get_mut::<PlayerMana>(player)
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
        let origins = meteor_strike_origins(player_pos, ae::Vec2::new(1.0, 0.0), 1.0);
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
    fn meteor_aims_with_the_left_stick_facing_on_a_null_aim() {
        // Aiming left (negative facing, no directional hold) puts the zone to the left.
        let left = meteor_strike_origins(ae::Vec2::new(100.0, 100.0), ae::Vec2::ZERO, -1.0);
        let mean_x = left.iter().map(|o| o.x).sum::<f32>() / METEOR_COUNT as f32;
        assert!(
            mean_x < 100.0,
            "a left-facing null-aim cast strikes to the left"
        );
    }
}
