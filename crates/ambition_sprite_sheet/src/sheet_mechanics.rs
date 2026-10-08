//! The mechanical projection of a sheet record: the part of it the simulation
//! reads, for the content identity.
//!
//! A sprite-authored body takes its collision box, its per-pose boxes and its
//! attack geometry from its sheet record. Published sprites are not in version
//! control, so two machines at one revision can hold different records. The
//! content fingerprint must see that difference in the mechanics, and must not
//! see a difference in packing alone (`Q122`): the atlas images, pages, row
//! bands and frame rects only say where a frame is drawn from.
//!
//! In the projection: the key, the target, the frame size, the body metrics
//! (boxes, parts, polygons, feet, per-animation hurtboxes, hitboxes and frame
//! durations), the tuning, the drawn facing, and each row's animation, frame
//! count, durations and mirror. Out of it: `image`, `images`, `label_width`,
//! `y_offset`, and each row's `row_index`, `page` and `rects`.

use crate::{
    AnimationBox, AnimationBoxFrame, AnimationMetrics, BodyMetrics, NamedPixelRect, PixelRect,
    SheetRecord, SheetRow,
};

/// The digest of the mechanical projection of `records`, in the order given.
/// Give them in key order: the digest is of a sequence.
pub fn sheet_mechanics_digest<'a>(records: impl IntoIterator<Item = &'a SheetRecord>) -> String {
    let mut hasher = blake3::Hasher::new();
    let mut bytes = Vec::new();
    for record in records {
        bytes.clear();
        write_record(record, &mut bytes);
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    hasher.finalize().to_hex().to_string()
}

/// Every variable-length field is length-prefixed and every optional field
/// is tagged, so one field cannot be read as part of the next.
fn write_record(record: &SheetRecord, out: &mut Vec<u8>) {
    put_str(out, &record.key);
    put_str(out, &record.target);
    put_u32(out, record.frame_width);
    put_u32(out, record.frame_height);
    put_opt(out, record.body_metrics.as_ref(), write_body_metrics);
    put_opt(out, record.tuning.as_ref(), |out, tuning| {
        put_f32(out, tuning.collision_scale);
        put_u32(out, tuning.frame_sample_inset);
    });
    put_bool(out, record.authored_faces_left);
    put_seq(out, &record.rows, write_row);
}

fn write_row(out: &mut Vec<u8>, row: &SheetRow) {
    put_str(out, &row.animation);
    put_u32(out, row.frame_count);
    put_u32(out, row.duration_ms);
    put_f32(out, row.duration_secs);
    put_opt(out, row.mirror_of.as_ref(), |out, mirror| put_str(out, mirror));
}

fn write_body_metrics(out: &mut Vec<u8>, metrics: &BodyMetrics) {
    put_opt(out, metrics.body_pixel_bbox.as_ref(), write_rect);
    put_seq(out, &metrics.body_pixel_parts, write_named_rect);
    put_u64(out, metrics.animations.len() as u64);
    for (name, animation) in &metrics.animations {
        put_str(out, name);
        write_animation(out, animation);
    }
    put_opt(out, metrics.feet_pixel.as_ref(), |out, feet| {
        put_f32(out, feet.x);
        put_f32(out, feet.y);
    });
    put_opt(out, metrics.feet_anchor_norm.as_ref(), |out, anchor| {
        put_f32(out, anchor.x);
        put_f32(out, anchor.y);
    });
    put_bool(out, metrics.authored_body);
}

fn write_animation(out: &mut Vec<u8>, animation: &AnimationMetrics) {
    put_opt(out, animation.frame_duration_secs.as_ref(), |out, secs| put_f32(out, *secs));
    put_opt(out, animation.hurtbox.as_ref(), write_box);
    put_opt(out, animation.hitbox.as_ref(), write_box);
}

fn write_box(out: &mut Vec<u8>, volume: &AnimationBox) {
    put_seq(out, &volume.parts, write_named_rect);
    put_opt(out, volume.bbox.as_ref(), write_rect);
    put_seq(out, &volume.poly, write_point);
    put_seq(out, &volume.frames, write_box_frame);
}

fn write_box_frame(out: &mut Vec<u8>, frame: &AnimationBoxFrame) {
    put_seq(out, &frame.parts, write_named_rect);
    put_opt(out, frame.bbox.as_ref(), write_rect);
    put_seq(out, &frame.poly, write_point);
}

fn write_rect(out: &mut Vec<u8>, rect: &PixelRect) {
    for value in [rect.x, rect.y, rect.w, rect.h] {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn write_named_rect(out: &mut Vec<u8>, rect: &NamedPixelRect) {
    put_str(out, &rect.name);
    write_rect(out, &rect.rect());
    put_seq(out, &rect.poly, write_point);
}

fn write_point(out: &mut Vec<u8>, point: &(f32, f32)) {
    put_f32(out, point.0);
    put_f32(out, point.1);
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn put_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn put_str(out: &mut Vec<u8>, value: &str) {
    put_u64(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

fn put_opt<T>(out: &mut Vec<u8>, value: Option<&T>, write: impl FnOnce(&mut Vec<u8>, &T)) {
    match value {
        Some(value) => {
            out.push(1);
            write(out, value);
        }
        None => out.push(0),
    }
}

fn put_seq<T>(out: &mut Vec<u8>, values: &[T], write: impl Fn(&mut Vec<u8>, &T)) {
    put_u64(out, values.len() as u64);
    for value in values {
        write(out, value);
    }
}

#[cfg(test)]
mod tests;
