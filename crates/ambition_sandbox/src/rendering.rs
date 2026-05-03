//! Bevy visual synchronization for engine state.
//!
//! This module owns the render-only component tags and visual sync systems.
//! Gameplay code should mutate `SandboxRuntime`; this module mirrors that state
//! into Bevy transforms/sprites.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::character_sprites::{
    CharacterAnim, CharacterAnimator, CharacterSpriteAssets,
};
use crate::config::{world_to_bevy, GRID_STEP, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER};
use crate::features::FeatureVisualKind;
use crate::physics;
use crate::rooms::{LoadingZone, LoadingZoneActivation};

#[derive(Resource)]
pub struct SceneEntities {
    pub player: Entity,
    pub hud: Entity,
}

#[derive(Component)]
pub struct PlayerVisual;

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct RoomVisual;

#[derive(Component)]
pub struct FeatureVisual {
    pub id: String,
}

#[derive(Component)]
pub struct HealthOverlayVisual;

pub fn sync_health_overlays(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    developer_tools: Res<crate::dev_tools::DeveloperTools>,
    overlays: Query<Entity, With<HealthOverlayVisual>>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }

    if !runtime.debug_enabled() || !developer_tools.show_health_bars {
        return;
    }

    spawn_health_overlay(
        &mut commands,
        &world.0,
        "player",
        runtime.player.aabb(),
        runtime.player_health,
        Color::srgba(0.30, 0.92, 1.00, 0.96),
    );

    for enemy in &runtime.features.enemies {
        if enemy.alive {
            let color = if enemy.archetype.is_sandbag() {
                Color::srgba(1.00, 0.66, 0.24, 0.96)
            } else {
                Color::srgba(1.00, 0.20, 0.22, 0.96)
            };
            spawn_health_overlay(
                &mut commands,
                &world.0,
                &enemy.name,
                enemy.aabb(),
                enemy.health,
                color,
            );
        }
    }
    for boss in &runtime.features.bosses {
        if boss.alive {
            spawn_health_overlay(
                &mut commands,
                &world.0,
                &boss.name,
                boss.aabb(),
                boss.health,
                Color::srgba(1.00, 0.32, 0.92, 0.96),
            );
        }
    }
    for breakable in &runtime.features.breakables {
        if !breakable.broken() {
            spawn_health_overlay(
                &mut commands,
                &world.0,
                &breakable.name,
                breakable.aabb(),
                breakable.breakable.health,
                Color::srgba(1.00, 0.72, 0.24, 0.96),
            );
        }
    }
}

fn spawn_health_overlay(
    commands: &mut Commands,
    world: &ae::World,
    name: &str,
    aabb: ae::Aabb,
    health: ae::Health,
    fill_color: Color,
) {
    let width = aabb.width().max(56.0);
    let height = 7.0;
    let y = aabb.top() - 26.0;
    let center_x = aabb.center().x;
    let left = center_x - width * 0.5;
    let ratio = health.ratio().clamp(0.0, 1.0);
    let fill_w = width * ratio;
    let text = format!("{}/{}", health.current.max(0), health.max);

    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.02, 0.03, 0.05, 0.86),
            BVec2::new(width + 5.0, height + 5.0),
        ),
        Transform::from_translation(world_to_bevy(
            world,
            ae::Vec2::new(center_x, y),
            WORLD_Z_PLAYER + 12.0,
        )),
        Name::new(format!("Health bar bg: {name}")),
        HealthOverlayVisual,
    ));
    if fill_w > 0.5 {
        commands.spawn((
            Sprite::from_color(fill_color, BVec2::new(fill_w, height)),
            Transform::from_translation(world_to_bevy(
                world,
                ae::Vec2::new(left + fill_w * 0.5, y),
                WORLD_Z_PLAYER + 13.0,
            )),
            Name::new(format!("Health bar fill: {name}")),
            HealthOverlayVisual,
        ));
    }
    commands.spawn((
        Text2d::new(text),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgba(0.96, 0.98, 1.0, 0.98)),
        Transform::from_translation(world_to_bevy(
            world,
            ae::Vec2::new(center_x, y - 13.0),
            WORLD_Z_PLAYER + 14.0,
        )),
        Name::new(format!("Health label: {name}")),
        HealthOverlayVisual,
    ));
}

pub fn sync_visuals(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut player_query: Query<(&mut Transform, &mut Sprite), With<PlayerVisual>>,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
) {
    if let Ok((mut transform, mut sprite)) = player_query.get_mut(entities.player) {
        transform.translation = world_to_bevy(&world.0, runtime.player.pos, WORLD_Z_PLAYER);
        if sprite.texture_atlas.is_none() {
            // Colored-rectangle fallback: stretch to the collision-box size
            // and tint by flash. Textured sprites keep their authored size
            // and are tinted in the animation system instead.
            sprite.custom_size = Some(BVec2::new(runtime.player.size.x, runtime.player.size.y));
            let alpha = if runtime.flash_timer > 0.0 { 0.72 } else { 1.0 };
            sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = runtime.features.view(&visual.id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = world_to_bevy(&world.0, view.pos, feature_z(view.kind));
        if sprite.texture_atlas.is_none() {
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = feature_color(view.kind, view.flash);
        }
        *visibility = if view.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Replace the colored-rectangle sprite on enemy/sandbag entities with the
/// goblin sprite-sheet sprite once the asset is available. Newly-spawned
/// feature visuals (initial setup or room transitions) are picked up here.
pub fn upgrade_enemy_sprites(
    mut commands: Commands,
    sprites: Option<Res<CharacterSpriteAssets>>,
    runtime: Res<crate::SandboxRuntime>,
    new_features: Query<(Entity, &FeatureVisual), Without<CharacterAnimator>>,
) {
    let Some(sprites) = sprites else {
        return;
    };
    let Some(goblin) = &sprites.goblin else {
        return;
    };
    for (entity, visual) in &new_features {
        let Some(view) = runtime.features.view(&visual.id) else {
            continue;
        };
        if !matches!(
            view.kind,
            FeatureVisualKind::Enemy | FeatureVisualKind::Sandbag
        ) {
            continue;
        }
        let mut sprite = Sprite::from_atlas_image(
            goblin.texture.clone(),
            bevy::image::TextureAtlas {
                layout: goblin.layout.clone(),
                index: goblin.spec.flat_index(CharacterAnim::Idle, 0),
            },
        );
        sprite.custom_size = Some(goblin.render_size);
        commands
            .entity(entity)
            .insert((sprite, CharacterAnimator::new(goblin.spec)));
    }
}

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
pub fn animate_player(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut query: Query<(&mut Sprite, &mut CharacterAnimator), With<PlayerVisual>>,
) {
    let Ok((mut sprite, mut animator)) = query.get_mut(entities.player) else {
        return;
    };
    let anim = crate::character_sprites::pick_player_anim(&runtime);
    animator.request(anim);
    let index = animator.tick(time.delta_secs());
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    }
    sprite.flip_x = runtime.player.facing < 0.0;
    // Keep the textured sprite at full opacity by default, with a subtle
    // red tint when invulnerable / hit so the existing flash signal still
    // reads. Tints multiply the texture color, so values below 1.0 darken
    // the channel.
    sprite.color = if runtime.flash_timer > 0.0 {
        Color::srgba(1.0, 0.55, 0.55, 1.0)
    } else {
        Color::WHITE
    };
}

/// Drive enemy sprite animation, atlas index, and facing flip.
pub fn animate_enemies(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    mut query: Query<
        (&FeatureVisual, &mut Sprite, &mut CharacterAnimator),
        Without<PlayerVisual>,
    >,
) {
    let dt = time.delta_secs();
    for (visual, mut sprite, mut animator) in &mut query {
        let Some(state) = runtime.features.enemy_anim_state(&visual.id) else {
            continue;
        };
        let anim = crate::character_sprites::pick_enemy_anim(state);
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.flip_x = state.facing < 0.0;
        sprite.color = if state.hit_flash {
            Color::srgba(1.0, 0.55, 0.55, 1.0)
        } else if state.attack_active || state.attack_windup {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}

pub fn spawn_room_visuals(
    commands: &mut Commands,
    world: &ae::World,
    loading_zones: &[LoadingZone],
    physics_settings: physics::PhysicsSandboxSettings,
) {
    spawn_grid(commands, world);
    for block in &world.blocks {
        spawn_block(commands, world, block, physics_settings);
    }
    for zone in loading_zones {
        spawn_loading_zone(commands, world, zone);
    }
    for object in &world.objects {
        spawn_room_object(commands, world, object);
    }
}

pub fn spawn_grid(commands: &mut Commands, world: &ae::World) {
    let grid_color = Color::srgba(0.12, 0.15, 0.22, 0.28);
    let mut x = 0.0;
    while x <= world.size.x {
        let center = ae::Vec2::new(x, world.size.y * 0.5);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(1.0, world.size.y)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
            RoomVisual,
        ));
        x += GRID_STEP;
    }
    let mut y = 0.0;
    while y <= world.size.y {
        let center = ae::Vec2::new(world.size.x * 0.5, y);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(world.size.x, 1.0)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
            RoomVisual,
        ));
        y += GRID_STEP;
    }
}

pub fn spawn_block(
    commands: &mut Commands,
    world: &ae::World,
    block: &ae::Block,
    physics_settings: physics::PhysicsSandboxSettings,
) {
    let size = block.aabb.half_size() * 2.0;
    commands.spawn((
        Sprite::from_color(block_color(block.kind), BVec2::new(size.x, size.y)),
        Transform::from_translation(world_to_bevy(world, block.aabb.center(), WORLD_Z_BLOCK)),
        Name::new(format!("Block: {}", block.name)),
        RoomVisual,
    ));
    physics::spawn_static_collider_for_block(commands, world, block, physics_settings);
}

pub fn spawn_loading_zone(commands: &mut Commands, world: &ae::World, zone: &LoadingZone) {
    let size = zone.aabb.half_size() * 2.0;
    let color = match zone.activation {
        LoadingZoneActivation::EdgeExit => Color::srgba(0.20, 0.95, 1.0, 0.22),
        LoadingZoneActivation::Door => Color::srgba(1.0, 0.72, 0.18, 0.46),
    };
    commands.spawn((
        Sprite::from_color(color, BVec2::new(size.x, size.y)),
        Transform::from_translation(world_to_bevy(
            world,
            zone.aabb.center(),
            WORLD_Z_BLOCK + 6.0,
        )),
        Name::new(format!("Loading zone: {}", zone.name)),
        RoomVisual,
    ));
    let label_pos = zone.aabb.center() + ae::Vec2::new(0.0, -zone.aabb.half_size().y - 18.0);
    spawn_world_label(commands, world, label_pos, &zone.name, 13.0);
}

pub fn block_color(kind: ae::BlockKind) -> Color {
    match kind {
        ae::BlockKind::Solid => Color::srgba(0.25, 0.28, 0.36, 1.0),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        } => Color::srgba(0.32, 0.20, 0.72, 0.88),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard,
        } => Color::srgba(0.52, 0.14, 0.80, 0.96),
        ae::BlockKind::OneWay => Color::srgba(0.36, 0.43, 0.62, 0.92),
        ae::BlockKind::Hazard => Color::srgba(0.96, 0.18, 0.26, 0.92),
        ae::BlockKind::PogoOrb => Color::srgba(0.30, 0.95, 0.64, 0.95),
        ae::BlockKind::Rebound { .. } => Color::srgba(1.0, 0.60, 0.20, 0.95),
    }
}

/// Follow the player in rooms larger than the window.
///
/// The simulation uses top-left world coordinates, while Bevy renders around a
/// centered camera. We convert the player to Bevy coordinates, then clamp the
/// camera center so the player can scroll through large rooms without showing
/// outside the generated level bounds. Small rooms remain centered.
pub fn camera_follow(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    developer_tools: Res<crate::dev_tools::DeveloperTools>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut Transform, &mut Projection), (With<Camera>, Without<PlayerVisual>)>,
) {
    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    let camera_scale = if developer_tools.overview_camera {
        overview_scale
    } else {
        1.0
    };

    let target = if developer_tools.overview_camera {
        // AMBITION_REVIEW(spatial): overview centers the composed active area, not
        // individual LDtk chunks. If active areas become sparse, switch this from
        // bounding-box center to a validated camera overview region.
        world_to_bevy(&world.0, world.0.size * 0.5, 0.0)
    } else {
        world_to_bevy(&world.0, runtime.player.pos, 0.0)
    };

    // Use the actual logical window size so resized, borderless, and fullscreen
    // windows clamp the camera correctly. In overview mode the orthographic
    // scale expands the effective view so large stitched areas can be inspected.
    let (view_w, view_h) = windows
        .single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((
            crate::config::WINDOW_W as f32,
            crate::config::WINDOW_H as f32,
        ));
    let half_view_w = view_w * camera_scale * 0.5;
    let half_view_h = view_h * camera_scale * 0.5;
    let min_x = -world.0.size.x * 0.5 + half_view_w;
    let max_x = world.0.size.x * 0.5 - half_view_w;
    let min_y = -world.0.size.y * 0.5 + half_view_h;
    let max_y = world.0.size.y * 0.5 - half_view_h;

    let x = if min_x <= max_x {
        target.x.clamp(min_x, max_x)
    } else {
        0.0
    };
    let y = if min_y <= max_y {
        target.y.clamp(min_y, max_y)
    } else {
        0.0
    };

    for (mut transform, mut projection) in &mut query {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = camera_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
    }
}

pub fn spawn_room_object(commands: &mut Commands, world: &ae::World, object: &ae::RoomObject) {
    if let Some(kind) = object_visual_kind(&object.kind) {
        let size = object.aabb.half_size() * 2.0;
        commands.spawn((
            Sprite::from_color(feature_color(kind, false), BVec2::new(size.x, size.y)),
            Transform::from_translation(world_to_bevy(
                world,
                object.aabb.center(),
                feature_z(kind),
            )),
            Name::new(format!("Room object: {}", object.name)),
            FeatureVisual {
                id: object.id.clone(),
            },
            RoomVisual,
        ));
        if matches!(kind, FeatureVisualKind::Npc | FeatureVisualKind::Chest) {
            spawn_world_label(
                commands,
                world,
                object.aabb.center() + ae::Vec2::new(0.0, -object.aabb.half_size().y - 22.0),
                &object.name,
                14.0,
            );
        }
    } else if let ae::RoomObjectKind::DebugLabel(label) = &object.kind {
        spawn_world_label(commands, world, label.position, &label.text, 14.0);
    } else if let ae::RoomObjectKind::DestinationLabel(label) = &object.kind {
        spawn_world_label(commands, world, label.position, &label.text(), 14.0);
    }
}

fn object_visual_kind(kind: &ae::RoomObjectKind) -> Option<FeatureVisualKind> {
    match kind {
        ae::RoomObjectKind::DamageVolume(_) => Some(FeatureVisualKind::Hazard),
        ae::RoomObjectKind::Pickup(_) => Some(FeatureVisualKind::Pickup),
        ae::RoomObjectKind::Chest(_) => Some(FeatureVisualKind::Chest),
        ae::RoomObjectKind::Breakable(_) => Some(FeatureVisualKind::Breakable),
        ae::RoomObjectKind::Interactable(interactable)
            if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) =>
        {
            Some(FeatureVisualKind::Npc)
        }
        ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom(name))
            if name.starts_with("sandbag_") =>
        {
            Some(FeatureVisualKind::Sandbag)
        }
        ae::RoomObjectKind::EnemySpawn(_) => Some(FeatureVisualKind::Enemy),
        ae::RoomObjectKind::BossSpawn(_) => Some(FeatureVisualKind::Boss),
        _ => None,
    }
}

fn feature_z(kind: FeatureVisualKind) -> f32 {
    match kind {
        FeatureVisualKind::Hazard => WORLD_Z_BLOCK + 8.0,
        FeatureVisualKind::Breakable => WORLD_Z_BLOCK + 5.0,
        FeatureVisualKind::Pickup => WORLD_Z_DUMMY + 4.0,
        FeatureVisualKind::Chest => WORLD_Z_DUMMY + 3.0,
        FeatureVisualKind::Npc => WORLD_Z_DUMMY + 2.0,
        FeatureVisualKind::Enemy => WORLD_Z_DUMMY + 1.0,
        FeatureVisualKind::Sandbag => WORLD_Z_DUMMY + 1.0,
        FeatureVisualKind::Boss => WORLD_Z_DUMMY + 1.0,
    }
}

fn feature_color(kind: FeatureVisualKind, flash: bool) -> Color {
    if flash {
        return Color::srgba(1.0, 1.0, 1.0, 1.0);
    }
    match kind {
        FeatureVisualKind::Hazard => Color::srgba(0.98, 0.12, 0.22, 0.94),
        FeatureVisualKind::Enemy => Color::srgba(0.93, 0.34, 0.28, 0.96),
        FeatureVisualKind::Sandbag => Color::srgba(0.78, 0.62, 0.42, 0.96),
        FeatureVisualKind::Boss => Color::srgba(0.78, 0.20, 0.92, 0.96),
        FeatureVisualKind::Breakable => Color::srgba(0.62, 0.42, 0.24, 0.96),
        FeatureVisualKind::Chest => Color::srgba(1.0, 0.74, 0.22, 0.96),
        FeatureVisualKind::Pickup => Color::srgba(0.42, 1.0, 0.74, 0.96),
        FeatureVisualKind::Npc => Color::srgba(0.42, 0.78, 1.0, 0.96),
    }
}

fn spawn_world_label(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    text: &str,
    font_size: f32,
) {
    commands.spawn((
        Text2d::new(text.to_string()),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(Color::srgba(0.86, 0.94, 1.0, 0.94)),
        Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_PLAYER + 8.0)),
        Name::new(format!("World label: {text}")),
        RoomVisual,
    ));
}
