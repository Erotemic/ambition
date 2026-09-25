//! Static world-visual spawning: blocks, water/climbable regions,
//! grid lines, loading-zone overlays, and authored `RoomObject`s.
//! `spawn_room_visuals` is the entry point called once per room
//! load.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::label_layout::WorldLabelFamily;
use super::nameplates::DoorNameplateSource;
use super::primitives::{
    block_color, feature_color, feature_z, spawn_world_label, BlockArt, BlockVisual, FeatureVisual,
    LockWallVisual, PropVisual, RoomVisual,
};
use ambition_platformer2d_core::config::{world_to_bevy, GRID_STEP, WORLD_Z_BLOCK, WORLD_Z_PLAYER};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::{LoadingZone, LoadingZoneActivation, PropDraw, PropSpec};
use ambition_sprite_sheet::character::{
    build_character_presentation, build_character_presentation_with_render_size, feet_anchor_for,
    CharacterAnimator,
};
use ambition_sprite_sheet::game_assets::{self, entity_sprite, entity_sprite_or_color, GameAssets};

/// Presentation consumer of [`ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested`].
///
/// The sim (sandbox reset) emits the request after it changes the active room.
/// This system reads the active room from [`RoomSet`] and rebuilds its static
/// visuals and parallax. The spawn stays on the render side, so the sim does not
/// import the render layer. A headless build does not run this system.
pub fn respawn_room_visuals_on_request(
    mut requests: MessageReader<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>,
    mut commands: Commands,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    physics_settings: Res<ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    if requests.is_empty() {
        return;
    }
    requests.clear();
    let spec = room_set.active_spec();
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    super::spawn_parallax_layers(
        &mut commands,
        session_scope,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        &mut commands,
        session_scope,
        spec,
        *physics_settings,
        assets.as_deref(),
    );
}

pub fn spawn_room_visuals(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    spec: &ambition_platformer2d_world::rooms::RoomSpec,
    physics_settings: ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings,
    assets: Option<&GameAssets>,
) {
    let world = &spec.world;
    spawn_grid(commands, session_scope, world);
    spawn_surface_chain_visuals(commands, session_scope, world);
    for block in &world.blocks {
        spawn_block(
            commands,
            session_scope,
            world,
            block,
            physics_settings,
            assets,
        );
    }
    for region in &world.water_regions {
        spawn_water_region(commands, session_scope, world, region);
    }
    for region in &world.climbable_regions {
        spawn_climbable_region(commands, session_scope, world, region);
    }
    for zone in &spec.loading_zones {
        spawn_loading_zone(commands, session_scope, world, zone, assets);
    }
    // Per-family authored visuals. Each family carries an `Authored<T>`
    // payload; `spawn_authored_visual` builds the sprite and label.
    // Hazards come through the `placements` channel. The visual needs only the
    // footprint and the hazard sprite, so a minimal `HazardVolumeSpec` is enough.
    for record in &spec.placements {
        if let ambition_entity_catalog::placements::PlacementSchema::Hazard(hazard) = &record.schema
        {
            let authored = ambition_platformer2d_world::rooms::Authored {
                id: record.id.as_str().to_string(),
                name: record.name.clone(),
                aabb: record.aabb,
                payload: ambition_platformer2d_world::rooms::HazardVolumeSpec::new(hazard.damage),
            };
            spawn_authored_hazard(commands, session_scope, world, &authored, assets);
        }
    }
    // Pickups come through the `placements` channel.
    for record in &spec.placements {
        if let ambition_entity_catalog::placements::PlacementSchema::Pickup(pickup) = &record.schema
        {
            // A pickup may author an animated sheet (a spinning ring). When it
            // resolves to a prop asset, bind it as a looping character sheet;
            // otherwise use the static per-kind sprite.
            let animated = pickup
                .sprite
                .as_deref()
                .and_then(|kind| assets.and_then(|a| a.characters.prop_asset_for_kind(kind)));
            if let Some(asset) = animated {
                spawn_animated_pickup(
                    commands,
                    session_scope,
                    world,
                    record.id.as_str(),
                    &record.name,
                    record.aabb,
                    asset,
                );
            } else {
                spawn_authored_basic(
                    commands,
                    session_scope,
                    world,
                    record.id.as_str(),
                    &record.name,
                    record.aabb,
                    FeatureVisualKind::Pickup,
                    game_assets::entity_sprite_for_pickup(pickup),
                    assets,
                );
            }
        }
    }
    // Chests come through the `placements` channel.
    for record in &spec.placements {
        if let ambition_entity_catalog::placements::PlacementSchema::Chest(chest) = &record.schema {
            let authored = ambition_platformer2d_world::rooms::Authored {
                id: record.id.as_str().to_string(),
                name: record.name.clone(),
                aabb: record.aabb,
                payload: chest.clone(),
            };
            spawn_authored_chest(commands, session_scope, world, &authored, assets);
        }
    }
    // Breakables come through the `placements` channel.
    for record in &spec.placements {
        if let ambition_entity_catalog::placements::PlacementSchema::Breakable(breakable) =
            &record.schema
        {
            spawn_authored_basic(
                commands,
                session_scope,
                world,
                record.id.as_str(),
                &record.name,
                record.aabb,
                FeatureVisualKind::Breakable,
                game_assets::entity_sprite_for_breakable(breakable),
                assets,
            );
        }
    }
    for enemy in &spec.enemy_spawns {
        // One actor kind: the actor sprite-upgrade fallback (keyed off
        // `is_sandbag`) picks the sandbag or enemy look.
        let kind = FeatureVisualKind::Actor;
        // ADR 0020: a mount and its rider are separate `EnemySpawn`s (linked by
        // `mounted_on`), so each renders through the single-actor path.
        spawn_authored_basic(
            commands,
            session_scope,
            world,
            &enemy.id,
            &enemy.name,
            enemy.aabb,
            kind,
            game_assets::entity_sprite_for_enemy(&enemy.payload.brain),
            assets,
        );
    }
    for boss in &spec.boss_spawns {
        spawn_authored_basic(
            commands,
            session_scope,
            world,
            &boss.id,
            &boss.name,
            boss.aabb,
            FeatureVisualKind::Actor,
            game_assets::entity_sprite_for_boss(&boss.payload),
            assets,
        );
    }
    // Interactables come through the `placements` channel.
    for record in &spec.placements {
        if let ambition_entity_catalog::placements::PlacementSchema::Interactable(spec_i) =
            &record.schema
        {
            let authored = ambition_platformer2d_world::rooms::Authored {
                id: record.id.as_str().to_string(),
                name: record.name.clone(),
                aabb: record.aabb,
                payload: spec_i.clone(),
            };
            spawn_authored_interactable(commands, session_scope, world, &authored, assets);
        }
    }
    for (index, label) in spec.debug_labels.iter().enumerate() {
        // Authored signage carries no id of its own, so the identity is
        // positional. It only has to be stable within a room load and unique
        // across families — the placement pass keys on it.
        spawn_world_label(
            commands,
            session_scope,
            world,
            format!("signage:{index}:{}", label.id),
            WorldLabelFamily::Signage,
            label.payload.position,
            &label.payload.text,
            14.0,
        );
    }
    for prop in &spec.props {
        spawn_room_prop(commands, session_scope, world, prop, assets);
    }
}

/// Render size and anchor for a prop's sprite, by the kind of thing it is.
///
/// Scenery scales the frame so the sheet's art lands on the collider, which
/// leaves only the crop's transparent margin. Built world takes the box exactly,
/// centred, because a feet anchor would slide a block off its surface.
pub(crate) fn prop_sprite_geometry(
    draw: PropDraw,
    spec: &ambition_sprite_sheet::character::CharacterSheetSpec,
    collision: BVec2,
) -> (BVec2, Anchor) {
    if draw.fills_box() {
        // Built world lines up with the geometry, to the pixel.
        (collision, Anchor::CENTER)
    } else {
        (
            ambition_sprite_sheet::character::sprite_render_size(spec, collision),
            feet_anchor_for(spec, collision),
        )
    }
}

/// Build the sprite, anchor and animator for a prop sheet at its collision size.
pub(crate) fn prop_sprite_bundle(
    draw: PropDraw,
    flip_y: bool,
    asset: &ambition_sprite_sheet::character::CharacterSpriteAsset,
    collision: BVec2,
) -> (Sprite, Anchor, CharacterAnimator) {
    let (render_size, anchor) = prop_sprite_geometry(draw, &asset.spec, collision);
    let (mut sprite, anchor, animator) =
        build_character_presentation_with_render_size(asset, render_size, anchor);
    // Which way the prop points is authored data, not a second sheet.
    sprite.flip_y = flip_y;
    (sprite, anchor, animator)
}

/// Spawn the visual entity for one [`PropSpec`]. Falls back to a coloured
/// rectangle when the prop's `kind` is unknown or its asset has not loaded.
///
/// Always inserts `RoomVisual` (so the room swap despawns it) and
/// `PropVisual { id, kind, name, size }` (for the prop-anim tick, debug
/// overlays, and per-name presentation systems). Render does not insert the
/// sim's `FeatureName`.
pub fn spawn_room_prop(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    prop: &PropSpec,
    assets: Option<&GameAssets>,
) {
    // Decorative props use the actor placeholder kind for their z/colour
    // fallback; only the z is read. A dedicated neutral kind would be cleaner.
    let kind = FeatureVisualKind::Actor;
    // Built world draws in front, so a body inside it is hidden; scenery sits
    // behind the cast.
    let z = if prop.draw.occludes_bodies() {
        WORLD_Z_PLAYER + 1.0
    } else {
        feature_z(kind)
    };
    let translation = world_to_bevy(world, prop.pos, z);
    let collision = BVec2::new(prop.size.x, prop.size.y);

    let mut entity = commands.spawn_session_scoped(
        session_scope,
        (
            Transform::from_translation(translation),
            Name::new(format!("Prop: {}", prop.name)),
            RoomVisual,
            PropVisual {
                id: prop.id.clone(),
                kind: prop.kind.clone(),
                name: prop.name.clone(),
                size: BVec2::new(prop.size.x, prop.size.y),
                draw: prop.draw,
                flip_y: prop.flip_y,
            },
        ),
    );

    if let Some(asset) = assets.and_then(|a| a.characters.prop_asset_for_kind(&prop.kind)) {
        entity.insert(prop_sprite_bundle(prop.draw, prop.flip_y, asset, collision));
    } else {
        // Fallback: a translucent placeholder rectangle, so authors see a
        // marker for unregistered prop kinds.
        entity.insert(Sprite::from_color(
            Color::srgba(0.55, 0.45, 0.85, 0.55),
            collision,
        ));
    }
}

/// Render a single `WaterRegion` as a tinted overlay quad. Any region source
/// (IntGrid `Water` or entity `WaterVolume`) uses this path. Two layers:
///
/// - Body: a tinted rect over the whole region. Clear sits behind the player
///   so the player stays visible; Murky sits in front and hides what is below.
/// - Surface strip: a brighter band along the top edge.
fn spawn_water_region(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    region: &ae::WaterRegion,
) {
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    let (body_color, body_z) = match region.kind {
        // Cool blue, mostly transparent, just above blocks.
        ae::WaterKind::Clear => (Color::srgba(0.24, 0.72, 0.88, 0.32), WORLD_Z_BLOCK + 5.0),
        // Dark teal, near-opaque, above the player so it hides what is below.
        ae::WaterKind::Murky => (Color::srgba(0.10, 0.20, 0.18, 0.88), WORLD_Z_PLAYER + 5.0),
    };
    commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(body_color, render),
            Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
            Name::new(format!("Water body ({:?})", region.kind)),
            RoomVisual,
        ),
    );

    // Surface strip: a 4px band at the top of the region, above the body and
    // the player, so the surface shows even through Murky.
    let strip_color = match region.kind {
        ae::WaterKind::Clear => Color::srgba(0.82, 0.95, 1.0, 0.85),
        ae::WaterKind::Murky => Color::srgba(0.55, 0.78, 0.62, 0.95),
    };
    let strip_h = 4.0;
    let strip_size = BVec2::new(size.x, strip_h);
    let strip_center = ae::Vec2::new(region.aabb.center().x, region.aabb.top() + strip_h * 0.5);
    commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(strip_color, strip_size),
            Transform::from_translation(world_to_bevy(world, strip_center, WORLD_Z_PLAYER + 6.0)),
            Name::new(format!("Water surface ({:?})", region.kind)),
            RoomVisual,
        ),
    );
}

/// Render a single `ClimbableRegion` as a tinted overlay quad with rung
/// stripes. Placeholder until ladder/vine/wall art exists. Each kind has its
/// own tint so the player can tell what they touch.
fn spawn_climbable_region(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    region: &ae::ClimbableRegion,
) {
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // Above blocks, below the player: the player climbs in front of it.
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
    commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(body_color, render),
            Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
            Name::new(format!("Climbable body ({:?})", region.kind)),
            RoomVisual,
        ),
    );

    // Rung stripes every 16 px on y. Skipped for Wall (rung alpha 0).
    if rung_color.alpha() > 0.0 {
        let rung_h = 3.0;
        let rung_size = BVec2::new(size.x, rung_h);
        let mut y = region.aabb.top() + 8.0;
        while y < region.aabb.bottom() - 4.0 {
            let center = ae::Vec2::new(region.aabb.center().x, y);
            commands.spawn_session_scoped(
                session_scope,
                (
                    Sprite::from_color(rung_color, rung_size),
                    Transform::from_translation(world_to_bevy(world, center, body_z + 0.5)),
                    Name::new(format!("Climbable rung ({:?})", region.kind)),
                    RoomVisual,
                ),
            );
            y += 16.0;
        }
    }
}

/// Draw the simulation's rideable surface chains as thin, rotated strips.
///
/// Generic room presentation: any game that authors a chain gets this visual.
pub fn spawn_surface_chain_visuals(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
) {
    const THICKNESS: f32 = 8.0;

    for chain in world.chains.iter().filter(|chain| chain.filled && !chain.closed) {
        commands.spawn_session_scoped(
            session_scope,
            (
                FilledGroundVisual {
                    top: chain
                        .points
                        .iter()
                        .map(|point| world_to_bevy(world, *point, 0.0).truncate())
                        .collect(),
                    floor: world_to_bevy(world, ae::Vec2::new(0.0, world.size.y), 0.0).y,
                },
                Transform::from_xyz(0.0, 0.0, WORLD_Z_BLOCK - 0.5),
                Visibility::Visible,
                Name::new(format!("Filled ground: {}", chain.name)),
                RoomVisual,
            ),
        );
    }

    for chain in &world.chains {
        for segment_index in 0..chain.segment_count() {
            let depth = chain.segment_depth(segment_index);
            let (z, color) = if depth < 0 {
                (WORLD_Z_BLOCK + 1.0, Color::srgba(0.12, 0.50, 0.60, 0.78))
            } else if depth > 0 {
                (WORLD_Z_PLAYER + 0.8, Color::srgba(0.08, 0.38, 0.46, 0.98))
            } else {
                (WORLD_Z_BLOCK + 2.0, Color::srgba(0.22, 0.88, 0.96, 0.92))
            };
            let (a_world, b_world) = chain.segment(segment_index);
            let a = world_to_bevy(world, a_world, z);
            let b = world_to_bevy(world, b_world, z);
            let delta = b.truncate() - a.truncate();
            let length = delta.length();
            if length <= f32::EPSILON {
                continue;
            }
            commands.spawn_session_scoped(
                session_scope,
                (
                    Sprite::from_color(color, BVec2::new(length, THICKNESS)),
                    Transform::from_translation((a + b) * 0.5)
                        .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
                    Name::new(format!(
                        "Surface: {} segment {} depth {}",
                        chain.name, segment_index, depth
                    )),
                    RoomVisual,
                ),
            );
        }
    }
}

/// The earth under a filled chain (see `SurfaceChain::filled`), in Bevy space:
/// the chain's points and the room floor's y.
///
/// Spawned with the room's other visuals, which have only `Commands`;
/// [`build_filled_ground_meshes`] gives it its mesh on the next frame.
#[derive(Component, Clone, Debug)]
pub struct FilledGroundVisual {
    top: Vec<BVec2>,
    floor: f32,
}

/// Mesh every [`FilledGroundVisual`] that has none yet: one quad per chain
/// segment, from the segment down to the room floor, shaded darker with depth.
pub fn build_filled_ground_meshes(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<bevy::sprite_render::ColorMaterial>>>,
    fresh: Query<(Entity, &FilledGroundVisual), Without<Mesh2d>>,
) {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};

    const TOP: [f32; 4] = [0.05, 0.20, 0.26, 1.0];
    const DEEP: [f32; 4] = [0.02, 0.05, 0.09, 1.0];
    /// How far below the surface the shade reaches `DEEP`.
    const SHADE_DEPTH: f32 = 480.0;

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    for (entity, ground) in &fresh {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut colors: Vec<[f32; 4]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let shade = |depth: f32| {
            let t = (depth / SHADE_DEPTH).clamp(0.0, 1.0);
            std::array::from_fn(|i| TOP[i] + (DEEP[i] - TOP[i]) * t)
        };
        for pair in ground.top.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let base = positions.len() as u32;
            positions.extend([
                [a.x, a.y, 0.0],
                [b.x, b.y, 0.0],
                [b.x, ground.floor, 0.0],
                [a.x, ground.floor, 0.0],
            ]);
            colors.extend([
                TOP,
                TOP,
                shade(b.y - ground.floor),
                shade(a.y - ground.floor),
            ]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_indices(Indices::U32(indices));
        commands.entity(entity).try_insert((
            Mesh2d(meshes.add(mesh)),
            bevy::sprite_render::MeshMaterial2d(
                materials.add(bevy::sprite_render::ColorMaterial::from_color(Color::WHITE)),
            ),
        ));
    }
}

pub fn spawn_grid(commands: &mut Commands, session_scope: SessionSpawnScope, world: &ae::World) {
    let grid_color = Color::srgba(0.12, 0.15, 0.22, 0.28);
    let mut x = 0.0;
    while x <= world.size.x {
        let center = ae::Vec2::new(x, world.size.y * 0.5);
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(grid_color, BVec2::new(1.0, world.size.y)),
                Transform::from_translation(world_to_bevy(world, center, -20.0)),
                RoomVisual,
            ),
        );
        x += GRID_STEP;
    }
    let mut y = 0.0;
    while y <= world.size.y {
        let center = ae::Vec2::new(world.size.x * 0.5, y);
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(grid_color, BVec2::new(world.size.x, 1.0)),
                Transform::from_translation(world_to_bevy(world, center, -20.0)),
                RoomVisual,
            ),
        );
        y += GRID_STEP;
    }
}

/// Pick a `Tiled` stretch value that keeps the slice count under
/// `MAX_TILES_PER_AXIS²`. Tiles are `source × stretch`, so a larger stretch
/// gives fewer tiles. Returns 1.0 (native size) when the block fits the cap.
fn tiled_block_stretch(render: BVec2, source_px: f32) -> f32 {
    const MAX_TILES_PER_AXIS: f32 = 32.0;
    let source = source_px.max(1.0);
    let tiles_x = (render.x / source).max(1.0);
    let tiles_y = (render.y / source).max(1.0);
    let needed = (tiles_x.max(tiles_y) / MAX_TILES_PER_AXIS).max(1.0);
    needed.ceil()
}

/// Marker for spawned single-image entity sprites whose `Handle<Image>` is
/// rebound when `GameAssets` is rebuilt for a quality change. It rebinds the
/// handle only, and keeps size, image mode, tint, visibility and entity
/// identity. Do not despawn and respawn instead.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundEntitySprite {
    // `pub(crate)`: `apply_block_art` rewrites this when a game names its own
    // art for a block. The asset-reload refresher reads it back.
    pub(crate) key: game_assets::EntitySprite,
}

impl BoundEntitySprite {
    fn new(key: game_assets::EntitySprite) -> Self {
        Self { key }
    }
}

/// Apply game-authored [`BlockArt`] over the kind-derived block presentation.
///
/// Update `BoundEntitySprite` as well as `Sprite`, so asset reloads keep the
/// resolved binding. Blocks without authored art keep their kind texture.
/// Clear the placeholder tint when named art takes over: `Sprite::color`
/// multiplies the image.
pub fn apply_block_art(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut blocks: Query<
        (
            Entity,
            &BlockArt,
            Option<&mut BoundEntitySprite>,
            &mut Sprite,
        ),
        Or<(Changed<BlockArt>, Added<BoundEntitySprite>)>,
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, BlockArt(art), bound, mut sprite) in &mut blocks {
        match bound {
            Some(mut bound) => {
                if bound.key != *art {
                    bound.key = *art;
                }
            }
            // `try_insert`: a block visual is room-scoped, so a room transition
            // can despawn it before the command flush, and `insert` would panic.
            // See `deferred_write_safety`. `flinch_struck_blocks` does the same.
            None => {
                commands
                    .entity(entity)
                    .try_insert(BoundEntitySprite::new(*art));
            }
        }
        // Warn on a missing handle. A per-block override means somebody asked
        // for specific art, so its absence must be reported, not silent.
        let Some(handle) = assets.entities.get(*art) else {
            warn!(
                "BlockArt names {art:?}, which is not in `GameAssets.entities` — \
                 the block keeps its BlockKind's texture. The sprite is declared \
                 but its image never reached this composition's catalog."
            );
            continue;
        };
        if sprite.image != *handle {
            sprite.image = handle.clone();
        }
        // Named art replaces the placeholder tint, which is only a multiplier.
        // Do this after the handle check, so missing art leaves the block as it
        // was rather than half-applied and white.
        if sprite.color != Color::WHITE {
            sprite.color = Color::WHITE;
        }
    }
}

pub fn refresh_entity_sprite_handles_on_game_assets_change(
    assets: Option<Res<GameAssets>>,
    mut sprites: Query<
        (&BoundEntitySprite, &mut Sprite),
        (
            Without<CharacterAnimator>,
            Without<ambition_sprite_sheet::boss::BossAnimator>,
        ),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    if !assets.is_changed() {
        return;
    }
    for (bound, mut sprite) in &mut sprites {
        if let Some(handle) = assets.entities.get(bound.key) {
            if sprite.image != *handle {
                sprite.image = handle.clone();
            }
        }
    }
}

pub fn spawn_block(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    block: &ae::Block,
    physics_settings: ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings,
    assets: Option<&GameAssets>,
) {
    let size = block.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // Tiled surfaces repeat the kind's tile at native scale so visible edges
    // match collision edges. Point objects may use prop art. Missing art falls
    // back to a coloured quad. `BlockArt` can replace the default later.
    let tile_key = game_assets::block_tile_sprite(block.kind);
    let is_tiled_surface = tile_key.is_some();
    let sprite_key = tile_key.or_else(|| game_assets::point_block_sprite(block.kind));
    // An authored placeholder colour wins over every art path at spawn: the
    // shape has no sprite yet. Read it before the art lookup so no texture is
    // bound, and `refresh_entity_sprite_handles_on_game_assets_change` cannot
    // paint over it on an asset reload. `apply_block_art` creates a binding
    // when a game names art for this block.
    let placeholder = block
        .art_color
        .map(|c| Sprite::from_color(Color::srgba(c[0], c[1], c[2], c[3]), render));
    let sprite_key = if placeholder.is_some() {
        None
    } else {
        sprite_key
    };
    let sprite = if let Some(flat) = placeholder {
        flat
    } else if is_tiled_surface {
        let tile_handle = assets
            .and_then(|a| sprite_key.and_then(|key| a.entities.get(key)))
            .cloned();
        match tile_handle {
            Some(image) => Sprite {
                image,
                custom_size: Some(render),
                image_mode: bevy::sprite::SpriteImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    // Clamp the slice count for very large IntGrid surfaces:
                    // at 1.0 stretch a 3072×3328 floor gives ~9984 slices and
                    // a bevy_sprite performance warning.
                    stretch_value: tiled_block_stretch(render, 32.0),
                },
                ..Default::default()
            },
            None => Sprite::from_color(block_color(block.kind), render),
        }
    } else {
        match assets {
            Some(a) => entity_sprite_or_color(a, sprite_key, render, block_color(block.kind)),
            None => Sprite::from_color(block_color(block.kind), render),
        }
    };
    let mut entity = commands.spawn_session_scoped(
        session_scope,
        (
            sprite,
            Transform::from_translation(world_to_bevy(world, block.aabb.center(), WORLD_Z_BLOCK)),
            Name::new(format!("Block: {}", block.name)),
            // The authored name lets a mid-run overlay subtraction (a broken
            // brick) despawn this sprite; see `sync_removed_block_visuals`.
            BlockVisual {
                block_name: block.name.clone(),
                // The durable identity: a bonk arrives as
                // `ContactSource::Block { id, .. }` and has no name.
                geo_id: block.id.clone(),
            },
            RoomVisual,
        ),
    );
    if let Some(key) = sprite_key {
        entity.insert(BoundEntitySprite::new(key));
    }
    spawn_static_collider_for_block(commands, world, block, physics_settings);
}

fn spawn_static_collider_for_block(
    _commands: &mut Commands,
    _world: &ae::World,
    _block: &ae::Block,
    _settings: ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings,
) {
    // Static physics colliders are installed by the sim/physics adapter when
    // that feature is enabled. Render only spawns visual block entities.
}

/// Width-to-height aspect of the authored `door_zone.png` (published with
/// `ground = true`, so its bottom edge is the door's feet). A door keeps this
/// aspect instead of stretching to the trigger box. Keep in sync with the
/// `door_zone` drawer in the sprite renderer.
const DOOR_SPRITE_ASPECT: f32 = 0.56;

pub fn spawn_loading_zone(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    zone: &LoadingZone,
    assets: Option<&GameAssets>,
) {
    let size = zone.aabb.half_size() * 2.0;
    let fallback_color = match zone.activation {
        LoadingZoneActivation::EdgeExit => Color::srgba(0.20, 0.95, 1.0, 0.22),
        LoadingZoneActivation::Door => Color::srgba(1.0, 0.72, 0.18, 0.46),
        // Walk-through portal: green, to differ from edge exits.
        LoadingZoneActivation::Walk => Color::srgba(0.40, 1.00, 0.55, 0.30),
    };
    // A `Door` is a standing prop: a bottom-centre anchor plants its feet on
    // the floor face of the trigger box, and it keeps its authored aspect.
    // Edge-exit and walk zones stay box-filling tints anchored at the centre.
    let grounded = matches!(zone.activation, LoadingZoneActivation::Door);
    let (render, sprite_pos, anchor) = if grounded {
        let height = size.y;
        let width = height * DOOR_SPRITE_ASPECT;
        // Bottom-centre of the box in world space (y-down, so +half_y is the floor).
        let foot = zone.aabb.center() + ae::Vec2::new(0.0, zone.aabb.half_size().y);
        (BVec2::new(width, height), foot, Anchor::BOTTOM_CENTER)
    } else {
        (
            BVec2::new(size.x, size.y),
            zone.aabb.center(),
            Anchor::CENTER,
        )
    };
    let sprite_key = game_assets::loading_zone_sprite(zone.activation);
    let sprite = match assets {
        Some(a) => entity_sprite(a, sprite_key, render, fallback_color),
        None => Sprite::from_color(fallback_color, render),
    };
    let mut visual = commands.spawn_session_scoped(
        session_scope,
        (
            sprite,
            anchor,
            Transform::from_translation(world_to_bevy(world, sprite_pos, WORLD_Z_BLOCK + 6.0)),
            Name::new(format!("Loading zone: {}", zone.name)),
            // Carries the zone id so portal-aware systems can hide this debug
            // door for portal-mode zones (the portal sprite is the door).
            crate::rendering::primitives::LoadingZoneVisual {
                id: zone.id.clone(),
            },
            RoomVisual,
            BoundEntitySprite::new(sprite_key),
        ),
    );
    if matches!(zone.activation, LoadingZoneActivation::Door) {
        visual.insert(DoorNameplateSource::new(
            zone.id.clone(),
            zone.name.clone(),
            zone.aabb,
        ));
    } else {
        let label_pos = zone.aabb.center() + ae::Vec2::new(0.0, -zone.aabb.half_size().y - 18.0);
        spawn_world_label(
            commands,
            session_scope,
            world,
            format!("fixture:zone:{}", zone.id),
            WorldLabelFamily::Fixture,
            label_pos,
            &zone.name,
            13.0,
        );
    }
}

/// Common spawn body for an authored entity with a sprite and no label.
/// Hazards, pickups, breakables, enemies and bosses differ only in `kind` and
/// the `EntitySprite` the asset bank resolves.
fn spawn_authored_basic(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    id: &str,
    name: &str,
    aabb: ae::Aabb,
    kind: FeatureVisualKind,
    entity_key: Option<game_assets::EntitySprite>,
    assets: Option<&GameAssets>,
) {
    let size = aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // Initial placeholder colour only; `sync_visuals` repaints from
    // `FeatureView::fighting` on the next frame.
    let sprite = match assets {
        Some(a) => entity_sprite_or_color(a, entity_key, render, feature_color(kind, false, false)),
        None => Sprite::from_color(feature_color(kind, false, false), render),
    };
    let mut entity = commands.spawn_session_scoped(
        session_scope,
        (
            sprite,
            Transform::from_translation(world_to_bevy(world, aabb.center(), feature_z(kind))),
            Name::new(format!("Room entity: {}", name)),
            FeatureVisual { id: id.to_string() },
            RoomVisual,
        ),
    );
    if let Some(key) = entity_key {
        entity.insert(BoundEntitySprite::new(key));
    }
}

/// Spawn a pickup whose visual is an animated character sheet (a spinning ring).
/// It is an ordinary [`FeatureVisual`]: `sync_visuals` positions it and hides it
/// on collection. It also carries a [`CharacterAnimator`], so
/// `animate_feature_sprites` plays its looping `idle` row. It floats, so it is
/// centre-anchored.
fn spawn_animated_pickup(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    id: &str,
    name: &str,
    aabb: ae::Aabb,
    asset: &ambition_sprite_sheet::character::CharacterSpriteAsset,
) {
    let size = aabb.half_size() * 2.0;
    let collision = BVec2::new(size.x, size.y);
    let (sprite, anchor, animator) =
        build_character_presentation(asset, collision, Anchor::CENTER);
    commands.spawn_session_scoped(
        session_scope,
        (
            sprite,
            anchor,
            animator,
            Transform::from_translation(world_to_bevy(
                world,
                aabb.center(),
                feature_z(FeatureVisualKind::Pickup),
            )),
            Name::new(format!("Pickup sprite: {name}")),
            FeatureVisual { id: id.to_string() },
            RoomVisual,
        ),
    );
}

fn spawn_authored_hazard(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::HazardVolumeSpec,
    >,
    assets: Option<&GameAssets>,
) {
    spawn_authored_basic(
        commands,
        session_scope,
        world,
        &authored.id,
        &authored.name,
        authored.aabb,
        FeatureVisualKind::Hazard,
        game_assets::entity_sprite_for_hazard(&authored.payload),
        assets,
    );
}

fn spawn_authored_chest(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::ChestSpec,
    >,
    assets: Option<&GameAssets>,
) {
    spawn_authored_basic(
        commands,
        session_scope,
        world,
        &authored.id,
        &authored.name,
        authored.aabb,
        FeatureVisualKind::Chest,
        game_assets::entity_sprite_for_chest(&authored.payload),
        assets,
    );
    // Chest label.
    let half_h = authored.aabb.half_size().y;
    spawn_world_label(
        commands,
        session_scope,
        world,
        format!("fixture:chest:{}", authored.id),
        WorldLabelFamily::Fixture,
        authored.aabb.center() + ae::Vec2::new(0.0, -half_h - 22.0),
        &authored.name,
        14.0,
    );
}

fn spawn_authored_interactable(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::InteractableSpec,
    >,
    assets: Option<&GameAssets>,
) {
    let interactable = &authored.payload;
    let kind = if matches!(
        interactable.kind,
        ambition_platformer2d_world::rooms::InteractionKindSpec::Npc { .. }
    ) {
        FeatureVisualKind::Actor
    } else if matches!(&interactable.kind, ambition_platformer2d_world::rooms::InteractionKindSpec::Custom(s) if s.starts_with("switch:"))
    {
        FeatureVisualKind::Switch
    } else {
        return;
    };
    spawn_authored_basic(
        commands,
        session_scope,
        world,
        &authored.id,
        &authored.name,
        authored.aabb,
        kind,
        game_assets::entity_sprite_for_interactable(interactable),
        assets,
    );
    // The nameplate system renders NPC labels. This path spawns only
    // sprites/features, so map labels, chest labels and zone labels stay separate.
}

/// Block-name prefixes that `sync_lock_wall_visuals` draws with the
/// `LockWallTile` sprite: encounter lock walls and flag-gated lock walls.
const LOCK_WALL_BLOCK_PREFIXES: &[&str] = &["lockwall:", "gated_lock:"];

fn is_lock_wall_block(name: &str) -> bool {
    LOCK_WALL_BLOCK_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Reconcile `LockWallVisual` entities against the lock-wall gate solids in
/// the per-frame collision overlay. Spawn a sprite for each new lock wall;
/// despawn a visual whose block is gone (encounter cleared or failed, flag
/// unlocked).
///
/// The walls live in [`FeatureEcsWorldOverlay::gate_solids`], not the authored
/// `RoomGeometry` base. Read the base only for the world-to-screen frame.
/// Without this system a lock wall collides but is invisible. `LockWallTile`
/// keeps it distinct from ordinary walls.
pub fn sync_lock_wall_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    overlay: Res<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>,
    assets: Option<Res<GameAssets>>,
    existing: Query<(Entity, &LockWallVisual)>,
) {
    use bevy::math::Vec2 as BVec2;

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    // Index existing visuals by block name to diff in linear time.
    let mut existing_by_name: std::collections::HashMap<String, Entity> =
        std::collections::HashMap::new();
    for (entity, visual) in &existing {
        existing_by_name.insert(visual.block_name.clone(), entity);
    }

    // Pass 1: spawn a visual for each lock-wall block without one. Mark
    // consumed names so pass 2 keeps them.
    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for block in &overlay.gate_solids {
        if !is_lock_wall_block(&block.name) {
            continue;
        }
        if existing_by_name.contains_key(&block.name) {
            consumed.insert(block.name.clone());
            continue;
        }
        let size = block.aabb.half_size() * 2.0;
        let render = BVec2::new(size.x, size.y);
        // Bright purple fallback, distinct from the solid-block fallback, so a
        // missing tile is obvious.
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
        commands.spawn_session_scoped(
            session_scope,
            (
                sprite,
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    block.aabb.center(),
                    // Just above the block layer, over any floor/wall art.
                    WORLD_Z_BLOCK + 4.0,
                )),
                Name::new(format!("LockWall: {}", block.name)),
                LockWallVisual {
                    block_name: block.name.clone(),
                },
                BoundEntitySprite::new(game_assets::EntitySprite::LockWallTile),
                RoomVisual,
            ),
        );
        consumed.insert(block.name.clone());
    }

    // Pass 2: despawn visuals whose gate solid is gone.
    for (name, entity) in &existing_by_name {
        if !consumed.contains(name) {
            commands.entity(*entity).despawn();
        }
    }
}

/// A struck block flinches; the collision box does not move.
///
/// The offset lives only on the visual's transform. Moving the block would
/// lift or push bodies and give rollback an animation to rewind. The geometry
/// is authoritative and static.
///
/// The flinch is against gravity, from the acceleration frame, so a block in a
/// flipped room flinches the way that room means.
pub fn flinch_struck_blocks(
    mut commands: Commands,
    mut struck: MessageReader<ambition_platformer2d_shared_tangle::block_nudge::BlockStruck>,
    time: Res<bevy::time::Time>,
    gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
    blocks: Query<(Entity, &BlockVisual)>,
    mut flinching: Query<(Entity, &mut Transform, &mut BlockFlinch, &BlockVisual)>,
) {
    for message in struck.read() {
        // A re-strike resets the clock and keeps the home. A fresh
        // `BlockFlinch` would have a `None` home, and the pass below would fill
        // it from the displaced position, so the block would drift away from
        // its collider on each hit.
        let mut restruck = false;
        for (_, _, mut flinch, visual) in &mut flinching {
            if visual.geo_id == message.id {
                flinch.elapsed = 0.0;
                restruck = true;
            }
        }
        if restruck {
            continue;
        }
        for (entity, visual) in &blocks {
            if visual.geo_id == message.id {
                commands.entity(entity).try_insert(BlockFlinch {
                    elapsed: 0.0,
                    home: None,
                });
            }
        }
    }

    let down = gravity
        .map(|g| g.gravity_accel(1.0))
        .unwrap_or(ambition_platformer2d_core::Vec2::new(0.0, 1.0));
    let rise = -down.normalize_or(ambition_platformer2d_core::Vec2::new(0.0, 1.0));
    for (entity, mut transform, mut flinch, _) in &mut flinching {
        // Capture the home on the first frame, not at insert, so a second
        // strike does not record the flinched position as home.
        let home = *flinch.home.get_or_insert(transform.translation);
        flinch.elapsed += time.delta_secs();
        let f = ambition_platformer2d_shared_tangle::block_nudge::nudge_fraction(flinch.elapsed);
        if flinch.elapsed >= ambition_platformer2d_shared_tangle::block_nudge::NUDGE_SECONDS {
            transform.translation = home;
            commands.entity(entity).remove::<BlockFlinch>();
            continue;
        }
        let offset = f * ambition_platformer2d_shared_tangle::block_nudge::NUDGE_RISE_PX;
        // World to Bevy: y is flipped, so convert the rise the same way the
        // spawn did.
        transform.translation = home + Vec3::new(rise.x * offset, -rise.y * offset, 0.0);
    }
}

/// A block visual mid-flinch. Presentation only; never rewound.
#[derive(Component, Debug)]
pub struct BlockFlinch {
    elapsed: f32,
    /// Where the quad sits at rest, captured on the first frame of the flinch.
    home: Option<Vec3>,
}

/// Despawn the sprite of any authored block the collision overlay subtracts
/// this frame (`removed_block_names`). The overlay already drops the block from
/// collision (`apply_overlay_subtractions`); this removes it from the drawn
/// world, without editing the authored
/// [`RoomGeometry`](ambition_platformer2d_core::RoomGeometry) base.
///
/// One-directional: room reload respawns the full block set via
/// [`spawn_room_visuals`]. Generic over the block name, so it serves every game.
///
/// # A name in both lists is a replacement
///
/// A contributor that changes what an authored block is states it as a
/// subtraction by name plus an addition with the same name, box and `GeoId`
/// (Mary-O's `contribute_discovered_hidden_blocks_to_overlay` promotes a
/// struck `BonkOnly` to a `Solid`). So a subtracted name that the same overlay
/// re-adds is skipped. `rebuild_feature_ecs_world_overlay` refills both lists
/// together, so there is no ordering hazard.
///
/// Do not draw everything in `overlay.blocks`. Most of it is engine collision
/// volumes (pogo targets, breakable ghosts) with `GeoId::anon()` and synthetic
/// names that never appear in `removed_block_names`. Only the intersection of
/// the two lists means "replaced".
pub fn sync_removed_block_visuals(
    mut commands: Commands,
    overlay: Option<
        Res<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>,
    >,
    blocks: Query<(Entity, &BlockVisual)>,
) {
    let Some(overlay) = overlay else {
        return;
    };
    if overlay.removed_block_names.is_empty() {
        return;
    }
    for (entity, visual) in &blocks {
        if !overlay
            .removed_block_names
            .iter()
            .any(|name| name == &visual.block_name)
        {
            continue;
        }
        // The replacement half: see the header.
        if overlay
            .blocks
            .iter()
            .any(|block| block.name == visual.block_name)
        {
            continue;
        }
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod lock_wall_visual_tests {
    use super::*;
    use ambition_platformer2d_core::RoomGeometry;
    use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;

    fn room() -> RoomGeometry {
        RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        ))
    }

    fn gate_wall() -> ae::Block {
        ae::Block::solid(
            "lockwall:goblin_encounter",
            ae::Vec2::new(300.0, 300.0),
            ae::Vec2::new(16.0, 100.0),
        )
    }

    fn lock_wall_names(app: &mut App) -> Vec<String> {
        let mut q = app.world_mut().query::<&LockWallVisual>();
        let mut names: Vec<String> = q.iter(app.world()).map(|v| v.block_name.clone()).collect();
        names.sort();
        names
    }

    /// The reconcile reads the overlay's `gate_solids`, not the authored base:
    /// a gate solid spawns a `LockWallVisual`, and dropping it from the overlay
    /// despawns the visual.
    #[test]
    fn lock_wall_visual_tracks_overlay_gate_solids() {
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            room(),
        );
        app.insert_resource(FeatureEcsWorldOverlay {
            gate_solids: vec![gate_wall()],
            ..Default::default()
        });
        app.add_systems(Update, sync_lock_wall_visuals);

        app.update();
        assert_eq!(
            lock_wall_names(&mut app),
            vec!["lockwall:goblin_encounter".to_string()],
            "a gate solid spawns its LockWallVisual"
        );

        // The contributor stops deriving the wall, so the visual despawns.
        app.world_mut()
            .resource_mut::<FeatureEcsWorldOverlay>()
            .gate_solids
            .clear();
        app.update();
        assert!(
            lock_wall_names(&mut app).is_empty(),
            "dropping the gate solid despawns the LockWallVisual"
        );
    }

    /// The removed-block reconcile despawns exactly the block visuals the
    /// overlay subtracts this frame. Other block visuals stay.
    #[test]
    fn removed_block_visual_despawns_only_subtracted_blocks() {
        let mut app = App::new();
        let brick = app
            .world_mut()
            .spawn(BlockVisual {
                block_name: "brick_1".to_string(),
                geo_id: ambition_platformer2d_core::GeoId::anon(),
            })
            .id();
        let ground = app
            .world_mut()
            .spawn(BlockVisual {
                block_name: "ground_open_teach".to_string(),
                geo_id: ambition_platformer2d_core::GeoId::anon(),
            })
            .id();
        app.insert_resource(FeatureEcsWorldOverlay {
            removed_block_names: vec!["brick_1".to_string()],
            ..Default::default()
        });
        app.add_systems(Update, sync_removed_block_visuals);

        app.update();
        assert!(
            app.world().get_entity(brick).is_err(),
            "the subtracted brick's visual is despawned"
        );
        assert!(
            app.world().get_entity(ground).is_ok(),
            "an un-subtracted block's visual is left standing"
        );
    }

    /// A name in both overlay lists is a replacement, and the visual survives.
    ///
    /// Also: an added block under a different name (like every engine pogo or
    /// breakable volume) must not rescue an ordinary removal.
    #[test]
    fn a_replaced_block_keeps_its_visual_while_a_removed_one_does_not() {
        let mut app = App::new();
        let promoted = app
            .world_mut()
            .spawn(BlockVisual {
                block_name: "maryo_block:Hidden:AlwaysCoin:1".to_string(),
                geo_id: ambition_platformer2d_core::GeoId::anon(),
            })
            .id();
        let broken = app
            .world_mut()
            .spawn(BlockVisual {
                block_name: "brick_1".to_string(),
                geo_id: ambition_platformer2d_core::GeoId::anon(),
            })
            .id();
        app.insert_resource(FeatureEcsWorldOverlay {
            removed_block_names: vec![
                "maryo_block:Hidden:AlwaysCoin:1".to_string(),
                "brick_1".to_string(),
            ],
            blocks: vec![
                // The replacement: same name as the subtraction above.
                ae::Block::solid(
                    "maryo_block:Hidden:AlwaysCoin:1",
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(16.0, 16.0),
                ),
                // The poison: an unrelated overlay block, shaped like an engine
                // pogo volume.
                ae::Block::solid(
                    "ecs-pogo-target 7 0",
                    ae::Vec2::new(400.0, 200.0),
                    ae::Vec2::new(16.0, 16.0),
                ),
            ],
            ..Default::default()
        });
        app.add_systems(Update, sync_removed_block_visuals);

        app.update();
        assert!(
            app.world().get_entity(promoted).is_ok(),
            "a block the same overlay is RE-ADDING was replaced, not removed — its \
             visual is what the replacement wants dressed (queue D69)",
        );
        assert!(
            app.world().get_entity(broken).is_err(),
            "an ordinary subtraction still despawns: an added block under another \
             name does not rescue it",
        );
    }

    /// With no overlay resource (a minimal app), the reconcile is a graceful
    /// no-op rather than a panic — it never despawns a block on its own.
    #[test]
    fn removed_block_visual_is_inert_without_an_overlay() {
        let mut app = App::new();
        let brick = app
            .world_mut()
            .spawn(BlockVisual {
                block_name: "brick_1".to_string(),
                geo_id: ambition_platformer2d_core::GeoId::anon(),
            })
            .id();
        app.add_systems(Update, sync_removed_block_visuals);
        app.update();
        assert!(
            app.world().get_entity(brick).is_ok(),
            "no overlay ⇒ nothing is subtracted"
        );
    }
}

#[cfg(test)]
mod prop_geometry_tests {
    use super::*;
    use ambition_sprite_sheet::character::sheets::{try_load_spec_for_target, SheetTuning};

    /// A pipe's art must match the surface a body stands on.
    ///
    /// Sizing pipe pieces like a character (art overflowing the collider on a
    /// feet anchor) drew the lip a tile above the standing surface. This reads
    /// the real baked pipe-head sheet, so it fails if the sheet's frame
    /// geometry drifts from the authored box.
    #[test]
    fn a_structure_props_art_exactly_fills_the_collider_a_body_stands_on() {
        let spec = try_load_spec_for_target("super_mary_o_pipe_top", &SheetTuning::new(1.0, 0))
            .expect("the pipe-head sheet is baked into the manifest");
        // What level 1-1 authors for one pipe piece: two tiles wide, one tall.
        let authored = BVec2::new(64.0, 32.0);

        let (size, anchor) = prop_sprite_geometry(PropDraw::Structure, &spec, authored);
        assert_eq!(
            size, authored,
            "built world is drawn at exactly its authored box, or its surfaces are \
             not where bodies stand on them"
        );
        assert_eq!(
            anchor,
            Anchor::CENTER,
            "and centred on that box — a feet anchor would slide it off the block"
        );

        // The bbox-quad route leaves only the crop's transparent margin (4.9%
        // here). Built world keeps its own path for the anchor and exactness.
        let (decoration, decoration_anchor) =
            prop_sprite_geometry(PropDraw::Decoration, &spec, authored);
        assert!(
            decoration.x > authored.x && decoration.y > authored.y,
            "character sizing draws the whole FRAME, so it is still larger than the \
             box by the crop's transparent margin ({decoration:?} vs {authored:?})"
        );
        assert_ne!(
            decoration_anchor,
            Anchor::CENTER,
            "character sizing hangs the quad off a feet anchor, which slides a pipe \
             head off the block a body stands on however well the quad is sized"
        );
    }
}
