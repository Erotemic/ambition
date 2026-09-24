//! Where a drawable sits relative to one portal pane, for compositing.
//!
//! A z constant cannot answer this. [`crate::PORTAL_WINDOW_Z`] is `9.5`; a
//! generic actor draws at `WORLD_Z_DUMMY + 1.0 = 11.0` and the player at
//! `WORLD_Z_PLAYER = 20.0`. Every actor wins the depth test against every pane,
//! so a far-side actor behind the aperture draws over the captured image that
//! should hide it.
//!
//! The relation is per pane. One body can be near one pane and far of another
//! in the same frame, and one entity z cannot represent both. This module
//! classifies one (pane, drawable) pair. It does not decide how the renderer
//! composites the result. The diagnostics and the compositor use this same
//! answer.

use ambition_platformer2d_core::Vec2;
use ambition_portal2d::PlacedPortal;

/// What a portal pane and one drawable are to each other this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneRelation {
    /// The drawable's bounds do not meet the pane at all — ordinary draw, no
    /// compositing question.
    Disjoint,
    /// On the VIEWER's side of the pane and overlapping it: it may occlude the
    /// aperture, which is what today's global z already gives.
    NearOccluder,
    /// On the far side and overlapping: the pane's captured image should cover
    /// the overlapping pixels. A global z gets this case wrong.
    FarCovered,
    /// Mid-transit: the split here/through presentation owns this body and the
    /// compositor must not add a third copy of it.
    Transiting,
}

/// Classify `drawable` (world-space bounds, as drawn) against one `pane`
/// ([`pane_relation`]).
///
/// `transiting` wins over geometry. A body crossing the plane is already drawn
/// as clipped pieces, and a second classification would add a duplicate copy.
///
/// A drawable exactly on the plane counts as the viewer's side. A body resting
/// against the hole from the room is the ordinary near case, and this bias
/// fails visible rather than invisible.
///
/// The pane's world rect: what it covers, and what is subtracted from a
/// far-side drawable. [`pane_relation`] and the compositor both use this rect.
/// Two spellings of `pos ± half_extent` could leave a hairline of the far body
/// along the pane edge.
pub fn pane_cover_rect(pane: &PlacedPortal) -> (Vec2, Vec2) {
    (pane.pos - pane.half_extent, pane.pos + pane.half_extent)
}

pub fn pane_relation(
    pane: &PlacedPortal,
    viewer: Vec2,
    drawable_min: Vec2,
    drawable_max: Vec2,
    transiting: bool,
) -> PaneRelation {
    if transiting {
        return PaneRelation::Transiting;
    }
    let (pane_min, pane_max) = pane_cover_rect(pane);
    let overlaps = drawable_min.x <= pane_max.x
        && drawable_max.x >= pane_min.x
        && drawable_min.y <= pane_max.y
        && drawable_max.y >= pane_min.y;
    if !overlaps {
        return PaneRelation::Disjoint;
    }

    // Use the portal domain's side test, not a private `.dot(normal)` that
    // could drift from it.
    let frame = pane.frame();
    let centre = (drawable_min + drawable_max) * 0.5;
    let drawable_front = ambition_portal2d::pieces::front_distance(centre, &frame);
    let viewer_front = ambition_portal2d::pieces::front_distance(viewer, &frame);

    let same_side = (drawable_front >= 0.0) == (viewer_front >= 0.0);
    if same_side {
        PaneRelation::NearOccluder
    } else {
        PaneRelation::FarCovered
    }
}

/// Does the drawn ordering match the relation, given the two actual z values?
///
/// This makes the bug countable in the diagnostics. It takes the z values
/// instead of assuming them, so the answer stays correct if a z constant
/// moves. The dump uses this and has no comparison of its own.
pub fn current_z_policy_is_correct_for(
    relation: PaneRelation,
    drawable_z: f32,
    pane_z: f32,
) -> bool {
    let drawn_above = drawable_z > pane_z;
    match relation {
        // Ordering cannot be observed: no overlap, or the split presentation
        // owns the body on its own layers.
        PaneRelation::Disjoint | PaneRelation::Transiting => true,
        // It may occlude the aperture, so being drawn above is correct.
        PaneRelation::NearOccluder => drawn_above,
        // The pane's captured image should cover it, so it must NOT be above.
        PaneRelation::FarCovered => !drawn_above,
    }
}

/// One axis-aligned piece of a drawable that the compositor may draw.
///
/// A piece is a sub-rect of the original sprite. The quad's own extent gives
/// its four outer edges, so this road needs no extra planes for them.
/// [`crate::PortalClipMaterial`] carries three half-planes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UncoveredPiece {
    pub min: Vec2,
    pub max: Vec2,
}

impl UncoveredPiece {
    /// Zero for a degenerate piece, which is why degenerate pieces are dropped
    /// rather than emitted: a zero-area quad is a draw call that shows nothing.
    pub fn area(&self) -> f32 {
        ((self.max.x - self.min.x) * (self.max.y - self.min.y)).max(0.0)
    }

    fn is_degenerate(&self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// Up to four disjoint pieces, held inline because this runs per (far drawable,
/// pane) per frame and the count is bounded by the geometry, not by content.
#[derive(Clone, Copy, Debug, Default)]
pub struct UncoveredPieces {
    pieces: [Option<UncoveredPiece>; 4],
    len: usize,
}

impl UncoveredPieces {
    fn push(&mut self, piece: UncoveredPiece) {
        if piece.is_degenerate() {
            return;
        }
        self.pieces[self.len] = Some(piece);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = UncoveredPiece> + '_ {
        self.pieces[..self.len].iter().filter_map(|p| *p)
    }
}

/// The part of `drawable` that `cover` does NOT hide, as up to four disjoint
/// axis-aligned pieces.
///
/// The covered region is never given to the renderer, so there is no ordering
/// to get wrong. A higher [`crate::PORTAL_WINDOW_Z`] would invert the bug onto
/// near-side bodies, and one actor z cannot serve two panes that disagree.
/// `no_piece_ever_overlaps_the_cover` checks this over a grid of offsets.
///
/// The shape of the decomposition matters: a full-width band below the cover,
/// a full-width band above it, then the left and right pieces of the middle
/// band. Full-width bands keep the four pieces disjoint. Four half-planes would
/// overlap at the corners and draw them twice, which shows on a translucent
/// sprite.
///
/// No overlap gives one whole piece; total cover gives none. The caller needs
/// no special case.
pub fn uncovered_remainder(
    drawable_min: Vec2,
    drawable_max: Vec2,
    cover_min: Vec2,
    cover_max: Vec2,
) -> UncoveredPieces {
    let mut out = UncoveredPieces::default();
    let whole = UncoveredPiece {
        min: drawable_min,
        max: drawable_max,
    };
    if whole.is_degenerate() {
        return out;
    }
    let cover = UncoveredPiece {
        min: cover_min,
        max: cover_max,
    };
    // No overlap: the cover hides nothing of this drawable, so it draws whole.
    // A degenerate cover takes this road too: a zero-area aperture hides
    // nothing, and must not blank the actor.
    if cover.is_degenerate() || !whole.overlaps(&cover) {
        out.push(whole);
        return out;
    }

    // Bands run the full width so the corners belong to exactly one piece.
    out.push(UncoveredPiece {
        min: drawable_min,
        max: Vec2::new(drawable_max.x, cover_min.y.min(drawable_max.y)),
    });
    out.push(UncoveredPiece {
        min: Vec2::new(drawable_min.x, cover_max.y.max(drawable_min.y)),
        max: drawable_max,
    });

    // The middle band is the vertical overlap only; its left and right remain.
    let band_lo = drawable_min.y.max(cover_min.y);
    let band_hi = drawable_max.y.min(cover_max.y);
    out.push(UncoveredPiece {
        min: Vec2::new(drawable_min.x, band_lo),
        max: Vec2::new(cover_min.x.min(drawable_max.x), band_hi),
    });
    out.push(UncoveredPiece {
        min: Vec2::new(cover_max.x.max(drawable_min.x), band_lo),
        max: Vec2::new(drawable_max.x, band_hi),
    });
    out
}

/// The clip half-planes one piece needs, as `(point, inward normal)` in engine
/// world space, ready for `clip_plane_render` to map into the render frame.
///
/// A plane is needed only where the piece's edge differs from the drawable's
/// own edge; the quad supplies the four outer edges. This uses the same rects
/// that [`uncovered_remainder`] produced. At most three are `Some`
/// (`no_piece_needs_more_than_the_materials_three_clip_planes`);
/// [`crate::PortalClipMaterial`] carries exactly three.
pub fn piece_clip_edges(
    piece: &UncoveredPiece,
    drawable_min: Vec2,
    drawable_max: Vec2,
) -> [Option<(Vec2, Vec2)>; 4] {
    let keep = |moved: bool, point: Vec2, normal: Vec2| moved.then_some((point, normal));
    [
        keep(
            piece.min.x > drawable_min.x,
            Vec2::new(piece.min.x, piece.min.y),
            Vec2::X,
        ),
        keep(
            piece.max.x < drawable_max.x,
            Vec2::new(piece.max.x, piece.min.y),
            -Vec2::X,
        ),
        keep(
            piece.min.y > drawable_min.y,
            Vec2::new(piece.min.x, piece.min.y),
            Vec2::Y,
        ),
        keep(
            piece.max.y < drawable_max.y,
            Vec2::new(piece.min.x, piece.max.y),
            -Vec2::Y,
        ),
    ]
}

#[cfg(test)]
mod clip_edge_tests {
    use super::*;

    /// The planes must reconstruct the piece. A normal pointing the wrong way
    /// keeps the complement, and the far body draws where it must not.
    #[test]
    fn the_planes_keep_the_piece_and_reject_outside_it() {
        let (dmin, dmax) = (Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let cover = (Vec2::new(3.0, 3.0), Vec2::new(7.0, 8.0));
        let pieces = uncovered_remainder(dmin, dmax, cover.0, cover.1);
        assert_eq!(pieces.len(), 4, "expected all four pieces for an interior cover");
        for piece in pieces.iter() {
            let edges = piece_clip_edges(&piece, dmin, dmax);
            // Sample the drawable densely; a point survives every active plane
            // exactly when it is inside this piece.
            let mut inside_kept = 0;
            for i in 0..=40 {
                for j in 0..=40 {
                    let p = Vec2::new(i as f32 * 0.25, j as f32 * 0.25);
                    let survives = edges.iter().flatten().all(|(point, normal)| {
                        (p - *point).dot(*normal) >= 0.0
                    });
                    let inside = p.x >= piece.min.x
                        && p.x <= piece.max.x
                        && p.y >= piece.min.y
                        && p.y <= piece.max.y;
                    assert_eq!(
                        survives, inside,
                        "point {p:?} survives={survives} but inside={inside} for {piece:?}"
                    );
                    inside_kept += usize::from(inside);
                }
            }
            assert!(inside_kept > 0, "piece {piece:?} contained no sample point");
        }
    }

    /// The count must match the material's budget. It is checked against the
    /// same pieces the drawing road uses.
    #[test]
    fn a_whole_uncovered_sprite_needs_no_planes_at_all() {
        let (dmin, dmax) = (Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let pieces = uncovered_remainder(dmin, dmax, Vec2::new(50.0, 50.0), Vec2::new(60.0, 60.0));
        let piece = pieces.iter().next().expect("one whole piece");
        let edges = piece_clip_edges(&piece, dmin, dmax);
        assert_eq!(
            edges.iter().flatten().count(),
            0,
            "an unclipped sprite must cost no planes; it is the common case"
        );
    }
}

#[cfg(test)]
mod remainder_tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec2, Vec2) {
        (Vec2::new(x0, y0), Vec2::new(x1, y1))
    }

    fn remainder(d: (Vec2, Vec2), c: (Vec2, Vec2)) -> UncoveredPieces {
        uncovered_remainder(d.0, d.1, c.0, c.1)
    }

    /// For any cover, no emitted piece overlaps it. A far-side body cannot draw
    /// inside the aperture because those pixels are never in a piece.
    #[test]
    fn no_piece_ever_overlaps_the_cover() {
        let d = rect(0.0, 0.0, 10.0, 10.0);
        let mut checked = 0;
        for x in -3..=13 {
            for y in -3..=13 {
                let c = rect(x as f32, y as f32, x as f32 + 4.0, y as f32 + 6.0);
                let cover = UncoveredPiece { min: c.0, max: c.1 };
                for piece in remainder(d, c).iter() {
                    assert!(
                        !piece.overlaps(&cover),
                        "piece {piece:?} overlaps cover {cover:?}"
                    );
                    checked += 1;
                }
            }
        }
        // Anti-vacuity floor: 17x17 offsets, and the interior ones emit four pieces.
        assert!(checked > 400, "only {checked} pieces examined");
    }

    /// The pieces must also TILE the visible part: dropping a piece would pass
    /// the overlap property above while leaving a hole in the actor.
    #[test]
    fn the_pieces_tile_exactly_the_visible_area() {
        let d = rect(0.0, 0.0, 10.0, 10.0);
        for (x, y, w, h) in [
            (2.0f32, 2.0f32, 4.0f32, 6.0f32),
            (-5.0, 3.0, 7.0, 2.0),
            (8.0, -2.0, 5.0, 20.0),
            (0.0, 0.0, 10.0, 5.0),
        ] {
            let c = rect(x, y, x + w, y + h);
            let overlap_w = (x + w).min(10.0) - x.max(0.0);
            let overlap_h = (y + h).min(10.0) - y.max(0.0);
            let hidden = overlap_w.max(0.0) * overlap_h.max(0.0);
            let visible: f32 = remainder(d, c).iter().map(|p| p.area()).sum();
            assert!(
                (visible - (100.0 - hidden)).abs() < 1e-3,
                "cover ({x},{y},{w},{h}): pieces cover {visible}, expected {}",
                100.0 - hidden
            );
        }
    }

    /// Disjointness is not implied by the two properties above. A double-drawn
    /// corner shows as a bright square on a translucent sprite.
    #[test]
    fn the_pieces_never_overlap_each_other() {
        let d = rect(0.0, 0.0, 10.0, 10.0);
        for x in -2..=12 {
            for y in -2..=12 {
                let c = rect(x as f32, y as f32, x as f32 + 3.0, y as f32 + 3.0);
                let pieces: Vec<_> = remainder(d, c).iter().collect();
                for (i, a) in pieces.iter().enumerate() {
                    for b in &pieces[i + 1..] {
                        assert!(!a.overlaps(b), "pieces {a:?} and {b:?} overlap");
                    }
                }
            }
        }
    }

    #[test]
    fn a_cover_that_hides_everything_emits_no_pieces() {
        let out = remainder(rect(1.0, 1.0, 4.0, 4.0), rect(0.0, 0.0, 9.0, 9.0));
        assert!(out.is_empty(), "fully covered drawable emitted {}", out.len());
    }

    /// Both ways "nothing is hidden" arrives take different roads through the
    /// function. A zero-area aperture read as total cover would blank the actor.
    #[test]
    fn nothing_hidden_draws_the_whole_sprite_once() {
        for cover in [
            rect(50.0, 50.0, 60.0, 60.0), // disjoint
            rect(5.0, 5.0, 5.0, 5.0),     // degenerate
        ] {
            let out = remainder(rect(0.0, 0.0, 10.0, 10.0), cover);
            assert_eq!(out.len(), 1, "cover {cover:?} should leave one whole piece");
            let piece = out.iter().next().expect("one piece");
            assert_eq!(piece.min, Vec2::new(0.0, 0.0));
            assert_eq!(piece.max, Vec2::new(10.0, 10.0));
        }
    }

    /// The three-half-plane budget. `clip_piece_transform` scales the quad to
    /// the whole sprite, and every cut is a half-plane in
    /// [`crate::PortalClipMaterial`], which carries three. A piece fits only if it
    /// differs from the drawable's bounds on at most three edges.
    ///
    /// Full-width bands make this true: each band moves one edge, and each middle
    /// piece moves three. This test fails if the decomposition is re-cut.
    #[test]
    fn no_piece_needs_more_than_the_materials_three_clip_planes() {
        let d = rect(0.0, 0.0, 10.0, 10.0);
        let mut worst = 0;
        for x in -3..=13 {
            for y in -3..=13 {
                let c = rect(x as f32, y as f32, x as f32 + 4.0, y as f32 + 6.0);
                for piece in remainder(d, c).iter() {
                    // A plane is needed only where the cover moved an edge in.
                    let planes = [
                        piece.min.x > d.0.x,
                        piece.max.x < d.1.x,
                        piece.min.y > d.0.y,
                        piece.max.y < d.1.y,
                    ]
                    .iter()
                    .filter(|moved| **moved)
                    .count();
                    assert!(
                        planes <= 3,
                        "piece {piece:?} needs {planes} planes; the material has 3"
                    );
                    worst = worst.max(planes);
                }
            }
        }
        // Anti-vacuity: some piece must need all three planes.
        assert_eq!(worst, 3, "no piece exercised the full three-plane budget");
    }

    /// A cover meeting an edge exactly must not emit a zero-area quad: that is
    /// a draw call that shows nothing, and four of them per pane per actor.
    #[test]
    fn an_edge_flush_cover_emits_no_degenerate_piece() {
        let out = remainder(rect(0.0, 0.0, 10.0, 10.0), rect(0.0, 0.0, 10.0, 4.0));
        assert_eq!(out.len(), 1, "expected only the band above");
        let piece = out.iter().next().expect("one piece");
        assert_eq!(piece.min, Vec2::new(0.0, 4.0));
        assert_eq!(piece.max, Vec2::new(10.0, 10.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal2d::{PlacedPortal, PortalChannel};

    /// A pane centred at `at`, facing `normal`, 46x9 like a real aperture.
    fn pane(at: Vec2, normal: Vec2) -> PlacedPortal {
        PlacedPortal {
            channel: PortalChannel::Authored(
                ambition_portal2d::PortalChannelColor::Purple,
            ),
            pos: at,
            normal,
            half_extent: Vec2::new(46.0, 9.0),
            host: None,
            host_lift: 0.0,
            vel: Vec2::ZERO,
            prev_pos: at,
        }
    }

    fn body(centre: Vec2) -> (Vec2, Vec2) {
        let half = Vec2::new(14.0, 23.0);
        (centre - half, centre + half)
    }

    /// A body behind the pane whose sprite overlaps it must be covered by the
    /// captured image. The global-z policy draws it on top.
    #[test]
    fn a_far_side_body_overlapping_the_pane_is_covered_and_the_z_policy_gets_it_wrong() {
        // Pane on a floor facing up (+y is "into the room" here); viewer above.
        let p = pane(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let viewer = Vec2::new(100.0, 360.0);
        let (min, max) = body(Vec2::new(100.0, 292.0)); // below the plane
        let relation = pane_relation(&p, viewer, min, max, false);
        assert_eq!(relation, PaneRelation::FarCovered);
        assert!(
            !current_z_policy_is_correct_for(relation, 11.0, crate::PORTAL_WINDOW_Z),
            "the whole finding is that today's z draws this one on top of the pane"
        );
        // The answer follows the numbers: the same far-side body drawn below the
        // pane is composited correctly.
        assert!(current_z_policy_is_correct_for(relation, 9.0, crate::PORTAL_WINDOW_Z));
    }

    /// The control: a classifier that answered `FarCovered` for everything
    /// would pass the test above.
    #[test]
    fn a_near_side_body_overlapping_the_pane_may_occlude_it() {
        let p = pane(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let viewer = Vec2::new(100.0, 360.0);
        let (min, max) = body(Vec2::new(100.0, 308.0)); // viewer's side
        let relation = pane_relation(&p, viewer, min, max, false);
        assert_eq!(relation, PaneRelation::NearOccluder);
        assert!(current_z_policy_is_correct_for(relation, 11.0, crate::PORTAL_WINDOW_Z));
        // A near-side body drawn below the pane is wrong too: the aperture it
        // stands in front of would hide it.
        assert!(!current_z_policy_is_correct_for(relation, 9.0, crate::PORTAL_WINDOW_Z));
    }

    #[test]
    fn a_body_that_does_not_meet_the_pane_is_disjoint() {
        let p = pane(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let (min, max) = body(Vec2::new(900.0, 300.0));
        assert_eq!(
            pane_relation(&p, Vec2::new(100.0, 360.0), min, max, false),
            PaneRelation::Disjoint
        );
    }

    /// Transit wins over geometry: the split presentation already draws this
    /// body, so classifying it too would add a third copy.
    #[test]
    fn a_transiting_body_belongs_to_the_split_presentation_whatever_its_bounds() {
        let p = pane(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let (min, max) = body(Vec2::new(100.0, 292.0)); // would be FarCovered
        assert_eq!(
            pane_relation(&p, Vec2::new(100.0, 360.0), min, max, true),
            PaneRelation::Transiting
        );
    }

    /// One body, two panes, different answers.
    ///
    /// The body sits between two facing apertures: in front of one and behind
    /// the other, in the same frame. Any fix that stores one ordering per actor
    /// fails here.
    #[test]
    fn one_body_is_near_one_pane_and_far_of_another_in_the_same_frame() {
        let viewer = Vec2::new(100.0, 360.0);
        let (min, max) = body(Vec2::new(100.0, 300.0));

        // Floor pane below the body, facing up: the body is in front of it.
        let below = pane(Vec2::new(100.0, 288.0), Vec2::new(0.0, 1.0));
        // Ceiling pane above the body, facing DOWN: the body is behind it.
        let above = pane(Vec2::new(100.0, 312.0), Vec2::new(0.0, -1.0));

        assert_eq!(
            pane_relation(&below, viewer, min, max, false),
            PaneRelation::NearOccluder,
            "the body is on the viewer's side of the floor pane"
        );
        assert_eq!(
            pane_relation(&above, viewer, min, max, false),
            PaneRelation::FarCovered,
            "the SAME body is behind the ceiling pane — one entity z cannot say both"
        );
    }

    /// The relation depends only on geometry, so build order must not change it.
    #[test]
    fn the_relation_does_not_depend_on_which_pane_is_asked_first() {
        let viewer = Vec2::new(100.0, 360.0);
        let (min, max) = body(Vec2::new(100.0, 300.0));
        let below = pane(Vec2::new(100.0, 288.0), Vec2::new(0.0, 1.0));
        let above = pane(Vec2::new(100.0, 312.0), Vec2::new(0.0, -1.0));

        let first = [
            pane_relation(&below, viewer, min, max, false),
            pane_relation(&above, viewer, min, max, false),
        ];
        let second = [
            pane_relation(&above, viewer, min, max, false),
            pane_relation(&below, viewer, min, max, false),
        ];
        assert_eq!(first[0], second[1]);
        assert_eq!(first[1], second[0]);
    }
}

#[cfg(test)]
mod band_tests {
    /// The portal band sits at or below `WORLD_Z_DUMMY`.
    ///
    /// The full claim spans two crates. This crate pins
    /// `portal band <= WORLD_Z_DUMMY`, and `ambition_render` pins
    /// `WORLD_Z_DUMMY < actor`. Together: `portal band <= WORLD_Z_DUMMY < actor`,
    /// with no optional feature on either side.
    ///
    /// This guards against raising [`crate::PORTAL_WINDOW_Z`] above the actors.
    /// That inverts the bug: a near-side actor would vanish behind an aperture it
    /// stands in front of.
    #[test]
    fn the_portal_band_stays_at_or_below_the_shared_world_datum() {
        let datum = ambition_platformer2d_core::config::WORLD_Z_DUMMY;
        for (name, z) in [
            ("PORTAL_EXIT_COPY_Z", crate::PORTAL_EXIT_COPY_Z),
            ("PORTAL_WINDOW_Z", crate::PORTAL_WINDOW_Z),
            ("PORTAL_RIM_OVERLAY_Z", crate::PORTAL_RIM_OVERLAY_Z),
        ] {
            assert!(
                z <= datum,
                "{name} = {z} is above WORLD_Z_DUMMY ({datum}). If this moved to \
                 fix a far-side actor drawing over a portal window: that INVERTS \
                 the bug — a NEAR-side actor would vanish behind an aperture it \
                 stands in front of. The fix is a per-pane compositing relation \
                 (`crate::pane_relation`), not a global z, because one body is \
                 near one pane and far of another in the same frame."
            );
        }
    }

    /// The control: the band must also be ordered within itself.
    #[test]
    fn the_portal_band_is_ordered_within_itself() {
        assert!(crate::PORTAL_EXIT_COPY_Z < crate::PORTAL_WINDOW_Z);
        assert!(crate::PORTAL_WINDOW_Z < crate::PORTAL_RIM_OVERLAY_Z);
    }
}
