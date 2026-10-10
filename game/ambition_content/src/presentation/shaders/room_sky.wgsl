// The corrupted sky of the two-state room look, over the clean one.
// See `room_look.rs`.
//
// This shader draws no art. The sky of the look is authored: two parallax
// themes of four layers each (`hub_clean`, `hub_corrupt`; the parallax
// renderer's `room_look_sky.py`). The parallax system draws the clean one.
// This quad is over it: it lays the four layers of the corrupted one where
// the air of the room is corrupted, each at the place the parallax system
// puts the same layer of the clean one, so a tower of one state is on the
// same tower of the other. Then it adds the fog between the sky and the play.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::{view, globals}
#import ambition_content::room_look::{value_noise, look_air_state}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// xy: the room size.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> room: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> front: vec4<f32>;
// x: how much fog is in front of the sky, 0..1. y: how much it is in patches.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> depth: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var sky_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var layer_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var far_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var near_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var atmosphere_texture: texture_2d<f32>;

/// Where the world point `w` (render coordinates) is on the panel of a layer
/// that follows `factor` of the camera and is `scale` views wide. The rule of
/// `sync_parallax_transform_to_camera` and `ParallaxLayerVisual::panel_size`
/// (`ambition_render`): the two must agree, or the two states of the sky do
/// not register.
fn panel_uv(w: vec2<f32>, factor: f32, scale: f32) -> vec2<f32> {
    // What the camera shows of the world, in world units
    // (`visible_world` in `ambition_render`): the play camera is orthographic.
    let visible = vec2<f32>(2.0 / view.clip_from_view[0][0], 2.0 / abs(view.clip_from_view[1][1]));
    let panel = max(visible.x, visible.y) * scale;
    let travel = max((vec2<f32>(panel) - visible) * 0.5, vec2<f32>(0.0));
    let camera = view.world_position.xy;
    let across = clamp(camera / max(room.xy, vec2<f32>(1.0)) + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0)) * 2.0
        - vec2<f32>(1.0);
    let centre = camera - across * travel * factor;
    let o = (w - centre) / panel;
    return vec2<f32>(0.5 + o.x, 0.5 - o.y);
}

/// The four layers of the corrupted sky at `w`, each over the last.
fn corrupt_sky(w: vec2<f32>) -> vec3<f32> {
    var col = textureSampleLevel(sky_texture, layer_sampler, panel_uv(w, 0.10, 1.20), 0.0).rgb;
    let far = textureSampleLevel(far_texture, layer_sampler, panel_uv(w, 0.20, 1.34), 0.0);
    col = mix(col, far.rgb, far.a);
    let near = textureSampleLevel(near_texture, layer_sampler, panel_uv(w, 0.42, 1.52), 0.0);
    col = mix(col, near.rgb, near.a);
    let atmosphere = textureSampleLevel(atmosphere_texture, layer_sampler, panel_uv(w, 0.60, 1.72), 0.0);
    return mix(col, atmosphere.rgb, atmosphere.a);
}

fn camera_engine() -> vec2<f32> {
    let c = view.world_position.xy;
    return vec2<f32>(c.x + room.x * 0.5, room.y * 0.5 - c.y);
}

/// How much fog is between the sky and the play at `p`: slow, wide patches,
/// a little more of it toward the floor. It is nearer than each layer of the
/// sky, so it follows the camera least.
fn fog_amount(p: vec2<f32>) -> f32 {
    if depth.x <= 0.0 {
        return 0.0;
    }
    let t = globals.time;
    let q = p - camera_engine() * 0.15;
    let wide = value_noise(q + vec2<f32>(t * 4.0, 0.0), 420.0, 77u);
    let fine = value_noise(q + vec2<f32>(-t * 2.5, t * 1.5), 170.0, 78u);
    let patches = mix(1.0, 0.35 + 1.3 * (0.65 * wide + 0.35 * fine), depth.y);
    let low = mix(0.85, 1.15, clamp(p.y / room.y, 0.0, 1.0));
    return clamp(depth.x * patches * low, 0.0, 0.92);
}

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let w = mesh.world_position.xy;
    let p = vec2<f32>(w.x + room.x * 0.5, room.y * 0.5 - w.y);
    // How far the air here is into the corrupted state. Air changes as haze,
    // through a lilac mist and not through grey.
    let air = look_air_state(p, front, globals.time);
    let fog = fog_amount(p);
    var veil = vec4<f32>(0.0);
    if air > 0.001 {
        let mist = vec3<f32>(0.800, 0.640, 0.860);
        let sky = pow(corrupt_sky(w), vec3<f32>(1.0 / 2.2));
        let through = smoothstep(0.35, 1.0, air);
        veil = vec4<f32>(mix(mist, sky, through), max(smoothstep(0.0, 0.55, air) * 0.85, through));
        // The fog of corrupted air is lilac.
        veil = vec4<f32>(mix(veil.rgb, vec3<f32>(0.430, 0.255, 0.570), fog), veil.a);
    }
    // Over the clean sky, which the parallax system drew, the fog is pale.
    let pale = vec4<f32>(0.965, 0.958, 0.950, fog * (1.0 - veil.a));
    let alpha = veil.a + pale.a;
    if alpha <= 0.0 {
        return vec4<f32>(0.0);
    }
    let col = (veil.rgb * veil.a + pale.rgb * pale.a) / alpha;
    // Display values to linear.
    return vec4<f32>(pow(max(col, vec3<f32>(0.0)), vec3<f32>(2.2)), alpha);
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// (`CompositingSpace::Srgb`) the shaded colour is written sRGB-encoded, as
// Bevy's own sprite and mesh shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
