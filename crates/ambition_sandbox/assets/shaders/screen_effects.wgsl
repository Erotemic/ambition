#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct ScreenEffectSettings {
    control: vec4<f32>,
    amounts: vec4<f32>,
    modulation: vec4<f32>,
}

@group(0) @binding(2) var<uniform> settings: ScreenEffectSettings;

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    ));
    return fract(sin(q.x + q.y) * 43758.5453);
}

fn safe_uv(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, vec2<f32>(0.001, 0.001), vec2<f32>(0.999, 0.999));
}

fn sample_screen(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(screen_texture, texture_sampler, safe_uv(uv));
}

fn rgb_split(uv: vec2<f32>, chroma: f32) -> vec4<f32> {
    let centered = uv - vec2<f32>(0.5, 0.5);
    let dir = normalize(centered + vec2<f32>(0.001, -0.001));
    let edge = length(centered) * 1.8;
    let offset = dir * chroma * (0.35 + edge);
    return vec4<f32>(
        sample_screen(uv + offset).r,
        sample_screen(uv).g,
        sample_screen(uv - offset).b,
        1.0
    );
}

fn apply_vignette(color: vec3<f32>, uv: vec2<f32>, amount: f32) -> vec3<f32> {
    let d = distance(uv, vec2<f32>(0.5, 0.5));
    let mask = 1.0 - smoothstep(0.22, 0.82, d);
    return color * mix(1.0, mask, clamp(amount, 0.0, 1.0));
}

fn apply_scanlines(color: vec3<f32>, uv: vec2<f32>, amount: f32, time: f32) -> vec3<f32> {
    let fast = sin((uv.y * 960.0 + time * 18.0) * 3.14159265);
    let slow = sin((uv.y * 240.0 - time * 2.0) * 3.14159265);
    let mask = 1.0 - amount * (0.35 + 0.35 * fast + 0.10 * slow);
    return color * clamp(mask, 0.25, 1.2);
}

fn robot_death_uv(uv: vec2<f32>, time: f32, distortion: f32, roll: f32) -> vec2<f32> {
    var out_uv = uv;
    let band = floor(uv.y * 84.0);
    let tick = floor(time * 24.0);
    let band_noise = hash21(vec2<f32>(band, tick));
    let tear_gate = step(0.86, band_noise);
    let tear = (band_noise - 0.5) * roll * tear_gate;
    let jitter = (hash21(vec2<f32>(floor(uv.y * 320.0), tick + 19.0)) - 0.5) * 0.004;
    out_uv.x += distortion * (jitter + tear * 0.035);
    out_uv.y += distortion * sin((uv.y * 58.0) + time * 24.0) * 0.0015;
    return out_uv;
}

fn crt_uv(uv: vec2<f32>, distortion: f32) -> vec2<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let r2 = dot(centered, centered);
    let warped = centered * (1.0 + r2 * 0.075 * distortion);
    return warped * 0.5 + vec2<f32>(0.5, 0.5);
}

fn underwater_uv(uv: vec2<f32>, time: f32, distortion: f32) -> vec2<f32> {
    var out_uv = uv;
    out_uv.x += sin(uv.y * 30.0 + time * 2.8) * 0.008 * distortion;
    out_uv.x += sin(uv.y * 79.0 - time * 1.4) * 0.003 * distortion;
    out_uv.y += cos(uv.x * 25.0 + time * 2.1) * 0.005 * distortion;
    return out_uv;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let preset = settings.control.x;
    let strength = clamp(settings.control.y, 0.0, 1.0);
    let time = settings.control.z;

    if strength <= 0.001 || preset < 0.5 {
        return sample_screen(in.uv);
    }

    let noise_amount = settings.amounts.x * strength;
    let vignette_amount = settings.amounts.y * strength;
    let distortion_amount = settings.amounts.z * strength;
    let scanline_amount = settings.amounts.w * strength;
    let chroma_amount = settings.modulation.x * strength;
    let roll_amount = settings.modulation.y * strength;
    let desaturate_amount = settings.modulation.z * strength;
    let ripple_amount = settings.modulation.w * strength;

    var uv = in.uv;
    if preset < 1.5 {
        uv = robot_death_uv(uv, time, distortion_amount, roll_amount);
    } else if preset < 2.5 {
        uv = crt_uv(uv, distortion_amount);
    } else {
        uv = underwater_uv(uv, time, distortion_amount * ripple_amount);
    }

    var color = rgb_split(uv, chroma_amount);
    var rgb = color.rgb;

    if preset > 2.5 {
        rgb = mix(rgb, rgb * vec3<f32>(0.74, 0.96, 1.06), 0.55 * strength);
        rgb = mix(rgb, vec3<f32>(0.08, 0.22, 0.28), 0.07 * strength);
    }

    let grain_seed = vec2<f32>(floor(in.uv.x * 1280.0), floor(in.uv.y * 720.0));
    let grain = hash21(grain_seed + vec2<f32>(time * 61.0, time * 17.0)) - 0.5;
    rgb += vec3<f32>(grain * noise_amount);

    if scanline_amount > 0.001 {
        rgb = apply_scanlines(rgb, in.uv, scanline_amount, time);
    }

    let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(rgb, vec3<f32>(luma), desaturate_amount);
    rgb = apply_vignette(rgb, in.uv, vignette_amount);

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
