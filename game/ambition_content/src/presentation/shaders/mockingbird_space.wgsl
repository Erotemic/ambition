// Where the Mockingbird's fight goes: the sky becomes space.
//
// One quad over the room, above the sky's own backdrop. As the fight climbs
// (`room.z`, 0 to 1) the dark comes down from the top and the stars come out,
// and the planet's limb falls away to the bottom of the room. In space the
// moon's lane is drawn as a faint dashed arc, and the moon crosses on it. All
// work is in engine world coordinates (y down).
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

// xy: the quad's corner; zw: its size.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> rect: vec4<f32>;
// xy: the room's size; z: how far the sky has become space; w: time.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> room: vec4<f32>;
// xy: the moon's centre; z: its radius; w: 1 while it is in the room.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> moon: vec4<f32>;
// x: where the moon comes in; y: where it goes out; z: the height of the
// lane's ends; w: how far its middle is above them.
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> lane: vec4<f32>;
// x: the sky's scroll in px/s.
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> sky: vec4<f32>;

const PI: f32 = 3.14159265;

fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn hash2(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash(p), hash(p + vec2<f32>(19.19, 73.31)));
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = p - i;
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    return noise(p) * 0.55 + noise(p * 2.07 + vec2<f32>(5.1, 2.3)) * 0.28 + noise(p * 4.21) * 0.17;
}

// One layer of stars: one star in each cell of `cell` px, some cells empty.
fn stars(p: vec2<f32>, cell: f32, t: f32) -> f32 {
    let g = p / cell;
    let i = floor(g);
    let r = hash2(i);
    let at = (i + vec2<f32>(0.15) + r * 0.7) * cell;
    let d = length(p - at);
    let bright = hash(i + vec2<f32>(7.0, 3.0));
    let lit = step(0.55, bright);
    let twinkle = 0.75 + 0.25 * sin(t * (1.0 + bright * 3.0) + bright * 40.0);
    let size = 0.9 + bright * 1.6;
    return lit * twinkle * smoothstep(size, 0.0, d) * (0.45 + 0.55 * bright);
}

// The moon's face at `q`, a point of its disc in -1..1.
fn moon_face(q: vec2<f32>) -> vec3<f32> {
    let r = length(q);
    // A sphere lit from the upper left.
    let z = sqrt(max(1.0 - r * r, 0.0));
    let light = clamp(dot(vec3<f32>(q.x, q.y, z), normalize(vec3<f32>(-0.45, -0.55, 0.70))), 0.0, 1.0);
    // Seas: large dark patches.
    let seas = smoothstep(0.50, 0.72, fbm(q * 2.3 + vec2<f32>(11.0, 4.0)));
    var tone = mix(0.86, 0.60, seas);
    // Craters: a dark bowl and a bright rim in some cells.
    let g = q * 5.0;
    let i = floor(g);
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let c = i + vec2<f32>(f32(dx), f32(dy));
            let h = hash2(c + vec2<f32>(2.0, 9.0));
            let size = 0.16 + 0.30 * hash(c + vec2<f32>(31.0, 17.0));
            let d = length(g - (c + vec2<f32>(0.2) + h * 0.6)) / size;
            let bowl = 1.0 - smoothstep(0.55, 0.95, d);
            let rim = smoothstep(0.80, 1.0, d) * (1.0 - smoothstep(1.0, 1.25, d));
            let has = step(0.42, hash(c + vec2<f32>(3.0, 3.0)));
            tone = tone - has * bowl * 0.13 + has * rim * 0.10;
        }
    }
    let shade = 0.22 + 0.78 * light;
    return vec3<f32>(0.95, 0.94, 0.88) * tone * shade;
}

fn shade(in: VertexOutput) -> vec4<f32> {
    let p = rect.xy + in.uv * rect.zw;
    let size = room.xy;
    let ascent = clamp(room.z, 0.0, 1.0);
    let t = room.w;
    let v = p.y / size.y;

    // The dark comes down from the top as the fight climbs.
    let dark = smoothstep(0.0, 1.0, ascent * 2.3 - v * 1.3);
    var colour = mix(vec3<f32>(0.020, 0.030, 0.095), vec3<f32>(0.004, 0.005, 0.020), clamp(1.0 - v, 0.0, 1.0));
    // A band of dust across the sky.
    let band = fbm(vec2<f32>(p.x * 0.0016 + p.y * 0.0012, p.y * 0.0034 - p.x * 0.0008));
    colour += vec3<f32>(0.060, 0.030, 0.110) * smoothstep(0.50, 0.85, band);
    // Stars, in three sizes. Far ones drift with the sky, slowly.
    let drift = vec2<f32>(sky.x * t * 0.012, 0.0);
    let star = stars(p + drift, 46.0, t) + stars(p + drift * 0.6 + vec2<f32>(13.0, 57.0), 83.0, t) * 1.3
        + stars(p + drift * 0.3 + vec2<f32>(91.0, 7.0), 151.0, t) * 1.7;
    colour += vec3<f32>(0.92, 0.95, 1.0) * star * dark * dark;
    var alpha = dark;

    // The planet's limb. Its top falls from the middle of the room to its
    // bottom as the fight climbs, and curves more.
    let climb = smoothstep(0.25, 1.0, ascent);
    let radius = size.x * mix(4.0, 1.25, climb);
    let top = size.y * mix(0.62, 0.90, climb);
    let centre = vec2<f32>(size.x * 0.5, top + radius);
    let d = length(p - centre) - radius;
    let planet = smoothstep(0.30, 0.75, ascent);
    if (d < 0.0) {
        // The planet: sea and cloud, which go by as the sky scrolls.
        let ground = (p - centre) * 0.0016 + vec2<f32>(-sky.x * t * 0.00012, 0.0);
        let cloud = smoothstep(0.48, 0.74, fbm(ground * 3.0 + vec2<f32>(0.0, 7.0)));
        let land = smoothstep(0.56, 0.62, fbm(ground * 1.3 + vec2<f32>(40.0, 2.0)));
        var surface = mix(vec3<f32>(0.020, 0.110, 0.300), vec3<f32>(0.060, 0.220, 0.110), land);
        surface = mix(surface, vec3<f32>(0.80, 0.86, 0.92), cloud * 0.85);
        // Lit near its limb by the air, darker into its body.
        let depth = clamp(-d / (size.y * 0.5), 0.0, 1.0);
        surface = surface * mix(1.0, 0.45, depth) + vec3<f32>(0.10, 0.30, 0.60) * exp(d / 26.0);
        colour = mix(colour, surface, planet);
        alpha = max(alpha, planet);
    } else {
        // The air: a thin bright line on the limb and a glow above it.
        let air = exp(-d / 34.0) * 0.85 + exp(-d / 150.0) * 0.30;
        colour += vec3<f32>(0.22, 0.56, 1.0) * air * planet;
        alpha = max(alpha, clamp(air, 0.0, 1.0) * planet);
    }

    // The moon's lane: dashes that run the way the moon goes, in space only.
    let in_space = smoothstep(0.92, 1.0, ascent);
    let u = (lane.x - p.x) / (lane.x - lane.y);
    if (u > 0.0 && u < 1.0) {
        let lane_y = lane.z - lane.w * sin(u * PI);
        let on_line = 1.0 - smoothstep(1.5, 3.5, abs(p.y - lane_y));
        let dash = step(0.5, fract(p.x / 56.0 + t * 0.6));
        colour += vec3<f32>(0.80, 0.84, 0.74) * on_line * dash * 0.22 * in_space;
    }

    // The moon.
    if (moon.w > 0.5) {
        let q = (p - moon.xy) / moon.z;
        let r = length(q);
        let halo = exp(-max(r - 1.0, 0.0) * 3.2) * 0.42;
        colour += vec3<f32>(0.78, 0.82, 0.92) * halo * step(1.0, r);
        alpha = max(alpha, halo * step(1.0, r));
        let disc = 1.0 - smoothstep(0.985, 1.0, r);
        colour = mix(colour, moon_face(q), disc);
        alpha = max(alpha, disc);
    }

    return vec4<f32>(colour, clamp(alpha, 0.0, 1.0));
}

// The camera blends in the space its main texture stores: under `SRGB_OUTPUT`
// the shaded colour is written sRGB-encoded, as Bevy's own sprite and mesh
// shaders write it.
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let colour = shade(mesh);
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(colour.rgb), colour.a);
#else
    return colour;
#endif
}
