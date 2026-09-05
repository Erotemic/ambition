//! Where a drawable sits relative to ONE portal pane, for compositing.
//!
//! ⛔⛔ THE RENDERER CANNOT ANSWER THIS WITH A Z CONSTANT, AND TODAY IT TRIES.
//! [`crate::PORTAL_WINDOW_Z`] is `9.5`; a generic actor draws at
//! `WORLD_Z_DUMMY + 1.0 = 11.0` and the player at `WORLD_Z_PLAYER = 20.0`. So
//! EVERY actor wins the depth test against EVERY pane. The constant's own doc
//! states the intent — *"below actors so a near-side actor still occludes it"* —
//! and a single global ordering can only serve half of it: a FAR-side actor
//! standing behind the aperture is drawn over the captured image it should be
//! hidden by. Reported by Jon 2026-09-05 with a screenshot of a far-side
//! Perfect Cellular Automaton punching through a seamless window.
//!
//! ⭐ THE RELATION IS PER PANE, WHICH IS WHY IT IS NOT A Z. One body can be NEAR
//! one pane and FAR of another in the same frame, and a single entity z cannot
//! represent both. ⇒ This module answers the question for one (pane, drawable)
//! pair and holds no opinion about how the renderer then composites it: the
//! classification is the shared authority, and the drawing road is free to be a
//! clipped overlay, a stencil, or a dedicated pass.
//!
//! ⚠ NOTHING CONSUMES THIS FOR DRAWING YET. It is built first because the
//! diagnostics and the eventual compositor need the SAME answer, and a dump that
//! computed its own would be a second authority for the fact under repair.

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
    /// On the FAR side and overlapping: the pane's captured image should cover
    /// the overlapping pixels. ⛔ This is the case the current z policy gets
    /// wrong, every time.
    FarCovered,
    /// Mid-transit: the split here/through presentation owns this body and the
    /// compositor must not add a third copy of it.
    Transiting,
}

/// Classify `drawable` (world-space bounds, as DRAWN) against one `pane`.
///
/// ⚠ `transiting` WINS OVER GEOMETRY, deliberately. A body crossing the plane is
/// already drawn as clipped pieces; asking whether its bounds overlap the pane
/// would classify it a second time and invite a duplicate copy.
///
/// ⚠ A drawable exactly ON the plane counts as the VIEWER's side. The pane is a
/// hole in a surface, so a body resting against it from the room is the ordinary
/// near case; `front_distance` returning exactly zero for a far-side body is a
/// measure-zero coincidence, and biasing it toward "occludes" fails visible
/// rather than invisible.
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
    let pane_min = pane.pos - pane.half_extent;
    let pane_max = pane.pos + pane.half_extent;
    let overlaps = drawable_min.x <= pane_max.x
        && drawable_max.x >= pane_min.x
        && drawable_min.y <= pane_max.y
        && drawable_max.y >= pane_min.y;
    if !overlaps {
        return PaneRelation::Disjoint;
    }

    // ⭐ ONE READING OF "WHICH SIDE", the portal domain's own named verb. A
    // second `.dot(normal)` here would be a private copy of a rule the sim
    // already owns and would drift from it the day a moving host changes what
    // "in front" means.
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

/// Does the drawn ordering MATCH the relation, given the two actual z values?
///
/// ⭐ THE POINT IS TO MAKE THE BUG COUNTABLE. A diagnostic that printed the
/// relation without saying which ones the renderer then gets wrong would leave
/// the reader to re-derive the very comparison the finding is about.
///
/// ⛔⛔ IT TAKES THE Z VALUES RATHER THAN ASSUMING THEM, AND THE FIRST VERSION
/// DID NOT. It was a bare `match` returning `false` for `FarCovered` — a hardcoded
/// statement that actors always outrank panes, written beside a dump that
/// computed the same fact from `z > PORTAL_WINDOW_Z`. **Two readings of one
/// fact, and the constant one goes stale the moment somebody moves a z.** If
/// `PORTAL_WINDOW_Z` were raised above the actor band tomorrow, the hardcoded
/// version would still report every far-side body as a violation and every
/// near-side one as fine — with the truth exactly inverted, in the instrument
/// built to find that class of error.
///
/// ⇒ Now there is ONE reading: the caller supplies what was actually drawn and
/// what the pane actually is, and the answer follows. The dump no longer needs a
/// second comparison of its own.
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
/// A piece is a SUB-RECT of the original sprite, so the four sides of the
/// rectangle it must not exceed come from the quad's own extent. That is why
/// this road needs no clip planes: [`crate::PortalClipMaterial`] carries three
/// half-planes and an axis-aligned rectangle wants four, but a quad IS its own
/// four planes.
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
/// ⭐⭐ THIS IS WHAT MAKES THE DEFECT INEXPRESSIBLE RATHER THAN CAUGHT. Jon's
/// report is a far-side body drawn over the pane that should hide it, and the
/// two obvious repairs were both ruled out for good reasons: raising
/// [`crate::PORTAL_WINDOW_Z`] above the player only inverts the bug onto
/// near-side bodies, and moving an actor's single z cannot serve two panes that
/// disagree about it in the same frame. ⇒ The covered region is never handed to
/// the renderer AT ALL. There is no ordering to get wrong, because the pixels
/// that could be wrong are not in any piece — asserted by
/// `no_piece_ever_overlaps_the_cover` over a grid of offsets.
///
/// The decomposition is the standard one and its shape is load-bearing: a
/// full-width band below the cover, a full-width band above it, then the left
/// and right pieces of the middle band only. Cutting the bands full-width first
/// is what keeps the four disjoint; four half-open half-planes would overlap at
/// the corners and draw them twice, which for a translucent sprite is visible.
///
/// Total on no overlap (one whole piece) and on total cover (none), so the
/// caller needs no special case for either.
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
    // ⚠ A degenerate cover takes this road too — a zero-area aperture must hide
    // NOTHING, and treating it as covering everything would blank the actor.
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

#[cfg(test)]
mod remainder_tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec2, Vec2) {
        (Vec2::new(x0, y0), Vec2::new(x1, y1))
    }

    fn remainder(d: (Vec2, Vec2), c: (Vec2, Vec2)) -> UncoveredPieces {
        uncovered_remainder(d.0, d.1, c.0, c.1)
    }

    /// ⭐⭐ THE PROPERTY THE WHOLE REPAIR RESTS ON, over a grid rather than a
    /// hand-picked case: whatever the cover is, NO emitted piece overlaps it.
    /// A far-side body cannot draw inside the aperture because those pixels are
    /// never in a piece — there is no z, and no ordering, to get wrong.
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
        // ⚠ An anti-vacuity floor that clears the largest arrangement, not zero:
        // 17x17 offsets, and the interior ones emit all four pieces.
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

    /// Disjointness is not implied by the two properties above — a double-drawn
    /// corner tiles the right AREA only if you count it twice, and on a
    /// translucent sprite it shows as a bright square.
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

    /// ⚠ Both of the ways "nothing is hidden" arrives, because they take
    /// DIFFERENT roads through the function and a zero-area aperture reading as
    /// a total cover would blank the actor outright — the loudest possible
    /// version of the bug being repaired.
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

    /// ⭐⭐ THE THREE-HALF-PLANE BUDGET, PINNED. A piece is NOT drawn as a
    /// sub-rect quad: `clip_piece_transform` scales the quad to the WHOLE
    /// sprite and every cut is a half-plane in [`crate::PortalClipMaterial`],
    /// which carries exactly THREE. So a piece is affordable only if it differs
    /// from the drawable's own bounds on at most three edges — the sprite's
    /// outer edges come free with the quad, and only the edges the cover moved
    /// need a plane.
    ///
    /// Cutting the bands full-width FIRST is what buys that: the two bands move
    /// one edge each, and the two middle pieces move three. Four half-planes
    /// meeting at the corners would need four on some piece and would not fit.
    /// ⇒ The decomposition's shape is now load-bearing for a SECOND independent
    /// reason, and this test fails if anyone re-cuts it.
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
        // ⚠ Anti-vacuity: if nothing ever needed 3 the budget would be untested
        // and a 4-plane decomposition could slip in under a looser bound.
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

    /// ⛔⛔ THE REPORTED BUG, as a classification. A body BEHIND the pane whose
    /// sprite overlaps it must be covered by the captured image — and the
    /// current global-z policy draws it on top, every time.
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
        // ⭐ AND THE ANSWER FOLLOWS THE NUMBERS, not a hardcoded verdict: put the
        // same far-side body BELOW the pane and it is composited correctly. A
        // constant `false` here would have called the fixed world broken too.
        assert!(current_z_policy_is_correct_for(relation, 9.0, crate::PORTAL_WINDOW_Z));
    }

    /// ⚠ THE CONTROL. Without it, a classifier that answered `FarCovered` for
    /// everything would pass the test above and be strictly worse than the bug.
    #[test]
    fn a_near_side_body_overlapping_the_pane_may_occlude_it() {
        let p = pane(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let viewer = Vec2::new(100.0, 360.0);
        let (min, max) = body(Vec2::new(100.0, 308.0)); // viewer's side
        let relation = pane_relation(&p, viewer, min, max, false);
        assert_eq!(relation, PaneRelation::NearOccluder);
        assert!(current_z_policy_is_correct_for(relation, 11.0, crate::PORTAL_WINDOW_Z));
        // ⚠ And a near-side body drawn BELOW the pane is wrong for the opposite
        // reason -- it would be hidden by an aperture it is standing in front of.
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

    /// ⚠ Transit wins over geometry: the split presentation already draws this
    /// body, and a compositor that also classified it would add a third copy.
    #[test]
    fn a_transiting_body_belongs_to_the_split_presentation_whatever_its_bounds() {
        let p = pane(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let (min, max) = body(Vec2::new(100.0, 292.0)); // would be FarCovered
        assert_eq!(
            pane_relation(&p, Vec2::new(100.0, 360.0), min, max, true),
            PaneRelation::Transiting
        );
    }

    /// ⭐⭐ THE POISON JON NAMED: ONE BODY, TWO PANES, DIFFERENT ANSWERS.
    ///
    /// This is the case that forbids fixing the bug by mutating the actor's
    /// single z. The body sits between two apertures facing each other: it is on
    /// the viewer's side of one and behind the other, in the same frame. Any
    /// implementation that stores one ordering per actor MUST fail here.
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

    /// ⚠ Order independence: the relation is a function of geometry, so building
    /// the same scene the other way round must not change it.
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
    /// ⛔⛔ THE OTHER HALF OF "THE PORTAL BAND SITS BELOW THE ACTOR BAND", and the
    /// half THIS crate can see unconditionally.
    ///
    /// The claim spans two crates: the portal z constants live here, and the
    /// `+ 1.0` that puts an actor above `WORLD_Z_DUMMY` lives in
    /// `ambition_render`. Asserting all of it there needed the optional
    /// `portal_render` feature — which is `default = []`, so the guard ran only
    /// under the exhaustive lane and would not have stopped anyone.
    ///
    /// ⇒ Split on the shared term. `WORLD_Z_DUMMY` is the datum both crates
    /// already depend on, so this pins `portal band <= datum` and
    /// `ambition_render` pins `datum < actor`; together
    /// `portal band <= WORLD_Z_DUMMY < actor`, with **no optional feature on
    /// either side**.
    ///
    /// ⭐⭐ THIS IS THE GUARD AGAINST THE RULED-OUT CHEAP FIX. Raising
    /// [`crate::PORTAL_WINDOW_Z`] above the cast is the two-line change that
    /// makes a reported screenshot look right and INVERTS the bug — a near-side
    /// actor would vanish behind an aperture it stands in front of.
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

    /// ⚠ The control: the band must be ORDERED within itself, or "below the
    /// datum" is satisfied by three constants that say nothing about each other.
    #[test]
    fn the_portal_band_is_ordered_within_itself() {
        assert!(crate::PORTAL_EXIT_COPY_Z < crate::PORTAL_WINDOW_Z);
        assert!(crate::PORTAL_WINDOW_Z < crate::PORTAL_RIM_OVERLAY_Z);
    }
}
