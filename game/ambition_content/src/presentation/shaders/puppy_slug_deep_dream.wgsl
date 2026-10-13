#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import ambition_render::frame_in_sprite
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var puppy_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var puppy_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<uniform> frame_rect: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var skin_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var skin_sampler: sampler;
// xy: how many tiles of the skin go across the frame, and down it.
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var<uniform> skin_tiles: vec4<f32>;

const PI: f32 = 3.141592653589793;

// THE DREAM IS MADE OF AUTHORED ART. This pass draws no art of its own: each
// colour it writes is a sample of the body's own frame or of the SKIN OF THE
// DREAM (`puppy_slug_dream_skin.png`, a picture of the slug's sprite target:
// eyes in a lattice, a whorl of ripples round each, dots as scales). It is the
// look of the pictures an image network dreamed in 2015. The sheet is good to
// look at with the pass off.
//
//   1. The flow: the art is sampled a little way off, along a slow field, so
//      its surface moves as if it breathes.
//   2. The skin: the tile is laid over the body twice, large and fine, and it
//      goes with the flow. It takes the colour and the light of the art under
//      it (the fur stays warm and the slug stays green), its eyes show whole,
//      and its red and its blue are a little apart, as in those pictures.
//   3. The copies: the frame again, small, in a grid that drifts. A dream of
//      this kind finds the animal again in each part of the animal.
//   4. The oil: the hue turns along the flow, and thin bright bands follow the
//      lines of equal light.
//
// Black line art stays black, and nothing is drawn where the sprite is clear.

// How far the art is moved by the flow, as a part of the frame.
const FLOW_REACH: f32 = 0.030;
// How much of the skin shows (its own alpha is 1 at an eye), large and fine;
// how far it takes the colour of the art under it; how far its red and its
// blue are apart, in tiles.
const SKIN_ALPHA: f32 = 0.86;
const SKIN_FINE_ALPHA: f32 = 0.22;
const SKIN_DYE: f32 = 0.7;
const SKIN_FRINGE: f32 = 0.008;
// How many copies of the frame go across it, and how much of a copy shows.
const COPIES: f32 = 5.0;
const COPY_ALPHA: f32 = 0.30;
// How far the hue turns (radians, each way) and how bright the bands are.
const HUE_TURN: f32 = 0.9;
const BAND_LIGHT: f32 = 0.16;

// Preserve black / near-black line art.
const BLACK_PRESERVE_LOW: f32 = 0.025;
const BLACK_PRESERVE_HIGH: f32 = 0.110;

fn hash21(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q.x + q.y) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i + vec2<f32>(0.0, 0.0));
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var amp = 0.5;
    var total = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 5; i = i + 1) {
        total += noise(p) * amp;
        norm += amp;
        p = mat2x2<f32>(1.62, 1.11, -1.04, 1.71) * p + vec2<f32>(3.7, 1.9);
        amp *= 0.53;
    }
    return total / max(norm, 0.001);
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let p = abs(fract(c.xxx + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - vec3<f32>(3.0));
    return c.z * mix(
        vec3<f32>(1.0),
        clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)),
        c.y
    );
}

// The pattern is laid over the body's FRAME (`frame_rect`), not over the whole
// quad: every coordinate below is the frame's own, 0..1 across it.
fn flipped() -> bool {
    return control.y > 0.5;
}

fn on_quad(frame_uv: vec2<f32>) -> vec2<f32> {
    return frame_in_sprite::on_quad(frame_uv, frame_rect, flipped());
}

fn atlas_uv(frame_uv: vec2<f32>) -> vec2<f32> {
    var local_uv = frame_in_sprite::quad_uv(frame_uv, frame_rect, flipped());
    if flipped() {
        local_uv.x = 1.0 - local_uv.x;
    }
    let atlas_min = uv_rect.xy;
    let atlas_max = uv_rect.zw;
    return mix(atlas_min, atlas_max, local_uv);
}

fn sample_frame(local_uv: vec2<f32>) -> vec4<f32> {
    return textureSample(puppy_texture, puppy_sampler, atlas_uv(local_uv));
}

// The art at a frame coordinate; clear where the coordinate is off the frame.
fn sample_art(frame_uv: vec2<f32>) -> vec4<f32> {
    let art = sample_frame(on_quad(frame_uv));
    let low = step(vec2<f32>(0.0), frame_uv);
    let high = step(frame_uv, vec2<f32>(1.0));
    return vec4<f32>(art.rgb, art.a * low.x * low.y * high.x * high.y);
}

// A colour turned about the grey axis: its hue changes and its light does not.
fn hue_turn(c: vec3<f32>, angle: f32) -> vec3<f32> {
    let k = vec3<f32>(0.57735);
    let ca = cos(angle);
    let sa = sin(angle);
    return c * ca + cross(k, c) * sa + k * dot(k, c) * (1.0 - ca);
}

// The frame again, small, in a grid of `cells` across it that drifts. Each
// cell wakes and sleeps on its own clock, half of the cells are mirrored, and
// each has a hue of its own.
fn copies(frame_uv: vec2<f32>, cells: f32, time: f32, seed: f32, drift: vec2<f32>) -> vec4<f32> {
    let p = frame_uv * cells + drift * time + vec2<f32>(seed * 3.1, seed * 1.7);
    let cell = floor(p);
    let q = fract(p);
    let n = hash21(cell + vec2<f32>(seed, 7.0));
    let wake = smoothstep(0.5, 0.92, 0.5 + 0.5 * sin(time * (0.45 + n) + n * 40.0));
    let mirror = step(0.5, hash21(cell + vec2<f32>(3.0, seed)));
    let src = vec2<f32>(mix(q.x, 1.0 - q.x, mirror), q.y);
    let art = sample_art(src);
    let edge = smoothstep(0.0, 0.14, min(min(q.x, 1.0 - q.x), min(q.y, 1.0 - q.y)));
    return vec4<f32>(hue_turn(art.rgb, n * 2.0 * PI), art.a * wake * edge);
}

// The skin of the dream at a place in tiles. It repeats.
fn sample_skin(p: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(skin_texture, skin_sampler, fract(p), 0.0);
}

fn dream_rgb(
    local_uv: vec2<f32>,
    base_rgb: vec3<f32>,
    time: f32,
    strength: f32,
    seed: f32
) -> vec3<f32> {
    let luma_of = vec3<f32>(0.299, 0.587, 0.114);

    // 1. The flow.
    let grain = vec2<f32>(5.0, 3.6);
    let f1 = fbm(local_uv * grain + vec2<f32>(time * 0.21 + seed, -time * 0.13));
    let f2 = fbm(local_uv * grain + vec2<f32>(11.3 - time * 0.17, 4.1 + time * 0.19 + seed * 2.3));
    let flow = (vec2<f32>(f1, f2) - vec2<f32>(0.5)) * FLOW_REACH * strength;
    let moved = sample_art(local_uv + flow);
    var rgb = mix(base_rgb, moved.rgb, moved.a * 0.85);

    // 2. The skin.
    let tiles = max(skin_tiles.xy, vec2<f32>(0.25));
    let place = (local_uv + flow * 1.6) * tiles
        + vec2<f32>(time * 0.012 + seed * 0.37, sin(time * 0.2) * 0.01 + seed * 0.61);
    let fringe = vec2<f32>(SKIN_FRINGE, -SKIN_FRINGE * 0.6) * strength;
    let skin = sample_skin(place);
    let skin_rgb = vec3<f32>(sample_skin(place + fringe).r, skin.g, sample_skin(place - fringe).b);
    let lit = 0.45 + 0.9 * dot(rgb, luma_of);
    let dyed = mix(skin_rgb, skin_rgb * (rgb * 1.6 + vec3<f32>(0.15)), SKIN_DYE);
    rgb = mix(rgb, dyed * lit, skin.a * SKIN_ALPHA * strength);
    let fine = sample_skin(place * 2.3 + vec2<f32>(0.31, 0.17) - flow * tiles * 2.0);
    rgb = mix(rgb, mix(fine.rgb, fine.rgb * rgb * 1.8, 0.6), fine.a * SKIN_FINE_ALPHA * strength);

    // 3. The copies.
    let small = copies(local_uv - flow * 3.0, COPIES, time, seed + 0.37, vec2<f32>(-0.09, 0.02));
    rgb = mix(rgb, small.rgb, small.a * COPY_ALPHA * strength);

    // 4. The oil.
    let field = f1 * 0.7 + f2 * 0.3;
    let luma = dot(rgb, luma_of);
    let turn = ((field - 0.5) * 2.0 * HUE_TURN + sin(time * 0.35 + seed * 6.0) * 0.35) * strength;
    rgb = hue_turn(rgb, turn);
    let grey = vec3<f32>(dot(rgb, luma_of));
    rgb = max(mix(grey, rgb, 1.0 + 0.3 * strength), vec3<f32>(0.0));
    let band = abs(fract((luma * 1.6 + field * 2.4 + time * 0.05) * 3.0) - 0.5);
    let ridge = 1.0 - smoothstep(0.03, 0.11, band);
    let rainbow = hsv2rgb(vec3<f32>(fract(field * 1.3 + time * 0.07 + seed), 0.7, 1.0));
    rgb += rainbow * ridge * BAND_LIGHT * strength * smoothstep(0.06, 0.3, luma);

    return rgb;
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let time = control.x;
    let strength = clamp(control.z, 0.0, 1.0);
    let seed = control.w * 17.0;

    // Bevy's 2D `Rectangle` mesh and the atlas agree on +y down in a frame, so
    // `mesh.uv` is the sample with no flip of its own.
    let uv = frame_in_sprite::frame_uv(mesh.uv, frame_rect, flipped());
    let base = sample_frame(uv);
    let dream = dream_rgb(uv, base.rgb, time, strength, seed);

    // Only draw where the sprite itself is visible.
    let mask_alpha = base.a;
    if mask_alpha <= 0.010 {
        discard;
    }

    // Preserve black / near-black line art.
    let max_channel = max(base.rgb.r, max(base.rgb.g, base.rgb.b));
    let preserve_black = 1.0 - smoothstep(
        BLACK_PRESERVE_LOW,
        BLACK_PRESERVE_HIGH,
        max_channel
    );
    let rgb = mix(dream, base.rgb, preserve_black);

    return vec4<f32>(
        clamp(rgb * tint.rgb, vec3<f32>(0.0), vec3<f32>(1.6)),
        mask_alpha * strength * tint.a
    );
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// (`CompositingSpace::Srgb`, a diagnostic camera's; the world's is linear) the
// shaded colour is written sRGB-encoded, as Bevy's own sprite and mesh shaders
// write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
