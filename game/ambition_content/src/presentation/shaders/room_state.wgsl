// The two-state look: what is not the architecture. See `room_look.rs`.
//
// This shader draws the veil of a blink wall and what floats in front of the
// architecture: effects of the front, not art. The sky is authored parallax
// layers (`room_sky.wgsl`), and the architecture is drawn one time into
// textures (`room_look/architecture.rs`, `room_plate.wgsl`).
//
// All work is in engine world coordinates (y down). Colours are written as
// display values and converted once, at the end.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#import ambition_content::room_look::{rand_cell, look_field, look_claim, look_is_corrupt, look_is_settled, LookClaim}
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
    if role < 1.5 {
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
