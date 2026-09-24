//! Art space to body space: the one legal conversion, and the sheet fact it
//! needs.
//!
//! Every gameplay rectangle a generator publishes (hurtbox, hitbox, body box)
//! is in frame pixels, a coordinate in the sheet's artwork. Artwork has a
//! handedness, recorded in [`SheetRecord::authored_faces_left`]. A blade drawn
//! left of the feet is forward for a left-drawn sheet and backward for a
//! right-drawn one, with identical pixels. So `facing` alone cannot convert a
//! frame pixel to a gameplay offset.
//!
//! This module owns both parts, and callers cannot skip either:
//!
//! - [`SheetRecord::art_is_mirrored`] is the one mirror decision. The
//!   renderer's `flip_x` and every geometry path read it.
//! - [`FrameToBody`] is the only public frame-pixel to body-local map. It is
//!   built from a `&SheetRecord`; no constructor takes or omits a handedness,
//!   so a caller with only `facing` cannot build one.
//!
//! Body-local is the frame combat already uses (`VolumeShape::place_at`):
//! origin at the body centre, `+x` toward the body's facing, `+y` toward its
//! feet. The caller that places the result applies facing and gravity, once.

use crate::{AnimationBox, AnimationMetrics, NamedPixelRect, PixelRect, SheetRecord};
use ambition_platformer2d_core as ae;

/// The mirror decision: whether art drawn facing `authored_faces_left` is
/// mirrored for a body facing `facing`.
///
/// This gives the renderer's `flip_x` and the `+x` sign for every
/// pixel-to-geometry map. It asks whether the requested facing differs from
/// the drawn facing, which stays correct for a sheet whose neutral pose points
/// `-x`.
///
/// It takes the flag, not a record, because the renderer gets the drawn facing
/// from a `CharacterSheetSpec` and geometry gets it from a `SheetRecord`. Both
/// must use this one decision.
pub fn art_is_mirrored(authored_faces_left: bool, facing: f32, gravity_dir: ae::Vec2) -> bool {
    ambition_platformer2d_shared_tangle::gravity::gravity_aware_flip_x(facing, gravity_dir)
        ^ authored_faces_left
}

impl SheetRecord {
    /// [`art_is_mirrored`] for this sheet.
    pub fn art_is_mirrored(&self, facing: f32, gravity_dir: ae::Vec2) -> bool {
        art_is_mirrored(self.authored_faces_left, facing, gravity_dir)
    }

    /// The sign that maps an art-space `+x` offset to body-space forward.
    ///
    /// `-1` for a left-drawn sheet. Only [`FrameToBody`] should apply it.
    pub fn art_forward_x(&self) -> f32 {
        if self.authored_faces_left {
            -1.0
        } else {
            1.0
        }
    }
}

/// Which authored geometry a consumer should read for the frame being shown.
///
/// Precedence: a per-frame sample outranks the per-animation box (so a moving
/// part tracks the pose), and an authored hull outranks rectangles (so a blade
/// arc stays an arc). The character attack path and the boss volume path both
/// use this, so they agree.
#[derive(Debug, Clone, Copy)]
pub enum SampledBox<'a> {
    /// An authored convex hull, in frame pixels.
    Poly(&'a [(f32, f32)]),
    /// Rectangles: named parts (possibly each with their own hull) plus the
    /// coarse single-rect fallback.
    Rects(&'a [NamedPixelRect], Option<PixelRect>),
}

impl<'a> SampledBox<'a> {
    /// [`sample`] as a constructor.
    pub fn sample(box_: &'a AnimationBox, frame: Option<usize>) -> Option<Self> {
        sample(box_, frame)
    }
}

/// Resolve `box_` to the geometry for `frame`, or `None` when nothing is
/// authored. `frame` clamps to the last authored sample, so a box that
/// out-lives its per-frame data holds its final shape rather than vanishing.
pub fn sample(box_: &AnimationBox, frame: Option<usize>) -> Option<SampledBox<'_>> {
    if let Some(index) = frame {
        if !box_.frames.is_empty() {
            let sample = &box_.frames[index.min(box_.frames.len() - 1)];
            if sample.is_populated() {
                if !sample.poly.is_empty() {
                    return Some(SampledBox::Poly(&sample.poly));
                }
                return Some(SampledBox::Rects(&sample.parts, sample.bbox));
            }
        }
    }
    if !box_.poly.is_empty() {
        return Some(SampledBox::Poly(&box_.poly));
    }
    if box_.parts.is_empty() && box_.bbox.is_none() {
        return None;
    }
    Some(SampledBox::Rects(&box_.parts, box_.bbox))
}

/// Which frame of `metrics` is drawn at `elapsed_s` seconds into the row.
///
/// `None` when the row publishes no `frame_duration_secs` (one shape for the
/// whole animation); the caller then uses the coarse box. The index may be past
/// the authored samples; [`FrameToBody::volume`] clamps it.
pub fn frame_at(metrics: &AnimationMetrics, elapsed_s: f32) -> Option<usize> {
    let duration = metrics.frame_duration_secs?;
    if duration <= 0.0 {
        return None;
    }
    Some((elapsed_s.max(0.0) / duration).floor() as usize)
}

/// A sheet's frame pixels, as body-local offsets for one body.
///
/// Built from the record, so the handedness is not a parameter. Cheap and
/// `Copy`; build one per query.
#[derive(Debug, Clone, Copy)]
pub struct FrameToBody {
    /// `+1` when the art is drawn facing `+x`, `-1` when it is drawn facing
    /// `-x`. Carries an art-space x offset into body-space forward.
    forward: f32,
    /// World units per frame pixel.
    scale: ae::Vec2,
    /// The frame pixel that plants at the body's anchor.
    anchor_px: ae::Vec2,
    /// Body-local position of that anchor. `collision.y * 0.5` for a
    /// feet-planted sheet (the body centre is half a body above its feet).
    anchor_local: ae::Vec2,
}

impl FrameToBody {
    /// A sheet whose `feet_pixel` plants at the body's toward-gravity face.
    /// This is the character renderer's anchor, so the box lands on the drawn
    /// blade.
    ///
    /// `render_size` is the drawn sprite quad in world units; `collision` is
    /// the body's collision box. Without `feet_pixel`, use bottom-centre, as
    /// the renderer does.
    pub fn planting_feet(
        record: &SheetRecord,
        render_size: ae::Vec2,
        collision: ae::Vec2,
    ) -> Self {
        let frame = ae::Vec2::new(
            record.frame_width.max(1) as f32,
            record.frame_height.max(1) as f32,
        );
        let feet = record
            .body_metrics
            .as_ref()
            .and_then(|m| m.feet_pixel)
            .map(|p| ae::Vec2::new(p.x, p.y))
            .unwrap_or(ae::Vec2::new(frame.x * 0.5, frame.y));
        Self {
            forward: record.art_forward_x(),
            scale: ae::Vec2::new(render_size.x / frame.x, render_size.y / frame.y),
            anchor_px: feet,
            anchor_local: ae::Vec2::new(0.0, collision.y * 0.5),
        }
    }

    /// One frame pixel as a body-local offset: `+x` forward, `+y` toward the
    /// feet, origin at the body centre.
    pub fn point(&self, px: f32, py: f32) -> ae::Vec2 {
        ae::Vec2::new(
            self.anchor_local.x + (px - self.anchor_px.x) * self.scale.x * self.forward,
            self.anchor_local.y + (py - self.anchor_px.y) * self.scale.y,
        )
    }

    /// The rectangle `rect` as a body-local AABB.
    pub fn rect(&self, rect: PixelRect) -> ae::Aabb {
        let (cx, cy) = rect.center();
        ae::Aabb::new(
            self.point(cx, cy),
            ae::Vec2::new(
                (rect.w as f32 * 0.5 * self.scale.x).abs(),
                (rect.h as f32 * 0.5 * self.scale.y).abs(),
            ),
        )
    }

    /// Every body-local volume `box_` authors for `frame`. A multi-part
    /// silhouette gives several volumes; a part with its own hull gives that
    /// hull.
    pub fn volumes(&self, box_: &AnimationBox, frame: Option<usize>) -> Vec<ae::CombatVolume> {
        let hull = |poly: &[(f32, f32)]| {
            ae::CombatVolume::convex(poly.iter().map(|(x, y)| self.point(*x, *y)).collect())
        };
        match sample(box_, frame) {
            None => Vec::new(),
            Some(SampledBox::Poly(poly)) => vec![hull(poly)],
            Some(SampledBox::Rects(parts, bbox)) => {
                if parts.is_empty() {
                    return bbox
                        .map(|r| vec![ae::CombatVolume::aabb(self.rect(r))])
                        .unwrap_or_default();
                }
                parts
                    .iter()
                    .map(|part| {
                        if part.poly.is_empty() {
                            ae::CombatVolume::aabb(self.rect(part.rect()))
                        } else {
                            hull(&part.poly)
                        }
                    })
                    .collect()
            }
        }
    }

    /// One body-local volume, for a consumer that uses one shape (the
    /// character attack path). A multi-part box becomes the union of its parts.
    pub fn volume(&self, box_: &AnimationBox, frame: Option<usize>) -> Option<ae::CombatVolume> {
        let mut volumes = self.volumes(box_, frame).into_iter();
        let first = volumes.next()?;
        let Some(second) = volumes.next() else {
            return Some(first);
        };
        let start = first.bounds();
        let (mut min, mut max) = (start.min, start.max);
        for volume in [second].into_iter().chain(volumes) {
            let b = volume.bounds();
            min = min.min(b.min);
            max = max.max(b.max);
        }
        Some(ae::CombatVolume::aabb(ae::Aabb::new(
            (min + max) * 0.5,
            (max - min) * 0.5,
        )))
    }
}

#[cfg(test)]
mod tests;
