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
// Four heads of the slug, two cells each way. It is sampled with `skin_sampler`.
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var faces_texture: texture_2d<f32>;

const PI: f32 = 3.141592653589793;

// THE DREAM IS MADE OF AUTHORED ART. This pass draws no art of its own: each
// colour it writes is a sample of the body's own frame or of two pictures of
// the slug's sprite target. `puppy_slug_dream_skin.png` is the SKIN of the
// dream: eyes in a lattice, a whorl of ripples round each, dots as scales.
// `puppy_slug_dream_faces.png` is its HEADS: four heads of the slug with
// nothing behind them. The sheet is good to look at with the pass off.
//
// What it must say (Jon, 2026-10-13): the pictures an image network dreamed in
// 2015 are fractal, things appear in them and disappear, the body molds and
// morphs, and you can not say where the head is at a given moment.
//
//   1. The flow: the art is sampled a way off, along a slow field, so the
//      body molds. Its inner lines go with it; only the line round the
//      outside of the body stays where it is.
//   2. The skin: the tile is laid over the body twice, large and fine, and it
//      goes with the flow. It takes the colour and the light of the art under
//      it, its eyes show whole, and its red and its blue are a little apart.
//      A veil goes slowly over the body, and under the veil the skin is all
//      that shows: the art comes and goes.
//   3. The heads: the heads of the slug, in grids at three sizes that all
//      grow, about the middle of the body. A head comes in small, grows, and
//      goes out large while a smaller one comes in: there is no largest and no
//      smallest. Each has a turn and a clock of its own. A head whose middle is
//      on the body is drawn whole, so a head can stand out of the body's edge.
//   4. The oil: the hue turns along the flow, and thin bright bands follow the
//      lines of equal light.
//
// Nothing is drawn where the sprite and the heads are both clear.

// How far the art is moved by the flow, as a part of the frame.
const FLOW_REACH: f32 = 0.055;
// How much of the skin shows (its own alpha is 1 at an eye), large and fine;
// how far it takes the colour of the art under it; how far its red and its
// blue are apart, in tiles.
const SKIN_ALPHA: f32 = 0.86;
const SKIN_FINE_ALPHA: f32 = 0.22;
const SKIN_DYE: f32 = 0.7;
const SKIN_FRINGE: f32 = 0.008;
// The heads. `HEAD_CELL`: how wide the cell of the smallest head is, in tiles
// of the skin (a head is half of its cell). `HEAD_GROWTH`: how many times it
// doubles in a second. `HEAD_ALPHA`: how much of a head shows at its most.
const HEAD_LAYERS: i32 = 3;
const HEAD_CELL: f32 = 0.2;
const HEAD_GROWTH: f32 = 0.13;
const HEAD_ALPHA: f32 = 0.92;
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

// The art at a frame coordinate; clear where the coordinate is off the frame.
fn sample_art(frame_uv: vec2<f32>) -> vec4<f32> {
    let art = textureSampleLevel(puppy_texture, puppy_sampler, atlas_uv(on_quad(frame_uv)), 0.0);
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

// The skin of the dream at a place in tiles. It repeats.
fn sample_skin(p: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(skin_texture, skin_sampler, fract(p), 0.0);
}

// The middle of the body in its frame: the heads grow about it.
const BODY_MIDDLE: vec2<f32> = vec2<f32>(0.5, 0.66);

// One grid of heads. `cell` is how wide a cell is in tiles; `who` names the
// grid (a grid keeps its name while it grows). A cell has a head or it does
// not; a head has a turn, a way it looks, one of the four heads, and a clock.
// It is drawn where the middle of its cell is on the body.
fn heads(local_uv: vec2<f32>, tiles: vec2<f32>, cell: f32, who: f32, time: f32, seed: f32) -> vec4<f32> {
    let p = (local_uv - BODY_MIDDLE) * tiles / cell + vec2<f32>(seed * 1.3 + who * 0.37, who * 0.61);
    let at = floor(p);
    let q = fract(p);
    let h = hash21(at + vec2<f32>(who * 13.7, seed));
    let h2 = hash21(at + vec2<f32>(seed + 5.1, who * 3.3));
    let there = step(0.36, h);
    let life = smoothstep(0.2, 0.7, 0.5 + 0.5 * sin(time * (0.5 + 0.9 * h2) + h * 40.0));
    let angle = (h2 - 0.5) * 1.4;
    let ca = cos(angle);
    let sa = sin(angle);
    let o = (q - vec2<f32>(0.5)) * 1.2;
    var r = vec2<f32>(ca * o.x - sa * o.y, sa * o.x + ca * o.y) + vec2<f32>(0.5);
    r.x = mix(r.x, 1.0 - r.x, step(0.5, fract(h * 7.0)));
    let low = step(vec2<f32>(0.0), r);
    let high = step(r, vec2<f32>(1.0));
    let which = floor(fract(h * 3.0) * 4.0);
    let corner = vec2<f32>(which % 2.0, floor(which / 2.0));
    let face = textureSampleLevel(faces_texture, skin_sampler, (corner + clamp(r, vec2<f32>(0.02), vec2<f32>(0.98))) * 0.5, 0.0);
    let middle_uv = BODY_MIDDLE + (at + vec2<f32>(0.5) - vec2<f32>(seed * 1.3 + who * 0.37, who * 0.61)) * cell / tiles;
    let on_body = step(0.5, sample_art(middle_uv).a);
    return vec4<f32>(hue_turn(face.rgb, (h - 0.5) * 1.6), face.a * low.x * low.y * high.x * high.y * there * life * on_body);
}

// What the dream makes of the body at a frame coordinate: its colour, and how
// much of it is there (the body, and a head that stands out of the body).
fn dream(local_uv: vec2<f32>, base: vec4<f32>, time: f32, strength: f32, seed: f32) -> vec4<f32> {
    let luma_of = vec3<f32>(0.299, 0.587, 0.114);

    // 1. The flow.
    let grain = vec2<f32>(3.4, 2.4);
    let f1 = fbm(local_uv * grain + vec2<f32>(time * 0.23 + seed, -time * 0.15));
    let f2 = fbm(local_uv * grain + vec2<f32>(11.3 - time * 0.19, 4.1 + time * 0.21 + seed * 2.3));
    let flow = (vec2<f32>(f1, f2) - vec2<f32>(0.5)) * FLOW_REACH * strength;
    let moved = sample_art(local_uv + flow);
    var rgb = mix(base.rgb, moved.rgb, moved.a);

    // The line round the outside of the body stays: it is where the art is
    // near black and the sprite is clear a little way off.
    let near = 0.035;
    let clear = 1.0 - min(
        min(sample_art(local_uv + vec2<f32>(near, 0.0)).a, sample_art(local_uv - vec2<f32>(near, 0.0)).a),
        min(sample_art(local_uv + vec2<f32>(0.0, near * 2.2)).a, sample_art(local_uv - vec2<f32>(0.0, near * 2.2)).a)
    );
    let black = 1.0 - smoothstep(BLACK_PRESERVE_LOW, BLACK_PRESERVE_HIGH, max(base.r, max(base.g, base.b)));
    let outline = black * smoothstep(0.1, 0.6, clear) * base.a;

    // 2. The skin.
    let tiles = max(skin_tiles.xy, vec2<f32>(0.25));
    let veil = smoothstep(0.5, 0.72, fbm(local_uv * vec2<f32>(2.6, 1.8) + vec2<f32>(time * 0.11 + seed * 3.0, -time * 0.07)));
    let place = (local_uv + flow * 1.6) * tiles
        + vec2<f32>(time * 0.02 + seed * 0.37, sin(time * 0.2) * 0.02 + seed * 0.61);
    let fringe = vec2<f32>(SKIN_FRINGE, -SKIN_FRINGE * 0.6) * strength;
    let skin = sample_skin(place);
    let skin_rgb = vec3<f32>(sample_skin(place + fringe).r, skin.g, sample_skin(place - fringe).b);
    let lit = 0.45 + 0.9 * dot(rgb, luma_of);
    let dyed = mix(skin_rgb, skin_rgb * (rgb * 1.6 + vec3<f32>(0.15)), SKIN_DYE * (1.0 - 0.6 * veil));
    let skin_shows = clamp(skin.a * mix(1.0, 2.6, veil), 0.0, 1.0);
    rgb = mix(rgb, dyed * mix(lit, 1.0, veil * 0.7), skin_shows * SKIN_ALPHA * strength);
    let fine = sample_skin(place * 2.3 + vec2<f32>(0.31, 0.17) - flow * tiles * 2.0);
    rgb = mix(rgb, mix(fine.rgb, fine.rgb * rgb * 1.8, 0.6), fine.a * SKIN_FINE_ALPHA * strength);

    // 3. The heads: the largest first, so a new small head is over an old large one.
    var cover = 0.0;
    let grown = time * HEAD_GROWTH;
    let turn_of = floor(grown);
    let part = fract(grown);
    for (var k = HEAD_LAYERS - 1; k >= 0; k = k - 1) {
        let level = f32(k) + part;
        let weight = sin(PI * level / f32(HEAD_LAYERS));
        let layer = heads(local_uv + flow * 2.0, tiles, HEAD_CELL * pow(2.0, level), f32(k) - turn_of, time, seed);
        let shows = layer.a * weight * weight * HEAD_ALPHA * strength;
        rgb = mix(rgb, layer.rgb, shows);
        cover = max(cover, shows);
    }

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

    rgb = mix(rgb, base.rgb, outline * (1.0 - cover));
    return vec4<f32>(rgb, max(base.a, cover));
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let time = control.x;
    let strength = clamp(control.z, 0.0, 1.0);
    let seed = control.w * 17.0;

    // Bevy's 2D `Rectangle` mesh and the atlas agree on +y down in a frame, so
    // `mesh.uv` is the sample with no flip of its own.
    let uv = frame_in_sprite::frame_uv(mesh.uv, frame_rect, flipped());
    let base = sample_art(uv);
    let dreamed = dream(uv, base, time, strength, seed);

    // Nothing where the sprite and the heads are both clear.
    if dreamed.a <= 0.010 {
        discard;
    }

    return vec4<f32>(
        clamp(dreamed.rgb * tint.rgb, vec3<f32>(0.0), vec3<f32>(1.6)),
        dreamed.a * strength * tint.a
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
