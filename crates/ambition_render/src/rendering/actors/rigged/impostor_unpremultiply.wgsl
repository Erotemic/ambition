// A part-drawn body's impostor, from premultiplied to straight alpha.
//
// The body's parts are composited by ordinary sprites over a transparent
// clear, and a sprite blends `src * a + dst * (1 - a)`: over transparent, a
// half-covered pixel keeps half its colour. The texture every reader of the
// body samples (the body's own quad, the hit flash, portal pieces, overlays)
// is a straight-alpha sprite texture, so the colour is divided back out here,
// pixel for pixel, with no filtering. The parts were blended in GAMMA space
// (raw sRGB values in a plain target, as the baked frame was composited), so the
// colour is decoded to linear here, once, for the sRGB target. Each cell's alpha is then scaled by its
// body's frame opacity: a frame that fades as one picture fades here, after its
// parts are composited.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var premultiplied: texture_2d<f32>;

struct ImpostorCellOpacity {
    opacity: array<vec4<f32>, 9>,
    side: u32,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> cells: ImpostorCellOpacity;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(premultiplied));
    let texel = vec2<i32>(clamp(floor(mesh.uv * size), vec2<f32>(0.0), size - vec2<f32>(1.0)));
    let c = textureLoad(premultiplied, texel, 0);
    if c.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let side = max(cells.side, 1u);
    let cell_texels = size / f32(side);
    let at = min(vec2<u32>(vec2<f32>(texel) / cell_texels), vec2<u32>(side - 1u));
    let cell = at.y * side + at.x;
    let opacity = cells.opacity[cell / 4u][cell % 4u];
    // The parts were blended from raw sRGB values (gamma space, as the baked
    // frame was); the straight target is sRGB, so decode once here.
    return vec4<f32>(srgb_to_linear(c.rgb / c.a), c.a * opacity);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}
