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
//! cut in pieces where the pattern repeats ([`anchored_pieces`]), and each
//! piece shows the part of the picture that is at its place in the room. So
//! the pattern goes on with no break from a block to the block next to it. A
//! room whose ground is many small blocks (each intro room is cells of 16)
//! would show the same corner of the picture on each one.
//!
//! A piece is a [`SkinQuad`], not an entity: the skin of a room is
//! presentation, and a room can have thousands of pieces. [`lay_room_skin`]
//! lays the quads of a room's blocks, and [`lay_room_terrain`] draws them as
//! a few meshes for each room ([`RoomTerrain`]): one for each picture, layer
//! and square of [`SKIN_CHUNK`]. The block stays the authority on what is
//! there. When a block goes (a broken brick) or gets art of its own, its room
//! lays its skin again, so the blocks next to it get the trims of the edges
//! that are open now, and only the meshes whose quads changed are built
//! again.
//!
//! The art is published textures on plain textured meshes: the terrain of a
//! room looks the same with each shader off.
//!
//! The sizes below are in world units and are the sizes the art is drawn to
//! (`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/terrain/skins.py`).
//! Change the two together.
//!
//! A block with art of its own (`EntityArt`) and a lock wall keep their art.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};
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
const MOST_FILL_PIECES: usize = 4096;
/// An open edge shorter than this gets no trim.
const SHORTEST_TRIM: f32 = 4.0;
/// Two blocks this near are in contact.
const CONTACT: f32 = 0.5;
/// How far into a blink wall the line of light on its edge goes, in world
/// units (`terrain/fixtures.py`, `BLINK_EDGE`).
/// How many times the side of a thing of the decor the side of its pool of
/// light is (`GLOW_SCALE` in `terrain/decor.py`).
const DECOR_GLOW_SCALE: f32 = 3.0;
const BLINK_EDGE_HEIGHT: f32 = 6.0;
/// How far into a hazard the spikes on its edge go, in world units
/// (`terrain/fixtures.py`, `HAZARD_EDGE`).
const HAZARD_EDGE_HEIGHT: f32 = 12.0;

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
    /// A surface that sends a body back to its start. It takes the danger
    /// fill and a row of spikes on each open edge, which point out of it. It
    /// covers no edge of a block next to it.
    Hazard,
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
        !matches!(self.kind, TerrainSurfaceKind::OneWay | TerrainSurfaceKind::Hazard)
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

/// A piece of the art of a fixture (a ladder, a body of water): a child of
/// the fixture.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixtureArtPiece;

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

/// The sprites of a picture that fills `min..min + size`, with a pattern of
/// `period` fixed to the room: `(sprite, place from the middle of the
/// rectangle)` for each. A period of 0 on an axis is a picture that does not
/// repeat on it: one piece, from its start. `None` when there would be more
/// than [`MOST_FILL_PIECES`].
fn pieces_of(image: &Handle<Image>, min: Vec2, size: Vec2, period: Vec2, z: f32) -> Option<Vec<(Sprite, Vec3)>> {
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
                piece(image.clone(), Vec2::new(ox, oy), Vec2::new(x1 - x0, y1 - y0), false),
                Vec3::new((x0 + x1) * 0.5 - centre.x, centre.y - (y0 + y1) * 0.5, z),
            ));
        }
    }
    Some(out)
}

/// The layer of a part of a skin, in front of the block it is on: the fill
/// at the back, the decor at the front.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkinLayer {
    /// The fill of a block, the field of a blink wall or a hazard, a one-way
    /// platform.
    Fill,
    /// The shade on a left or right edge.
    Side,
    /// The underside of a bottom edge, and the line of a blink wall or the
    /// spikes of a hazard on an open edge.
    Under,
    /// The cap of a top edge.
    Cap,
    /// The pool of light of a thing of the decor.
    DecorGlow,
    /// A thing of the decor.
    Decor,
}

impl SkinLayer {
    /// How far in front of its block the layer is drawn.
    pub fn z(self) -> f32 {
        match self {
            SkinLayer::Fill => 0.005,
            SkinLayer::Side => 0.01,
            SkinLayer::Under => 0.02,
            SkinLayer::Cap => 0.03,
            SkinLayer::DecorGlow => 0.035,
            SkinLayer::Decor => 0.04,
        }
    }
}

/// A turn of a quad of a skin about its centre, counterclockwise as the room
/// is drawn (y up).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuarterTurn {
    None,
    Left,
    Half,
    Right,
}

impl QuarterTurn {
    /// `v`, turned (y up).
    fn apply(self, v: Vec2) -> Vec2 {
        match self {
            QuarterTurn::None => v,
            QuarterTurn::Left => Vec2::new(-v.y, v.x),
            QuarterTurn::Half => -v,
            QuarterTurn::Right => Vec2::new(v.y, -v.x),
        }
    }
}

/// One rectangle of a picture of a skin: the unit the skin of a room is laid
/// in. It is presentation only: the block it is on (`owner`) is the
/// authority on what is there.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinQuad {
    /// The block visual it is part of.
    pub owner: Entity,
    pub part: RoomDressingPart,
    pub layer: SkinLayer,
    /// Its centre, in the coordinates of its room (y is down).
    pub centre: Vec2,
    /// Its size along the axes of its picture, before the turn, in world
    /// units.
    pub size: Vec2,
    /// The part of the picture it shows, in texture pixels.
    pub texture: Rect,
    pub turn: QuarterTurn,
    pub flip_x: bool,
}

impl SkinQuad {
    /// A quad that shows the part of its picture that starts at `texture_min`
    /// and is `size`, both in world units, at the scale of the skins.
    fn at_scale(owner: Entity, part: RoomDressingPart, layer: SkinLayer, centre: Vec2, size: Vec2, texture_min: Vec2) -> Self {
        Self {
            owner,
            part,
            layer,
            centre,
            size,
            texture: Rect::from_corners(texture_min * SKIN_PX_PER_UNIT, (texture_min + size) * SKIN_PX_PER_UNIT),
            turn: QuarterTurn::None,
            flip_x: false,
        }
    }

    /// The area it covers, in world units.
    pub fn area(&self) -> f32 {
        self.size.x * self.size.y
    }
}

/// What the skin of one block is.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockSkin {
    /// The theme has no picture for it: it keeps its own sprite.
    Untouched,
    /// Its quads draw it, and its own sprite draws nothing.
    Laid,
    /// Its fill is too large for quads: its own sprite repeats this picture.
    /// The quads of its edges are laid.
    OwnFill(RoomDressingPart),
}

/// Which pictures the theme of a room has.
pub trait SkinParts {
    fn has(&self, part: RoomDressingPart) -> bool;
}

impl<F: Fn(RoomDressingPart) -> bool> SkinParts for F {
    fn has(&self, part: RoomDressingPart) -> bool {
        self(part)
    }
}

/// The quads of a picture that fills `min..min + size` with a pattern of
/// `period` fixed to the room. `false` when they would be more than
/// [`MOST_FILL_PIECES`]. A period of 0 on an axis is a picture that does not
/// repeat on it.
fn fill_quads(owner: Entity, part: RoomDressingPart, min: Vec2, size: Vec2, period: Vec2, out: &mut Vec<SkinQuad>) -> bool {
    let max = min + size;
    let cut = |lo: f32, hi: f32, period: f32| {
        if period > 0.0 { anchored_pieces(lo, hi, period) } else { vec![(lo, hi, 0.0)] }
    };
    let across = cut(min.x, max.x, period.x);
    let down = cut(min.y, max.y, period.y);
    if across.len() * down.len() > MOST_FILL_PIECES {
        return false;
    }
    for &(x0, x1, ox) in &across {
        for &(y0, y1, oy) in &down {
            out.push(SkinQuad::at_scale(
                owner,
                part,
                SkinLayer::Fill,
                Vec2::new(x0 + x1, y0 + y1) * 0.5,
                Vec2::new(x1 - x0, y1 - y0),
                Vec2::new(ox, oy),
            ));
        }
    }
    true
}

/// The line on each open edge of `surface` (the light of a blink wall, the
/// spikes of a hazard): the quads, each turned so that the top row of the
/// picture is on the edge. A line goes `height` into the block, and half of
/// the block at most when the edge across from it is open too.
fn edge_line_quads(owner: Entity, part: RoomDressingPart, surface: &TerrainSurface, others: &[TerrainSurface], height: f32, out: &mut Vec<SkinQuad>) {
    let open = |edge: Edge| open_edge_spans(surface, edge, others);
    let across = |edge: Edge| match edge {
        Edge::Top => Edge::Bottom,
        Edge::Bottom => Edge::Top,
        Edge::Left => Edge::Right,
        Edge::Right => Edge::Left,
    };
    let (min, max) = (surface.min, surface.max());
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        let depth_of = |edge: Edge| {
            let through = match edge {
                Edge::Top | Edge::Bottom => surface.size.y,
                Edge::Left | Edge::Right => surface.size.x,
            };
            let room_for_it = if open(across(edge)).is_empty() { through } else { through * 0.5 };
            height.min(room_for_it)
        };
        let depth = depth_of(edge);
        // A line on a side stops where the line of the top or of the bottom
        // is: the two would be one on the other in the corner.
        let (from, to) = match edge {
            Edge::Top | Edge::Bottom => (f32::NEG_INFINITY, f32::INFINITY),
            Edge::Left | Edge::Right => (
                if open(Edge::Top).is_empty() { f32::NEG_INFINITY } else { min.y + depth_of(Edge::Top) },
                if open(Edge::Bottom).is_empty() { f32::INFINITY } else { max.y - depth_of(Edge::Bottom) },
            ),
        };
        for (a, b) in open(edge) {
            let (a, b) = (a.max(from), b.min(to));
            if b - a < 0.5 {
                continue;
            }
            for (lo, hi, offset) in anchored_pieces(a, b, SKIN_PERIOD) {
                let along = (lo + hi) * 0.5;
                let (centre, turn) = match edge {
                    Edge::Top => (Vec2::new(along, min.y + depth * 0.5), QuarterTurn::None),
                    Edge::Bottom => (Vec2::new(along, max.y - depth * 0.5), QuarterTurn::Half),
                    Edge::Left => (Vec2::new(min.x + depth * 0.5, along), QuarterTurn::Left),
                    Edge::Right => (Vec2::new(max.x - depth * 0.5, along), QuarterTurn::Right),
                };
                let mut quad = SkinQuad::at_scale(owner, part, SkinLayer::Under, centre, Vec2::new(hi - lo, depth), Vec2::new(offset, 0.0));
                quad.turn = turn;
                out.push(quad);
            }
        }
    }
}

/// Lay the skin on one block: push its quads to `out` and say what its own
/// sprite is. `others` are the blocks near it in its room (it may be among
/// them); `keep_out` is where its decor must not stand.
pub fn lay_block_skin(
    owner: Entity,
    surface: &TerrainSurface,
    others: &[TerrainSurface],
    keep_out: Option<&TerrainKeepOut>,
    decor_spacing: f32,
    parts: &impl SkinParts,
    out: &mut Vec<SkinQuad>,
) -> BlockSkin {
    let (min, max) = (surface.min, surface.max());
    let fill = |part: RoomDressingPart, period: Vec2, out: &mut Vec<SkinQuad>| {
        if fill_quads(owner, part, min, surface.size, period, out) { BlockSkin::Laid } else { BlockSkin::OwnFill(part) }
    };
    match surface.kind {
        TerrainSurfaceKind::Cover => BlockSkin::Untouched,
        TerrainSurfaceKind::BlinkSoft | TerrainSurfaceKind::BlinkHard | TerrainSurfaceKind::Hazard => {
            let (field, line, height) = match surface.kind {
                TerrainSurfaceKind::BlinkSoft => (RoomDressingPart::BlinkSoft, RoomDressingPart::BlinkEdge, BLINK_EDGE_HEIGHT),
                TerrainSurfaceKind::BlinkHard => (RoomDressingPart::BlinkHard, RoomDressingPart::BlinkEdge, BLINK_EDGE_HEIGHT),
                _ => (RoomDressingPart::HazardFill, RoomDressingPart::HazardEdge, HAZARD_EDGE_HEIGHT),
            };
            if !parts.has(field) {
                return BlockSkin::Untouched;
            }
            let skin = fill(field, Vec2::splat(SKIN_PERIOD), out);
            if parts.has(line) {
                edge_line_quads(owner, line, surface, others, height, out);
            }
            skin
        }
        TerrainSurfaceKind::OneWay => {
            if !parts.has(RoomDressingPart::OneWay) {
                return BlockSkin::Untouched;
            }
            fill(RoomDressingPart::OneWay, Vec2::new(SKIN_PERIOD, 16.0), out)
        }
        TerrainSurfaceKind::Solid => {
            if !parts.has(RoomDressingPart::Fill) {
                return BlockSkin::Untouched;
            }
            let skin = fill(RoomDressingPart::Fill, Vec2::splat(SKIN_PERIOD), out);
            if parts.has(RoomDressingPart::Side) {
                let width = SIDE_WIDTH.min(surface.size.x);
                for edge in [Edge::Left, Edge::Right] {
                    let right = edge == Edge::Right;
                    let x = if right { max.x - width * 0.5 } else { min.x + width * 0.5 };
                    for (a, b) in open_edge_spans(surface, edge, others) {
                        for (y0, y1, oy) in anchored_pieces(a, b, SKIN_PERIOD) {
                            let mut quad = SkinQuad::at_scale(
                                owner,
                                RoomDressingPart::Side,
                                SkinLayer::Side,
                                Vec2::new(x, (y0 + y1) * 0.5),
                                Vec2::new(width, y1 - y0),
                                Vec2::new(0.0, oy),
                            );
                            quad.flip_x = right;
                            out.push(quad);
                        }
                    }
                }
            }
            if parts.has(RoomDressingPart::Under) {
                for (a, b) in open_edge_spans(surface, Edge::Bottom, others) {
                    for (x0, x1, ox) in anchored_pieces(a, b, SKIN_PERIOD) {
                        out.push(SkinQuad::at_scale(
                            owner,
                            RoomDressingPart::Under,
                            SkinLayer::Under,
                            Vec2::new((x0 + x1) * 0.5, max.y - UNDER_EDGE + UNDER_HEIGHT * 0.5),
                            Vec2::new(x1 - x0, UNDER_HEIGHT),
                            Vec2::new(ox, 0.0),
                        ));
                    }
                }
            }
            let top_spans = open_edge_spans(surface, Edge::Top, others);
            if parts.has(RoomDressingPart::Cap) {
                for &(a, b) in &top_spans {
                    for (x0, x1, ox) in anchored_pieces(a, b, SKIN_PERIOD) {
                        out.push(SkinQuad::at_scale(
                            owner,
                            RoomDressingPart::Cap,
                            SkinLayer::Cap,
                            // The top of the cap picture is `CAP_SURFACE` over
                            // the top of the block.
                            Vec2::new((x0 + x1) * 0.5, min.y - CAP_SURFACE + CAP_HEIGHT * 0.5),
                            Vec2::new(x1 - x0, CAP_HEIGHT),
                            Vec2::new(ox, 0.0),
                        ));
                    }
                }
            }
            if parts.has(RoomDressingPart::Decor) {
                let cell = DECOR_CELL_PX as f32;
                let glow = parts.has(RoomDressingPart::DecorGlow);
                for &(a, b) in &top_spans {
                    for (x, variant) in decor_on_span(a, b, min.y, decor_spacing, keep_out) {
                        let texture = Rect::new(variant as f32 * cell, 0.0, (variant + 1) as f32 * cell, cell);
                        // It stands on the surface, one unit into the cap.
                        let centre = Vec2::new(x, min.y - DECOR_SIZE * 0.5 + 1.5);
                        let quad = |part, layer, side| SkinQuad {
                            owner,
                            part,
                            layer,
                            centre,
                            size: Vec2::splat(side),
                            texture,
                            turn: QuarterTurn::None,
                            flip_x: false,
                        };
                        out.push(quad(RoomDressingPart::Decor, SkinLayer::Decor, DECOR_SIZE));
                        // The pool of light of the thing: the same square of
                        // its own picture, larger, behind it. The square of a
                        // thing with no light is empty.
                        if glow {
                            out.push(quad(RoomDressingPart::DecorGlow, SkinLayer::DecorGlow, DECOR_SIZE * DECOR_GLOW_SCALE));
                        }
                    }
                }
            }
            skin
        }
    }
}

/// The blocks of a room, found by place: the blocks near a block are the
/// ones whose edges it can be in contact with.
struct NearBlocks {
    cells: std::collections::HashMap<(i32, i32), Vec<usize>>,
}

impl NearBlocks {
    const CELL: f32 = SKIN_PERIOD;

    fn cells_of(min: Vec2, max: Vec2) -> impl Iterator<Item = (i32, i32)> {
        let lo = ((min - Vec2::splat(CONTACT)) / Self::CELL).floor();
        let hi = ((max + Vec2::splat(CONTACT)) / Self::CELL).floor();
        (lo.x as i32..=hi.x as i32).flat_map(move |x| (lo.y as i32..=hi.y as i32).map(move |y| (x, y)))
    }

    fn new(surfaces: &[TerrainSurface]) -> Self {
        let mut cells: std::collections::HashMap<(i32, i32), Vec<usize>> = std::collections::HashMap::new();
        for (index, surface) in surfaces.iter().enumerate() {
            for cell in Self::cells_of(surface.min, surface.max()) {
                cells.entry(cell).or_default().push(index);
            }
        }
        Self { cells }
    }

    /// The blocks that touch or overlap `surface`'s box grown by the contact
    /// distance, into `out` (cleared first), each one time.
    fn near(&self, surface: &TerrainSurface, surfaces: &[TerrainSurface], indices: &mut Vec<usize>, out: &mut Vec<TerrainSurface>) {
        indices.clear();
        for cell in Self::cells_of(surface.min, surface.max()) {
            if let Some(found) = self.cells.get(&cell) {
                indices.extend_from_slice(found);
            }
        }
        indices.sort_unstable();
        indices.dedup();
        out.clear();
        out.extend(indices.iter().map(|&index| surfaces[index]));
    }
}

/// Lay the skin of a whole room: each block's [`BlockSkin`], in the order
/// of `blocks`, and every quad. A block with art of its own (`with_art`)
/// lays no skin, but it still covers the edges of the blocks next to it.
pub fn lay_room_skin(
    blocks: &[(Entity, TerrainSurface, bool)],
    keep_out: Option<&TerrainKeepOut>,
    decor_spacing: f32,
    parts: &impl SkinParts,
) -> (Vec<BlockSkin>, Vec<SkinQuad>) {
    let surfaces: Vec<TerrainSurface> = blocks.iter().map(|(_, surface, _)| *surface).collect();
    let index = NearBlocks::new(&surfaces);
    let (mut indices, mut near) = (Vec::new(), Vec::new());
    let mut quads = Vec::new();
    let skins = blocks
        .iter()
        .map(|(owner, surface, with_art)| {
            if *with_art {
                return BlockSkin::Untouched;
            }
            index.near(surface, &surfaces, &mut indices, &mut near);
            lay_block_skin(*owner, surface, &near, keep_out, decor_spacing, parts, &mut quads)
        })
        .collect();
    (skins, quads)
}

/// The side of the square of the room one mesh of its skin covers, in world
/// units: a whole number of [`SKIN_PERIOD`]s. A block that changes rebuilds
/// only the meshes its quads are in.
pub const SKIN_CHUNK: f32 = SKIN_PERIOD * 8.0;

/// One mesh of a room's skin: the quads of one picture and one layer whose
/// centres are in one chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SkinChunkKey {
    cell: (i32, i32),
    layer: SkinLayer,
    part: u8,
}

struct SkinChunk {
    entity: Entity,
    mesh: Handle<Mesh>,
    /// A number for its quads: a chunk whose number is the same is not built
    /// again.
    stamp: u64,
}

/// The skin of a room, drawn as a few meshes and not as an entity for each
/// piece of each block. One for each room whose metadata names a theme, on
/// the room's terrain entity (with its [`TerrainKeepOut`]); the meshes are its
/// children, so they go with the room and its view layers.
#[derive(Component)]
pub struct RoomTerrain {
    pub theme: ParallaxTheme,
    /// The size of the room, which places it in the frame of the room's
    /// visuals.
    pub world_size: Vec2,
    /// The blocks or the art of the room changed: lay its skin again.
    dirty: bool,
    owners: std::collections::HashSet<Entity>,
    quads: Vec<SkinQuad>,
    chunks: std::collections::BTreeMap<SkinChunkKey, SkinChunk>,
    materials: std::collections::HashMap<RoomDressingPart, Handle<ColorMaterial>>,
}

impl RoomTerrain {
    pub fn new(theme: ParallaxTheme, world_size: Vec2) -> Self {
        Self {
            theme,
            world_size,
            dirty: true,
            owners: Default::default(),
            quads: Vec::new(),
            chunks: Default::default(),
            materials: Default::default(),
        }
    }

    /// The quads of the block visual `owner` that the skin of the room has
    /// now.
    pub fn quads_of(&self, owner: Entity) -> impl Iterator<Item = &SkinQuad> {
        self.quads.iter().filter(move |quad| quad.owner == owner)
    }

    /// How many meshes draw the skin of the room.
    pub fn mesh_count(&self) -> usize {
        self.chunks.len()
    }
}

/// A mesh of the skin of a room: a child of its [`RoomTerrain`].
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainSkinMesh;

fn chunk_key(quad: &SkinQuad) -> SkinChunkKey {
    let cell = (quad.centre / SKIN_CHUNK).floor();
    SkinChunkKey { cell: (cell.x as i32, cell.y as i32), layer: quad.layer, part: quad.part as u8 }
}

fn chunk_stamp<'a>(quads: impl Iterator<Item = &'a SkinQuad>, image_size: Vec2) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // The UVs are each quad's rectangle over the size of its picture.
    image_size.x.to_bits().hash(&mut hasher);
    image_size.y.to_bits().hash(&mut hasher);
    for quad in quads {
        for value in [quad.centre.x, quad.centre.y, quad.size.x, quad.size.y, quad.texture.min.x, quad.texture.min.y, quad.texture.max.x, quad.texture.max.y] {
            value.to_bits().hash(&mut hasher);
        }
        quad.turn.hash(&mut hasher);
        quad.flip_x.hash(&mut hasher);
    }
    hasher.finish()
}

/// The mesh of `quads` of one picture of `image_size` pixels, in the frame of
/// the visuals of a room of `world_size` (its middle is the origin, y up).
fn skin_mesh<'a>(quads: impl Iterator<Item = &'a SkinQuad>, world_size: Vec2, image_size: Vec2) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for quad in quads {
        let centre = Vec2::new(quad.centre.x - world_size.x * 0.5, world_size.y * 0.5 - quad.centre.y);
        let half = quad.size * 0.5;
        let (mut u0, mut u1) = (quad.texture.min.x / image_size.x, quad.texture.max.x / image_size.x);
        let (v0, v1) = (quad.texture.min.y / image_size.y, quad.texture.max.y / image_size.y);
        if quad.flip_x {
            std::mem::swap(&mut u0, &mut u1);
        }
        let base = positions.len() as u32;
        // The corners of the picture, from its top left, y up.
        for (corner, uv) in [
            (Vec2::new(-half.x, half.y), [u0, v0]),
            (Vec2::new(half.x, half.y), [u1, v0]),
            (Vec2::new(half.x, -half.y), [u1, v1]),
            (Vec2::new(-half.x, -half.y), [u0, v1]),
        ] {
            let at = centre + quad.turn.apply(corner);
            positions.push([at.x, at.y, 0.0]);
            uvs.push(uv);
        }
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Keep the skin of each room in step with its blocks.
///
/// A room lays its skin when its theme's skin has been asked for (the room's
/// loader does that with the parallax of the theme) and the pictures are
/// loaded, and again when a block of it comes, goes (a broken brick), or gets
/// or loses art of its own: the edges of the blocks next to a block that went are open
/// now, and they get their trims. Only the meshes whose quads changed are
/// built again. It runs after `apply_entity_art`: a block with art of its own
/// keeps it.
#[allow(clippy::too_many_arguments)]
pub fn lay_room_terrain(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<ColorMaterial>>>,
    density: Option<Res<TerrainDecorDensity>>,
    mut image_events: MessageReader<AssetEvent<Image>>,
    mut rooms: Query<(Entity, &mut RoomTerrain, Ref<TerrainKeepOut>, Option<&InRoomInstance>)>,
    // A block that comes, or one whose rectangle, kind or theme is written.
    added: Query<Option<&InRoomInstance>, Changed<TerrainSurface>>,
    arted: Query<Option<&InRoomInstance>, (With<TerrainSurface>, Added<EntityArt>)>,
    mut removed: RemovedComponents<TerrainSurface>,
    // A block whose art goes is skinned again: its own sprite is still the
    // art's until the skin writes it.
    mut unarted: RemovedComponents<EntityArt>,
    mut blocks: Query<(Entity, &TerrainSurface, &mut Sprite, Option<&InRoomInstance>, Has<EntityArt>, Has<TerrainSkinned>)>,
) {
    let stamp = |room: Option<&InRoomInstance>| room.map(|room| room.0);
    for room in added.iter().chain(arted.iter()) {
        let room = stamp(room);
        for (_, mut terrain, _, at) in &mut rooms {
            if stamp(at) == room && !terrain.dirty {
                terrain.dirty = true;
            }
        }
    }
    for gone in removed.read().chain(unarted.read()) {
        for (_, mut terrain, ..) in &mut rooms {
            if terrain.owners.contains(&gone) {
                terrain.dirty = true;
            }
        }
    }
    // A picture written in place can have another size, and the UVs are
    // over its size.
    let pictures_changed: Vec<AssetId<Image>> = image_events
        .read()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect();
    let density_changed = density.as_ref().is_some_and(|density| density.is_changed());
    let (Some(assets), Some(mut meshes), Some(mut materials)) = (assets, meshes, materials) else {
        return;
    };
    let skins = &assets.room_dressing;
    let spacing = density.map_or(TerrainDecorDensity::default().spacing, |density| density.spacing);
    for (root, mut terrain, keep_out, at) in &mut rooms {
        let theme = terrain.theme;
        // A part whose picture is another handle now (a reload, a quality
        // change) draws the new one; one written in place is laid again.
        let mut pictures_moved = false;
        for (part, material) in &terrain.materials {
            let wanted = skins.get(theme, *part);
            if wanted.is_some_and(|handle| pictures_changed.contains(&handle.id())) {
                pictures_moved = true;
            }
            if materials.get(material).is_some_and(|drawn| drawn.texture.as_ref() != wanted) {
                if let Some(mut drawn) = materials.get_mut(material) {
                    drawn.texture = wanted.cloned();
                }
                pictures_moved = true;
            }
        }
        if (pictures_moved || density_changed || keep_out.is_changed()) && !terrain.dirty {
            terrain.dirty = true;
        }
        if !terrain.dirty || !skins.attempted(terrain.theme) {
            continue;
        }
        let room = stamp(at);
        let laid: Vec<(Entity, TerrainSurface, bool)> = blocks
            .iter()
            .filter(|(_, surface, _, block_room, ..)| surface.theme == theme && stamp(*block_room) == room)
            .map(|(entity, surface, _, _, with_art, _)| (entity, *surface, with_art))
            .collect();
        let parts = |part| skins.get(theme, part).is_some();
        let (block_skins, quads) = lay_room_skin(&laid, Some(&*keep_out), spacing, &parts);
        // Each picture the quads show must be loaded: its size places the
        // quads in it. A room whose pictures are not here yet waits.
        let mut sizes: std::collections::HashMap<RoomDressingPart, Vec2> = std::collections::HashMap::new();
        let mut waiting = false;
        for quad in &quads {
            if sizes.contains_key(&quad.part) {
                continue;
            }
            match skins.get(theme, quad.part).and_then(|handle| images.get(handle)) {
                Some(image) => {
                    sizes.insert(quad.part, image.size().as_vec2());
                }
                None => waiting = true,
            }
        }
        if waiting {
            continue;
        }
        // Each block's own sprite: nothing where its quads draw it, the
        // repeated fill where it is too large for quads.
        for ((entity, surface, with_art), skin) in laid.iter().zip(&block_skins) {
            // A block with art of its own keeps its sprite as the art made it.
            if *with_art {
                continue;
            }
            let Ok((_, _, mut sprite, _, _, skinned)) = blocks.get_mut(*entity) else {
                continue;
            };
            if !skinned {
                commands.entity(*entity).try_insert(TerrainSkinned);
            }
            match skin {
                BlockSkin::Untouched => {}
                BlockSkin::Laid => {
                    // The quality refresh rebinds a `BoundEntitySprite` to the
                    // tile of its kind, which would put it back.
                    commands.entity(*entity).try_remove::<BoundEntitySprite>();
                    if sprite.color != Color::NONE || sprite.image != Handle::default() {
                        *sprite = Sprite::from_color(Color::NONE, surface.size);
                    }
                }
                BlockSkin::OwnFill(part) => {
                    commands.entity(*entity).try_remove::<BoundEntitySprite>();
                    // The fill is its picture, its size and its tiles: a
                    // block that grows keeps its picture and needs the rest.
                    if let Some(image) = skins.get(theme, *part) {
                        let fill = repeated(image.clone(), surface.size);
                        if sprite.image != fill.image || sprite.custom_size != fill.custom_size || sprite.image_mode != fill.image_mode {
                            *sprite = fill;
                        }
                    }
                }
            }
        }
        // The meshes: build those whose quads changed, and drop those with
        // none left.
        let mut groups: std::collections::BTreeMap<SkinChunkKey, Vec<usize>> = std::collections::BTreeMap::new();
        for (index, quad) in quads.iter().enumerate() {
            groups.entry(chunk_key(quad)).or_default().push(index);
        }
        let terrain = &mut *terrain;
        let gone: Vec<SkinChunkKey> = terrain.chunks.keys().filter(|key| !groups.contains_key(key)).copied().collect();
        for key in gone {
            if let Some(chunk) = terrain.chunks.remove(&key) {
                commands.entity(chunk.entity).try_despawn();
            }
        }
        for (key, members) in &groups {
            let part = quads[members[0]].part;
            let chunk_stamp = chunk_stamp(members.iter().map(|&index| &quads[index]), sizes[&part]);
            if terrain.chunks.get(key).is_some_and(|chunk| chunk.stamp == chunk_stamp) {
                continue;
            }
            let mesh = skin_mesh(members.iter().map(|&index| &quads[index]), terrain.world_size, sizes[&part]);
            match terrain.chunks.get_mut(key) {
                Some(chunk) => {
                    // The same asset, built again: nothing else holds it.
                    let _ = meshes.insert(&chunk.mesh, mesh);
                    chunk.stamp = chunk_stamp;
                }
                None => {
                    let material = terrain
                        .materials
                        .entry(part)
                        .or_insert_with(|| {
                            materials.add(ColorMaterial {
                                texture: skins.get(theme, part).cloned(),
                                // A skin picture has clear parts (a shade, a
                                // pool of light). `from_color` of an opaque
                                // colour would draw them opaque.
                                alpha_mode: bevy::sprite_render::AlphaMode2d::Blend,
                                ..ColorMaterial::from_color(Color::WHITE)
                            })
                        })
                        .clone();
                    let handle = meshes.add(mesh);
                    let entity = commands
                        .spawn((
                            Mesh2d(handle.clone()),
                            MeshMaterial2d(material),
                            Transform::from_xyz(0.0, 0.0, key.layer.z()),
                            TerrainSkinMesh,
                            Name::new("Terrain skin mesh"),
                            ChildOf(root),
                        ))
                        .id();
                    terrain.chunks.insert(*key, SkinChunk { entity, mesh: handle, stamp: chunk_stamp });
                }
            }
        }
        terrain.owners = laid.iter().map(|(entity, ..)| *entity).collect();
        terrain.quads = quads;
        terrain.dirty = false;
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

/// The width of the ladder picture, and how much of it at each side is a rail
/// with the end of a rung (`terrain/fixtures.py`: a rail is at 2.2 and at
/// 13.8, and what holds a rung to it is 1.2 each way).
const LADDER_WIDTH: f32 = 16.0;
const LADDER_RAIL: f32 = 5.0;

/// The pieces of a ladder that is wider than its picture: one ladder with
/// long rungs, and not two ladders side by side. Each row of the picture is
/// cut in three: the left rail, the right rail, and the middle of the rung,
/// which is made as long as the ladder needs.
fn wide_ladder_pieces(image: &Handle<Image>, min: Vec2, size: Vec2, z: f32) -> Vec<(Sprite, Vec3)> {
    let centre = min + size * 0.5;
    let middle = size.x - LADDER_RAIL * 2.0;
    // (start in the picture, width in the picture, start in the room, width
    // in the room)
    let across = [
        (0.0, LADDER_RAIL, min.x, LADDER_RAIL),
        (LADDER_RAIL, LADDER_WIDTH - LADDER_RAIL * 2.0, min.x + LADDER_RAIL, middle),
        (LADDER_WIDTH - LADDER_RAIL, LADDER_RAIL, min.x + LADDER_RAIL + middle, LADDER_RAIL),
    ];
    let mut out = Vec::new();
    for (y0, y1, oy) in anchored_pieces(min.y, min.y + size.y, 32.0) {
        for (tx, tw, x0, w) in across {
            let mut sprite = piece(image.clone(), Vec2::new(tx, oy), Vec2::new(tw, y1 - y0), false);
            sprite.custom_size = Some(Vec2::new(w, y1 - y0));
            out.push((sprite, Vec3::new(x0 + w * 0.5 - centre.x, centre.y - (y0 + y1) * 0.5, z)));
        }
    }
    out
}

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
        let pieces = if fixture.kind == ThemedFixtureKind::Ladder && size.x > LADDER_WIDTH {
            Some(wide_ladder_pieces(image, min, size, 0.005))
        } else {
            pieces_of(image, min, size, period, 0.005)
        };
        let Some(pieces) = pieces else {
            continue;
        };
        *sprite = Sprite::from_color(Color::NONE, fixture.size);
        commands.queue(move |world: &mut World| {
            let Ok(mut body) = world.get_entity_mut(entity) else {
                return;
            };
            body.despawn_related::<Children>();
            body.with_children(|parent| {
                for (sprite, at) in pieces {
                    let at = at + Vec3::new(shift.x, -shift.y, 0.0);
                    parent.spawn((sprite, Transform::from_translation(at), FixtureArtPiece, Name::new("Fixture art piece")));
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

        let pieces = fixture_pieces(&mut world, ladder);
        assert_eq!(pieces.iter().map(|(size, _)| size.x * size.y).sum::<f32>(), 16.0 * 96.0, "the pieces cover the ladder");
        assert_eq!(world.entity(ladder).get::<Children>().unwrap().len(), pieces.len(), "the rung of the placeholder is gone");
        assert_eq!(world.entity(ladder).get::<Sprite>().unwrap().color, Color::NONE);

        let strip = fixture_pieces(&mut world, surface);
        assert!(strip.iter().all(|(size, _)| size.y == WATER_SURFACE_HEIGHT));
        // The strip is 4 high with its middle 2 below the top of the water.
        // The top of the picture is `WATER_SURFACE_LINE` over the top of the
        // water, so its middle is 4 - 2.6 = 1.4 below the top: 0.6 over the
        // middle of the strip.
        assert!(strip.iter().all(|(_, at)| (at.y - 0.6).abs() < 1e-4), "{strip:?}");

        // A ladder wider than its picture is one ladder: a rail at each
        // side, and the middle of each rung made as long as the ladder needs.
        let wide = fixture(&mut world, ParallaxTheme::Cave, ThemedFixtureKind::Ladder, Vec2::new(200.0, 0.0), Vec2::new(40.0, 32.0));
        world.run_system_once(dress_themed_fixtures).unwrap();
        let mut widths: Vec<f32> = fixture_pieces(&mut world, wide).iter().map(|(size, _)| size.x).collect();
        widths.sort_by(f32::total_cmp);
        assert_eq!(widths, vec![LADDER_RAIL, LADDER_RAIL, 40.0 - LADDER_RAIL * 2.0]);

        assert_eq!(world.entity(plain).get::<Sprite>().unwrap().color, Color::WHITE, "a theme with no ladder");
        assert_eq!(world.entity(plain).get::<Children>().unwrap().len(), 1, "and its rung stays");
    }

    fn fixture_pieces(world: &mut World, body: Entity) -> Vec<(Vec2, Vec3)> {
        let children: Vec<Entity> = world.entity(body).get::<Children>().map(|c| c.iter().collect()).unwrap_or_default();
        children
            .into_iter()
            .filter(|child| world.entity(*child).contains::<FixtureArtPiece>())
            .map(|child| {
                let entity = world.entity(child);
                (entity.get::<Sprite>().unwrap().custom_size.unwrap(), entity.get::<Transform>().unwrap().translation)
            })
            .collect()
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

    /// Every picture of a skin, or none.
    fn every_part(_: RoomDressingPart) -> bool {
        true
    }

    fn no_part(_: RoomDressingPart) -> bool {
        false
    }

    /// The blocks of a test room, each its own owner, with no art and no decor.
    fn lay(blocks: &[TerrainSurface], parts: &impl SkinParts) -> (Vec<BlockSkin>, Vec<SkinQuad>) {
        let owned: Vec<(Entity, TerrainSurface, bool)> =
            blocks.iter().enumerate().map(|(index, surface)| (Entity::from_raw_u32(index as u32 + 1).unwrap(), *surface, false)).collect();
        lay_room_skin(&owned, None, 0.0, parts)
    }

    fn owner(index: usize) -> Entity {
        Entity::from_raw_u32(index as u32 + 1).unwrap()
    }

    fn area_of(quads: &[SkinQuad], owner: Entity, part: RoomDressingPart) -> f32 {
        quads.iter().filter(|quad| quad.owner == owner && quad.part == part).map(SkinQuad::area).sum()
    }

    /// The pattern is fixed to the room. Two cells of 16 side by side (the
    /// ground of an intro room) show two parts of the fill that are side by
    /// side in the picture. Before, each block began the picture at its own
    /// corner, and each cell showed the same corner of it.
    #[test]
    fn two_blocks_side_by_side_show_two_parts_of_the_picture_side_by_side() {
        let cells = [surface(TerrainSurfaceKind::Solid, 80.0, 96.0, 16.0, 16.0), surface(TerrainSurfaceKind::Solid, 96.0, 96.0, 16.0, 16.0)];
        let (_, quads) = lay(&cells, &every_part);
        let fill = |index: usize| {
            quads.iter().find(|quad| quad.owner == owner(index) && quad.part == RoomDressingPart::Fill).expect("a cell has one piece of fill").texture
        };
        // 80 and 96 are 16 and 32 into the pattern of 64; 96 down is 32 into it.
        assert_eq!(fill(0), Rect::new(32.0, 64.0, 64.0, 96.0));
        assert_eq!(fill(1), Rect::new(64.0, 64.0, 96.0, 96.0));
        assert_eq!(fill(0).max.x, fill(1).min.x, "the right cell goes on where the left one ends");
    }

    /// The floor gets its fill and a cap on the part of its top the wall
    /// does not stand on, with the surface line of the cap picture on the
    /// top of the block. The control is the same room with a theme that has
    /// no skin: no block is laid, and no quad.
    #[test]
    fn a_block_gets_its_fill_and_a_trim_on_each_open_edge() {
        let floor = surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0);
        let wall = surface(TerrainSurfaceKind::Solid, 0.0, 0.0, 48.0, 100.0);
        let (skins, quads) = lay(&[floor, wall], &every_part);
        assert_eq!(skins, vec![BlockSkin::Laid, BlockSkin::Laid]);
        assert_eq!(area_of(&quads, owner(0), RoomDressingPart::Fill), 200.0 * 32.0, "the pieces of the fill cover the block");
        assert_eq!(area_of(&quads, owner(0), RoomDressingPart::Cap), 152.0 * CAP_HEIGHT, "from the wall (48) to the end (200)");
        assert_eq!(area_of(&quads, owner(0), RoomDressingPart::Under), 200.0 * UNDER_HEIGHT, "the whole underside is open");
        let mut caps: Vec<&SkinQuad> = quads.iter().filter(|quad| quad.owner == owner(0) && quad.part == RoomDressingPart::Cap).collect();
        caps.sort_by(|a, b| a.centre.x.total_cmp(&b.centre.x));
        // The first piece of the cap is 48..64.
        assert_eq!(caps[0].centre.x, 56.0);
        // The top of the cap picture is CAP_SURFACE over the top of the block.
        assert!(caps.iter().all(|quad| quad.centre.y - CAP_HEIGHT * 0.5 == 100.0 - CAP_SURFACE));
        assert_eq!(area_of(&quads, owner(1), RoomDressingPart::Under), 0.0, "the wall stands on the floor");

        let (skins, quads) = lay(&[floor, wall], &no_part);
        assert_eq!(skins, vec![BlockSkin::Untouched, BlockSkin::Untouched]);
        assert!(quads.is_empty());
    }

    /// A blink wall takes the field of its kind and a line of light on each
    /// open edge. The wall stands on the floor, so its bottom edge has no
    /// line, and the floor has no cap under it.
    #[test]
    fn a_blink_wall_takes_its_field_and_a_line_on_each_open_edge() {
        let floor = surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0);
        let wall = surface(TerrainSurfaceKind::Solid, 0.0, 0.0, 48.0, 100.0);
        let blink = surface(TerrainSurfaceKind::BlinkSoft, 100.0, 60.0, 32.0, 40.0);
        let (_, quads) = lay(&[floor, wall, blink], &every_part);
        assert_eq!(area_of(&quads, owner(2), RoomDressingPart::BlinkSoft), 32.0 * 40.0);
        // The top (32) and the two sides are open. A side is 40, less the
        // corner the line of the top is in.
        let side = 40.0 - BLINK_EDGE_HEIGHT;
        assert_eq!(area_of(&quads, owner(2), RoomDressingPart::BlinkEdge), (32.0 + side + side) * BLINK_EDGE_HEIGHT);
        // Each line is inside the wall, against its edge, and a line is cut
        // where the pattern repeats (x 128): two pieces on the top, one on a
        // side.
        let rims: Vec<&SkinQuad> = quads.iter().filter(|quad| quad.part == RoomDressingPart::BlinkEdge).collect();
        let inset = BLINK_EDGE_HEIGHT * 0.5;
        let top: Vec<_> = rims.iter().filter(|quad| quad.centre.y == 60.0 + inset).collect();
        assert_eq!(top.len(), 2, "the top: {rims:?}");
        assert!(top.iter().all(|quad| quad.turn == QuarterTurn::None));
        assert_eq!(rims.iter().filter(|quad| quad.centre.x == 100.0 + inset && quad.turn == QuarterTurn::Left).count(), 1, "the left: {rims:?}");
        assert_eq!(rims.iter().filter(|quad| quad.centre.x == 132.0 - inset && quad.turn == QuarterTurn::Right).count(), 1, "the right: {rims:?}");
        assert_eq!(rims.len(), 4);
        let caps: f32 = quads.iter().filter(|quad| quad.owner == owner(0) && quad.part == RoomDressingPart::Cap).map(|quad| quad.size.x).sum();
        assert_eq!(caps, 200.0 - 48.0 - 32.0, "no cap under the wall of the room or under the blink wall");
    }

    /// A hazard takes the danger fill and a row of spikes on each open edge.
    /// A strip that lies on the floor has spikes on its top and its two
    /// ends, and none on the floor side. The spikes of two edges that are
    /// across from each other share the strip: each row is half of it deep.
    /// The floor keeps its cap under the strip: a hazard covers no edge.
    #[test]
    fn a_hazard_takes_spikes_that_point_out_of_each_open_edge() {
        let floor = surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0);
        let wall = surface(TerrainSurfaceKind::Solid, 0.0, 0.0, 48.0, 100.0);
        let strip = surface(TerrainSurfaceKind::Hazard, 64.0, 84.0, 64.0, 16.0);
        let (_, quads) = lay(&[floor, wall, strip], &every_part);
        let rims: Vec<&SkinQuad> = quads.iter().filter(|quad| quad.part == RoomDressingPart::HazardEdge).collect();
        // The top: 64 long and the full height of the spikes. Each end is
        // 16, less the corner the spikes of the top are in: 4 long.
        let top: Vec<_> = rims.iter().filter(|quad| quad.size.x == 64.0 && quad.centre.y == 84.0 + HAZARD_EDGE_HEIGHT * 0.5).collect();
        assert_eq!(top.len(), 1, "{rims:?}");
        assert_eq!(top[0].size.y, HAZARD_EDGE_HEIGHT);
        assert_eq!(rims.iter().filter(|quad| quad.size.x == 16.0 - HAZARD_EDGE_HEIGHT).count(), 2, "the two ends: {rims:?}");
        assert_eq!(rims.len(), 3, "no spikes on the floor side: {rims:?}");
        let caps: f32 = quads.iter().filter(|quad| quad.owner == owner(0) && quad.part == RoomDressingPart::Cap).map(|quad| quad.size.x).sum();
        assert_eq!(caps, 200.0 - 48.0, "the floor keeps its cap under the strip");
    }

    /// A block with art of its own lays no skin, and it still covers the edge
    /// of the block next to it. The control is the same block with no art.
    #[test]
    fn a_block_with_art_lays_no_skin_and_still_covers_its_neighbour() {
        let floor = surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0);
        let brick = surface(TerrainSurfaceKind::Solid, 64.0, 84.0, 16.0, 16.0);
        for with_art in [false, true] {
            let blocks = [(owner(0), floor, false), (owner(1), brick, with_art)];
            let (skins, quads) = lay_room_skin(&blocks, None, 0.0, &every_part);
            assert_eq!(skins[1] == BlockSkin::Untouched, with_art);
            assert_eq!(quads.iter().any(|quad| quad.owner == owner(1)), !with_art);
            assert_eq!(area_of(&quads, owner(0), RoomDressingPart::Cap), (200.0 - 16.0) * CAP_HEIGHT, "with art {with_art}");
        }
    }

    /// A turned quad of the mesh: its picture's top row is on the edge it is
    /// turned to, and a flipped one shows its picture mirrored.
    #[test]
    fn a_turned_quad_puts_the_top_of_its_picture_on_its_edge() {
        let mut quad = SkinQuad::at_scale(owner(0), RoomDressingPart::BlinkEdge, SkinLayer::Under, Vec2::new(10.0, 10.0), Vec2::new(4.0, 2.0), Vec2::ZERO);
        quad.turn = QuarterTurn::Left;
        let mesh = skin_mesh(std::iter::once(&quad), Vec2::ZERO, Vec2::new(128.0, 128.0));
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("positions");
        };
        // Room (10, 10) is (10, -10) y up. Turned left, the top left corner
        // of the picture (-2, 1) goes to (-1, -2): the top row is on the left.
        assert_eq!(positions[0], [9.0, -12.0, 0.0]);
        assert_eq!(positions[1], [9.0, -8.0, 0.0]);
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

    /// A room with every picture of the Cave skin loaded, a floor and a
    /// brick on it.
    fn terrain_world(asked_for: bool) -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        let mut images = Assets::<Image>::default();
        let mut assets = GameAssets::default();
        if asked_for {
            for &part in RoomDressingPart::ALL {
                let image = images.add(Image::new_fill(
                    bevy::render::render_resource::Extent3d { width: 128, height: 128, depth_or_array_layers: 1 },
                    bevy::render::render_resource::TextureDimension::D2,
                    &[255, 255, 255, 255],
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::all(),
                ));
                assets.room_dressing.insert(ParallaxTheme::Cave, part, image);
            }
        }
        world.insert_resource(assets);
        world.insert_resource(images);
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<ColorMaterial>>();
        world.insert_resource(TerrainDecorDensity { spacing: 0.0 });
        world.init_resource::<Messages<AssetEvent<Image>>>();
        let room = world.spawn((RoomTerrain::new(ParallaxTheme::Cave, Vec2::new(512.0, 256.0)), TerrainKeepOut::default(), Transform::default())).id();
        let block = |world: &mut World, s: TerrainSurface| world.spawn((s, Sprite::from_color(Color::WHITE, s.size))).id();
        let floor = block(&mut world, surface(TerrainSurfaceKind::Solid, 0.0, 100.0, 200.0, 32.0));
        let brick = block(&mut world, surface(TerrainSurfaceKind::Solid, 64.0, 84.0, 16.0, 16.0));
        (world, room, floor, brick)
    }

    fn meshes_of(world: &World, room: Entity) -> Vec<Entity> {
        world.entity(room).get::<Children>().map(|children| children.iter().collect()).unwrap_or_default()
    }

    /// A room waits for its skin, then draws it as meshes, and its blocks
    /// draw nothing themselves; no block has a child.
    #[test]
    fn a_room_waits_for_its_skin_then_draws_it_as_meshes() {
        let (mut world, room, floor, _) = terrain_world(false);
        world.run_system_once(lay_room_terrain).unwrap();
        assert!(meshes_of(&world, room).is_empty(), "the skin is not asked for yet");
        assert_eq!(world.entity(floor).get::<Sprite>().unwrap().color, Color::WHITE);

        let (mut world, room, floor, brick) = terrain_world(true);
        world.run_system_once(lay_room_terrain).unwrap();
        let meshes = meshes_of(&world, room);
        assert!(!meshes.is_empty());
        assert!(meshes.iter().all(|mesh| world.entity(*mesh).contains::<Mesh2d>()));
        assert_eq!(world.entity(room).get::<RoomTerrain>().unwrap().mesh_count(), meshes.len());
        for block in [floor, brick] {
            assert_eq!(world.entity(block).get::<Sprite>().unwrap().color, Color::NONE, "its quads draw it");
            assert!(world.entity(block).get::<Children>().is_none(), "a piece of skin is not an entity");
            assert!(world.entity(block).contains::<TerrainSkinned>());
        }
    }

    /// A brick that goes takes its quads with it, and the floor under it
    /// gets the cap of the edge that is open now. The meshes of the far
    /// chunk of the room are not built again.
    #[test]
    fn a_block_that_goes_opens_the_edge_of_its_neighbour() {
        let (mut world, room, floor, brick) = terrain_world(true);
        // A block far away, in another chunk.
        let far = world.spawn((surface(TerrainSurfaceKind::Solid, 1100.0, 100.0, 64.0, 32.0), Sprite::from_color(Color::WHITE, Vec2::new(64.0, 32.0)))).id();
        world.run_system_once(lay_room_terrain).unwrap();
        let cap = |world: &World| -> f32 {
            world.entity(room).get::<RoomTerrain>().unwrap().quads_of(floor).filter(|quad| quad.part == RoomDressingPart::Cap).map(|quad| quad.size.x).sum()
        };
        assert_eq!(cap(&world), 200.0 - 16.0, "the brick covers the floor");
        let far_mesh = |world: &World| -> Vec<AssetId<Mesh>> {
            let terrain = world.entity(room).get::<RoomTerrain>().unwrap();
            let far_cell = ((1100.0 / SKIN_CHUNK).floor() as i32, 0);
            terrain.chunks.iter().filter(|(key, _)| key.cell.0 == far_cell.0).map(|(_, chunk)| chunk.mesh.id()).collect()
        };
        let before = far_mesh(&world);
        assert!(!before.is_empty());
        let far_entities = |world: &World| -> Vec<Entity> {
            let terrain = world.entity(room).get::<RoomTerrain>().unwrap();
            terrain.chunks.iter().filter(|(key, _)| key.cell.0 == (1100.0 / SKIN_CHUNK).floor() as i32).map(|(_, chunk)| chunk.entity).collect()
        };
        let entities_before = far_entities(&world);

        world.despawn(brick);
        world.run_system_once(lay_room_terrain).unwrap();
        assert_eq!(cap(&world), 200.0, "the floor gets the cap where the brick was");
        assert_eq!(world.entity(room).get::<RoomTerrain>().unwrap().quads_of(brick).count(), 0);
        assert_eq!(far_mesh(&world), before, "the far chunk keeps its mesh");
        assert_eq!(far_entities(&world), entities_before, "and its entities");
        assert!(world.entity(room).get::<RoomTerrain>().unwrap().quads_of(far).count() > 0);
    }

    /// A block whose art goes is skinned like its neighbours, though its
    /// surface is not written: its quads draw it, and its own sprite is clear.
    #[test]
    fn a_block_whose_art_goes_takes_the_skin() {
        let (mut world, room, floor, brick) = terrain_world(true);
        world.entity_mut(brick).insert(EntityArt(ambition_sprite_sheet::game_assets::EntitySprite::DoorZone));
        let lay = world.register_system(lay_room_terrain);
        world.run_system(lay).unwrap();
        let terrain = |world: &World| world.entity(room).get::<RoomTerrain>().unwrap().quads_of(brick).count();
        assert_eq!(terrain(&world), 0, "the premise: a block with art lays no skin");
        assert_eq!(world.entity(brick).get::<Sprite>().unwrap().color, Color::WHITE, "the premise: it keeps its sprite");
        assert_eq!(world.entity(floor).get::<Sprite>().unwrap().color, Color::NONE);

        world.entity_mut(brick).remove::<EntityArt>();
        world.run_system(lay).unwrap();
        assert!(terrain(&world) > 0, "the block whose art went has no skin");
        assert_eq!(world.entity(brick).get::<Sprite>().unwrap().color, Color::NONE, "its own sprite still draws");
    }

    /// A block too large for quads fills itself with its picture repeated;
    /// when it grows, the fill grows with it, though the picture is the same.
    #[test]
    fn a_block_that_fills_itself_keeps_its_fill_at_its_size() {
        let (mut world, _, _, _) = terrain_world(true);
        let huge = surface(TerrainSurfaceKind::Solid, 0.0, 400.0, 10_000.0, 2_000.0);
        let block = world.spawn((huge, Sprite::from_color(Color::WHITE, huge.size))).id();
        let lay = world.register_system(lay_room_terrain);
        world.run_system(lay).unwrap();
        let fill = |world: &World| {
            let sprite = world.entity(block).get::<Sprite>().unwrap();
            (sprite.custom_size, matches!(sprite.image_mode, bevy::sprite::SpriteImageMode::Tiled { .. }))
        };
        assert_eq!(fill(&world), (Some(huge.size), true), "the premise: the block fills itself");

        let grown = Vec2::new(12_000.0, 2_000.0);
        world.entity_mut(block).get_mut::<TerrainSurface>().unwrap().size = grown;
        world.run_system(lay).unwrap();
        assert_eq!(fill(&world), (Some(grown), true), "the fill kept the size of the block before it grew");
    }

    /// The skin is laid again when any of what it is made of changes in
    /// place, not only when a block comes or goes: a block's rectangle, the
    /// room's keep-out and the decor density (the quads), the size of a
    /// picture (the UVs), and the picture a part draws (the material). The
    /// control: a frame where nothing changed builds no mesh.
    #[test]
    fn the_skin_follows_each_of_its_inputs_changed_in_place() {
        let (mut world, room, floor, brick) = terrain_world(true);
        // One system, run each time: its change filters read what changed
        // since its last run, as in the app (a system made for one run sees
        // everything as changed).
        let lay = world.register_system(lay_room_terrain);
        world.run_system(lay).unwrap();
        let cap = |world: &World| -> f32 {
            world.entity(room).get::<RoomTerrain>().unwrap().quads_of(floor).filter(|quad| quad.part == RoomDressingPart::Cap).map(|quad| quad.size.x).sum()
        };
        let quad_count = |world: &World| world.entity(room).get::<RoomTerrain>().unwrap().quads.len();
        let stamps = |world: &World| -> Vec<u64> { world.entity(room).get::<RoomTerrain>().unwrap().chunks.values().map(|chunk| chunk.stamp).collect() };

        let before = stamps(&world);
        world.run_system(lay).unwrap();
        assert_eq!(stamps(&world), before, "control: nothing changed, nothing built");

        // The brick moves off the floor, in place.
        assert_eq!(cap(&world), 200.0 - 16.0);
        world.entity_mut(brick).get_mut::<TerrainSurface>().unwrap().min.x = 400.0;
        world.run_system(lay).unwrap();
        assert_eq!(cap(&world), 200.0, "a block's rectangle written in place");

        // Decor comes with a density, and goes with a keep-out over it.
        let bare = quad_count(&world);
        world.resource_mut::<TerrainDecorDensity>().spacing = 40.0;
        world.run_system(lay).unwrap();
        let decorated = quad_count(&world);
        assert!(decorated > bare, "the decor density written in place ({bare} quads, then {decorated})");
        world.entity_mut(room).get_mut::<TerrainKeepOut>().unwrap().0 = vec![(Vec2::new(-1000.0, -1000.0), Vec2::new(2000.0, 2000.0))];
        world.run_system(lay).unwrap();
        assert_eq!(quad_count(&world), bare, "the room's keep-out written in place");

        // The cap's picture is written in place at twice the size.
        let cap_picture = world.resource::<GameAssets>().room_dressing.get(ParallaxTheme::Cave, RoomDressingPart::Cap).unwrap().clone();
        let before = stamps(&world);
        let larger = Image::new_fill(
            bevy::render::render_resource::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            &[255, 255, 255, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let _ = world.resource_mut::<Assets<Image>>().insert(&cap_picture, larger);
        world.write_message(AssetEvent::<Image>::Modified { id: cap_picture.id() });
        world.run_system(lay).unwrap();
        assert_ne!(stamps(&world), before, "a picture's size written in place (its UVs)");

        // The cap draws another picture.
        let other = world.resource_mut::<Assets<Image>>().add(Image::default());
        world.resource_mut::<GameAssets>().room_dressing.insert(ParallaxTheme::Cave, RoomDressingPart::Cap, other.clone());
        world.run_system(lay).unwrap();
        let material = world.entity(room).get::<RoomTerrain>().unwrap().materials[&RoomDressingPart::Cap].clone();
        assert_eq!(
            world.resource::<Assets<ColorMaterial>>().get(&material).unwrap().texture.as_ref(),
            Some(&other),
            "the picture a part draws, moved to another handle"
        );
    }
}
