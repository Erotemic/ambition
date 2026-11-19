// The light of the time of day (`daylight.rs`): what the world under it is
// multiplied by. The material's blend multiplies it into the target, so a
// shadow stays a shadow and night darkens rather than fogs.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> light: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4(light.rgb, 1.0);
#ifdef SRGB_OUTPUT
    color = vec4(linear_to_srgb(color.rgb), 1.0);
#endif
    return color;
}
