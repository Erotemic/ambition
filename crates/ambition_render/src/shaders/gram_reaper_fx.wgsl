// The Gram Reaper's presence, at Ultra (Jon, 2026-10-31: "a shader effect
// for gram reaper to make it more visually interesting").
//
// NOT YET WIRED: a Material2d for the boss's sprite (the `gram_reaper`
// sheet), drawn in place of its plain sprite. It reworks the sprite's own
// pixels (it paints no new figure) and draws the sheet as authored at
// strength 0, so with shaders off the boss loses only this motion.
//
// - SMOKE HEM: the lower robe burns away into drifting smoke: a flowing,
//   domain-warped noise eats its alpha upward from the hem, the eaten edge
//   glowing violet like embers, the smoke rising over time.
// - GALAXY FACE: the void inside its hood is not empty: where the sprite is
//   near black inside the hood's circle, a slow turning field of stars and a
//   violet nebula show, as if the hood opens on deep space.
// - EYE FLARE: its eyes bloom, a soft glow and a long horizontal flare
//   streaking from them, beating with its fury.
// - LIVING CONSTELLATION: the graph round it lives: pulses of light run
//   along its edges (its bright violet lines), its nodes throb, each on its
//   own clock.
//
// Uniforms (plain vec4s, as the workspace's other overlays):
// - uv_rect: the current frame on its sheet (min.x, min.y, max.x, max.y).
// - control: x fury 0..1 (menace at 0.3, the strike and enrage at 1), y 1
//   when flipped, z smoke 0..1 (the hem's burn; higher in vanish and
//   death), w a seed.
// - face: xy the hood's centre in the frame (0..1, y down; about 0.52, 0.27
//   on the idle frame), z the hood's radius (about 0.06), w the eyes' glow.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> face: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var sprite_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var sprite_sampler: sampler;

const TAU: f32 = 6.28318530718;
const VIOLET: vec3<f32> = vec3<f32>(0.69, 0.5, 1.0);
const EMBER: vec3<f32> = vec3<f32>(0.85, 0.55, 1.0);
const NODE: vec3<f32> = vec3<f32>(0.93, 0.88, 1.0);
const NEBULA: vec3<f32> = vec3<f32>(0.35, 0.12, 0.6);
// Where the hem starts to burn (frame UV, y down) and how far up the burn
// can reach at full smoke.
const HEM_TOP: f32 = 0.72;
const BURN_REACH: f32 = 0.22;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// Smooth value noise.
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

// Four octaves of it.
fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var k = 0; k < 4; k = k + 1) {
        v = v + a * noise(q);
        q = q * 2.03 + vec2<f32>(1.7, 9.2);
        a = a * 0.5;
    }
    return v;
}

fn sample_frame(local: vec2<f32>) -> vec4<f32> {
    let inside = clamp(local, vec2<f32>(0.001), vec2<f32>(0.999));
    return textureSample(sprite_texture, sprite_sampler, mix(uv_rect.xy, uv_rect.zw, inside));
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    var local = mesh.uv;
    if control.y > 0.5 {
        local.x = 1.0 - local.x;
    }
    let fury = clamp(control.x, 0.0, 1.0);
    let smoke = clamp(control.z, 0.0, 1.0);
    let t = globals.time + control.w * 17.3;
    let base = sample_frame(local);
    var colour = base.rgb;
    var alpha = base.a;
    let lum = dot(base.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));

    // SMOKE HEM: a rising, warped noise burns the robe away from the hem.
    let rise = vec2<f32>(local.x * 6.0, local.y * 6.0 + t * 0.35);
    let warp = vec2<f32>(fbm(rise + vec2<f32>(t * 0.1, 0.0)), fbm(rise + vec2<f32>(5.2, t * 0.07)));
    let n = fbm(rise + warp * 1.6);
    let reach = HEM_TOP - (0.05 + BURN_REACH * smoke);
    let depth = clamp((local.y - reach) / (1.0 - reach), 0.0, 1.0);
    let burn = depth * (0.9 + 0.6 * smoke) - n * 0.75;
    let eaten = smoothstep(0.05, 0.2, burn);
    let ember = smoothstep(0.0, 0.08, burn) * (1.0 - eaten) * base.a;
    alpha = alpha * (1.0 - eaten);
    colour = colour + EMBER * ember * (0.8 + 0.6 * fury);
    // Wisps of smoke curling above the burnt edge.
    let wisp = smoothstep(0.55, 0.85, fbm(rise * 1.7 - warp)) * depth * (1.0 - base.a * (1.0 - eaten));
    // Smoke glows violet where it leaves the robe and darkens as it rises.
    colour = colour + mix(VIOLET * 0.35, VIOLET * 0.1, smoothstep(0.0, 0.6, 1.0 - depth)) * wisp;
    alpha = max(alpha, wisp * 0.3 * (0.5 + smoke));

    // GALAXY FACE: stars and nebula in the dark of the hood.
    let to_face = (local - face.xy) / max(face.z, 1e-3);
    let in_hood = smoothstep(1.0, 0.6, length(to_face)) * smoothstep(0.08, 0.02, lum) * base.a;
    if in_hood > 0.001 {
        let a = t * 0.15;
        let spun = vec2<f32>(to_face.x * cos(a) - to_face.y * sin(a), to_face.x * sin(a) + to_face.y * cos(a));
        let cell = floor(spun * 9.0);
        let twinkle = 0.5 + 0.5 * sin(t * (1.0 + hash(cell) * 3.0) + hash(cell + 4.0) * TAU);
        let in_cell = fract(spun * 9.0) - 0.5;
        // Stars, the brightest with a small cross of light.
        let core_star = smoothstep(0.35, 0.0, length(in_cell));
        let cross = (exp(-abs(in_cell.x) * 40.0) + exp(-abs(in_cell.y) * 40.0)) * smoothstep(0.5, 0.0, length(in_cell)) * 0.4;
        let star = step(0.9, hash(cell)) * (core_star + cross * step(0.97, hash(cell))) * twinkle;
        // A spiral arm through the nebula.
        let r = length(spun);
        let arm = 0.5 + 0.5 * sin(atan2(spun.y, spun.x) * 2.0 - log(r + 0.05) * 4.0 + t * 0.2);
        let nebula = fbm(spun * 2.5 + vec2<f32>(t * 0.05, 0.0)) * (0.6 + 0.6 * arm);
        colour = mix(colour, NEBULA * nebula * 1.4 + NODE * star, in_hood * 0.9);
    }

    // EYE FLARE: a glow round its eyes and a long horizontal flare.
    let eyes = face.xy + vec2<f32>(face.z * 0.15, -face.z * 0.05);
    let d = local - eyes;
    let beat = 0.75 + 0.25 * sin(t * (1.5 + 4.0 * fury) * TAU);
    let glow = exp(-dot(d, d) / (face.z * face.z * 0.25)) * 0.5;
    let flare = exp(-pow(d.y / (face.z * 0.04), 2.0)) * exp(-abs(d.x) / (face.z * (1.2 + 1.8 * fury))) * 0.7;
    let light = (glow + flare) * face.w * beat;
    // Two-tone: a white-hot core to the flare, violet out along it.
    colour = colour + VIOLET * light + NODE * flare * face.w * beat * 0.4 * exp(-abs(d.x) / (face.z * 0.5));
    alpha = max(alpha, clamp(light, 0.0, 1.0) * 0.8);

    // LIVING CONSTELLATION: pulses along the bright violet lines; nodes throb.
    let violet = smoothstep(0.35, 0.7, lum) * smoothstep(0.05, 0.25, base.b - base.g) * base.a;
    // Pulses flow OUT from the hood along the graph: energy radiating.
    let pulse = pow(0.5 + 0.5 * sin(length(local - face.xy) * 55.0 - t * 6.0), 8.0);
    colour = colour + NODE * violet * pulse * (0.4 + 0.6 * fury);
    let node = smoothstep(0.85, 0.95, lum) * base.a;
    let throb = 0.5 + 0.5 * sin(t * 3.0 + hash(floor(local * 60.0)) * TAU);
    colour = colour + VIOLET * node * throb * 0.4;

    // A violet BACKLIGHT along its silhouette, strongest on its back (left).
    let e = 0.006;
    let around = sample_frame(local + vec2<f32>(e, 0.0)).a + sample_frame(local - vec2<f32>(e, 0.0)).a
        + sample_frame(local + vec2<f32>(0.0, e)).a + sample_frame(local - vec2<f32>(0.0, e)).a;
    let rim = clamp(base.a - around * 0.25, 0.0, 1.0) * (1.0 - eaten);
    let back = smoothstep(0.65, 0.3, local.x);
    colour = colour + VIOLET * rim * (0.35 + 0.5 * back) * (0.7 + 0.3 * fury);

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
