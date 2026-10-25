// Generic character-body overlay: samples the source sprite's atlas frame
// and outputs a flat-tinted silhouette whose alpha is modulated by the
// `intensity` uniform. Used as a sibling-mesh overlay on top of the
// regular character sprite — when `intensity` falls to zero the
// overlay produces transparent fragments (discarded) and the source
// sprite renders normally underneath.
//
// Every cue shares it: the flashes, the blink, the charge, and the gate
// transit. The colour is a uniform because which cue is showing — and its
// priority — is decided on the CPU, in one place, from resolved simulation
// facts. The gate transit alone is more than a flat silhouette: the body
// turns to light in scanlines that slip sideways, and dissolves into it cell
// by cell, each cell's edge white as it goes (or condenses out of it, when
// the transit falls).

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// control.x = intensity (0..1 — 1.0 = a fully opaque tinted silhouette)
// control.y = flip_x flag (0 = no flip, >0.5 = mirror UV horizontally)
// control.z = gate transit (0 = a flat cue; 0..1 = how far into the light)
// control.w = the transit's clock, seconds
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var color_sampler: sampler;
// tint.rgb = silhouette colour; tint.a is unused.
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> tint: vec4<f32>;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let intensity = clamp(control.x, 0.0, 1.0);
    if intensity <= 0.001 {
        discard;
    }
    // Map the mesh's [0,1] quad UV into the current atlas frame's
    // sub-rect, honoring the horizontal flip flag so the overlay
    // tracks the source sprite's facing direction.
    var local_uv = mesh.uv;
    if control.y > 0.5 {
        local_uv.x = 1.0 - local_uv.x;
    }
    let transit = clamp(control.z, 0.0, 1.0);
    let seconds = control.w;
    if transit > 0.0 {
        // Each scanline slips sideways, re-chosen 24 times a second.
        let row = floor(mesh.uv.y * 48.0);
        local_uv.x = local_uv.x + (hash2(vec2<f32>(row, floor(seconds * 24.0))) - 0.5) * 0.1 * transit;
    }
    let atlas_uv = mix(
        uv_rect.xy,
        uv_rect.zw,
        clamp(local_uv, vec2<f32>(0.0), vec2<f32>(1.0)),
    );
    let sample = textureSample(color_texture, color_sampler, atlas_uv);
    // Carve the silhouette: transparent pixels in the source contribute
    // nothing — without this the overlay would draw a solid white
    // rectangle, blanking out the air around the character.
    if sample.a <= 0.01 {
        discard;
    }
    if transit > 0.0 {
        // A cell goes when its noise falls under the threshold: none at 0,
        // all at 1. The band just above the threshold is the edge.
        let edge = 0.14;
        let n = hash2(floor(mesh.uv * vec2<f32>(14.0, 28.0)));
        let threshold = transit * (1.0 + edge) - edge;
        if n < threshold {
            discard;
        }
        let rim = 1.0 - smoothstep(0.0, edge, n - threshold);
        let scan = 0.5 + 0.5 * sin(mesh.uv.y * 90.0 - seconds * 14.0);
        let light = mix(tint.rgb, vec3<f32>(1.0), clamp(rim + 0.25 * scan, 0.0, 1.0));
        return vec4<f32>(light, sample.a * intensity * clamp(0.45 + 0.35 * scan + rim, 0.0, 1.0));
    }
    // Flat tint, alpha-masked by the source pixel and modulated by
    // intensity. Pre-multiplied alpha would be `vec4(intensity*a, ...)`,
    // but Bevy's 2D blend pipeline expects straight alpha here.
    return vec4<f32>(tint.rgb, sample.a * intensity);
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
