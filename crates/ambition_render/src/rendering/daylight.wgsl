// The light of the time of day (`daylight.rs`): what the world under it is
// multiplied by. The material's blend multiplies it into the target, so a
// shadow stays a shadow and night darkens rather than fogs.
//
// Where the town's lamps reach (High and Ultra), the world is multiplied by
// the lamps' colour instead, as much as they light there, and never darker
// than the hour.

#import ambition_sprite_fx::mesh_fragment::MeshFragment
#import ambition_render::lamps::{Lamps, MAX_LAMPS, light_of}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> light: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> lamps: Lamps;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var reach: texture_2d<f32>;

@fragment
fn fragment(mesh: MeshFragment) -> @location(0) vec4<f32> {
    let p = mesh.world_position.xy;
    var dark = 1.0;
    var lit = 0.0;
    var tint = vec3(0.0);
    for (var n = 0u; n < min(lamps.count, MAX_LAMPS); n++) {
        let given = light_of(lamps.at[n], u32(lamps.color[n].w), n, reach, p, 0.0);
        if given > 0.0 {
            dark *= 1.0 - given;
            lit += given;
            tint += lamps.color[n].rgb * given;
        }
    }
    var m = light.rgb;
    if lit > 0.0 {
        m = max(m, mix(m, tint / lit, 1.0 - dark));
    }
    var color = vec4(m, 1.0);
#ifdef SRGB_OUTPUT
    color = vec4(linear_to_srgb(color.rgb), 1.0);
#endif
    return color;
}
