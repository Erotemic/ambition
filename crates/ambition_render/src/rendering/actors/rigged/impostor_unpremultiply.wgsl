// A part-drawn body's impostor, from premultiplied to straight alpha.
//
// The body's parts are composited by ordinary sprites over a transparent
// clear, and a sprite blends `src * a + dst * (1 - a)`: over transparent, a
// half-covered pixel keeps half its colour. The texture every reader of the
// body samples (the body's own quad, the hit flash, portal pieces, overlays)
// is a straight-alpha sprite texture, so the colour is divided back out here,
// pixel for pixel, with no filtering. The parts were blended in GAMMA space (as
// the baked frame was composited, and as the world camera blends them drawn
// directly); the premultiplied target is sRGB, so a load decodes it, and the
// colour is encoded back to the values that were blended, divided there, and
// decoded once for the sRGB straight target. Each cell's alpha is then scaled by its
// body's frame opacity: a frame that fades as one picture fades here, after its
// parts are composited. A cell whose row has a teleport warp is taken apart
// here too (`warped`), for the same reason: it is one picture.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var premultiplied: texture_2d<f32>;

struct ImpostorCellOpacity {
    opacity: array<vec4<f32>, 9>,
    // Per cell: (hue in turns, saturation, value, 0): the body's colour
    // shift (`CharacterColorShift`), applied in the art's sRGB space.
    shift: array<vec4<f32>, 36>,
    // Per cell: (kind, progress, left, right): the teleport warp of the
    // body's row. Kind 0 is none, 1 a departure, 2 an arrival. Left and right
    // are the span of the body across the cell, in sheet pixels.
    warp: array<vec4<f32>, 36>,
    side: u32,
    // 1: the parts were blended in gamma space; 0: in linear light.
    gamma: u32,
    // Sheet pixels on a side of one cell.
    cell_px: f32,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> cells: ImpostorCellOpacity;

// The straight-alpha colour of one texel of the composite, in the art's sRGB
// values, and its alpha. Zero for an empty texel.
fn load_straight(texel: vec2<i32>) -> vec4<f32> {
    let loaded = textureLoad(premultiplied, texel, 0);
    // A load decodes the sRGB target to linear: the values blended, when the
    // parts were blended in linear light; encoded back, when in gamma.
    var c = loaded;
    if cells.gamma == 1u {
        c = vec4<f32>(linear_to_srgb(loaded.rgb), loaded.a);
    }
    if c.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    var straight = c.rgb / c.a;
    // Divided in the space the parts were blended in; the rest of this pass
    // works in the art's sRGB values.
    if cells.gamma != 1u {
        straight = linear_to_srgb(straight);
    }
    return vec4<f32>(straight, c.a);
}

fn ease(t: f32) -> f32 {
    let x = clamp(t, 0.0, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

// A body taken apart by a teleport: the cell's picture cut into vertical
// slivers five sheet pixels wide. Departing, each sliver slides away from the
// middle of the body, rises and fades, and every third one flickers. Arriving,
// the slivers come together, and the whole body fades in over them. The
// numbers are the ones the player robot's baked effect used.
//
// `local` is the texel in the cell, `origin` the cell's first texel, `w` the
// cell's warp. A sliver is drawn where it lands, so this asks which sliver
// lands on `local`: the few nearest, the last drawn first.
fn warped(local: vec2<f32>, origin: vec2<f32>, cell_texels: vec2<f32>, w: vec4<f32>) -> vec4<f32> {
    // Texels to one sheet pixel.
    let u = cell_texels.x / max(cells.cell_px, 1.0);
    let x1 = w.z * u;
    let x2 = w.w * u;
    let span = max(x2 - x1, 1.0);
    let slice_w = max(1.0, floor(5.0 * u));
    let departs = w.x < 1.5;
    var progress = ease(w.y);
    if departs {
        progress = ease((w.y - 0.02) / 0.98);
    }
    // How far the outermost slivers slide.
    let spread = select(24.0 * u * (1.0 - progress), 22.0 * u * progress, departs);
    // The slivers reach past the body's span, for an arm or an antenna.
    let reach = 20.0 * u;
    let first = x1 - reach;
    let count = ceil((x2 + reach - first) / slice_w);
    // Undo the slide to find the sliver: local.x = x + ((x - x1) / span - 0.5) * spread.
    let guess = (local.x + 0.5 * spread + spread * x1 / span) / (1.0 + spread / span);
    let nearest = floor((guess - first) / slice_w);
    let beat = i32(w.y * 5.0);
    var slivers = vec4<f32>(0.0);
    for (var k = 3; k >= -3; k--) {
        let i = nearest + f32(k);
        if i < 0.0 || i >= count {
            continue;
        }
        let xs = first + i * slice_w;
        let frac = clamp((xs + slice_w * 0.5 - x1) / span, 0.0, 1.0);
        var dx = (frac - 0.5) * spread;
        var dy = 0.0;
        var alpha = 1.0;
        if departs {
            dx += sin(frac * 3.14159265 * 7.0 + progress * 7.0) * 1.8 * u * progress;
            dy = -(5.0 + abs(frac - 0.5) * 18.0) * u * progress;
            alpha = max(0.06, 1.0 - 0.88 * progress);
            if progress > 0.35 && (i32(i) + i32(progress * 10.0)) % 3 == 0 {
                alpha *= 0.35;
            }
        } else {
            dy = -(3.0 + abs(frac - 0.5) * 16.0) * u * (1.0 - progress);
            alpha = min(1.0, 0.18 + 0.94 * progress);
            if progress < 0.45 && (i32(i) + beat) % 4 == 0 {
                alpha *= 0.55;
            }
        }
        let landed = floor(xs + dx);
        if local.x < landed || local.x >= landed + slice_w {
            continue;
        }
        let source = vec2<f32>(xs + (local.x - landed), local.y - floor(dy));
        if source.x < 0.0 || source.y < 0.0 || source.x >= cell_texels.x || source.y >= cell_texels.y {
            continue;
        }
        let texel = load_straight(vec2<i32>(origin + source));
        if texel.a > 0.0 {
            // A sliver turns to light as it goes, and its leading edge is the
            // brightest part of it: the body reads as energy, not as glass.
            let gone = 1.0 - alpha;
            let edge = step(local.x - landed, max(1.0, u)) * gone;
            let light = vec3<f32>(0.35, 0.92, 1.0);
            let colour = mix(texel.rgb, light, clamp(0.55 * gone + 0.6 * edge, 0.0, 1.0));
            slivers = vec4<f32>(colour, texel.a * max(alpha, 0.75 * edge));
            break;
        }
    }
    if departs {
        return slivers;
    }
    // Arriving: the whole body, as one picture, fades in over its slivers.
    let whole = load_straight(vec2<i32>(origin + local));
    let whole_alpha = whole.a * ease((progress - 0.34) / 0.66);
    let alpha = whole_alpha + slivers.a * (1.0 - whole_alpha);
    if alpha <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>((whole.rgb * whole_alpha + slivers.rgb * slivers.a * (1.0 - whole_alpha)) / alpha, alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(premultiplied));
    let texel = vec2<i32>(clamp(floor(mesh.uv * size), vec2<f32>(0.0), size - vec2<f32>(1.0)));
    let side = max(cells.side, 1u);
    let cell_texels = size / f32(side);
    let at = min(vec2<u32>(vec2<f32>(texel) / cell_texels), vec2<u32>(side - 1u));
    let cell = at.y * side + at.x;
    let warp = cells.warp[cell];
    var pixel: vec4<f32>;
    if warp.x > 0.5 {
        let origin = vec2<f32>(at) * cell_texels;
        pixel = warped(vec2<f32>(texel) - origin, origin, cell_texels, warp);
    } else {
        pixel = load_straight(texel);
        pixel.a *= cells.opacity[cell / 4u][cell % 4u];
    }
    if pixel.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let shift = cells.shift[cell];
    var straight = pixel.rgb;
    if shift.x != 0.0 || shift.y != 1.0 || shift.z != 1.0 {
        straight = shifted(straight, shift.xyz);
    }
    // The straight target takes linear colour.
    return vec4<f32>(srgb_to_linear(straight), pixel.a);
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

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}
