// The field of the void: the picture loses its colour, and a part of its
// light, near each void thing. See `void_field.rs`.
//
// A whole-screen pass, because an overlay can not read the picture under it.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct VoidFieldSettings {
    // x = the number of sources, y = view width over view height,
    // z = how much light the void takes at its middle (0 to 1),
    // w = the number of wards.
    control: vec4<f32>,
    // For each source: x, y = its middle in the view (0 to 1, y down);
    // z = its radius, in view heights; w = its strength (0 to 1).
    sources: array<vec4<f32>, 8>,
    // For each ward, as for a source. A ward gives the picture its colour
    // back: where it is, the void takes nothing.
    wards: array<vec4<f32>, 4>,
}

@group(0) @binding(2) var<uniform> settings: VoidFieldSettings;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let colour = textureSample(screen_texture, texture_sampler, in.uv);
    var field = 0.0;
    let count = i32(settings.control.x);
    for (var index = 0; index < count; index++) {
        let source = settings.sources[index];
        // In view heights, so a field is round on a wide view.
        let offset = (in.uv - source.xy) * vec2<f32>(settings.control.y, 1.0);
        let near = 1.0 - smoothstep(0.25, 1.0, length(offset) / max(source.z, 0.0001));
        field = max(field, near * source.w);
    }
    var ward = 0.0;
    let ward_count = i32(settings.control.w);
    for (var index = 0; index < ward_count; index++) {
        let source = settings.wards[index];
        let offset = (in.uv - source.xy) * vec2<f32>(settings.control.y, 1.0);
        let near = 1.0 - smoothstep(0.45, 1.0, length(offset) / max(source.z, 0.0001));
        ward = max(ward, near * source.w);
    }
    field = field * (1.0 - ward);
    // The grey of a colour as an eye sees it. The values are read as the
    // camera stores them: a grey of display values is the grey a player sees.
    let grey = dot(colour.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let drained = mix(colour.rgb, vec3<f32>(grey), field);
    return vec4<f32>(drained * (1.0 - settings.control.z * field), colour.a);
}
