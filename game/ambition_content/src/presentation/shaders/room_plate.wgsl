// One piece of the architecture of the two-state look, drawn from its plate.
// See `room_look/plates.rs`.
//
// The plate holds the piece two times: clean, and corrupted. This shader
// reads the two, and the front of the room says which one a point shows. It
// adds what the front does to the stone near it, and what falls from a
// platform. The art itself is not computed here.
//
// All work is in engine world coordinates (y down). A texel of the plate is
// one world px.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#import ambition_content::room_look::{rand_cell, value_noise, look_field, look_is_corrupt, look_is_settled, look_air_state}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> piece: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> window: vec4<f32>;
// x: the role of the piece. yz: the size of the part of the window that the
// plate holds, from its upper left corner.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> kind: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> front: vec4<f32>;
// The window on the plate, in texels: clean (xy), corrupted (zw).
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> cells: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var plate: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var plate_sampler: sampler;

const MAGENTA: vec3<f32> = vec3<f32>(1.00, 0.16, 0.66);
const CYAN: vec3<f32> = vec3<f32>(0.13, 0.88, 1.00);
const ROLE_SURFACE: f32 = 1.0;
const ROLE_UNDERSIDE: f32 = 3.0;

/// A texel of the plate as a display colour with straight alpha.
fn display(texel: vec4<f32>) -> vec4<f32> {
    if texel.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(pow(max(texel.rgb / texel.a, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)), texel.a);
}

/// What falls from a platform: water on the clean side, light on the
/// corrupted side. The same streams, at the same places. `l` is from the
/// lower left corner of the platform.
fn streams(l: vec2<f32>, p: vec2<f32>, t: f32) -> vec4<f32> {
    let w = piece.z;
    if l.x < 0.0 || l.x >= w || l.y < 0.0 {
        return vec4<f32>(0.0);
    }
    let col_i = floor(l.x / 40.0);
    let key = vec2<f32>(col_i + piece.x, piece.y);
    if rand_cell(key, 680u) > 0.42 {
        return vec4<f32>(0.0);
    }
    let r = rand_cell(key, 681u);
    let cx = (col_i + 0.3 + 0.4 * r) * 40.0;
    let len = 70.0 + (piece.w - 80.0) * rand_cell(key, 682u);
    if l.y >= len {
        return vec4<f32>(0.0);
    }
    let dx = abs(l.x - cx);
    let fall = pow(1.0 - l.y / len, 1.6);
    let air = look_air_state(p, front, t);
    // Light.
    let flow = 0.62 + 0.38 * sin(l.y * 0.12 - t * 6.0 + r * 6.283);
    let light_a = fall * flow * (smoothstep(1.4, 0.3, dx) + exp(-dx / 5.0) * 0.32) * air;
    let light = select(MAGENTA, CYAN, r > 0.82) + vec3<f32>(0.25 * fall);
    // Water: a thin fall that widens, with streaks that run down it, and
    // mist where it ends. Half of the streams only.
    var water_a = 0.0;
    if r < 0.5 {
        let width = 1.6 + 3.2 * (l.y / len);
        let streak = 0.55 + 0.45 * value_noise(vec2<f32>(l.x * 3.0, l.y - t * 150.0), 9.0, 683u);
        let body = smoothstep(width, width - 1.4, dx) * streak * (0.35 + 0.50 * fall);
        let mist = exp(-dx / 16.0) * smoothstep(len * 0.55, len, l.y) * 0.30;
        water_a = max(body, mist) * (1.0 - air);
    }
    let water = vec3<f32>(0.930, 0.965, 1.000);
    let a = max(light_a, water_a);
    if a <= 0.001 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(mix(water, light, light_a / a), clamp(a, 0.0, 1.0));
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    // The position in the window, in texels. The quad is the window, so the
    // art stays on a quad that moves (a block that flinches).
    let local = mesh.uv * window.zw;
    let p = window.xy + local;
    let held = step(local.x, kind.y) * step(local.y, kind.z);
    // A texel stays sharp, and the filter blends only across its edge, by the
    // width of one screen pixel.
    let footprint = max(fwidth(local), vec2<f32>(0.0001));
    let seam = floor(local + vec2<f32>(0.5));
    let sharp = seam + clamp((local - seam) / footprint, vec2<f32>(-0.5), vec2<f32>(0.5));
    let at = clamp(sharp, vec2<f32>(0.5), kind.yz - vec2<f32>(0.5));
    let size = vec2<f32>(textureDimensions(plate));
    let clean = display(textureSampleLevel(plate, plate_sampler, (cells.xy + at) / size, 0.0) * held);
    let corrupt = display(textureSampleLevel(plate, plate_sampler, (cells.zw + at) / size, 0.0) * held);

    let role = kind.x;
    let l = p - piece.xy;
    let inside = l.x >= 0.0 && l.y >= 0.0 && l.x < piece.z && l.y < piece.w;
    let corrupted = look_is_corrupt(p, front, t);
    var col = select(clean, corrupt, corrupted);

    // What the front does to the stone near it. Far from the front there is
    // none of it, and the plate is the whole picture.
    if role == ROLE_SURFACE && inside && col.a > 0.0 && !look_is_settled(p, front) {
        if corrupted {
            // The blocks that changed last are hot still: the light of the
            // front is in them, and it fades into the mass.
            let heat = exp(-max(look_field(p, front, t), 0.0) / 46.0);
            let cell_heat = heat * (0.45 + 0.55 * rand_cell(floor(p / 16.0), 603u));
            col = vec4<f32>(col.rgb + MAGENTA * cell_heat * 0.42 + vec3<f32>(cell_heat * cell_heat * 0.20), col.a);
            // The mass burns where it meets clean stone.
            if !look_is_corrupt(p - front.zw * 3.0, front, t) {
                col = vec4<f32>(MAGENTA * 0.95 + vec3<f32>(0.25), 1.0);
            }
        } else {
            // Clean stone near the front already carries the stain.
            let wash = exp(min(look_field(p, front, t), 0.0) / 80.0);
            col = vec4<f32>(col.rgb + wash * vec3<f32>(0.090, -0.050, 0.050), col.a);
            // Cracks in it. They are where the front is and not where its
            // edge is this frame: the edge moves all the time, and a crack
            // that came and went with it was a flicker.
            let still = exp(min(look_field(p, front, 0.0), 0.0) / 80.0);
            let crack = rand_cell(floor(p / vec2<f32>(48.0, 16.0)), 650u);
            let fy = p.y % 16.0;
            if still > 0.25 && crack < still * 0.5 && fy < 1.2 {
                col = vec4<f32>(mix(col.rgb, MAGENTA, 0.75), col.a);
            }
        }
    }
    if role == ROLE_UNDERSIDE && col.a <= 0.0 {
        col = streams(l, p, t);
    }
    // Display values to linear.
    return vec4<f32>(pow(max(col.rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), col.a);
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// (`CompositingSpace::Srgb`) the shaded colour is written sRGB-encoded, as
// Bevy's own sprite and mesh shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
