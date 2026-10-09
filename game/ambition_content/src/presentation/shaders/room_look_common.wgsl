#define_import_path ambition_content::room_look

// What the room looks share: hashes, noise, and the far architecture that
// each look draws in its own way. See `room_look.rs`.
//
// All positions are in engine world coordinates (y down).

fn hash_u32(value: u32) -> u32 {
    var h = value;
    h = h ^ (h >> 16u);
    h = h * 0x7feb352du;
    h = h ^ (h >> 15u);
    h = h * 0x846ca68bu;
    h = h ^ (h >> 16u);
    return h;
}

/// A value in [0, 1) for one integer cell.
fn rand_cell(cell: vec2<f32>, salt: u32) -> f32 {
    let x = bitcast<u32>(i32(floor(cell.x)));
    let y = bitcast<u32>(i32(floor(cell.y)));
    let n = (x * 1597334677u) ^ (y * 3812015801u) ^ (salt * 668265263u);
    return f32(hash_u32(n) >> 8u) * (1.0 / 16777216.0);
}

fn value_noise(p: vec2<f32>, scale: f32, salt: u32) -> f32 {
    let g = p / scale;
    let i = floor(g);
    var f = g - i;
    f = f * f * (3.0 - 2.0 * f);
    let a = rand_cell(i, salt);
    let b = rand_cell(i + vec2<f32>(1.0, 0.0), salt);
    let c = rand_cell(i + vec2<f32>(0.0, 1.0), salt);
    let d = rand_cell(i + vec2<f32>(1.0, 1.0), salt);
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

/// 1.0 inside the silhouette of a row of far towers. `room_h` is the room
/// height: the towers stand on the room floor.
fn towers(q: vec2<f32>, period: f32, salt: u32, room_h: f32) -> f32 {
    let i = floor(q.x / period);
    let r = rand_cell(vec2<f32>(i, 0.0), salt);
    let r2 = rand_cell(vec2<f32>(i, 1.0), salt);
    let half_w = period * (0.14 + 0.16 * r2);
    let cx = (i + 0.5) * period + (r - 0.5) * period * 0.25;
    let top = room_h * (0.10 + 0.72 * r);
    let dx = abs(q.x - cx);
    let spire_h = 50.0 + 90.0 * r2;
    let body = dx < half_w && q.y > top;
    let cap = dx < half_w + 7.0 && q.y > top - 12.0 && q.y <= top;
    let up = (q.y - (top - 12.0 - spire_h)) / spire_h;
    let spire = q.y <= top - 12.0 && up > 0.0 && dx < (half_w - 4.0) * up;
    // An arched window row, so the tower is not a plain bar.
    let wy = (q.y - top - 40.0) % 150.0;
    let window = body && dx < half_w * 0.34 && wy > 0.0 && wy < 46.0 && q.y > top + 40.0
        && (wy > half_w * 0.34 || length(vec2<f32>(dx, wy - half_w * 0.34)) < half_w * 0.34);
    return select(0.0, 1.0, (body || cap || spire) && !window);
}

/// 1.0 inside a viaduct: tiers of round arches on piers, every 620 px of
/// height.
fn arcade(q: vec2<f32>) -> f32 {
    let tier_h = 470.0;
    let tier = floor(q.y / tier_h);
    if rand_cell(vec2<f32>(tier, 7.0), 26u) < 0.18 {
        return 0.0;
    }
    let y = q.y - tier * tier_h - 120.0;
    if y < 0.0 || y > 250.0 {
        return 0.0;
    }
    // The deck, then arches below it.
    if y < 26.0 {
        return select(1.0, 0.0, y > 8.0 && y < 12.0);
    }
    let span = 190.0;
    let x = q.x - floor(q.x / span) * span - span * 0.5;
    let radius = span * 0.5 - 20.0;
    let spring = 26.0 + radius + 10.0;
    let open = (y > spring && abs(x) < radius) || length(vec2<f32>(x, y - spring)) < radius;
    return select(1.0, 0.0, open);
}

/// Distance to the nearest construction line of the sky: rings and axes on a
/// sparse lattice. The clean state draws them in gold, the corrupted state
/// lights them.
fn sky_line_distance(q: vec2<f32>, turn: f32) -> f32 {
    let period = 560.0;
    let cell = floor(q / period);
    let r = rand_cell(cell, 40u);
    if r > 0.48 {
        return 1.0e4;
    }
    let centre = (cell + vec2<f32>(0.3 + 0.4 * rand_cell(cell, 41u), 0.3 + 0.4 * rand_cell(cell, 42u))) * period;
    let d = q - centre;
    let big = 110.0 + 70.0 * rand_cell(cell, 43u);
    let len = length(d);
    var dist = abs(len - big);
    dist = min(dist, abs(len - big * 0.62));
    dist = min(dist, abs(len - big * 0.14));
    // A diamond inscribed in the inner ring.
    let cs = vec2<f32>(cos(turn), sin(turn));
    let dr = vec2<f32>(d.x * cs.x - d.y * cs.y, d.x * cs.y + d.y * cs.x);
    let dia = abs(abs(dr.x) + abs(dr.y) - big * 0.62) * 0.7071;
    dist = min(dist, dia);
    // The axes, a little longer than the outer ring.
    if abs(d.y) < big * 1.45 { dist = min(dist, abs(d.x)); }
    if abs(d.x) < big * 1.45 { dist = min(dist, abs(d.y)); }
    return dist;
}

/// A floating island on a sparse lattice: a flat top and steps that go in
/// below it. `x` = 1.0 inside the island, `y` = the depth below its top in px,
/// `z` = a value in [0, 1) for the island.
fn island(q: vec2<f32>) -> vec3<f32> {
    let period = vec2<f32>(300.0, 230.0);
    let cell = floor(q / period);
    let r = rand_cell(cell, 60u);
    if r > 0.42 {
        return vec3<f32>(0.0);
    }
    let centre = (cell + vec2<f32>(0.25 + 0.5 * rand_cell(cell, 61u), 0.3 + 0.4 * rand_cell(cell, 62u))) * period;
    let half_w = 26.0 + 44.0 * rand_cell(cell, 63u);
    let depth = half_w * (0.9 + 0.6 * rand_cell(cell, 64u));
    let o = q - centre;
    if o.y < 0.0 || o.y > depth || abs(o.x) > half_w {
        return vec3<f32>(0.0);
    }
    let k = floor(o.y / 12.0) * 12.0 / depth;
    if abs(o.x) > half_w * (1.0 - k) * (1.0 - 0.25 * k) {
        return vec3<f32>(0.0);
    }
    return vec3<f32>(1.0, o.y, r / 0.42);
}

/// `a` modulo `b`, in `[0, b)` for a negative `a` also.
fn pmod(a: f32, b: f32) -> f32 {
    return a - b * floor(a / b);
}
