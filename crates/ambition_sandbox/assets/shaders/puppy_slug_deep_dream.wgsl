#import bevy_sprite_render::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> uv_rect: vec4<f32>;
@group(2) @binding(1) var<uniform> control: vec4<f32>;
@group(2) @binding(2) var<uniform> tint: vec4<f32>;
@group(2) @binding(3) var puppy_texture: texture_2d<f32>;
@group(2) @binding(4) var puppy_sampler: sampler;

const PI: f32 = 3.141592653589793;

fn hash21(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q.x + q.y) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i + vec2<f32>(0.0, 0.0));
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var amp = 0.5;
    var total = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 5; i = i + 1) {
        total += noise(p) * amp;
        norm += amp;
        p = mat2x2<f32>(1.62, 1.11, -1.04, 1.71) * p + vec2<f32>(3.7, 1.9);
        amp *= 0.53;
    }
    return total / max(norm, 0.001);
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let p = abs(fract(c.xxx + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - vec3<f32>(3.0));
    return c.z * mix(vec3<f32>(1.0), clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

fn atlas_uv(local_uv_in: vec2<f32>) -> vec2<f32> {
    var local_uv = local_uv_in;
    if control.y > 0.5 {
        local_uv.x = 1.0 - local_uv.x;
    }
    let atlas_min = uv_rect.xy;
    let atlas_max = uv_rect.zw;
    let texel = vec2<f32>(1.0) / vec2<f32>(textureDimensions(puppy_texture));
    return mix(atlas_min + texel, atlas_max - texel, clamp(local_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}

fn sample_frame(local_uv: vec2<f32>) -> vec4<f32> {
    return textureSample(puppy_texture, puppy_sampler, atlas_uv(local_uv));
}

fn dream_rgb(local_uv: vec2<f32>, base_rgb: vec3<f32>, time: f32, strength: f32, seed: f32) -> vec3<f32> {
    let centred = local_uv - vec2<f32>(0.5, 0.5);
    let radius = length(centred);
    let angle = atan2(centred.y, centred.x);

    let fold_count = 5.0 + floor(seed * 4.0);
    let folded_angle = abs(fract(angle / (2.0 * PI) * fold_count + 0.5) - 0.5) * 2.0 * PI;
    let folded = vec2<f32>(cos(folded_angle), sin(folded_angle)) * radius;
    let dream_field = fbm(folded * 8.0 + vec2<f32>(time * 0.55 + seed, -time * 0.32));
    let fine_field = fbm(local_uv * 26.0 + vec2<f32>(seed * 9.0, time * 1.7));

    let melt_gate = smoothstep(0.28, 0.92, dream_field + local_uv.y * 0.34);
    let drip = vec2<f32>(
        sin((local_uv.y * 18.0 + time * 2.2 + seed) * PI) * 0.018,
        (melt_gate * melt_gate) * (0.08 + 0.04 * sin(time * 1.9 + local_uv.x * 10.0))
    ) * strength;
    let warped_uv = clamp(local_uv + drip + (fine_field - 0.5) * 0.030 * strength, vec2<f32>(0.0), vec2<f32>(1.0));

    let split = (0.010 + 0.018 * dream_field) * strength;
    let red = sample_frame(clamp(warped_uv + vec2<f32>(split, -split * 0.35), vec2<f32>(0.0), vec2<f32>(1.0))).r;
    let green = sample_frame(warped_uv).g;
    let blue = sample_frame(clamp(warped_uv - vec2<f32>(split * 0.7, split), vec2<f32>(0.0), vec2<f32>(1.0))).b;
    var rgb = mix(base_rgb, vec3<f32>(red, green, blue), 0.55 * strength);

    let hue = fract(dream_field * 0.82 + fine_field * 0.33 + time * 0.09 + seed);
    let rainbow = hsv2rgb(vec3<f32>(hue, 0.88, 1.15));
    let luma = dot(base_rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(rgb, rainbow * (0.45 + luma * 0.95), 0.78 * strength);

    let ridge = 1.0 - smoothstep(0.020, 0.120, abs(fract((dream_field + fine_field) * 6.0) - 0.5));
    rgb += rainbow * ridge * 0.42 * strength;
    return rgb;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let time = control.x;
    let strength = clamp(control.z, 0.0, 1.0);
    let seed = control.w * 17.0;

    var uv = mesh.uv;
    var base = sample_frame(uv);
    let flipped_y_uv = vec2<f32>(mesh.uv.x, 1.0 - mesh.uv.y);
    let flipped_y_base = sample_frame(flipped_y_uv);
    if flipped_y_base.a > base.a {
        uv = flipped_y_uv;
        base = flipped_y_base;
    }

    // Prefer the source sprite alpha as the mask. If the material quad is
    // alive but atlas UVs are still wrong, keep a tiny rectangular diagnostic
    // shimmer instead of disappearing completely; that makes UV/material bugs
    // visible while the full-screen menu effect provides the reference look.
    let edge_fade = smoothstep(0.00, 0.08, mesh.uv.x)
        * smoothstep(0.00, 0.08, mesh.uv.y)
        * smoothstep(0.00, 0.08, 1.0 - mesh.uv.x)
        * smoothstep(0.00, 0.08, 1.0 - mesh.uv.y);
    let diagnostic_alpha = edge_fade * 0.16;
    let mask_alpha = max(base.a, diagnostic_alpha);
    if mask_alpha <= 0.010 {
        discard;
    }

    let base_rgb = mix(vec3<f32>(0.16, 0.10, 0.30), base.rgb, step(0.015, base.a));
    let rgb = dream_rgb(uv, base_rgb, time, strength, seed);

    let dissolve_noise = fbm(uv * 14.0 + vec2<f32>(time * -0.8, time * 0.45 + seed));
    let dissolve_wave = 0.5 + 0.5 * sin(time * 2.6 + uv.y * 13.0 + seed * 6.0);
    let dissolve = smoothstep(0.18, 0.74, dissolve_noise + dissolve_wave * 0.35);
    let overlay_alpha = mask_alpha * mix(0.48, 0.92, dissolve) * strength;

    return vec4<f32>(clamp(rgb * tint.rgb, vec3<f32>(0.0), vec3<f32>(1.6)), overlay_alpha * tint.a);
}
