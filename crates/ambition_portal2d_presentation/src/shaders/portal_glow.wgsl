// A portal as a thin line of light along its opening. See `glow.rs`.
//
// The quad's x is along the opening and its +y is toward the room (the side a
// body goes in from). The line has a bright core, a glow that is wider on the
// room side, a node at each end of the opening, a pulse that runs from the
// ends to the middle, and faint streaks drawn into it from the room side.
//
// `phase.x` opens it from its middle (a portal that was added), and `phase.y`
// breaks it up and closes it (a portal that was removed). All lengths are px.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// x: the opening's length; y, z: the quad's size along and across; w: a seed.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> shape: vec4<f32>;
// x: how far it has opened (0 to 1); y: how far it has dissolved (0 to 1).
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> phase: vec4<f32>;
// The colour of the room side, and of the other side (linear).
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> front: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> back: vec4<f32>;

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

// 0 to 1, a little past 1 on the way: the line snaps open.
fn ease_out_back(x: f32) -> f32 {
    let c = 1.70158;
    let y = x - 1.0;
    return 1.0 + (c + 1.0) * y * y * y + c * y * y;
}

fn shade(in: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    let seed = shape.w;
    let p = vec2<f32>((in.uv.x - 0.5) * shape.y, (0.5 - in.uv.y) * shape.z);
    let appear = clamp(phase.x, 0.0, 1.0);
    let gone = clamp(phase.y, 0.0, 1.0);
    let fresh = 1.0 - appear;

    // The line opens from its middle, and closes to its middle at the end of
    // its dissolve.
    let whole = 0.5 * shape.x;
    let half = whole * ease_out_back(appear) * (1.0 - smoothstep(0.6, 1.0, gone));
    let past = max(abs(p.x) - half, 0.0);
    let d = length(vec2<f32>(past, p.y));
    let in_span = 1.0 - step(half, abs(p.x));
    let room_side = smoothstep(-1.0, 1.0, p.y);
    let tint = mix(back.rgb, front.rgb, room_side);

    // A dissolving line breaks up: the low places of a noise go first.
    let grain = noise(vec2<f32>(p.x * 0.33 + seed * 31.0, seed * 7.0));
    // The line itself breaks; its glow only thins, so no hard column of dark
    // is cut out of it.
    let holds = smoothstep(gone * 1.25 - 0.25, gone * 1.25, grain);
    let out = 1.0 - gone * gone;
    let fade = holds * out;
    let haze = mix(1.0, holds, 0.35) * out;

    // The core, and a glow that is wider on the room side. A fresh line and a
    // dissolving one glow wider.
    let flicker = 0.92 + 0.08 * sin(t * 9.0 + seed * 6.28 + p.x * 0.21);
    let core = exp(-d * d / 1.3) * flicker;
    // The glow ends before the quad does: no edge of the quad is drawn.
    let support = clamp(1.0 - d / (0.5 * shape.z - 4.0), 0.0, 1.0);
    let reach = mix(2.6, 6.0, room_side) * (1.0 + 2.4 * fresh + 1.6 * gone);
    let glow = exp(-d / reach) * 0.78 * support * support;

    // A node at each end: where the opening stops.
    let end_d = length(vec2<f32>(abs(p.x) - half, p.y));
    let nodes = exp(-end_d * end_d / 7.0) * (0.85 + 2.5 * fresh);

    // A pulse that runs from the two ends to the middle.
    let along = abs(p.x) / max(half, 1.0);
    let lap = fract(along * 1.4 + t * 0.5 + seed);
    let pulse = smoothstep(0.0, 0.07, lap) * (1.0 - smoothstep(0.07, 0.34, lap));
    let pulses = pulse * exp(-abs(p.y) / 2.6) * in_span * 0.5;

    // The entrance: faint streaks drawn into the line from the room side.
    let streak = noise(vec2<f32>(p.x * 0.55 + seed * 17.0, (p.y + t * 22.0) * 0.07));
    let streaks = smoothstep(0.55, 0.95, streak) * in_span * step(0.0, p.y)
        * smoothstep(0.0, 2.0, p.y) * exp(-p.y / 7.0) * 0.26 * appear * support;

    // Placed: a flare across the line at its middle, which goes as it opens.
    let flare = exp(-abs(p.x) / 2.4) * exp(-abs(p.y) / 13.0) * fresh * fresh * 1.6;

    // Dissolved: bits of the line lift off to both sides and go out.
    let cell = floor(p.x / 7.0);
    let lift = hash(vec2<f32>(cell, seed * 53.0));
    let which = step(0.35, hash(vec2<f32>(cell + 9.0, seed * 11.0))) * 2.0 - 1.0;
    let bit_at = vec2<f32>((cell + 0.5) * 7.0, which * gone * (8.0 + 30.0 * lift));
    let bit_d = length(p - bit_at);
    let bits = exp(-bit_d * bit_d / 2.4) * smoothstep(0.0, 0.2, gone) * (1.0 - gone)
        * (1.0 - step(whole, abs(p.x))) * 0.9;

    let soft = (glow + streaks) * haze + pulses * fade + bits;
    let hot = (core + nodes) * fade + flare;
    let white = vec3<f32>(1.0, 1.0, 1.0);
    let total = soft + hot;
    let colour = (tint * soft + mix(tint, white, 0.5) * hot) / max(total, 0.0001);
    return vec4<f32>(colour, clamp(total, 0.0, 1.0));
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
