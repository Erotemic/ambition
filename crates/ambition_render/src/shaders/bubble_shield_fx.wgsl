// The live layer of the bubble shield. See `bubble_shield_fx.rs`.
//
// The bubble itself is a sprite (`bubble_shield_field.png`). This shader
// draws on it from two more pictures of the sprite submodule: a honeycomb
// (`bubble_shield_lattice.png`) and cracks (`bubble_shield_cracks.png`). It
// draws no art of its own: it says where on the bubble each picture shows,
// and how bright.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// rgb: the hue of the guard's state. a: the strength of the layer.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> tint: vec4<f32>;
// x: how whole the guard is, 1 to 0. y: the pulse of a blow it took, 1 to 0.
// z: 1 in a parry. w: the width of the quad over its height.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> state: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var lattice_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var lattice_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var cracks_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var cracks_sampler: sampler;

// Where the field ends, as a part of the half-width of the quad (the rim of
// the field picture).
const EDGE: f32 = 0.94;
// Cells of the honeycomb picture across the bubble.
const TILES: f32 = 1.25;

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let t = globals.time;
    let p = (mesh.uv - vec2<f32>(0.5)) * 2.0;
    let r = length(p) / EDGE;
    if r >= 1.0 {
        return vec4<f32>(0.0);
    }
    let whole = clamp(state.x, 0.0, 1.0);
    let pulse = clamp(state.y, 0.0, 1.0);
    let parry = clamp(state.z, 0.0, 1.0);

    // The honeycomb is on a ball: near the edge the ball turns away, so the
    // cells are small there. It drifts slowly round the ball.
    let depth = sqrt(max(1.0 - r * r, 0.0));
    let on_ball = (p / EDGE) / (1.0 + depth);
    let cell_uv = fract(on_ball * vec2<f32>(state.w, 1.0) * TILES + vec2<f32>(t * 0.021, t * 0.034));
    let line = textureSample(lattice_texture, lattice_sampler, cell_uv).a;

    // It shows a little everywhere and more toward the rim, where a real
    // bubble shows its skin.
    let toward_rim = smoothstep(0.35, 0.98, r);
    // A slow band of light that goes round the bubble.
    let round = atan2(p.y, p.x);
    let sweep = pow(0.5 + 0.5 * sin(round * 2.0 + r * 3.0 - t * 1.7), 3.0);
    // A blow: a ring that goes from the middle to the rim, and the whole
    // honeycomb is brighter for that beat.
    let ring_at = 1.0 - pulse;
    let ring = step(0.001, pulse) * (1.0 - smoothstep(0.0, 0.22, abs(r - ring_at)));
    let show = clamp(0.07 + 0.34 * toward_rim + 0.30 * sweep * toward_rim + 0.85 * ring + 0.35 * pulse + 0.75 * parry, 0.0, 1.0);
    let cell_alpha = line * show;
    let cell_rgb = mix(tint.rgb, vec3<f32>(1.0), clamp(0.30 + 0.55 * ring + 0.4 * parry, 0.0, 1.0));

    // The cracks: the red of the picture is when each part shows, from the
    // rim (first) to the tips (last).
    let spent = 1.0 - whole;
    let crack = textureSample(cracks_texture, cracks_sampler, mesh.uv);
    let shown = step(0.04, spent) * (1.0 - smoothstep(spent * 1.15 - 0.05, spent * 1.15 + 0.02, crack.r));
    let crack_alpha = crack.a * shown;
    let crack_rgb = mix(tint.rgb, vec3<f32>(1.0), 0.82);

    let alpha = max(cell_alpha * 0.85, crack_alpha) * tint.a * (1.0 - smoothstep(0.96, 1.0, r));
    let rgb = mix(cell_rgb, crack_rgb, step(cell_alpha * 0.85, crack_alpha));
    return vec4<f32>(pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), clamp(alpha, 0.0, 1.0));
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
