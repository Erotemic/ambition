#define_import_path ambition_render::frame_in_sprite

// A body's frame inside the image its root sprite shows (`FrameInSprite`), for
// a shader drawn on the root's quad that patterns over the BODY. A baked frame
// is its whole image; a composited body's image is a square impostor cell with
// the frame inside a margin, where a wide body is a thin band and a pattern laid
// over the whole quad is magnified onto it.
//
// `frame_rect` is the frame's box in the image (min.xy, max.xy; +y down,
// unflipped); `flipped`, whether the sprite mirrors its image. Frame
// coordinates run 0..1 across the frame and past it where the quad has margin.

// The frame's corner on the quad (mesh uv), mirrored with the image.
fn frame_min(frame_rect: vec4<f32>, flipped: bool) -> vec2<f32> {
    if flipped {
        return vec2<f32>(1.0 - frame_rect.z, frame_rect.y);
    }
    return frame_rect.xy;
}

fn frame_size(frame_rect: vec4<f32>) -> vec2<f32> {
    return max(frame_rect.zw - frame_rect.xy, vec2<f32>(1e-4));
}

// The frame coordinate of a point of the quad.
fn frame_uv(quad_uv: vec2<f32>, frame_rect: vec4<f32>, flipped: bool) -> vec2<f32> {
    return (quad_uv - frame_min(frame_rect, flipped)) / frame_size(frame_rect);
}

// The point of the quad at a frame coordinate, kept on the quad.
fn quad_uv(frame_uv: vec2<f32>, frame_rect: vec4<f32>, flipped: bool) -> vec2<f32> {
    let at = frame_min(frame_rect, flipped) + frame_uv * frame_size(frame_rect);
    return clamp(at, vec2<f32>(0.0), vec2<f32>(1.0));
}

// A frame coordinate kept on the quad.
fn on_quad(frame_uv: vec2<f32>, frame_rect: vec4<f32>, flipped: bool) -> vec2<f32> {
    let lo = -frame_min(frame_rect, flipped) / frame_size(frame_rect);
    let hi = (vec2<f32>(1.0) - frame_min(frame_rect, flipped)) / frame_size(frame_rect);
    return clamp(frame_uv, lo, hi);
}
