#define_import_path ambition_render::card_view

// Find where the camera axis crosses the play plane.
fn camera_on_play_plane(position: vec3<f32>, back: vec3<f32>) -> vec2<f32> {
    return position.xy - back.xy * (position.z / back.z);
}
