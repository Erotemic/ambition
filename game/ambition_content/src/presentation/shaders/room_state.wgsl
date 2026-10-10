// The two-state look: what is not the architecture. See `room_look.rs`.
//
// This shader draws the sky behind the room, the veil of a blink wall, and
// what floats in front of the architecture. The architecture itself (blocks,
// the ornament below a platform, door frames) is drawn one time into
// textures (`room_look/architecture.rs`, `room_plate.wgsl`).
//
// All work is in engine world coordinates (y down). Colours are written as
// display values and converted once, at the end.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::{view, globals}
#import ambition_content::room_look::{rand_cell, value_noise, towers, arcade, island, sky_line_distance, look_field, look_claim, look_air_state, look_is_corrupt, look_is_settled, LookClaim}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> piece: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> room: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> front: vec4<f32>;

const MAGENTA: vec3<f32> = vec3<f32>(1.00, 0.16, 0.66);
const CYAN: vec3<f32> = vec3<f32>(0.13, 0.88, 1.00);
const GOLD: vec3<f32> = vec3<f32>(0.788, 0.635, 0.290);
const GOLD_LIT: vec3<f32> = vec3<f32>(0.940, 0.820, 0.500);

// ----------------------------------------------------------------- field --

/// Signed world distance behind the front. Positive is corrupted.
fn field(p: vec2<f32>) -> f32 {
    return look_field(p, front, globals.time);
}

/// Which block of the front claims `p`.
fn claim(p: vec2<f32>) -> LookClaim {
    return look_claim(p, front, globals.time);
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

// The corrupted sky is behind the play, and it must read so: mid values, low
// contrast, soft light. The darkest values and the hard bright lines of this
// state are for what a body touches (the architecture), so a tower of the
// sky is not as dark as a block, and has no lit edge.
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
        col = col * (0.86 + 0.05 * rand_cell(ca, 501u));
    }
    let qb = p - cam * 0.42 + vec2<f32>(97.0, 60.0);
    let cb = floor(qb / 32.0);
    let near_t = towers((cb + vec2<f32>(0.5)) * 32.0, 370.0, 24u, room.y);
    if near_t > 0.5 && rand_cell(cb, 510u) > 0.16 {
        let r = rand_cell(cb, 511u);
        col = col * (0.78 + 0.06 * r);
        let u = qb / 32.0 - cb;
        if u.y < 0.16 {
            col = col * 1.06;
        }
    }
    // The islands, in blocks, with light below them.
    let qi = p - cam * 0.36 + vec2<f32>(140.0, 40.0);
    let ci = floor(qi / 12.0);
    let isle = island((ci + vec2<f32>(0.5)) * 12.0);
    if isle.x > 0.5 && rand_cell(ci, 540u) > 0.10 {
        let r = rand_cell(ci, 541u);
        col = col * (0.64 + 0.10 * r);
        let u = qi / 12.0 - ci;
        if isle.y < 12.0 && u.y < 0.3 { col = col * 1.18; }
        if r > 0.93 { col = mix(col, select(CYAN, MAGENTA, isle.z > 0.5), 0.22); }
    } else {
        let above = island(qi - vec2<f32>(0.0, 16.0));
        if above.x > 0.5 {
            col = col + MAGENTA * 0.10 * (0.7 + 0.3 * sin(t * 1.5 + above.z * 6.283));
        }
    }
    // The viaduct, in blocks.
    let qv = p - cam * 0.28;
    let cv = floor(qv / 32.0);
    if arcade((cv + vec2<f32>(0.5)) * 32.0) > 0.5 && rand_cell(cv, 520u) > 0.12 {
        let r = rand_cell(cv, 521u);
        col = col * (0.72 + 0.06 * r);
        let u = qv / 32.0 - cv;
        if u.y < 0.15 { col = col * 1.08; }
        if u.x > 0.965 || u.y > 0.965 { col = col * 0.90; }
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
    col = col + glyph * (smoothstep(1.6, 0.2, ld) * 0.26 + exp(-ld / 9.0) * 0.07) * pulse;
    // Light leaks: thin vertical beams, with data that falls down them.
    let bq = p - cam * 0.2;
    let bi = floor(bq.x / 300.0);
    if rand_cell(vec2<f32>(bi, 3.0), 80u) < 0.62 {
        let bx = (bi + 0.2 + 0.6 * rand_cell(vec2<f32>(bi, 4.0), 81u)) * 300.0;
        let dx = abs(bq.x - bx);
        let flow = 0.6 + 0.4 * sin(bq.y * 0.045 - t * 3.2 + bi * 1.7);
        let beam = exp(-dx / 5.0) * 0.26 + exp(-dx / 44.0) * 0.10;
        col = col + mix(MAGENTA, CYAN, rand_cell(vec2<f32>(bi, 5.0), 82u) * 0.5) * beam * flow;
    }
    // Motes that rise.
    let mq = p + vec2<f32>(0.0, t * 22.0);
    let mc = floor(mq / 44.0);
    if rand_cell(mc, 90u) < 0.05 {
        let mo = (mq / 44.0 - mc) - vec2<f32>(0.2 + 0.6 * rand_cell(mc, 91u), 0.5);
        if max(abs(mo.x), abs(mo.y)) < 0.045 {
            col = mix(col, mix(MAGENTA, CYAN, rand_cell(mc, 92u)), 0.55);
        }
    }
    // Air between the eye and the sky: it takes contrast off what is far.
    return mix(col, vec3<f32>(0.330, 0.165, 0.470), 0.16);
}

/// How far the open air at `p` is into the corrupted state, 0..1.
fn air_state(p: vec2<f32>) -> f32 {
    return look_air_state(p, front, globals.time);
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

// ------------------------------------------------------------------ veil --

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
    if look_is_corrupt(p, front, t) {
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

// --------------------------------------------------------------- overlay --

/// Loose blocks that float off the mass, and short tears beside the front.
fn overlay(p: vec2<f32>) -> vec4<f32> {
    // Far from the front there is no loose block and no tear: a room that is
    // all clean or all corrupted pays one dot product for this layer.
    if look_is_settled(p, front) {
        return vec4<f32>(0.0);
    }
    let t = globals.time;
    let c = claim(p);
    if c.state == 2 {
        // A small cube that bobs in its cell. It is hollow: an outline and a
        // faint fill. A loose block is not a place to stand, and a filled
        // block in front of the room reads as one.
        let bob = sin(t * 1.3 + c.seed * 6.283) * 0.07;
        let u = p / c.size - c.cell - vec2<f32>(0.5, 0.5 + bob);
        let half = 0.42;
        let m = max(abs(u.x), abs(u.y));
        if m < half {
            let lit = fract(c.tone * 31.0) < 0.34;
            let tone = select(CYAN, MAGENTA, c.seed > 0.5);
            let ink = vec3<f32>(0.200, 0.130, 0.360) * (0.8 + 0.4 * c.seed);
            if m > half - 1.6 / c.size {
                return vec4<f32>(select(ink * 1.6, tone, lit), select(0.70, 0.90, lit));
            }
            return vec4<f32>(ink, 0.26);
        }
        // A soft light around the lit ones.
        if fract(c.tone * 31.0) < 0.34 {
            let glow = exp(-(m - half) * c.size / 5.0) * 0.30;
            return vec4<f32>(select(CYAN, MAGENTA, c.seed > 0.5), glow);
        }
        return vec4<f32>(0.0);
    }
    // Tears: short runs of light, for a few frames, close to the front. Few
    // of them: motion at the edge of sight takes the eye off the play.
    // On the corrupted side of the front only: the clean side is still.
    let behind = field(p);
    let near_t = exp(-abs(behind) / 90.0) * step(-30.0, behind);
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

// ------------------------------------------------------------------ main --

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let w = mesh.world_position.xy;
    let p = vec2<f32>(w.x + room.x * 0.5, room.y * 0.5 - w.y);
    let role = room.z;
    var col: vec4<f32>;
    if role < 0.5 {
        col = vec4<f32>(backdrop(p), 1.0);
    } else if role < 1.5 {
        // The one surface this shader draws: a blink wall.
        col = veil(p);
    } else {
        col = overlay(p);
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
