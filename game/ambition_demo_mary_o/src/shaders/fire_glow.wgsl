// The glow of a fire in Mary-O: her spark blossom, and each orb of a bar of
// fire. See `fire_shader.rs`.
//
// The fire itself is a sprite. This shader draws light about it: a soft
// round glow that the flame picture (`fire_glow.png`, drawn in the sprite
// submodule) breaks into tongues that go up. The picture is read in world
// space, so two fires side by side do not flicker as one.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// The colour at the rim of the glow, and its strength.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> outer: vec4<f32>;
// The colour at its middle. `a`: how fast the tongues go up, in pictures per second.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> inner: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var flame_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var flame_sampler: sampler;

// The size of one flame picture, in world px.
const TILE: f32 = 72.0;

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    let w = mesh.world_position.xy;
    // 0 at the middle of the quad, 1 at its edge.
    let from_middle = distance(mesh.uv, vec2<f32>(0.5)) * 2.0;
    let falloff = pow(clamp(1.0 - from_middle, 0.0, 1.0), 1.7);
    // World y is up here, so a picture that moves down in `v` goes up.
    let uv = vec2<f32>(w.x / TILE, -w.y / TILE + t * inner.a);
    let tongues = textureSample(flame_texture, flame_sampler, uv).r;
    let pulse = 0.86 + 0.14 * sin(t * 11.0 + w.x * 0.05 + w.y * 0.03);
    let alpha = clamp(falloff * (0.40 + 0.85 * tongues) * pulse * outer.a, 0.0, 1.0);
    let rgb = mix(outer.rgb, inner.rgb, falloff);
    return vec4<f32>(pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
