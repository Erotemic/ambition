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

use crate::enemy_projectile::{EnemyProjectileSpawn, EnemyProjectileState};
use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerMana, PrimaryPlayer};
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
    mut enemy_projectiles: ResMut<EnemyProjectileState>,
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
    let aim = crate::item_pickup::held_shot_aim(&control, kin.facing);
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
        enemy_projectiles.spawn_with_faction(
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
        );
    }
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;
    use crate::player::PlayerBaseSize;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<EnemyProjectileState>();
        app.add_systems(Update, fire_volley_system);
        app
    }

    fn spawn_player_holding_volley(app: &mut App) {
        let spec = crate::brain::held_item_by_id(VOLLEY_ID).unwrap();
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
    fn attack_with_the_volley_spawns_a_fan_of_player_faction_bolts() {
        let mut app = test_app();
        spawn_player_holding_volley(&mut app);
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        let state = app.world().resource::<EnemyProjectileState>();
        assert_eq!(
            state.bodies.len(),
            VOLLEY_SHOT_COUNT,
            "one bolt per fan slot"
        );
        // Every bolt is Player-faction so the faction-aware pool routes its
        // damage to enemies, not the player who fired it.
        assert!(
            state
                .bodies
                .iter()
                .all(|b| b.body.faction == ProjectileFaction::Player),
            "the wielded volley fires player-faction bolts"
        );
        // The bolts fan out — not all the same direction.
        let dirs: Vec<f32> = state
            .bodies
            .iter()
            .map(|b| b.body.vel.y.atan2(b.body.vel.x))
            .collect();
        assert!(
            dirs.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-3),
            "the volley spreads across distinct angles"
        );
    }

    #[test]
    fn no_volley_without_attack() {
        let mut app = test_app();
        spawn_player_holding_volley(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<EnemyProjectileState>().bodies.len(),
            0
        );
    }
}
