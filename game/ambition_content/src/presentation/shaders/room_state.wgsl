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
#import ambition_content::room_look::{rand_cell, value_noise, towers, arcade, island, soft_edge, sky_line_distance, look_field, look_claim, look_air_state, look_is_corrupt, look_is_settled, LookClaim}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> piece: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> room: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> front: vec4<f32>;
// How far behind the play the sky is drawn (`RoomLookDepth`). x: the width of
// the edge of the nearest far architecture, in world px. y: how many times
// wider the edge of the farthest is. z: how much fog is in front of the sky,
// 0..1. w: how much the fog is in patches, 0..1.
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> depth: vec4<f32>;

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
//
// The sky is behind the play, and it must read so. The play is sharp and
// clear: hard edges, the darkest values, the bright lines. The sky is the
// opposite of each: its architecture is out of focus (`depth.x`, `depth.y`),
// it has mid values and low contrast, and there is fog between it and the
// play (`depth.z`, `depth.w`).

fn camera_engine() -> vec2<f32> {
    let c = view.world_position.xy;
    return vec2<f32>(c.x + room.x * 0.5, room.y * 0.5 - c.y);
}

// How much of the camera's motion each row of the far architecture follows.
// More is farther.
const FAR_CITY: f32 = 0.78;
const FAR_TOWERS: f32 = 0.62;
const NEAR_TOWERS: f32 = 0.42;
const ISLANDS: f32 = 0.36;
const VIADUCT: f32 = 0.28;

/// The width of the edge of a thing of the sky that follows `follow` of the
/// camera's motion: the nearest row has `depth.x`, the farthest `depth.y`
/// times that.
fn blur_of(follow: f32) -> f32 {
    let far = clamp((follow - VIADUCT) / (FAR_CITY - VIADUCT), 0.0, 1.0);
    return depth.x * mix(1.0, depth.y, far);
}

/// How near `p` is to a line of a lattice of `pitch` px, 0..1, with an edge
/// that is as soft as the nearest row of the sky. A wider line is fainter.
fn lattice_line(p: vec2<f32>, pitch: f32) -> f32 {
    let g = p - floor(p / pitch) * pitch;
    let d = min(min(g.x, pitch - g.x), min(g.y, pitch - g.y));
    return (1.0 - smoothstep(0.5, 1.0 + depth.x, d)) / (1.0 + 0.5 * depth.x);
}

fn backdrop_clean(p: vec2<f32>) -> vec3<f32> {
    let cam = camera_engine();
    let g = clamp(p.y / room.y, 0.0, 1.0);
    var col = mix(vec3<f32>(0.972, 0.952, 0.905), vec3<f32>(0.868, 0.895, 0.938), g);
    // Drawing paper: a faint grid.
    col = col * (1.0 - 0.025 * lattice_line(p, 64.0));
    // A far city, then towers at two depths.
    let city = towers(p - cam * FAR_CITY + vec2<f32>(31.0, -140.0), 88.0, 28u, room.y, blur_of(FAR_CITY));
    col = mix(col, vec3<f32>(0.868, 0.888, 0.925), city * 0.70);
    let far_t = towers(p - cam * FAR_TOWERS, 170.0, 20u, room.y, blur_of(FAR_TOWERS));
    col = mix(col, vec3<f32>(0.822, 0.850, 0.902), far_t * 0.85);
    let near_t = towers(p - cam * NEAR_TOWERS + vec2<f32>(97.0, 60.0), 370.0, 24u, room.y, blur_of(NEAR_TOWERS));
    col = mix(col, vec3<f32>(0.770, 0.805, 0.872), near_t * 0.90);
    // Islands that float between the towers and the viaduct.
    let isle_soft = blur_of(ISLANDS);
    let isle = island(p - cam * ISLANDS + vec2<f32>(140.0, 40.0), isle_soft);
    if isle.x > 0.0 {
        var stone = mix(vec3<f32>(0.800, 0.828, 0.888), vec3<f32>(0.740, 0.772, 0.846), isle.z);
        // A pale top, and a gold line below it.
        stone = mix(stone, vec3<f32>(0.940, 0.945, 0.955), 1.0 - soft_edge(isle.y - 4.0, isle_soft));
        let line = soft_edge(isle.y - 5.0, isle_soft) * soft_edge(6.5 - isle.y, isle_soft);
        stone = mix(stone, vec3<f32>(0.820, 0.730, 0.500), line);
        col = mix(col, stone, isle.x * 0.90);
    }
    // A viaduct of arches, nearer again.
    let via = arcade(p - cam * VIADUCT, blur_of(VIADUCT));
    col = mix(col, vec3<f32>(0.735, 0.770, 0.845), via * 0.92);
    let mist = value_noise(p - cam * 0.3 + vec2<f32>(globals.time * 7.0, 0.0), 340.0, 30u);
    col = mix(col, vec3<f32>(0.975, 0.965, 0.945), smoothstep(0.45, 0.95, mist) * 0.55);
    // Construction lines.
    let ld = sky_line_distance(p - cam * 0.30, globals.time * 0.04);
    col = mix(col, vec3<f32>(0.800, 0.690, 0.420), smoothstep(1.3 + depth.x, 0.3, ld) * 0.75 / (1.0 + 0.3 * depth.x));
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

/// How dark the block `c` of one row of the corrupted sky makes the sky: 1.0
/// where the row has no block. The rows are the rows of the clean sky,
/// rebuilt in blocks, and some blocks are missing.
fn sky_block(row: i32, c: vec2<f32>) -> f32 {
    if row == 0 {
        if towers((c + vec2<f32>(0.5)) * 16.0, 88.0, 28u, room.y, 0.0) > 0.5 && rand_cell(c, 530u) > 0.10 {
            return 0.88 + 0.06 * rand_cell(c, 531u);
        }
    } else if row == 1 {
        if towers((c + vec2<f32>(0.5)) * 24.0, 170.0, 20u, room.y, 0.0) > 0.5 && rand_cell(c, 500u) > 0.13 {
            return 0.86 + 0.05 * rand_cell(c, 501u);
        }
    } else if row == 2 {
        if towers((c + vec2<f32>(0.5)) * 32.0, 370.0, 24u, room.y, 0.0) > 0.5 && rand_cell(c, 510u) > 0.16 {
            return 0.78 + 0.06 * rand_cell(c, 511u);
        }
    } else if row == 3 {
        if island((c + vec2<f32>(0.5)) * 12.0, 0.0).x > 0.5 && rand_cell(c, 540u) > 0.10 {
            return 0.64 + 0.10 * rand_cell(c, 541u);
        }
    } else {
        if arcade((c + vec2<f32>(0.5)) * 32.0, 0.0) > 0.5 && rand_cell(c, 520u) > 0.12 {
            return 0.72 + 0.06 * rand_cell(c, 521u);
        }
    }
    return 1.0;
}

/// [`sky_block`] at `q`, for blocks of `size` px, with an edge between two
/// blocks that is `soft` px wide. A point that is not near an edge reads one
/// block.
fn sky_blocks(row: i32, q: vec2<f32>, size: f32, soft: f32) -> f32 {
    // In `f`, 0.5 is the edge between the block `i` and the next one.
    let g = q / size - vec2<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let t = vec2<f32>(soft_edge((f.x - 0.5) * size, soft), soft_edge((f.y - 0.5) * size, soft));
    var shade = 0.0;
    let w00 = (1.0 - t.x) * (1.0 - t.y);
    let w10 = t.x * (1.0 - t.y);
    let w01 = (1.0 - t.x) * t.y;
    let w11 = t.x * t.y;
    if w00 > 0.0 { shade = shade + w00 * sky_block(row, i); }
    if w10 > 0.0 { shade = shade + w10 * sky_block(row, i + vec2<f32>(1.0, 0.0)); }
    if w01 > 0.0 { shade = shade + w01 * sky_block(row, i + vec2<f32>(0.0, 1.0)); }
    if w11 > 0.0 { shade = shade + w11 * sky_block(row, i + vec2<f32>(1.0, 1.0)); }
    return shade;
}

fn backdrop_corrupt(p: vec2<f32>) -> vec3<f32> {
    let cam = camera_engine();
    let t = globals.time;
    let g = clamp(p.y / room.y, 0.0, 1.0);
    var col = mix(vec3<f32>(0.150, 0.085, 0.275), vec3<f32>(0.400, 0.185, 0.530), g);
    let haze = value_noise(p - cam * 0.3 + vec2<f32>(t * 9.0, 0.0), 300.0, 71u);
    col = col + (haze - 0.4) * vec3<f32>(0.090, 0.025, 0.110);
    // The same city and towers, rebuilt in blocks. A block of the sky has no
    // lit edge and no neon: those are for what a body touches.
    col = col * sky_blocks(0, p - cam * FAR_CITY + vec2<f32>(31.0, -140.0), 16.0, blur_of(FAR_CITY));
    col = col * sky_blocks(1, p - cam * FAR_TOWERS, 24.0, blur_of(FAR_TOWERS));
    col = col * sky_blocks(2, p - cam * NEAR_TOWERS + vec2<f32>(97.0, 60.0), 32.0, blur_of(NEAR_TOWERS));
    // The islands, in blocks, with light below them.
    let qi = p - cam * ISLANDS + vec2<f32>(140.0, 40.0);
    let isle = sky_blocks(3, qi, 12.0, blur_of(ISLANDS));
    col = col * isle;
    let above = island(qi - vec2<f32>(0.0, 16.0), blur_of(ISLANDS) + 6.0);
    col = col + MAGENTA * 0.10 * above.x * smoothstep(0.8, 1.0, isle) * (0.7 + 0.3 * sin(t * 1.5 + above.z * 6.283));
    // The viaduct, in blocks.
    col = col * sky_blocks(4, p - cam * VIADUCT, 32.0, blur_of(VIADUCT));
    // The lattice the world is written on.
    col = col + vec3<f32>(0.010, 0.030, 0.045) * lattice_line(p, 64.0);
    // The construction lines are a glyph now, and it is on.
    let ld = sky_line_distance(p - cam * 0.30, t * 0.22);
    let pulse = 0.72 + 0.28 * sin(t * 1.7 + p.x * 0.004);
    let glyph = mix(MAGENTA, CYAN, 0.28);
    col = col + glyph * (smoothstep(1.6 + depth.x, 0.2, ld) * 0.26 / (1.0 + 0.3 * depth.x) + exp(-ld / 9.0) * 0.07) * pulse;
    // Light leaks: soft vertical beams, with data that falls down them.
    let bq = p - cam * 0.2;
    let bi = floor(bq.x / 300.0);
    if rand_cell(vec2<f32>(bi, 3.0), 80u) < 0.62 {
        let bx = (bi + 0.2 + 0.6 * rand_cell(vec2<f32>(bi, 4.0), 81u)) * 300.0;
        let dx = abs(bq.x - bx);
        let flow = 0.6 + 0.4 * sin(bq.y * 0.045 - t * 3.2 + bi * 1.7);
        let beam = exp(-dx / (5.0 + depth.x)) * 0.26 + exp(-dx / 44.0) * 0.10;
        col = col + mix(MAGENTA, CYAN, rand_cell(vec2<f32>(bi, 5.0), 82u) * 0.5) * beam * flow;
    }
    // Motes that rise.
    let mq = p + vec2<f32>(0.0, t * 22.0);
    let mc = floor(mq / 44.0);
    if rand_cell(mc, 90u) < 0.05 {
        let mo = ((mq / 44.0 - mc) - vec2<f32>(0.2 + 0.6 * rand_cell(mc, 91u), 0.5)) * 44.0;
        let mote = 1.0 - smoothstep(1.0, 2.6 + 0.5 * depth.x, length(mo));
        col = mix(col, mix(MAGENTA, CYAN, rand_cell(mc, 92u)), 0.55 * mote);
    }
    return col;
}

/// How far the open air at `p` is into the corrupted state, 0..1.
fn air_state(p: vec2<f32>) -> f32 {
    return look_air_state(p, front, globals.time);
}

/// The fog between the sky and the play, over the sky colour `col`. `a` is
/// the state of the air at `p`: the fog is pale in clean air and lilac in
/// corrupted air. It is in slow, wide patches, and it is nearer than each row
/// of the sky, so it follows the camera least.
fn fog(col: vec3<f32>, p: vec2<f32>, a: f32) -> vec3<f32> {
    if depth.z <= 0.0 {
        return col;
    }
    let t = globals.time;
    let q = p - camera_engine() * 0.15;
    let wide = value_noise(q + vec2<f32>(t * 4.0, 0.0), 420.0, 77u);
    let fine = value_noise(q + vec2<f32>(-t * 2.5, t * 1.5), 170.0, 78u);
    // About 1.0, more and less of it in patches.
    let patches = mix(1.0, 0.35 + 1.3 * (0.65 * wide + 0.35 * fine), depth.w);
    // Fog lies low: a little more of it toward the floor of the room.
    let low = mix(0.85, 1.15, clamp(p.y / room.y, 0.0, 1.0));
    let amount = clamp(depth.z * patches * low, 0.0, 0.92);
    let tint = mix(vec3<f32>(0.965, 0.958, 0.950), vec3<f32>(0.430, 0.255, 0.570), a);
    return mix(col, tint, amount);
}

fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let a = air_state(p);
    var col: vec3<f32>;
    if a <= 0.001 {
        col = backdrop_clean(p);
    } else if a >= 0.999 {
        col = backdrop_corrupt(p);
    } else {
        // Through a lilac mist, not through grey.
        let mist = vec3<f32>(0.800, 0.640, 0.860);
        let clean = mix(backdrop_clean(p), mist, smoothstep(0.0, 0.55, a) * 0.85);
        col = mix(clean, backdrop_corrupt(p), smoothstep(0.35, 1.0, a));
    }
    return fog(col, p, a);
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
