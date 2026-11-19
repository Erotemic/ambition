// The lights of a room at one pixel (`lamps.rs`): the same sum the CPU light
// map makes per texel (`darkness::lamp_texels`), per pixel.
#define_import_path ambition_render::lamps

const MAX_LAMPS: u32 = 64u;
// Where a light's falloff starts, as a part of its radius (`darkness::LIGHT_CORE`).
const LIGHT_CORE: f32 = 0.35;
// How far into the solid a light still shows, and how soft its shadow's edge
// is (`occlusion::FACE`, `occlusion::SOFT`: a grid cell).
const FACE: f32 = 16.0;
const SOFT: f32 = 16.0;

struct Lamps {
    // Per light: x, y, radius, intensity.
    at: array<vec4<f32>, 64>,
    // Per light: r, g, b, and the directions in its reach row (0: unshadowed).
    color: array<vec4<f32>, 64>,
    count: u32,
};

// A direction's turn from +x, 0..4 a quarter at a time (`occlusion::diamond`).
fn diamond(v: vec2<f32>) -> f32 {
    if v.x >= 0.0 && v.y >= 0.0 {
        return v.y / (v.x + v.y);
    }
    if v.x < 0.0 && v.y >= 0.0 {
        return 1.0 - v.x / (v.y - v.x);
    }
    if v.x < 0.0 {
        return 2.0 - v.y / (-v.x - v.y);
    }
    return 3.0 + v.x / (v.x - v.y);
}

// How much of light `n` (whose reach row holds `rays` directions) reaches
// `p` past the ground (`occlusion::lit_part`).
fn lit_part(reach: texture_2d<f32>, n: u32, rays: u32, at: vec2<f32>, p: vec2<f32>) -> f32 {
    let offset = p - at;
    let distance = length(offset);
    if distance < 1e-3 {
        return 1.0;
    }
    let turn = diamond(offset) / 4.0 * f32(rays);
    let below = u32(floor(turn)) % rays;
    let t = fract(turn);
    let r0 = textureLoad(reach, vec2<i32>(i32(below), i32(n)), 0).r;
    let r1 = textureLoad(reach, vec2<i32>(i32((below + 1u) % rays), i32(n)), 0).r;
    let limit = mix(r0, r1, t);
    return clamp((limit + FACE - distance) / SOFT, 0.0, 1.0);
}

// How much of the darkness light `n` (`lamp`: x, y, radius, intensity; its
// reach row `rays` long, 0 unshadowed) takes away at `p`: all of its
// intensity inside `core` (a part of its radius: a dark room's lamp fills a
// pool, `LIGHT_CORE`; a night window's light falls off from its glass, 0),
// none past its radius, none past its reach. Each shader sums its lights
// itself (a uniform cannot be passed by pointer): the darkness left is the
// product of what each leaves, the light their sum.
fn light_of(lamp: vec4<f32>, rays: u32, n: u32, reach: texture_2d<f32>, p: vec2<f32>, core: f32) -> f32 {
    let d = distance(p, lamp.xy) / max(lamp.z, 1.0);
    if d >= 1.0 {
        return 0.0;
    }
    let s = clamp((d - core) / (1.0 - core), 0.0, 1.0);
    var light = lamp.w * (1.0 - s * s * (3.0 - 2.0 * s));
    if rays > 0u && light > 0.0 {
        light *= lit_part(reach, n, rays, lamp.xy, p);
    }
    return light;
}
