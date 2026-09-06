// One simple visual manipulation of a sprite's own pixels.
//
// Draws the sprite's atlas frame like the built-in sprite shader would, then
// applies ONE manipulation to the sampled colour. Deliberately a single
// branchy shader rather than four pipelines: these are cheap per-fragment
// operations and one material keeps the effects batchable with each other.
//
// The operations differ in what they can express, which is the whole reason
// this exists as more than `Sprite.color`:
//
//   MULTIPLY   sample * colour. What `Sprite.color` already does for free.
//              Cannot recolour saturated art — orange * green is dark mud.
//   HUE        rotate the hue, keep luminance and saturation. Recolours art
//              that HAS a colour, which multiply cannot: this is what turns
//              a blue gun into a red one and keeps its shading and highlights.
//   SATURATE   scale saturation about luminance. 0 = greyscale, 1 = identity.
//   SILHOUETTE flat colour masked by the sprite's alpha — shape only, no
//              interior detail. The `hit_flash` overlay's operation.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// uv_rect = (min.x, min.y, max.x, max.y) of this sprite's frame in the atlas.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
// control.x = operation (0 multiply, 1 hue, 2 saturate, 3 silhouette)
// control.y = flip_x flag (0 = no flip, >0.5 = mirror UV horizontally)
// control.z = scalar argument: hue degrees, or saturation factor
// control.w = reserved
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
// Straight-alpha colour argument (linear RGBA) for MULTIPLY and SILHOUETTE.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> colour: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var color_sampler: sampler;

// Rec. 601 luma — the axis SATURATE pivots about, so desaturating a pixel
// walks it toward a grey of the same apparent brightness rather than toward
// the average of its channels.
const LUMA: vec3<f32> = vec3<f32>(0.299, 0.587, 0.114);

// Hue rotation via an explicit HSV round trip.
//
// ⛔ THIS WAS A 3x3 LUMA-AXIS ROTATION MATRIX AND THAT WAS A BUG WAITING TO
// HAPPEN. `mat3x3<f32>(a, b, c)` in WGSL takes COLUMNS, so writing the three
// rows of a rotation matrix into it silently transposes — and the transpose of
// a rotation is its INVERSE, i.e. the hue turns the wrong way. Nothing would
// have failed: the gun would simply have been the wrong colour, disagreeing
// with the portals it opens, on a path no unit test can reach because it runs
// on the GPU. The round trip below is a few more ALU ops and cannot be read two
// ways.
//
// Grey pixels have saturation 0, so their hue is irrelevant and they survive
// unchanged — white highlights stay white, black outlines stay black.
fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let max_c = max(c.r, max(c.g, c.b));
    let min_c = min(c.r, min(c.g, c.b));
    let delta = max_c - min_c;
    var h = 0.0;
    if delta > 0.0 {
        if max_c == c.r {
            h = ((c.g - c.b) / delta) % 6.0;
        } else if max_c == c.g {
            h = (c.b - c.r) / delta + 2.0;
        } else {
            h = (c.r - c.g) / delta + 4.0;
        }
        h = h * 60.0;
    }
    let s = select(0.0, delta / max_c, max_c > 0.0);
    return vec3<f32>(h, s, max_c);
}

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x / 60.0;
    let c = hsv.z * hsv.y;
    let x = c * (1.0 - abs((h % 2.0) - 1.0));
    let m = hsv.z - c;
    var rgb = vec3<f32>(0.0);
    if h < 1.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if h < 2.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if h < 3.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if h < 4.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if h < 5.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }
    return rgb + vec3<f32>(m);
}

fn rotate_hue(rgb: vec3<f32>, degrees: f32) -> vec3<f32> {
    var hsv = rgb_to_hsv(rgb);
    // The sprite pipeline hands us LINEAR colour and the pair hues this is
    // driven by are sRGB-space angles; rotating in either space turns the wheel
    // by the same number of degrees, which is what the caller asked for.
    hsv.x = (hsv.x + degrees) % 360.0;
    if hsv.x < 0.0 {
        hsv.x = hsv.x + 360.0;
    }
    return clamp(hsv_to_rgb(hsv), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var uv = mesh.uv;
    if control.y > 0.5 {
        uv.x = 1.0 - uv.x;
    }
    // Map the quad's 0..1 UV onto this sprite's frame within its atlas.
    let frame_uv = vec2<f32>(
        mix(uv_rect.x, uv_rect.z, uv.x),
        mix(uv_rect.y, uv_rect.w, uv.y),
    );
    let sample = textureSample(color_texture, color_sampler, frame_uv);

    let op = control.x;
    if op < 0.5 {
        return sample * colour;
    }
    if op < 1.5 {
        // Premultiplication would rotate the hue of the ALPHA-weighted colour
        // and darken edge pixels; operate on the straight colour and carry the
        // sprite's own alpha through untouched.
        return vec4<f32>(rotate_hue(sample.rgb, control.z), sample.a);
    }
    if op < 2.5 {
        let luma = dot(sample.rgb, LUMA);
        let saturated = mix(vec3<f32>(luma), sample.rgb, control.z);
        return vec4<f32>(clamp(saturated, vec3<f32>(0.0), vec3<f32>(1.0)), sample.a);
    }
    // Silhouette: the sprite contributes its SHAPE, the argument its colour.
    return vec4<f32>(colour.rgb, sample.a * colour.a);
}
