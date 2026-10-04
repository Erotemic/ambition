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
    // Per cell: (hue in turns, saturation, value, 0): the body's colour
    // shift (`CharacterColorShift`), applied in the art's sRGB space.
    shift: array<vec4<f32>, 36>,
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
    let shift = cells.shift[cell];
    var straight = c.rgb / c.a;
    if shift.x != 0.0 || shift.y != 1.0 || shift.z != 1.0 {
        straight = shifted(straight, shift.xyz);
    }
    // The parts were blended from raw sRGB values (gamma space, as the baked
    // frame was); the straight target is sRGB, so decode once here.
    return vec4<f32>(srgb_to_linear(straight), c.a * opacity);
}

// `rgb` (sRGB, 0..1) with its hue turned by `shift.x` turns and its
// saturation and value scaled by `shift.y` and `shift.z`.
fn shifted(rgb: vec3<f32>, shift: vec3<f32>) -> vec3<f32> {
    let hi = max(rgb.r, max(rgb.g, rgb.b));
    let lo = min(rgb.r, min(rgb.g, rgb.b));
    let delta = hi - lo;
    var hue = 0.0;
    if delta > 0.0 {
        if hi == rgb.r {
            hue = (rgb.g - rgb.b) / delta;
        } else if hi == rgb.g {
            hue = 2.0 + (rgb.b - rgb.r) / delta;
        } else {
            hue = 4.0 + (rgb.r - rgb.g) / delta;
        }
        hue = fract(hue / 6.0);
    }
    let saturation = select(0.0, delta / hi, hi > 0.0);
    let h = fract(hue + shift.x) * 6.0;
    let s = clamp(saturation * shift.y, 0.0, 1.0);
    let v = clamp(hi * shift.z, 0.0, 1.0);
    // Offsets in sixths of a turn for r, g, b (checked against Python's colorsys
    // over 2000 random colours and shifts: exact; (5, 3, 1) turned red magenta).
    let k = vec3<f32>(0.0, 4.0, 2.0);
    let p = clamp(abs(fract((h + k) / 6.0) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
    return v * mix(vec3<f32>(1.0), p, s);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}
