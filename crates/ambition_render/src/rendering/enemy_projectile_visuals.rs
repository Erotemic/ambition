//! Per-frame sprite rebuild for enemy projectiles. Mirrors the
//! player-projectile visuals system but with hostile per-owner art:
//! GNU-ton apples render as a generated apple sprite,
//! `lasersword:`-prefixed pirate volleys render as a small spinning
//! laser-sword sprite, and everything else falls back to a
//! red/orange rectangle.

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::projectile_visuals::{ProjectileVisualLink, VisualProjectile};
use ambition_gameplay_core::enemy_projectile::EnemyProjectile;
use ambition_gameplay_core::projectile::ProjectileOwnerId;

#[derive(Component)]
pub struct EnemyProjectileVisual;

/// Which art an enemy-projectile visual uses, recorded at spawn so the
/// per-frame refresh applies the right transform (rotation for the spinning
/// lasersword, flip for the rectangle, position-only for the apple).
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum EnemyVisualKind {
    Apple,
    Lasersword,
    Rect,
}

/// `owner_id` prefix stamp used by GNU-ton's apple-rain attack so the
/// visuals layer can swap the default red rectangle for the generated
/// apple sprite.
const APPLE_OWNER_PREFIX: &str = "gnu_ton_apple";
const APPLE_SPRITE_PATH: &str = "sprites/gnu_ton_boss/gnu_ton_apple.png";
/// Owner prefix used by `PirateOnShark` discharges. Routes the
/// projectile to the `lasersword` sprite rendered rotated along its
/// velocity vector. Set by the firing logic for shark-rider archetypes.
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

/// Spritesheet path for the spinning-lasersword projectile, exposed so the
/// player's held gun-sword shot can render the SAME sword the pirates fire.
pub const LASERSWORD_SHEET: &str = LASERSWORD_SHEET_PATH;

/// Texture handles used by enemy projectile visuals. Stored in system-local
/// state so per-frame visual sync does not call `AssetServer::load` and churn
/// image decode / asset bookkeeping every tick.
pub struct EnemyProjectileVisualArt {
    apple: Handle<Image>,
    lasersword: Handle<Image>,
}

impl EnemyProjectileVisualArt {
    fn load(asset_server: &AssetServer) -> Self {
        Self {
            apple: asset_server.load(APPLE_SPRITE_PATH),
            lasersword: asset_server.load(LASERSWORD_SHEET_PATH),
        }
    }
}

/// Build the lasersword projectile sprite (idle frame, pommel-anchored) + its
/// z-rotation for a shot traveling at `vel` (world space, y-down). Shared by the
/// enemy volley visual and the player's held gun-sword shot so both render an
/// identical spinning sword aligned to its velocity.
pub fn lasersword_projectile_sprite(
    texture: Handle<Image>,
    vel: ambition_gameplay_core::engine_core::Vec2,
) -> (Sprite, Anchor, Quat) {
    // Bevy +Y is up; sandbox +Y is down — flip Y when computing rotation.
    let bevy_dx = vel.x;
    let bevy_dy = -vel.y;
    let angle = if bevy_dx == 0.0 && bevy_dy == 0.0 {
        0.0
    } else {
        bevy_dy.atan2(bevy_dx)
    };
    let aspect = LASERSWORD_FRAME_W / LASERSWORD_FRAME_H;
    let render = Vec2::new(LASERSWORD_RENDER_WIDTH, LASERSWORD_RENDER_WIDTH / aspect);
    let anchor_x_norm = (LASERSWORD_POMMEL_X_PX - LASERSWORD_FRAME_W * 0.5) / LASERSWORD_FRAME_W;
    let anchor_y_norm = -(LASERSWORD_POMMEL_Y_PX - LASERSWORD_FRAME_H * 0.5) / LASERSWORD_FRAME_H;
    let mut sprite = Sprite::from_image(texture);
    sprite.custom_size = Some(render);
    // Clip to the first idle frame (the sheet has a label column + idle +
    // dissipate rows; without this it tiles a grid of swords).
    sprite.rect = Some(Rect::from_corners(
        Vec2::new(LASERSWORD_IDLE_FRAME_X, LASERSWORD_IDLE_FRAME_Y),
        Vec2::new(
            LASERSWORD_IDLE_FRAME_X + LASERSWORD_FRAME_W,
            LASERSWORD_IDLE_FRAME_Y + LASERSWORD_FRAME_H,
        ),
    ));
    (
        sprite,
        Anchor(Vec2::new(anchor_x_norm, anchor_y_norm)),
        Quat::from_rotation_z(angle),
    )
}

fn is_apple_owner(owner_id: &str) -> bool {
    owner_id.starts_with(APPLE_OWNER_PREFIX)
}

fn is_lasersword_owner(owner_id: &str) -> bool {
    owner_id.starts_with(LASERSWORD_OWNER_PREFIX)
}

/// Maintain a persistent sprite for each in-flight enemy projectile entity
/// (Phase 3d): spawn one when the projectile appears (kind picked from
/// `owner_id`), refresh its transform each frame, despawn it when the
/// projectile entity is gone. Mirrors the player pool's persistent visuals.
pub fn sync_enemy_projectile_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<ambition_gameplay_core::GameWorld>,
    // Localized gravity so a projectile's "upright" follows the acceleration frame.
    gravity: ambition_gameplay_core::physics::GravityCtx,
    // Enemy projectiles that don't yet have a visual get one spawned.
    new_projectiles: Query<
        (
            Entity,
            &ambition_gameplay_core::player::BodyKinematics,
            &ProjectileOwnerId,
        ),
        (With<EnemyProjectile>, Without<ProjectileVisualLink>),
    >,
    // Live bodies for the per-frame transform refresh.
    bodies: Query<&ambition_gameplay_core::player::BodyKinematics, With<EnemyProjectile>>,
    mut visuals: Query<
        (
            Entity,
            &VisualProjectile,
            &EnemyVisualKind,
            &mut Transform,
            &mut Sprite,
        ),
        With<EnemyProjectileVisual>,
    >,
    mut art: Local<Option<EnemyProjectileVisualArt>>,
) {
    let art = art.get_or_insert_with(|| EnemyProjectileVisualArt::load(&asset_server));

    // Spawn one persistent visual per NEW enemy projectile entity.
    for (proj_entity, kin, owner) in &new_projectiles {
        let render_size = bevy::math::Vec2::new((kin.size.x).max(8.0), (kin.size.y).max(8.0));
        let translation = ambition_gameplay_core::config::world_to_bevy(
            &world.0,
            kin.pos,
            ambition_gameplay_core::config::WORLD_Z_PLAYER + 1.8,
        );
        let visual = if is_apple_owner(&owner.0) {
            spawn_apple_visual(
                &mut commands,
                &art.apple,
                translation,
                render_size,
                proj_entity,
            )
        } else if is_lasersword_owner(&owner.0) {
            spawn_lasersword_visual(
                &mut commands,
                &art.lasersword,
                translation,
                kin.vel,
                proj_entity,
            )
        } else {
            // Hostile orange-red: readable against the sky-blue background
            // of the pirate arena and visually distinct from the warm
            // yellow of player fireballs.
            let tint = Color::srgba(1.0, 0.45, 0.18, 0.95);
            let mut sprite = Sprite::from_color(tint, render_size);
            sprite.flip_x = kin.vel.x < 0.0;
            commands
                .spawn((
                    sprite,
                    Transform::from_translation(translation),
                    EnemyProjectileVisual,
                    EnemyVisualKind::Rect,
                    VisualProjectile(proj_entity),
                    Name::new("Enemy projectile"),
                ))
                .id()
        };
        commands
            .entity(proj_entity)
            .insert(ProjectileVisualLink(visual));
    }

    // Refresh existing visuals from their live body; despawn orphans whose
    // projectile entity is gone.
    for (visual_entity, link, kind, mut transform, mut sprite) in &mut visuals {
        let Ok(kin) = bodies.get(link.0) else {
            commands.entity(visual_entity).despawn();
            continue;
        };
        let z = ambition_gameplay_core::config::WORLD_Z_PLAYER + 1.8;
        transform.translation = ambition_gameplay_core::config::world_to_bevy(&world.0, kin.pos, z);
        match kind {
            EnemyVisualKind::Apple => {
                // Keep the apple upright relative to its local gravity (identity
                // under normal gravity; rotates under sideways/inverted).
                transform.rotation = Quat::from_rotation_z(
                    ambition_gameplay_core::platformer_runtime::gravity::gravity_upright_angle(
                        gravity.dir_at(kin.pos),
                    ),
                );
            }
            EnemyVisualKind::Lasersword => {
                // Re-align the blade to the live velocity each frame (the old
                // per-frame rebuild recomputed the rotation from `vel`).
                let (_, _, rotation) = lasersword_projectile_sprite(sprite.image.clone(), kin.vel);
                transform.rotation = rotation;
            }
            EnemyVisualKind::Rect => {
                sprite.flip_x = kin.vel.x < 0.0;
            }
        }
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
    vel: ambition_gameplay_core::engine_core::Vec2,
    projectile: Entity,
) -> Entity {
    let (sprite, anchor, rotation) = lasersword_projectile_sprite(texture.clone(), vel);
    commands
        .spawn((
            sprite,
            anchor,
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            EnemyProjectileVisual,
            EnemyVisualKind::Lasersword,
            VisualProjectile(projectile),
            Name::new("Lasersword projectile"),
        ))
        .id()
}

/// Generated apple projectile sprite, scaled to match the projectile
/// body size so it reads cleanly against the arena background.
fn spawn_apple_visual(
    commands: &mut Commands,
    texture: &Handle<Image>,
    translation: bevy::math::Vec3,
    render_size: bevy::math::Vec2,
    projectile: Entity,
) -> Entity {
    let mut sprite = Sprite::from_image(texture.clone());
    sprite.custom_size = Some(bevy::math::Vec2::new(
        render_size.x * 1.12,
        render_size.y * 1.12,
    ));
    commands
        .spawn((
            sprite,
            Transform::from_translation(translation),
            EnemyProjectileVisual,
            EnemyVisualKind::Apple,
            VisualProjectile(projectile),
            Name::new("GNU-ton apple"),
        ))
        .id()
}
