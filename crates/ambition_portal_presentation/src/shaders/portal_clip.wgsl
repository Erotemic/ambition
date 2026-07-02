// Portal-clipped body piece: samples the source sprite's atlas frame (same
// UV mapping as the hit-flash overlay) and discards every fragment behind any
// active world-space clip half-plane. This is the texture-accurate version of
// the AABB piece decomposition in `ambition_portal::pieces` — the `here`
// slice is the sprite clipped to the front of the entry plane, the `through`
// slice is the exit copy clipped to the front of the exit plane (plus the
// exit aperture span), so the two slices tile continuously across the seam
// while a body straddles a portal pair.
//
// Clipping happens in RENDER-world space (Bevy 2D, y-up), entirely
// independent of atlas trim rects, anchors, flips, and rotations — the mesh
// pose handles those, the plane test only sees final world positions.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// control.x = flip_x flag (0 = no flip, >0.5 = mirror UV horizontally)
// control.y/z/w = reserved
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
// Straight-alpha sprite tint (linear RGBA), multiplied into the sample.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> tint: vec4<f32>;
// Clip half-planes as (point.xy, normal.xy) in render-world coordinates.
// Fragments with dot(p - point, normal) < 0 are discarded. A zero normal
// disables the plane (WebGL2-friendly: plain vec4 uniforms, no arrays).
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> clip0: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> clip1: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<uniform> clip2: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var color_sampler: sampler;

fn behind_plane(plane: vec4<f32>, p: vec2<f32>) -> bool {
    // Zero-length normal = plane disabled.
    if dot(plane.zw, plane.zw) < 0.5 {
        return false;
    }
    return dot(p - plane.xy, plane.zw) < 0.0;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Sample BEFORE any position-dependent discard: implicit-derivative
    // texture sampling requires uniform control flow.
    var local_uv = mesh.uv;
    if control.x > 0.5 {
        local_uv.x = 1.0 - local_uv.x;
    }
    let atlas_uv = mix(
        uv_rect.xy,
        uv_rect.zw,
        clamp(local_uv, vec2<f32>(0.0), vec2<f32>(1.0)),
    );
    let sample = textureSample(color_texture, color_sampler, atlas_uv);

    let p = mesh.world_position.xy;
    if behind_plane(clip0, p) || behind_plane(clip1, p) || behind_plane(clip2, p) {
        discard;
    }
    if sample.a <= 0.01 {
        discard;
    }
    return sample * tint;
}
