// A part-drawn body's impostor, from premultiplied to straight alpha.
//
// The body's parts are composited by ordinary sprites over a transparent
// clear, and a sprite blends `src * a + dst * (1 - a)`: over transparent, a
// half-covered pixel keeps half its colour. The texture every reader of the
// body samples (the body's own quad, the hit flash, portal pieces, overlays)
// is a straight-alpha sprite texture, so the colour is divided back out here,
// pixel for pixel, with no filtering.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var premultiplied: texture_2d<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(premultiplied));
    let texel = vec2<i32>(clamp(floor(mesh.uv * size), vec2<f32>(0.0), size - vec2<f32>(1.0)));
    let c = textureLoad(premultiplied, texel, 0);
    if c.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(c.rgb / c.a, c.a);
}
