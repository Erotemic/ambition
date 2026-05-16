//! Static world-visual spawning: blocks, water/climbable regions,
//! grid lines, loading-zone overlays, and authored `RoomObject`s.
//! `spawn_room_visuals` is the entry point called once per room
//! load.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{
    block_color, feature_color, feature_z, object_visual_kind, spawn_world_label, FeatureVisual,
    LockWallVisual, RoomVisual,
};
use crate::character_sprites::sprite_render_size;
use crate::config::{world_to_bevy, GRID_STEP, WORLD_Z_BLOCK, WORLD_Z_PLAYER};
use crate::features::FeatureVisualKind;
use crate::game_assets::{self, entity_sprite, entity_sprite_or_color, GameAssets};
use crate::physics;
use crate::rooms::{LoadingZone, LoadingZoneActivation};

pub fn spawn_room_visuals(
    commands: &mut Commands,
    world: &ae::World,
    loading_zones: &[LoadingZone],
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&GameAssets>,
) {
    spawn_grid(commands, world);
    for block in &world.blocks {
        spawn_block(commands, world, block, physics_settings, assets);
    }
    for region in &world.water_regions {
        spawn_water_region(commands, world, region);
    }
    for region in &world.climbable_regions {
        spawn_climbable_region(commands, world, region);
    }
    for zone in loading_zones {
        spawn_loading_zone(commands, world, zone, assets);
    }
    for object in &world.objects {
        spawn_room_object(commands, world, object, assets);
    }
}

/// Render a single `WaterRegion` as a tinted overlay quad. Source-
/// agnostic: any region — IntGrid `Water` or entity `WaterVolume` —
/// uses the same path. Two layers per kind:
///
/// - **Body**: a tinted rect spanning the whole region. Clear sits
///   *behind* the player so the player is visible while submerged;
///   Murky sits *in front of* the player so it actually hides what
///   is underneath.
/// - **Surface strip**: a brighter band along the top edge so the
///   water surface reads at a glance even with a flat tint.
fn spawn_water_region(commands: &mut Commands, world: &ae::World, region: &ae::WaterRegion) {
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    let (body_color, body_z) = match region.kind {
        // Cool blue, mostly transparent. Z just above blocks so the
        // floor tint shows through; player draws on top normally.
        ae::WaterKind::Clear => (Color::srgba(0.24, 0.72, 0.88, 0.32), WORLD_Z_BLOCK + 5.0),
        // Dark teal, near-opaque. Z above the player so anything
        // beneath the surface is genuinely hidden.
        ae::WaterKind::Murky => (Color::srgba(0.10, 0.20, 0.18, 0.88), WORLD_Z_PLAYER + 5.0),
    };
    commands.spawn((
        Sprite::from_color(body_color, render),
        Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
        Name::new(format!("Water body ({:?})", region.kind)),
        RoomVisual,
    ));

    // Surface strip: a brighter band 4px tall at the very top of the
    // region. The strip always renders above the body and the
    // player so the surface reads cleanly even through Murky.
    let strip_color = match region.kind {
        ae::WaterKind::Clear => Color::srgba(0.82, 0.95, 1.0, 0.85),
        ae::WaterKind::Murky => Color::srgba(0.55, 0.78, 0.62, 0.95),
    };
    let strip_h = 4.0;
    let strip_size = BVec2::new(size.x, strip_h);
    let strip_center = ae::Vec2::new(region.aabb.center().x, region.aabb.top() + strip_h * 0.5);
    commands.spawn((
        Sprite::from_color(strip_color, strip_size),
        Transform::from_translation(world_to_bevy(world, strip_center, WORLD_Z_PLAYER + 6.0)),
        Name::new(format!("Water surface ({:?})", region.kind)),
        RoomVisual,
    ));
}

/// Render a single `ClimbableRegion` as a tinted overlay quad +
/// "rung" stripes for visual rhythm. Mirror of `spawn_water_region`'s
/// shape; placeholder until proper ladder/vine/wall sprite art lands.
/// All three kinds share the same overlay shape but with kind-specific
/// tint so the player can tell at a glance what they're touching.
fn spawn_climbable_region(
    commands: &mut Commands,
    world: &ae::World,
    region: &ae::ClimbableRegion,
) {
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // Sit above blocks but below the player so the ladder reads as
    // background scenery the player climbs in front of.
    let body_z = WORLD_Z_BLOCK + 4.0;
    let (body_color, rung_color) = match region.kind {
        // Brown ladder with darker rung accents.
        ae::ClimbableKind::Ladder => (
            Color::srgba(0.76, 0.52, 0.28, 0.90),
            Color::srgba(0.45, 0.30, 0.15, 1.0),
        ),
        // Green vine with yellow-green leaf accents.
        ae::ClimbableKind::Vine => (
            Color::srgba(0.37, 0.64, 0.32, 0.85),
            Color::srgba(0.65, 0.85, 0.40, 1.0),
        ),
        // Tan/sand climbable wall, no rung accents.
        ae::ClimbableKind::Wall => (
            Color::srgba(0.61, 0.48, 0.29, 0.80),
            Color::srgba(0.45, 0.35, 0.20, 0.0), // alpha=0 = no rungs
        ),
    };
    commands.spawn((
        Sprite::from_color(body_color, render),
        Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
        Name::new(format!("Climbable body ({:?})", region.kind)),
        RoomVisual,
    ));

    // Add rung stripes spaced every 16 px on the y axis. Skipped for
    // Wall (rung_color alpha=0). Quick visual rhythm so a tall ladder
    // doesn't look like a flat colored block.
    if rung_color.alpha() > 0.0 {
        let rung_h = 3.0;
        let rung_size = BVec2::new(size.x, rung_h);
        let mut y = region.aabb.top() + 8.0;
        while y < region.aabb.bottom() - 4.0 {
            let center = ae::Vec2::new(region.aabb.center().x, y);
            commands.spawn((
                Sprite::from_color(rung_color, rung_size),
                Transform::from_translation(world_to_bevy(world, center, body_z + 0.5)),
                Name::new(format!("Climbable rung ({:?})", region.kind)),
                RoomVisual,
            ));
            y += 16.0;
        }
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
    assets: Option<&GameAssets>,
) {
    let size = block.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // IntGrid-derived blocks (named "ldtk *" by `int_grid_value_to_block`)
    // can be arbitrary aspect ratios (1904×32 floors, 48×240 pillars, …).
    // Stretching the single 128-px entity-art textures across those
    // smears the texture's internal structure into a false repeat.
    // Solution: tiled 32×32 textures (one per BlockKind) repeated via
    // `Sprite::image_mode = Tiled` so the texture renders at native
    // pixel scale and TILES to fill `custom_size` — exactly what a
    // long stone floor or tall pillar wants.
    //
    // Falls back to a colored quad when the tile asset is missing
    // (no-asset mode, missing file). Authored entity-derived blocks
    // (e.g. authored Solid rectangles outside the IntGrid layer) keep
    // the entity-art path because their footprints match the texture
    // aspect ratio.
    let is_intgrid_block = block.name.starts_with("ldtk ");
    let sprite = if is_intgrid_block {
        let tile_handle = assets
            .and_then(|a| {
                game_assets::block_tile_sprite(block.kind).and_then(|key| a.entities.get(key))
            })
            .cloned();
        match tile_handle {
            Some(image) => Sprite {
                image,
                custom_size: Some(render),
                image_mode: bevy::sprite::SpriteImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    stretch_value: 1.0,
                },
                ..Default::default()
            },
            None => Sprite::from_color(block_color(block.kind), render),
        }
    } else {
        match assets {
            Some(a) => entity_sprite_or_color(
                a,
                game_assets::block_sprite(block.kind),
                render,
                block_color(block.kind),
            ),
            None => Sprite::from_color(block_color(block.kind), render),
        }
    };
    commands.spawn((
        sprite,
        Transform::from_translation(world_to_bevy(world, block.aabb.center(), WORLD_Z_BLOCK)),
        Name::new(format!("Block: {}", block.name)),
        RoomVisual,
    ));
    physics::spawn_static_collider_for_block(commands, world, block, physics_settings);
}

pub fn spawn_loading_zone(
    commands: &mut Commands,
    world: &ae::World,
    zone: &LoadingZone,
    assets: Option<&GameAssets>,
) {
    let size = zone.aabb.half_size() * 2.0;
    let fallback_color = match zone.activation {
        LoadingZoneActivation::EdgeExit => Color::srgba(0.20, 0.95, 1.0, 0.22),
        LoadingZoneActivation::Door => Color::srgba(1.0, 0.72, 0.18, 0.46),
        // Walk-through portal: green tint to distinguish from edge
        // exits while still reading as "step in and go."
        LoadingZoneActivation::Walk => Color::srgba(0.40, 1.00, 0.55, 0.30),
    };
    let render = BVec2::new(size.x, size.y);
    let sprite = match assets {
        Some(a) => entity_sprite(
            a,
            game_assets::loading_zone_sprite(zone.activation),
            render,
            fallback_color,
        ),
        None => Sprite::from_color(fallback_color, render),
    };
    commands.spawn((
        sprite,
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

pub fn spawn_room_object(
    commands: &mut Commands,
    world: &ae::World,
    object: &ae::RoomObject,
    assets: Option<&GameAssets>,
) {
    if let Some(kind) = object_visual_kind(&object.kind) {
        let size = object.aabb.half_size() * 2.0;
        let render = BVec2::new(size.x, size.y);
        let entity_key = game_assets::entity_sprite_for_room_object(&object.kind);
        let sprite = match assets {
            Some(a) => entity_sprite_or_color(a, entity_key, render, feature_color(kind, false)),
            None => Sprite::from_color(feature_color(kind, false), render),
        };
        commands.spawn((
            sprite,
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
            // NPCs render with `collision_scale > 1`, so the sprite extends
            // past the AABB top. When a character sheet is registered for
            // this NPC, lift the label to clear the sprite's actual top by
            // 12px; otherwise fall back to the AABB-based 22px gap (chests
            // + anonymous NPCs render inside their AABB).
            let half_h = object.aabb.half_size().y;
            let mut label_dy = -half_h - 22.0;
            if matches!(kind, FeatureVisualKind::Npc) {
                if let Some(ch) = assets.and_then(|a| a.characters.npc_asset_for_name(&object.name))
                {
                    let collision = object.aabb.half_size() * 2.0;
                    let render_h = sprite_render_size(ch.spec, collision).y;
                    // World y is y-down: sprite top relative to AABB centre
                    // is `half_h - render_h` (negative when render exceeds
                    // collision). `min` so a registered sheet only ever
                    // pushes the label further up.
                    let sprite_top_dy = half_h - render_h;
                    label_dy = label_dy.min(sprite_top_dy - 12.0);
                }
            }
            spawn_world_label(
                commands,
                world,
                object.aabb.center() + ae::Vec2::new(0.0, label_dy),
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

/// Reconcile `LockWallVisual` Bevy entities against the encounter-
/// driven `lockwall:*` blocks in `world.blocks`. Spawn a sprite for
/// any new lock wall, despawn entities whose backing block has been
/// removed (encounter cleared / failed).
///
/// Without this system the lock wall has collision (the engine reads
/// `world.blocks` every frame) but no rendered tile — the player
/// bumps into an invisible barrier. The dedicated `LockWallTile`
/// asset keeps the visual distinct from regular solid walls so the
/// "this just slammed shut" beat reads at a glance.
pub fn sync_lock_wall_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    assets: Option<Res<GameAssets>>,
    existing: Query<(Entity, &LockWallVisual)>,
) {
    use bevy::math::Vec2 as BVec2;

    // Index existing visuals by their backing block name so we can
    // diff against the world snapshot in linear time.
    let mut existing_by_name: std::collections::HashMap<String, Entity> =
        std::collections::HashMap::new();
    for (entity, visual) in &existing {
        existing_by_name.insert(visual.block_name.clone(), entity);
    }

    // Pass 1: spawn a visual for any lockwall block that doesn't have
    // one yet. Mark consumed names so the despawn pass below leaves
    // them alone.
    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for block in &world.0.blocks {
        if !block.name.starts_with("lockwall:") {
            continue;
        }
        if existing_by_name.contains_key(&block.name) {
            consumed.insert(block.name.clone());
            continue;
        }
        let size = block.aabb.half_size() * 2.0;
        let render = BVec2::new(size.x, size.y);
        // Bright purple fallback when no asset is loaded — distinct
        // from the standard solid-block fallback so a missing tile
        // is obvious in playtest.
        let fallback = Color::srgba(0.65, 0.20, 0.85, 0.92);
        let sprite = match assets.as_deref() {
            Some(a) => entity_sprite_or_color(
                a,
                Some(game_assets::EntitySprite::LockWallTile),
                render,
                fallback,
            ),
            None => Sprite::from_color(fallback, render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(
                &world.0,
                block.aabb.center(),
                // Sit just above the regular block layer so a lock
                // wall reads on top of any floor/wall art it overlaps.
                WORLD_Z_BLOCK + 4.0,
            )),
            Name::new(format!("LockWall: {}", block.name)),
            LockWallVisual {
                block_name: block.name.clone(),
            },
            RoomVisual,
        ));
        consumed.insert(block.name.clone());
    }

    // Pass 2: despawn visuals whose block disappeared (encounter
    // cleared / failed → `sync_lock_walls` removed the block).
    for (name, entity) in &existing_by_name {
        if !consumed.contains(name) {
            commands.entity(*entity).despawn();
        }
    }
}
