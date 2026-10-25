// The wormhole of a gate, at Ultra. See `gate_portal_fx.rs`.
//
// The membrane is a sprite. This shader draws on it from one more picture of
// the sprite submodule, the tunnel (`interdimensional_gate_tunnel.png`, which
// tiles both ways: across is the way round, down is the way in), streamed
// down a twisting tunnel toward a light at its far end. It draws no art of
// its own, and shows only in the dark of the membrane's own frame (the void
// of the triangle door), so it opens, closes and turns with the authored
// sheet; the door's lines, its lip and its halo stay the art's.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// The membrane's current frame on its sheet: (min.x, min.y, max.x, max.y).
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// x: strength, 0 to 1. y: 1 when the membrane is flipped. z: how far through
// a passage's wave. w: its way, 1 coming out, -1 going in, 0 none.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var membrane_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var membrane_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var tunnel_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var tunnel_sampler: sampler;

const TAU: f32 = 6.28318530718;
// How fast the tunnel streams in, and how hard it twists with depth.
const INFLOW: f32 = 0.55;
const TWIST: f32 = 0.12;
const CORE: vec3<f32> = vec3<f32>(0.75, 0.95, 1.0);
const CREST: vec3<f32> = vec3<f32>(0.6, 0.95, 1.0);
// The aperture's radius over the frame's half width (the ring's 39.5 of 96
// design units): the tunnel is measured from it.
const APERTURE: f32 = 0.411;

fn tunnel(uv: vec2<f32>) -> vec3<f32> {
    // Sampled at its full size: the wrap at the seam has no derivative to
    // pick a level from.
    return textureSampleLevel(tunnel_texture, tunnel_sampler, fract(uv), 0.0).rgb;
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let strength = clamp(control.x, 0.0, 1.0);
    if strength <= 0.001 {
        discard;
    }
    var local_uv = mesh.uv;
    if control.y > 0.5 {
        local_uv.x = 1.0 - local_uv.x;
    }
    let membrane = textureSample(membrane_texture, membrane_sampler, mix(uv_rect.xy, uv_rect.zw, local_uv));
    // Inside the membrane only, and only in its dark: the door's lines, its
    // lip and its halo are the art's.
    let lum = dot(membrane.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let inside = smoothstep(0.8, 0.97, membrane.a) * smoothstep(0.12, 0.03, lum);
    if inside <= 0.001 {
        discard;
    }
    let t = globals.time;
    let p = (mesh.uv - vec2<f32>(0.5)) * 2.0;
    var r = length(p) / APERTURE;
    // A passage's wave: a ring that runs out from the middle and pushes the
    // tunnel as it goes, in for a body going in, out for one coming out.
    let k = clamp(control.z, 0.0, 1.0);
    var crest = 0.0;
    if abs(control.w) > 0.5 {
        let front = k * 1.1;
        crest = exp(-pow((r - front) / 0.12, 2.0)) * (1.0 - k);
        r = max(0.0, r - control.w * 0.08 * crest);
    }
    let depth = 0.32 / (r + 0.06) - t * INFLOW;
    let round = atan2(p.y, p.x) / TAU;
    let near = tunnel(vec2<f32>(round + TWIST * depth + t * 0.03, depth));
    let far = tunnel(vec2<f32>(round * 2.0 + 0.37 - TWIST * depth, depth * 0.5 + t * 0.1));
    var colour = near * 0.7 + far * 0.45;
    // The far end of the tunnel: a light at the vanishing point.
    colour = colour + CORE * exp(-r * r * 14.0) * 0.9;
    // Darker toward the lip, so the tunnel reads as deep.
    let fog = smoothstep(0.95, 0.35, r);
    colour = colour * (0.35 + 0.65 * fog) + CREST * crest * 0.8;
    return vec4<f32>(colour, inside * strength * clamp(0.55 + 0.4 * fog + crest * 0.3, 0.0, 1.0));
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
