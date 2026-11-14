// The light about a glowing body, computed per pixel (`ActorGlowMaterial`).
//
// The light is wide and faint: the player's highlight is 1.6 body heights
// across at 1.8% opacity. The world draws into an 8-bit target, so a gradient
// that faint lands on two or three stored levels, and a mask texture of it
// reads as hard rings. Here the falloff is computed in float and its coverage
// is dithered on the screen, so the step between two stored levels is a
// scatter of pixels instead of a ring.
//
// The pipeline blends premultiplied (`ActorGlowMaterial::specialize`), so one
// pipeline draws both looks:
//   look.x < 0.5  smooth: painted over what is behind (`rgb * a`, `a`), with
//                 the falloff of the mask it replaces.
//   look.x >= 0.5 light: added to what is behind (`rgb * i`, alpha 0), with
//                 the falloff of a lamp: a bright core and a long soft tail.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// colour.rgb: the light's hue at full brightness, linear. colour.a: its
// strength at the middle, 0..1.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> colour: vec4<f32>;
// look.x: 0 smooth, 1 light. look.yzw unused.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> look: vec4<f32>;

// The mask's falloff, `exp(-FALLOFF * d^2)`, lowered to reach 0 at the rim.
const FALLOFF: f32 = 3.2;
// How far the dither may move the coverage, at most, either way. It is also
// held to the coverage itself, so its mean is the coverage (an 8-bit target
// clamps a negative output before blending, which would bias it).
const DITHER: f32 = 0.016;
// The lamp: its core's size (as a squared fraction of the radius) and how
// bright its middle is against the authored strength.
const LAMP_CORE: f32 = 0.035;
const LAMP_GAIN: f32 = 2.2;

// Interleaved gradient noise (Jimenez 2014): cheap, even, and without the
// low-frequency blotches of a hash.
fn ign(p: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
}

// Triangular noise in -1..1: two samples, so the scatter has no flat
// plateaus where one sample's error happens to cancel.
fn triangular(p: vec2<f32>) -> f32 {
    return ign(p) + ign(p + vec2<f32>(47.0, 17.0)) - 1.0;
}

fn dithered(amount: f32, noise: f32) -> f32 {
    return max(amount + noise * min(amount, DITHER), 0.0);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let p = mesh.uv * 2.0 - 1.0;
    let d2 = dot(p, p);
    if d2 >= 1.0 {
        discard;
    }
    let strength = clamp(colour.a, 0.0, 1.0);
    let noise = triangular(mesh.position.xy);
    var out: vec4<f32>;
    if look.x < 0.5 {
        let rim = exp(-FALLOFF);
        let falloff = (exp(-FALLOFF * d2) - rim) / (1.0 - rim);
        let a = dithered(strength * falloff, noise);
        out = vec4<f32>(colour.rgb * a, a);
    } else {
        // Zero, and level, at the rim: no edge where the quad ends.
        let window = (1.0 - d2) * (1.0 - d2);
        let lamp = LAMP_CORE / (LAMP_CORE + d2) * window;
        let i = dithered(strength * LAMP_GAIN * lamp, noise);
        out = vec4<f32>(colour.rgb * i, 0.0);
    }
    if max(out.a, max(out.r, max(out.g, out.b))) <= 0.0 {
        discard;
    }
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(out.rgb), out.a);
#else
    return out;
#endif
}
