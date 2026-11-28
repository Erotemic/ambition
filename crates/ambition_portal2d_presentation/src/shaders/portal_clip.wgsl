// The mesh contains the complete clipped material region.

#import ambition_sprite_fx::mesh_fragment::MeshFragment
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// control.x = flip_x flag (0 = no flip, >0.5 = mirror UV horizontally)
// control.y = flip_y flag (0 = no flip, >0.5 = mirror UV vertically)
// control.z = silhouette flag (>0.5 = paint tint.rgb masked by the sample's
//             alpha and tint.a, the hit-flash overlay's look, instead of the
//             sampled colour). A declared non-sprite drawable is what sets it.
// control.w = alpha mode: straight (0), premultiplied (1), opaque (2).
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
// Straight-alpha sprite tint (linear RGBA), multiplied into the sample.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> sample_plane: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var color_sampler: sampler;

// Only use texels from the visible side of the exit. The capture also
// contains the rim and room behind the edge of an emerging body.
fn window_sample(uv: vec2<f32>, ordinary: vec3<f32>) -> vec3<f32> {
    let normal = sample_plane.xy;
    if all(normal == vec2<f32>(0.0)) {
        return ordinary;
    }
    let size = vec2<f32>(textureDimensions(color_texture));
    let step = normal / size;
    let distance = dot(uv, normal) + sample_plane.z;
    if distance >= dot(abs(step), vec2<f32>(1.0)) {
        return ordinary;
    }
    let pixel = uv * size - vec2<f32>(0.5);
    let base = vec2<i32>(floor(pixel));
    let frac = fract(pixel);
    let limit = vec2<i32>(size) - vec2<i32>(1);
    var sum = vec3<f32>(0.0);
    var total = 0.0;
    for (var i = 0; i < 4; i += 1) {
        let offset = vec2<i32>(i % 2, i / 2);
        let tap = clamp(base + offset, vec2<i32>(0), limit);
        let center = (vec2<f32>(tap) + vec2<f32>(0.5)) / size;
        if dot(center, normal) + sample_plane.z >= 0.0 {
            let factors = mix(vec2<f32>(1.0) - frac, frac, vec2<f32>(offset));
            let weight = factors.x * factors.y;
            sum += weight * textureLoad(color_texture, tap, 0).rgb;
            total += weight;
        }
    }
    if total > 0.0 {
        return sum / total;
    }
    return ordinary;
}

fn shade(mesh: MeshFragment) -> vec4<f32> {
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

    // An untouched window target has alpha zero. A capture clears it to alpha one.
    if sample.a <= 0.01 {
        discard;
    }
    if control.w > 1.5 {
        return vec4<f32>(window_sample(atlas_uv, sample.rgb) * tint.rgb, 1.0);
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
fn fragment(mesh: MeshFragment) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
