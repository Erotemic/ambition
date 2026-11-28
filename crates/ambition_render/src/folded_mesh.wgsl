// A folded mesh (`folded_mesh.rs`): the colours its triangles were built with,
// scaled by `strength`. The material's blend adds them to what is under
// them. It draws no shape: the shape is the mesh.

#import ambition_sprite_fx::mesh_fragment::MeshFragment
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> strength: vec4<f32>;

@fragment
fn fragment(mesh: MeshFragment) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    var color = mesh.color * strength;
#else
    var color = strength;
#endif
#ifdef SRGB_OUTPUT
    color = vec4(linear_to_srgb(color.rgb), color.a);
#endif
    return color;
}
