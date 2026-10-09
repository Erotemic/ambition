// The collision truth of a room, drawn as a drawing. See `room_look.rs`.
//
// A solid is a closed outline with a hatched fill. A one-way platform has a
// bright top line and an open underside, and marks below it point up: a body
// goes up through it. All work is in engine world coordinates (y down).
// Colours are written as display values and converted once, at the end.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::{view, globals}
#import ambition_content::room_look::{rand_cell, value_noise, towers, arcade, sky_line_distance, pmod}
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> piece: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> room: vec4<f32>;

const PAPER_TOP: vec3<f32> = vec3<f32>(0.028, 0.052, 0.112);
const PAPER_BOTTOM: vec3<f32> = vec3<f32>(0.043, 0.088, 0.168);
const LINE_SOLID: vec3<f32> = vec3<f32>(0.300, 0.780, 1.000);
const LINE_ONE_WAY: vec3<f32> = vec3<f32>(0.360, 0.960, 0.600);
const LINE_FAR: vec3<f32> = vec3<f32>(0.140, 0.340, 0.620);
const PATH: vec3<f32> = vec3<f32>(0.980, 0.820, 0.300);
const NODE: vec3<f32> = vec3<f32>(0.860, 0.970, 1.000);

/// 1.0 on a line `width` wide at distance `d` from its centre.
fn stroke(d: f32, width: f32) -> f32 {
    return smoothstep(width + 0.6, width - 0.2, d);
}

fn camera_engine() -> vec2<f32> {
    let c = view.world_position.xy;
    return vec2<f32>(c.x + room.x * 0.5, room.y * 0.5 - c.y);
}

fn line_of_kind() -> vec3<f32> {
    return select(LINE_SOLID, LINE_ONE_WAY, room.w > 0.5);
}

// -------------------------------------------------------------- backdrop --

/// 1.0 inside the far architecture: towers and a viaduct.
fn far_shape(q: vec2<f32>) -> f32 {
    return max(towers(q, 300.0, 20u, room.y), arcade(q));
}

/// The height of the path of lane `lane` at `x`.
fn path_height(x: f32, lane: f32) -> f32 {
    let r = rand_cell(vec2<f32>(lane, 0.0), 50u);
    let r2 = rand_cell(vec2<f32>(lane, 1.0), 51u);
    return (lane + 0.5) * 260.0 + (r - 0.5) * 70.0
        + 58.0 * sin(x * 0.011 + r * 6.283)
        + 24.0 * sin(x * 0.031 + r2 * 6.283);
}

fn backdrop(p: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let cam = camera_engine();
    var col = mix(PAPER_TOP, PAPER_BOTTOM, clamp(p.y / room.y, 0.0, 1.0));
    // The grid of the drawing, minor and major.
    if pmod(p.x, 16.0) < 1.0 || pmod(p.y, 16.0) < 1.0 {
        col = col + vec3<f32>(0.010, 0.026, 0.048);
    }
    if pmod(p.x, 64.0) < 1.0 || pmod(p.y, 64.0) < 1.0 {
        col = col + vec3<f32>(0.022, 0.058, 0.100);
    }
    // Far architecture, as outlines only.
    let q = p - cam * 0.45;
    let here = far_shape(q);
    let edge = abs(here - far_shape(q + vec2<f32>(1.6, 0.0))) + abs(here - far_shape(q + vec2<f32>(0.0, 1.6)));
    col = col + LINE_FAR * (min(edge, 1.0) * 0.75 + here * 0.045);
    // A construction figure.
    let figure = sky_line_distance(p - cam * 0.30, t * 0.05);
    col = col + LINE_FAR * stroke(figure, 0.5) * 0.9;

    // Paths: dashed lines that march, with nodes, and one packet on each.
    let lane = floor(p.y / 260.0);
    let h = path_height(p.x, lane);
    let slope = (path_height(p.x + 1.0, lane) - h);
    let d = abs(p.y - h) / sqrt(1.0 + slope * slope);
    let dash = step(fract(p.x / 18.0 - t * 0.9), 0.55);
    col = mix(col, PATH, stroke(d, 0.7) * dash * 0.85);
    let nx = round(p.x / 170.0) * 170.0;
    let to_node = length(p - vec2<f32>(nx, path_height(nx, lane)));
    col = mix(col, NODE, stroke(abs(to_node - 6.0), 0.7));
    col = mix(col, vec3<f32>(1.0), stroke(to_node, 1.8));
    col = col + NODE * exp(-to_node / 9.0) * 0.16;
    let px = fract(t * 0.06 + rand_cell(vec2<f32>(lane, 2.0), 52u)) * (room.x + 400.0) - 200.0;
    let to_packet = length(p - vec2<f32>(px, path_height(px, lane)));
    col = col + (PATH + vec3<f32>(0.2)) * (stroke(to_packet, 2.2) + exp(-to_packet / 11.0) * 0.45);
    return col;
}

// --------------------------------------------------------------- surface --

fn surface(p: vec2<f32>) -> vec4<f32> {
    let l = p - piece.xy;
    let s = piece.zw;
    let tone = line_of_kind();
    let one_way = room.w > 0.5;
    // Signed distance to the block edge. Negative is inside.
    let q = abs(l - s * 0.5) - s * 0.5;
    let sd = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
    if sd > 0.0 {
        // The line lights the paper beside it.
        return vec4<f32>(tone, exp(-sd / 3.5) * 0.30);
    }
    var col = mix(vec3<f32>(0.030, 0.070, 0.140), tone * 0.22, select(0.35, 0.55, one_way));
    var alpha = select(0.90, 0.62, one_way);
    // Tile lines, then the hatch that says "material".
    if pmod(p.x, 16.0) < 1.0 || pmod(p.y, 16.0) < 1.0 {
        col = col + tone * 0.060;
    }
    if !one_way && pmod(p.x + p.y, 9.0) < 1.0 {
        col = col + tone * 0.090;
    }
    col = col + tone * exp(sd / 6.0) * 0.22;
    // The outline. A one-way platform is closed on top only: the other
    // three sides are dashed.
    var outline = stroke(-sd, 0.8);
    if one_way {
        let top = l.y < 2.2;
        let dashed = step(fract((l.x + l.y) / 8.0), 0.5);
        outline = select(outline * dashed * 0.65, 1.0, top);
    }
    col = mix(col, tone, outline);
    alpha = max(alpha, outline);
    // A node at each corner.
    let corner = abs(abs(l - s * 0.5) - s * 0.5);
    if corner.x < 3.0 && corner.y < 3.0 {
        col = NODE;
        alpha = 1.0;
    }
    return vec4<f32>(col, alpha);
}

// ------------------------------------------------------------- underside --

fn underside(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    let l = p - piece.xy;
    let w = piece.z;
    if l.x < 0.0 || l.x >= w || l.y < 0.0 {
        return vec4<f32>(0.0);
    }
    let tone = line_of_kind();
    let fade = pow(clamp(1.0 - l.y / piece.w, 0.0, 1.0), 1.6);
    // A dashed drop line from each end of the platform.
    let end = min(l.x, w - 1.0 - l.x);
    var a = stroke(end, 0.5) * step(fract(l.y / 10.0 - t * 0.5), 0.5) * fade * 0.7;
    if room.w > 0.5 {
        // Marks that rise: a body goes up through this platform.
        let dx = pmod(l.x, 44.0) - 22.0;
        let yy = pmod(l.y + t * 16.0, 26.0);
        let mark = abs(abs(dx) * 0.9 - (yy - 8.0));
        let near_fade = clamp(1.0 - l.y / 78.0, 0.0, 1.0);
        a = max(a, stroke(mark, 0.6) * step(abs(dx), 7.0) * near_fade * 0.75);
    } else if l.y < 9.0 {
        // The ground symbol below a solid.
        a = max(a, step(pmod(l.x + l.y, 8.0), 1.0) * 0.55);
    }
    return vec4<f32>(tone, a);
}

// ---------------------------------------------------------------- portal --

/// The trigger box of a door: the volume a press reads, as a marching dashed
/// outline with brackets at its corners.
fn portal(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    let l = p - piece.xy;
    let s = piece.zw;
    let q = abs(l - s * 0.5) - s * 0.5;
    let sd = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
    if sd > 0.0 {
        return vec4<f32>(PATH, exp(-sd / 3.0) * 0.16);
    }
    // March along the outline, in one direction.
    let along = select(l.x - l.y, l.y - l.x, (l.x < 1.5) || (l.y > s.y - 1.5));
    let dashed = step(fract(along / 9.0 - t * 0.8), 0.5);
    var a = stroke(-sd, 0.6) * dashed * 0.9;
    let corner = abs(abs(l - s * 0.5) - s * 0.5);
    if min(corner.x, corner.y) < 1.4 && max(corner.x, corner.y) < 9.0 {
        a = 1.0;
    }
    return vec4<f32>(PATH, max(a, 0.055));
}

// --------------------------------------------------------------- overlay --

fn overlay(p: vec2<f32>) -> vec4<f32> {
    let t = globals.time;
    // A register mark at each major grid crossing.
    let gx = pmod(p.x + 32.0, 64.0) - 32.0;
    let gy = pmod(p.y + 32.0, 64.0) - 32.0;
    let mark = (abs(gx) < 4.5 && abs(gy) < 0.6) || (abs(gy) < 4.5 && abs(gx) < 0.6);
    var col = NODE;
    var a = select(0.0, 0.34, mark);
    // A scan that goes down the room.
    let scan_y = fract(t * 0.07) * (room.y + 200.0) - 100.0;
    let behind = scan_y - p.y;
    if behind > 0.0 {
        a = max(a, exp(-behind / 1.6) * 0.40 + exp(-behind / 70.0) * 0.050);
        col = mix(col, LINE_SOLID, 0.6);
    }
    return vec4<f32>(col, a);
}

// ------------------------------------------------------------------ main --

fn shade(mesh: VertexOutput) -> vec4<f32> {
    let w = mesh.world_position.xy;
    let p = vec2<f32>(w.x + room.x * 0.5, room.y * 0.5 - w.y);
    let role = room.z;
    var col: vec4<f32>;
    if role < 0.5 {
        col = vec4<f32>(backdrop(p), 1.0);
    } else if role < 1.5 {
        col = surface(p);
    } else if role < 2.5 {
        col = overlay(p);
    } else if role < 3.5 {
        col = underside(p);
    } else {
        col = portal(p);
    }
    // Display values to linear.
    return vec4<f32>(pow(max(col.rgb, vec3<f32>(0.0)), vec3<f32>(2.2)), col.a);
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
