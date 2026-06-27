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
use ambition_engine_core as ae;
use crate::features::HeldItem;
use crate::player::{PlayerInputFrame, BodyMana};
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::actor::BodyKinematics;
use crate::projectile::ProjectileFaction;

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

/// `Attack` while holding the volley gauntlet fires a fan of **player-faction**
/// bolts along the aim direction (right-stick / movement axis / facing, via the
/// shared `held_shot_aim`). Plain Attack only — `Shield + Attack` drops the item
/// (the id is excluded from throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_volley_system(
    gravity: crate::physics::GravityCtx,
    user_settings: Option<Res<crate::persistence::settings::UserSettings>>,
    mut players: Query<
        (
            Entity,
            &PlayerInputFrame,
            &BodyKinematics,
            &HeldItem,
            &mut BodyMana,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let Ok((entity, input, kin, held, mut mana)) = players.single_mut() else {
        return;
    };
    if !input.frame.attack_pressed || input.frame.shield_held {
        return;
    }
    if held.spec.id != VOLLEY_ID {
        return;
    }
    // Costs mana — out of mana, no volley.
    if !mana.meter.try_spend(VOLLEY_MANA_COST) {
        return;
    }
    let gravity_dir = gravity.dir_at(kin.pos);
    let modes = crate::items::pickup::control_frame_modes_from_settings(user_settings.as_deref());
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let aim_local =
        crate::items::pickup::held_shot_aim_local(&input.frame, kin.facing, frame, modes);
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
        effects.write(crate::effects::EffectRequest {
            // The firing actor owns every bolt, so a kill attributes back to the
            // player (the executor stamps `ProjectileOwner` from this entity).
            owner: entity,
            effect: crate::effects::Effect::Projectiles {
                faction: ProjectileFaction::Player,
                shots: vec![EnemyProjectileSpawn {
                    origin,
                    dir,
                    speed: VOLLEY_SPEED,
                    damage: VOLLEY_DAMAGE,
                    max_lifetime: VOLLEY_LIFETIME,
                    half_extent: VOLLEY_HALF,
                    owner_id: "player_volley".into(),
                    gravity: 0.0,
                    visual_tag: 0,
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
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // Chain the enemy-pool spawn consumer after the fire system.
        app.add_systems(
            Update,
            (
                fire_volley_system,
                crate::enemy_projectile::apply_projectile_effects,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn attack_with_the_volley_spawns_a_fan_of_player_faction_bolts() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, VOLLEY_ID);
        app.world_mut()
            .get_mut::<PlayerInputFrame>(player)
            .unwrap()
            .frame
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
        // Every bolt is owned by the firing player entity, so a kill attributes
        // back to them (the executor stamps `ProjectileOwner` from the request).
        let owners: Vec<_> = app
            .world_mut()
            .query::<&crate::projectile::ProjectileOwner>()
            .iter(app.world())
            .map(|o| o.0)
            .collect();
        assert_eq!(owners.len(), VOLLEY_SHOT_COUNT, "every bolt carries an owner");
        assert!(
            owners.iter().all(|&o| o == player),
            "bolts are owned by the firing player, got {owners:?} (player {player:?})"
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

    #[test]
    fn volley_origin_uses_extent_along_the_local_aim_axis() {
        let size = ae::Vec2::new(24.0, 40.0);
        let side = volley_origin_local_offset(ae::Vec2::new(1.0, 0.0), size);
        let head = volley_origin_local_offset(ae::Vec2::new(0.0, -1.0), size);
        assert_eq!(side, ae::Vec2::new(20.0, 0.0));
        assert_eq!(head, ae::Vec2::new(0.0, -28.0));
    }

    #[test]
    fn volley_origin_is_c4_equivariant_for_local_aim() {
        let pos = ae::Vec2::new(100.0, 100.0);
        let size = ae::Vec2::new(24.0, 40.0);
        let local_aim = ae::Vec2::new(0.0, -1.0);
        let expected_local = volley_origin_local_offset(local_aim, size);
        for gravity_dir in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let world_origin = volley_origin_world(pos, size, local_aim, frame);
            let local_origin = frame.to_local(world_origin - pos);
            assert!(
                (local_origin - expected_local).length() < 0.001,
                "volley origin should preserve local geometry under gravity {gravity_dir:?}; got {local_origin:?}"
            );
        }
    }
}
