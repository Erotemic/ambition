// The field of a spirit: near each spirit the picture ripples, as air over a
// fire does, and takes the spirit's colour. See `spirit_field.rs`.
//
// A whole-screen pass, because an overlay can not read the picture under it.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct SpiritFieldSettings {
    // x = the number of sources, y = view width over view height,
    // z = seconds, w = unused.
    control: vec4<f32>,
    // For each source: x, y = its middle in the view (0 to 1, y down);
    // z = its radius, in view heights; w = its strength (0 to 1).
    sources: array<vec4<f32>, 4>,
    // For each source: its tint (xyz).
    tints: array<vec4<f32>, 4>,
    // The view of the camera in its target: x, y = its top left, z, w = its
    // size, each as a part of the target.
    view: vec4<f32>,
}

@group(0) @binding(2) var<uniform> settings: SpiritFieldSettings;

// How far the picture bends at the middle of a field, in view heights.
const WARP: f32 = 0.006;
// How much of its colour a field gives the picture at its middle.
const TINT: f32 = 0.45;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let at = (in.uv - settings.view.xy) / max(settings.view.zw, vec2<f32>(0.0001));
    if at.x < 0.0 || at.y < 0.0 || at.x > 1.0 || at.y > 1.0 {
        return textureSample(screen_texture, texture_sampler, in.uv);
    }
    let aspect = vec2<f32>(settings.control.y, 1.0);
    let seconds = settings.control.z;
    var bend = vec2<f32>(0.0);
    var tint = vec3<f32>(0.0);
    var field = 0.0;
    let count = i32(settings.control.x);
    for (var index = 0; index < count; index++) {
        let source = settings.sources[index];
        let offset = (at - source.xy) * aspect;
        let reach = length(offset) / max(source.z, 0.0001);
        let near = (1.0 - smoothstep(0.2, 1.0, reach)) * source.w;
        // Rings that go out from the spirit, and a slow shimmer that rises
        // across them, as heat does.
        let ring = sin(reach * 28.0 - seconds * 4.0);
        let rise = sin(at.y * 90.0 + seconds * 6.0 + at.x * 20.0);
        let away = offset / max(length(offset), 0.0001);
        bend += (away * ring + vec2<f32>(rise, 0.0) * 0.6) * near * WARP * source.z;
        tint += settings.tints[index].xyz * near;
        field = max(field, near);
    }
    // Bend in view units back to target units.
    let colour = textureSample(screen_texture, texture_sampler, in.uv + bend / aspect * settings.view.zw);
    // The spirit's colour, laid over the picture as light: the darks take it
    // as a glow, the lights keep their own.
    let lit = mix(colour.rgb, colour.rgb * (0.6 + tint) + tint * 0.12, TINT * field);
    return vec4<f32>(lit, colour.a);
}
