// One skin of a two-state room, on its side of the front. See `room_look.rs`.
//
// The skin is published art (`terrain/two_state.py` in the sprite renderer),
// laid on the room's blocks by the room's terrain. This shader draws no art:
// it shows the skin where the room is in the skin's state, and adds what the
// front does to the stone near it. With the shader off, the game draws the
// same skins as plain meshes.
//
// All work is in engine world coordinates (y down).

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#import ambition_content::room_look::{rand_cell, look_field, look_is_corrupt_in, look_is_settled}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// `x, y` = the room size; `z` = 1 for the corrupted skin, 0 for the clean.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> room: vec4<f32>;
// A point on the front (`x, y`) and its normal (`z, w`), which points into
// the corrupted side.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> front: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var skin: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var skin_sampler: sampler;
/// The room's front, one texel a cell (`room_front_field.wgsl`), and its grid.
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> grid: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var front_field: texture_2d<f32>;

// Linear values of the look's colours.
const MAGENTA: vec3<f32> = vec3<f32>(1.0, 0.0221, 0.3967);
const STAIN: vec3<f32> = vec3<f32>(0.020, -0.012, 0.012);

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    // The room is drawn centred on the origin, y up (`world_to_bevy`).
    let p = vec2<f32>(mesh.world_position.x + room.x * 0.5, room.y * 0.5 - mesh.world_position.y);
    let corrupted = look_is_corrupt_in(front_field, grid, p, front, t);
    if corrupted != (room.z > 0.5) {
        return vec4<f32>(0.0);
    }
    var col = textureSample(skin, skin_sampler, mesh.uv);
    if col.a <= 0.0 || look_is_settled(p, front) {
        return col;
    }
    if corrupted {
        // The blocks that changed last are hot still: the light of the front
        // is in them, and it fades into the mass.
        let heat = exp(-max(look_field(p, front, t), 0.0) / 46.0);
        let cell_heat = heat * (0.45 + 0.55 * rand_cell(floor(p / 16.0), 603u));
        col = vec4<f32>(col.rgb + MAGENTA * cell_heat * 0.30 + vec3<f32>(cell_heat * cell_heat * 0.05), col.a);
        // The mass burns where it meets clean stone.
        if !look_is_corrupt_in(front_field, grid, p - front.zw * 3.0, front, t) {
            col = vec4<f32>(MAGENTA * 0.9 + vec3<f32>(0.05), col.a);
        }
    } else {
        // Clean stone near the front already carries the stain.
        let wash = exp(min(look_field(p, front, t), 0.0) / 80.0);
        col = vec4<f32>(col.rgb + wash * STAIN, col.a);
        // Cracks in it, where the front is and not where its edge is this
        // frame: a crack that came and went with the edge was a flicker.
        let still = exp(min(look_field(p, front, 0.0), 0.0) / 80.0);
        let crack = rand_cell(floor(p / vec2<f32>(48.0, 16.0)), 650u);
        let fy = p.y % 16.0;
        if still > 0.25 && crack < still * 0.5 && fy < 1.2 {
            col = vec4<f32>(mix(col.rgb, MAGENTA, 0.75), col.a);
        }
    }
    return col;
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// the shaded colour is written sRGB-encoded, as Bevy's own mesh shaders write
// it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
