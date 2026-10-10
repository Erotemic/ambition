//! The architecture of the two-state look, as pixels.
//!
//! One block of a room, the ornament below a platform, or the frame of a
//! door: each in the clean state and in the corrupted state. [`colour_at`]
//! gives the colour of one world point of one piece. The corrupted state is
//! not a second set of art: it is computed from the construction of the clean
//! state (the same silhouette in blocks, the same gold lines as lit lines).
//!
//! This is the one authority on what the architecture looks like. It runs one
//! time for each room, on the CPU, into textures ([`super::plates`]). No
//! shader draws the architecture again each frame. Thus nothing here reads
//! the time or the front of the room: what moves is added when the texture is
//! drawn (`room_plate.wgsl`).
//!
//! All positions are in engine world coordinates (y down). Colours are
//! display values with straight alpha.
//!
//! Two rules keep the corrupted state honest about collision:
//!
//! - The top of a block has one bright line, and each other side of the block
//!   has a rim. The rim is on the collision box, not on the ragged blocks.
//! - What is not a place to stand (the ornament below a platform, a block
//!   that grew past the box) has less contrast and lets the sky through.

use bevy::math::{Vec2, Vec3, Vec4};

use super::rand_cell;

const MAGENTA: Vec3 = Vec3::new(1.00, 0.16, 0.66);
const CYAN: Vec3 = Vec3::new(0.13, 0.88, 1.00);
const STONE: Vec3 = Vec3::new(0.890, 0.862, 0.800);
const STONE_LIT: Vec3 = Vec3::new(0.965, 0.945, 0.898);
const STONE_SHADE: Vec3 = Vec3::new(0.775, 0.745, 0.686);
const MORTAR: Vec3 = Vec3::new(0.715, 0.682, 0.620);
const GOLD: Vec3 = Vec3::new(0.788, 0.635, 0.290);
const GOLD_LIT: Vec3 = Vec3::new(0.940, 0.820, 0.500);
const INK_LO: Vec3 = Vec3::new(0.020, 0.018, 0.055);
const INK_HI: Vec3 = Vec3::new(0.120, 0.104, 0.240);
/// The walking line of a corrupted block.
const WALK_LINE: Vec3 = Vec3::new(0.780, 0.720, 1.000);
/// The rim on the sides and the underside of a corrupted block.
const RIM: Vec3 = Vec3::new(0.300, 0.520, 0.900);
/// The corrupted sky, about where the architecture is: what a thing that is
/// not a place to stand fades toward.
const CORRUPT_AIR: Vec3 = Vec3::new(0.300, 0.150, 0.430);

const LEAF_DARK: Vec3 = Vec3::new(0.205, 0.345, 0.225);
const LEAF_MID: Vec3 = Vec3::new(0.305, 0.470, 0.270);
const LEAF_LIT: Vec3 = Vec3::new(0.470, 0.625, 0.340);

/// The voxel edge of the corrupted architecture, in world px.
const VOXEL: f32 = 16.0;
/// How far the art of an underside reaches below its platform: the longest
/// ivy strand. What falls from the platform goes farther, and it is not art
/// of the plate.
const UNDERSIDE_ART_REACH: f32 = 124.0;

/// What a piece of the architecture is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Part {
    /// One solid or one-way block. The piece is the block.
    Surface,
    /// The ornament below one platform. The piece starts at the lower left
    /// corner of the platform, is as wide as it, and is as tall as the
    /// ornament reaches.
    Underside,
    /// The frame of one door. The piece is the trigger box of the door, and
    /// `aspect` is the width of the door sprite as a fraction of its height.
    Door { aspect: f32 },
}

/// One piece of the architecture of a room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Piece {
    pub part: Part,
    pub min: Vec2,
    pub size: Vec2,
}

impl Piece {
    /// How far the art of this piece reaches past `min`/`size`: ivy and
    /// loose blocks past a block, the arch and the pilasters past a door.
    pub fn pad(&self) -> f32 {
        match self.part {
            Part::Surface => 16.0,
            Part::Underside => 0.0,
            Part::Door { .. } => 56.0,
        }
    }

    /// The size of the part of the piece, from `min`, that has art. A plate
    /// holds this part only.
    pub fn art_size(&self) -> Vec2 {
        match self.part {
            Part::Underside => Vec2::new(self.size.x, self.size.y.min(UNDERSIDE_ART_REACH)),
            Part::Surface | Part::Door { .. } => self.size,
        }
    }
}

/// The two states of the look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    Clean,
    Corrupt,
}

/// The colour of `piece` at the world point `p` in `state`.
pub(super) fn colour_at(piece: &Piece, state: State, p: Vec2) -> Vec4 {
    match (piece.part, state) {
        (Part::Surface, State::Clean) => surface_clean(piece, p),
        (Part::Surface, State::Corrupt) => surface_corrupt(piece, p),
        (Part::Underside, State::Clean) => underside_clean(piece, p),
        (Part::Underside, State::Corrupt) => underside_corrupt(piece, p),
        (Part::Door { aspect }, State::Clean) => door_clean(piece, aspect, p),
        (Part::Door { aspect }, State::Corrupt) => door_corrupt(piece, aspect, p),
    }
}

fn luma(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// The shader `smoothstep`. `from` can be more than `to`.
fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

fn opaque(colour: Vec3) -> Vec4 {
    colour.extend(1.0)
}

// --------------------------------------------------------------- surface --

/// Distance to the lines of a gold inlay sigil, for large wall faces.
fn sigil_distance(l: Vec2, s: Vec2) -> f32 {
    if s.x < 150.0 || s.y < 150.0 {
        return 1.0e4;
    }
    let period = 320.0;
    // Centre the lattice on the piece.
    let o = l - s * 0.5 + Vec2::splat(period * 0.5);
    let cell = (o / period).floor();
    let centre = (cell + Vec2::splat(0.5)) * period + s * 0.5 - Vec2::splat(period * 0.5);
    // Keep the whole sigil on the face.
    let radius = 34.0;
    if centre.x < radius * 1.8
        || centre.y < radius * 1.8 + 16.0
        || centre.x > s.x - radius * 1.8
        || centre.y > s.y - radius * 1.8
    {
        return 1.0e4;
    }
    let d = l - centre;
    let len = d.length();
    let mut dist = (len - radius).abs();
    dist = dist.min((len - radius * 0.62).abs());
    dist = dist.min((d.x.abs() + d.y.abs() - radius * 0.62).abs() * 0.7071);
    if d.y.abs() < radius * 1.5 {
        dist = dist.min(d.x.abs());
    }
    dist.min((len - radius * 0.15).abs())
}

/// The clean architecture of one block: its colour, and the distance to its
/// nearest gold ornament line (the corrupted state lights the same lines).
/// `l` is the position in the block, `s` is the block size, `p` is the world
/// position: the masonry is laid in world space, so two blocks that touch
/// share their courses.
fn clean_stone(l: Vec2, s: Vec2, p: Vec2) -> (Vec3, f32) {
    let mut col;
    let mut ornament = 1.0e4_f32;
    let pillar = s.x <= 48.0 && s.y > 96.0;
    let slab = s.y <= 40.0 && !pillar;

    if pillar {
        // A fluted shaft with gold bands.
        let flute = l.x % 8.0;
        col = STONE * (1.0 - 0.07 * smoothstep(0.0, 3.0, (flute - 4.0).abs()) + 0.035);
        if flute < 1.0 {
            col = MORTAR;
        }
        let band = (l.y + 60.0) % 224.0;
        if band < 22.0 {
            col = if band > 15.0 { STONE_SHADE } else { STONE_LIT };
            ornament = (band - 3.0).abs().min((band - 13.0).abs());
        }
    } else if slab {
        // A platform: one dressed stone with a groove.
        col = STONE * (1.0 - 0.05 * (l.y / s.y));
        let joint = (p.x + 24.0) % 96.0;
        if joint < 1.0 && l.y > 10.0 {
            col = MORTAR;
        }
        // An engraved line with a gold stud in each stone.
        let mid = (11.0 + s.y - 7.0) * 0.5;
        if (l.y - mid).abs() < 0.6 {
            col *= 0.90;
        }
        let sx = (p.x + 24.0) % 96.0 - 48.0;
        let stud = sx.abs() + (l.y - mid).abs() - 3.2;
        ornament = ornament.min(stud.max(0.0) + 0.45);
    } else {
        // Coursed masonry, half-bond.
        let course = 16.0;
        let brick = 32.0;
        let row = (p.y / course).floor();
        let shift = (row - 2.0 * (row * 0.5).floor()) * brick * 0.5;
        let bx = ((p.x + shift) / brick).floor();
        let fy = p.y - row * course;
        let fx = p.x + shift - bx * brick;
        let tone = 1.0 + (rand_cell(Vec2::new(bx, row), 12) - 0.5) * 0.06;
        col = STONE * tone * (1.0 - 0.07 * (l.y / 420.0).clamp(0.0, 1.0));
        if fy < 1.0 || fx < 1.0 {
            col = mix(col, MORTAR, 0.75);
        } else if fy < 2.0 {
            col *= 1.03;
        }
        ornament = ornament.min(sigil_distance(l, s));
    }

    if !pillar {
        // The cap: a lit top, a shaded fascia, a gold fillet, dentils.
        if l.y < 5.0 {
            col = STONE_LIT;
        } else if l.y < 8.0 {
            col = STONE_SHADE;
        } else if l.y < 11.0 {
            col = STONE_LIT * 0.97;
        } else if l.y < 16.0 && !slab {
            col = if (l.x % 12.0) < 6.0 { STONE_LIT } else { STONE_SHADE * 0.94 };
        }
        ornament = ornament.min((l.y - 9.0).abs() + 0.4);
        if slab {
            // The underside steps in.
            if l.y > s.y - 5.0 {
                col = STONE_SHADE * 0.93;
            }
            ornament = ornament.min((l.y - (s.y - 7.0)).abs() + 0.6);
        }
    }
    // Light from the upper left.
    if l.x < 2.0 {
        col = mix(col, STONE_LIT, 0.8);
    }
    if l.x > s.x - 3.0 {
        col = mix(col, STONE_SHADE, 0.8);
    }
    if l.y > s.y - 2.0 {
        col = mix(col, STONE_SHADE, 0.8);
    }

    // Gold: a bright core and a darker edge.
    let gold = smoothstep(1.5, 0.5, ornament);
    let glint = 0.5 + 0.5 * ((p.x + p.y) * 0.06).sin();
    col = mix(col, mix(GOLD, GOLD_LIT, glint * glint), gold);
    (col, ornament)
}

/// What the voxel `cell` of a corrupted block is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Voxel {
    /// No block.
    None,
    /// A block of the mass.
    Solid,
    /// A block that the mass lost, inside the collision box.
    Lost,
    /// A block that grew past a side or the underside of the box.
    Grown,
}

impl Voxel {
    /// Whether light stops here: a block of the mass.
    fn is_mass(self) -> bool {
        self == Self::Solid
    }
}

/// What the voxel `cell` of the corrupted block (`min`, `s`) is. The top row
/// is always of the mass, so the standing surface stays where collision is.
fn voxel(cell: Vec2, min: Vec2, s: Vec2) -> Voxel {
    let c = (cell + Vec2::splat(0.5)) * VOXEL - min;
    let inside = c.x >= 0.0 && c.y >= 0.0 && c.x < s.x && c.y < s.y;
    let r = rand_cell(cell, 600);
    if inside {
        let top_row = c.y < VOXEL;
        let edge = c.x < VOXEL || c.x > s.x - VOXEL || c.y > s.y - VOXEL;
        return if top_row || !edge || r > 0.20 { Voxel::Solid } else { Voxel::Lost };
    }
    // Never above the top.
    let near = c.x > -VOXEL && c.x < s.x + VOXEL && c.y > 0.0 && c.y < s.y + VOXEL;
    if near && r < 0.07 {
        Voxel::Grown
    } else {
        Voxel::None
    }
}

/// The corrupted architecture of one block.
fn corrupt_stone(l: Vec2, s: Vec2, p: Vec2, min: Vec2) -> Vec4 {
    let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < s.x && l.y < s.y;
    let cell = (p / VOXEL).floor();
    // The voxels are on the world lattice, and a box need not be. A voxel
    // whose middle is past the box is still of the mass where the box is: the
    // box has no hole.
    let here = match voxel(cell, min, s) {
        Voxel::None | Voxel::Grown if inside => Voxel::Solid,
        // And a voxel of the box stops at the box: the mass has no part past
        // it that reads as more floor or more wall.
        Voxel::Solid | Voxel::Lost if !inside => Voxel::None,
        here => here,
    };
    let u = p - cell * VOXEL;
    let r = rand_cell(cell, 601);
    let r2 = rand_cell(cell, 602);
    // The rim of the collision box: the box is where the body stops, with
    // whatever blocks the mass lost.
    let rim = inside && (l.x < 1.5 || l.x >= s.x - 1.5 || l.y >= s.y - 1.5);
    let walk = inside && l.y < 2.0;
    match here {
        Voxel::None => return Vec4::ZERO,
        Voxel::Grown => {
            // Not a place to stand: a dim block that the sky shows through.
            let seam = u.x >= VOXEL - 1.0 || u.y >= VOXEL - 1.0;
            let ink = mix(mix(INK_LO, INK_HI, 0.35 + 0.4 * r), CORRUPT_AIR, 0.45);
            return (ink * if seam { 0.7 } else { 1.0 }).extend(0.55);
        }
        Voxel::Lost => {
            // The box is whole here, so it is not the sky: a dark hollow.
            let mut ink = mix(INK_LO, INK_HI, 0.08 + 0.06 * r);
            if rim {
                ink = mix(ink, RIM, 0.70);
            }
            return opaque(ink);
        }
        Voxel::Solid => {}
    }
    // The block keeps the shading of the stone it was.
    let (stone, ornament) = clean_stone(l.clamp(Vec2::ZERO, s - Vec2::splat(0.01)), s, p);
    let v = ((luma(stone) - 0.55) / 0.40).clamp(0.0, 1.0);
    let v = 0.22 + 0.42 * v + 0.36 * r;
    let mut ink = mix(INK_LO, INK_HI, v) + Vec3::new(0.030, -0.010, -0.030) * (r2 - 0.5);

    let above = voxel(cell + Vec2::new(0.0, -1.0), min, s).is_mass();
    let below = voxel(cell + Vec2::new(0.0, 1.0), min, s).is_mass();
    let left = voxel(cell + Vec2::new(-1.0, 0.0), min, s).is_mass();
    let right = voxel(cell + Vec2::new(1.0, 0.0), min, s).is_mass();
    // Voxel faces: a lit top where nothing is above, a lit left side.
    if !above && u.y < 5.0 {
        ink = ink * 1.25 + Vec3::new(0.150, 0.130, 0.280);
    }
    if !left && u.x < 2.0 {
        ink = ink * 1.2 + Vec3::splat(0.030);
    }
    // Seams. Some voxels are 2x2, with no seam inside.
    let big = rand_cell((cell * 0.5).floor(), 610) < 0.4;
    let u2 = p - (p / (VOXEL * 2.0)).floor() * VOXEL * 2.0;
    let seam = (u.x >= VOXEL - 1.0 || u.y >= VOXEL - 1.0)
        && !(big && u2.x < VOXEL * 2.0 - 1.0 && u2.y < VOXEL * 2.0 - 1.0);
    if seam {
        ink *= 0.5;
    }

    let mut neon = Vec3::ZERO;
    // The light of a line: a core and a short glow.
    let line = |d: f32| smoothstep(1.2, 0.2, d.abs()) + (-d.abs() / 3.0).exp() * 0.30;
    // Cracks run along seams and light the stone beside them.
    let crack_h = below && rand_cell(Vec2::new((p.x / 80.0).floor(), cell.y), 620) < 0.10;
    let crack_v = right && rand_cell(Vec2::new(cell.x, (p.y / 64.0).floor()), 621) < 0.08;
    if crack_h {
        neon = neon.max(MAGENTA * line(VOXEL - 0.5 - u.y) * 0.86);
    }
    if crack_v {
        neon = neon.max(MAGENTA * line(VOXEL - 0.5 - u.x) * 0.86);
    }
    // Exposed edges inside the box: cyan describes them in some regions,
    // magenta in a few.
    let zone = rand_cell((p / 112.0).floor(), 630);
    let mut edge = 1.0e4_f32;
    if !above {
        edge = edge.min(u.y);
    }
    if !left {
        edge = edge.min(u.x);
    }
    if !right {
        edge = edge.min(VOXEL - 1.0 - u.x);
    }
    if !below {
        edge = edge.min(VOXEL - 1.0 - u.y);
    }
    let edge_light = smoothstep(1.1, 0.3, edge) + (-edge / 3.0).exp() * 0.14;
    // Not the top: the walking line owns it.
    let side = edge < u.y || above;
    if zone < 0.26 && side {
        neon = neon.max(CYAN * edge_light * 0.85);
    } else if zone > 0.92 && side {
        neon = neon.max(MAGENTA * edge_light * 0.85);
    }
    // The gold ornament is the same drawing, now a circuit.
    let circuit = if rand_cell((p / 224.0).floor(), 640) >= 0.5 { CYAN } else { MAGENTA };
    let lit = smoothstep(1.3, 0.4, ornament) + (-ornament / 4.0).exp() * 0.18;
    neon = neon.max(circuit * lit * 0.94);
    // Scanlines.
    ink *= 0.93 + 0.07 * (p.y.floor() % 2.0);
    let mut col = ink + neon;
    if rim {
        col = mix(col, RIM, 0.80);
    }
    if walk {
        col = WALK_LINE;
    }
    opaque(col)
}

// ------------------------------------------------------------------- ivy --

fn leaf_colour(leaf: u8) -> Vec3 {
    match leaf {
        1 => LEAF_DARK,
        2 => LEAF_MID,
        _ => LEAF_LIT,
    }
}

/// What ivy is at `q`, which is from a top corner of a block: x goes into
/// the block from its end, y goes down. 0 = none, 1..=3 = a leaf, dark to
/// lit. The ivy is thick on the corner and thin away from it.
fn ivy_clump(q: Vec2, key: Vec2, salt: u32) -> u8 {
    if q.x < -11.0 || q.x > 52.0 || q.y < -8.0 || q.y > 44.0 {
        return 0;
    }
    let from_corner = Vec2::new(q.x - 6.0, (q.y - 5.0) * 0.75).length();
    // Thin to nothing at the edge of the piece, so no cut shows.
    let fringe = smoothstep(-8.0, -1.0, q.y) * smoothstep(-11.0, -4.0, q.x);
    let density = (-(from_corner - 9.0).max(0.0) / 12.0).exp() * 0.95 * fringe;
    let cell = (q / 6.0).floor();
    let id = cell + key;
    if rand_cell(id, salt) > density {
        return 0;
    }
    let jitter = Vec2::new(rand_cell(id, salt + 1), rand_cell(id, salt + 2)) - Vec2::splat(0.5);
    let o = q - (cell + Vec2::splat(0.5)) * 6.0 - jitter * 2.4;
    let angle = rand_cell(id, salt + 3) * 3.1416;
    let (sin, cos) = angle.sin_cos();
    let e = Vec2::new(o.x * cos + o.y * sin, o.y * cos - o.x * sin) / Vec2::new(3.9, 2.4);
    if e.dot(e) > 1.0 {
        return 0;
    }
    1 + (rand_cell(id, salt + 4) * 2.99).floor() as u8
}

/// The key of a block, the same for its surface piece and its underside
/// piece: the left edge and the bottom edge of the block.
fn ivy_key(left: f32, bottom: f32) -> Vec2 {
    Vec2::new(left, bottom)
}

/// Whether an end of the block has ivy. `end` is 0.0 for the left end.
fn has_ivy(key: Vec2, end: f32) -> bool {
    rand_cell(key + Vec2::new(end * 13.0, 0.0), 700) < 0.55
}

/// The ivy on the top corners of a block. `l` is the position in the block.
fn ivy_on_surface(l: Vec2, s: Vec2, key: Vec2) -> u8 {
    if s.x < 96.0 || l.y > 44.0 || (l.x > 52.0 && l.x < s.x - 52.0) {
        return 0;
    }
    if has_ivy(key, 0.0) {
        let leaf = ivy_clump(l, key, 710);
        if leaf != 0 {
            return leaf;
        }
    }
    if has_ivy(key, 1.0) {
        return ivy_clump(Vec2::new(s.x - l.x, l.y), key + Vec2::new(7.0, 3.0), 720);
    }
    0
}

/// The ivy that hangs below the ends of a platform: three strands at each end
/// that has ivy. `l` is from the lower left corner of the platform, `w` is
/// its width.
///
/// The strands do not move. A strand that moved in the wind was a line one
/// pixel wide that changed pixels each frame, which the eye reads as a
/// flicker and not as wind.
fn ivy_strands(l: Vec2, w: f32, key: Vec2) -> u8 {
    if l.y < 0.0 || l.y > 120.0 || (l.x > 40.0 && l.x < w - 40.0) {
        return 0;
    }
    for end in 0..2 {
        if !has_ivy(key, end as f32) {
            continue;
        }
        let x = if end == 1 { w - l.x } else { l.x };
        for k in 0..3 {
            let id = key + Vec2::new(k as f32 * 5.0 + end as f32 * 31.0, 0.0);
            let len = 34.0 + 84.0 * rand_cell(id, 730);
            if l.y > len {
                continue;
            }
            let sx = 4.0 + k as f32 * 9.0 + (rand_cell(id, 731) - 0.5) * 5.0;
            let dx = x - sx;
            if dx.abs() < 0.7 {
                return 1;
            }
            // A leaf on each side in turn, every 7 px, with some left out.
            let row = (l.y / 7.0).floor();
            if rand_cell(id + Vec2::new(0.0, row), 732) < 0.78 {
                let side = if (row as i32 + k) % 2 == 0 { 1.0 } else { -1.0 };
                let o = Vec2::new(dx - side * 3.4, l.y - (row + 0.5) * 7.0);
                let e = Vec2::new(o.x + o.y * 0.35 * side, o.y) / Vec2::new(3.6, 2.2);
                if e.dot(e) < 1.0 {
                    return 2 + (rand_cell(id + Vec2::new(1.0, row), 733) * 1.99) as u8;
                }
            }
        }
    }
    0
}

/// A leaf after the rewrite: a dead block, and a few of them are lit. It is
/// not black: black blocks beside a platform read as pieces of the platform.
fn dead_leaf(p: Vec2) -> Vec4 {
    let r = rand_cell((p / 4.0).floor(), 740);
    if r > 0.95 {
        return (CYAN * 0.75).extend(0.9);
    }
    mix(Vec3::new(0.060, 0.130, 0.150) * (0.7 + 0.6 * r), CORRUPT_AIR, 0.30).extend(0.80)
}

/// `p` snapped to the middle of its block of 4 px, as a position from `min`.
fn in_blocks_of_4(p: Vec2, min: Vec2) -> Vec2 {
    ((p / 4.0).floor() + Vec2::splat(0.5)) * 4.0 - min
}

fn surface_clean(piece: &Piece, p: Vec2) -> Vec4 {
    let l = p - piece.min;
    let s = piece.size;
    let leaf = ivy_on_surface(l, s, ivy_key(piece.min.x, piece.min.y + s.y));
    if leaf != 0 {
        return opaque(leaf_colour(leaf));
    }
    let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < s.x && l.y < s.y;
    if !inside {
        return Vec4::ZERO;
    }
    opaque(clean_stone(l, s, p).0)
}

fn surface_corrupt(piece: &Piece, p: Vec2) -> Vec4 {
    let l = p - piece.min;
    let s = piece.size;
    let stone = corrupt_stone(l, s, p, piece.min);
    // The walking line is over the ivy: it is where the body stands.
    let walk = l.x >= 0.0 && l.x < s.x && l.y >= 0.0 && l.y < 2.0;
    if !walk && ivy_on_surface(in_blocks_of_4(p, piece.min), s, ivy_key(piece.min.x, piece.min.y + s.y)) != 0 {
        let leaf = dead_leaf(p);
        // On the box the leaf is on the stone, and the box stays opaque.
        if stone.w >= 1.0 {
            return opaque(mix(stone.truncate(), leaf.truncate(), leaf.w));
        }
        return leaf;
    }
    stone
}

// ------------------------------------------------------------- underside --

/// What the ornament below a platform is made of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ornament {
    Air,
    Stone,
    GoldLine,
    Cloth,
    ClothGold,
}

/// What the ornament below a platform is made of at `l` (from the platform's
/// lower left corner, y down). `w` is the platform width, and `min` is that
/// corner in the world.
fn underside_part(l: Vec2, w: f32, min: Vec2) -> Ornament {
    if l.x < 0.0 || l.x >= w || l.y < 0.0 {
        return Ornament::Air;
    }
    let bays = (w / 150.0).round().max(1.0);
    let bay = w / bays;
    let i = (l.x / bay).floor();
    let x = l.x - (i + 0.5) * bay;
    let half = bay * 0.5;
    // The soffit.
    if l.y < 6.0 {
        return Ornament::Stone;
    }
    // A stepped bracket at each end of the bay.
    let from_end = half - x.abs();
    let step = (l.y / 8.0).floor();
    if from_end < 17.0 - step * 3.0 && l.y < 48.0 {
        return Ornament::Stone;
    }
    // A shallow arch between the brackets: thin at the crown.
    let span = half - 15.0;
    let rise = (span * 0.55).min(38.0);
    if x.abs() < span {
        let k = x.abs() / span;
        let curve = 6.0 + rise * (1.0 - (1.0 - k * k).max(0.0).sqrt());
        if l.y < curve {
            return if l.y > curve - 1.8 { Ornament::GoldLine } else { Ornament::Stone };
        }
    }
    // A banner hangs in some bays.
    if bay >= 110.0 && rand_cell(Vec2::new(i + min.x, min.y), 660) < 0.55 {
        let ax = x.abs();
        let hem = 66.0 + ax * 0.9;
        if ax < 14.0 && l.y < hem {
            let emblem = (ax + (l.y - 36.0).abs() - 6.5).abs();
            if ax > 11.8 || l.y > hem - 2.2 || emblem < 1.0 {
                return Ornament::ClothGold;
            }
            return Ornament::Cloth;
        }
    }
    Ornament::Air
}

fn underside_clean(piece: &Piece, p: Vec2) -> Vec4 {
    let l = p - piece.min;
    let w = piece.size.x;
    let leaf = ivy_strands(l, w, ivy_key(piece.min.x, piece.min.y));
    if leaf != 0 {
        return opaque(leaf_colour(leaf));
    }
    match underside_part(l, w, piece.min) {
        Ornament::Air => Vec4::ZERO,
        Ornament::Stone => opaque(mix(STONE, STONE_SHADE, 0.62) * (1.0 - 0.10 * (l.y / 46.0).clamp(0.0, 1.0))),
        Ornament::GoldLine | Ornament::ClothGold => opaque(mix(GOLD, GOLD_LIT, 0.35)),
        Ornament::Cloth => {
            let fold = 0.92 + 0.08 * (l.x * 0.9).sin();
            opaque(Vec3::new(0.300, 0.395, 0.585) * fold)
        }
    }
}

fn underside_corrupt(piece: &Piece, p: Vec2) -> Vec4 {
    let l = p - piece.min;
    let w = piece.size.x;
    if ivy_strands(in_blocks_of_4(p, piece.min), w, ivy_key(piece.min.x, piece.min.y)) != 0 {
        return dead_leaf(p);
    }
    // The same ornament, in blocks half the size of the architecture's.
    let vox = VOXEL * 0.5;
    let cell = (p / vox).floor();
    let part = underside_part((cell + Vec2::splat(0.5)) * vox - piece.min, w, piece.min);
    let r = rand_cell(cell, 670);
    if part == Ornament::Air || r <= 0.14 {
        return Vec4::ZERO;
    }
    // It is not a place to stand, so it is not as dark as a block, it has no
    // hard seams, and the sky shows through it.
    let mut col = mix(INK_LO, INK_HI, 0.45 + 0.4 * r);
    if part == Ornament::Cloth || part == Ornament::ClothGold {
        col = Vec3::new(0.150, 0.045, 0.160) * (0.8 + 0.4 * r);
    }
    col = mix(col, CORRUPT_AIR, 0.42);
    let u = p - cell * vox;
    if u.x >= vox - 1.0 || u.y >= vox - 1.0 {
        col *= 0.85;
    }
    // The mass is in blocks. The gold lines stay thin, and they are lit.
    let exact = underside_part(l, w, piece.min);
    if exact == Ornament::GoldLine {
        col = mix(col, CYAN, 0.55);
    } else if exact == Ornament::ClothGold {
        col = mix(col, MAGENTA, 0.55);
    }
    col.extend(0.78)
}

// ------------------------------------------------------------------ door --

/// What the frame of a door is made of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Frame {
    Air,
    Stone,
    LitStone,
    GoldLine,
    /// The recessed field in the arch.
    Field,
    ShadedStone,
    Keystone,
}

/// What the frame of a door is made of at `l` (from the upper left corner of
/// the door sprite, y down). `s` is the size of the door sprite.
fn door_part(l: Vec2, s: Vec2) -> Frame {
    let cx = l.x - s.x * 0.5;
    let ax = cx.abs();
    let half = s.x * 0.5;
    let inner = half + 2.0;
    let outer = half + 15.0;
    if l.y > s.y {
        return Frame::Air;
    }
    if l.y >= 0.0 {
        // Two pilasters on a plinth. The door fills the opening.
        if ax < half - 1.0 {
            return Frame::Air;
        }
        if l.y > s.y - 9.0 && ax < outer + 4.0 {
            return if l.y < s.y - 7.0 { Frame::LitStone } else { Frame::ShadedStone };
        }
        if ax >= inner && ax < outer {
            let across = (ax - inner) / (outer - inner);
            if (across - 0.5).abs() < 0.06 {
                return Frame::ShadedStone;
            }
            return if (cx < 0.0) == (across > 0.78) { Frame::LitStone } else { Frame::Stone };
        }
        return Frame::Air;
    }
    // The impost: a band at the spring of the arch.
    if l.y >= -6.0 {
        if ax < outer + 3.0 {
            return if l.y > -2.4 && l.y < -0.8 { Frame::GoldLine } else { Frame::LitStone };
        }
        return Frame::Air;
    }
    // The arch.
    let up = -(l.y + 6.0);
    let d = Vec2::new(cx, up).length();
    if ax < 5.5 && d >= inner && d < outer + 5.0 {
        return Frame::Keystone;
    }
    if d < inner {
        // A small gold figure in the field.
        let figure = (ax + (up - inner * 0.46).abs() - 5.0).abs();
        return if figure < 0.9 { Frame::GoldLine } else { Frame::Field };
    }
    if d < outer {
        return if d < inner + 1.7 || d > outer - 1.5 { Frame::GoldLine } else { Frame::Stone };
    }
    Frame::Air
}

/// The size of the door sprite of `piece`, and its upper left corner. The
/// sprite stands on the floor of its trigger box, centred.
fn door_sprite(piece: &Piece, aspect: f32) -> (Vec2, Vec2) {
    let s = Vec2::new(piece.size.y * aspect, piece.size.y);
    let origin = Vec2::new(piece.min.x + (piece.size.x - s.x) * 0.5, piece.min.y);
    (s, origin)
}

fn door_clean(piece: &Piece, aspect: f32, p: Vec2) -> Vec4 {
    let (s, origin) = door_sprite(piece, aspect);
    opaque(match door_part(p - origin, s) {
        Frame::Air => return Vec4::ZERO,
        Frame::Stone => mix(STONE, STONE_SHADE, 0.20),
        Frame::LitStone => STONE_LIT,
        Frame::GoldLine => mix(GOLD, GOLD_LIT, 0.30),
        Frame::Field => mix(STONE, STONE_SHADE, 0.70),
        Frame::ShadedStone => STONE_SHADE,
        Frame::Keystone => GOLD_LIT,
    })
}

fn door_corrupt(piece: &Piece, aspect: f32, p: Vec2) -> Vec4 {
    let (s, origin) = door_sprite(piece, aspect);
    let vox = VOXEL * 0.5;
    let cell = (p / vox).floor();
    let part = door_part((cell + Vec2::splat(0.5)) * vox - origin, s);
    let r = rand_cell(cell, 690);
    if part == Frame::Air || r < 0.10 {
        return Vec4::ZERO;
    }
    let u = p - cell * vox;
    let mut col = mix(INK_LO, INK_HI, 0.20 + 0.55 * r);
    if part == Frame::LitStone {
        col = col * 1.5 + Vec3::new(0.030, 0.026, 0.060);
    }
    if part == Frame::Field {
        col = INK_LO + MAGENTA * 0.11;
    }
    if u.x >= vox - 1.0 || u.y >= vox - 1.0 {
        col *= 0.55;
    }
    // The mass is in blocks. The gold lines stay thin, and they are lit.
    if door_part(p - origin, s) == Frame::GoldLine {
        col = mix(col, CYAN, 0.90);
    }
    if part == Frame::Keystone {
        col = MAGENTA * 0.85 + Vec3::splat(0.20);
    }
    opaque(col)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(size: Vec2) -> Piece {
        Piece { part: Part::Surface, min: Vec2::new(320.0, 480.0), size }
    }

    /// Each texel of a piece, with its world point: the piece and its pad.
    fn texels(piece: &Piece) -> impl Iterator<Item = Vec2> + '_ {
        let pad = piece.pad();
        let from = piece.min - Vec2::splat(pad);
        let size = piece.size + Vec2::splat(pad * 2.0);
        (0..size.y as u32).flat_map(move |y| (0..size.x as u32).map(move |x| from + Vec2::new(x as f32 + 0.5, y as f32 + 0.5)))
    }

    /// The rule that keeps the corrupted state honest: where the body stands
    /// there is one bright line, on each column of the top of the box.
    #[test]
    fn the_top_of_a_corrupted_block_is_one_bright_line() {
        for (size, min) in [
            (Vec2::new(256.0, 32.0), Vec2::new(320.0, 480.0)),
            (Vec2::new(48.0, 192.0), Vec2::new(320.0, 480.0)),
            (Vec2::new(400.0, 300.0), Vec2::new(320.0, 480.0)),
            // Not on the voxel lattice.
            (Vec2::new(250.0, 30.0), Vec2::new(323.0, 490.0)),
        ] {
            let piece = Piece { part: Part::Surface, min, size };
            for x in 0..size.x as u32 {
                for y in 0..2 {
                    let p = piece.min + Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let colour = colour_at(&piece, State::Corrupt, p);
                    assert_eq!(colour, opaque(WALK_LINE), "block {size}: column {x}, row {y}");
                }
            }
        }
    }

    /// The corrupted block is opaque over its whole collision box: no block
    /// that the mass lost shows the sky where the body stops.
    #[test]
    fn a_corrupted_block_is_opaque_over_its_collision_box() {
        let mut lost = 0;
        for piece in [block(Vec2::new(400.0, 300.0)), Piece { part: Part::Surface, min: Vec2::new(323.0, 490.0), size: Vec2::new(250.0, 90.0) }] {
          for p in texels(&piece) {
            let l = p - piece.min;
            let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < piece.size.x && l.y < piece.size.y;
            let colour = colour_at(&piece, State::Corrupt, p);
            if inside {
                assert_eq!(colour.w, 1.0, "the box shows the sky at {l}");
                lost += usize::from(voxel((p / VOXEL).floor(), piece.min, piece.size) == Voxel::Lost);
            } else {
                assert!(colour.w < 1.0, "past the box at {l} there is an opaque block: it reads as a place to stand");
            }
          }
        }
        assert!(lost > 0, "this block lost no voxel, so the test did not see the case it is for");
    }

    /// Clean stone fills its box and only ivy is past it.
    #[test]
    fn clean_stone_fills_its_box_and_only_ivy_is_past_it() {
        let piece = block(Vec2::new(256.0, 32.0));
        let (mut past, mut leaves) = (0, 0);
        for p in texels(&piece) {
            let l = p - piece.min;
            let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < piece.size.x && l.y < piece.size.y;
            let colour = colour_at(&piece, State::Clean, p);
            if inside {
                assert_eq!(colour.w, 1.0, "clean stone has a hole at {l}");
            } else if colour.w > 0.0 {
                past += 1;
                let rgb = colour.truncate();
                leaves += usize::from([LEAF_DARK, LEAF_MID, LEAF_LIT].contains(&rgb));
            }
        }
        assert_eq!(past, leaves, "something that is not a leaf is drawn past the box");
    }

    /// A plate holds the art part of a piece only, so no art is past it.
    #[test]
    fn an_underside_has_no_art_below_its_art_reach() {
        let piece = Piece { part: Part::Underside, min: Vec2::new(320.0, 512.0), size: Vec2::new(900.0, 230.0) };
        assert!(piece.art_size().y < piece.size.y);
        let mut drawn = 0;
        for p in texels(&piece) {
            for state in [State::Clean, State::Corrupt] {
                let alpha = colour_at(&piece, state, p).w;
                drawn += usize::from(alpha > 0.0);
                assert!(alpha == 0.0 || p.y - piece.min.y < piece.art_size().y, "{state:?} art at {}", p - piece.min);
            }
        }
        assert!(drawn > 1000, "the underside drew {drawn} texels");
    }

    /// The two states of an underside and of a door frame have one
    /// silhouette: the corrupted one is the clean one in blocks. A piece that
    /// is empty in one state is a piece that was not generated.
    #[test]
    fn each_part_draws_something_in_each_state() {
        let pieces = [
            block(Vec2::new(256.0, 32.0)),
            Piece { part: Part::Underside, min: Vec2::new(320.0, 512.0), size: Vec2::new(256.0, 230.0) },
            Piece { part: Part::Door { aspect: 0.52 }, min: Vec2::new(900.0, 300.0), size: Vec2::new(96.0, 160.0) },
        ];
        for piece in &pieces {
            for state in [State::Clean, State::Corrupt] {
                let drawn = texels(piece).filter(|p| colour_at(piece, state, *p).w > 0.0).count();
                assert!(drawn > 200, "{:?} in {state:?} draws {drawn} texels", piece.part);
            }
        }
    }
}
