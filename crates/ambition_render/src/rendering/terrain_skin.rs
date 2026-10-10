//! Lay the terrain skin of a room's theme on its blocks.
//!
//! A room is made of collision blocks: rectangles. With no skin, each block
//! draws the tile of its kind, the same grey brick in each biome. A theme
//! that has a skin (`RoomDressingSet` in `ambition_sprite_sheet`) gives each
//! solid block its fill, and gives each edge of the block that no other
//! block covers a trim: a cap on a top edge, an underside on a bottom edge
//! and a shade on a left or right edge. A one-way platform draws the skin's
//! platform. A few things of the theme's decor (a barrel, a crystal, a stone
//! lantern) stand on the open top edges, clear of each thing of the play
//! ([`TerrainKeepOut`]).
//!
//! The pattern of a skin is fixed to the room, not to a block: each part is
//! drawn in pieces that are cut where the pattern repeats ([`anchored_pieces`]),
//! and each piece shows the part of the picture that is at its place in the
//! room. So the pattern goes on with no break from a block to the block next
//! to it. A room whose ground is many small blocks (each intro room is cells
//! of 16) would show the same corner of the picture on each one.
//!
//! The art is published textures and this is sprites: the terrain of a room
//! looks the same with each shader off.
//!
//! The sizes below are in world units and are the sizes the art is drawn to
//! (`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/terrain/skins.py`).
//! Change the two together.
//!
//! Limits:
//!
//! - The trims are found one time, when the skin is laid. A block that is
//!   removed later (a broken brick) takes its own trims with it, and the
//!   blocks next to it do not get the trim of the edge that is now open.
//! - A block with art of its own (`EntityArt`) and a lock wall keep their
//!   art.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_sprite_sheet::game_assets::{
    GameAssets, ParallaxTheme, RoomDressingPart, DECOR_CELL_PX, DECOR_VARIANTS,
};

use super::primitives::EntityArt;
use super::world::BoundEntitySprite;

/// Texture pixels to the world unit in each part of a skin.
pub const SKIN_PX_PER_UNIT: f32 = 2.0;
/// The height of the cap picture.
pub const CAP_HEIGHT: f32 = 20.0;
/// How far below the top of the cap picture the surface of the block is.
pub const CAP_SURFACE: f32 = 4.0;
/// The height of the underside picture.
pub const UNDER_HEIGHT: f32 = 16.0;
/// How far below the top of the underside picture the bottom of the block is.
pub const UNDER_EDGE: f32 = 4.0;
/// The width of the side picture.
pub const SIDE_WIDTH: f32 = 16.0;
/// The side of one square of the decor, in world units. The player is 48
/// high: a barrel is a third of that and a lamp post is three quarters.
pub const DECOR_SIZE: f32 = 44.0;
/// The distance after which each part of a skin repeats, along the direction
/// it is laid in. (The one-way platform repeats down each 16.)
pub const SKIN_PERIOD: f32 = 64.0;
/// A block whose fill would be more pieces than this draws it as one sprite
/// that repeats from the corner of the block. The pattern of so large a block
/// meets no other block's.
const MOST_FILL_PIECES: usize = 400;
/// An open edge shorter than this gets no trim.
const SHORTEST_TRIM: f32 = 4.0;
/// Two blocks this near are in contact.
const CONTACT: f32 = 0.5;
/// How far into a blink wall the line of light on its edge goes, in world
/// units (`terrain/fixtures.py`, `BLINK_EDGE`).
const BLINK_EDGE_HEIGHT: f32 = 6.0;

/// What a block is to the skin of its room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainSurfaceKind {
    /// Solid ground: it takes the fill and the trims.
    Solid,
    /// A one-way platform: it takes the platform picture. It covers no edge
    /// of a block next to it.
    OneWay,
    /// A wall that is drawn its own way. It takes no skin, and it covers the
    /// edges of the solid blocks it is in contact with.
    Cover,
    /// A wall a blink goes through. It takes the field of light and a line
    /// of light on each open edge, and it covers edges as [`Self::Cover`]
    /// does.
    BlinkSoft,
    /// A wall no blink goes through. It takes the armour and the line.
    BlinkHard,
}

/// A block visual the skin of its room can be laid on. `spawn_block` puts it
/// on the sprite of a block whose room names a theme.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TerrainSurface {
    pub theme: ParallaxTheme,
    pub kind: TerrainSurfaceKind,
    /// The upper left corner of the block, in the coordinates of its room
    /// (y is down).
    pub min: Vec2,
    pub size: Vec2,
}

impl TerrainSurface {
    fn max(&self) -> Vec2 {
        self.min + self.size
    }

    fn covers_edges(&self) -> bool {
        !matches!(self.kind, TerrainSurfaceKind::OneWay)
    }
}

/// Where the decor of a room must not stand: the footprint of each thing of
/// the play (a door, a pickup, a hazard, a place an actor comes in). One for
/// each room, from `spawn_room_visuals`. Each rectangle is `(min, max)` in the
/// coordinates of the room.
#[derive(Component, Clone, Debug, Default)]
pub struct TerrainKeepOut(pub Vec<(Vec2, Vec2)>);

impl TerrainKeepOut {
    /// True when a thing of the decor that stands at `x` on a surface at
    /// height `surface` is clear of each rectangle.
    pub fn clear_at(&self, x: f32, surface: f32) -> bool {
        // A thing of the decor is `DECOR_SIZE` wide and high. The margin
        // keeps it from the edge of a door frame or of a sprite that is
        // larger than its footprint.
        let (left, right) = (x - DECOR_SIZE * 0.5 - 4.0, x + DECOR_SIZE * 0.5 + 4.0);
        let (top, bottom) = (surface - DECOR_SIZE - 8.0, surface + 4.0);
        !self
            .0
            .iter()
            .any(|(min, max)| min.x < right && max.x > left && min.y < bottom && max.y > top)
    }
}

/// How much decor a room has.
#[derive(Resource, Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Resource)]
pub struct TerrainDecorDensity {
    /// The mean distance between two things of the decor on an open top
    /// edge, in world units. 0 or less puts no decor.
    pub spacing: f32,
}

impl Default for TerrainDecorDensity {
    fn default() -> Self {
        Self { spacing: 150.0 }
    }
}

/// A thing of the decor: a child of the block it stands on.
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainDecorItem;

/// A number from 0 to 1 for `n`, the same each time.
fn dice(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x7F4A_7C15;
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Where the decor stands on the open top span `a..b` of a block whose top
/// is at `surface`, and which square of the strip each one is: `(x, variant)`.
/// The answer is the same each time for one span.
pub fn decor_on_span(a: f32, b: f32, surface: f32, spacing: f32, keep_out: Option<&TerrainKeepOut>) -> Vec<(f32, u32)> {
    let margin = DECOR_SIZE * 0.5 + 2.0;
    if spacing <= 0.0 || b - a < 2.0 * margin + 8.0 {
        return Vec::new();
    }
    let seed = (a as i32 as u32).wrapping_mul(73_856_093) ^ (surface as i32 as u32).wrapping_mul(19_349_663);
    let mut out = Vec::new();
    let mut index = 0u32;
    let mut x = a + margin + dice(seed) * spacing;
    while x < b - margin {
        let roll = |salt: u32| dice(seed.wrapping_add(index.wrapping_mul(2_654_435_761)).wrapping_add(salt));
        if keep_out.is_none_or(|keep_out| keep_out.clear_at(x, surface)) {
            let variant = ((roll(1) * DECOR_VARIANTS as f32) as u32).min(DECOR_VARIANTS - 1);
            out.push((x, variant));
        }
        x += spacing * (0.55 + 0.9 * roll(2));
        index += 1;
    }
    out
}

/// A door visual of a room that names a theme: it takes the door of the
/// theme when the theme has one. `spawn_loading_zone` puts it on.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemedDoor(pub ParallaxTheme);

/// The skin question of this block is answered: it has its skin, or its
/// theme has none.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TerrainSkinned;

/// A piece of the skin of a block: a child of the block.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainTrim {
    /// A piece of the fill, or of a one-way platform.
    Fill,
    Cap,
    Under,
    SideLeft,
    SideRight,
    /// The line of light on an edge of a blink wall.
    Rim,
}

/// An edge of a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// The parts of `lo..hi` that none of `covers` covers, in order.
pub fn open_spans(lo: f32, hi: f32, covers: impl IntoIterator<Item = (f32, f32)>) -> Vec<(f32, f32)> {
    let mut covers: Vec<(f32, f32)> = covers
        .into_iter()
        .map(|(a, b)| (a.max(lo), b.min(hi)))
        .filter(|(a, b)| b > a)
        .collect();
    covers.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut open = Vec::new();
    let mut at = lo;
    for (a, b) in covers {
        if a > at {
            open.push((at, a));
        }
        at = at.max(b);
    }
    if hi > at {
        open.push((at, hi));
    }
    open
}

/// The parts of `edge` of `block` that no block of `others` is in contact
/// with: along x for the top and the bottom, along y for the left and the
/// right. `others` may have `block` in it.
pub fn open_edge_spans(block: &TerrainSurface, edge: Edge, others: &[TerrainSurface]) -> Vec<(f32, f32)> {
    let (min, max) = (block.min, block.max());
    let covers = others.iter().filter(|other| other.covers_edges() && *other != block).filter_map(|other| {
        let (omin, omax) = (other.min, other.max());
        // `other` covers the edge where it has a point just past the edge.
        let past = match edge {
            Edge::Top => min.y - CONTACT,
            Edge::Bottom => max.y + CONTACT,
            Edge::Left => min.x - CONTACT,
            Edge::Right => max.x + CONTACT,
        };
        match edge {
            Edge::Top | Edge::Bottom => (omin.y < past && omax.y > past).then_some((omin.x, omax.x)),
            Edge::Left | Edge::Right => (omin.x < past && omax.x > past).then_some((omin.y, omax.y)),
        }
    });
    let (lo, hi) = match edge {
        Edge::Top | Edge::Bottom => (min.x, max.x),
        Edge::Left | Edge::Right => (min.y, max.y),
    };
    open_spans(lo, hi, covers).into_iter().filter(|(a, b)| b - a >= SHORTEST_TRIM).collect()
}

/// Cut `lo..hi` where a pattern of `period` repeats: `(start, end, offset)`
/// for each piece, in order. `offset` is how far into the pattern the piece
/// starts. The pattern is fixed to the coordinates: a piece at 70..100 of a
/// pattern of 64 starts 6 into it, in each block that has that place.
pub fn anchored_pieces(lo: f32, hi: f32, period: f32) -> Vec<(f32, f32, f32)> {
    let mut out = Vec::new();
    let mut at = lo;
    while at < hi - 1e-3 {
        let cell = (at / period + 1e-4).floor();
        let end = ((cell + 1.0) * period).min(hi);
        out.push((at, end, (at - cell * period).max(0.0)));
        at = end;
    }
    out
}

/// A sprite that shows the part of `image` that starts at `texture_min` and
/// is `size`, both in world units, at the scale of the skins.
fn piece(image: Handle<Image>, texture_min: Vec2, size: Vec2, flip_x: bool) -> Sprite {
    Sprite {
        image,
        rect: Some(Rect::from_corners(texture_min * SKIN_PX_PER_UNIT, (texture_min + size) * SKIN_PX_PER_UNIT)),
        custom_size: Some(size),
        flip_x,
        ..Default::default()
    }
}

/// A sprite of `size` that repeats `image` from its own corner, at the scale
/// of the skins: the fill of a block that is too large for pieces.
fn repeated(image: Handle<Image>, size: Vec2) -> Sprite {
    // A tile is `texture px * stretch_value` units. A large surface makes
    // each tile a whole number of times larger, so the count of tiles on an
    // axis stays under the limit of the sprite slicer (`spawn_block`).
    const MOST_TILES: f32 = 32.0;
    let larger = (size.x.max(size.y) / SKIN_PERIOD / MOST_TILES).max(1.0).ceil();
    Sprite {
        image,
        custom_size: Some(size),
        image_mode: bevy::sprite::SpriteImageMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: larger / SKIN_PX_PER_UNIT,
        },
        ..Default::default()
    }
}

/// The pieces of a picture that fills the block `min..min + size`, with a
/// pattern of `period`: `(sprite, place in the block)` for each. `None` when
/// there would be more than [`MOST_FILL_PIECES`].
fn fill_pieces(image: &Handle<Image>, surface: &TerrainSurface, period: Vec2, z: f32) -> Option<Vec<(TerrainTrim, Sprite, Vec3)>> {
    pieces_of(image, surface.min, surface.size, period, z)
}

/// [`fill_pieces`] for a rectangle that is not a block. A period of 0 on an
/// axis is a picture that does not repeat on it: one piece, from its start.
fn pieces_of(image: &Handle<Image>, min: Vec2, size: Vec2, period: Vec2, z: f32) -> Option<Vec<(TerrainTrim, Sprite, Vec3)>> {
    let max = min + size;
    let cut = |lo: f32, hi: f32, period: f32| {
        if period > 0.0 { anchored_pieces(lo, hi, period) } else { vec![(lo, hi, 0.0)] }
    };
    let across = cut(min.x, max.x, period.x);
    let down = cut(min.y, max.y, period.y);
    if across.len() * down.len() > MOST_FILL_PIECES {
        return None;
    }
    let centre = min + size * 0.5;
    let mut out = Vec::with_capacity(across.len() * down.len());
    for &(x0, x1, ox) in &across {
        for &(y0, y1, oy) in &down {
            out.push((
                TerrainTrim::Fill,
                piece(image.clone(), Vec2::new(ox, oy), Vec2::new(x1 - x0, y1 - y0), false),
                Vec3::new((x0 + x1) * 0.5 - centre.x, centre.y - (y0 + y1) * 0.5, z),
            ));
        }
    }
    Some(out)
}

/// Lay the skin on each block visual whose skin question is open.
///
/// It waits for the skin of the theme to be asked for (the room's loader does
/// that with the parallax of the theme), so a block that comes before its
/// skin is skinned when the skin comes. It runs after `apply_entity_art`: a
/// block with art of its own keeps it.
pub fn skin_terrain_surfaces(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut open: Query<
        (Entity, &TerrainSurface, &mut Sprite, Option<&InRoomInstance>),
        (Without<TerrainSkinned>, Without<EntityArt>),
    >,
    all: Query<(&TerrainSurface, Option<&InRoomInstance>)>,
    keep_outs: Query<(&TerrainKeepOut, Option<&InRoomInstance>)>,
    density: Option<Res<TerrainDecorDensity>>,
) {
    let Some(assets) = assets else {
        return;
    };
    if open.is_empty() {
        return;
    }
    let skins = &assets.room_dressing;
    let spacing = density.map_or(TerrainDecorDensity::default().spacing, |density| density.spacing);
    for (entity, surface, mut sprite, room) in &mut open {
        if !skins.attempted(surface.theme) {
            continue;
        }
        let part = |part| skins.get(surface.theme, part).cloned();
        // `try_insert` and `queue_silenced`-free child spawns: a block visual
        // is room-scoped, and a room transition can despawn it before the
        // command flush (`deferred_write_safety`).
        commands.entity(entity).try_insert(TerrainSkinned);
        let centre = surface.min + surface.size * 0.5;
        let half = surface.size * 0.5;
        let mut trims: Vec<(TerrainTrim, Sprite, Vec3)> = Vec::new();
        // The pieces that are turned: the line on a left or right edge.
        let mut turned: Vec<(TerrainTrim, Sprite, Transform)> = Vec::new();
        let mut decor_items: Vec<(Sprite, Vec3)> = Vec::new();
        // The sprite of the block is kept, with its size, and draws nothing:
        // the pieces are its children and go where it goes.
        let hidden = Sprite::from_color(Color::NONE, surface.size);
        match surface.kind {
            TerrainSurfaceKind::Cover => {}
            TerrainSurfaceKind::BlinkSoft | TerrainSurfaceKind::BlinkHard => {
                let field = if surface.kind == TerrainSurfaceKind::BlinkSoft {
                    RoomDressingPart::BlinkSoft
                } else {
                    RoomDressingPart::BlinkHard
                };
                let Some(fill) = part(field) else {
                    continue;
                };
                commands.entity(entity).try_remove::<BoundEntitySprite>();
                match fill_pieces(&fill, surface, Vec2::splat(SKIN_PERIOD), 0.005) {
                    Some(pieces) => {
                        *sprite = hidden;
                        trims.extend(pieces);
                    }
                    None => *sprite = repeated(fill, surface.size),
                }
                if let Some(image) = part(RoomDressingPart::BlinkEdge) {
                    let room = room.map(|stamp| stamp.0);
                    let others: Vec<TerrainSurface> = all
                        .iter()
                        .filter(|(_, stamp)| stamp.map(|stamp| stamp.0) == room)
                        .map(|(other, _)| *other)
                        .collect();
                    let depth = BLINK_EDGE_HEIGHT.min(surface.size.x).min(surface.size.y);
                    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                        for (a, b) in open_edge_spans(surface, edge, &others) {
                            for (lo, hi, offset) in anchored_pieces(a, b, SKIN_PERIOD) {
                                let line = piece(image.clone(), Vec2::new(offset, 0.0), Vec2::new(hi - lo, depth), false);
                                let along = (lo + hi) * 0.5;
                                // The top row of the picture is the outer
                                // side: the piece is turned so that row is
                                // on the edge.
                                let (at, turn) = match edge {
                                    Edge::Top => (Vec2::new(along - centre.x, half.y - depth * 0.5), 0.0),
                                    Edge::Bottom => (Vec2::new(along - centre.x, depth * 0.5 - half.y), std::f32::consts::PI),
                                    Edge::Left => (Vec2::new(depth * 0.5 - half.x, centre.y - along), std::f32::consts::FRAC_PI_2),
                                    Edge::Right => (Vec2::new(half.x - depth * 0.5, centre.y - along), -std::f32::consts::FRAC_PI_2),
                                };
                                turned.push((
                                    TerrainTrim::Rim,
                                    line,
                                    Transform::from_translation(at.extend(0.02)).with_rotation(Quat::from_rotation_z(turn)),
                                ));
                            }
                        }
                    }
                }
            }
            TerrainSurfaceKind::OneWay => {
                if let Some(image) = part(RoomDressingPart::OneWay) {
                    // The quality refresh rebinds a `BoundEntitySprite` to the
                    // tile of its kind, which would take the skin off again.
                    commands.entity(entity).try_remove::<BoundEntitySprite>();
                    match fill_pieces(&image, surface, Vec2::new(SKIN_PERIOD, 16.0), 0.005) {
                        Some(pieces) => {
                            *sprite = hidden;
                            trims.extend(pieces);
                        }
                        None => *sprite = repeated(image, surface.size),
                    }
                }
            }
            TerrainSurfaceKind::Solid => {
                let Some(fill) = part(RoomDressingPart::Fill) else {
                    continue;
                };
                commands.entity(entity).try_remove::<BoundEntitySprite>();
                match fill_pieces(&fill, surface, Vec2::splat(SKIN_PERIOD), 0.005) {
                    Some(pieces) => {
                        *sprite = hidden;
                        trims.extend(pieces);
                    }
                    None => *sprite = repeated(fill, surface.size),
                }
                let room = room.map(|stamp| stamp.0);
                let others: Vec<TerrainSurface> = all
                    .iter()
                    .filter(|(_, stamp)| stamp.map(|stamp| stamp.0) == room)
                    .map(|(other, _)| *other)
                    .collect();
                if let Some(image) = part(RoomDressingPart::Side) {
                    for (edge, trim) in [(Edge::Left, TerrainTrim::SideLeft), (Edge::Right, TerrainTrim::SideRight)] {
                        let right = edge == Edge::Right;
                        let width = SIDE_WIDTH.min(surface.size.x);
                        let x = if right { half.x - width * 0.5 } else { width * 0.5 - half.x };
                        for (a, b) in open_edge_spans(surface, edge, &others) {
                            for (y0, y1, oy) in anchored_pieces(a, b, SKIN_PERIOD) {
                                trims.push((
                                    trim,
                                    piece(image.clone(), Vec2::new(0.0, oy), Vec2::new(width, y1 - y0), right),
                                    Vec3::new(x, centre.y - (y0 + y1) * 0.5, 0.01),
                                ));
                            }
                        }
                    }
                }
                if let Some(image) = part(RoomDressingPart::Under) {
                    for (a, b) in open_edge_spans(surface, Edge::Bottom, &others) {
                        for (x0, x1, ox) in anchored_pieces(a, b, SKIN_PERIOD) {
                            trims.push((
                                TerrainTrim::Under,
                                piece(image.clone(), Vec2::new(ox, 0.0), Vec2::new(x1 - x0, UNDER_HEIGHT), false),
                                Vec3::new((x0 + x1) * 0.5 - centre.x, -half.y + UNDER_EDGE - UNDER_HEIGHT * 0.5, 0.02),
                            ));
                        }
                    }
                }
                let top_spans = open_edge_spans(surface, Edge::Top, &others);
                if let Some(image) = part(RoomDressingPart::Cap) {
                    for &(a, b) in &top_spans {
                        for (x0, x1, ox) in anchored_pieces(a, b, SKIN_PERIOD) {
                            trims.push((
                                TerrainTrim::Cap,
                                piece(image.clone(), Vec2::new(ox, 0.0), Vec2::new(x1 - x0, CAP_HEIGHT), false),
                                Vec3::new((x0 + x1) * 0.5 - centre.x, half.y + CAP_SURFACE - CAP_HEIGHT * 0.5, 0.03),
                            ));
                        }
                    }
                }
                let mut decor: Vec<(Sprite, Vec3)> = Vec::new();
                if let Some(image) = part(RoomDressingPart::Decor) {
                    let keep_out = keep_outs.iter().find(|(_, stamp)| stamp.map(|stamp| stamp.0) == room).map(|(keep_out, _)| keep_out);
                    let cell = DECOR_CELL_PX as f32;
                    for &(a, b) in &top_spans {
                        for (x, variant) in decor_on_span(a, b, surface.min.y, spacing, keep_out) {
                            let variant = variant as f32;
                            decor.push((
                                Sprite {
                                    image: image.clone(),
                                    rect: Some(Rect::new(variant * cell, 0.0, (variant + 1.0) * cell, cell)),
                                    custom_size: Some(Vec2::splat(DECOR_SIZE)),
                                    ..Default::default()
                                },
                                // It stands on the surface, one unit into the cap.
                                Vec3::new(x - centre.x, half.y + DECOR_SIZE * 0.5 - 1.5, 0.04),
                            ));
                        }
                    }
                }
                decor_items = decor;
            }
        }
        if trims.is_empty() && decor_items.is_empty() && turned.is_empty() {
            continue;
        }
        commands.queue(move |world: &mut World| {
            // The block can be gone: its room retired this frame.
            let Ok(mut block) = world.get_entity_mut(entity) else {
                return;
            };
            block.with_children(|parent| {
                for (trim, sprite, at) in trims {
                    parent.spawn((sprite, Transform::from_translation(at), trim, Name::new("Terrain skin piece")));
                }
                for (trim, sprite, at) in turned {
                    parent.spawn((sprite, at, trim, Name::new("Terrain skin piece")));
                }
                for (sprite, at) in decor_items {
                    parent.spawn((sprite, Transform::from_translation(at), TerrainDecorItem, Name::new("Terrain decor")));
                }
            });
        });
    }
}

/// What a fixture of a room is to the art of its theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemedFixtureKind {
    Ladder,
    WaterClear,
    WaterMurky,
    WaterSurface,
}

/// A fixture visual of a room that names a theme (a ladder, a body of water,
/// the line of its surface): it is drawn as a flat colour until the art of
/// the theme takes its place. `min` and `size` are its rectangle in the
/// coordinates of its room (y is down).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ThemedFixture {
    pub theme: ParallaxTheme,
    pub kind: ThemedFixtureKind,
    pub min: Vec2,
    pub size: Vec2,
}

/// The height of the surface picture of water, and how far below its top the
/// line of the water is (`terrain/fixtures.py`).
const WATER_SURFACE_HEIGHT: f32 = 8.0;
const WATER_SURFACE_LINE: f32 = 2.6;

/// Give each ladder and each body of water of a room that names a theme the
/// art of that theme, in pieces that are fixed to the room.
///
/// The flat colour and the rungs of the placeholder go when the art comes.
/// A theme with no art for a fixture leaves the placeholder as it is.
pub fn dress_themed_fixtures(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut fixtures: Query<(Entity, &ThemedFixture, &mut Sprite), Without<TerrainSkinned>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, fixture, mut sprite) in &mut fixtures {
        if !assets.room_dressing.attempted(fixture.theme) {
            continue;
        }
        commands.entity(entity).try_insert(TerrainSkinned);
        let (part, period, min, size) = match fixture.kind {
            ThemedFixtureKind::Ladder => (RoomDressingPart::Ladder, Vec2::new(16.0, 32.0), fixture.min, fixture.size),
            ThemedFixtureKind::WaterClear => (RoomDressingPart::WaterClear, Vec2::splat(SKIN_PERIOD), fixture.min, fixture.size),
            ThemedFixtureKind::WaterMurky => (RoomDressingPart::WaterMurky, Vec2::splat(SKIN_PERIOD), fixture.min, fixture.size),
            // The picture is higher than the strip it takes the place of, and
            // its line goes on the top of the water.
            ThemedFixtureKind::WaterSurface => (
                RoomDressingPart::WaterSurface,
                Vec2::new(SKIN_PERIOD, 0.0),
                Vec2::new(fixture.min.x, fixture.min.y - WATER_SURFACE_LINE),
                Vec2::new(fixture.size.x, WATER_SURFACE_HEIGHT),
            ),
        };
        let Some(image) = assets.room_dressing.get(fixture.theme, part) else {
            continue;
        };
        // The pieces are placed from the middle of the rectangle they fill,
        // and the sprite is at the middle of the fixture: the difference.
        let shift = (min + size * 0.5) - (fixture.min + fixture.size * 0.5);
        let Some(pieces) = pieces_of(image, min, size, period, 0.005) else {
            continue;
        };
        *sprite = Sprite::from_color(Color::NONE, fixture.size);
        commands.queue(move |world: &mut World| {
            let Ok(mut body) = world.get_entity_mut(entity) else {
                return;
            };
            body.despawn_related::<Children>();
            body.with_children(|parent| {
                for (trim, sprite, at) in pieces {
                    let at = at + Vec3::new(shift.x, -shift.y, 0.0);
                    parent.spawn((sprite, Transform::from_translation(at), trim, Name::new("Fixture art piece")));
                }
            });
        });
    }
}

/// Give each door of a room that names a theme the door of that theme.
///
/// A door keeps its size and its place: the door of each theme has the shape
/// of the door of the entity sheet. A door the look of a room has dressed
/// (`EntityArt`) keeps that art.
pub fn dress_themed_doors(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut doors: Query<(Entity, &ThemedDoor, &mut Sprite), (Without<TerrainSkinned>, Without<EntityArt>)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, ThemedDoor(theme), mut sprite) in &mut doors {
        if !assets.room_dressing.attempted(*theme) {
            continue;
        }
        commands.entity(entity).try_insert(TerrainSkinned);
        if let Some(image) = assets.room_dressing.get(*theme, RoomDressingPart::Door) {
            sprite.image = image.clone();
            // The quality refresh rebinds a `BoundEntitySprite` to the door
            // of the entity sheet.
            commands.entity(entity).try_remove::<BoundEntitySprite>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// A ladder takes the ladder of its theme in pieces that cover it, and its
    /// placeholder rungs go. The surface of a body of water is higher than the
    /// strip it takes the place of, and its line is on the top of the water.
    /// The control is a theme with no art: the placeholder stays.
    #[test]
    fn a_fixture_takes_the_art_of_its_theme_and_its_placeholder_goes() {
        let mut world = World::new();
        let mut assets = GameAssets::default();
        for &part in RoomDressingPart::ALL {
            assets.room_dressing.insert(ParallaxTheme::Cave, part, Handle::default());
        }
        assets.room_dressing.mark_attempted(ParallaxTheme::Lab);
        world.insert_resource(assets);
        let fixture = |world: &mut World, theme: ParallaxTheme, kind: ThemedFixtureKind, min: Vec2, size: Vec2| {
            let body = world.spawn((ThemedFixture { theme, kind, min, size }, Sprite::from_color(Color::WHITE, size))).id();
            world.entity_mut(body).with_children(|parent| {
                parent.spawn(Name::new("rung"));
            });
            body
        };
        let ladder = fixture(&mut world, ParallaxTheme::Cave, ThemedFixtureKind::Ladder, Vec2::new(96.0, 64.0), Vec2::new(16.0, 96.0));
        let surface = fixture(&mut world, ParallaxTheme::Cave, ThemedFixtureKind::WaterSurface, Vec2::new(0.0, 200.0), Vec2::new(128.0, 4.0));
        let plain = fixture(&mut world, ParallaxTheme::Lab, ThemedFixtureKind::Ladder, Vec2::new(96.0, 64.0), Vec2::new(16.0, 96.0));
        world.run_system_once(dress_themed_fixtures).unwrap();

        let pieces = trims_of(&mut world, ladder);
        assert_eq!(pieces.iter().map(|(_, size, _)| size.x * size.y).sum::<f32>(), 16.0 * 96.0, "the pieces cover the ladder");
        assert_eq!(world.entity(ladder).get::<Children>().unwrap().len(), pieces.len(), "the rung of the placeholder is gone");
        assert_eq!(world.entity(ladder).get::<Sprite>().unwrap().color, Color::NONE);

        let strip = trims_of(&mut world, surface);
        assert!(strip.iter().all(|(_, size, _)| size.y == WATER_SURFACE_HEIGHT));
        // The strip is 4 high with its middle 2 below the top of the water.
        // The top of the picture is `WATER_SURFACE_LINE` over the top of the
        // water, so its middle is 4 - 2.6 = 1.4 below the top: 0.6 over the
        // middle of the strip.
        assert!(strip.iter().all(|(_, _, at)| (at.y - 0.6).abs() < 1e-4), "{strip:?}");

        assert_eq!(world.entity(plain).get::<Sprite>().unwrap().color, Color::WHITE, "a theme with no ladder");
        assert_eq!(world.entity(plain).get::<Children>().unwrap().len(), 1, "and its rung stays");
    }

    /// A door of a room whose theme has a door takes it and keeps its size.
    /// The controls: a theme with no door leaves the sprite as it is, and a
    /// door the look of a room has dressed keeps that art.
    #[test]
    fn a_door_takes_the_door_of_its_theme_and_keeps_its_size() {
        let mut world = World::new();
        let mut images = Assets::<Image>::default();
        let themed = images.add(Image::default());
        let plain = images.add(Image::default());
        let mut assets = GameAssets::default();
        assets.room_dressing.insert(ParallaxTheme::Cave, RoomDressingPart::Door, themed.clone());
        assets.room_dressing.mark_attempted(ParallaxTheme::Lab);
        world.insert_resource(assets);
        let door = |world: &mut World, theme: ParallaxTheme| {
            world
                .spawn((ThemedDoor(theme), Sprite { image: plain.clone(), custom_size: Some(Vec2::new(40.0, 80.0)), ..Default::default() }))
                .id()
        };
        let (cave, lab) = (door(&mut world, ParallaxTheme::Cave), door(&mut world, ParallaxTheme::Lab));
        let dressed = door(&mut world, ParallaxTheme::Cave);
        world.entity_mut(dressed).insert(EntityArt(ambition_sprite_sheet::game_assets::EntitySprite::DoorZone));
        world.run_system_once(dress_themed_doors).unwrap();
        let sprite = |world: &World, entity: Entity| world.entity(entity).get::<Sprite>().unwrap().clone();
        assert_eq!(sprite(&world, cave).image, themed);
        assert_eq!(sprite(&world, cave).custom_size, Some(Vec2::new(40.0, 80.0)));
        assert_eq!(sprite(&world, lab).image, plain, "a theme with no door");
        assert_eq!(sprite(&world, dressed).image, plain, "a door with art of its own");
    }

    fn surface(kind: TerrainSurfaceKind, x: f32, y: f32, w: f32, h: f32) -> TerrainSurface {
        TerrainSurface { theme: ParallaxTheme::Cave, kind, min: Vec2::new(x, y), size: Vec2::new(w, h) }
    }

    #[test]
    fn the_open_part_of_a_span_is_what_no_cover_takes() {
        assert_eq!(open_spans(0.0, 100.0, []), vec![(0.0, 100.0)]);
        assert_eq!(open_spans(0.0, 100.0, [(20.0, 40.0), (30.0, 60.0), (90.0, 140.0)]), vec![(0.0, 20.0), (60.0, 90.0)]);
        assert_eq!(open_spans(0.0, 100.0, [(-10.0, 110.0)]), vec![]);
    }

    /// A floor with a wall that stands on its left part and a platform over
    /// its right part. The top of the floor is open only where the wall is
    /// not: a one-way platform covers nothing, and a wall of a blink kind
    /// covers as a solid one does.
    #[test]
    fn an_edge_is_open_where_no_solid_block_is_in_contact_with_it() {
        let floor = surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0);
        let wall = surface(TerrainSurfaceKind::Solid, 0.0, 0.0, 48.0, 100.0);
        let blink = surface(TerrainSurfaceKind::Cover, 150.0, 60.0, 16.0, 40.0);
        let platform = surface(TerrainSurfaceKind::OneWay, 60.0, 84.0, 60.0, 16.0);
        let far_block = surface(TerrainSurfaceKind::Solid, 100.0, 20.0, 40.0, 30.0);
        let all = [floor, wall, blink, platform, far_block];
        assert_eq!(open_edge_spans(&floor, Edge::Top, &all), vec![(48.0, 150.0), (166.0, 200.0)]);
        assert_eq!(open_edge_spans(&floor, Edge::Bottom, &all), vec![(0.0, 200.0)]);
        assert_eq!(open_edge_spans(&wall, Edge::Bottom, &all), vec![], "the wall stands on the floor");
        assert_eq!(open_edge_spans(&wall, Edge::Right, &all), vec![(0.0, 100.0)]);
        assert_eq!(
            open_edge_spans(&far_block, Edge::Bottom, &all),
            vec![(100.0, 140.0)],
            "control: a block in the air has its whole underside open"
        );
    }

    fn skinned_world(with_skin: bool) -> (World, Entity, Entity) {
        let mut world = World::new();
        let mut assets = GameAssets::default();
        if with_skin {
            for &part in RoomDressingPart::ALL {
                assets.room_dressing.insert(ParallaxTheme::Cave, part, Handle::default());
            }
        } else {
            assets.room_dressing.mark_attempted(ParallaxTheme::Cave);
        }
        world.insert_resource(assets);
        let floor = world
            .spawn((surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0), Sprite::from_color(Color::WHITE, Vec2::new(200.0, 32.0))))
            .id();
        let wall = world
            .spawn((surface(TerrainSurfaceKind::Solid, 0.0, 0.0, 48.0, 100.0), Sprite::from_color(Color::WHITE, Vec2::new(48.0, 100.0))))
            .id();
        (world, floor, wall)
    }

    fn trims_of(world: &mut World, block: Entity) -> Vec<(TerrainTrim, Vec2, Vec3)> {
        let children: Vec<Entity> = world.entity(block).get::<Children>().map(|c| c.iter().collect()).unwrap_or_default();
        let mut out: Vec<(TerrainTrim, Vec2, Vec3)> = children
            .into_iter()
            .filter_map(|child| {
                let entity = world.entity(child);
                Some((
                    *entity.get::<TerrainTrim>()?,
                    entity.get::<Sprite>().unwrap().custom_size.unwrap(),
                    entity.get::<Transform>().unwrap().translation,
                ))
            })
            .collect();
        out.sort_by(|a, b| (a.0 as u8).cmp(&(b.0 as u8)).then(a.2.x.total_cmp(&b.2.x)));
        out
    }

    #[test]
    fn a_span_is_cut_where_the_pattern_repeats() {
        assert_eq!(anchored_pieces(0.0, 64.0, 64.0), vec![(0.0, 64.0, 0.0)]);
        assert_eq!(
            anchored_pieces(48.0, 200.0, 64.0),
            vec![(48.0, 64.0, 48.0), (64.0, 128.0, 0.0), (128.0, 192.0, 0.0), (192.0, 200.0, 0.0)]
        );
        assert_eq!(anchored_pieces(-20.0, 10.0, 64.0), vec![(-20.0, 0.0, 44.0), (0.0, 10.0, 0.0)]);
        assert!(anchored_pieces(5.0, 5.0, 64.0).is_empty());
    }

    /// The pattern is fixed to the room. Two cells of 16 side by side (the
    /// ground of an intro room) show two parts of the fill that are side by
    /// side in the picture. Before, each block began the picture at its own
    /// corner, and each cell showed the same corner of it.
    #[test]
    fn two_blocks_side_by_side_show_two_parts_of_the_picture_side_by_side() {
        let mut world = World::new();
        let mut assets = GameAssets::default();
        for &part in RoomDressingPart::ALL {
            assets.room_dressing.insert(ParallaxTheme::Cave, part, Handle::default());
        }
        world.insert_resource(assets);
        let cell = |world: &mut World, x: f32| {
            world
                .spawn((surface(TerrainSurfaceKind::Solid, x, 96.0, 16.0, 16.0), Sprite::from_color(Color::WHITE, Vec2::splat(16.0))))
                .id()
        };
        let (left, right) = (cell(&mut world, 80.0), cell(&mut world, 96.0));
        world.run_system_once(skin_terrain_surfaces).unwrap();
        let fill_rect = |world: &mut World, block: Entity| {
            let children: Vec<Entity> = world.entity(block).get::<Children>().unwrap().iter().collect();
            children
                .into_iter()
                .find(|child| world.entity(*child).get::<TerrainTrim>() == Some(&TerrainTrim::Fill))
                .map(|child| world.entity(child).get::<Sprite>().unwrap().rect.unwrap())
                .expect("a cell has one piece of fill")
        };
        let (a, b) = (fill_rect(&mut world, left), fill_rect(&mut world, right));
        // 80 and 96 are 16 and 32 into the pattern of 64; 96 down is 32 into it.
        assert_eq!(a, Rect::new(32.0, 64.0, 64.0, 96.0));
        assert_eq!(b, Rect::new(64.0, 64.0, 96.0, 96.0));
        assert_eq!(a.max.x, b.min.x, "the right cell goes on where the left one ends");
    }

    /// The floor gets its fill and a cap on the part of its top the wall
    /// does not stand on, with the surface line of the cap picture on the
    /// top of the block. The control is the same room with a theme that has
    /// no skin: the block keeps its sprite and gets no piece.
    #[test]
    fn a_block_gets_its_fill_and_a_trim_on_each_open_edge() {
        let (mut world, floor, wall) = skinned_world(true);
        world.run_system_once(skin_terrain_surfaces).unwrap();
        let trims = trims_of(&mut world, floor);
        let area = |kind: TerrainTrim| -> f32 {
            trims.iter().filter(|(trim, ..)| *trim == kind).map(|(_, size, _)| size.x * size.y).sum()
        };
        assert_eq!(area(TerrainTrim::Fill), 200.0 * 32.0, "the pieces of the fill cover the block");
        assert_eq!(area(TerrainTrim::Cap), 152.0 * CAP_HEIGHT, "from the wall (48) to the end (200)");
        assert_eq!(area(TerrainTrim::Under), 200.0 * UNDER_HEIGHT, "the whole underside is open");
        let caps: Vec<_> = trims.iter().filter(|(trim, ..)| *trim == TerrainTrim::Cap).collect();
        // The block is 200 by 32 with its centre at x 100. The first piece of
        // the cap is 48..64: its centre is at x 56, which is -44 in the block.
        assert_eq!(caps[0].2.x, -44.0);
        // The top of the cap picture is CAP_SURFACE over the top of the block.
        assert!(caps.iter().all(|(_, _, at)| at.y + CAP_HEIGHT * 0.5 == 16.0 + CAP_SURFACE));
        assert_eq!(trims_of(&mut world, wall).iter().filter(|(trim, ..)| *trim == TerrainTrim::Under).count(), 0);
        assert_eq!(
            world.entity(floor).get::<Sprite>().unwrap().color,
            Color::NONE,
            "the sprite of the block draws nothing: its pieces do"
        );

        let (mut world, floor, _) = skinned_world(false);
        world.run_system_once(skin_terrain_surfaces).unwrap();
        assert_eq!(world.entity(floor).get::<Sprite>().unwrap().color, Color::WHITE);
        assert!(trims_of(&mut world, floor).is_empty());
        assert!(world.entity(floor).contains::<TerrainSkinned>(), "and it stops asking");
    }

    /// A blink wall takes the field of its kind and a line of light on each
    /// open edge. The wall stands on the floor, so its bottom edge has no
    /// line, and the floor has no cap under it. The control is a theme with
    /// no art: the wall keeps its sprite.
    #[test]
    fn a_blink_wall_takes_its_field_and_a_line_on_each_open_edge() {
        for with_skin in [true, false] {
            let (mut world, floor, _) = skinned_world(with_skin);
            let blink = world
                .spawn((surface(TerrainSurfaceKind::BlinkSoft, 100.0, 60.0, 32.0, 40.0), Sprite::from_color(Color::WHITE, Vec2::new(32.0, 40.0))))
                .id();
            world.run_system_once(skin_terrain_surfaces).unwrap();
            let trims = trims_of(&mut world, blink);
            if !with_skin {
                assert!(trims.is_empty());
                assert_eq!(world.entity(blink).get::<Sprite>().unwrap().color, Color::WHITE);
                continue;
            }
            let area = |kind: TerrainTrim| -> f32 {
                trims.iter().filter(|(trim, ..)| *trim == kind).map(|(_, size, _)| size.x * size.y).sum()
            };
            assert_eq!(area(TerrainTrim::Fill), 32.0 * 40.0);
            // The top (32) and the two sides (40 each) are open.
            assert_eq!(area(TerrainTrim::Rim), (32.0 + 40.0 + 40.0) * BLINK_EDGE_HEIGHT);
            // Each line is inside the wall, against its edge. The wall is 32
            // by 40 with its centre at (0, 0), and a line is cut where the
            // pattern repeats (x 128, y 64): two pieces on each edge.
            let rims: Vec<_> = trims.iter().filter(|(trim, ..)| *trim == TerrainTrim::Rim).map(|(_, _, at)| at.truncate()).collect();
            let inset = BLINK_EDGE_HEIGHT * 0.5;
            assert_eq!(rims.iter().filter(|at| at.y == 20.0 - inset).count(), 2, "the top: {rims:?}");
            assert_eq!(rims.iter().filter(|at| at.x == inset - 16.0).count(), 2, "the left: {rims:?}");
            assert_eq!(rims.iter().filter(|at| at.x == 16.0 - inset).count(), 2, "the right: {rims:?}");
            assert_eq!(rims.len(), 6);
            let caps: f32 = trims_of(&mut world, floor)
                .iter()
                .filter(|(trim, ..)| *trim == TerrainTrim::Cap)
                .map(|(_, size, _)| size.x)
                .sum();
            assert_eq!(caps, 200.0 - 48.0 - 32.0, "no cap under the wall of the room or under the blink wall");
        }
    }

    /// The decor of a span is the same each time, each thing is inside the
    /// span, and none stands on a thing of the play. The control is the same
    /// span with no keep-out: it has a thing where the keep-out is.
    #[test]
    fn the_decor_of_a_span_is_inside_it_and_clear_of_the_play() {
        let free = decor_on_span(0.0, 2000.0, 300.0, 100.0, None);
        assert_eq!(free, decor_on_span(0.0, 2000.0, 300.0, 100.0, None), "the same each time");
        assert!(free.len() >= 10, "{}", free.len());
        assert!(free.iter().all(|(x, variant)| *x > DECOR_SIZE * 0.5 && *x < 2000.0 - DECOR_SIZE * 0.5 && *variant < DECOR_VARIANTS));
        assert!(free.iter().map(|(_, variant)| *variant).collect::<std::collections::HashSet<_>>().len() >= 4, "it uses the strip");
        // A door 40 wide and 80 high that stands on the surface at each place
        // the free span has a thing.
        let doors = TerrainKeepOut(free.iter().map(|(x, _)| (Vec2::new(x - 20.0, 220.0), Vec2::new(x + 20.0, 300.0))).collect());
        assert!(decor_on_span(0.0, 2000.0, 300.0, 100.0, Some(&doors)).is_empty());
        // A thing of the play far over the surface is not in the way.
        let high = TerrainKeepOut(vec![(Vec2::new(0.0, 0.0), Vec2::new(2000.0, 200.0))]);
        assert_eq!(decor_on_span(0.0, 2000.0, 300.0, 100.0, Some(&high)), free);
        assert!(decor_on_span(0.0, 50.0, 300.0, 100.0, None).is_empty(), "a short ledge has none");
        assert!(decor_on_span(0.0, 2000.0, 300.0, 0.0, None).is_empty(), "no spacing, no decor");
    }

    /// A block that comes before its skin is asked for stays open, and is
    /// skinned when the skin is there.
    #[test]
    fn a_block_waits_for_a_skin_that_is_not_asked_for_yet() {
        let (mut world, floor, _) = skinned_world(true);
        let skins = std::mem::take(&mut world.resource_mut::<GameAssets>().room_dressing);
        world.run_system_once(skin_terrain_surfaces).unwrap();
        assert!(!world.entity(floor).contains::<TerrainSkinned>());
        world.resource_mut::<GameAssets>().room_dressing = skins;
        world.run_system_once(skin_terrain_surfaces).unwrap();
        assert!(world.entity(floor).contains::<TerrainSkinned>());
        assert!(!trims_of(&mut world, floor).is_empty());
    }
}
