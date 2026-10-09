// The Mockingbird on fire: flames that come off its own drawn body.
//
// The quad is larger than the body's sprite, so the flames have room. Each
// pixel asks whether there is body below it and downwind of it: if there is,
// a flame from that body reaches it. The body itself is not covered: it
// takes a glow and a bright windward edge. The sky blows to the left, so the flames
// rise and trail left, whichever way the body faces. Red fire and cold fire
// are one field with two colour ramps.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// x: how high the flames are (0 none); y: x-flip flag; z: how cold the fire
// is (0 red, 1 blue); w: time in seconds.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
// x: the quad as a multiple of the sprite; y: the sprite's width / height.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> shape: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var body_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var body_sampler: sampler;

const STEPS: i32 = 18;
// How far a flame reaches, in sprite heights, at height 1.
const REACH: f32 = 0.42;
// How far it trails left for each unit it rises.
const LEAN: f32 = 0.85;

fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = p - i;
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    return noise(p) * 0.6 + noise(p * 2.1 + vec2<f32>(3.7, 1.3)) * 0.3 + noise(p * 4.3) * 0.1;
}

// The body's alpha at `s`, a point of the sprite as it is drawn (0..1 across
// the sprite, outside it nothing).
fn body(s: vec2<f32>) -> f32 {
    if (s.x < 0.0 || s.x > 1.0 || s.y < 0.0 || s.y > 1.0) {
        return 0.0;
    }
    var u = s.x;
    if (control.y > 0.5) {
        u = 1.0 - u;
    }
    let uv = vec2<f32>(mix(uv_rect.x, uv_rect.z, u), mix(uv_rect.y, uv_rect.w, s.y));
    return textureSampleLevel(body_texture, body_sampler, uv, 0.0).a;
}

fn shade(in: VertexOutput) -> vec4<f32> {
    let height = control.x;
    let cold = clamp(control.z, 0.0, 1.0);
    let t = control.w;
    let aspect = max(shape.y, 0.01);
    // The point of the sprite this pixel is at.
    let s = (in.uv - vec2<f32>(0.5)) * shape.x + vec2<f32>(0.5);
    // In square units, so a flame is as wide on a wide sprite as on a tall one.
    let sq = vec2<f32>(s.x * aspect, s.y);

    // The flame field: how near the nearest body below and downwind is. A
    // flame is strong at the body and gone at its reach.
    let reach = REACH * height;
    var near = 2.0;
    // Each pixel looks at its own offsets, which a smooth noise gives it, so
    // the steps of the search do not draw as bands.
    let offset = noise(sq * 46.0 + vec2<f32>(t * 3.0, t * 5.0));
    // A flame bends as it rises: its search bends with a slow noise.
    let bend = (fbm(sq * vec2<f32>(5.0, 3.0) + vec2<f32>(t * 1.7, t * 3.1)) - 0.5) * 0.16;
    for (var i = STEPS; i >= 1; i = i - 1) {
        let k = (f32(i) - offset) / f32(STEPS);
        let sway = sin(t * 6.0 + sq.y * 17.0 + f32(i) * 0.9) * 0.010 * k;
        let under = s + vec2<f32>((LEAN * reach * k + sway + bend * k) / aspect, reach * k);
        if (body(under) > 0.5) {
            near = k;
        }
    }
    let strength = clamp(1.0 - near, 0.0, 1.0);
    // Tongues: noise that travels up and to the left with the flames. Far
    // from the body only the noise's peaks are left, so a flame ends in
    // tongues and not in a line.
    let flow = fbm(sq * vec2<f32>(8.0, 5.0) + vec2<f32>(t * 2.6, t * 4.4));
    let lick = fbm(sq * vec2<f32>(19.0, 13.0) + vec2<f32>(t * 4.0, t * 7.5));
    let fuel = strength * (0.12 + 1.35 * flow + 0.45 * lick) - 0.36 - 0.24 * near;
    let on_body = body(s);
    // Off the body only: the body is not hidden by its own fire.
    let flame = smoothstep(0.0, 0.34, fuel) * (1.0 - on_body) * step(near, 1.5);

    // The body takes the fire's glow, and its windward edge is bright.
    let windward = on_body * (1.0 - body(s - vec2<f32>(LEAN * 0.03 / aspect, 0.03)));
    let lit = min(height, 1.0);
    let glow = on_body * (0.10 + 0.08 * flow) * lit;
    let rim = windward * (0.45 + 0.35 * lick) * lit;
    let alpha = clamp(flame * 0.94 + glow + rim, 0.0, 1.0);
    // How hot this pixel is drawn: the core at the body, the edge at the tip.
    let hot = clamp(max(flame * pow(strength, 1.5) * 1.15, rim), 0.0, 1.0);

    let red = mix(
        mix(vec3<f32>(0.62, 0.03, 0.01), vec3<f32>(1.0, 0.36, 0.03), smoothstep(0.0, 0.5, hot)),
        vec3<f32>(1.0, 0.84, 0.34),
        smoothstep(0.60, 1.0, hot),
    );
    let blue = mix(
        mix(vec3<f32>(0.02, 0.05, 0.55), vec3<f32>(0.05, 0.36, 1.0), smoothstep(0.0, 0.5, hot)),
        vec3<f32>(0.42, 0.86, 1.0),
        smoothstep(0.60, 1.0, hot),
    );
    return vec4<f32>(mix(red, blue, cold), alpha);
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// the shaded colour is written sRGB-encoded, as Bevy's own sprite and mesh
// shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
