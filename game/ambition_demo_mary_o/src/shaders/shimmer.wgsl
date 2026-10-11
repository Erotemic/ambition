// The shimmer in the air of a Mary-O room. See `shimmer_shader.rs`.
//
// This shader draws no shape. Each shape is in the shimmer picture of the
// biome (`<biome>_shimmer.png`: red is the shapes, green is when each one is
// bright, blue is the ribbons). The shader moves the picture, makes each
// shape come and go, and gives it its colour. All work is in engine world
// coordinates (y down). Colours are display values, converted at the end.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::{view, globals}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// Room width and height; the strength of the shapes; the strength of the ribbons.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> room: vec4<f32>;
// The two colours of the shapes. `a` of the first: how fast a shape comes
// and goes. `a` of the second: the size of one picture in world px.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> shape_a: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> shape_b: vec4<f32>;
// The two colours of the ribbons. `a` of the first: how fast the shapes go
// up, in world px per second.
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> ribbon_a: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> ribbon_b: vec4<f32>;
// Where the ribbons are in the room: the part of its height they start at,
// and the part of its height they go over.
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<uniform> band: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var shimmer_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var shimmer_sampler: sampler;

fn engine(w: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(w.x + room.x * 0.5, room.y * 0.5 - w.y);
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let p = engine(mesh.world_position.xy);
    let camera = engine(view.world_position.xy);
    let t = globals.time;
    let tile = shape_b.a;
    var colour = vec3<f32>(0.0);
    var cover = 0.0;
    // Two sheets of shapes at two depths. The far one is smaller, fainter,
    // and stays with the camera more.
    for (var sheet = 0; sheet < 2; sheet++) {
        let far = f32(sheet);
        let follow = mix(0.55, 0.78, far);
        let size = tile * mix(1.0, 0.62, far);
        let rise = vec2<f32>(0.0, t * ribbon_a.a);
        let uv = (p - camera * follow + rise) / size + vec2<f32>(0.37, 0.11) * far;
        let texel = textureSample(shimmer_texture, shimmer_sampler, uv);
        let when = texel.g;
        let wave = sin(t * shape_a.a * (0.6 + when) + when * 40.0);
        let bright = pow(max(wave * 0.5 + 0.5, 0.0), 3.0);
        let a = texel.r * bright * mix(1.0, 0.55, far) * step(0.02, when) * room.z;
        colour += mix(shape_a.rgb, shape_b.rgb, fract(when * 7.0)) * a;
        cover += a;
    }
    // The ribbons: one picture three tiles long, across a band of the room.
    let across = (p.x - camera.x * 0.8) / (tile * 3.0) + t * 0.004;
    let down = clamp((p.y / room.y - band.x) / band.y, 0.0, 0.998);
    let ribbon = textureSample(shimmer_texture, shimmer_sampler, vec2<f32>(across, down)).b;
    let breath = 0.78 + 0.22 * sin(p.x * 0.012 + t * 0.6);
    let a = ribbon * breath * room.w;
    colour += mix(ribbon_a.rgb, ribbon_b.rgb, 0.5 + 0.5 * sin(p.x * 0.004 + t * 0.1)) * a;
    cover += a;
    let alpha = clamp(cover, 0.0, 1.0);
    let rgb = colour / max(cover, 0.0001);
    return vec4<f32>(pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), alpha);
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// the shaded colour is written sRGB-encoded, as Bevy's own sprite and mesh
// shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
