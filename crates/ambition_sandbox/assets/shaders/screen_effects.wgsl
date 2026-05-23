#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct ScreenEffectSettings {
    control: vec4<f32>,
    strengths: vec4<f32>,
    crt: vec4<f32>,
    grain_and_vignette: vec4<f32>,
    robot: vec4<f32>,
    underwater: vec4<f32>,
}

@group(0) @binding(2) var<uniform> settings: ScreenEffectSettings;

const PI: f32 = 3.14159265;

fn hash_u32(value: u32) -> u32 {
    var h = value;
    h = h ^ (h >> 16u);
    h = h * 0x7feb352du;
    h = h ^ (h >> 15u);
    h = h * 0x846ca68bu;
    h = h ^ (h >> 16u);
    return h;
}

fn rand_cell(cell: vec2<f32>, frame: f32, salt: u32) -> f32 {
    let x = u32(max(floor(cell.x), 0.0));
    let y = u32(max(floor(cell.y), 0.0));
    let f = u32(max(floor(frame), 0.0));
    let n = (x * 1597334677u)
        ^ (y * 3812015801u)
        ^ (f * 277803737u)
        ^ (salt * 668265263u);
    return f32(hash_u32(n) >> 8u) * (1.0 / 16777216.0);
}

fn triangular_noise(cell: vec2<f32>, frame: f32, salt: u32) -> f32 {
    return rand_cell(cell, frame, salt) + rand_cell(cell, frame, salt + 101u) - 1.0;
}

fn safe_uv(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, vec2<f32>(0.001, 0.001), vec2<f32>(0.999, 0.999));
}

fn sample_screen(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(screen_texture, texture_sampler, safe_uv(uv));
}

fn screen_size() -> vec2<f32> {
    return vec2<f32>(textureDimensions(screen_texture));
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

fn apply_robot_scanlines(color: vec3<f32>, uv: vec2<f32>, amount: f32, time: f32) -> vec3<f32> {
    let fast = sin((uv.y * 960.0 + time * 18.0) * PI);
    let slow = sin((uv.y * 240.0 - time * 2.0) * PI);
    let mask = 1.0 - amount * (0.35 + 0.35 * fast + 0.10 * slow);
    return color * clamp(mask, 0.25, 1.2);
}

fn robot_death_uv(uv: vec2<f32>, time: f32, amount: f32, tear_amount: f32) -> vec2<f32> {
    var out_uv = uv;
    let band = floor(uv.y * 84.0);
    let tick = floor(time * 24.0);
    let band_noise = rand_cell(vec2<f32>(band, 0.0), tick, 41u);
    let tear_gate = step(0.86, band_noise);
    let tear = (band_noise - 0.5) * tear_amount * tear_gate;
    let jitter = (rand_cell(vec2<f32>(floor(uv.y * 320.0), 0.0), tick + 19.0, 67u) - 0.5) * 0.004;
    out_uv.x += amount * (jitter + tear * 0.035);
    out_uv.y += amount * sin((uv.y * 58.0) + time * 24.0) * 0.0015;
    return out_uv;
}

fn underwater_uv(uv: vec2<f32>, time: f32, amount: f32) -> vec2<f32> {
    var out_uv = uv;
    out_uv.x += sin(uv.y * 30.0 + time * 2.8) * 0.008 * amount;
    out_uv.x += sin(uv.y * 79.0 - time * 1.4) * 0.003 * amount;
    out_uv.y += cos(uv.x * 25.0 + time * 2.1) * 0.005 * amount;
    return out_uv;
}


fn dream_hash21(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q.x + q.y) * 43758.5453123);
}

fn dream_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = dream_hash21(i + vec2<f32>(0.0, 0.0));
    let b = dream_hash21(i + vec2<f32>(1.0, 0.0));
    let c = dream_hash21(i + vec2<f32>(0.0, 1.0));
    let d = dream_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn dream_fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var amp = 0.5;
    var total = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 5; i = i + 1) {
        total += dream_noise(p) * amp;
        norm += amp;
        p = mat2x2<f32>(1.62, 1.11, -1.04, 1.71) * p + vec2<f32>(3.7, 1.9);
        amp *= 0.53;
    }
    return total / max(norm, 0.001);
}

fn dream_hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let p = abs(fract(c.xxx + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - vec3<f32>(3.0));
    return c.z * mix(vec3<f32>(1.0), clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

fn apply_deep_dream_screen(rgb_in: vec3<f32>, uv: vec2<f32>, time: f32, strength: f32) -> vec3<f32> {
    let centred = uv - vec2<f32>(0.5, 0.5);
    let radius = length(centred);
    let angle = atan2(centred.y, centred.x);
    let fold_count = 7.0;
    let folded_angle = abs(fract(angle / (2.0 * PI) * fold_count + 0.5) - 0.5) * 2.0 * PI;
    let folded = vec2<f32>(cos(folded_angle), sin(folded_angle)) * radius;

    let dream_field = dream_fbm(folded * 7.0 + vec2<f32>(time * 0.42, -time * 0.28));
    let fine_field = dream_fbm(uv * 28.0 + vec2<f32>(time * 1.3, time * -0.9));
    let melt_gate = smoothstep(0.25, 0.92, dream_field + uv.y * 0.30);
    let drip = vec2<f32>(
        sin((uv.y * 18.0 + time * 2.1) * PI) * 0.020,
        melt_gate * melt_gate * (0.060 + 0.050 * sin(time * 1.7 + uv.x * 10.0))
    ) * strength;
    let warped_uv = safe_uv(uv + drip + (fine_field - 0.5) * 0.035 * strength);

    let split = (0.006 + 0.018 * dream_field) * strength;
    let chroma_rgb = vec3<f32>(
        sample_screen(warped_uv + vec2<f32>(split, -split * 0.35)).r,
        sample_screen(warped_uv).g,
        sample_screen(warped_uv - vec2<f32>(split * 0.7, split)).b
    );

    let hue = fract(dream_field * 0.82 + fine_field * 0.33 + time * 0.08);
    let rainbow = dream_hsv2rgb(vec3<f32>(hue, 0.86, 1.12));
    let luma = dot(rgb_in, vec3<f32>(0.299, 0.587, 0.114));
    var rgb = mix(rgb_in, chroma_rgb, 0.45 * strength);
    rgb = mix(rgb, rainbow * (0.38 + luma * 0.95), 0.68 * strength);

    let ridge = 1.0 - smoothstep(0.020, 0.120, abs(fract((dream_field + fine_field) * 6.0) - 0.5));
    rgb += rainbow * ridge * 0.34 * strength;

    // Soft reappearing/melting rhythm, but never full-screen blinks to black.
    let dissolve_noise = dream_fbm(uv * 12.0 + vec2<f32>(time * -0.7, time * 0.4));
    let dissolve_wave = 0.5 + 0.5 * sin(time * 2.3 + uv.y * 13.0);
    let dissolve = smoothstep(0.18, 0.74, dissolve_noise + dissolve_wave * 0.35);
    return mix(rgb_in, rgb, mix(0.42, 0.92, dissolve) * strength);
}

fn crt_warp(uv: vec2<f32>, amount: f32) -> vec2<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let r2 = dot(centered, centered);
    let warped = centered * (1.0 + vec2<f32>(0.050, 0.065) * r2 * amount);
    return warped * 0.5 + vec2<f32>(0.5, 0.5);
}

fn crt_soft_sample(uv: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
    let c = sample_screen(uv).rgb;
    let h = sample_screen(uv + vec2<f32>(texel.x, 0.0)).rgb
        + sample_screen(uv - vec2<f32>(texel.x, 0.0)).rgb;
    let v = sample_screen(uv + vec2<f32>(0.0, texel.y)).rgb
        + sample_screen(uv - vec2<f32>(0.0, texel.y)).rgb;
    return c * 0.64 + h * 0.11 + v * 0.07;
}

fn crt_beam(scan_y: f32, amount: f32) -> f32 {
    let row_wave = 0.5 + 0.5 * sin(scan_y * PI);
    let bright_beam = pow(row_wave, 0.55);
    let dark_gap = 1.0 - pow(1.0 - row_wave, 1.7);
    let beam = mix(dark_gap, bright_beam, 0.55);
    return mix(1.0, 0.50 + 0.62 * beam, amount);
}

fn crt_shadow_mask(pixel: vec2<f32>, amount: f32) -> vec3<f32> {
    let column = u32(floor(pixel.x)) % 3u;
    var mask = vec3<f32>(0.72, 0.72, 0.72);
    if column == 0u {
        mask = vec3<f32>(1.22, 0.70, 0.70);
    } else if column == 1u {
        mask = vec3<f32>(0.70, 1.18, 0.70);
    } else {
        mask = vec3<f32>(0.72, 0.72, 1.24);
    }

    let slot = u32(floor(pixel.y * 0.5)) % 2u;
    if slot == 1u {
        mask *= vec3<f32>(0.91, 0.91, 0.91);
    }

    return mix(vec3<f32>(1.0), mask, clamp(amount, 0.0, 1.0));
}

fn crt_color(
    source_uv: vec2<f32>,
    mask_uv: vec2<f32>,
    strength: f32,
    scanline_setting: f32,
    mask_setting: f32,
    curvature_setting: f32,
    bloom_setting: f32,
    chroma_setting: f32,
) -> vec3<f32> {
    let size = screen_size();
    let texel = 1.0 / size;
    let curvature = curvature_setting * strength;
    let warped_uv = crt_warp(source_uv, curvature);
    let base = sample_screen(source_uv).rgb;

    let in_bounds = step(0.0, warped_uv.x)
        * step(0.0, warped_uv.y)
        * step(warped_uv.x, 1.0)
        * step(warped_uv.y, 1.0);

    let centered = warped_uv - vec2<f32>(0.5, 0.5);
    let edge = length(centered) * 1.65;
    let chroma = chroma_setting * strength * 0.010 * (0.45 + edge);
    let dir = normalize(centered + vec2<f32>(0.0001, -0.0001));
    let offset = dir * chroma;

    var rgb = vec3<f32>(
        crt_soft_sample(warped_uv + offset, texel).r,
        crt_soft_sample(warped_uv, texel).g,
        crt_soft_sample(warped_uv - offset, texel).b
    );

    let glow = sample_screen(warped_uv + texel * vec2<f32>(1.5, 1.0)).rgb
        + sample_screen(warped_uv + texel * vec2<f32>(-1.5, 1.0)).rgb
        + sample_screen(warped_uv + texel * vec2<f32>(1.5, -1.0)).rgb
        + sample_screen(warped_uv + texel * vec2<f32>(-1.5, -1.0)).rgb;
    let glow_rgb = glow * 0.25;
    rgb += glow_rgb * glow_rgb * (bloom_setting * strength * 0.18);

    let pixel = mask_uv * size;
    let beam = crt_beam(pixel.y, scanline_setting * strength);
    let mask = crt_shadow_mask(pixel, mask_setting * strength);
    rgb *= beam * mask;

    rgb = (rgb - vec3<f32>(0.5)) * (1.0 + 0.16 * strength) + vec3<f32>(0.5);
    rgb *= 1.0 + 0.16 * strength;

    let corner = smoothstep(0.70, 1.08, max(abs(centered.x) * 1.18, abs(centered.y)) * 2.0);
    rgb *= 1.0 - corner * 0.40 * strength;
    rgb *= in_bounds;

    return clamp(mix(base, rgb, clamp(strength, 0.0, 1.0)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_film_grain(
    rgb: vec3<f32>,
    pixel: vec2<f32>,
    time: f32,
    amount: f32,
    grain_size: f32,
    grain_fps: f32,
    luma_bias: f32,
) -> vec3<f32> {
    if amount <= 0.001 {
        return rgb;
    }

    let cell_size = max(grain_size, 1.0);
    let cell = floor(pixel / vec2<f32>(cell_size, cell_size));
    let frame = floor(time * max(grain_fps, 1.0));

    let mono = triangular_noise(cell, frame, 503u);
    let channel_noise = vec3<f32>(
        triangular_noise(cell, frame, 613u),
        triangular_noise(cell, frame, 877u),
        triangular_noise(cell, frame, 1193u)
    );
    let noise = mix(vec3<f32>(mono), channel_noise, 0.22);

    let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    let shadow_weight = 1.0 - smoothstep(0.10, 0.75, luma);
    let mid_weight = clamp(1.0 - abs(luma - 0.5) * 2.0, 0.0, 1.0);
    let response = mix(1.0, 0.72 + shadow_weight * 0.48 + mid_weight * 0.20, luma_bias);
    let amplitude = amount * 0.16 * clamp(response, 0.35, 1.55);

    return rgb + noise * amplitude;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let global_strength = clamp(settings.control.x, 0.0, 1.0);
    let time = settings.control.y;
    let grain_fps = settings.control.z;
    let grain_size = settings.control.w;

    if global_strength <= 0.001 {
        return sample_screen(in.uv);
    }

    let crt_strength = clamp(settings.strengths.x, 0.0, 1.0);
    let film_grain_strength = clamp(settings.strengths.y, 0.0, 1.0);
    let robot_strength = clamp(settings.strengths.z, 0.0, 1.0);
    let underwater_strength = clamp(settings.strengths.w, 0.0, 1.0);

    let crt_scanlines = clamp(settings.crt.x, 0.0, 1.0);
    let crt_mask = clamp(settings.crt.y, 0.0, 1.0);
    let crt_curvature = clamp(settings.crt.z, 0.0, 1.0);
    let crt_bloom = clamp(settings.crt.w, 0.0, 1.0);

    let grain_luma_bias = clamp(settings.grain_and_vignette.x, 0.0, 1.0);
    let vignette_strength = clamp(settings.grain_and_vignette.y, 0.0, 1.0);
    let crt_chroma = clamp(settings.grain_and_vignette.z, 0.0, 1.0);

    let robot_static = clamp(settings.robot.x, 0.0, 1.0);
    let robot_tear = clamp(settings.robot.y, 0.0, 1.0);
    let robot_desaturate = clamp(settings.robot.z, 0.0, 1.0);
    let robot_scanlines = clamp(settings.robot.w, 0.0, 1.0);

    let underwater_distortion = clamp(settings.underwater.x, 0.0, 1.0);
    let deep_dream_strength = clamp(settings.underwater.y, 0.0, 1.0);

    var uv = in.uv;
    if underwater_strength > 0.001 {
        uv = underwater_uv(uv, time, underwater_strength * underwater_distortion);
    }
    if robot_strength > 0.001 {
        uv = robot_death_uv(uv, time, robot_strength, robot_tear);
    }

    var rgb: vec3<f32>;
    if crt_strength > 0.001 {
        rgb = crt_color(
            uv,
            in.uv,
            crt_strength,
            crt_scanlines,
            crt_mask,
            crt_curvature,
            crt_bloom,
            crt_chroma
        );
    } else {
        var split_amount = 0.0;
        if robot_strength > 0.001 {
            split_amount = 0.0045 * robot_strength;
        }
        rgb = rgb_split(uv, split_amount).rgb;
    }

    if underwater_strength > 0.001 {
        rgb = mix(rgb, rgb * vec3<f32>(0.74, 0.96, 1.06), 0.55 * underwater_strength);
        rgb = mix(rgb, vec3<f32>(0.08, 0.22, 0.28), 0.07 * underwater_strength);
    }

    if deep_dream_strength > 0.001 {
        rgb = apply_deep_dream_screen(rgb, uv, time, deep_dream_strength);
    }

    let size = screen_size();
    let pixel = floor(in.uv * size);

    if robot_strength > 0.001 {
        rgb = apply_film_grain(
            rgb,
            pixel,
            time,
            robot_strength * robot_static,
            1.0,
            24.0,
            0.25
        );
        rgb = apply_robot_scanlines(rgb, in.uv, robot_scanlines * robot_strength, time);

        let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
        rgb = mix(rgb, vec3<f32>(luma), robot_desaturate * robot_strength);
    }

    if film_grain_strength > 0.001 {
        rgb = apply_film_grain(
            rgb,
            pixel,
            time,
            film_grain_strength,
            grain_size,
            grain_fps,
            grain_luma_bias
        );
    }

    if vignette_strength > 0.001 {
        rgb = apply_vignette(rgb, in.uv, vignette_strength);
    }

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
