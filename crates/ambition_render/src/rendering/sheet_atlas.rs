//! Shared helpers for turning a baked [`SheetRecord`] into a Bevy
//! [`TextureAtlasLayout`] + looking up its animation rows by name.
//!
//! These were private to `shrine_visuals`; lifted here so any animated-prop /
//! effect visual driven by a generated spritesheet (the shrine obelisk, the
//! `robot_slash` melee effect, …) shares one record→atlas path instead of
//! re-deriving frame rects. The record carries each frame's exact rect (which
//! already includes the sheet's label-column offset), so this works for any
//! sheet layout — uniform grids and label-padded sheets alike.

use ambition_sprite_sheet::SheetRecord;
use bevy::image::TextureAtlasLayout;
use bevy::math::{URect, UVec2};

/// Per-frame inset (px) trimmed off each atlas cell to avoid neighbour bleed
/// when the sprite is scaled. One pixel is enough at our frame sizes.
const FRAME_INSET: u32 = 1;

/// Build a [`TextureAtlasLayout`] whose textures are the record's frames in
/// flat row-major order (row 0 frames, then row 1, …) — matching
/// [`row_start_index`]'s flattening.
pub(crate) fn atlas_layout_from_record(record: &SheetRecord) -> TextureAtlasLayout {
    let mut textures = Vec::new();
    let mut total_w = 1u32;
    let mut total_h = 1u32;

    for row in &record.rows {
        for rect in row.rects.iter().take(row.frame_count as usize) {
            let rect = frame_rect_to_urect(rect).expect("sheet frame rect must be non-negative");
            let rect = inset_rect(rect, FRAME_INSET);
            total_w = total_w.max(rect.max.x);
            total_h = total_h.max(rect.max.y);
            textures.push(rect);
        }
    }

    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
    for rect in textures {
        layout.add_texture(rect);
    }
    layout
}

/// Flat atlas index of the first frame of `animation` (rows flattened in
/// order), or `None` if the sheet has no such row.
pub(crate) fn row_start_index(record: &SheetRecord, animation: &str) -> Option<usize> {
    let mut flat = 0usize;
    for row in &record.rows {
        if row.animation == animation {
            return Some(flat);
        }
        flat += row.frame_count as usize;
    }
    None
}

pub(crate) fn row_frame_count(record: &SheetRecord, animation: &str) -> Option<usize> {
    record
        .rows
        .iter()
        .find(|row| row.animation == animation)
        .map(|row| row.frame_count as usize)
}

pub(crate) fn row_duration(record: &SheetRecord, animation: &str) -> Option<f32> {
    record
        .rows
        .iter()
        .find(|row| row.animation == animation)
        .map(|row| row.duration_secs)
}

fn frame_rect_to_urect(rect: &ambition_sprite_sheet::FrameRect) -> Option<URect> {
    let x = u32::try_from(rect.x).ok()?;
    let y = u32::try_from(rect.y).ok()?;
    let w = u32::try_from(rect.w).ok()?;
    let h = u32::try_from(rect.h).ok()?;
    Some(URect {
        min: UVec2::new(x, y),
        max: UVec2::new(x + w, y + h),
    })
}

fn inset_rect(rect: URect, inset: u32) -> URect {
    let inset = inset.min(rect.width().min(rect.height()) / 4);
    URect {
        min: UVec2::new(rect.min.x + inset, rect.min.y + inset),
        max: UVec2::new(rect.max.x - inset, rect.max.y - inset),
    }
}
