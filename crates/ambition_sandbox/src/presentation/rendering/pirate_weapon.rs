//! Gun-sword (`lasersword_with_guns`) visual layered on top of any
//! actor entity carrying a [`crate::features::HeldItem`] component.
//! Mounted riders and dismounted pirates both keep the component while
//! they still have the weapon, so this visual is item-driven rather
//! than mount-state-driven.
//!
//! Each frame we:
//! 1. Find every alive actor holding the `gun_sword` item.
//! 2. Compute the rider's hand world position from `rider.pos` +
//!    facing-aware hand offset (`HAND_OFFSET_NORM` scaled by the
//!    rider's body height).
//! 3. Compute the aim direction from the hand to the primary player
//!    body (`atan2(dy, dx)`).
//! 4. Spawn a sprite for the gun-sword's idle frame, positioned at
//!    the hand and rotated so the blade points along the aim
//!    direction.
//!
//! Despawn-and-respawn each tick mirrors the pattern in
//! `sync_enemy_projectile_visuals` — no per-entity lifecycle
//! plumbing, the visual set always reflects the live rider set.

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::features::{ActorRuntime, FeatureId, HeldItem};

#[derive(Component)]
pub struct PirateWeaponVisual;

/// Filename of the wielded gun-sword spritesheet. Lives under
/// `crates/ambition_sandbox/assets/sprites/` (installed by
/// `python3 -m ambition_sprite2d_renderer install lasersword_with_guns`).
const WEAPON_SHEET_PATH: &str = "sprites/lasersword_with_guns_spritesheet.png";

/// The PNG at `WEAPON_SHEET_PATH` is the FULL spritesheet (label
/// column + idle / fire / dissipate rows of frames laid out
/// horizontally). To display a single frame we have to specify its
/// source rectangle via `Sprite::rect`. Default to the first idle
/// frame. Numbers read from `lasersword_with_guns_spritesheet.yaml`,
/// row `idle`, frame 0 — bump these if RENDER_SCALE or the
/// auto-crop output changes.
const WEAPON_LABEL_W: f32 = 118.0;
const WEAPON_FRAME_W: f32 = 177.0;
const WEAPON_FRAME_H: f32 = 46.0;
const WEAPON_IDLE_FRAME_X: f32 = WEAPON_LABEL_W;
const WEAPON_IDLE_FRAME_Y: f32 = 0.0;

/// Pixel position of the GRIP anchor inside the idle frame (relative
/// to the frame's top-left corner; read from the spritesheet YAML).
const WEAPON_GRIP_X_PX: f32 = 36.45;
const WEAPON_GRIP_Y_PX: f32 = 23.8;

/// Gun-sword render width as a fraction of the rider's body height.
/// Keeps the weapon proportional to whoever's wielding it; a heavy
/// pirate's gun-sword reads bigger than a raider's automatically.
const WEAPON_WIDTH_PER_RIDER_HEIGHT: f32 = 64.0 / 72.0;

/// Hand position relative to the rider's CENTER, normalized to the
/// rider's body height, in a "facing-right" convention. X is flipped
/// automatically for left-facing pirates.
/// - +x: in front of the body (sword-arm side)
/// - -y: above the rider's center (small-of-back / waist height)
const HAND_OFFSET_NORM: Vec2 = Vec2::new(0.18, -0.05);

/// World-space hand position for a rider. The hand offset is scaled
/// off the rider's own `size.y`, so the same anchor math works for a
/// PirateRaider (78-tall) and a PirateHeavy (110-tall) without
/// per-character tuning.
pub fn rider_hand_world_pos(
    rider_pos: crate::engine_core::Vec2,
    facing: f32,
    rider_height: f32,
) -> crate::engine_core::Vec2 {
    let facing_sign = if facing >= 0.0 { 1.0 } else { -1.0 };
    let hand_local_x = HAND_OFFSET_NORM.x * rider_height * facing_sign;
    let hand_local_y = HAND_OFFSET_NORM.y * rider_height;
    crate::engine_core::Vec2::new(rider_pos.x + hand_local_x, rider_pos.y + hand_local_y)
}

pub fn sync_pirate_weapon_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<crate::GameWorld>,
    rider_actors: Query<(
        &FeatureId,
        &ActorRuntime,
        &HeldItem,
        Option<&crate::features::EnemyKinematics>,
        Option<&crate::features::EnemyStatus>,
    )>,
    player_q: Query<
        &crate::player::PlayerKinematics,
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
        ),
    >,
    existing: Query<Entity, With<PirateWeaponVisual>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Ok(player) = player_q.single() else {
        return;
    };
    let texture = asset_server.load(WEAPON_SHEET_PATH);

    for (_id, actor, held_item, kin, status) in &rider_actors {
        if held_item.id() != "gun_sword" {
            continue;
        }
        if !matches!(actor, ActorRuntime::Enemy) {
            continue;
        }
        let (Some(kin), Some(status)) = (kin, status) else {
            continue;
        };
        if !status.alive {
            continue;
        }

        let rider_height = kin.size.y;
        let hand_world = rider_hand_world_pos(kin.pos, kin.facing, rider_height);
        let aim_world = player.pos;
        let dx = aim_world.x - hand_world.x;
        let dy = aim_world.y - hand_world.y;
        // World-Y grows downward in our sandbox, but Bevy-Y is up.
        // The sprite's canonical "forward" in image coords is +X
        // (sprite-Y is also image-Y, where down is +). Convert by
        // flipping the angle's y component.
        let bevy_angle = dy_world_to_bevy_angle(dx, dy);

        // Custom anchor so the grip pixel sits at the Transform's
        // translation point. Bevy's Anchor uses (0, 0) = center and
        // +y goes UP — but image pixels' +y goes down, so we negate
        // the Y component when normalizing.
        let anchor_x_norm = (WEAPON_GRIP_X_PX - WEAPON_FRAME_W * 0.5) / WEAPON_FRAME_W;
        let anchor_y_norm = -(WEAPON_GRIP_Y_PX - WEAPON_FRAME_H * 0.5) / WEAPON_FRAME_H;

        let aspect = WEAPON_FRAME_W / WEAPON_FRAME_H;
        let weapon_width = WEAPON_WIDTH_PER_RIDER_HEIGHT * rider_height;
        let render = bevy::math::Vec2::new(weapon_width, weapon_width / aspect);

        let translation = world_to_bevy(
            &world.0,
            hand_world,
            // Above the rider visual so the weapon sits on top of
            // the pirate's hand rather than disappearing behind their
            // torso.
            WORLD_Z_PLAYER + 0.7,
        );

        let mut sprite = Sprite::from_image(texture.clone());
        sprite.custom_size = Some(render);
        // Source rect on the spritesheet PNG — the first idle frame.
        // Without this, `Sprite::from_image` renders the whole sheet
        // (label column + idle/fire/dissipate rows) scaled into
        // `custom_size`, which looks like a tiled grid of swords.
        sprite.rect = Some(Rect::from_corners(
            Vec2::new(WEAPON_IDLE_FRAME_X, WEAPON_IDLE_FRAME_Y),
            Vec2::new(
                WEAPON_IDLE_FRAME_X + WEAPON_FRAME_W,
                WEAPON_IDLE_FRAME_Y + WEAPON_FRAME_H,
            ),
        ));

        commands.spawn((
            sprite,
            Anchor(Vec2::new(anchor_x_norm, anchor_y_norm)),
            Transform {
                translation,
                rotation: Quat::from_rotation_z(bevy_angle),
                scale: Vec3::ONE,
            },
            PirateWeaponVisual,
            Name::new("Pirate gun-sword"),
        ));
    }
}

/// World-space (dx, dy) with image conventions (+Y down) to a Bevy
/// rotation angle (in radians, CCW positive about Z). The sprite
/// renders with the blade pointing along +X (sprite forward), so
/// `atan2` of the aim direction in Bevy-Y space gives the rotation
/// needed.
fn dy_world_to_bevy_angle(dx_world: f32, dy_world: f32) -> f32 {
    // Sandbox Y grows downward; Bevy Y grows upward. Flip Y when
    // crossing into Bevy's frame for the atan2 call.
    let dx = dx_world;
    let dy = -dy_world;
    dy.atan2(dx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aim_along_positive_x_is_zero_angle() {
        let angle = dy_world_to_bevy_angle(1.0, 0.0);
        assert!(angle.abs() < 1.0e-6, "got {angle}");
    }

    #[test]
    fn aim_along_negative_y_world_is_quarter_turn_up_in_bevy() {
        // World -Y is "up" in the sandbox; in Bevy that's +Y.
        // Aiming "up" should rotate the sprite +90°.
        let angle = dy_world_to_bevy_angle(0.0, -1.0);
        assert!(
            (angle - std::f32::consts::FRAC_PI_2).abs() < 1.0e-5,
            "got {angle}"
        );
    }

    #[test]
    fn hand_offset_flips_with_facing() {
        let pos = crate::engine_core::Vec2::new(100.0, 50.0);
        let right = rider_hand_world_pos(pos, 1.0, 78.0);
        let left = rider_hand_world_pos(pos, -1.0, 78.0);
        assert!(right.x > pos.x, "right-facing hand should be to the right");
        assert!(left.x < pos.x, "left-facing hand should be to the left");
        // Y is the same regardless of facing.
        assert!((right.y - left.y).abs() < 1.0e-5);
    }
}
