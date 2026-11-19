// The darkness of a dark room at High and Ultra (`darkness.rs`), per pixel:
// the veil over the room, `veil.w` dark in its ambient (`veil.rgb`, sRGB),
// less what each light takes away (`lamps.wgsl`). Where a lamp's light fades
// the veil takes the lamp's colour, so a lantern's pool has a warm rim. The
// same sum the CPU light map makes per texel (`darkness::light_map`).

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import ambition_render::lamps::{Lamps, MAX_LAMPS, LIGHT_CORE, light_of}
#import bevy_render::color_operations::srgb_to_linear
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// How much of a lamp's colour its rim leaves in the veil (`darkness::RIM_WARMTH`).
const RIM_WARMTH: f32 = 0.3;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> veil: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> lamps: Lamps;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var reach: texture_2d<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let p = mesh.world_position.xy;
    var dark = 1.0;
    var lit = 0.0;
    var tint = vec3(0.0);
    for (var n = 0u; n < min(lamps.count, MAX_LAMPS); n++) {
        let given = light_of(lamps.at[n], u32(lamps.color[n].w), n, reach, p, LIGHT_CORE);
        if given > 0.0 {
            dark *= 1.0 - given;
            lit += given;
            tint += lamps.color[n].rgb * given;
        }
    }
    let shade = min((veil.rgb * dark + RIM_WARMTH * tint) / (dark + lit), vec3(1.0));
    // What the eye sees as `veil.w * dark` dark (`darkness::veil_alpha`).
    let alpha = 1.0 - pow(1.0 - clamp(veil.w * dark, 0.0, 1.0), 2.2);
    var color = vec4(srgb_to_linear(shade), alpha);
#ifdef SRGB_OUTPUT
    color = vec4(linear_to_srgb(color.rgb), color.a);
#endif
    return color;
}
