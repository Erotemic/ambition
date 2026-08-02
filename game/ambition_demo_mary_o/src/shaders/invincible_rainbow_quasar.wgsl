#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_rect: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> detail: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var color_sampler: sampler;

const PI: f32 = 3.141592653589793;
const TAU: f32 = 6.283185307179586;

fn hash21(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q.x + q.y) * 43758.5453123);
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let p = abs(fract(c.xxx + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - vec3<f32>(3.0));
    return c.z * mix(
        vec3<f32>(1.0),
        clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)),
        c.y
    );
}

fn atlas_uv(local_uv_in: vec2<f32>) -> vec2<f32> {
    var local_uv = local_uv_in;
    if control.y > 0.5 {
        local_uv.x = 1.0 - local_uv.x;
    }
    return mix(uv_rect.xy, uv_rect.zw, clamp(local_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}

fn sample_frame(local_uv: vec2<f32>) -> vec4<f32> {
    return textureSample(color_texture, color_sampler, atlas_uv(local_uv));
}

fn silhouette_edge(local_uv: vec2<f32>, alpha: f32) -> f32 {
    let texel = max(detail.xy, vec2<f32>(0.0001));
    let left = sample_frame(local_uv - vec2<f32>(texel.x, 0.0)).a;
    let right = sample_frame(local_uv + vec2<f32>(texel.x, 0.0)).a;
    let up = sample_frame(local_uv - vec2<f32>(0.0, texel.y)).a;
    let down = sample_frame(local_uv + vec2<f32>(0.0, texel.y)).a;
    let neighbour_min = min(min(left, right), min(up, down));
    return clamp(alpha - neighbour_min, 0.0, 1.0);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let time = control.x;
    let strength = clamp(control.z, 0.0, 1.0);
    let seed = control.w * 19.73;
    let local_uv = mesh.uv;
    let base = sample_frame(local_uv);

    if base.a <= 0.010 {
        discard;
    }

    let centred = local_uv - vec2<f32>(0.5, 0.5);
    let radius = length(centred);
    let angle = atan2(centred.y, centred.x);

    // Fast travelling spectral ribbons, bent by a rotating quasar field.
    let spiral = sin(angle * 5.0 - time * 4.8 + radius * 28.0 + seed) * 0.055;
    let ribbon_hue = fract(
        time * 0.38
        + local_uv.x * 0.70
        + local_uv.y * 0.42
        + radius * 0.34
        + spiral
        + seed * 0.07
    );
    let rainbow = hsv2rgb(vec3<f32>(ribbon_hue, 0.92, 1.25));

    // Concentric quasar shock rings repeatedly expand through the sprite.
    let ring_phase = fract(radius * 4.8 - time * 0.82 + seed * 0.11);
    let ring = 1.0 - smoothstep(0.035, 0.150, abs(ring_phase - 0.5));

    // Tiny deterministic stellar flashes live inside the silhouette.
    let star_grid = vec2<f32>(13.0, 17.0);
    let cell = floor(local_uv * star_grid);
    let star_seed = hash21(cell + vec2<f32>(floor(seed), floor(seed * 1.7)));
    let star_gate = smoothstep(0.91, 0.985, star_seed);
    let star_cycle = max(0.0, sin(time * 7.5 + star_seed * TAU));
    let star = star_gate * pow(star_cycle, 18.0);

    let scan = 0.5 + 0.5 * sin((local_uv.x + local_uv.y) * 34.0 - time * 10.0 + seed);
    let pulse = 0.5 + 0.5 * sin(time * 5.6 + seed * 0.3);
    let edge = silhouette_edge(local_uv, base.a);

    let base_luma = dot(base.rgb, vec3<f32>(0.299, 0.587, 0.114));
    var quasar_rgb = rainbow * (0.68 + base_luma * 0.82);
    quasar_rgb += rainbow * ring * (0.35 + 0.25 * pulse);
    quasar_rgb += vec3<f32>(1.0, 1.0, 0.92) * star * 1.8;
    quasar_rgb += rainbow * scan * 0.10;

    // Keep Mary-O's pixel-art linework legible while the costume churns through
    // rainbow energy. The edge itself gets a bright pearl/cyan corona.
    let darkest_channel = max(base.r, max(base.g, base.b));
    let preserve_ink = 1.0 - smoothstep(0.035, 0.145, darkest_channel);
    var rgb = mix(base.rgb, quasar_rgb, strength * 0.92);
    rgb = mix(rgb, base.rgb, preserve_ink * 0.92);
    rgb += edge * mix(vec3<f32>(1.0, 0.96, 0.72), rainbow, 0.42) * (0.75 + 0.35 * pulse);

    let overlay_alpha = base.a * detail.w * strength * (0.76 + 0.16 * pulse);
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.65)), overlay_alpha);
}
