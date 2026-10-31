// The Spectral Perturbation's tremble and shimmer, at Ultra (Jon,
// 2026-10-31: "parts with vibrations to visually communicate the
// perturbation aspect of it. And perhaps more of a shimmer on other parts. A
// small cheap shader might help here too, but we need it to work without
// shaders as well").
//
// NOT YET WIRED: a Material2d for the creature's sprite (the
// `spectral_perturbation` sheet), drawn in place of its plain sprite. The
// sheet already trembles and shimmers on its own, frame by frame (shaders
// off); this adds what a sheet cannot: motion between frames, smooth and
// continuous. At strength 0 it draws the sprite exactly as authored.
//
// What it does, all from the sprite's own pixels (it paints no new art):
// - VIBRATION: the lookup trembles, fast and small, more the further a pixel
//   is from the core (the shards' tips shake, the core holds), each band of
//   the body on its own phase so the parts quiver out of step; and the
//   colour splits as it shakes, cyan one way and magenta the other (the
//   sheet's vibration echoes, now continuous).
// - SHIMMER: a bright band sweeps across the pale shards (only the pale,
//   unsaturated pixels: the shards, not the core or the ribbons), and the
//   brightest pixels sparkle, each on its own clock.
// - RIM: the sprite's own edge (where its alpha falls off) pulses a soft
//   cyan, faster and brighter as the vibration rises.
//
// Uniforms (plain vec4s, as the workspace's other overlays):
// - uv_rect: the current frame on its sheet (min.x, min.y, max.x, max.y).
// - control: x vibration 0..1 (the pose's: low at rest, high in the
//   telegraph, recovery, hurt and death), y 1 when flipped, z shimmer 0..1,
//   w a per-creature seed (so two of them never tremble in step).
// - core: xy the core's place in the frame (0..1, y down; about 0.48, 0.42
//   on the sheet), z the core's glow 0..1, w unused.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> core: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var sprite_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var sprite_sampler: sampler;

const TAU: f32 = 6.28318530718;
const CYAN: vec3<f32> = vec3<f32>(0.44, 0.95, 0.94);
const MAGENTA: vec3<f32> = vec3<f32>(0.89, 0.36, 0.81);
const WHITE: vec3<f32> = vec3<f32>(0.97, 0.98, 1.0);
// The tremble: its rate (cycles a second), its reach (in frame UV at full
// vibration, at a pixel far from the core) and the bands it quivers in.
const TREMBLE_HZ: f32 = 17.0;
const TREMBLE_REACH: f32 = 0.006;
const BANDS: f32 = 9.0;
// The colour split's reach, against the tremble's.
const SPLIT: f32 = 0.7;
// The shimmer's sweep: its rate (sweeps a second) and its width.
const SWEEP_HZ: f32 = 0.45;
const SWEEP_WIDTH: f32 = 0.07;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// The sprite at a frame UV (0..1 within the frame), clamped to the frame so a
// trembling lookup never reads its neighbour on the sheet.
fn sample_frame(local: vec2<f32>) -> vec4<f32> {
    let inside = clamp(local, vec2<f32>(0.001), vec2<f32>(0.999));
    let edge = step(0.0, local.x) * step(local.x, 1.0) * step(0.0, local.y) * step(local.y, 1.0);
    return textureSample(sprite_texture, sprite_sampler, mix(uv_rect.xy, uv_rect.zw, inside)) * edge;
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    var local = mesh.uv;
    if control.y > 0.5 {
        local.x = 1.0 - local.x;
    }
    let vibration = clamp(control.x, 0.0, 1.0);
    let shimmer = clamp(control.z, 0.0, 1.0);
    let t = globals.time + control.w * 13.7;

    // VIBRATION: a tremble that grows away from the core; each band of the
    // body (bands across its height) on its own phase.
    let from_core = local - core.xy;
    let reach = smoothstep(0.04, 0.4, length(from_core));
    let band = floor(local.y * BANDS);
    let phase = hash(vec2<f32>(band, control.w)) * TAU;
    let wobble = sin(t * TREMBLE_HZ * TAU + phase) * 0.7 + sin(t * TREMBLE_HZ * 1.7 * TAU + phase * 2.3) * 0.3;
    let along = normalize(vec2<f32>(-from_core.y, from_core.x) + vec2<f32>(1e-4, 0.0));
    let shake = along * wobble * TREMBLE_REACH * vibration * reach;
    let base = sample_frame(local + shake);

    // The colour split: cyan one way, magenta the other, as it shakes.
    let split = shake * (1.0 + SPLIT / max(TREMBLE_REACH, 1e-4) * 0.004 * vibration);
    let ahead = sample_frame(local + shake + split);
    let behind = sample_frame(local + shake - split);
    var colour = base.rgb;
    var alpha = base.a;
    let echo = vibration * 0.55;
    colour = colour + CYAN * ahead.a * (1.0 - base.a) * echo;
    colour = colour + MAGENTA * behind.a * (1.0 - base.a) * echo;
    alpha = max(alpha, max(ahead.a, behind.a) * echo);

    // SHIMMER: a band sweeping across the pale shards, and sparkles.
    let lum = dot(base.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sat = max(base.r, max(base.g, base.b)) - min(base.r, min(base.g, base.b));
    let pale = smoothstep(0.45, 0.75, lum) * smoothstep(0.35, 0.12, sat) * base.a;
    let sweep_pos = fract(t * SWEEP_HZ) * 1.6 - 0.3;
    let diagonal = (local.x + local.y * 0.6) * 0.62;
    let sweep = exp(-pow((diagonal - sweep_pos) / SWEEP_WIDTH, 2.0));
    colour = colour + WHITE * sweep * pale * 0.55 * shimmer;
    let cell = floor(local * 90.0);
    let clock = fract(t * (0.6 + hash(cell) * 1.4) + hash(cell + 7.0));
    let sparkle = smoothstep(0.92, 1.0, 1.0 - abs(clock * 2.0 - 1.0)) * step(0.8, lum) * step(0.93, hash(cell + 3.0));
    colour = colour + WHITE * sparkle * 0.9 * shimmer * base.a;

    // RIM: the sprite's own edge pulses cyan, more as it trembles.
    let d = 0.006;
    let around = sample_frame(local + vec2<f32>(d, 0.0)).a + sample_frame(local - vec2<f32>(d, 0.0)).a
        + sample_frame(local + vec2<f32>(0.0, d)).a + sample_frame(local - vec2<f32>(0.0, d)).a;
    let rim = clamp(base.a - around * 0.25, 0.0, 1.0) + clamp(around * 0.25 - base.a, 0.0, 1.0) * 0.6;
    let pulse = 0.5 + 0.5 * sin(t * (2.0 + 6.0 * vibration) * TAU);
    colour = colour + CYAN * rim * (0.25 + 0.5 * vibration) * (0.6 + 0.4 * pulse) * max(shimmer, vibration);
    alpha = max(alpha, rim * 0.5 * vibration);

    // The core breathes its own light.
    let core_light = exp(-dot(from_core, from_core) * 900.0) * core.z;
    colour = colour + mix(MAGENTA, CYAN, 0.5 + 0.5 * sin(t * 1.3 * TAU)) * core_light * 0.5 * base.a;

    if alpha <= 0.002 {
        discard;
    }
    return vec4<f32>(colour, alpha);
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// the shaded colour is written sRGB-encoded, as Bevy's own shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
