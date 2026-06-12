//! Volley — a player-wielded **ranged** boss attack: a fan of bolts that damage
//! enemies, fired through the now faction-aware shared projectile pool
//! (`EnemyProjectileState::spawn_with_faction`).
//!
//! This is the ranged counterpart to `crate::abilities::ranged::shockwave` (the wielded AOE). The
//! pool used to be faction-segregated — `update_enemy_projectiles` only ever
//! damaged the player — so a player-fired bolt in it would hit the player. Now
//! the body's `ProjectileFaction` routes its damage: a `Player` shot damages
//! enemies/bosses and expires on contact, an `Enemy` shot still hits the player.
//! Same pool, same step system, faction is the only difference — the projectile
//! analog of the shockwave's faction-tagged `Hitbox`.

use bevy::prelude::*;

use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerMana, PrimaryPlayer};
use crate::projectile::{ProjectileFaction, SpawnProjectile};

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

/// `Attack` while holding the volley gauntlet fires a fan of **player-faction**
/// bolts along the aim direction (right-stick / movement axis / facing, via the
/// shared `held_shot_aim`). Plain Attack only — `Shield + Attack` drops the item
/// (the id is excluded from throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_volley_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (&BodyKinematics, &HeldItem, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, held, mut mana)) = players.single_mut() else {
        return;
    };
    if held.spec.id != VOLLEY_ID {
        return;
    }
    // Costs mana — out of mana, no volley.
    if !mana.meter.try_spend(VOLLEY_MANA_COST) {
        return;
    }
    let aim = crate::items::pickup::held_shot_aim(&control, kin.facing);
    if aim == ae::Vec2::ZERO {
        return;
    }
    let base_angle = aim.y.atan2(aim.x);
    let origin = kin.pos + aim * (kin.size.x * 0.5 + 8.0);
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
        spawn_projectiles.write(SpawnProjectile::enemy(
            EnemyProjectileSpawn {
                origin,
                dir,
                speed: VOLLEY_SPEED,
                damage: VOLLEY_DAMAGE,
                max_lifetime: VOLLEY_LIFETIME,
                half_extent: VOLLEY_HALF,
                owner_id: "player_volley".into(),
                gravity: 0.0,
            },
            ProjectileFaction::Player,
        ));
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
        app.add_message::<SpawnProjectile>();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // Phase 3b: firing emits a SpawnProjectile; the enemy-pool consumer
        // spawns the projectile entity, so chain it after the fire system.
        app.add_systems(
            Update,
            (
                fire_volley_system,
                crate::enemy_projectile::apply_enemy_spawn_projectile_messages,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn attack_with_the_volley_spawns_a_fan_of_player_faction_bolts() {
        let mut app = test_app();
        spawn_primary_player_holding(&mut app, VOLLEY_ID);
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        let bodies = enemy_projectile_bodies(&mut app);
        assert_eq!(bodies.len(), VOLLEY_SHOT_COUNT, "one bolt per fan slot");
        // Every bolt is Player-faction so the faction-aware pool routes its
        // damage to enemies, not the player who fired it.
        assert!(
            bodies
                .iter()
                .all(|b| b.body.game.faction == ProjectileFaction::Player),
            "the wielded volley fires player-faction bolts"
        );
        // The bolts fan out — not all the same direction.
        let dirs: Vec<f32> = bodies
            .iter()
            .map(|b| b.body.kin.vel.y.atan2(b.body.kin.vel.x))
            .collect();
        assert!(
            dirs.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-3),
            "the volley spreads across distinct angles"
        );
    }

    #[test]
    fn no_volley_without_attack() {
        let mut app = test_app();
        spawn_primary_player_holding(&mut app, VOLLEY_ID);
        app.update();
        assert_eq!(enemy_projectile_bodies(&mut app).len(), 0);
    }
}
