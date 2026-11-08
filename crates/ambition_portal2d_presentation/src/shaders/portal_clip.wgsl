// The mesh contains the complete clipped material region.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// control.x = flip_x flag (0 = no flip, >0.5 = mirror UV horizontally)
// control.y = flip_y flag (0 = no flip, >0.5 = mirror UV vertically)
// control.z = silhouette flag (>0.5 = paint tint.rgb masked by the sample's
//             alpha and tint.a, the hit-flash overlay's look, instead of the
//             sampled colour). A declared non-sprite drawable is what sets it.
// control.w = premultiplied flag (>0.5: the texture is a capture whose colour
//             is premultiplied by its alpha; it is divided out)
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
// Straight-alpha sprite tint (linear RGBA), multiplied into the sample.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var color_sampler: sampler;

fn shade(mesh: VertexOutput) -> vec4<f32> {
    // Sample BEFORE any position-dependent discard: implicit-derivative
    // texture sampling requires uniform control flow.
    var local_uv = mesh.uv;
    if control.x > 0.5 {
        local_uv.x = 1.0 - local_uv.x;
    }
    if control.y > 0.5 {
        local_uv.y = 1.0 - local_uv.y;
    }
    let atlas_uv = mix(
        uv_rect.xy,
        uv_rect.zw,
        clamp(local_uv, vec2<f32>(0.0), vec2<f32>(1.0)),
    );
    let sample = textureSample(color_texture, color_sampler, atlas_uv);

    if sample.a <= 0.01 {
        discard;
    }
    // A capture over a clear of nothing holds colour premultiplied by its
    // cover: divided out, it blends as a sprite does.
    var colour = sample;
    if control.w > 0.5 {
        colour = vec4<f32>(sample.rgb / sample.a, sample.a);
    }
    if control.z > 0.5 {
        // Silhouette: the same output the hit-flash shader produces, so a
        // clipped piece of a flashing body looks like the flash it replaces.
        if tint.a <= 0.001 {
            discard;
        }
        return vec4<f32>(tint.rgb, sample.a * tint.a);
    }
    return colour * tint;
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
