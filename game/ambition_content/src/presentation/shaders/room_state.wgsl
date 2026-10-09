// One architecture in two states: clean and corrupted. See `room_look.rs`.
//
// All work is in engine world coordinates (y down). Colours are written as
// display values and converted once, at the end.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::{view, globals}
#import ambition_content::room_look::{rand_cell, value_noise, towers, arcade, island, sky_line_distance}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> piece: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> room: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> front: vec4<f32>;

const MAGENTA: vec3<f32> = vec3<f32>(1.00, 0.16, 0.66);
const CYAN: vec3<f32> = vec3<f32>(0.13, 0.88, 1.00);
const STONE: vec3<f32> = vec3<f32>(0.890, 0.862, 0.800);
const STONE_LIT: vec3<f32> = vec3<f32>(0.965, 0.945, 0.898);
const STONE_SHADE: vec3<f32> = vec3<f32>(0.775, 0.745, 0.686);
const MORTAR: vec3<f32> = vec3<f32>(0.715, 0.682, 0.620);
const GOLD: vec3<f32> = vec3<f32>(0.788, 0.635, 0.290);
const GOLD_LIT: vec3<f32> = vec3<f32>(0.940, 0.820, 0.500);
const INK_LO: vec3<f32> = vec3<f32>(0.020, 0.018, 0.055);
const INK_HI: vec3<f32> = vec3<f32>(0.120, 0.104, 0.240);

/// The voxel edge of the corrupted architecture, in world px.
const VOXEL: f32 = 16.0;

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// ----------------------------------------------------------------- field --

/// Signed world distance behind the front. Positive is corrupted.
fn field(p: vec2<f32>) -> f32 {
    let t = globals.time;
    let wobble = (value_noise(p, 260.0, 7u) - 0.5) * 260.0
        + (value_noise(p + vec2<f32>(t * 5.0, t * 2.0), 90.0, 11u) - 0.5) * 70.0;
    let breathe = sin(t * 0.33) * 30.0;
    return dot(p - front.xy, front.zw) + wobble + breathe;
}

struct Level {
    size: f32,
    // How far behind the front the solid mass of this level starts.
    depth: f32,
    // How ragged that start is, per cell.
    jitter: f32,
    // How far the loose blocks of this level scatter ahead of the mass.
    reach: f32,
    // How many of them, at the mass edge.
    loose: f32,
}

fn level(k: i32) -> Level {
    if k == 0 { return Level(128.0, 360.0, 180.0, 0.0, 0.0); }
    if k == 1 { return Level(64.0, 90.0, 110.0, 150.0, 0.09); }
    if k == 2 { return Level(32.0, 10.0, 56.0, 280.0, 0.15); }
    return Level(16.0, -6.0, 28.0, 340.0, 0.09);
}

struct Claim {
    // 0 = clean, 1 = the corrupted mass, 2 = a loose block.
    state: i32,
    size: f32,
    cell: vec2<f32>,
    r: f32,
    r2: f32,
}

/// Which block of the front claims `p`: the coarsest level that is corrupt.
fn claim(p: vec2<f32>) -> Claim {
    for (var k = 0; k < 4; k++) {
        let lv = level(k);
        let cell = floor(p / lv.size);
        let salt = u32(k);
        let fc = field((cell + vec2<f32>(0.5)) * lv.size);
        let ra = rand_cell(cell, 100u + salt);
        let rb = rand_cell(cell, 150u + salt);
        let mass = fc + (ra - 0.5) * lv.jitter > lv.depth;
        let gap = clamp((lv.depth - fc) / max(lv.reach, 1.0), 0.0, 1.0);
        let stray = fc > lv.depth - lv.reach && rb < lv.loose * (1.0 - gap) * (1.0 - gap);
        if mass || stray {
            let r = rand_cell(cell, 200u + salt);
            let r2 = rand_cell(cell, 300u + salt);
            return Claim(select(2, 1, mass), lv.size, cell, r, r2);
        }
    }
    return Claim(0, 0.0, vec2<f32>(0.0), 0.0, 0.0);
}

// -------------------------------------------------------------- backdrop --

fn camera_engine() -> vec2<f32> {
    let c = view.world_position.xy;
    return vec2<f32>(c.x + room.x * 0.5, room.y * 0.5 - c.y);
}

fn backdrop_clean(p: vec2<f32>) -> vec3<f32> {
    let cam = camera_engine();
    let g = clamp(p.y / room.y, 0.0, 1.0);
    var col = mix(vec3<f32>(0.972, 0.952, 0.905), vec3<f32>(0.868, 0.895, 0.938), g);
    // Drawing paper: a faint grid.
    let grid = p % 64.0;
    if grid.x < 1.0 || grid.y < 1.0 {
        col = col * 0.975;
    }
    // A far city, then towers at two depths.
    let city = towers(p - cam * 0.78 + vec2<f32>(31.0, -140.0), 88.0, 28u, room.y);
    col = mix(col, vec3<f32>(0.868, 0.888, 0.925), city * 0.70);
    let far_t = towers(p - cam * 0.62, 170.0, 20u, room.y);
    col = mix(col, vec3<f32>(0.822, 0.850, 0.902), far_t * 0.85);
    let near_t = towers(p - cam * 0.42 + vec2<f32>(97.0, 60.0), 370.0, 24u, room.y);
    col = mix(col, vec3<f32>(0.770, 0.805, 0.872), near_t * 0.90);
    // Islands that float between the towers and the viaduct.
    let isle = island(p - cam * 0.36 + vec2<f32>(140.0, 40.0));
    if isle.x > 0.5 {
        var stone = mix(vec3<f32>(0.800, 0.828, 0.888), vec3<f32>(0.740, 0.772, 0.846), isle.z);
        if isle.y < 4.0 { stone = vec3<f32>(0.940, 0.945, 0.955); }
        if isle.y >= 5.0 && isle.y < 6.5 { stone = vec3<f32>(0.820, 0.730, 0.500); }
        col = mix(col, stone, 0.90);
    }
    // A viaduct of arches, nearer again.
    let via = arcade(p - cam * 0.28);
    col = mix(col, vec3<f32>(0.735, 0.770, 0.845), via * 0.92);
    let mist = value_noise(p - cam * 0.3 + vec2<f32>(globals.time * 7.0, 0.0), 340.0, 30u);
    col = mix(col, vec3<f32>(0.975, 0.965, 0.945), smoothstep(0.45, 0.95, mist) * 0.55);
    // Construction lines.
    let ld = sky_line_distance(p - cam * 0.30, globals.time * 0.04);
    col = mix(col, vec3<f32>(0.800, 0.690, 0.420), smoothstep(1.3, 0.3, ld) * 0.75);
    // Light from the upper left, in soft shafts.
    let shaft = sin((p.x + p.y * 0.7 - cam.x * 0.2) * 0.0065 + globals.time * 0.05);
    col = col + smoothstep(0.55, 1.0, shaft) * vec3<f32>(0.030, 0.026, 0.012);
    // Gold dust in the light.
    let dq = p - cam * 0.1 + vec2<f32>(globals.time * 5.0, -globals.time * 8.0);
    let dc = floor(dq / 52.0);
    if rand_cell(dc, 94u) < 0.10 {
        let o = (dq / 52.0 - dc) - vec2<f32>(0.2 + 0.6 * rand_cell(dc, 95u), 0.2 + 0.6 * rand_cell(dc, 96u));
        let twinkle = 0.5 + 0.5 * sin(globals.time * 2.0 + rand_cell(dc, 97u) * 6.283);
        col = mix(col, vec3<f32>(0.930, 0.800, 0.470), smoothstep(0.035, 0.012, length(o)) * twinkle);
    }
    return col;
}

fn backdrop_corrupt(p: vec2<f32>) -> vec3<f32> {
    let cam = camera_engine();
    let t = globals.time;
    let g = clamp(p.y / room.y, 0.0, 1.0);
    var col = mix(vec3<f32>(0.150, 0.085, 0.275), vec3<f32>(0.400, 0.185, 0.530), g);
    let haze = value_noise(p - cam * 0.3 + vec2<f32>(t * 9.0, 0.0), 300.0, 71u);
    col = col + (haze - 0.4) * vec3<f32>(0.090, 0.025, 0.110);
    // The same city and towers, rebuilt in blocks. Some blocks are missing.
    let qc = p - cam * 0.78 + vec2<f32>(31.0, -140.0);
    let cc = floor(qc / 16.0);
    if towers((cc + vec2<f32>(0.5)) * 16.0, 88.0, 28u, room.y) > 0.5 && rand_cell(cc, 530u) > 0.10 {
        col = col * (0.88 + 0.06 * rand_cell(cc, 531u));
    }
    let qa = p - cam * 0.62;
    let ca = floor(qa / 24.0);
    let far_t = towers((ca + vec2<f32>(0.5)) * 24.0, 170.0, 20u, room.y);
    if far_t > 0.5 && rand_cell(ca, 500u) > 0.13 {
        col = col * (0.78 + 0.08 * rand_cell(ca, 501u));
        let ua = qa / 24.0 - ca;
        if ua.y < 0.14 { col = col * 1.18; }
    }
    let qb = p - cam * 0.42 + vec2<f32>(97.0, 60.0);
    let cb = floor(qb / 32.0);
    let near_t = towers((cb + vec2<f32>(0.5)) * 32.0, 370.0, 24u, room.y);
    if near_t > 0.5 && rand_cell(cb, 510u) > 0.16 {
        let r = rand_cell(cb, 511u);
        col = col * (0.66 + 0.10 * r);
        let u = qb / 32.0 - cb;
        if u.y < 0.16 {
            col = col * 1.18 + vec3<f32>(0.015, 0.010, 0.030);
        }
        if (u.x > 0.96 || u.y > 0.96) && r > 0.8 {
            col = mix(col, MAGENTA, 0.35);
        }
    }
    // The islands, in blocks, with light below them.
    let qi = p - cam * 0.36 + vec2<f32>(140.0, 40.0);
    let ci = floor(qi / 12.0);
    let isle = island((ci + vec2<f32>(0.5)) * 12.0);
    if isle.x > 0.5 && rand_cell(ci, 540u) > 0.10 {
        let r = rand_cell(ci, 541u);
        col = col * (0.40 + 0.14 * r);
        let u = qi / 12.0 - ci;
        if isle.y < 12.0 && u.y < 0.3 { col = col * 1.7 + vec3<f32>(0.030, 0.025, 0.060); }
        if r > 0.93 { col = mix(col, select(CYAN, MAGENTA, isle.z > 0.5), 0.55); }
    } else {
        let above = island(qi - vec2<f32>(0.0, 16.0));
        if above.x > 0.5 {
            col = col + MAGENTA * 0.20 * (0.7 + 0.3 * sin(t * 1.5 + above.z * 6.283));
        }
    }
    // The viaduct, in blocks.
    let qv = p - cam * 0.28;
    let cv = floor(qv / 32.0);
    if arcade((cv + vec2<f32>(0.5)) * 32.0) > 0.5 && rand_cell(cv, 520u) > 0.12 {
        let r = rand_cell(cv, 521u);
        col = col * (0.52 + 0.10 * r);
        let u = qv / 32.0 - cv;
        if u.y < 0.15 { col = col * 1.22 + vec3<f32>(0.015, 0.010, 0.030); }
        if u.x > 0.965 || u.y > 0.965 {
            col = mix(col * 0.5, select(CYAN, MAGENTA, r > 0.5), select(0.0, 0.45, r > 0.90 || r < 0.06));
        }
    }
    // The lattice the world is written on.
    let lat = p % 64.0;
    if lat.x < 1.0 || lat.y < 1.0 {
        col = col + vec3<f32>(0.010, 0.030, 0.045);
    }
    // The construction lines are a glyph now, and it is on.
    let ld = sky_line_distance(p - cam * 0.30, t * 0.22);
    let pulse = 0.72 + 0.28 * sin(t * 1.7 + p.x * 0.004);
    let glyph = mix(MAGENTA, CYAN, 0.28);
    col = col + glyph * (smoothstep(1.0, 0.2, ld) * 0.50 + exp(-ld / 7.0) * 0.10) * pulse;
    // Light leaks: thin vertical beams, with data that falls down them.
    let bq = p - cam * 0.2;
    let bi = floor(bq.x / 300.0);
    if rand_cell(vec2<f32>(bi, 3.0), 80u) < 0.62 {
        let bx = (bi + 0.2 + 0.6 * rand_cell(vec2<f32>(bi, 4.0), 81u)) * 300.0;
        let dx = abs(bq.x - bx);
        let flow = 0.6 + 0.4 * sin(bq.y * 0.045 - t * 3.2 + bi * 1.7);
        let beam = exp(-dx / 2.2) * 0.85 + exp(-dx / 34.0) * 0.20;
        col = col + mix(MAGENTA, CYAN, rand_cell(vec2<f32>(bi, 5.0), 82u) * 0.5) * beam * flow;
    }
    // Motes that rise.
    let mq = p + vec2<f32>(0.0, t * 22.0);
    let mc = floor(mq / 44.0);
    if rand_cell(mc, 90u) < 0.05 {
        let mo = (mq / 44.0 - mc) - vec2<f32>(0.2 + 0.6 * rand_cell(mc, 91u), 0.5);
        if max(abs(mo.x), abs(mo.y)) < 0.045 {
            col = mix(MAGENTA, CYAN, rand_cell(mc, 92u));
        }
    }
    return col;
}

/// How far the open air at `p` is into the corrupted state, 0..1. Built
/// things break at a hard edge; air changes as haze.
fn air_state(p: vec2<f32>) -> f32 {
    // The haze has wisps, and they drift.
    let t = globals.time;
    let wisp = (value_noise(p + vec2<f32>(t * 13.0, -t * 5.0), 130.0, 13u) - 0.5) * 110.0;
    return smoothstep(-260.0, 110.0, field(p) + wisp);
}

fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let a = air_state(p);
    if a <= 0.001 { return backdrop_clean(p); }
    if a >= 0.999 { return backdrop_corrupt(p); }
    // Through a lilac mist, not through grey.
    let mist = vec3<f32>(0.800, 0.640, 0.860);
    let clean = mix(backdrop_clean(p), mist, smoothstep(0.0, 0.55, a) * 0.85);
    return mix(clean, backdrop_corrupt(p), smoothstep(0.35, 1.0, a));
}

// --------------------------------------------------------------- surface --

/// Distance to the lines of a gold inlay sigil, for large wall faces.
fn sigil_distance(l: vec2<f32>, s: vec2<f32>) -> f32 {
    if s.x < 150.0 || s.y < 150.0 {
        return 1.0e4;
    }
    let period = 320.0;
    // Centre the lattice on the piece.
    let o = l - s * 0.5 + vec2<f32>(period * 0.5);
    let cell = floor(o / period);
    let centre = (cell + vec2<f32>(0.5)) * period + s * 0.5 - vec2<f32>(period * 0.5);
    // Keep the whole sigil on the face.
    let radius = 34.0;
    if centre.x < radius * 1.8 || centre.y < radius * 1.8 + 16.0
        || centre.x > s.x - radius * 1.8 || centre.y > s.y - radius * 1.8 {
        return 1.0e4;
    }
    let d = l - centre;
    let len = length(d);
    var dist = abs(len - radius);
    dist = min(dist, abs(len - radius * 0.62));
    dist = min(dist, abs(abs(d.x) + abs(d.y) - radius * 0.62) * 0.7071);
    if abs(d.y) < radius * 1.5 { dist = min(dist, abs(d.x)); }
    dist = min(dist, abs(len - radius * 0.15));
    return dist;
}

struct Stone {
    col: vec3<f32>,
    // Distance to the nearest gold ornament line. The corrupted state lights
    // the same lines.
    ornament: f32,
}

/// The clean architecture of one block. `l` is the position in the block, `s`
/// is the block size, `p` is the world position (the masonry is laid in world
/// space, so two blocks that touch share their courses).
fn clean_surface(l: vec2<f32>, s: vec2<f32>, p: vec2<f32>) -> Stone {
    var col = STONE;
    var ornament = 1.0e4;
    let pillar = s.x <= 48.0 && s.y > 96.0;
    let slab = s.y <= 40.0 && !pillar;

    if pillar {
        // A fluted shaft with gold bands.
        let flute = l.x % 8.0;
        col = STONE * (1.0 - 0.07 * smoothstep(0.0, 3.0, abs(flute - 4.0)) + 0.035);
        if flute < 1.0 { col = MORTAR; }
        let band = (l.y + 60.0) % 224.0;
        if band < 22.0 {
            col = select(STONE_LIT, STONE_SHADE, band > 15.0);
            ornament = min(abs(band - 3.0), abs(band - 13.0));
        }
    } else if slab {
        // A platform: one dressed stone with a groove.
        col = STONE * (1.0 - 0.05 * (l.y / s.y));
        let joint = (p.x + 24.0) % 96.0;
        if joint < 1.0 && l.y > 10.0 { col = MORTAR; }
        // An engraved line with a gold stud in each stone.
        let mid = (11.0 + s.y - 7.0) * 0.5;
        if abs(l.y - mid) < 0.6 { col = col * 0.90; }
        let sx = (p.x + 24.0) % 96.0 - 48.0;
        let stud = abs(sx) + abs(l.y - mid) - 3.2;
        ornament = min(ornament, max(stud, 0.0) + 0.45);
    } else {
        // Coursed masonry, half-bond.
        let course = 16.0;
        let brick = 32.0;
        let row = floor(p.y / course);
        let shift = (row - 2.0 * floor(row * 0.5)) * brick * 0.5;
        let bx = floor((p.x + shift) / brick);
        let fy = p.y - row * course;
        let fx = p.x + shift - bx * brick;
        let tone = 1.0 + (rand_cell(vec2<f32>(bx, row), 12u) - 0.5) * 0.06;
        col = STONE * tone * (1.0 - 0.07 * clamp(l.y / 420.0, 0.0, 1.0));
        if fy < 1.0 || fx < 1.0 {
            col = mix(col, MORTAR, 0.75);
        } else if fy < 2.0 {
            col = col * 1.03;
        }
        ornament = min(ornament, sigil_distance(l, s));
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
            col = select(STONE_SHADE * 0.94, STONE_LIT, (l.x % 12.0) < 6.0);
        }
        ornament = min(ornament, abs(l.y - 9.0) + 0.4);
        if slab {
            // The underside steps in.
            if l.y > s.y - 5.0 { col = STONE_SHADE * 0.93; }
            ornament = min(ornament, abs(l.y - (s.y - 7.0)) + 0.6);
        }
    }
    // Light from the upper left.
    if l.x < 2.0 { col = mix(col, STONE_LIT, 0.8); }
    if l.x > s.x - 3.0 { col = mix(col, STONE_SHADE, 0.8); }
    if l.y > s.y - 2.0 { col = mix(col, STONE_SHADE, 0.8); }

    // Gold: a bright core and a darker edge.
    let gold = smoothstep(1.5, 0.5, ornament);
    let glint = 0.5 + 0.5 * sin((p.x + p.y) * 0.06);
    col = mix(col, mix(GOLD, GOLD_LIT, glint * glint), gold);
    return Stone(col, ornament);
}

/// Whether the voxel `cell` of the corrupted block is solid. The top row is
/// always solid, so the standing surface stays where collision is.
fn voxel_solid(cell: vec2<f32>, min: vec2<f32>, s: vec2<f32>) -> bool {
    let c = (cell + vec2<f32>(0.5)) * VOXEL - min;
    let inside = c.x >= 0.0 && c.y >= 0.0 && c.x < s.x && c.y < s.y;
    let r = rand_cell(cell, 600u);
    if inside {
        let top_row = c.y < VOXEL;
        let edge = c.x < VOXEL || c.x > s.x - VOXEL || c.y > s.y - VOXEL;
        return top_row || !edge || r > 0.20;
    }
    // A cell that grows past a side or the underside. Never above the top.
    let near_t = c.x > -VOXEL && c.x < s.x + VOXEL && c.y > 0.0 && c.y < s.y + VOXEL;
    return near_t && r < 0.07;
}

fn corrupt_surface(l: vec2<f32>, s: vec2<f32>, p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    let cell = floor(p / VOXEL);
    if !voxel_solid(cell, piece.xy, s) {
        let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < s.x && l.y < s.y;
        if inside {
            // A missing voxel: the sky shows through the block sprite.
            return vec4<f32>(backdrop(p), 1.0);
        }
        return vec4<f32>(0.0);
    }
    let u = p - cell * VOXEL;
    let r = rand_cell(cell, 601u);
    let r2 = rand_cell(cell, 602u);
    // The block keeps the shading of the stone it was.
    let stone = clean_surface(clamp(l, vec2<f32>(0.0), s - vec2<f32>(0.01)), s, p);
    var v = clamp((luma(stone.col) - 0.55) / 0.40, 0.0, 1.0);
    v = 0.22 + 0.42 * v + 0.36 * r;
    var ink = mix(INK_LO, INK_HI, v) + (r2 - 0.5) * vec3<f32>(0.030, -0.010, -0.030);

    let above = voxel_solid(cell + vec2<f32>(0.0, -1.0), piece.xy, s);
    let below = voxel_solid(cell + vec2<f32>(0.0, 1.0), piece.xy, s);
    let left = voxel_solid(cell + vec2<f32>(-1.0, 0.0), piece.xy, s);
    let right = voxel_solid(cell + vec2<f32>(1.0, 0.0), piece.xy, s);
    // Voxel faces: a lit top where nothing is above, a lit left side.
    if !above && u.y < 5.0 {
        ink = ink * 1.25 + vec3<f32>(0.150, 0.130, 0.280);
        // The walking surface of the block: one bright line, always.
        if l.y < 2.0 && l.y >= 0.0 {
            ink = vec3<f32>(0.780, 0.720, 1.000);
        }
    }
    if !left && u.x < 2.0 {
        ink = ink * 1.2 + vec3<f32>(0.030);
    }
    // Seams. Some voxels are 2x2, with no seam inside.
    let big = rand_cell(floor(cell * 0.5), 610u) < 0.4;
    let u2 = p - floor(p / (VOXEL * 2.0)) * VOXEL * 2.0;
    let seam = (u.x >= VOXEL - 1.0 || u.y >= VOXEL - 1.0)
        && !(big && u2.x < VOXEL * 2.0 - 1.0 && u2.y < VOXEL * 2.0 - 1.0);
    if seam {
        ink = ink * 0.5;
    }

    var neon = vec3<f32>(0.0);
    let pulse = 0.72 + 0.28 * sin(t * 2.2 + p.x * 0.021 + p.y * 0.013);
    // Cracks run along seams and light the stone beside them.
    let crack_h = below && rand_cell(vec2<f32>(floor(p.x / 80.0), cell.y), 620u) < 0.10;
    let crack_v = right && rand_cell(vec2<f32>(cell.x, floor(p.y / 64.0)), 621u) < 0.08;
    if crack_h {
        let d = VOXEL - 0.5 - u.y;
        neon = max(neon, MAGENTA * (smoothstep(1.2, 0.2, abs(d)) + exp(-abs(d) / 3.0) * 0.30) * pulse);
    }
    if crack_v {
        let d = VOXEL - 0.5 - u.x;
        neon = max(neon, MAGENTA * (smoothstep(1.2, 0.2, abs(d)) + exp(-abs(d) / 3.0) * 0.30) * pulse);
    }
    // Exposed edges: cyan describes them in some regions, magenta in a few.
    let zone = rand_cell(floor(p / 112.0), 630u);
    var edge = 1.0e4;
    if !above { edge = min(edge, u.y); }
    if !left { edge = min(edge, u.x); }
    if !right { edge = min(edge, VOXEL - 1.0 - u.x); }
    if !below { edge = min(edge, VOXEL - 1.0 - u.y); }
    let edge_light = smoothstep(1.1, 0.3, edge) + exp(-edge / 3.0) * 0.14;
    // Not the top: the walking line owns it.
    let side = edge < u.y || above;
    if zone < 0.26 && side {
        neon = max(neon, CYAN * edge_light * 0.85);
    } else if zone > 0.92 && side {
        neon = max(neon, MAGENTA * edge_light * 0.85);
    }
    // The gold ornament is the same drawing, now a live circuit.
    let travel = smoothstep(0.80, 1.0, sin((p.x + p.y) * 0.09 - t * 5.0));
    let circuit = mix(MAGENTA, CYAN, step(0.5, rand_cell(floor(p / 224.0), 640u)));
    let lit = smoothstep(1.3, 0.4, stone.ornament) + exp(-stone.ornament / 4.0) * 0.18;
    neon = max(neon, (circuit + vec3<f32>(travel * 0.6)) * lit * (0.8 + 0.2 * pulse));

    // The rewrite goes on: a slow wave of light leaves the front and goes
    // into the mass, one block at a time.
    let reach = field((cell + vec2<f32>(0.5)) * VOXEL);
    let wave = fract(reach / 620.0 - t * 0.085);
    neon = neon + mix(CYAN, MAGENTA, 0.35) * smoothstep(0.045, 0.0, wave) * 0.16;
    // Scanlines.
    ink = ink * (0.93 + 0.07 * (floor(p.y) % 2.0));
    return vec4<f32>(ink + neon, 1.0);
}

// ------------------------------------------------------------------- ivy --

const LEAF_DARK: vec3<f32> = vec3<f32>(0.205, 0.345, 0.225);
const LEAF_MID: vec3<f32> = vec3<f32>(0.305, 0.470, 0.270);
const LEAF_LIT: vec3<f32> = vec3<f32>(0.470, 0.625, 0.340);

fn leaf_colour(leaf: i32) -> vec3<f32> {
    if leaf == 1 { return LEAF_DARK; }
    if leaf == 2 { return LEAF_MID; }
    return LEAF_LIT;
}

/// What ivy is at `q`, which is from a top corner of a block: x goes into
/// the block from its end, y goes down. 0 = none, 1..3 = a leaf, dark to lit.
/// The ivy is thick on the corner and thin away from it.
fn ivy_clump(q: vec2<f32>, key: vec2<f32>, salt: u32) -> i32 {
    if q.x < -11.0 || q.x > 52.0 || q.y < -8.0 || q.y > 44.0 {
        return 0;
    }
    let from_corner = length(vec2<f32>(q.x - 6.0, (q.y - 5.0) * 0.75));
    // Thin to nothing at the edge of the quad, so no cut shows.
    let fringe = smoothstep(-8.0, -1.0, q.y) * smoothstep(-11.0, -4.0, q.x);
    let density = exp(-max(from_corner - 9.0, 0.0) / 12.0) * 0.95 * fringe;
    let cell = floor(q / 6.0);
    let id = cell + key;
    if rand_cell(id, salt) > density {
        return 0;
    }
    let jitter = vec2<f32>(rand_cell(id, salt + 1u), rand_cell(id, salt + 2u)) - vec2<f32>(0.5);
    let o = q - (cell + vec2<f32>(0.5)) * 6.0 - jitter * 2.4;
    let angle = rand_cell(id, salt + 3u) * 3.1416;
    let cs = vec2<f32>(cos(angle), sin(angle));
    let e = vec2<f32>(o.x * cs.x + o.y * cs.y, o.y * cs.x - o.x * cs.y) / vec2<f32>(3.9, 2.4);
    if dot(e, e) > 1.0 {
        return 0;
    }
    return 1 + i32(floor(rand_cell(id, salt + 4u) * 2.99));
}

/// The key of the block this quad draws, the same for its surface quad and
/// its underside quad: the left edge and the bottom edge of the block.
fn ivy_key(bottom: f32) -> vec2<f32> {
    return vec2<f32>(piece.x, bottom);
}

/// Whether an end of the block has ivy. `end` is 0.0 for the left end.
fn has_ivy(key: vec2<f32>, end: f32) -> bool {
    return rand_cell(key + vec2<f32>(end * 13.0, 0.0), 700u) < 0.55;
}

/// The ivy on the corners of the block of this surface quad.
fn ivy_on_surface(l: vec2<f32>, s: vec2<f32>) -> i32 {
    if s.x < 96.0 || l.y > 44.0 || (l.x > 52.0 && l.x < s.x - 52.0) {
        return 0;
    }
    let key = ivy_key(piece.y + s.y);
    if has_ivy(key, 0.0) {
        let leaf = ivy_clump(l, key, 710u);
        if leaf != 0 { return leaf; }
    }
    if has_ivy(key, 1.0) {
        return ivy_clump(vec2<f32>(s.x - l.x, l.y), key + vec2<f32>(7.0, 3.0), 720u);
    }
    return 0;
}

/// The ivy that hangs below the ends of a platform: three strands at each end
/// that has ivy, and they move in the wind. `l` is from the lower left corner
/// of the platform, `w` is its width.
fn ivy_strands(l: vec2<f32>, w: f32) -> i32 {
    if l.y < 0.0 || l.y > 120.0 || (l.x > 40.0 && l.x < w - 40.0) {
        return 0;
    }
    let t = globals.time;
    let key = ivy_key(piece.y);
    for (var end = 0; end < 2; end++) {
        if !has_ivy(key, f32(end)) {
            continue;
        }
        let x = select(l.x, w - l.x, end == 1);
        for (var k = 0; k < 3; k++) {
            let id = key + vec2<f32>(f32(k) * 5.0 + f32(end) * 31.0, 0.0);
            let len = 34.0 + 84.0 * rand_cell(id, 730u);
            if l.y > len {
                continue;
            }
            let wind = sin(t * 1.2 + key.x * 0.07 + l.y * 0.045 + f32(k)) * 2.2 * (l.y / len);
            let sx = 4.0 + f32(k) * 9.0 + (rand_cell(id, 731u) - 0.5) * 5.0 + wind;
            let dx = x - sx;
            if abs(dx) < 0.7 {
                return 1;
            }
            // A leaf on each side in turn, every 7 px, with some left out.
            let row = floor(l.y / 7.0);
            if rand_cell(id + vec2<f32>(0.0, row), 732u) < 0.78 {
                let side = select(-1.0, 1.0, (i32(row) + k) % 2 == 0);
                let o = vec2<f32>(dx - side * 3.4, l.y - (row + 0.5) * 7.0);
                let e = vec2<f32>(o.x + o.y * 0.35 * side, o.y) / vec2<f32>(3.6, 2.2);
                if dot(e, e) < 1.0 {
                    return 2 + i32(rand_cell(id + vec2<f32>(1.0, row), 733u) * 1.99);
                }
            }
        }
    }
    return 0;
}

/// A leaf after the rewrite: a dead block, and a few of them are lit.
fn dead_leaf(p: vec2<f32>) -> vec3<f32> {
    let r = rand_cell(floor(p / 4.0), 740u);
    if r > 0.92 {
        return CYAN * 0.9;
    }
    return vec3<f32>(0.020, 0.070, 0.060) * (0.6 + r);
}

/// A blink wall: a veil, not stone. A blink goes through it, so it is glass
/// in a gold lattice on the clean side and a curtain of blocks that flicker
/// on the corrupted side. The wall's own sprite shows through it a little.
/// A hard wall has a closer lattice and less light.
fn veil(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    let l = p - piece.xy;
    let s = piece.zw;
    if l.x < 0.0 || l.y < 0.0 || l.x >= s.x || l.y >= s.y {
        return vec4<f32>(0.0);
    }
    let hard = room.w > 2.5;
    let pitch = select(14.0, 9.0, hard);
    let a = abs(fract((p.x + p.y) / pitch) - 0.5) * pitch;
    let b = abs(fract((p.x - p.y) / pitch) - 0.5) * pitch;
    let lattice = smoothstep(1.3, 0.4, min(a, b));
    let rim = min(min(l.x, l.y), min(s.x - l.x, s.y - l.y));
    let frame = smoothstep(2.4, 1.2, rim);
    if claim(p).state == 1 {
        let cell = floor(p / 8.0);
        let beat = floor(t * 2.5 + rand_cell(cell, 750u) * 2.5);
        let flick = rand_cell(cell + vec2<f32>(beat * 3.0, 0.0), 751u);
        var col = vec3<f32>(0.150, 0.060, 0.260) * (0.7 + 0.6 * rand_cell(cell, 752u));
        if flick > select(0.86, 0.93, hard) {
            col = mix(col, MAGENTA, 0.55);
        }
        col = mix(col, MAGENTA * 0.85, lattice * 0.55);
        col = mix(col, CYAN, frame);
        return vec4<f32>(col, select(0.80, 0.90, hard));
    }
    var col = select(vec3<f32>(0.760, 0.710, 0.940), vec3<f32>(0.600, 0.530, 0.860), hard);
    // Light goes across the glass.
    let shimmer = smoothstep(0.75, 1.0, sin((p.x + p.y) * 0.035 - t * 1.4));
    col = col + shimmer * vec3<f32>(0.090, 0.090, 0.050);
    col = mix(col, mix(GOLD, GOLD_LIT, 0.3), max(lattice * 0.85, frame));
    return vec4<f32>(col, select(0.78, 0.88, hard));
}

fn surface(p: vec2<f32>) -> vec4<f32> {
    if room.w > 1.5 {
        return veil(p);
    }
    let l = p - piece.xy;
    let s = piece.zw;
    let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < s.x && l.y < s.y;
    let c = claim(p);
    if c.state == 1 {
        // The ivy in blocks of 4 px.
        let lq = (floor(p / 4.0) + vec2<f32>(0.5)) * 4.0 - piece.xy;
        if ivy_on_surface(lq, s) != 0 {
            return vec4<f32>(dead_leaf(p), 1.0);
        }
        var col = corrupt_surface(l, s, p);
        // The blocks that changed last are hot still: the light of the front
        // is in them, and it fades into the mass.
        if col.a > 0.0 && inside {
            let heat = exp(-max(field(p), 0.0) / 46.0);
            let cell_heat = heat * (0.45 + 0.55 * rand_cell(floor(p / VOXEL), 603u));
            col = vec4<f32>(col.rgb + MAGENTA * cell_heat * 0.42 + vec3<f32>(cell_heat * cell_heat * 0.20), 1.0);
        }
        // The mass burns where it meets clean stone.
        if col.a > 0.0 && inside && claim(p - front.zw * 3.0).state != 1 {
            col = vec4<f32>(MAGENTA * 0.95 + vec3<f32>(0.25), 1.0);
        }
        return col;
    }
    let leaf = ivy_on_surface(l, s);
    if leaf != 0 {
        return vec4<f32>(leaf_colour(leaf), 1.0);
    }
    if !inside {
        return vec4<f32>(0.0);
    }
    var col = clean_surface(l, s, p).col;
    // Clean stone near the front already carries the stain.
    let f = field(p);
    let wash = exp(min(f, 0.0) / 80.0);
    col = col + wash * vec3<f32>(0.090, -0.050, 0.050);
    let crack = rand_cell(floor(p / vec2<f32>(48.0, 16.0)), 650u);
    let fy = p.y % 16.0;
    if wash > 0.25 && crack < wash * 0.5 && fy < 1.2 {
        col = mix(col, MAGENTA, 0.75);
    }
    return vec4<f32>(col, 1.0);
}

// --------------------------------------------------------------- overlay --

/// Loose blocks that float off the mass, and short tears beside the front.
fn overlay(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    let c = claim(p);
    if c.state == 2 {
        // A small cube that bobs in its cell.
        let bob = sin(t * 1.3 + c.r * 6.283) * 0.07;
        let u = p / c.size - c.cell - vec2<f32>(0.5, 0.5 + bob);
        let half = 0.42;
        let m = max(abs(u.x), abs(u.y));
        if m < half {
            var col = vec3<f32>(0.045, 0.038, 0.105) * (0.7 + c.r);
            if u.y < -half * 0.52 {
                col = col * 2.2 + vec3<f32>(0.070, 0.060, 0.120);
            } else if u.x > half * 0.55 {
                col = col * 0.55;
            }
            let pick = fract(c.r2 * 31.0);
            let tone = select(CYAN, MAGENTA, c.r > 0.5);
            if pick < 0.34 && m > half - 1.3 / c.size {
                col = tone;
            }
            if fract(c.r2 * 57.0) < 0.08 && c.size <= 16.0 {
                col = tone;
            }
            return vec4<f32>(col, 1.0);
        }
        // A soft light around the lit ones.
        if fract(c.r2 * 31.0) < 0.34 {
            let glow = exp(-(m - half) * c.size / 5.0) * 0.30;
            return vec4<f32>(select(CYAN, MAGENTA, c.r > 0.5), glow);
        }
        return vec4<f32>(0.0);
    }
    // Tears: short runs of light, for a few frames, close to the front. Few
    // of them: motion at the edge of sight takes the eye off the play.
    let near_t = exp(-abs(field(p)) / 90.0);
    let band = floor(p.y / 2.0);
    let seg = floor(p.x / 56.0 + rand_cell(vec2<f32>(band, 0.0), 33u) * 3.0);
    let tick = floor(t * 6.0);
    let roll = rand_cell(vec2<f32>(seg + tick * 13.0, band), 31u);
    if roll > 1.0 - 0.010 * near_t * near_t {
        let tone = select(CYAN, MAGENTA, rand_cell(vec2<f32>(seg, band), 37u) > 0.5);
        return vec4<f32>(tone, 0.85);
    }
    return vec4<f32>(0.0);
}

// ------------------------------------------------------------- underside --

/// What the ornament below a platform is made of at `l` (from the platform's
/// lower left corner, y down). `w` is the platform width.
/// 0 = air, 1 = stone, 2 = a gold line, 3 = banner cloth, 4 = banner gold.
fn underside_part(l: vec2<f32>, w: f32) -> i32 {
    if l.x < 0.0 || l.x >= w || l.y < 0.0 {
        return 0;
    }
    let bays = max(1.0, round(w / 150.0));
    let bay = w / bays;
    let i = floor(l.x / bay);
    let x = l.x - (i + 0.5) * bay;
    let half = bay * 0.5;
    // The soffit.
    if l.y < 6.0 {
        return 1;
    }
    // A stepped bracket at each end of the bay.
    let from_end = half - abs(x);
    let step = floor(l.y / 8.0);
    if from_end < 17.0 - step * 3.0 && l.y < 48.0 {
        return 1;
    }
    // A shallow arch between the brackets: thin at the crown.
    let span = half - 15.0;
    let rise = min(span * 0.55, 38.0);
    if abs(x) < span {
        let k = abs(x) / span;
        let curve = 6.0 + rise * (1.0 - sqrt(max(0.0, 1.0 - k * k)));
        if l.y < curve {
            return select(1, 2, l.y > curve - 1.8);
        }
    }
    // A banner hangs in some bays.
    if bay >= 110.0 && rand_cell(vec2<f32>(i + piece.x, piece.y), 660u) < 0.55 {
        let ax = abs(x);
        let hem = 66.0 + ax * 0.9;
        if ax < 14.0 && l.y < hem {
            let emblem = abs(ax + abs(l.y - 36.0) - 6.5);
            if ax > 11.8 || l.y > hem - 2.2 || emblem < 1.0 {
                return 4;
            }
            return 3;
        }
    }
    return 0;
}

fn underside(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    let l = p - piece.xy;
    let w = piece.z;
    let corrupted = claim(p).state == 1;
    if corrupted {
        let lq = (floor(p / 4.0) + vec2<f32>(0.5)) * 4.0 - piece.xy;
        if ivy_strands(lq, w) != 0 {
            return vec4<f32>(dead_leaf(p), 1.0);
        }
    } else {
        let leaf = ivy_strands(l, w);
        if leaf != 0 {
            return vec4<f32>(leaf_colour(leaf), 1.0);
        }
    }
    if corrupted {
        // The same ornament, in blocks half the size of the architecture's.
        let vox = VOXEL * 0.5;
        let cell = floor(p / vox);
        let part = underside_part((cell + vec2<f32>(0.5)) * vox - piece.xy, w);
        let r = rand_cell(cell, 670u);
        if part != 0 && r > 0.14 {
            let u = p - cell * vox;
            var col = mix(INK_LO, INK_HI, 0.15 + 0.5 * r);
            if part == 3 {
                col = vec3<f32>(0.115, 0.030, 0.120) * (0.7 + 0.6 * r);
            }
            if u.x >= vox - 1.0 || u.y >= vox - 1.0 {
                col = col * 0.55;
            }
            // The mass is in blocks. The gold lines stay thin, and they are lit.
            let exact = underside_part(l, w);
            if exact == 2 || exact == 4 {
                let tone = select(CYAN, MAGENTA, exact == 4);
                col = mix(col, tone, 0.85 * (0.75 + 0.25 * sin(t * 2.0 + p.x * 0.05)));
            }
            return vec4<f32>(col, 1.0);
        }
    } else {
        let part = underside_part(l, w);
        if part == 1 {
            return vec4<f32>(mix(STONE, STONE_SHADE, 0.62) * (1.0 - 0.10 * clamp(l.y / 46.0, 0.0, 1.0)), 1.0);
        }
        if part == 2 || part == 4 {
            return vec4<f32>(mix(GOLD, GOLD_LIT, 0.35), 1.0);
        }
        if part == 3 {
            let fold = 0.92 + 0.08 * sin(l.x * 0.9);
            return vec4<f32>(vec3<f32>(0.300, 0.395, 0.585) * fold, 1.0);
        }
    }
    // What falls from a platform. Water on the clean side, light on the
    // corrupted side: the same streams, at the same places.
    if l.x < 0.0 || l.x >= w || l.y < 0.0 {
        return vec4<f32>(0.0);
    }
    let col_i = floor(l.x / 40.0);
    let key = vec2<f32>(col_i + piece.x, piece.y);
    if rand_cell(key, 680u) > 0.42 {
        return vec4<f32>(0.0);
    }
    let r = rand_cell(key, 681u);
    let cx = (col_i + 0.3 + 0.4 * r) * 40.0;
    let len = 70.0 + (piece.w - 80.0) * rand_cell(key, 682u);
    if l.y >= len {
        return vec4<f32>(0.0);
    }
    let dx = abs(l.x - cx);
    let fall = pow(1.0 - l.y / len, 1.6);
    let air = air_state(p);
    // Light.
    let flow = 0.62 + 0.38 * sin(l.y * 0.12 - t * 6.0 + r * 6.283);
    let light_a = fall * flow * (smoothstep(1.4, 0.3, dx) + exp(-dx / 5.0) * 0.32) * air;
    let light = select(MAGENTA, CYAN, r > 0.82) + vec3<f32>(0.25 * fall);
    // Water: a thin fall that widens, with streaks that run down it, and
    // mist where it ends. Half of the streams only.
    var water_a = 0.0;
    if r < 0.5 {
        let width = 1.6 + 3.2 * (l.y / len);
        let streak = 0.55 + 0.45 * value_noise(vec2<f32>(l.x * 3.0, l.y - t * 150.0), 9.0, 683u);
        let body = smoothstep(width, width - 1.4, dx) * streak * (0.35 + 0.50 * fall);
        let mist = exp(-dx / 16.0) * smoothstep(len * 0.55, len, l.y) * 0.30;
        water_a = max(body, mist) * (1.0 - air);
    }
    let water = vec3<f32>(0.930, 0.965, 1.000);
    let a = max(light_a, water_a);
    if a <= 0.001 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(mix(water, light, light_a / a), clamp(a, 0.0, 1.0));
}

// ---------------------------------------------------------------- portal --

/// What the frame of a door is made of at `l` (from the upper left corner of
/// the door sprite, y down). `s` is the size of the door sprite.
/// 0 = air, 1 = stone, 2 = lit stone, 3 = a gold line, 4 = the recessed
/// field in the arch, 5 = shaded stone, 6 = the keystone.
fn portal_part(l: vec2<f32>, s: vec2<f32>) -> i32 {
    let cx = l.x - s.x * 0.5;
    let ax = abs(cx);
    let half = s.x * 0.5;
    let inner = half + 2.0;
    let outer = half + 15.0;
    if l.y > s.y {
        return 0;
    }
    if l.y >= 0.0 {
        // Two pilasters on a plinth. The door fills the opening.
        if ax < half - 1.0 {
            return 0;
        }
        if l.y > s.y - 9.0 && ax < outer + 4.0 {
            return select(5, 2, l.y < s.y - 7.0);
        }
        if ax >= inner && ax < outer {
            let across = (ax - inner) / (outer - inner);
            if abs(across - 0.5) < 0.06 { return 5; }
            return select(1, 2, (cx < 0.0) == (across > 0.78));
        }
        return 0;
    }
    // The impost: a band at the spring of the arch.
    if l.y >= -6.0 {
        if ax < outer + 3.0 {
            return select(2, 3, l.y > -2.4 && l.y < -0.8);
        }
        return 0;
    }
    // The arch.
    let up = -(l.y + 6.0);
    let d = length(vec2<f32>(cx, up));
    if ax < 5.5 && d >= inner && d < outer + 5.0 {
        return 6;
    }
    if d < inner {
        // A small gold figure in the field.
        let figure = abs(ax + abs(up - inner * 0.46) - 5.0);
        return select(4, 3, figure < 0.9);
    }
    if d < outer {
        return select(1, 3, d < inner + 1.7 || d > outer - 1.5);
    }
    return 0;
}

fn portal(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    // The door sprite stands on the floor of its trigger box, centred.
    let s = vec2<f32>(piece.w * room.w, piece.w);
    let origin = vec2<f32>(piece.x + (piece.z - s.x) * 0.5, piece.y);
    if claim(p).state == 1 {
        let vox = VOXEL * 0.5;
        let cell = floor(p / vox);
        let part = portal_part((cell + vec2<f32>(0.5)) * vox - origin, s);
        let r = rand_cell(cell, 690u);
        if part == 0 || r < 0.10 {
            return vec4<f32>(0.0);
        }
        let u = p - cell * vox;
        let pulse = 0.70 + 0.30 * sin(t * 2.4 + origin.x * 0.03);
        var col = mix(INK_LO, INK_HI, 0.20 + 0.55 * r);
        if part == 2 { col = col * 1.5 + vec3<f32>(0.030, 0.026, 0.060); }
        if part == 4 { col = INK_LO + MAGENTA * 0.13 * pulse; }
        if u.x >= vox - 1.0 || u.y >= vox - 1.0 { col = col * 0.55; }
        // The mass is in blocks. The gold lines stay thin, and they are lit.
        if portal_part(p - origin, s) == 3 { col = mix(col, CYAN, 0.90); }
        if part == 6 { col = MAGENTA * pulse + vec3<f32>(0.20); }
        return vec4<f32>(col, 1.0);
    }
    let part = portal_part(p - origin, s);
    if part == 0 {
        return vec4<f32>(0.0);
    }
    var col = mix(STONE, STONE_SHADE, 0.20);
    if part == 2 { col = STONE_LIT; }
    if part == 3 { col = mix(GOLD, GOLD_LIT, 0.30); }
    if part == 4 { col = mix(STONE, STONE_SHADE, 0.70); }
    if part == 5 { col = STONE_SHADE; }
    if part == 6 { col = GOLD_LIT; }
    return vec4<f32>(col, 1.0);
}

// ------------------------------------------------------------------ main --

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let w = mesh.world_position.xy;
    let p = vec2<f32>(w.x + room.x * 0.5, room.y * 0.5 - w.y);
    let role = room.z;
    var col: vec4<f32>;
    if role < 0.5 {
        col = vec4<f32>(backdrop(p), 1.0);
    } else if role < 1.5 {
        col = surface(p);
    } else if role < 2.5 {
        col = overlay(p);
    } else if role < 3.5 {
        col = underside(p);
    } else {
        col = portal(p);
    }
    // Display values to linear.
    return vec4<f32>(pow(max(col.rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), col.a);
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// (`CompositingSpace::Srgb`) the shaded colour is written sRGB-encoded, as
// Bevy's own sprite and mesh shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
