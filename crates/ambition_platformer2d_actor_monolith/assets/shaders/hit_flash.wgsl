// Generic character-body overlay: samples the source sprite's atlas frame
// and outputs a flat-tinted silhouette whose alpha is modulated by the
// `intensity` uniform. Used as a sibling-mesh overlay on top of the
// regular character sprite — when `intensity` falls to zero the
// overlay produces transparent fragments (discarded) and the source
// sprite renders normally underneath.
//
// Two cues share it: the white damage flash and the intangibility blink.
// The colour is a uniform because which cue is showing — and its priority —
// is decided on the CPU, in one place, from resolved simulation facts.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// control.x = intensity (0..1 — 1.0 = a fully opaque tinted silhouette)
// control.y = flip_x flag (0 = no flip, >0.5 = mirror UV horizontally)
// control.z = reserved
// control.w = reserved
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var color_sampler: sampler;
// tint.rgb = silhouette colour; tint.a is unused.
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> tint: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
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
    // Flat tint, alpha-masked by the source pixel and modulated by
    // intensity. Pre-multiplied alpha would be `vec4(intensity*a, ...)`,
    // but Bevy's 2D blend pipeline expects straight alpha here.
    return vec4<f32>(tint.rgb, sample.a * intensity);
}
