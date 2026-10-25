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
use ambition_platformer2d_core::config::{world_to_bevy, GRID_STEP, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER};
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
    // The zones a game draws no picture for. Absent: each zone has its picture.
    hidden_zones: Option<Res<super::gate_portal_visuals::HiddenLoadingZones>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
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
            hidden_zones.as_deref(),
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
    // The loading zones that have no picture and no nameplate
    // (`HiddenLoadingZones`): a place to arrive at, or a zone that is a secret.
    hidden_zones: Option<&super::gate_portal_visuals::HiddenLoadingZones>,
) {
    let world = &spec.world;
    spawn_grid(commands, session_scope, world);
    // The theme the room names: its blocks take the terrain skin of it. A
    // room with a look of its own (a `palette`) has a skin too: the look is
    // drawn over published art, so the room keeps its look with each shader
    // off.
    let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::named_by_room_metadata(&spec.metadata);
    spawn_surface_chain_visuals(commands, session_scope, world, theme);
    if let Some(theme) = theme {
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
        // The terrain of the room: where its decor must not stand, and the
        // meshes of its skin, which are its children.
        let size = BVec2::new(world.size.x, world.size.y);
        // A theme with two states lays the skin of each on the same blocks;
        // the room's look shows each one where the room is in that state.
        let states = std::iter::once(theme).chain(theme.corrupted());
        for laid in states {
            commands.spawn_session_scoped(
                session_scope,
                (
                    super::terrain_skin::TerrainKeepOut(keep_out.clone()),
                    super::terrain_skin::RoomTerrain::state_of(theme, laid, size),
                    Transform::default(),
                    Visibility::default(),
                    RoomVisual,
                    Name::new("Terrain of the room"),
                ),
            );
        }
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
        spawn_water_region(commands, session_scope, world, region, theme);
    }
    for region in &world.climbable_regions {
        spawn_climbable_region(commands, session_scope, world, region, theme);
    }
    for zone in &spec.loading_zones {
        // No picture is no nameplate also: the nameplate of a zone that is not
        // drawn named a place nothing showed (the name of the sky zone of
        // Mary-O 1-1, over a level whose secret it is).
        if hidden_zones.is_some_and(|hidden| hidden.0.contains(&zone.id)) {
            continue;
        }
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
    for (index, prop) in spec.props.iter().enumerate() {
        spawn_room_prop(commands, session_scope, world, prop, index, assets);
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

/// Where the scenery of a room draws: behind the actors of the playable plane
/// (`feature_z(Actor)`) and over the limb trail (`WORLD_Z_DUMMY + 0.5`). The
/// room's scenery draws in the order the room lists it, the first furthest
/// back, so two pieces that overlap (a house before an aqueduct) always draw
/// the same way round.
const SCENERY_Z_BACK: f32 = WORLD_Z_DUMMY + 0.55;
const SCENERY_Z_FRONT: f32 = WORLD_Z_DUMMY + 0.95;
/// The z step between pieces of scenery listed one after the other.
const SCENERY_Z_STEP: f32 = 1.0 / 4096.0;

/// Spawn the visual entity for one [`PropSpec`], the `index`th the room
/// lists. Falls back to a coloured rectangle when the prop's `kind` is
/// unknown or its asset has not loaded.
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
    index: usize,
    assets: Option<&GameAssets>,
) {
    // Built world draws in front, so a body inside it is hidden; scenery sits
    // behind the cast, in the room's order.
    let z = if prop.draw.occludes_bodies() {
        WORLD_Z_PLAYER + super::BODY_DEPTH_BAND
    } else {
        (SCENERY_Z_BACK + SCENERY_Z_STEP * index as f32).min(SCENERY_Z_FRONT)
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

/// Backdrop depth: after all regular parallax (-18 through -15), before
/// collision/world surfaces (0). It is not a camera or overlay layer.
const WATER_BACKDROP_Z: f32 = -8.0;

/// Render a water region in its room's coordinates.
///
/// The optional theme backplate is bounded by the real water volume. It draws
/// over camera-relative scenery but behind terrain and characters. The water
/// tint and surface strip retain their independent opacity and depth.
fn spawn_water_region(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    region: &ae::WaterRegion,
    // The theme the room names, if its water takes the art of the theme
    // (`terrain_skin::dress_themed_fixtures`).
    theme: Option<ambition_sprite_sheet::game_assets::ParallaxTheme>,
) {
    use super::terrain_skin::{ThemedFixture, ThemedFixtureKind};
    let fixture = |kind: ThemedFixtureKind, min: ae::Vec2, size: BVec2| {
        theme.map(|theme| ThemedFixture { theme, kind, min: BVec2::new(min.x, min.y), size })
    };
    let region_min = region.aabb.center() - region.aabb.half_size();
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // The themed backdrop is a world-space scene, not another parallax panel.
    // A split view, a merged view, and a portal capture see the same boundary.
    // Other themes can publish their own backdrop without changing this path.
    if let Some(theme) = theme {
        // The port keeps a plain opaque matte until its generated art loads.
        // An unpainted biome has no new matte and retains its former water.
        let fallback = if theme == ambition_sprite_sheet::game_assets::ParallaxTheme::PiratePort {
            Color::srgb(0.12, 0.36, 0.44)
        } else {
            Color::NONE
        };
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(fallback, render),
                Transform::from_translation(world_to_bevy(
                    world,
                    region.aabb.center(),
                    WATER_BACKDROP_Z,
                )),
                ThemedFixture {
                    theme,
                    kind: ThemedFixtureKind::WaterBackdrop,
                    min: BVec2::new(region_min.x, region_min.y),
                    size: render,
                },
                Name::new("Water backdrop"),
                RoomVisual,
            ),
        );
    }
    let (body_color, body_z) = match region.kind {
        // Cool blue, mostly transparent, just above blocks.
        ae::WaterKind::Clear => (Color::srgba(0.24, 0.72, 0.88, 0.32), WORLD_Z_BLOCK + 5.0),
        // Dark teal, near-opaque, above the player so it hides what is below.
        ae::WaterKind::Murky => (Color::srgba(0.10, 0.20, 0.18, 0.88), WORLD_Z_PLAYER + 5.0),
    };
    let mut body = commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(body_color, render),
            Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
            Name::new(format!("Water body ({:?})", region.kind)),
            RoomVisual,
        ),
    );
    let body_kind = match region.kind {
        ae::WaterKind::Clear => ThemedFixtureKind::WaterClear,
        ae::WaterKind::Murky => ThemedFixtureKind::WaterMurky,
    };
    if let Some(fixture) = fixture(body_kind, region_min, render) {
        body.insert(fixture);
    }

    // Surface strip: a 4px band at the top of the region, above the body and
    // the player, so the surface shows even through Murky.
    let strip_color = match region.kind {
        ae::WaterKind::Clear => Color::srgba(0.82, 0.95, 1.0, 0.85),
        ae::WaterKind::Murky => Color::srgba(0.55, 0.78, 0.62, 0.95),
    };
    let strip_h = 4.0;
    let strip_size = BVec2::new(size.x, strip_h);
    let strip_center = ae::Vec2::new(region.aabb.center().x, region.aabb.top() + strip_h * 0.5);
    let mut strip = commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(strip_color, strip_size),
            Transform::from_translation(world_to_bevy(world, strip_center, WORLD_Z_PLAYER + 6.0)),
            Name::new(format!("Water surface ({:?})", region.kind)),
            RoomVisual,
        ),
    );
    if let Some(fixture) = fixture(ThemedFixtureKind::WaterSurface, region_min, strip_size) {
        strip.insert(fixture);
    }
}

#[cfg(test)]
mod water_backdrop_tests {
    use super::*;
    use super::super::terrain_skin::{ThemedFixture, ThemedFixtureKind};

    #[test]
    fn port_water_owns_a_world_aligned_backdrop_between_parallax_and_play() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, |mut commands: Commands| {
            let room = ae::World::new(
                "port_docks",
                ae::Vec2::new(2816.0, 960.0),
                ae::Vec2::ZERO,
                Vec::new(),
            );
            let water = ae::WaterRegion::new(
                ae::aabb_from_min_size(
                    ae::Vec2::new(0.0, 784.0),
                    ae::Vec2::new(2816.0, 144.0),
                ),
                ae::WaterKind::Clear,
                Default::default(),
            );
            spawn_water_region(
                &mut commands,
                SessionSpawnScope::UNSCOPED,
                &room,
                &water,
                Some(ambition_sprite_sheet::game_assets::ParallaxTheme::PiratePort),
            );
        });
        app.update();

        let mut query = app.world_mut().query::<(&Name, &Sprite, &Transform, &ThemedFixture)>();
        let backdrops: Vec<_> = query
            .iter(app.world())
            .filter(|(name, _, _, _)| name.as_str() == "Water backdrop")
            .collect();
        assert_eq!(backdrops.len(), 1);
        let (_, sprite, transform, fixture) = backdrops[0];
        assert_eq!(sprite.custom_size, Some(BVec2::new(2816.0, 144.0)));
        assert_eq!(sprite.color, Color::srgb(0.12, 0.36, 0.44));
        assert_eq!(fixture.kind, ThemedFixtureKind::WaterBackdrop);
        assert_eq!(fixture.min, BVec2::new(0.0, 784.0));
        // A world-space waterline at y=784 in a 960-high room.
        assert_eq!(transform.translation, Vec3::new(0.0, -376.0, WATER_BACKDROP_Z));
        assert!(WATER_BACKDROP_Z > -15.0 && WATER_BACKDROP_Z < WORLD_Z_BLOCK);
    }
}

/// Render a single `ClimbableRegion` as a tinted overlay quad with rung
/// stripes. Placeholder until ladder/vine/wall art exists. Each kind has its
/// own tint so the player can tell what they touch.
fn spawn_climbable_region(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    region: &ae::ClimbableRegion,
    // The theme the room names, if its ladders take the art of the theme
    // (`terrain_skin::dress_themed_fixtures`).
    theme: Option<ambition_sprite_sheet::game_assets::ParallaxTheme>,
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
    let mut body = commands.spawn_session_scoped(
        session_scope,
        (
            Sprite::from_color(body_color, render),
            Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
            Name::new(format!("Climbable body ({:?})", region.kind)),
            RoomVisual,
        ),
    );
    if let (Some(theme), ae::ClimbableKind::Ladder) = (theme, region.kind) {
        let min = region.aabb.center() - region.aabb.half_size();
        body.insert(super::terrain_skin::ThemedFixture {
            theme,
            kind: super::terrain_skin::ThemedFixtureKind::Ladder,
            min: BVec2::new(min.x, min.y),
            size: render,
        });
    }

    // Rung stripes every 16 px on y. Skipped for Wall (rung alpha 0). They
    // are children of the body: the art of a theme takes the place of the
    // body and of its rungs.
    if rung_color.alpha() > 0.0 {
        let rung_h = 3.0;
        let rung_size = BVec2::new(size.x, rung_h);
        let centre_y = region.aabb.center().y;
        let (top, bottom) = (region.aabb.top(), region.aabb.bottom());
        let label = format!("Climbable rung ({:?})", region.kind);
        body.with_children(|parent| {
            let mut y = top + 8.0;
            while y < bottom - 4.0 {
                parent.spawn((
                    Sprite::from_color(rung_color, rung_size),
                    // Room y is down and the y of a child is up.
                    Transform::from_xyz(0.0, centre_y - y, 0.5),
                    Name::new(label.clone()),
                ));
                y += 16.0;
            }
        });
    }
}

/// Draw the simulation's rideable surface chains: the earth under each, and
/// each segment as a thin strip turned along it.
///
/// Generic room presentation: any game that authors a chain gets this visual.
/// In a room that names a `theme`, the earth and the strips take the skin of
/// the theme (`chain_skin`).
pub fn spawn_surface_chain_visuals(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    theme: Option<ambition_sprite_sheet::game_assets::ParallaxTheme>,
) {
    use super::chain_skin::{ChainEarth, ChainTurf, TurfFace, FAR_LANE_DIM};
    const THICKNESS: f32 = 8.0;

    // A hidden chain is drawn by the room's art, not here.
    let drawn = || world.chains.iter().filter(|chain| !chain.hidden);
    for chain in drawn().filter(|chain| chain.filled && !chain.closed) {
        commands.spawn_session_scoped(
            session_scope,
            (
                ChainEarth::under(world, &chain.points, theme),
                Transform::from_xyz(0.0, 0.0, WORLD_Z_BLOCK - 0.5),
                Visibility::Visible,
                Name::new(format!("Filled ground: {}", chain.name)),
                RoomVisual,
            ),
        );
    }

    for chain in drawn().filter(|chain| !chain.earth.is_empty()) {
        commands.spawn_session_scoped(
            session_scope,
            (
                ChainEarth::painted(world, &chain.earth, theme),
                Transform::from_xyz(0.0, 0.0, WORLD_Z_BLOCK - 0.5),
                Visibility::Visible,
                Name::new(format!("Painted earth: {}", chain.name)),
                RoomVisual,
            ),
        );
    }

    // Track in the air (a loop, the ramp into it) has depth lanes and no earth
    // of its own: it is drawn as a band of earth on the side a body does not
    // ride, one band for each lane, at the depth of the lane.
    for chain in drawn().filter(|chain| !chain.depth_lanes.is_empty() && chain.earth.is_empty() && !chain.filled)
    {
        let count = chain.segment_count();
        if count == 0 {
            continue;
        }
        let normal = |segment: usize| {
            let n = chain.normal(segment);
            BVec2::new(n.x, n.y)
        };
        // The way out of the earth at a vertex: between the normals of the
        // two segments that meet at it.
        let out_at = |vertex: usize| {
            let before = if vertex > 0 {
                Some(vertex - 1)
            } else {
                chain.closed.then(|| count - 1)
            };
            let after = if vertex < count { Some(vertex) } else { chain.closed.then_some(0) };
            (before.map_or(BVec2::ZERO, normal) + after.map_or(BVec2::ZERO, normal))
                .try_normalize()
                .unwrap_or(BVec2::NEG_Y)
        };
        let mut lanes: Vec<i8> = (0..count).map(|segment| chain.segment_depth(segment)).collect();
        lanes.sort_unstable();
        lanes.dedup();
        for lane in lanes {
            let segments: Vec<[(BVec2, BVec2); 2]> = (0..count)
                .filter(|segment| chain.segment_depth(*segment) == lane)
                .map(|segment| {
                    let (a, b) = chain.segment(segment);
                    let end = if chain.closed { (segment + 1) % count } else { segment + 1 };
                    [(BVec2::new(a.x, a.y), out_at(segment)), (BVec2::new(b.x, b.y), out_at(end))]
                })
                .collect();
            let (z, dim) = if lane < 0 {
                (WORLD_Z_BLOCK + 0.5, FAR_LANE_DIM)
            } else if lane > 0 {
                (WORLD_Z_PLAYER + 0.3, 1.0)
            } else {
                (WORLD_Z_BLOCK + 1.5, 1.0)
            };
            commands.spawn_session_scoped(
                session_scope,
                (
                    ChainEarth::band(world, &segments, theme, dim),
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::Visible,
                    Name::new(format!("Track earth: {} lane {}", chain.name, lane)),
                    RoomVisual,
                ),
            );
        }
    }

    for chain in drawn() {
        // How far along the chain the segment starts: the cap of a skin goes
        // on along the whole chain.
        let mut along = 0.0;
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
            // Turned along the segment, the strip's own up is the side a body
            // rides (`SurfaceChain::normal`).
            let mut strip = commands.spawn_session_scoped(
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
            if let Some(theme) = theme {
                let normal = chain.normal(segment_index);
                let face = TurfFace::of(BVec2::new(normal.x, normal.y));
                strip.insert(ChainTurf { theme, along, length, lane: depth, face });
            }
            along += length;
        }
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

/// The side of one tile of a tiled block, in world units: the size of the
/// tile art at full resolution, one pixel to a unit.
const BLOCK_TILE_UNITS: f32 = 32.0;

/// The side of one tile of a block of `render` size, in world units:
/// [`BLOCK_TILE_UNITS`], or a whole multiple of it that keeps the count of
/// tiles on an axis under `MAX_TILES_PER_AXIS` (the sprite slicer warns over
/// that; a 3072 by 3328 floor would be 9984 tiles).
fn tiled_block_tile_units(render: BVec2) -> f32 {
    const MAX_TILES_PER_AXIS: f32 = 32.0;
    let tiles = (render.x.max(render.y) / BLOCK_TILE_UNITS).max(1.0);
    BLOCK_TILE_UNITS * (tiles / MAX_TILES_PER_AXIS).max(1.0).ceil()
}

/// A block that repeats its tile, and the side of one tile in world units.
///
/// `SpriteImageMode::Tiled` sizes a tile as `texture px * stretch_value`, so
/// the stretch that gives this side depends on the texture. The texture of a
/// lower quality tier is smaller (16 px at half, 8 px at quarter), and with a
/// stretch chosen for the 32 px one a `?` block drew four or sixteen `?`.
/// [`fit_tiled_block_tiles`] sets the stretch from the image that is bound.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TiledBlockTile {
    pub tile_units: f32,
}

impl TiledBlockTile {
    /// The stretch that draws a tile of this side from a texture `texture_px`
    /// wide.
    pub fn stretch_for(self, texture_px: u32) -> f32 {
        self.tile_units / (texture_px.max(1) as f32)
    }
}

/// Keep each tiled block's tile at its world size for the image it has now:
/// at spawn, when the image arrives, and when a quality change or
/// [`EntityArt`] binds another.
///
/// It reads each tiled block on each frame and writes only one whose stretch
/// is wrong, so a settled room marks no sprite changed. A skinned block draws
/// nothing itself (`terrain_skin`), and its pieces have their own scale.
pub fn fit_tiled_block_tiles(
    images: Res<Assets<Image>>,
    mut blocks: Query<
        (&TiledBlockTile, &mut Sprite),
        Without<super::terrain_skin::TerrainSkinned>,
    >,
) {
    for (tile, mut sprite) in &mut blocks {
        let Some(image) = images.get(&sprite.image) else {
            continue;
        };
        let want = tile.stretch_for(image.width());
        let bevy::sprite::SpriteImageMode::Tiled { tile_x, tile_y, stretch_value } = sprite.image_mode
        else {
            continue;
        };
        if (stretch_value - want).abs() > f32::EPSILON {
            sprite.image_mode =
                bevy::sprite::SpriteImageMode::Tiled { tile_x, tile_y, stretch_value: want };
        }
    }
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
    let tile = TiledBlockTile { tile_units: tiled_block_tile_units(render) };
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
                    // For the full-resolution tile, until the image is
                    // here to say its size (`fit_tiled_block_tiles`).
                    stretch_value: tile.stretch_for(BLOCK_TILE_UNITS as u32),
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
    if is_tiled_surface && block.art_color.is_none() {
        entity.insert(tile);
    }
    // A block that draws the tile of its kind can take the skin of its room.
    // A placeholder colour is authored and a lock wall has its own art.
    let surface_kind = match block.kind {
        ae::BlockKind::Solid => Some(super::terrain_skin::TerrainSurfaceKind::Solid),
        ae::BlockKind::OneWay => Some(super::terrain_skin::TerrainSurfaceKind::OneWay),
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Soft } => {
            Some(super::terrain_skin::TerrainSurfaceKind::BlinkSoft)
        }
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Hard } => {
            Some(super::terrain_skin::TerrainSurfaceKind::BlinkHard)
        }
        ae::BlockKind::Hazard => Some(super::terrain_skin::TerrainSurfaceKind::Hazard),
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

/// How many runs of [`sync_lock_wall_visuals`] a live room must be the same
/// room before a wall of it that opens or shuts is heard: about half a
/// second. The walls of a room are made in its first runs, and the walls of
/// the room before it go in them.
const LOCK_WALL_QUIET_FRAMES: u32 = 30;

/// The authored `look` of the lock wall that the gate solid `block_name` is:
/// the prop sheet that draws it (a door, a gate).
///
/// A gated wall is named by its own id. An encounter's wall is named by its
/// encounter, and it is the room's wall that authors no gate.
fn lock_wall_look<'a>(
    spec: &'a ambition_platformer2d_world::rooms::RoomSpec,
    block_name: &str,
) -> Option<&'a str> {
    let wall = match block_name.strip_prefix("gated_lock:") {
        Some(id) => spec.lock_walls.iter().find(|wall| wall.gated_by.is_some() && wall.id.trim() == id),
        None => spec.lock_walls.iter().find(|wall| wall.gated_by.is_none()),
    };
    wall.and_then(|wall| wall.look.as_deref())
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
///
/// A gated wall that opens or shuts while its room is live is heard: a lock
/// that opens, a gate that falls. A wall that stands when its room becomes
/// live makes no sound, and a wall that goes with its room makes none.
///
/// The room that a body enters can take the live room of the room it left.
/// So "the room was live already" is not asked of the live room alone: a room
/// is heard after it has been the same room (the same root, the same
/// geometry) for [`LOCK_WALL_QUIET_FRAMES`] runs of this system. Each door of
/// a room that had just become live fell with a sound, all at one time
/// (Jon, 2026-10-12).
pub fn sync_lock_wall_visuals(
    mut commands: Commands,
    mut sfx: Option<ambition_sfx::SfxWriter>,
    // Each live room: its root, the name of its geometry, and for how many
    // runs it has been that room.
    mut settled: Local<
        std::collections::HashMap<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, (Entity, String, u32)>,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    // Every live room's walls, each drawn in its own room and stamped with
    // it. A sole-room read did not run while two rooms were live.
    rooms: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &ambition_platformer2d_core::RoomGeometry,
            &ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    assets: Option<Res<GameAssets>>,
    // The authored walls of each live room, for the look a wall names.
    // Absent: a world with no room set, where each wall is the tile.
    specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
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
    let mut existing_by_name: std::collections::HashMap<Key, (Entity, ae::Vec2)> = std::collections::HashMap::new();
    for (entity, visual, room) in &existing {
        existing_by_name.insert((room.map(|room| room.0), visual.block_name.clone()), (entity, visual.centre));
    }
    // Count the runs each live room has been the same room. A room that is
    // new, or that took the place of another, starts again from none.
    let mut heard_rooms = std::collections::HashSet::new();
    let mut now = std::collections::HashMap::new();
    for (root, room, world, _) in &rooms {
        let runs = match settled.get(room) {
            Some((was_root, was_name, runs)) if *was_root == root && *was_name == world.0.name => runs + 1,
            _ => 0,
        };
        if runs >= LOCK_WALL_QUIET_FRAMES {
            heard_rooms.insert(*room);
        }
        now.insert(*room, (root, world.0.name.clone(), runs));
    }
    *settled = now;
    let mut heard = |id: ambition_sfx::SfxId, pos: ae::Vec2| {
        if let Some(sfx) = sfx.as_mut() {
            sfx.write(ambition_sfx::SfxMessage::Play { id, pos });
        }
    };

    // Pass 1: spawn a visual for each lock-wall block without one. Mark
    // consumed keys so pass 2 keeps them.
    let mut consumed: std::collections::HashSet<Key> = std::collections::HashSet::new();
    let blocks = rooms.iter().flat_map(|(_, room, world, overlay)| {
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
        // A gate that shuts in a room that was live already is heard.
        if block.name.starts_with("gated_lock:") && heard_rooms.contains(&room) {
            heard(ambition_sfx::ids::WORLD_GATE_FALL, block.aabb.center());
        }
        let size = block.aabb.half_size() * 2.0;
        let render = BVec2::new(size.x, size.y);
        // A wall that names a look is drawn with that sheet, which fills the
        // box of the wall. A look with no sheet loaded draws the tile.
        let look = specs
            .as_ref()
            .and_then(|specs| {
                let (_, definition) = specs.live_rooms().find(|(live, _)| *live == room)?;
                lock_wall_look(specs.rooms().spec(definition), &block.name)
            })
            .and_then(|kind| assets.as_deref()?.characters.prop_asset_for_kind(kind));
        if let Some(asset) = look {
            let mut wall = commands.spawn_session_scoped(
                session_scope.in_room(Some(room)),
                (
                    prop_sprite_bundle(
                        PropDraw::Structure,
                        false,
                        asset,
                        render,
                    ),
                    Transform::from_translation(world_to_bevy(&world.0, block.aabb.center(), WORLD_Z_BLOCK + 4.0)),
                    Name::new(format!("LockWall: {}", block.name)),
                    LockWallVisual {
                        block_name: block.name.clone(),
                        centre: block.aabb.center(),
                    },
                    RoomVisual,
                ),
            );
            if heard_rooms.contains(&room) {
                wall.insert(LockWallSlide::shuts());
            }
            consumed.insert(key);
            continue;
        }
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
        let mut wall = commands.spawn_session_scoped(
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
                    centre: block.aabb.center(),
                },
                BoundEntitySprite::new(game_assets::EntitySprite::LockWallTile),
                RoomVisual,
            ),
        );
        if heard_rooms.contains(&room) {
            wall.insert(LockWallSlide::shuts());
        }
        consumed.insert(key);
    }

    // Pass 2: despawn visuals whose gate solid is gone.
    for (key, (entity, centre)) in &existing_by_name {
        if !consumed.contains(key) {
            // A wall that opens in a room that stays live is seen to open: it
            // goes up into its lintel, and then its picture goes. A gate is
            // heard also. A wall that goes with its room goes at once.
            if key.0.is_some_and(|room| heard_rooms.contains(&room)) {
                if key.1.starts_with("gated_lock:") {
                    heard(ambition_sfx::ids::WORLD_LOCK_OPEN, *centre);
                }
                commands.entity(*entity).remove::<LockWallVisual>().insert(LockWallSlide::opens());
                continue;
            }
            commands.entity(*entity).try_despawn();
        }
    }
}

/// A lock wall whose picture slides: up into its lintel when it opens, down
/// out of it when it shuts.
///
/// The wall of the simulation opens and shuts at once: this is the picture
/// only. A body can go under a door that is still on its way up.
#[derive(Component, Clone, Debug)]
pub struct LockWallSlide {
    /// The picture goes up, and goes when it is all the way up. `false`: it
    /// comes down, and stays.
    opens: bool,
    elapsed: f32,
    /// The picture as it is when the wall is shut: its size, where it is,
    /// and the part of its image it shows. Read on the first run.
    shut: Option<(BVec2, f32, Option<Rect>)>,
}

impl LockWallSlide {
    fn opens() -> Self {
        Self { opens: true, elapsed: 0.0, shut: None }
    }

    fn shuts() -> Self {
        Self { opens: false, elapsed: 0.0, shut: None }
    }

    /// How long a wall of `height` world units takes: a tall one takes
    /// longer, and a gate comes down faster than it goes up.
    fn seconds(&self, height: f32) -> f32 {
        let up = (0.30 + height / 640.0).clamp(0.35, 0.9);
        if self.opens { up } else { up * 0.45 }
    }
}

/// Slide each lock wall that opens or shuts ([`LockWallSlide`]).
///
/// The picture is cut at the lintel: a door that is part of the way up shows
/// its lower part, in the upper part of its opening. A picture that is a
/// column that tiles (a grille) is not cut: it gets shorter, so its foot goes
/// up and its head stays.
pub fn slide_lock_wall_visuals(
    mut commands: Commands,
    time: Res<Time>,
    mut walls: Query<(Entity, &mut LockWallSlide, &mut Sprite, &mut Transform)>,
) {
    for (entity, mut slide, mut sprite, mut transform) in &mut walls {
        let (size, y, rect) = *slide.shut.get_or_insert_with(|| {
            // The picture is still from here: no animator may set its frame
            // while this cuts it.
            commands.entity(entity).remove::<CharacterAnimator>();
            (sprite.custom_size.unwrap_or(BVec2::ONE), transform.translation.y, sprite.rect)
        });
        slide.elapsed += time.delta_secs();
        let t = (slide.elapsed / slide.seconds(size.y)).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        // How much of the way up the picture is.
        let up = if slide.opens { eased } else { 1.0 - eased };
        if t >= 1.0 {
            if slide.opens {
                commands.entity(entity).try_despawn();
                continue;
            }
            commands.entity(entity).remove::<LockWallSlide>();
        }
        sprite.custom_size = Some(BVec2::new(size.x, (size.y * (1.0 - up)).max(0.01)));
        transform.translation.y = y + size.y * up * 0.5;
        let tiles = matches!(sprite.image_mode, bevy::sprite::SpriteImageMode::Sliced(_));
        if let (Some(rect), false) = (rect, tiles) {
            sprite.rect = Some(Rect { min: BVec2::new(rect.min.x, rect.min.y + rect.height() * up), max: rect.max });
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

/// Run condition of [`flinch_struck_blocks`]: a block was struck, or one is
/// still flinching (the flinch plays out over frames with no new strike).
pub fn a_block_is_struck_or_flinching(
    struck: Option<Res<bevy::ecs::message::Messages<ambition_platformer2d_shared_tangle::block_nudge::BlockStruck>>>,
    flinching: Query<(), With<BlockFlinch>>,
) -> bool {
    // A peek, not a read (`ambition_platformer2d_core::a_message_waits`).
    struck.is_some_and(|struck| !struck.is_empty()) || !flinching.is_empty()
}

/// A block visual mid-flinch. Presentation only; never rewound.
#[derive(Component, Debug)]
pub struct BlockFlinch {
    elapsed: f32,
    /// Where the quad sits at rest, captured on the first frame of the flinch.
    home: Option<Vec3>,
}

/// Run condition of [`sync_removed_block_visuals`]: a live room's overlay
/// names a removed block. With none, no block visual is taken away.
pub fn a_room_has_removed_blocks(
    overlays: Query<&ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>,
) -> bool {
    overlays.iter().any(|overlay| !overlay.removed_block_names.is_empty())
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
mod flinch_tests {
    use super::*;

    /// A struck block rises and settles home over the frames after the
    /// strike, with no strike message on them: the pass runs while a flinch
    /// plays, not only on the frame of the strike.
    #[test]
    fn a_struck_block_flinches_and_settles_home() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
        app.add_message::<ambition_platformer2d_shared_tangle::block_nudge::BlockStruck>();
        app.add_systems(Update, flinch_struck_blocks.run_if(a_block_is_struck_or_flinching));
        let id = ambition_platformer2d_core::GeoId::anon();
        let block = app
            .world_mut()
            .spawn((BlockVisual { block_name: "brick".into(), geo_id: id.clone() }, Transform::default()))
            .id();
        app.update();
        app.world_mut().write_message(ambition_platformer2d_shared_tangle::block_nudge::BlockStruck::new(id));
        app.update();
        let frames = (ambition_platformer2d_shared_tangle::block_nudge::NUDGE_SECONDS / 0.016) as usize + 8;
        let mut rose = false;
        for _ in 0..frames {
            app.update();
            rose |= app.world().get::<Transform>(block).unwrap().translation != Vec3::ZERO;
        }
        assert!(rose, "the struck block never rose");
        assert!(app.world().get::<BlockFlinch>(block).is_none(), "the flinch never finished");
        assert_eq!(app.world().get::<Transform>(block).unwrap().translation, Vec3::ZERO, "the block did not settle home");
    }
}

#[cfg(test)]
mod tiled_block_tile_tests {
    use super::*;

    /// A `?` block is 32 units and its tile is 32 px at full quality, 16 px at
    /// half and 8 px at quarter. At each, the block is one tile.
    #[test]
    fn a_tile_keeps_its_world_size_at_each_texture_tier() {
        for texture_px in [32_u32, 16, 8] {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.init_asset::<Image>();
            app.add_systems(Update, fit_tiled_block_tiles);
            let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: texture_px,
                    height: texture_px,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[0, 0, 0, 255],
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            ));
            let render = BVec2::splat(32.0);
            let tile = TiledBlockTile { tile_units: tiled_block_tile_units(render) };
            let block = app
                .world_mut()
                .spawn((
                    tile,
                    Sprite {
                        image,
                        custom_size: Some(render),
                        image_mode: bevy::sprite::SpriteImageMode::Tiled {
                            tile_x: true,
                            tile_y: true,
                            stretch_value: tile.stretch_for(BLOCK_TILE_UNITS as u32),
                        },
                        ..Default::default()
                    },
                ))
                .id();
            app.update();
            let sprite = app.world().entity(block).get::<Sprite>().expect("the block's sprite");
            let bevy::sprite::SpriteImageMode::Tiled { stretch_value, .. } = sprite.image_mode else {
                panic!("the block stopped tiling");
            };
            assert_eq!(
                texture_px as f32 * stretch_value,
                32.0,
                "a {texture_px} px tile must still be 32 units on a side"
            );
        }
    }

    /// The cap on the count of tiles is a whole multiple of the tile, so a
    /// large floor keeps whole tiles.
    #[test]
    fn a_large_surface_takes_a_whole_multiple_of_the_tile() {
        assert_eq!(tiled_block_tile_units(BVec2::new(32.0, 32.0)), 32.0);
        assert_eq!(tiled_block_tile_units(BVec2::new(1024.0, 64.0)), 32.0);
        assert_eq!(tiled_block_tile_units(BVec2::new(3072.0, 3328.0)), 128.0);
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

    /// The sounds this system wrote, by cue id.
    fn heard(app: &mut App) -> Vec<ambition_sfx::SfxId> {
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .filter_map(|message| match message.request {
                ambition_sfx::SfxMessage::Play { id, .. } => Some(id),
                _ => None,
            })
            .collect()
    }

    /// A door that opens in a room is heard, and so is one that shuts. The
    /// doors a room has when a body enters it are not heard, and the doors
    /// of the room it left are not: the next room takes the live room of the
    /// room before it, and each of its doors fell with a sound at one time.
    #[test]
    fn a_door_is_heard_when_it_opens_and_not_when_its_room_is_entered() {
        let door = || ae::Block::solid("gated_lock:nave_door", ae::Vec2::new(300.0, 300.0), ae::Vec2::new(32.0, 128.0));
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), room());
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            FeatureEcsWorldOverlay { gate_solids: vec![door()], ..Default::default() },
        );
        app.add_systems(Update, sync_lock_wall_visuals);
        let overlay = |app: &mut App, solids: Vec<ae::Block>| {
            ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<FeatureEcsWorldOverlay>(app.world_mut())
                .expect("the live room has a collision overlay")
                .gate_solids = solids;
        };

        // The room is entered with its door shut.
        for _ in 0..LOCK_WALL_QUIET_FRAMES + 5 {
            app.update();
        }
        assert!(heard(&mut app).is_empty(), "a door that stands when its room is entered makes no sound");

        // The door opens, then it shuts again.
        overlay(&mut app, Vec::new());
        app.update();
        assert_eq!(heard(&mut app), vec![ambition_sfx::ids::WORLD_LOCK_OPEN], "a door that opens is heard one time");
        overlay(&mut app, vec![door()]);
        app.update();
        assert_eq!(heard(&mut app), vec![ambition_sfx::ids::WORLD_GATE_FALL], "a door that shuts is heard one time");

        // The body goes to the next room, which takes this live room: the
        // door of the room it left goes, and the doors of the next room come.
        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<RoomGeometry>(app.world_mut())
            .expect("the live room has its geometry")
            .0
            .name = "the next room".into();
        overlay(&mut app, vec![ae::Block::solid("gated_lock:next_door", ae::Vec2::new(500.0, 300.0), ae::Vec2::new(32.0, 128.0))]);
        for _ in 0..5 {
            app.update();
        }
        assert!(heard(&mut app).is_empty(), "the doors of a room that is entered make no sound");
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
