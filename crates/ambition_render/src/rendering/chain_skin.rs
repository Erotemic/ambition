//! Lay the terrain skin of a room's theme on its surface chains.
//!
//! A surface chain is a line a body rides (a hill, a ramp, a loop), and the
//! earth under it is polygons. With no theme the earth is one flat colour that
//! darkens with depth, and each segment of a chain is a strip of colour. A
//! room that names a theme whose skin has the parts gives the earth the fill
//! of the skin and gives each segment its cap, turned to the segment, with
//! the top of the cap on the side a body rides.
//!
//! The pattern of the fill is fixed to the room, as the pattern of a block's
//! is (`terrain_skin`): the earth is cut where the pattern repeats and each
//! piece shows the part of the picture at its place. The cap goes on along
//! the length of a chain, so its pattern does not start again at each
//! segment.
//!
//! The art is published textures: the ground looks the same with each shader
//! off.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxTheme, RoomDressingPart};

use super::terrain_skin::{
    anchored_pieces, CAP_HEIGHT, CAP_SURFACE, SKIN_PERIOD, SKIN_PX_PER_UNIT,
};

/// The colour of earth with no skin, at its top and far below it.
const PLAIN_TOP: [f32; 4] = [0.05, 0.20, 0.26, 1.0];
const PLAIN_DEEP: [f32; 4] = [0.02, 0.05, 0.09, 1.0];
/// What a skin's fill is multiplied by far below the top of the earth.
const SKIN_DEEP: [f32; 4] = [0.40, 0.43, 0.56, 1.0];
/// How far below the top the shade is full.
const SHADE_DEPTH: f32 = 480.0;
/// How much longer than its length a piece of cap is drawn, so that two
/// pieces that meet at a bend leave no gap.
const CAP_OVERLAP: f32 = 0.75;
/// What the cap of a segment behind the plane of the play is multiplied by.
const FAR_LANE_TINT: Color = Color::srgb(0.62, 0.66, 0.78);

/// One corner of a polygon of earth: where it is in its room (y is down) and
/// how far below the top of the earth it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EarthVertex {
    pub at: Vec2,
    pub depth: f32,
}

/// Earth to draw: convex polygons in the coordinates of a room.
/// [`build_chain_earth_meshes`] gives it its mesh.
#[derive(Component, Clone, Debug)]
pub struct ChainEarth {
    polygons: Vec<Vec<EarthVertex>>,
    world_size: Vec2,
    theme: Option<ParallaxTheme>,
}

/// On a [`ChainEarth`] whose mesh will not change: it has the fill of its
/// theme, or it is known that it gets none.
#[derive(Component, Clone, Copy, Debug)]
pub struct ChainEarthSettled;

impl ChainEarth {
    /// The earth under a filled chain (`SurfaceChain::filled`): one polygon
    /// under each segment, down to the floor of the room.
    pub fn under(world: &ae::World, top: &[ae::Vec2], theme: Option<ParallaxTheme>) -> Self {
        let floor = world.size.y;
        let polygons = top
            .windows(2)
            .map(|pair| {
                let (a, b) = (
                    Vec2::new(pair[0].x, pair[0].y),
                    Vec2::new(pair[1].x, pair[1].y),
                );
                vec![
                    EarthVertex { at: a, depth: 0.0 },
                    EarthVertex { at: b, depth: 0.0 },
                    EarthVertex {
                        at: Vec2::new(b.x, floor),
                        depth: (floor - b.y).max(0.0),
                    },
                    EarthVertex {
                        at: Vec2::new(a.x, floor),
                        depth: (floor - a.y).max(0.0),
                    },
                ]
            })
            .collect();
        Self {
            polygons,
            world_size: Vec2::new(world.size.x, world.size.y),
            theme,
        }
    }

    /// Painted earth (`SurfaceChain::earth`): its own polygons. The depth of
    /// a corner is how far it is below the top of the earth in its column.
    pub fn painted(
        world: &ae::World,
        earth: &[Vec<ae::Vec2>],
        theme: Option<ParallaxTheme>,
    ) -> Self {
        /// Column width for "the top of the earth here": one LDtk cell.
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
            // A corner on a column boundary is in both columns: the
            // shallower answer keeps the face of a cliff lit from its top.
            let top = [column(p.x), column(p.x - 0.01)]
                .into_iter()
                .filter_map(|col| tops.get(&col).copied())
                .fold(f32::INFINITY, f32::min);
            if top.is_finite() {
                (p.y - top).max(0.0)
            } else {
                0.0
            }
        };
        let polygons = earth
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .map(|p| EarthVertex {
                        at: Vec2::new(p.x, p.y),
                        depth: depth(*p),
                    })
                    .collect()
            })
            .collect();
        Self {
            polygons,
            world_size: Vec2::new(world.size.x, world.size.y),
            theme,
        }
    }
}

/// The part of a convex `polygon` that is in the square cell at `min` with
/// side `side`.
fn clip_to_cell(polygon: &[EarthVertex], min: Vec2, side: f32) -> Vec<EarthVertex> {
    let max = min + Vec2::splat(side);
    let mut out = polygon.to_vec();
    for (axis, bound, keeps_greater) in [
        (0, min.x, true),
        (0, max.x, false),
        (1, min.y, true),
        (1, max.y, false),
    ] {
        let along = |v: &EarthVertex| if axis == 0 { v.at.x } else { v.at.y };
        let inside = |v: &EarthVertex| {
            if keeps_greater {
                along(v) >= bound
            } else {
                along(v) <= bound
            }
        };
        let input = std::mem::take(&mut out);
        for (i, a) in input.iter().enumerate() {
            let b = &input[(i + 1) % input.len()];
            if inside(a) {
                out.push(*a);
            }
            if inside(a) != inside(b) {
                let t = (bound - along(a)) / (along(b) - along(a));
                out.push(EarthVertex {
                    at: a.at.lerp(b.at, t),
                    depth: a.depth + (b.depth - a.depth) * t,
                });
            }
        }
        if out.len() < 3 {
            return Vec::new();
        }
    }
    out
}

/// The mesh of `earth`. With `skinned`, each polygon is cut on the grid of
/// the skin's pattern and each piece has the texture coordinates of its place
/// in its cell; its colour is what multiplies the picture.
fn earth_mesh(earth: &ChainEarth, skinned: bool) -> Mesh {
    let (top, deep) = if skinned {
        ([1.0; 4], SKIN_DEEP)
    } else {
        (PLAIN_TOP, PLAIN_DEEP)
    };
    let shade = |depth: f32| -> [f32; 4] {
        let t = (depth / SHADE_DEPTH).clamp(0.0, 1.0);
        std::array::from_fn(|i| top[i] + (deep[i] - top[i]) * t)
    };
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut push = |polygon: &[EarthVertex], cell: Vec2| {
        if polygon.len() < 3 {
            return;
        }
        let base = positions.len() as u32;
        for vertex in polygon {
            // Room to Bevy: the middle of the room is the origin, and y is up.
            positions.push([
                vertex.at.x - earth.world_size.x * 0.5,
                earth.world_size.y * 0.5 - vertex.at.y,
                0.0,
            ]);
            colors.push(shade(vertex.depth));
            uvs.push([
                (vertex.at.x - cell.x) / SKIN_PERIOD,
                (vertex.at.y - cell.y) / SKIN_PERIOD,
            ]);
        }
        // Convex: a fan from the first corner.
        for k in 1..polygon.len() as u32 - 1 {
            indices.extend([base, base + k, base + k + 1]);
        }
    };
    for polygon in &earth.polygons {
        if !skinned {
            push(polygon, Vec2::ZERO);
            continue;
        }
        let (min, max) = polygon.iter().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(lo, hi), v| (lo.min(v.at), hi.max(v.at)),
        );
        let cell_of = |v: f32| (v / SKIN_PERIOD).floor() as i64;
        for cx in cell_of(min.x)..=cell_of(max.x - 1e-3).max(cell_of(min.x)) {
            for cy in cell_of(min.y)..=cell_of(max.y - 1e-3).max(cell_of(min.y)) {
                let cell = Vec2::new(cx as f32, cy as f32) * SKIN_PERIOD;
                push(&clip_to_cell(polygon, cell, SKIN_PERIOD), cell);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Mesh each [`ChainEarth`] that is not settled.
///
/// Earth has a mesh from its first frame: plain, until the skin of its theme
/// is asked for (the room's loader does that with the parallax of the theme).
/// Then it takes the fill of the skin, or keeps the plain mesh if the skin
/// has no fill, and it is settled.
pub fn build_chain_earth_meshes(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<ColorMaterial>>>,
    assets: Option<Res<GameAssets>>,
    open: Query<(Entity, &ChainEarth, Has<Mesh2d>), Without<ChainEarthSettled>>,
) {
    if open.is_empty() {
        return;
    }
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    for (entity, earth, meshed) in &open {
        // What the skin says for this earth: `None` while it is not known.
        let fill = match (earth.theme, assets.as_deref()) {
            (Some(theme), Some(assets)) if assets.room_dressing.attempted(theme) => Some(
                assets
                    .room_dressing
                    .get(theme, RoomDressingPart::Fill)
                    .cloned(),
            ),
            (Some(_), Some(_)) => None,
            _ => Some(None),
        };
        let mut visual = commands.entity(entity);
        match fill {
            Some(Some(image)) => {
                visual.try_insert((
                    Mesh2d(meshes.add(earth_mesh(earth, true))),
                    MeshMaterial2d(materials.add(ColorMaterial {
                        texture: Some(image),
                        ..ColorMaterial::from_color(Color::WHITE)
                    })),
                    ChainEarthSettled,
                ));
            }
            known => {
                if !meshed {
                    visual.try_insert((
                        Mesh2d(meshes.add(earth_mesh(earth, false))),
                        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
                    ));
                }
                if known.is_some() {
                    visual.try_insert(ChainEarthSettled);
                }
            }
        }
    }
}

/// On the strip of a chain segment whose room names a theme: the cap of the
/// theme takes the place of the strip ([`dress_chain_turf`]).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ChainTurf {
    pub theme: ParallaxTheme,
    /// How far along its chain the segment starts.
    pub along: f32,
    pub length: f32,
    /// The depth lane of the segment (`SurfaceChain::segment_depth`).
    pub lane: i8,
}

/// On a [`ChainTurf`] whose question is answered.
#[derive(Component, Clone, Copy, Debug)]
pub struct ChainTurfDressed;

/// The pieces of cap on a segment: for each, how far into the pattern it
/// starts, its length, and where its middle is from the middle of the
/// segment, along it.
fn turf_pieces(turf: &ChainTurf) -> Vec<(f32, f32, f32)> {
    anchored_pieces(turf.along, turf.along + turf.length, SKIN_PERIOD)
        .into_iter()
        .map(|(lo, hi, offset)| {
            (
                offset,
                hi - lo,
                (lo + hi) * 0.5 - (turf.along + turf.length * 0.5),
            )
        })
        .collect()
}

/// Give each chain segment of a room that names a theme the cap of the theme.
///
/// The strip of the segment is turned along it with its own up on the side a
/// body rides, so a piece of cap that is its child has its top on that side.
/// A theme with no cap leaves the strip as it is.
pub fn dress_chain_turf(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut open: Query<(Entity, &ChainTurf, &mut Sprite), Without<ChainTurfDressed>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, turf, mut sprite) in &mut open {
        if !assets.room_dressing.attempted(turf.theme) {
            continue;
        }
        commands.entity(entity).try_insert(ChainTurfDressed);
        let Some(cap) = assets
            .room_dressing
            .get(turf.theme, RoomDressingPart::Cap)
            .cloned()
        else {
            continue;
        };
        let color = if turf.lane < 0 {
            FAR_LANE_TINT
        } else {
            Color::WHITE
        };
        let pieces: Vec<(Sprite, Vec3)> = turf_pieces(turf)
            .into_iter()
            .map(|(offset, length, along)| {
                let piece = Sprite {
                    image: cap.clone(),
                    color,
                    rect: Some(Rect::new(
                        offset * SKIN_PX_PER_UNIT,
                        0.0,
                        (offset + length) * SKIN_PX_PER_UNIT,
                        CAP_HEIGHT * SKIN_PX_PER_UNIT,
                    )),
                    custom_size: Some(Vec2::new(length + CAP_OVERLAP, CAP_HEIGHT)),
                    ..Default::default()
                };
                // The surface is `CAP_SURFACE` below the top of the picture.
                (
                    piece,
                    Vec3::new(along, CAP_SURFACE - CAP_HEIGHT * 0.5, 0.01),
                )
            })
            .collect();
        *sprite = Sprite::from_color(Color::NONE, sprite.custom_size.unwrap_or(Vec2::ZERO));
        // A world closure: a strip is room-scoped, and a room transition can
        // despawn it before the command flush.
        commands.queue(move |world: &mut World| {
            let Ok(mut strip) = world.get_entity_mut(entity) else {
                return;
            };
            strip.with_children(|parent| {
                for (piece, at) in pieces {
                    parent.spawn((
                        piece,
                        Transform::from_translation(at),
                        Name::new("Chain turf piece"),
                    ));
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(polygon: &[EarthVertex]) -> f32 {
        let n = polygon.len();
        (0..n)
            .map(|i| polygon[i].at.perp_dot(polygon[(i + 1) % n].at))
            .sum::<f32>()
            .abs()
            * 0.5
    }

    /// Earth that is cut on the grid of the pattern is the same earth: the
    /// pieces have its area, each is in one cell, and a depth is carried to
    /// a cut. The control is the polygon itself, which is in four cells.
    #[test]
    fn earth_cut_on_the_pattern_grid_is_the_same_earth() {
        let corner = |x: f32, y: f32| EarthVertex {
            at: Vec2::new(x, y),
            depth: y,
        };
        // A slope: its top goes from y = 10 to y = 40.
        let polygon = [
            corner(20.0, 10.0),
            corner(100.0, 40.0),
            corner(100.0, 120.0),
            corner(20.0, 120.0),
        ];
        let mut total = 0.0;
        let mut cells = 0;
        for cx in 0..2 {
            for cy in 0..2 {
                let cell = Vec2::new(cx as f32, cy as f32) * SKIN_PERIOD;
                let piece = clip_to_cell(&polygon, cell, SKIN_PERIOD);
                if piece.is_empty() {
                    continue;
                }
                cells += 1;
                total += area(&piece);
                for vertex in &piece {
                    let local = (vertex.at - cell) / SKIN_PERIOD;
                    assert!(
                        (-1e-4..=1.0 + 1e-4).contains(&local.x)
                            && (-1e-4..=1.0 + 1e-4).contains(&local.y),
                        "{vertex:?} is out of its cell"
                    );
                    assert!(
                        (vertex.depth - vertex.at.y).abs() < 1e-3,
                        "a cut must carry the depth: {vertex:?}"
                    );
                }
            }
        }
        assert_eq!(cells, 4, "premise: the polygon is in four cells");
        assert!(
            (total - area(&polygon)).abs() < 0.5,
            "the pieces have {total}, the polygon {}",
            area(&polygon)
        );
    }

    /// The cap goes on along the chain: a segment that crosses the place
    /// where the pattern repeats is two pieces, and the second starts the
    /// pattern again.
    #[test]
    fn a_segment_across_the_repeat_of_the_cap_is_two_pieces() {
        let turf = ChainTurf {
            theme: ParallaxTheme::Skybridge,
            along: 50.0,
            length: 30.0,
            lane: 0,
        };
        let pieces = turf_pieces(&turf);
        assert_eq!(pieces.len(), 2);
        let (offset, length, along) = pieces[0];
        assert!(
            (offset - 50.0).abs() < 1e-3
                && (length - 14.0).abs() < 1e-3
                && (along + 8.0).abs() < 1e-3,
            "{pieces:?}"
        );
        let (offset, length, along) = pieces[1];
        assert!(
            offset.abs() < 1e-3 && (length - 16.0).abs() < 1e-3 && (along - 7.0).abs() < 1e-3,
            "{pieces:?}"
        );
    }
}
