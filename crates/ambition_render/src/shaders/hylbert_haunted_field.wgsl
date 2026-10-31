// The haunted floors of Hylbert's Hotel: the room's ambiance, at Ultra (Jon,
// 2026-10-31: "a shader for the spectral level itself to give it ambiance.
// It would be nice if it also had a subtle matrix digital rain like effect
// as part of it").
//
// NOT YET WIRED: a whole-screen pass over the camera's picture (as
// `spirit_field.wgsl`), on while the camera is in a haunted room. The rooms
// must look good without it (their art is authored); this only adds mood.
//
// - COLD GRADE: the picture a little desaturated, its shadows pushed to
//   violet and its blacks lifted toward teal, a vignette closing in.
// - FOG: two layers of slow drifting fog, teal, thicker toward the floor of
//   the view, anchored to the world so it parallaxes as the camera moves.
// - DIGITAL RAIN: columns of falling glyphs (procedural 3x5 symbols, a new
//   one each step, the head bright, the trail fading), faint and only where
//   the picture is dark, so it haunts the walls and never covers play.
// - THE SPECTRAL GAP: near the gap (the hole that steadies the room), the
//   rain slows to a crawl, the fog stops churning and settles into faint
//   level lines: everything near it holds still.
// - FLICKER: now and then the whole room dims for a beat, as a ghost
//   passes the lights.
//
// Settings (one uniform struct, as the spirit field's):
// - control: x strength 0..1, y view width over height, z seconds,
//   w rain strength 0..1.
// - view: the camera's view in its target (x, y top left; z, w size; each a
//   part of the target).
// - world: x, y the camera's world position in view heights (so fog and
//   rain stay put as it moves); z the spectral gap's x in the view (0..1),
//   or below 0 for none; w the gap's reach in view heights.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct HauntedFieldSettings {
    control: vec4<f32>,
    view: vec4<f32>,
    world: vec4<f32>,
}

@group(0) @binding(2) var<uniform> settings: HauntedFieldSettings;

const TEAL: vec3<f32> = vec3<f32>(0.36, 0.93, 0.84);
const VIOLET: vec3<f32> = vec3<f32>(0.42, 0.26, 0.62);
const RAIN: vec3<f32> = vec3<f32>(0.18, 0.95, 0.66);
const RAIN_HEAD: vec3<f32> = vec3<f32>(0.55, 1.0, 0.82);
// Rain: columns across a view height, glyph rows down it, steps a second.
const COLUMNS: f32 = 38.0;
const ROWS: f32 = 26.0;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var k = 0; k < 4; k = k + 1) {
        v = v + a * noise(q);
        q = q * 2.07 + vec2<f32>(3.1, 7.7);
        a = a * 0.5;
    }
    return v;
}

// One glyph of the rain: a 3x5 grid of bits from `seed`, lit where `cell`
// (0..1 within the glyph's cell) falls on a set bit. A margin round it keeps
// the glyphs apart.
fn glyph(cell: vec2<f32>, seed: f32) -> f32 {
    let inner = (cell - vec2<f32>(0.18, 0.1)) / vec2<f32>(0.64, 0.8);
    if inner.x < 0.0 || inner.y < 0.0 || inner.x >= 1.0 || inner.y >= 1.0 {
        return 0.0;
    }
    let bit = floor(inner * vec2<f32>(3.0, 5.0));
    // A symmetric glyph reads as a symbol, not noise: mirror the columns.
    let mirrored = bit.y * 3.0 + min(bit.x, 2.0 - bit.x);
    let on = step(0.45, hash(vec2<f32>(seed, mirrored)));
    // The middle column always has its middle bit: every glyph has a spine.
    let spine = step(0.5, 1.0 - abs(bit.x - 1.0)) * step(abs(bit.y - 2.0), 0.5);
    return max(on, spine);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let picture = textureSample(screen_texture, texture_sampler, in.uv);
    let at = (in.uv - settings.view.xy) / max(settings.view.zw, vec2<f32>(0.0001));
    if at.x < 0.0 || at.y < 0.0 || at.x > 1.0 || at.y > 1.0 {
        return picture;
    }
    let strength = clamp(settings.control.x, 0.0, 1.0);
    if strength <= 0.001 {
        return picture;
    }
    let aspect = settings.control.y;
    let seconds = settings.control.z;
    let rain_strength = clamp(settings.control.w, 0.0, 1.0);
    // The view in view heights, anchored to the world.
    let world = vec2<f32>(at.x * aspect, at.y) + settings.world.xy;

    // The spectral gap's calm: 1 at the gap, 0 beyond its reach.
    var calm = 0.0;
    if settings.world.z >= 0.0 {
        let dx = (at.x - settings.world.z) * aspect;
        calm = 1.0 - smoothstep(0.0, max(settings.world.w, 0.01), abs(dx));
    }
    let churn = 1.0 - 0.9 * calm;

    // COLD GRADE.
    var colour = picture.rgb;
    let lum = dot(colour, vec3<f32>(0.2126, 0.7152, 0.0722));
    colour = mix(colour, vec3<f32>(lum), 0.18 * strength);
    let shadow = 1.0 - smoothstep(0.0, 0.45, lum);
    colour = colour + VIOLET * shadow * 0.08 * strength + TEAL * 0.015 * strength;
    let edge = at - vec2<f32>(0.5);
    let vignette = smoothstep(0.85, 0.25, length(edge * vec2<f32>(1.0, 1.25)));
    colour = colour * mix(1.0, 0.72 + 0.28 * vignette, strength);

    // FOG: two drifting layers, thicker low in the view; still near the gap.
    let drift = seconds * 0.04 * churn;
    let fog_a = fbm(world * 2.2 + vec2<f32>(drift, -drift * 0.4));
    let fog_b = fbm(world * 4.5 - vec2<f32>(drift * 1.7, 0.0));
    let low = smoothstep(0.35, 1.0, at.y);
    var fog = (fog_a * 0.65 + fog_b * 0.35) * (0.25 + 0.75 * low);
    // Near the gap the fog lies in level lines: the room held steady.
    let level = 0.5 + 0.5 * sin(world.y * 120.0);
    fog = mix(fog, fog * 0.6 + level * 0.12 * low, calm);
    colour = colour + TEAL * smoothstep(0.35, 0.85, fog) * 0.26 * strength;

    // DIGITAL RAIN: in the dark only.
    let dark = 1.0 - smoothstep(0.08, 0.35, lum);
    if rain_strength > 0.001 && dark > 0.001 {
        let column = floor(world.x * COLUMNS);
        let speed = (0.6 + hash(vec2<f32>(column, 1.0)) * 1.4) * (1.0 - 0.85 * calm);
        let row_f = world.y * ROWS;
        let fall = seconds * speed * 6.0 + hash(vec2<f32>(column, 2.0)) * 100.0;
        // The head falls down the column; the trail fades behind it.
        let trail_len = 6.0 + hash(vec2<f32>(column, 3.0)) * 14.0;
        let lag = fract((fall - row_f) / 40.0) * 40.0;
        let trail = clamp(1.0 - lag / trail_len, 0.0, 1.0);
        let row = floor(row_f);
        let step_i = floor(seconds * (2.0 + hash(vec2<f32>(column, row)) * 3.0) * (1.0 - 0.9 * calm));
        let cell = vec2<f32>(fract(world.x * COLUMNS), fract(row_f));
        let lit = glyph(cell, hash(vec2<f32>(column * 7.0 + row, step_i)));
        let head = smoothstep(1.2, 0.0, lag);
        // Only some columns rain at any time.
        let raining = step(0.55, hash(vec2<f32>(column, floor(seconds * 0.05 + hash(vec2<f32>(column, 9.0))))));
        let rain = lit * trail * raining * dark * rain_strength * strength;
        colour = colour + mix(RAIN, RAIN_HEAD, head) * rain * (0.07 + 0.16 * head);
    }

    // FLICKER: now and then a beat of dimming, as a ghost passes the lights.
    let beat = floor(seconds * 1.5);
    let flicker = step(0.97, hash(vec2<f32>(beat, 5.0))) * (0.5 + 0.5 * sin(seconds * 40.0));
    colour = colour * (1.0 - 0.18 * flicker * strength);

    return vec4<f32>(colour, picture.a);
}
