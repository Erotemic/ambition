#define_import_path ambition_render::shared_mesh_fragment

#import bevy_render::view::View
#import bevy_render::globals::Globals

// Both Bevy mesh vertex stages put the world position at location 0.
struct MeshFragment {
    @location(0) world_position: vec4<f32>,
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
