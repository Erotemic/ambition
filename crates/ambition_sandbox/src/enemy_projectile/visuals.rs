//! Per-frame sprite rebuild for enemy projectiles. Mirrors the
//! player-projectile visuals system but with hostile per-owner art:
//! GNU-ton apples render as the apple stack, `lasersword:`-prefixed
//! pirate volleys render as a small spinning laser-sword sprite, and
//! everything else falls back to a red/orange rectangle.

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::state::EnemyProjectileState;

#[derive(Component)]
pub struct EnemyProjectileVisual;

/// `owner_id` prefix stamp used by GNU-ton's apple-rain attack so the
/// visuals layer can swap the default red rectangle for the apple
/// shape (red body + green leaf + brown stem).
const APPLE_OWNER_PREFIX: &str = "gnu_ton_apple";
/// Owner prefix used by `PirateOnShark` discharges. Routes the
/// projectile to the `lasersword` sprite rendered rotated along its
/// velocity vector. Set by `EnemyRuntime::update` when the firing
/// archetype is `PirateOnShark`.
const LASERSWORD_OWNER_PREFIX: &str = "lasersword";

const LASERSWORD_SHEET_PATH: &str = "sprites/lasersword_spritesheet.png";

/// The PNG at `LASERSWORD_SHEET_PATH` is the FULL spritesheet (label
/// column + idle / dissipate rows). To display a single frame we
/// have to clip to its source rectangle via `Sprite::rect`. Default
/// to the first idle frame. Read from
/// `lasersword_spritesheet.yaml`, row `idle`, frame 0.
const LASERSWORD_LABEL_W: f32 = 110.0;
const LASERSWORD_FRAME_W: f32 = 169.0;
const LASERSWORD_FRAME_H: f32 = 44.0;
const LASERSWORD_IDLE_FRAME_X: f32 = LASERSWORD_LABEL_W;
const LASERSWORD_IDLE_FRAME_Y: f32 = 0.0;

/// Pommel anchor in the idle frame — rotation pivot of the
/// projectile sprite. Game rotation aligns the blade to the
/// projectile's velocity vector. Read from
/// `lasersword_spritesheet.yaml::rects[0].anchors.pommel`.
const LASERSWORD_POMMEL_X_PX: f32 = 14.0;
const LASERSWORD_POMMEL_Y_PX: f32 = 22.0;

const LASERSWORD_RENDER_WIDTH: f32 = 56.0;

fn is_apple_owner(owner_id: &str) -> bool {
    owner_id.starts_with(APPLE_OWNER_PREFIX)
}

fn is_lasersword_owner(owner_id: &str) -> bool {
    owner_id.starts_with(LASERSWORD_OWNER_PREFIX)
}

pub fn sync_enemy_projectile_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<crate::GameWorld>,
    state: Res<EnemyProjectileState>,
    existing: Query<Entity, With<EnemyProjectileVisual>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let lasersword_texture = asset_server.load(LASERSWORD_SHEET_PATH);
    for projectile in &state.bodies {
        let body = &projectile.body;
        let render_size = bevy::math::Vec2::new(
            (body.half_extent.x * 2.0).max(8.0),
            (body.half_extent.y * 2.0).max(8.0),
        );
        let translation =
            crate::config::world_to_bevy(&world.0, body.pos, crate::config::WORLD_Z_PLAYER + 1.8);
        if is_apple_owner(&projectile.owner_id) {
            spawn_apple_visual(&mut commands, translation, render_size);
            continue;
        }
        if is_lasersword_owner(&projectile.owner_id) {
            spawn_lasersword_visual(&mut commands, &lasersword_texture, translation, body.vel);
            continue;
        }
        // Hostile orange-red: readable against the sky-blue background
        // of the pirate arena and visually distinct from the warm
        // yellow of player fireballs.
        let tint = Color::srgba(1.0, 0.45, 0.18, 0.95);
        let mut sprite = Sprite::from_color(tint, render_size);
        sprite.flip_x = body.vel.x < 0.0;
        commands.spawn((
            sprite,
            Transform::from_translation(translation),
            EnemyProjectileVisual,
            Name::new("Enemy projectile"),
        ));
    }
}

/// Spawn the lasersword-projectile visual at ``translation``, rotated
/// so the blade points along the projectile's velocity. The sprite's
/// pommel anchor is set so rotation pivots about the back-of-grip,
/// which is what the projectile metadata reports as ``pommel`` and
/// matches how the wielded weapon was rendered.
fn spawn_lasersword_visual(
    commands: &mut Commands,
    texture: &Handle<Image>,
    translation: bevy::math::Vec3,
    vel: ambition_engine::Vec2,
) {
    // Bevy +Y is up; sandbox +Y is down — flip Y when computing the
    // sprite rotation from the velocity vector.
    let bevy_dx = vel.x;
    let bevy_dy = -vel.y;
    let angle = if bevy_dx == 0.0 && bevy_dy == 0.0 {
        0.0
    } else {
        bevy_dy.atan2(bevy_dx)
    };
    let aspect = LASERSWORD_FRAME_W / LASERSWORD_FRAME_H;
    let render = bevy::math::Vec2::new(LASERSWORD_RENDER_WIDTH, LASERSWORD_RENDER_WIDTH / aspect);
    let anchor_x_norm = (LASERSWORD_POMMEL_X_PX - LASERSWORD_FRAME_W * 0.5) / LASERSWORD_FRAME_W;
    let anchor_y_norm = -(LASERSWORD_POMMEL_Y_PX - LASERSWORD_FRAME_H * 0.5) / LASERSWORD_FRAME_H;
    let mut sprite = Sprite::from_image(texture.clone());
    sprite.custom_size = Some(render);
    // Clip to the first idle frame — without this the whole
    // multi-row spritesheet (label column + idle + dissipate)
    // would be scaled into `custom_size`, looking like a tiled
    // grid of swords.
    sprite.rect = Some(Rect::from_corners(
        Vec2::new(LASERSWORD_IDLE_FRAME_X, LASERSWORD_IDLE_FRAME_Y),
        Vec2::new(
            LASERSWORD_IDLE_FRAME_X + LASERSWORD_FRAME_W,
            LASERSWORD_IDLE_FRAME_Y + LASERSWORD_FRAME_H,
        ),
    ));
    commands.spawn((
        sprite,
        Anchor(Vec2::new(anchor_x_norm, anchor_y_norm)),
        Transform {
            translation,
            rotation: Quat::from_rotation_z(angle),
            scale: Vec3::ONE,
        },
        EnemyProjectileVisual,
        Name::new("Lasersword projectile"),
    ));
}

/// Three-sprite apple: red body, green leaf, brown stem. The stack
/// is intentionally simple — we don't have an apple PNG yet and the
/// flat-color triplet still reads at a glance against the GNU-ton
/// arena's warm background. All three sprites carry
/// `EnemyProjectileVisual` so the per-frame cleanup loop sweeps
/// them together with the rest of the projectile visuals.
fn spawn_apple_visual(
    commands: &mut Commands,
    translation: bevy::math::Vec3,
    render_size: bevy::math::Vec2,
) {
    // Bump the apparent radius a touch above the collision box so the
    // shape reads as round even though the body is a flat square.
    let body_size = bevy::math::Vec2::new(render_size.x * 1.05, render_size.y * 1.05);
    let body_color = Color::srgba(0.82, 0.16, 0.18, 1.0);
    commands.spawn((
        Sprite::from_color(body_color, body_size),
        Transform::from_translation(translation),
        EnemyProjectileVisual,
        Name::new("Apple"),
    ));
    let stem_color = Color::srgba(0.30, 0.18, 0.08, 1.0);
    let stem_size = bevy::math::Vec2::new(2.0_f32.max(render_size.x * 0.12), render_size.y * 0.30);
    let stem_translation = translation + bevy::math::Vec3::new(0.0, body_size.y * 0.55, 0.04);
    commands.spawn((
        Sprite::from_color(stem_color, stem_size),
        Transform::from_translation(stem_translation),
        EnemyProjectileVisual,
        Name::new("Apple stem"),
    ));
    let leaf_color = Color::srgba(0.20, 0.62, 0.22, 1.0);
    let leaf_size = bevy::math::Vec2::new(body_size.x * 0.55, body_size.y * 0.30);
    let leaf_translation =
        translation + bevy::math::Vec3::new(body_size.x * 0.30, body_size.y * 0.60, 0.05);
    commands.spawn((
        Sprite::from_color(leaf_color, leaf_size),
        Transform::from_translation(leaf_translation),
        EnemyProjectileVisual,
        Name::new("Apple leaf"),
    ));
}
