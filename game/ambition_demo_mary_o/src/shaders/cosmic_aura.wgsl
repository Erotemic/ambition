// The light about Eternal Cosmic Mary-O. See `cosmic_aura.rs`.
//
// She is a sprite. This shader draws light behind her: a soft round glow,
// and in it the stars of the starlight shimmer picture
// (`starlight_shimmer.png`, drawn in the sprite submodule: red is the shapes,
// green is when each one is bright). The picture is read in world space, so
// the stars stay in the air and she goes through them.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// The colour at the middle of the glow, and its strength.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> inner: vec4<f32>;
// The colour at its rim. `a`: how fast a star comes and goes.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> outer: vec4<f32>;
// The colour of a star. `a`: the size of one picture, in world px.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> star: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var star_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var star_sampler: sampler;

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    let w = mesh.world_position.xy;
    // 0 at the middle of the quad, 1 at its edge.
    let from_middle = distance(mesh.uv, vec2<f32>(0.5)) * 2.0;
    let falloff = pow(clamp(1.0 - from_middle, 0.0, 1.0), 1.6);
    let breath = 0.86 + 0.14 * sin(t * 2.2);
    let glow = falloff * inner.a * breath;
    // World y is up here, so a picture that moves down in `v` goes up.
    let uv = vec2<f32>(w.x / star.a, -w.y / star.a + t * 0.03);
    let texel = textureSample(star_texture, star_sampler, uv);
    let when = texel.g;
    let wave = sin(t * outer.a * (0.6 + when) + when * 40.0);
    let bright = pow(max(wave * 0.5 + 0.5, 0.0), 3.0);
    // A star is in the glow, and it has no hard end at the edge of the quad.
    let twinkle = texel.r * bright * step(0.02, when) * clamp(falloff * 3.0, 0.0, 1.0);
    let alpha = clamp(glow + twinkle, 0.0, 1.0);
    let rgb = mix(mix(outer.rgb, inner.rgb, falloff), star.rgb, clamp(twinkle / max(alpha, 0.0001), 0.0, 1.0));
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
