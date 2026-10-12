// The barrier of Eternal Cosmic Mary-O. See `cosmic_barrier.rs`.
//
// She is a sprite made of parts. This shader reads the parts that are her
// body (not her wings) from their page and draws light on the border of
// their shape: a bright line next to her, a band about the line, and a soft
// band about that. It draws nothing on her. A pulse goes up the barrier.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

struct Barrier {
    // The quad under her feet: its low corner (x, y) and its size (z, w),
    // in sheet pixels, +y up.
    quad: vec4<f32>,
    // Each part: its low corner and its size, in the same space.
    part_box: array<vec4<f32>, 4>,
    // Each part on the page: (u0, v0, u1, v1), v down.
    part_uv: array<vec4<f32>, 4>,
    // x: how many parts. y: sheet pixels to one pixel of her art.
    // z: the strength of the barrier.
    control: vec4<f32>,
    // xy: a corner of the pixels of her art.
    grid: vec4<f32>,
    line: vec4<f32>,
    band: vec4<f32>,
    glow: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> barrier: Barrier;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var page_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var page_sampler: sampler;

// How much of her body is at `p` (sheet pixels from her feet, +y up).
fn body_at(p: vec2<f32>) -> f32 {
    var most = 0.0;
    let count = i32(barrier.control.x);
    for (var i = 0; i < count; i = i + 1) {
        let box = barrier.part_box[i];
        let q = (p - box.xy) / box.zw;
        if q.x < 0.0 || q.x > 1.0 || q.y < 0.0 || q.y > 1.0 {
            continue;
        }
        let uv_rect = barrier.part_uv[i];
        let uv = vec2<f32>(mix(uv_rect.x, uv_rect.z, q.x), mix(uv_rect.w, uv_rect.y, q.y));
        most = max(most, textureSampleLevel(page_texture, page_sampler, uv, 0.0).a);
    }
    return most;
}

// True when her body is within `reach` of `p`, in one of eight directions.
fn near_body(p: vec2<f32>, reach: f32) -> f32 {
    let d = reach * 0.7071;
    var most = body_at(p + vec2<f32>(reach, 0.0));
    most = max(most, body_at(p + vec2<f32>(-reach, 0.0)));
    most = max(most, body_at(p + vec2<f32>(0.0, reach)));
    most = max(most, body_at(p + vec2<f32>(0.0, -reach)));
    most = max(most, body_at(p + vec2<f32>(d, d)));
    most = max(most, body_at(p + vec2<f32>(-d, d)));
    most = max(most, body_at(p + vec2<f32>(d, -d)));
    most = max(most, body_at(p + vec2<f32>(-d, -d)));
    return step(0.5, most);
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    let cell = max(barrier.control.y, 1.0);
    let raw = barrier.quad.xy + vec2<f32>(mesh.uv.x, 1.0 - mesh.uv.y) * barrier.quad.zw;
    // One answer for each pixel of her art, so the barrier is of her pixels.
    let p = barrier.grid.xy + (floor((raw - barrier.grid.xy) / cell) + vec2<f32>(0.5)) * cell;
    if body_at(p) >= 0.5 {
        return vec4<f32>(0.0);
    }
    let in_line = near_body(p, cell);
    let in_band = near_body(p, cell * 2.0);
    let in_glow = near_body(p, cell * 3.0);
    if in_glow < 0.5 {
        return vec4<f32>(0.0);
    }
    // The pulse: a line of light that goes up her, again and again.
    let high = barrier.quad.w;
    let pulse_y = barrier.quad.y + fract(t * 0.55) * (high + 24.0 * cell) - 12.0 * cell;
    let pulse = 1.0 - clamp(abs(p.y - pulse_y) / (5.0 * cell), 0.0, 1.0);
    // The colour of the band goes slowly from one end of her to the other.
    let along = 0.5 + 0.5 * sin((p.y / high) * 5.0 - t * 2.4 + p.x * 0.03);
    var colour = mix(barrier.glow.rgb, barrier.band.rgb, along);
    var alpha = barrier.glow.a * (0.7 + 0.3 * along);
    if in_band >= 0.5 {
        colour = mix(barrier.band.rgb, barrier.line.rgb, along * 0.6);
        alpha = barrier.band.a;
    }
    if in_line >= 0.5 {
        colour = barrier.line.rgb;
        alpha = barrier.line.a;
    }
    colour = mix(colour, vec3<f32>(1.0), pulse * 0.85);
    alpha = clamp((alpha + pulse * 0.45) * barrier.control.z, 0.0, 1.0);
    return vec4<f32>(pow(max(colour, vec3<f32>(0.0)), vec3<f32>(2.2)), alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
