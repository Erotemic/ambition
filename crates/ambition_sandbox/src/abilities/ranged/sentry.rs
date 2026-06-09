//! Sentry — a player-wielded **deployable turret**. `Attack` drops a stationary
//! sentry that auto-fires player-faction bolts at the nearest enemy in range on
//! a cadence, for a few seconds, then expires. It fills a gap in the kit: the
//! puppy-slug summon (`crate::abilities::thrown::puppy_slug_gun`) is *passive* (the slugs just
//! wander), and every other wielded ability is a one-shot the player aims — the
//! sentry is the first thing the player deploys that **autonomously attacks**.
//!
//! It fires through the same faction-aware projectile pool the volley uses
//! (`EnemyProjectileState::spawn_with_faction(..., Player)`), so its bolts damage
//! enemies/bosses and ignore the player. Bosses carry `BodyKinematics`, but the
//! sentry targets by `FeatureAabb` + `ActorFaction::Enemy`, so it shoots mobs
//! (not bosses or the player). Pairs with the vortex: drop a sentry, vortex the
//! mob onto it.

use bevy::prelude::*;

use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::engine_core as ae;
use crate::features::{ActorFaction, FeatureAabb, FeatureSimEntity, HeldItem};
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerMana, PrimaryPlayer};
use crate::projectile::{ProjectileFaction, SpawnProjectile};

/// Held-item id of the sentry gauntlet.
pub const SENTRY_ID: &str = "sentry";

/// Mana the sentry spends per deploy (out of 100).
const SENTRY_MANA_COST: f32 = 28.0;

/// How long (s) a deployed sentry lives.
const SENTRY_LIFETIME_S: f32 = 5.0;
/// Seconds between shots.
const SENTRY_FIRE_INTERVAL_S: f32 = 0.55;
/// Targeting range (px) — enemies beyond this are ignored.
const SENTRY_RANGE: f32 = 480.0;
const SENTRY_BOLT_SPEED: f32 = 430.0;
const SENTRY_BOLT_DAMAGE: i32 = 2;
const SENTRY_BOLT_LIFETIME: f32 = 1.4;
const SENTRY_BOLT_HALF: ae::Vec2 = ae::Vec2::new(7.0, 7.0);

/// A deployed sentry: lives at `pos`, fires when `fire_cooldown` hits zero.
#[derive(Component, Debug, Clone, Copy)]
pub struct Sentry {
    pub pos: ae::Vec2,
    pub remaining_s: f32,
    pub fire_cooldown: f32,
}

/// `Attack` while holding the sentry gauntlet drops a [`Sentry`] at the player's
/// feet. Plain Attack only — `Shield + Attack` drops the item (the id is
/// `UseSystem`).
pub fn fire_sentry_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (&BodyKinematics, &HeldItem, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut commands: Commands,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, held, mut mana)) = players.single_mut() else {
        return;
    };
    if held.spec.id != SENTRY_ID {
        return;
    }
    if !mana.meter.try_spend(SENTRY_MANA_COST) {
        return;
    }
    commands.spawn((
        Sentry {
            pos: kin.pos,
            remaining_s: SENTRY_LIFETIME_S,
            // A short arm delay before the first shot.
            fire_cooldown: 0.25,
        },
        Name::new("Sentry turret"),
    ));
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

/// Tick every sentry: age it out, and when its cadence is ready, fire one
/// player-faction bolt at the nearest Enemy-faction actor within range. Runs on
/// `scaled_dt` (bullet-time slows the turret with everything else).
pub fn update_sentries(
    world_time: Res<crate::WorldTime>,
    mut commands: Commands,
    mut sentries: Query<(Entity, &mut Sentry)>,
    enemies: Query<(&FeatureAabb, &ActorFaction), With<FeatureSimEntity>>,
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for (entity, mut sentry) in &mut sentries {
        sentry.remaining_s -= dt;
        if sentry.remaining_s <= 0.0 {
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
            continue;
        }
        sentry.fire_cooldown -= dt;
        if sentry.fire_cooldown > 0.0 {
            continue;
        }
        // Nearest enemy within range.
        let target = enemies
            .iter()
            .filter(|(_, f)| **f == ActorFaction::Enemy)
            .map(|(aabb, _)| aabb.center)
            .filter(|c| c.distance(sentry.pos) <= SENTRY_RANGE)
            .min_by(|a, b| {
                a.distance_squared(sentry.pos)
                    .total_cmp(&b.distance_squared(sentry.pos))
            });
        let Some(target) = target else {
            // No target — idle (keep the cadence ready so it fires the instant
            // an enemy wanders in).
            sentry.fire_cooldown = 0.0;
            continue;
        };
        let dir = (target - sentry.pos).normalize_or_zero();
        if dir == ae::Vec2::ZERO {
            continue;
        }
        spawn_projectiles.write(SpawnProjectile::enemy(
            EnemyProjectileSpawn {
                origin: sentry.pos,
                dir,
                speed: SENTRY_BOLT_SPEED,
                damage: SENTRY_BOLT_DAMAGE,
                max_lifetime: SENTRY_BOLT_LIFETIME,
                half_extent: SENTRY_BOLT_HALF,
                owner_id: "player_sentry".into(),
                gravity: 0.0,
            },
            ProjectileFaction::Player,
        ));
        sentry.fire_cooldown = SENTRY_FIRE_INTERVAL_S;
        sfx.write(crate::audio::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: sentry.pos,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;
    use crate::enemy_projectile::test_support::enemy_projectile_bodies;
    use crate::enemy_projectile::EnemyProjectileState;
    use crate::player::PlayerBaseSize;
    use crate::projectile::ProjectileSeqCounter;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<SpawnProjectile>();
        app.insert_resource(ControlFrame::default());
        app.insert_resource(crate::WorldTime {
            raw_dt: 0.1,
            scaled_dt: 0.1,
        });
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // Phase 3b: update_sentries emits SpawnProjectile; the enemy-pool
        // consumer spawns the projectile entity (chained after).
        app.add_systems(
            Update,
            (
                fire_sentry_system,
                update_sentries,
                crate::enemy_projectile::apply_enemy_spawn_projectile_messages,
            )
                .chain(),
        );
        app
    }

    fn spawn_player_holding_sentry(app: &mut App) {
        let spec = crate::brain::held_item_by_id(SENTRY_ID).unwrap();
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            PlayerBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            HeldItem::new(spec),
            PlayerMana::default(),
        ));
    }

    #[test]
    fn deployed_sentry_fires_a_player_bolt_at_a_nearby_enemy() {
        let mut app = test_app();
        spawn_player_holding_sentry(&mut app);
        // An enemy within range of where the sentry will deploy (100,100).
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureAabb::new(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
        ));
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update(); // deploy (arm delay 0.25; dt 0.1 → not yet firing)
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = false;
        // Tick until past the arm delay + a fire interval.
        for _ in 0..10 {
            app.update();
        }
        let bodies = enemy_projectile_bodies(&mut app);
        assert!(
            !bodies.is_empty(),
            "the sentry should have fired at the enemy"
        );
        assert!(
            bodies
                .iter()
                .all(|b| b.body.game.faction == ProjectileFaction::Player),
            "sentry bolts are player-faction (damage enemies, not the player)"
        );
    }

    #[test]
    fn sentry_with_no_enemy_in_range_does_not_fire_and_expires() {
        let mut app = test_app();
        spawn_player_holding_sentry(&mut app);
        // Enemy far outside SENTRY_RANGE.
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureAabb::new(ae::Vec2::new(2000.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
        ));
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = false;
        for _ in 0..5 {
            app.update();
        }
        assert!(
            enemy_projectile_bodies(&mut app).is_empty(),
            "no target in range → no shots"
        );
        // Age out (lifetime 5s at 0.1/tick → 50 ticks).
        for _ in 0..55 {
            app.update();
        }
        let count = app.world_mut().query::<&Sentry>().iter(app.world()).count();
        assert_eq!(count, 0, "the sentry expires and despawns");
    }
}
