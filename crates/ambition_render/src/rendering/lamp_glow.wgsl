// The glow of a lit window at night (`daylight.rs`): light ADDED about each
// lamp's source, so its glass shines brighter than its paint, as a lit
// window does in the dark. The veil under it can only darken. `glow.x` is
// how dark the hour is (none by day), `glow.y` the glow's size as a part of
// the lamp's radius.

#import ambition_sprite_fx::mesh_fragment::MeshFragment
#import ambition_render::lamps::{Lamps, MAX_LAMPS}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> glow: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> lamps: Lamps;

@fragment
fn fragment(mesh: MeshFragment) -> @location(0) vec4<f32> {
    let p = mesh.world_position.xy;
    var added = vec3(0.0);
    for (var n = 0u; n < min(lamps.count, MAX_LAMPS); n++) {
        let lamp = lamps.at[n];
        let d = distance(p, lamp.xy) / max(lamp.z * glow.y, 1.0);
        if d < 3.0 {
            added += lamps.color[n].rgb * lamp.w * exp(-d * d);
        }
    }
    var color = vec4(added * glow.x, 1.0);
#ifdef SRGB_OUTPUT
    color = vec4(linear_to_srgb(color.rgb), 1.0);
#endif
    return color;
}
