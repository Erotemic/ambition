// The Flying Spaghetti Monster's sauce: its REVEAL MAP drawn over its sprite.
//
// Each texel of the map is marinara at the damage it appears: alpha is
// `1 - threshold`, so a texel shows once the god's damage fraction reaches its
// threshold, and a texel with alpha 0 never shows. RGB is the sauce's own
// shading. The god starts the fight clean and ends it dressed.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// x: damage fraction in [0, 1]; y: x-flip flag (0 or 1).
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var sauce_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var sauce_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var u = in.uv.x;
    if (control.y > 0.5) {
        u = 1.0 - u;
    }
    let uv = vec2<f32>(mix(uv_rect.x, uv_rect.z, u), mix(uv_rect.y, uv_rect.w, in.uv.y));
    let texel = textureSample(sauce_texture, sauce_sampler, uv);
    let damage = clamp(control.x, 0.0, 1.0);
    // Shown once `damage >= threshold`, i.e. `alpha >= 1 - damage`; a narrow
    // ramp so a splat's edge arrives with it rather than a frame later.
    let cut = 1.0 - damage;
    let shown = smoothstep(cut - 0.015, cut + 0.005, texel.a) * step(0.004, texel.a) * step(0.001, damage);
    return vec4<f32>(texel.rgb, shown);
}
