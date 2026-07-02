//! Persistent Bevy sprites for every in-flight projectile — player AND enemy —
//! plus the per-player charge indicator.
//!
//! There is ONE art-selection path: each projectile entity carries a
//! [`ProjectileVisualKind`] component (set at spawn), and the renderer asks that
//! kind for its data [`ProjectileArt`] descriptor. The renderer matches only on
//! the descriptor's generic `source` / `size` / `rotation` — never on the named
//! kind, and never on `owner_id`. A new projectile kind that reuses existing
//! render capabilities needs no edit here (the engine-for-other-games test).
//!
//! Animated sheet kinds (e.g. the PCA's glider) cycle their frames from the
//! sheet manifest's row metadata via a per-projectile [`ProjectileFrameAnim`]
//! timer — the frame rects come from the [`SheetRegistry`], not hardcoded.

use bevy::math::{Rect, Vec2};
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_gameplay_core::actor::BodyKinematics;
use ambition_gameplay_core::assets::game_assets::{EntitySprite, GameAssets};
use ambition_gameplay_core::physics::GravityCtx;
use ambition_gameplay_core::platformer_runtime::gravity::gravity_upright_angle;
use ambition_gameplay_core::projectile::{
    LiveProjectile, ProjectileArtSource, ProjectileKind, ProjectileRenderSize, ProjectileRotation,
    ProjectileVisualKind,
};
use ambition_sprite_sheet::SheetRegistry;

/// Marker on the persistent per-projectile sprite entity.
#[derive(Component)]
pub struct ProjectileVisual;

/// Back-reference from a projectile visual to its sim projectile entity. Used to
/// refresh the visual's transform each frame and despawn it once the projectile
/// entity is gone.
#[derive(Component, Clone, Copy)]
pub struct VisualProjectile(pub Entity);

/// Forward link from a projectile entity to its spawned visual entity, so the
/// "spawn a visual for projectiles that don't have one yet" pass is idempotent
/// (a projectile is only matched while it lacks this component).
#[derive(Component, Clone, Copy)]
pub struct ProjectileVisualLink(#[allow(dead_code)] pub Entity);

/// Per-projectile frame cycler for an animated spritesheet kind. Holds the row's
/// source rects (read once from the manifest at spawn) and steps them on the
/// row's authored cadence, scaled by `WorldTime` so bullet-time slows the
/// animation with the sim.
#[derive(Component)]
pub struct ProjectileFrameAnim {
    frames: Vec<Rect>,
    frame_dur: f32,
    elapsed: f32,
    index: usize,
}

/// Marker on the transient charge-indicator sprite in front of the player.
#[derive(Component)]
pub struct PlayerChargeVisual;

/// Projectile sprites render just in front of the player plane.
fn projectile_z() -> f32 {
    ambition_engine_core::config::WORLD_Z_PLAYER + 2.0
}

/// One resolved, ready-to-spawn sprite for a projectile, plus the per-frame
/// behavior the refresh pass needs.
struct BuiltVisual {
    sprite: Sprite,
    anchor: Option<Anchor>,
    anim: Option<ProjectileFrameAnim>,
}

/// Bevy `Rect` for a manifest frame rect.
fn frame_rect(r: &ambition_sprite_sheet::FrameRect) -> Rect {
    Rect::from_corners(
        Vec2::new(r.x as f32, r.y as f32),
        Vec2::new((r.x + r.w) as f32, (r.y + r.h) as f32),
    )
}

/// Render size in px for the given descriptor + live body.
fn render_size(size: ProjectileRenderSize, body: Vec2, frame_aspect: f32) -> Vec2 {
    match size {
        ProjectileRenderSize::Body { min, scale } => {
            Vec2::new(body.x.max(min), body.y.max(min)) * scale
        }
        // Width fixed; height follows the source frame aspect (w/h).
        ProjectileRenderSize::FixedWidth(w) => Vec2::new(w, w / frame_aspect.max(0.0001)),
    }
}

/// Pommel-anchor pivot (normalized, Bevy y-up) for a velocity-aligned blade, read
/// from the frame's `"pommel"` manifest anchor (frame-local px). Falls back to
/// center when absent.
///
/// AMBITION_REVIEW(spatial): frame-local pixel anchor → normalized sprite
/// anchor; y is negated because the sheet is y-down while Bevy anchors are y-up.
fn pommel_anchor(rect: &ambition_sprite_sheet::FrameRect) -> Anchor {
    let (fw, fh) = (rect.w as f32, rect.h as f32);
    match rect.anchors.get("pommel") {
        Some(p) if fw > 0.0 && fh > 0.0 => {
            Anchor(Vec2::new((p.x - fw * 0.5) / fw, -(p.y - fh * 0.5) / fh))
        }
        _ => Anchor::CENTER,
    }
}

/// Z-rotation that aligns a sprite's +X axis with `vel` (world, y-down).
///
/// AMBITION_REVIEW(spatial): Bevy +Y is up, sim +Y is down — flip Y before atan2.
fn velocity_aligned_angle(vel: ambition_engine_core::Vec2) -> f32 {
    let (dx, dy) = (vel.x, -vel.y);
    if dx == 0.0 && dy == 0.0 {
        0.0
    } else {
        dy.atan2(dx)
    }
}

/// Build the sprite (and optional anchor / frame animator) for a freshly-spawned
/// projectile from its kind's art descriptor.
fn build_visual(
    kind: ProjectileVisualKind,
    kin: &BodyKinematics,
    asset_server: &AssetServer,
    sheets: &SheetRegistry,
    energy: Option<&Handle<Image>>,
) -> BuiltVisual {
    let art = kind.art();
    let body = Vec2::new(kin.size.x, kin.size.y);
    let rgba = |c: [f32; 4]| Color::srgba(c[0], c[1], c[2], c[3]);

    match art.source {
        ProjectileArtSource::EnergyTinted { rgba: c } => {
            let size = render_size(art.size, body, 1.0);
            let mut sprite = match energy.cloned() {
                Some(image) => Sprite {
                    image,
                    color: rgba(c),
                    custom_size: Some(size),
                    ..Default::default()
                },
                None => Sprite::from_color(rgba(c), size),
            };
            sprite.flip_x = kin.vel.x < 0.0;
            BuiltVisual {
                sprite,
                anchor: None,
                anim: None,
            }
        }
        ProjectileArtSource::SolidColor { rgba: c } => {
            let size = render_size(art.size, body, 1.0);
            let mut sprite = Sprite::from_color(rgba(c), size);
            sprite.flip_x = kin.vel.x < 0.0;
            BuiltVisual {
                sprite,
                anchor: None,
                anim: None,
            }
        }
        ProjectileArtSource::Image { path } => {
            let mut sprite = Sprite::from_image(asset_server.load(path));
            sprite.custom_size = Some(render_size(art.size, body, 1.0));
            BuiltVisual {
                sprite,
                anchor: None,
                anim: None,
            }
        }
        ProjectileArtSource::Sheet {
            target,
            animation,
            animate,
        } => build_sheet_visual(
            art.size,
            art.rotation,
            target,
            animation,
            animate,
            kin,
            asset_server,
            sheets,
        ),
    }
}

/// Build a spritesheet-backed visual: clip to the row's first frame (static) or
/// attach a [`ProjectileFrameAnim`] that cycles the row (animated). The image
/// path + frame rects come from the manifest, never hardcoded.
#[allow(clippy::too_many_arguments)]
fn build_sheet_visual(
    size: ProjectileRenderSize,
    rotation: ProjectileRotation,
    target: &str,
    animation: &str,
    animate: bool,
    kin: &BodyKinematics,
    asset_server: &AssetServer,
    sheets: &SheetRegistry,
) -> BuiltVisual {
    // Resolve the sheet + the requested animation row from the manifest.
    let record = sheets.get(target);
    let row = record.and_then(|rec| rec.rows.iter().find(|r| r.animation == animation));
    let Some((record, row)) = record.zip(row) else {
        // Missing sheet / row: fall back to a small magenta quad so the shot is
        // still visible (and the mistake obvious) rather than invisible.
        let size = render_size(size, Vec2::new(kin.size.x, kin.size.y), 1.0);
        return BuiltVisual {
            sprite: Sprite::from_color(Color::srgb(1.0, 0.0, 1.0), size),
            anchor: None,
            anim: None,
        };
    };
    let frames: Vec<Rect> = row.rects.iter().map(frame_rect).collect();
    let first = frames.first().copied().unwrap_or(Rect::from_corners(
        Vec2::ZERO,
        Vec2::new(record.frame_width as f32, record.frame_height as f32),
    ));
    let frame_aspect = (first.width() / first.height()).max(0.0001);
    let mut sprite = Sprite::from_image(asset_server.load(format!("sprites/{}", record.image)));
    sprite.custom_size = Some(render_size(
        size,
        Vec2::new(kin.size.x, kin.size.y),
        frame_aspect,
    ));
    sprite.rect = Some(first);

    let anchor = matches!(rotation, ProjectileRotation::VelocityAligned)
        .then(|| pommel_anchor(&row.rects[0]));

    let anim = (animate && frames.len() > 1).then(|| ProjectileFrameAnim {
        frames,
        frame_dur: row.duration_secs.max(1.0 / 1000.0),
        elapsed: 0.0,
        index: 0,
    });

    BuiltVisual {
        sprite,
        anchor,
        anim,
    }
}

/// Spawn + maintain one persistent sprite for each in-flight projectile (player
/// and enemy alike). Runs after `step_projectiles`. Art is a pure function of
/// the projectile's [`ProjectileVisualKind`]; this system never reads `owner_id`.
#[allow(clippy::too_many_arguments)]
pub fn sync_projectile_visuals(
    mut commands: Commands,
    world: Res<ambition_gameplay_core::RoomGeometry>,
    world_time: Res<ambition_gameplay_core::WorldTime>,
    gravity: GravityCtx,
    asset_server: Res<AssetServer>,
    sheets: Res<SheetRegistry>,
    game_assets: Option<Res<GameAssets>>,
    // Projectiles that don't have a visual yet get one spawned.
    new_projectiles: Query<
        (Entity, &BodyKinematics, &ProjectileVisualKind),
        (With<LiveProjectile>, Without<ProjectileVisualLink>),
    >,
    // Live bodies for the per-frame transform refresh.
    bodies: Query<&BodyKinematics, With<LiveProjectile>>,
    mut visuals: Query<
        (
            Entity,
            &VisualProjectile,
            &ProjectileVisualKind,
            Option<&mut ProjectileFrameAnim>,
            &mut Transform,
            &mut Sprite,
        ),
        With<ProjectileVisual>,
    >,
) {
    let energy = game_assets
        .as_deref()
        .and_then(|a| a.entities.get(EntitySprite::ProjectileEnergy));

    // Spawn one persistent visual per NEW projectile entity.
    for (proj_entity, kin, kind) in &new_projectiles {
        let built = build_visual(*kind, kin, &asset_server, &sheets, energy);
        let translation =
            ambition_engine_core::config::world_to_bevy(&world.0, kin.pos, projectile_z());
        let mut visual = commands.spawn((
            built.sprite,
            Transform::from_translation(translation),
            ProjectileVisual,
            *kind,
            VisualProjectile(proj_entity),
            Name::new(format!("Projectile visual: {}", kind.label())),
        ));
        if let Some(anchor) = built.anchor {
            visual.insert(anchor);
        }
        if let Some(anim) = built.anim {
            visual.insert(anim);
        }
        let visual = visual.id();
        commands
            .entity(proj_entity)
            .insert(ProjectileVisualLink(visual));
    }

    // Refresh existing visuals from their live body; despawn orphans.
    let dt = world_time.scaled_dt;
    for (visual_entity, link, kind, anim, mut transform, mut sprite) in &mut visuals {
        let Ok(kin) = bodies.get(link.0) else {
            commands.entity(visual_entity).despawn();
            continue;
        };
        transform.translation =
            ambition_engine_core::config::world_to_bevy(&world.0, kin.pos, projectile_z());

        match kind.art().rotation {
            ProjectileRotation::FlipToTravel => {
                sprite.flip_x = kin.vel.x < 0.0;
            }
            ProjectileRotation::GravityUpright => {
                transform.rotation =
                    Quat::from_rotation_z(gravity_upright_angle(gravity.dir_at(kin.pos)));
            }
            ProjectileRotation::VelocityAligned => {
                transform.rotation = Quat::from_rotation_z(velocity_aligned_angle(kin.vel));
            }
        }

        // Advance the frame animation (if any) and clip the sprite to it.
        if let Some(mut anim) = anim {
            if !anim.frames.is_empty() {
                anim.elapsed += dt;
                while anim.elapsed >= anim.frame_dur {
                    anim.elapsed -= anim.frame_dur;
                    anim.index = (anim.index + 1) % anim.frames.len();
                }
                sprite.rect = Some(anim.frames[anim.index]);
            }
        }
    }
}

/// Draw the per-player charge indicator: a growing tinted quad in front of the
/// player while the fire button is held (before a Hadouken motion commits).
/// Rebuilt each frame; player-only (it is not projectile art).
pub fn sync_projectile_charge_visuals(
    mut commands: Commands,
    world: Res<ambition_gameplay_core::RoomGeometry>,
    player_q: Query<
        (
            &BodyKinematics,
            &ambition_gameplay_core::projectile::PlayerProjectileState,
        ),
        With<ambition_gameplay_core::actor::PlayerEntity>,
    >,
    existing_charge: Query<Entity, With<PlayerChargeVisual>>,
) {
    for entity in &existing_charge {
        commands.entity(entity).despawn();
    }
    for (body, state) in &player_q {
        let Some(hold) = state.charging else {
            continue;
        };
        let tier = state.charge_tuning.tier_for_hold(hold);
        let base = ProjectileKind::Fireball.half_extent();
        let (size_mult, alpha) = match tier {
            0 => (0.7, 0.55),
            1 => (1.1, 0.78),
            _ => (1.5, 0.95),
        };
        let render_size = Vec2::new(base.x * 2.0 * size_mult, base.y * 2.0 * size_mult);
        let facing = if body.facing.abs() < f32::EPSILON {
            1.0
        } else {
            body.facing.signum()
        };
        let charge_pos = ambition_engine_core::Vec2::new(
            body.pos.x + facing * (body.size.x * 0.5 + 6.0),
            body.pos.y - body.size.y * 0.20,
        );
        commands.spawn((
            Sprite::from_color(
                Color::srgba(1.0, 0.74, 0.30, alpha),
                Vec2::new(render_size.x, render_size.y),
            ),
            Transform::from_translation(ambition_engine_core::config::world_to_bevy(
                &world.0,
                charge_pos,
                ambition_engine_core::config::WORLD_Z_PLAYER + 1.5,
            )),
            PlayerChargeVisual,
            Name::new("Player projectile charge indicator"),
        ));
    }
}
