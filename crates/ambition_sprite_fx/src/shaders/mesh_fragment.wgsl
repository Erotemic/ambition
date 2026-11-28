#define_import_path ambition_sprite_fx::mesh_fragment

#import bevy_render::view::View
#import bevy_render::globals::Globals

// These inputs match the Bevy mesh vertex outputs. Only the color location
// differs. The 3D vertex stage supplies UVs when the mesh has that attribute.
struct MeshFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
#ifdef AMBITION_CARD_MESH
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#else
    @location(2) uv: vec2<f32>,
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
#endif
}

@group(0) @binding(0) var<uniform> view: View;
// The 2D and 3D view layouts bind Globals at different locations.
#ifdef AMBITION_CARD_MESH
@group(0) @binding(11) var<uniform> globals: Globals;
#else
@group(0) @binding(1) var<uniform> globals: Globals;
#endif

fn camera_on_play_plane() -> vec2<f32> {
#ifdef AMBITION_CARD_MESH
    let back = view.world_from_view[2].xyz;
    let position = view.world_position;
    return position.xy - back.xy * (position.z / back.z);
#else
    return view.world_position.xy;
#endif
}
