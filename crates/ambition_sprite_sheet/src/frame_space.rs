//! ART SPACE → BODY SPACE: the one legal crossing, and the sheet fact that
//! makes it legal.
//!
//! Every gameplay rectangle a generator publishes — a hurtbox, an attack
//! hitbox, a body box — is a **frame pixel**: a coordinate in the sheet's own
//! artwork. Artwork has a handedness. [`SheetRecord::authored_faces_left`] is
//! the sheet's record of which way its drawing points, and a frame pixel cannot
//! be turned into a gameplay offset without it: a blade drawn to the left of
//! the feet is *forward* for a left-drawn sheet and *backward* for a
//! right-drawn one, and the pixels are identical either way.
//!
//! The renderer had that term (`flip_x = (facing < 0) XOR authored_faces_left`)
//! and the geometry paths did not, because each one re-derived the mirror from
//! `facing` alone. That is why Pointed Polygon's jab came out behind her: her
//! art is drawn facing left, her jab poly sits at `x < feet_x` as *forward*,
//! and a consumer reading `facing` by itself has no way to know that.
//!
//! So this module owns both halves and hands out no way to skip either:
//!
//! - [`SheetRecord::art_is_mirrored`] is the single mirror decision. The
//!   renderer's `flip_x` and every geometry path read it, so a sheet cannot be
//!   mirrored by one and not the other.
//! - [`FrameToBody`] is the only public frame-pixel → body-local map, and it is
//!   constructed from a `&SheetRecord`. There is no constructor that takes a
//!   handedness (so none can be passed wrong) and none that omits one (so none
//!   can be forgotten). A caller holding only a `facing` cannot build one.
//!
//! **Body-local** is the frame the rest of combat already speaks
//! (`VolumeShape::place_at`): origin at the body centre, `+x` toward the body's
//! committed facing, `+y` toward its feet. Facing and gravity are applied by
//! whoever places the result — never here, and never twice.

use crate::{AnimationBox, AnimationMetrics, NamedPixelRect, PixelRect, SheetRecord};
use ambition_platformer2d_core as ae;

/// THE mirror decision: whether art drawn facing `authored_faces_left` is drawn
/// mirrored for a body facing `facing`.
///
/// `flip_x` for the renderer, the `+x` sign for every pixel→geometry map. It
/// asks *"does the requested facing differ from the facing this art was drawn
/// in"*, which is the only form that stays right for a sheet whose neutral pose
/// points `-x`.
///
/// It takes the flag rather than a record because the drawn facing reaches the
/// renderer through a `CharacterSheetSpec` and reaches geometry through a
/// `SheetRecord` — two carriers, one decision. Splitting the decision to match
/// the carriers is exactly how the geometry half came to be missing it.
pub fn art_is_mirrored(authored_faces_left: bool, facing: f32, gravity_dir: ae::Vec2) -> bool {
    ambition_platformer2d_shared_tangle::gravity::gravity_aware_flip_x(facing, gravity_dir)
        ^ authored_faces_left
}

impl SheetRecord {
    /// [`art_is_mirrored`] for this sheet.
    pub fn art_is_mirrored(&self, facing: f32, gravity_dir: ae::Vec2) -> bool {
        art_is_mirrored(self.authored_faces_left, facing, gravity_dir)
    }

    /// The sign that carries an art-space `+x` offset into body-space FORWARD.
    ///
    /// `-1` for a left-drawn sheet: its art's `+x` runs toward the body's back.
    /// This is the whole of the handedness, and [`FrameToBody`] is the only
    /// thing that should ever apply it.
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
/// The precedence, in one place: a per-frame sample outranks the coarse
/// per-animation box (so a moving part tracks the drawn pose), and within
/// either, an authored hull outranks rectangles (so a blade arc stays an arc).
/// Both the character attack path and the boss volume path resolve through
/// this, which is what stops them drifting apart.
#[derive(Debug, Clone, Copy)]
pub enum SampledBox<'a> {
    /// An authored convex hull, in frame pixels.
    Poly(&'a [(f32, f32)]),
    /// Rectangles: named parts (possibly each with their own hull) plus the
    /// coarse single-rect fallback.
    Rects(&'a [NamedPixelRect], Option<PixelRect>),
}

impl<'a> SampledBox<'a> {
    /// [`sample`], as a constructor — the spelling a consumer reads best.
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
/// `None` when the row publishes no `frame_duration_secs`, which is every sheet
/// that authors one shape for the whole animation — a caller then takes the
/// coarse box, unchanged. Clamping to the authored samples is
/// [`FrameToBody::volume`]'s job, so a row may legitimately return an index
/// past its own data.
pub fn frame_at(metrics: &AnimationMetrics, elapsed_s: f32) -> Option<usize> {
    let duration = metrics.frame_duration_secs?;
    if duration <= 0.0 {
        return None;
    }
    Some((elapsed_s.max(0.0) / duration).floor() as usize)
}

/// A sheet's frame pixels, as body-local offsets for one body.
///
/// Built from the record, so the handedness is not a parameter anyone can get
/// wrong. Cheap and `Copy` — build one per query.
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
    /// A sheet whose `feet_pixel` plants at the body's toward-gravity face —
    /// the anchor the character renderer uses, so the box lands on the drawn
    /// blade.
    ///
    /// `render_size` is the drawn sprite quad in world units; `collision` is
    /// the body's collision box. Falls back to bottom-centre when the sheet
    /// published no `feet_pixel`, which is what the renderer assumes too.
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

    /// Every body-local volume `box_` authors for `frame`. Multi-part
    /// silhouettes stay several volumes; a part that authored its own hull IS
    /// that hull.
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

    /// The single body-local volume for a consumer that carries one shape —
    /// the character attack path. A multi-part box collapses to the union of
    /// its pieces, which is the honest reading of "one volume for all of this".
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
