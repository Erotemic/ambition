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

/// How much of a point is inside an edge that is `soft` px wide, where `d` is
/// the distance of the point into the shape. A `soft` of 0.0 is a hard edge.
/// The far architecture takes its blur from this: a thing that is behind the
/// play is out of focus.
fn soft_edge(d: f32, soft: f32) -> f32 {
    let half = max(soft, 0.001) * 0.5;
    return smoothstep(-half, half, d);
}

/// How much of `q` is inside the silhouette of a row of far towers, 0..1.
/// `room_h` is the room height: the towers stand on the room floor. `soft` is
/// the width of the edge of the silhouette in px.
fn towers(q: vec2<f32>, period: f32, salt: u32, room_h: f32, soft: f32) -> f32 {
    let i = floor(q.x / period);
    let r = rand_cell(vec2<f32>(i, 0.0), salt);
    let r2 = rand_cell(vec2<f32>(i, 1.0), salt);
    let half_w = period * (0.14 + 0.16 * r2);
    let cx = (i + 0.5) * period + (r - 0.5) * period * 0.25;
    let top = room_h * (0.10 + 0.72 * r);
    let dx = abs(q.x - cx);
    let spire_h = 50.0 + 90.0 * r2;
    let body = soft_edge(half_w - dx, soft) * soft_edge(q.y - top, soft);
    let cap = soft_edge(half_w + 7.0 - dx, soft) * soft_edge(q.y - (top - 12.0), soft) * soft_edge(top - q.y, soft);
    let up = (q.y - (top - 12.0 - spire_h)) / spire_h;
    let spire = soft_edge((half_w - 4.0) * up - dx, soft) * soft_edge(top - 12.0 - q.y, soft);
    // An arched window row, so the tower is not a plain bar.
    let wy = (q.y - top - 40.0) % 150.0;
    let wr = half_w * 0.34;
    let arch = max(soft_edge(wy - wr, soft), soft_edge(wr - length(vec2<f32>(dx, wy - wr)), soft));
    let window = soft_edge(wr - dx, soft) * soft_edge(wy, soft) * soft_edge(46.0 - wy, soft)
        * soft_edge(q.y - (top + 40.0), soft) * arch;
    return max(max(body, cap), spire) * (1.0 - window);
}

/// How much of `q` is inside a viaduct, 0..1: tiers of round arches on piers,
/// every 470 px of height. `soft` is the width of its edge in px.
fn arcade(q: vec2<f32>, soft: f32) -> f32 {
    let tier_h = 470.0;
    let tier = floor(q.y / tier_h);
    if rand_cell(vec2<f32>(tier, 7.0), 26u) < 0.18 {
        return 0.0;
    }
    let y = q.y - tier * tier_h - 120.0;
    let band = soft_edge(y, soft) * soft_edge(250.0 - y, soft);
    if band <= 0.0 {
        return 0.0;
    }
    // The deck, with a groove in it, then arches below it.
    let deck = 1.0 - soft_edge(y - 8.0, soft) * soft_edge(12.0 - y, soft);
    let span = 190.0;
    let x = q.x - floor(q.x / span) * span - span * 0.5;
    let radius = span * 0.5 - 20.0;
    let spring = 26.0 + radius + 10.0;
    let open = max(
        soft_edge(y - spring, soft) * soft_edge(radius - abs(x), soft),
        soft_edge(radius - length(vec2<f32>(x, y - spring)), soft),
    );
    return band * mix(deck, 1.0 - open, soft_edge(y - 26.0, soft));
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
/// below it. `x` = how much of `q` is inside the island (0..1), `y` = the
/// depth below its top in px, `z` = a value in [0, 1) for the island. `soft`
/// is the width of its edge in px.
fn island(q: vec2<f32>, soft: f32) -> vec3<f32> {
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
    if o.y < -soft || o.y > depth + soft || abs(o.x) > half_w + soft {
        return vec3<f32>(0.0);
    }
    // A step each 12 px. Between two steps the width goes from one to the
    // other across the soft edge.
    let g = o.y / 12.0 - 0.5;
    let n = floor(g);
    let t = soft_edge((g - n - 0.5) * 12.0, soft);
    let k0 = max(n, 0.0) * 12.0 / depth;
    let k1 = (n + 1.0) * 12.0 / depth;
    let width = mix(half_w * (1.0 - k0) * (1.0 - 0.25 * k0), half_w * (1.0 - k1) * (1.0 - 0.25 * k1), t);
    let cover = soft_edge(o.y, soft) * soft_edge(depth - o.y, soft) * soft_edge(width - abs(o.x), soft);
    return vec3<f32>(cover, o.y, r / 0.42);
}

/// `a` modulo `b`, in `[0, b)` for a negative `a` also.
fn pmod(a: f32, b: f32) -> f32 {
    return a - b * floor(a / b);
}

// ------------------------------------------------- the front of a room --
//
// The two-state look has a front: a line across the room with a ragged edge
// made of blocks. `front` is a point on the line (`xy`) and its normal
// (`zw`), which points into the corrupted side. `t` is the time in seconds.

/// Signed world distance behind the front. Positive is corrupted.
fn look_field(p: vec2<f32>, front: vec4<f32>, t: f32) -> f32 {
    let wobble = (value_noise(p, 260.0, 7u) - 0.5) * 260.0
        + (value_noise(p + vec2<f32>(t * 5.0, t * 2.0), 90.0, 11u) - 0.5) * 70.0;
    let breathe = sin(t * 0.33) * 30.0;
    return dot(p - front.xy, front.zw) + wobble + breathe;
}

struct LookLevel {
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

fn look_level(k: i32) -> LookLevel {
    if k == 0 { return LookLevel(128.0, 360.0, 180.0, 0.0, 0.0); }
    if k == 1 { return LookLevel(64.0, 90.0, 110.0, 150.0, 0.09); }
    if k == 2 { return LookLevel(32.0, 10.0, 56.0, 280.0, 0.15); }
    return LookLevel(16.0, -6.0, 28.0, 340.0, 0.09);
}

struct LookClaim {
    // 0 = clean, 1 = the corrupted mass, 2 = a loose block.
    state: i32,
    size: f32,
    cell: vec2<f32>,
    // Two values in [0, 1) for the block. (A field name must not end in a
    // digit: the shader composer writes such a name with a suffix, and a
    // shader that imports the struct then does not find the field.)
    seed: f32,
    tone: f32,
}

/// Which block of the front claims `p`: the coarsest level that is corrupt.
fn look_claim(p: vec2<f32>, front: vec4<f32>, t: f32) -> LookClaim {
    for (var k = 0; k < 4; k++) {
        let lv = look_level(k);
        let cell = floor(p / lv.size);
        let salt = u32(k);
        let fc = look_field((cell + vec2<f32>(0.5)) * lv.size, front, t);
        let ra = rand_cell(cell, 100u + salt);
        let rb = rand_cell(cell, 150u + salt);
        let mass = fc + (ra - 0.5) * lv.jitter > lv.depth;
        let gap = clamp((lv.depth - fc) / max(lv.reach, 1.0), 0.0, 1.0);
        let stray = fc > lv.depth - lv.reach && rb < lv.loose * (1.0 - gap) * (1.0 - gap);
        if mass || stray {
            let r = rand_cell(cell, 200u + salt);
            let r2 = rand_cell(cell, 300u + salt);
            return LookClaim(select(2, 1, mass), lv.size, cell, r, r2);
        }
    }
    return LookClaim(0, 0.0, vec2<f32>(0.0), 0.0, 0.0);
}

// How far `look_field` can be from the plane distance: its wobble (130 and
// 35) and its breath (30).
const LOOK_FIELD_SWING: f32 = 195.0;
// Past this plane distance behind the front, each point is of the corrupted
// mass: the largest blocks (128 px, whose middle is at most 91 px from the
// point) are of the mass from a field of 450.
const LOOK_SURELY_CORRUPT: f32 = 740.0;
// Past this plane distance ahead of the front, each point is clean: the
// loose blocks that go farthest ahead (16 px, at most 12 px away) stop at a
// field of -346.
const LOOK_SURELY_CLEAN: f32 = -560.0;

/// Plane distance behind the front. Positive is the corrupted side.
fn look_plane(p: vec2<f32>, front: vec4<f32>) -> f32 {
    return dot(p - front.xy, front.zw);
}

/// Whether `p` is far from the front: its state is known from the plane
/// alone, and nothing the front does to the things near it reaches it. A
/// room that is all clean or all corrupted is far from its front at each
/// point, so it pays for no noise.
fn look_is_settled(p: vec2<f32>, front: vec4<f32>) -> bool {
    let d = look_plane(p, front);
    return d > LOOK_SURELY_CORRUPT || d < LOOK_SURELY_CLEAN;
}

/// Whether the block of the front that claims `p` is of the corrupted mass.
fn look_is_corrupt(p: vec2<f32>, front: vec4<f32>, t: f32) -> bool {
    let d = look_plane(p, front);
    if d > LOOK_SURELY_CORRUPT { return true; }
    if d < LOOK_SURELY_CLEAN { return false; }
    return look_claim(p, front, t).state == 1;
}

/// How far the open air at `p` is into the corrupted state, 0..1. Built
/// things break at a hard edge; air changes as haze.
fn look_air_state(p: vec2<f32>, front: vec4<f32>, t: f32) -> f32 {
    // Far from the front the haze is all or nothing (its wisps are 55).
    let d = look_plane(p, front);
    if d > 110.0 + 55.0 + LOOK_FIELD_SWING { return 1.0; }
    if d < -260.0 - 55.0 - LOOK_FIELD_SWING { return 0.0; }
    // The haze has wisps, and they drift.
    let wisp = (value_noise(p + vec2<f32>(t * 13.0, -t * 5.0), 130.0, 13u) - 0.5) * 110.0;
    return smoothstep(-260.0, 110.0, look_field(p, front, t) + wisp);
}
