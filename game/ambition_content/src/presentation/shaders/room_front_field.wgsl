// The front of a two-state room, one texel a 16 px cell: the claim of the
// cell (`look_claim`), which every pixel of the room's skins and overlay
// reads in place of working it out again (`look_claim_in`). See
// `room_look.rs` (`RoomFrontFieldMaterial`).
//
// The texel stores the state over 2 (red) and the level over 3 (green). The
// target is sRGB and the camera blends in linear light, so a texel reads back
// within a few thousandths of what was written; the reader rounds.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#import ambition_content::room_look::{look_claim, LOOK_FIELD_CELL}

/// `x, y` = the world position of the quad's middle; `z, w` = the field's
/// size in cells.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> quad: vec4<f32>;
/// `x, y` = the field's first cell, in the room's world px (`grid` of
/// `look_claim_in`).
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> origin: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> front: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // The quad is the field, y up; texel (0, 0) is its top-left cell.
    let half = quad.zw * LOOK_FIELD_CELL * 0.5;
    let local = vec2<f32>(mesh.world_position.x - quad.x + half.x, quad.y + half.y - mesh.world_position.y);
    let texel = floor(local / LOOK_FIELD_CELL);
    let p = origin.xy + (texel + vec2<f32>(0.5)) * LOOK_FIELD_CELL;
    let claim = look_claim(p, front, globals.time);
    var level = 0.0;
    if claim.size < 100.0 { level = 1.0; }
    if claim.size < 50.0 { level = 2.0; }
    if claim.size < 20.0 { level = 3.0; }
    return vec4<f32>(f32(claim.state) / 2.0, level / 3.0, 0.0, 1.0);
}
