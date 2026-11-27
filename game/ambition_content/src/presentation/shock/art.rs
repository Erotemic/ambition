//! Place painted bone parts and electrical arcs on a body.

use ambition_characters::actor::Landmark;
use ambition_render::{
    folded_mesh::Tris,
    lines::strands::{jagged, ribbon, Rng},
};
use bevy::prelude::Vec2;

pub(super) const DURATION: f32 = 0.22;
const ARC_SEGMENTS: usize = 8;

pub(super) struct Skeleton {
    head: Vec2,
    hands: [Option<Vec2>; 2],
    feet: [Option<Vec2>; 2],
}

impl Skeleton {
    pub(super) fn from_landmarks(joint: impl Fn(Landmark) -> Option<Vec2>) -> Option<Self> {
        let head = joint(Landmark::Head)?;
        let hands = [joint(Landmark::HandFar), joint(Landmark::HandNear)];
        let feet = [joint(Landmark::FootFar), joint(Landmark::FootNear)];
        (hands.iter().any(Option::is_some) && feet.iter().any(Option::is_some)).then_some(Self {
            head,
            hands,
            feet,
        })
    }

    pub(super) fn standing(center: Vec2, size: Vec2, side: Vec2, down: Vec2) -> Self {
        let at = |x, y| Some(center + side * (x * size.x) + down * (y * size.y));
        Self {
            head: center - down * (0.30 * size.y),
            hands: [at(-0.35, 0.12), at(0.35, 0.12)],
            feet: [at(-0.19, 0.50), at(0.19, 0.50)],
        }
    }

    pub(super) fn parts(self, center: Vec2, size: Vec2, forward: Vec2, down: Vec2) -> Vec<Bone> {
        let h = size.y;
        let neck = self.head + down * (0.075 * h);
        let hip = center + down * (0.10 * h);
        let shoulder = neck.lerp(hip, 0.16);
        let trunk_down = (hip - neck).normalize_or(down);
        let trunk_forward =
            Vec2::new(trunk_down.y, -trunk_down.x) * forward.dot(-down.perp()).signum();
        let mut parts = Vec::with_capacity(14);
        for index in [0, 1] {
            let far = index == 0;
            let side = forward * if far { -1.0 } else { 1.0 };
            let first = parts.len();
            if let Some(foot) = self.feet[index] {
                let socket = hip + side * (0.075 * size.x);
                let knee = socket.lerp(foot, 0.53) + forward * (0.04 * h);
                parts.extend(Bone::limb(socket, knee, foot, 0.46 * h));
                parts.push(Bone::upright(
                    Tile::Foot,
                    foot,
                    Vec2::splat(0.38 * h),
                    forward,
                    down,
                ));
            }
            if let Some(hand) = self.hands[index] {
                let socket = shoulder + trunk_forward * if far { -0.072 * h } else { 0.072 * h };
                let elbow = socket.lerp(hand, 0.56) + down * (0.054 * h);
                parts.extend(Bone::limb(socket, elbow, hand, 0.36 * h));
                parts.push(Bone::aimed(
                    Tile::Hand,
                    hand,
                    Vec2::splat(0.34 * h),
                    hand - elbow,
                ));
            }
            if far {
                for draw in &mut parts[first..] {
                    draw.far = true;
                }
                parts.push(Bone::upright(
                    Tile::Trunk,
                    hip.midpoint(neck),
                    Vec2::new(0.54 * h, (hip - neck).length() * TILE_OVER_JOINTS),
                    forward,
                    trunk_down,
                ));
                parts.push(Bone::upright(
                    Tile::Skull,
                    self.head - down * (0.035 * h),
                    Vec2::splat(0.47 * h),
                    forward,
                    down,
                ));
            }
        }
        parts
    }
}

// The atlas cells are 64 pixels across. A shaft or trunk spans 40 pixels between joints.
const CELL: f32 = 64.0;
const TILE_OVER_JOINTS: f32 = CELL / 40.0;

#[derive(Clone, Copy)]
enum Tile {
    Skull,
    Trunk,
    Shaft,
    Pair,
    Hand,
    Foot,
}

pub(super) struct Bone {
    tile: Tile,
    pub at: Vec2,
    pub size: Vec2,
    pub angle: f32,
    pub flip_x: bool,
    pub far: bool,
}

impl Bone {
    pub fn rect(&self) -> bevy::math::Rect {
        let index = self.tile as u32;
        let min = Vec2::new((index % 4) as f32, (index / 4) as f32) * CELL;
        bevy::math::Rect {
            min,
            max: min + Vec2::splat(CELL),
        }
    }

    fn upright(tile: Tile, at: Vec2, size: Vec2, forward: Vec2, down: Vec2) -> Self {
        Self {
            tile,
            at,
            size,
            angle: down.x.atan2(down.y),
            flip_x: down.perp().dot(forward) > 0.0,
            far: false,
        }
    }

    fn aimed(tile: Tile, at: Vec2, size: Vec2, along: Vec2) -> Self {
        Self {
            tile,
            at,
            size,
            angle: -along.to_angle(),
            flip_x: false,
            far: false,
        }
    }

    fn limb(start: Vec2, mid: Vec2, end: Vec2, width: f32) -> impl Iterator<Item = Self> {
        [(Tile::Shaft, start, mid), (Tile::Pair, mid, end)]
            .into_iter()
            .filter_map(move |(tile, from, to)| {
                let delta = to - from;
                (delta.length_squared() > 0.01).then(|| {
                    Self::aimed(
                        tile,
                        from.midpoint(to),
                        Vec2::new(delta.length() * TILE_OVER_JOINTS, width),
                        delta,
                    )
                })
            })
    }
}

pub(super) fn arcs(seed: u64, age: f32, center: Vec2, size: Vec2, side: Vec2, down: Vec2) -> Tris {
    let mut tris = Tris::default();
    let remaining = (1.0 - age / DURATION).clamp(0.0, 1.0);

    let mut rng = Rng::new(seed ^ ((age / 0.044) as u64));
    let half = size * Vec2::new(0.58, 0.52);
    let width = (size.y * 0.043).clamp(1.0, 3.5);
    let at = |angle: f32, reach: f32| {
        center + reach * (side * half.x * angle.cos() + down * half.y * angle.sin())
    };
    for i in 0..3 {
        let angle = std::f32::consts::TAU * i as f32 / 3.0 + rng.range(-0.6, 0.6);
        let from = at(angle, 0.76);
        let to = at(angle + rng.range(0.6, 1.65), rng.range(1.02, 1.4));
        let points = jagged(&mut rng, from, to, ARC_SEGMENTS, 0.19);
        ribbon(
            &mut tris,
            &points,
            &[width * 3.0; ARC_SEGMENTS + 1],
            [0.34, 0.24, 1.0, remaining * 0.70],
            [0.32, 0.18, 1.0, 0.0],
        );
        ribbon(
            &mut tris,
            &points,
            &[width; ARC_SEGMENTS + 1],
            [0.91, 0.97, 1.0, remaining.sqrt()],
            [0.54, 0.65, 1.0, 0.0],
        );
    }
    tris
}
