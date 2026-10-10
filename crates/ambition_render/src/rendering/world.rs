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
    block_color, feature_color, feature_z, spawn_world_label, EntityArt, BlockVisual, FeatureVisual,
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

/// Marks that one live room's static visuals are spawned. It is stamped with
/// its room and is a [`RoomVisual`], so it retires with the room's other
/// visuals.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PresentedRoomVisuals;

/// Give each live room its static visuals, stamped with that room.
///
/// The marker is the memo, and it is in the world: a live room with no marker
/// has no visuals. So a room opened beside another live room is drawn, a room
/// that replaces another (a crossing, a reset) is drawn again after the
/// retirement takes the old visuals, and a retirement takes only the visuals
/// of the room that retires (`InRoomInstance::leaves_with`).
pub fn present_live_room_visuals(
    mut commands: Commands,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    presented: Query<
        &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance,
        With<PresentedRoomVisuals>,
    >,
    assets: Option<Res<GameAssets>>,
    active_session: Option<Res<ActiveSessionScope>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    // A device that draws no screen shader draws no room look: a room with a
    // look then has its blocks only, and they take the skin of its theme.
    let looks_are_drawn = quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders());
    for (room, definition) in rooms.live_rooms() {
        if presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let scope = session_scope.in_room(Some(room));
        spawn_room_visuals(
            &mut commands,
            scope,
            rooms.rooms().spec(definition),
            assets.as_deref(),
            looks_are_drawn,
        );
        commands.spawn_session_scoped(
            scope,
            (PresentedRoomVisuals, RoomVisual, Name::new("presented room visuals")),
        );
    }
}

pub fn spawn_room_visuals(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    spec: &ambition_platformer2d_world::rooms::RoomSpec,
    assets: Option<&GameAssets>,
    // Whether this device draws the look of a room that has one
    // (`ShaderBudget::draws_screen_shaders`).
    looks_are_drawn: bool,
) {
    let world = &spec.world;
    spawn_grid(commands, session_scope, world);
    spawn_surface_chain_visuals(commands, session_scope, world);
    // The theme the room names: its blocks take the terrain skin of it.
    // A room with a look of its own (a `palette`) draws its blocks its own way:
    // no skin goes on them. On a device that does not draw the look, the
    // blocks are the room, and they take the skin. (The choice is made when
    // the room is presented: a room that is live when the tier changes keeps
    // what it has until it is presented again.)
    let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::named_by_room_metadata(&spec.metadata)
        .filter(|_| spec.metadata.visual_profile.palette.is_none() || !looks_are_drawn);
    if theme.is_some() {
        // Where the decor of the skin must not stand: on a thing of the play.
        let rect = |aabb: &ae::Aabb| {
            let (centre, half) = (aabb.center(), aabb.half_size());
            (BVec2::new(centre.x - half.x, centre.y - half.y), BVec2::new(centre.x + half.x, centre.y + half.y))
        };
        let around = |pos: ae::Vec2| (BVec2::new(pos.x - 28.0, pos.y - 44.0), BVec2::new(pos.x + 28.0, pos.y + 44.0));
        let mut keep_out: Vec<(BVec2, BVec2)> = Vec::new();
        keep_out.extend(spec.loading_zones.iter().map(|zone| rect(&zone.aabb)));
        keep_out.extend(spec.placements.iter().map(|record| rect(&record.aabb)));
        keep_out.extend(spec.enemy_spawns.iter().map(|spawn| rect(&spawn.aabb)));
        keep_out.extend(spec.boss_spawns.iter().map(|spawn| rect(&spawn.aabb)));
        keep_out.extend(spec.props.iter().map(|prop| around(prop.pos)));
        keep_out.extend(spec.shrines.iter().map(|shrine| around(shrine.pos)));
        keep_out.extend(spec.ground_items.iter().map(|item| around(item.pos)));
        keep_out.extend(spec.portal_gun_spawns.iter().map(|spawn| around(spawn.pos)));
        // A block that is not ground (a hazard, an orb, a pad) is a thing of
        // the play too.
        keep_out.extend(
            world
                .blocks
                .iter()
                .filter(|block| {
                    !matches!(
                        block.kind,
                        ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
                    )
                })
                .map(|block| rect(&block.aabb)),
        );
        commands.spawn_session_scoped(
            session_scope,
            (
                super::terrain_skin::TerrainKeepOut(keep_out),
                RoomVisual,
                Name::new("Terrain decor keep-out"),
            ),
        );
    }
    for block in &world.blocks {
        spawn_block(
            commands,
            session_scope,
            world,
            block,
            assets,
            theme,
        );
    }
    for region in &world.water_regions {
        spawn_water_region(commands, session_scope, world, region);
    }
    for region in &world.climbable_regions {
        spawn_climbable_region(commands, session_scope, world, region);
    }
    for zone in &spec.loading_zones {
        spawn_loading_zone(commands, session_scope, world, zone, assets, theme);
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
        // A sign that names controls by action is written from the control
        // prompt, so it starts with every control unresolved.
        let legend = super::control_legend::ControlLegend::for_text(&label.payload.text);
        let text = legend
            .as_ref()
            .map_or_else(|| label.payload.text.clone(), |legend| legend.unresolved_text());
        let sign = spawn_world_label(
            commands,
            session_scope,
            world,
            format!("signage:{index}:{}", label.id),
            WorldLabelFamily::Signage,
            label.payload.position,
            &text,
            14.0,
        );
        if let Some(legend) = legend {
            commands.entity(sign).insert(legend);
        }
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

/// How Bevy draws a column that tiles in a box `size`: the cap at the top,
/// the end at the bottom, and whole tiles between them
/// ([`ambition_sprite_sheet::character::ColumnSlices::fill`]).
///
/// The slicer cuts the atlas rect of the frame in three rows. Its corners
/// have no width, so its top and bottom sides are the cap and the end, and
/// its centre is the tile.
pub(crate) fn column_image_mode(
    slices: ambition_sprite_sheet::character::ColumnSlices,
    size: BVec2,
) -> bevy::sprite::SpriteImageMode {
    use bevy::sprite::{BorderRect, SliceScaleMode, SpriteImageMode, TextureSlicer};
    let fill = slices.fill(size);
    // The slicer tiles the centre across also, by the same scale. A tile as
    // wide as the box is one column. In a box too short for a whole tile the
    // scale is less than the width asks for, and a tile would repeat across:
    // there the one tile is stretched.
    let one_column = slices.rect.x * fill.tile_scale + 1e-3 >= size.x;
    SpriteImageMode::Sliced(TextureSlicer {
        border: BorderRect {
            min_inset: BVec2::new(0.0, slices.cap),
            max_inset: BVec2::new(0.0, slices.end),
        },
        center_scale_mode: if one_column {
            // A little more than the scale, so that a rounding of the last
            // tile does not leave a row for one more.
            SliceScaleMode::Tile { stretch_value: fill.tile_scale * (1.0 + 1e-5) }
        } else {
            SliceScaleMode::Stretch
        },
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: fill.scale,
    })
}

/// Build the sprite, anchor and animator for a prop sheet at its collision size.
///
/// A sheet that is a column that tiles (a rope) fills the box, whatever the
/// prop is: the box gives the column its length.
pub(crate) fn prop_sprite_bundle(
    draw: PropDraw,
    flip_y: bool,
    asset: &ambition_sprite_sheet::character::CharacterSpriteAsset,
    collision: BVec2,
) -> (Sprite, Anchor, CharacterAnimator) {
    let (render_size, anchor) = prop_sprite_geometry(draw, &asset.spec, collision);
    let (mut sprite, mut anchor, mut animator) =
        build_character_presentation_with_render_size(asset, render_size, anchor);
    if let Some(slices) = asset.spec.column_slices() {
        sprite.custom_size = Some(collision);
        sprite.image_mode = column_image_mode(slices, collision);
        sprite.flip_y = flip_y;
        // The box is the quad: no frame sizes it again.
        animator.keeps_its_quad = true;
        return (sprite, Anchor::CENTER, animator);
    }
    // A prop that fills its box is a piece of built world, and the next piece
    // touches it. It samples inside its frame, so its edge row is opaque and
    // the pieces show no line between them (`CharacterAnimator::sample_rect`).
    if draw.fills_box() {
        animator.samples_inside_frame = true;
        sprite.rect = animator.sample_rect();
    }
    // Which way the prop points is authored data, not a second sheet.
    sprite.flip_y = flip_y;
    if flip_y {
        // A packed sheet trims each frame, and the anchor places the trimmed
        // rect in its frame. A flip mirrors the frame, so it mirrors that
        // placement also, as the facing flip does for x
        // (`draw_animator_frame`). Without this the art of a pipe head that
        // hangs from a ceiling stood off its shaft by the trim.
        anchor.0.y = -anchor.0.y;
    }
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
        WORLD_Z_PLAYER + super::BODY_DEPTH_BAND
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

    for chain in world.chains.iter().filter(|chain| !chain.earth.is_empty()) {
        commands.spawn_session_scoped(
            session_scope,
            (
                PaintedEarthVisual::new(world, &chain.earth),
                Transform::from_xyz(0.0, 0.0, WORLD_Z_BLOCK - 0.5),
                Visibility::Visible,
                Name::new(format!("Painted earth: {}", chain.name)),
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

/// Painted earth (see `SurfaceChain::earth`), in Bevy space: its convex
/// polygons, and each vertex's depth below the top of the earth in its column,
/// which shades it the way [`FilledGroundVisual`] shades depth below a chain.
#[derive(Component, Clone, Debug)]
pub struct PaintedEarthVisual {
    polygons: Vec<Vec<BVec2>>,
    depths: Vec<Vec<f32>>,
}

impl PaintedEarthVisual {
    fn new(world: &ae::World, earth: &[Vec<ae::Vec2>]) -> Self {
        /// Column width for the shading's "top of the earth here": one LDtk cell.
        const COLUMN: f32 = 16.0;
        let column = |x: f32| (x / COLUMN).floor() as i64;
        let mut tops: std::collections::HashMap<i64, f32> = std::collections::HashMap::new();
        for polygon in earth {
            let (min_x, max_x, min_y) = polygon.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY),
                |(a, b, c), p| (a.min(p.x), b.max(p.x), c.min(p.y)),
            );
            for col in column(min_x)..column(max_x - 0.01).max(column(min_x)) + 1 {
                let top = tops.entry(col).or_insert(min_y);
                *top = top.min(min_y);
            }
        }
        let depth = |p: ae::Vec2| {
            // A vertex on a column boundary belongs to both columns: the
            // shallower answer keeps a cliff's face lit from its top.
            let top = [column(p.x), column(p.x - 0.01)]
                .into_iter()
                .filter_map(|col| tops.get(&col).copied())
                .fold(f32::INFINITY, f32::min);
            if top.is_finite() { (p.y - top).max(0.0) } else { 0.0 }
        };
        Self {
            polygons: earth
                .iter()
                .map(|polygon| {
                    polygon.iter().map(|p| world_to_bevy(world, *p, 0.0).truncate()).collect()
                })
                .collect(),
            depths: earth.iter().map(|polygon| polygon.iter().map(|p| depth(*p)).collect()).collect(),
        }
    }
}

/// Mesh every [`FilledGroundVisual`] and [`PaintedEarthVisual`] that has none
/// yet. Filled ground is one quad per chain segment, from the segment down to
/// the room floor; painted earth is its own polygons. Both shade darker with
/// depth.
pub fn build_filled_ground_meshes(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<bevy::sprite_render::ColorMaterial>>>,
    fresh: Query<(Entity, &FilledGroundVisual), Without<Mesh2d>>,
    fresh_earth: Query<(Entity, &PaintedEarthVisual), Without<Mesh2d>>,
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
    for (entity, earth) in &fresh_earth {
        let shade = |depth: f32| -> [f32; 4] {
            let t = (depth / SHADE_DEPTH).clamp(0.0, 1.0);
            std::array::from_fn(|i| TOP[i] + (DEEP[i] - TOP[i]) * t)
        };
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut colors: Vec<[f32; 4]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (polygon, depths) in earth.polygons.iter().zip(&earth.depths) {
            let base = positions.len() as u32;
            positions.extend(polygon.iter().map(|p| [p.x, p.y, 0.0]));
            colors.extend(depths.iter().map(|d| shade(*d)));
            // Convex: a fan from the first vertex.
            for k in 1..polygon.len().saturating_sub(1) as u32 {
                indices.extend([base, base + k, base + k + 1]);
            }
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
    // `pub(crate)`: `apply_entity_art` rewrites this when a game names its own
    // art for a block. The asset-reload refresher reads it back.
    pub(crate) key: game_assets::EntitySprite,
}

impl BoundEntitySprite {
    fn new(key: game_assets::EntitySprite) -> Self {
        Self { key }
    }
}

/// Apply game-authored [`EntityArt`] over the kind-derived presentation of a
/// block or a door.
///
/// Update `BoundEntitySprite` as well as `Sprite`, so asset reloads keep the
/// resolved binding. Blocks without authored art keep their kind texture.
/// Clear the placeholder tint when named art takes over: `Sprite::color`
/// multiplies the image.
pub fn apply_entity_art(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut blocks: Query<
        (
            Entity,
            &EntityArt,
            Option<&mut BoundEntitySprite>,
            &mut Sprite,
        ),
        Or<(Changed<EntityArt>, Added<BoundEntitySprite>)>,
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, EntityArt(art), bound, mut sprite) in &mut blocks {
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
                "EntityArt names {art:?}, which is not in `GameAssets.entities` — \
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
    assets: Option<&GameAssets>,
    // The theme the room of the block names, if it names one
    // (`terrain_skin`).
    theme: Option<ambition_sprite_sheet::game_assets::ParallaxTheme>,
) {
    let size = block.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // Tiled surfaces repeat the kind's tile at native scale so visible edges
    // match collision edges. Point objects may use prop art. Missing art falls
    // back to a coloured quad. `EntityArt` can replace the default later.
    let tile_key = game_assets::block_tile_sprite(block.kind);
    let is_tiled_surface = tile_key.is_some();
    let sprite_key = tile_key.or_else(|| game_assets::point_block_sprite(block.kind));
    // An authored placeholder colour wins over every art path at spawn: the
    // shape has no sprite yet. Read it before the art lookup so no texture is
    // bound, and `refresh_entity_sprite_handles_on_game_assets_change` cannot
    // paint over it on an asset reload. `apply_entity_art` creates a binding
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
    // A block that draws the tile of its kind can take the skin of its room.
    // A placeholder colour is authored and a lock wall has its own art.
    let surface_kind = match block.kind {
        ae::BlockKind::Solid => Some(super::terrain_skin::TerrainSurfaceKind::Solid),
        ae::BlockKind::OneWay => Some(super::terrain_skin::TerrainSurfaceKind::OneWay),
        ae::BlockKind::BlinkWall { .. } => Some(super::terrain_skin::TerrainSurfaceKind::Cover),
        _ => None,
    };
    if let (Some(theme), Some(kind), None, false) =
        (theme, surface_kind, block.art_color, is_lock_wall_block(&block.name))
    {
        let min = block.aabb.center() - block.aabb.half_size();
        entity.insert(super::terrain_skin::TerrainSurface {
            theme,
            kind,
            min: BVec2::new(min.x, min.y),
            size: render,
        });
    }
}

/// Width-to-height aspect of the authored `door_zone.png` (published with
/// `ground = true`, so its bottom edge is the door's feet). A door keeps this
/// aspect instead of stretching to the trigger box. The published texture is
/// 126 x 242 px. Keep in sync with the door drawers in the sprite renderer
/// (`targets/props/entities.py`): all doors have one shape and one size.
pub const DOOR_SPRITE_ASPECT: f32 = 126.0 / 242.0;

pub fn spawn_loading_zone(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    zone: &LoadingZone,
    assets: Option<&GameAssets>,
    // The theme the room names, if its doors take the door of the theme
    // (`terrain_skin::dress_themed_doors`).
    theme: Option<ambition_sprite_sheet::game_assets::ParallaxTheme>,
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
    if let (LoadingZoneActivation::Door, Some(theme)) = (zone.activation, theme) {
        visual.insert(super::terrain_skin::ThemedDoor(theme));
    }
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
    // Every live room's walls, each drawn in its own room and stamped with
    // it. A sole-room read did not run while two rooms were live.
    rooms: Query<
        (
            &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &ambition_platformer2d_core::RoomGeometry,
            &ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    assets: Option<Res<GameAssets>>,
    existing: Query<(
        Entity,
        &LockWallVisual,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    )>,
) {
    use bevy::math::Vec2 as BVec2;

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    // Index existing visuals by (live room, block name) to diff in linear
    // time. Two live rooms can hold a wall with one name (two instances of
    // one room).
    type Key = (Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>, String);
    let mut existing_by_name: std::collections::HashMap<Key, Entity> = std::collections::HashMap::new();
    for (entity, visual, room) in &existing {
        existing_by_name.insert((room.map(|room| room.0), visual.block_name.clone()), entity);
    }

    // Pass 1: spawn a visual for each lock-wall block without one. Mark
    // consumed keys so pass 2 keeps them.
    let mut consumed: std::collections::HashSet<Key> = std::collections::HashSet::new();
    let blocks = rooms.iter().flat_map(|(room, world, overlay)| {
        overlay.gate_solids.iter().map(move |block| (*room, world, block))
    });
    for (room, world, block) in blocks {
        if !is_lock_wall_block(&block.name) {
            continue;
        }
        let key = (Some(room), block.name.clone());
        if existing_by_name.contains_key(&key) {
            consumed.insert(key);
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
            session_scope.in_room(Some(room)),
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
        consumed.insert(key);
    }

    // Pass 2: despawn visuals whose gate solid is gone.
    for (key, entity) in &existing_by_name {
        if !consumed.contains(key) {
            commands.entity(*entity).try_despawn();
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
///
/// Each block visual reads the overlay of its own live room (its stamp, or
/// the sole live room), so a brick broken in one live room does not take the
/// same-named brick of another (view half, cut V2j).
pub fn sync_removed_block_visuals(
    mut commands: Commands,
    overlays: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay,
    >,
    blocks: Query<(Entity, &BlockVisual)>,
) {
    for (entity, visual) in &blocks {
        // A block whose live room cannot be told, or whose room has no
        // overlay, keeps its visual.
        let Some(overlay) = overlays.of(entity) else {
            continue;
        };
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
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            room(),
        );
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay {
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
        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<FeatureEcsWorldOverlay>(app.world_mut()).expect("the live room has a collision overlay")
            .gate_solids
            .clear();
        app.update();
        assert!(
            lock_wall_names(&mut app).is_empty(),
            "dropping the gate solid despawns the LockWallVisual"
        );
    }

    /// View half, cut V2d: each live room draws its own lock walls. Two live
    /// rooms of different sizes hold a wall with one name (two instances of
    /// one room do). Each wall is drawn once, stamped with its room, and
    /// placed by its room's flip. When one room's wall drops, only that
    /// room's visual goes. Before V2d this road read the sole live room and
    /// drew no wall while two rooms were live.
    #[test]
    fn each_live_room_draws_its_own_lock_walls() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let geometry = |w: f32, h: f32| {
            ae::RoomGeometry(ae::World::new("lock room", ae::Vec2::new(w, h), ae::Vec2::new(16.0, 16.0), Vec::new()))
        };
        let walls = || FeatureEcsWorldOverlay {
            gate_solids: vec![gate_wall()],
            ..Default::default()
        };
        let second = LiveRoomInstance::ACTIVATION.next();
        let mut app = App::new();
        let first_root =
            ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry(800.0, 600.0));
        app.world_mut().entity_mut(first_root).insert(walls());
        let second_root =
            ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(app.world_mut(), second, geometry(400.0, 300.0));
        app.world_mut().entity_mut(second_root).insert(walls());
        app.add_systems(Update, sync_lock_wall_visuals);
        let drawn = |app: &mut App| {
            let mut drawn: Vec<(LiveRoomInstance, bevy::math::Vec2)> = app
                .world_mut()
                .query_filtered::<(&InRoomInstance, &Transform), With<LockWallVisual>>()
                .iter(app.world())
                .map(|(room, transform)| (room.0, transform.translation.truncate()))
                .collect();
            drawn.sort_by_key(|(room, _)| *room);
            drawn
        };
        // The wall's centre is (308, 350).
        let flip = |w: f32, h: f32| bevy::math::Vec2::new(308.0 - w * 0.5, h * 0.5 - 350.0);
        app.update();
        app.update();
        assert_eq!(
            drawn(&mut app),
            vec![(LiveRoomInstance::ACTIVATION, flip(800.0, 600.0)), (second, flip(400.0, 300.0))],
            "a live room's lock wall was not drawn once, in its own room"
        );
        app.world_mut()
            .get_mut::<FeatureEcsWorldOverlay>(second_root)
            .expect("the second room's overlay")
            .gate_solids
            .clear();
        app.update();
        assert_eq!(
            drawn(&mut app),
            vec![(LiveRoomInstance::ACTIVATION, flip(800.0, 600.0))],
            "a wall that dropped in one room took the other room's visual, or kept its own"
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
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay {
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
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay {
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

    /// Each block visual reads its own live room's overlay (view half, cut
    /// V2j). Two live instances of one room, each with a brick of the same
    /// name: the brick broken in the second instance loses its visual, and
    /// the same brick in the first instance keeps it. Before, the reconcile
    /// read the sole live room, so while two rooms were live no broken brick
    /// lost its visual.
    #[test]
    fn a_brick_broken_in_one_live_room_keeps_the_same_brick_of_another() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
        };
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(
            app.world_mut(),
            second,
            FeatureEcsWorldOverlay {
                removed_block_names: vec!["brick_1".to_string()],
                ..Default::default()
            },
        );
        let brick = |room| {
            (
                BlockVisual {
                    block_name: "brick_1".to_string(),
                    geo_id: ambition_platformer2d_core::GeoId::anon(),
                },
                InRoomInstance(room),
            )
        };
        let kept = app.world_mut().spawn(brick(LiveRoomInstance::ACTIVATION)).id();
        let broken = app.world_mut().spawn(brick(second)).id();
        app.add_systems(Update, sync_removed_block_visuals);
        app.update();
        assert!(app.world().get_entity(broken).is_err(), "the brick broken in the second room kept its visual");
        assert!(app.world().get_entity(kept).is_ok(), "the first room's brick lost its visual to the second room's break");
    }

    /// With no live room overlay (a minimal app), the reconcile is a graceful
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

    /// A prop drawn upside down is the mirror of the prop drawn upright (the
    /// head of a pipe that hangs from a ceiling). The pipe-head sheet is
    /// trimmed: its art starts two rows under the top of its frame.
    ///
    /// Measured before: the flip turned the picture inside its trimmed rect
    /// and left the rect where it was, so the head's art stood two rows off
    /// the shaft it joins (Jon, 2026-10-08: a gap above the bottom cap).
    #[test]
    fn a_prop_drawn_upside_down_is_the_mirror_of_the_upright_prop() {
        use ambition_sprite_sheet::character::{CharacterSpriteAsset, CharacterSpritePage};
        let spec = try_load_spec_for_target("super_mary_o_pipe_top", &SheetTuning::new(1.0, 0))
            .expect("the pipe-head sheet is baked into the manifest");
        let asset = CharacterSpriteAsset {
            texture: Handle::default(),
            layout: Handle::default(),
            spec,
            pages: vec![CharacterSpritePage { texture: Handle::default(), layout: Handle::default() }],
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        };
        let authored = BVec2::new(64.0, 32.0);
        // `(top, bottom)` of the drawn art about the middle of its box, +y up.
        let drawn = |flip_y: bool| {
            let (sprite, anchor, _) = prop_sprite_bundle(PropDraw::Enclosure, flip_y, &asset, authored);
            assert_eq!(sprite.flip_y, flip_y);
            let size = sprite.custom_size.expect("a sized sprite");
            let middle = -anchor.0.y * size.y;
            (middle + size.y * 0.5, middle - size.y * 0.5)
        };
        let (top, bottom) = drawn(false);
        assert!(top < authored.y * 0.5 - 1.0, "premise: the upright art starts under the top of its box ({top})");
        assert!((bottom + authored.y * 0.5).abs() < 0.01, "premise: the upright art stands on the bottom of its box ({bottom})");

        let (flipped_top, flipped_bottom) = drawn(true);
        assert!(
            (flipped_top + bottom).abs() < 0.01 && (flipped_bottom + top).abs() < 0.01,
            "upside down the art spans {flipped_bottom}..{flipped_top}; the mirror of {bottom}..{top} is {}..{}",
            -top,
            -bottom
        );
    }

    /// A piece of built world samples inside its frame, so its edge row is
    /// opaque and the pieces of a pipe show no line between them.
    ///
    /// Measured before, in a capture of the first pipe of 1-1 at 1280x720:
    /// a line across the shaft where its first tile starts, with the tiles
    /// overlapped by 2 px. The edge row of a tile was filtered against the
    /// transparent padding of its atlas, and drawn part transparent and
    /// darker (Jon, 2026-10-08: "many seams in the pipe parts").
    #[test]
    fn a_prop_that_fills_its_box_samples_inside_its_frame() {
        use ambition_sprite_sheet::character::{CharacterSpriteAsset, CharacterSpritePage};
        let asset = |target: &str| CharacterSpriteAsset {
            texture: Handle::default(),
            layout: Handle::default(),
            spec: try_load_spec_for_target(target, &SheetTuning::new(1.0, 0))
                .expect("the pipe sheets are baked into the manifest"),
            pages: vec![CharacterSpritePage { texture: Handle::default(), layout: Handle::default() }],
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        };
        let authored = BVec2::new(64.0, 32.0);
        // The shaft's frame is 53 x 32 texels and the head's is 61 x 30
        // (their published sheet records).
        for (target, texels) in [
            ("super_mary_o_pipe_body", BVec2::new(53.0, 32.0)),
            ("super_mary_o_pipe_top", BVec2::new(61.0, 30.0)),
        ] {
            let asset = asset(target);
            for draw in [PropDraw::Structure, PropDraw::Enclosure] {
                let (sprite, _, animator) = prop_sprite_bundle(draw, false, &asset, authored);
                assert!(animator.samples_inside_frame, "{target} as {draw:?}");
                assert_eq!(
                    sprite.rect,
                    Some(Rect::new(0.5, 0.5, texels.x - 0.5, texels.y - 0.5)),
                    "{target} as {draw:?} samples half a texel inside its frame"
                );
            }
            // Scenery is sized like a character and touches nothing.
            let (sprite, _, animator) = prop_sprite_bundle(PropDraw::Decoration, false, &asset, authored);
            assert!(!animator.samples_inside_frame && sprite.rect.is_none(), "{target} as scenery");
        }
    }

    /// A prop sheet with no rig and no pages, as the tests here build it.
    fn baked_prop_asset(target: &str) -> ambition_sprite_sheet::character::CharacterSpriteAsset {
        use ambition_sprite_sheet::character::{CharacterSpriteAsset, CharacterSpritePage};
        CharacterSpriteAsset {
            texture: Handle::default(),
            layout: Handle::default(),
            spec: try_load_spec_for_target(target, &SheetTuning::new(1.0, 0))
                .unwrap_or_else(|| panic!("the sheet `{target}` is baked into the manifest")),
            pages: vec![CharacterSpritePage { texture: Handle::default(), layout: Handle::default() }],
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        }
    }

    /// What to run when a rope arm is red on a machine that has the rope
    /// sheet of an older renderer. Published sheets are not tracked: each
    /// checkout draws its own.
    const PUBLISH_THE_ROPE: &str =
        "the published rope sheet states no column tile: run `./scripts/regen/sprites.sh --target cut_rope_rope`";

    /// `(top, bottom)` of the quad a prop is drawn at, about the middle of
    /// its box, +y up.
    fn drawn_span(sprite: &Sprite, anchor: &Anchor) -> (f32, f32) {
        let size = sprite.custom_size.expect("a sized sprite");
        let middle = -anchor.0.y * size.y;
        (middle + size.y * 0.5, middle - size.y * 0.5)
    }

    /// The rope of the cut-the-rope arena hangs from the top of its box to
    /// the bottom, for a box of any height: the map gives the rope its
    /// length.
    ///
    /// Measured before (Jon, 2026-10-09: "the rope is disconnected from the
    /// ceiling"): the frame was fitted to the box as one picture. The box is
    /// 8 x 160 and the art was 20 x 192 texels, so the width set the scale
    /// (0.4), and the rope was drawn 76.8 high from the bottom of its box. Its
    /// top was 83.2 under the ceiling.
    #[test]
    fn a_rope_is_drawn_from_the_top_of_its_box_to_the_bottom() {
        let asset = baked_prop_asset("cut_rope_rope");
        assert!(asset.spec.column_slices().is_some(), "{PUBLISH_THE_ROPE}");
        // The box the arena authors, a short one, a long one and a wide one.
        for authored in [
            BVec2::new(8.0, 160.0),
            BVec2::new(8.0, 48.0),
            BVec2::new(8.0, 700.0),
            BVec2::new(16.0, 320.0),
        ] {
            let (sprite, anchor, _) = prop_sprite_bundle(PropDraw::Decoration, false, &asset, authored);
            let (top, bottom) = drawn_span(&sprite, &anchor);
            assert!(
                (top - authored.y * 0.5).abs() < 0.01 && (bottom + authored.y * 0.5).abs() < 0.01,
                "in a box of {authored:?} the rope is drawn from {top} to {bottom} about the middle \
                 of its box; the box is from {} to {}",
                authored.y * 0.5,
                -authored.y * 0.5
            );
            assert!(
                (sprite.custom_size.expect("a sized sprite").x - authored.x).abs() < 0.01,
                "the rope is as wide as its box: {:?} in {authored:?}",
                sprite.custom_size
            );
        }
    }

    /// The rope is its tie at the top of the box, its knot at the bottom, and
    /// whole tiles of braid between them, in one column. These are the
    /// slices Bevy draws for the sprite.
    #[test]
    fn a_rope_is_its_cap_then_whole_tiles_then_its_end() {
        use bevy::sprite::SpriteImageMode;
        let asset = baked_prop_asset("cut_rope_rope");
        let slices = asset.spec.column_slices().expect(PUBLISH_THE_ROPE);
        assert!(
            slices.cap > 1.0 && slices.end > 1.0 && (slices.cap + slices.tile + slices.end - slices.rect.y).abs() < 1e-3,
            "premise: the rope has a cap, a tile and an end, and they are its rect: {slices:?}"
        );
        let rect = Rect::from_corners(BVec2::ZERO, slices.rect);
        for (authored, at_least) in [
            (BVec2::new(8.0, 160.0), 10),
            (BVec2::new(8.0, 700.0), 60),
            (BVec2::new(16.0, 320.0), 10),
            // A box that holds the cap, the end and less than two tiles.
            (BVec2::new(8.0, 40.0), 1),
        ] {
            let (sprite, _, animator) = prop_sprite_bundle(PropDraw::Decoration, false, &asset, authored);
            assert!(animator.keeps_its_quad, "a frame may size the rope's quad again");
            let SpriteImageMode::Sliced(slicer) = &sprite.image_mode else {
                panic!("the rope is not drawn in slices: {:?}", sprite.image_mode);
            };
            // Slices that have an area, from the top of the box down.
            let mut drawn: Vec<_> = slicer
                .compute_slices(rect, sprite.custom_size)
                .into_iter()
                .filter(|slice| slice.draw_size.x > 1e-3 && slice.draw_size.y > 1e-3)
                .collect();
            drawn.sort_by(|a, b| b.offset.y.total_cmp(&a.offset.y));
            let fill = slices.fill(authored);
            assert!(fill.tiles as usize >= at_least, "{authored:?} holds {} tiles", fill.tiles);
            assert_eq!(
                drawn.len(),
                fill.tiles as usize + 2,
                "in {authored:?}: the cap, {} tiles and the end. A slice more is a tile cut short \
                 or a second column: {drawn:#?}",
                fill.tiles
            );
            // Each slice is as wide as the box, and they stack with no gap
            // from the top of the box to its bottom.
            let mut edge = authored.y * 0.5;
            for slice in &drawn {
                assert!((slice.draw_size.x - authored.x).abs() < 1e-3, "in {authored:?} a slice is not as wide as the box: {slice:?}");
                let top = slice.offset.y + slice.draw_size.y * 0.5;
                assert!((top - edge).abs() < 1e-2, "in {authored:?} a slice starts at {top}, and the one above ended at {edge}");
                edge = top - slice.draw_size.y;
            }
            assert!((edge + authored.y * 0.5).abs() < 1e-2, "in {authored:?} the slices end at {edge}");
            // The first is the cap, the last is the end, and each one between
            // is the whole tile.
            let (cap, end) = (&drawn[0], &drawn[drawn.len() - 1]);
            assert!(
                cap.texture_rect.min.y.abs() < 1e-3 && (cap.texture_rect.max.y - slices.cap).abs() < 1e-3,
                "the first slice is not the cap: {cap:?}"
            );
            assert!(
                (end.texture_rect.min.y - (slices.rect.y - slices.end)).abs() < 1e-3
                    && (end.texture_rect.max.y - slices.rect.y).abs() < 1e-3,
                "the last slice is not the end: {end:?}"
            );
            // (The last tile gives up the little that
            // `column_image_mode` adds to each tile: under 0.02 of a texel.)
            for tile in &drawn[1..drawn.len() - 1] {
                assert!(
                    (tile.texture_rect.min.y - slices.cap).abs() < 1e-3
                        && (tile.texture_rect.max.y - (slices.cap + slices.tile)).abs() < 0.02,
                    "in {authored:?} a tile is cut: it shows rows {}..{} of {}..{}",
                    tile.texture_rect.min.y,
                    tile.texture_rect.max.y,
                    slices.cap,
                    slices.cap + slices.tile
                );
            }
            // The braid keeps its shape: a tile is not more than one tile's
            // share taller than it is wide for.
            assert!(
                fill.tile_scale >= fill.scale - 1e-4 && fill.tile_scale <= fill.scale * (1.0 + 1.0 / fill.tiles as f32) + 1e-4,
                "in {authored:?} a tile is drawn at {} for a width scale of {}",
                fill.tile_scale,
                fill.scale
            );
        }
    }

    /// The prop tick draws the rope's frame each frame. It leaves the quad
    /// that fills the box: a frame's trimmed rect does not size the rope
    /// again.
    #[test]
    fn the_frame_tick_does_not_size_a_rope_again() {
        let asset = baked_prop_asset("cut_rope_rope");
        assert!(asset.spec.column_slices().is_some(), "{PUBLISH_THE_ROPE}");
        let authored = BVec2::new(8.0, 160.0);
        let (mut sprite, mut anchor, mut animator) = prop_sprite_bundle(PropDraw::Decoration, false, &asset, authored);
        // What `animate_props` does with a prop each frame.
        crate::rendering::actors::draw_held_frame(&mut sprite, &mut animator, &mut anchor, false);
        assert_eq!(
            (sprite.custom_size, anchor),
            (Some(authored), Anchor::CENTER),
            "after one frame the rope does not fill its box"
        );
        assert!(matches!(sprite.image_mode, bevy::sprite::SpriteImageMode::Sliced(_)));
    }

    /// A reduced copy of the rope sheet has the same cap, tile and end, as
    /// parts of its own smaller rect, and each boundary is on a whole texel.
    #[test]
    fn each_quality_tier_of_the_rope_has_the_same_parts() {
        use ambition_sprite_sheet::character::sheets::try_load_spec_for_target_scaled;
        use ambition_sprite_sheet::character::TextureResolutionScale as Scale;
        let full = baked_prop_asset("cut_rope_rope").spec.column_slices().expect(PUBLISH_THE_ROPE);
        let parts = |slices: ambition_sprite_sheet::character::ColumnSlices| {
            [slices.cap / slices.rect.y, slices.tile / slices.rect.y, slices.end / slices.rect.y]
        };
        let mut reduced = 0;
        for scale in [Scale::Half, Scale::Quarter, Scale::Potato] {
            let Some(spec) = try_load_spec_for_target_scaled("cut_rope_rope", &SheetTuning::new(1.0, 0), scale) else {
                continue;
            };
            reduced += 1;
            let slices = spec.column_slices().unwrap_or_else(|| panic!("the {scale:?} copy of the rope declares no column tile"));
            assert!(slices.rect.y < full.rect.y, "premise: the {scale:?} copy is smaller: {slices:?}");
            for (part, (here, there)) in parts(slices).into_iter().zip(parts(full)).enumerate() {
                assert!((here - there).abs() < 1e-4, "{scale:?}: part {part} is {here} of the rect; the full sheet has {there}");
            }
            for rows in [slices.cap, slices.tile, slices.end] {
                assert!((rows - rows.round()).abs() < 1e-3, "{scale:?}: a boundary is inside a texel: {slices:?}");
            }
        }
        assert!(reduced > 0, "no reduced copy of the rope sheet is baked, so this arm compared nothing");
    }

    /// A sheet that declares no column tile is one picture, as before.
    #[test]
    fn a_sheet_with_no_column_tile_is_one_picture() {
        for target in ["cut_rope_anvil", "super_mary_o_pipe_top"] {
            let asset = baked_prop_asset(target);
            assert_eq!(asset.spec.column_slices(), None, "{target}");
            let (sprite, _, animator) = prop_sprite_bundle(PropDraw::Decoration, false, &asset, BVec2::new(64.0, 32.0));
            assert!(
                matches!(sprite.image_mode, bevy::sprite::SpriteImageMode::Auto) && !animator.keeps_its_quad,
                "{target} is drawn in slices"
            );
        }
    }

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
