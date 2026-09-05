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

/// Does the CURRENT global-z policy already draw this relation correctly?
///
/// ⭐ THE POINT OF THIS FUNCTION IS TO MAKE THE BUG COUNTABLE. Every actor z is
/// above every pane z, so today's renderer composites `NearOccluder` correctly
/// and `FarCovered` wrongly, always. A diagnostic that printed the relation
/// without saying which ones the renderer then gets wrong would leave the reader
/// to re-derive the very comparison the finding is about.
pub fn current_z_policy_is_correct_for(relation: PaneRelation) -> bool {
    match relation {
        // Drawn above the pane, and should be.
        PaneRelation::NearOccluder => true,
        // Not overlapping, so ordering cannot be observed.
        PaneRelation::Disjoint => true,
        // The split presentation owns it on its own layers.
        PaneRelation::Transiting => true,
        // ⛔ Drawn above the pane and should be covered BY it.
        PaneRelation::FarCovered => false,
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
            !current_z_policy_is_correct_for(relation),
            "the whole finding is that today's z draws this one on top of the pane"
        );
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
        assert!(current_z_policy_is_correct_for(relation));
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
