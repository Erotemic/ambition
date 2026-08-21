//! **Four pointers, and every device that drives one.**
//!
//! Jon, 2026-08-05: *"the arrows or game stick or mouse should move a cursor
//! that can click on elements."* — and 2026-08-20, one per seat, because that
//! is what Smash does and one shared pointer made four people take turns.
//!
//! A cursor that three input sources drive is a seam, not a widget detail, and
//! it is written here as one. Everything in this module is a plain value over
//! rectangles: no Bevy queries, no entities, no assumption about what the
//! rectangles ARE. [`super`] measures the screen into [`HitRect`]s once per
//! frame and asks these functions where the cursor should go.
//!
//! ## Why a TAP snaps and a HELD stick does not
//!
//! A mouse or a finger reports a position, so the cursor takes it. A d-pad
//! reports a DIRECTION as a just-pressed edge, and integrating a velocity from
//! an edge produces a cursor that jumps one step per tap and cannot be steered
//! — so a direction moves the cursor to the nearest thing IN that direction
//! ([`snap`]). Every stop is on something clickable, a token in hand lands on a
//! portrait rather than between two, and the whole screen is reachable in a
//! bounded number of presses.
//!
//! ⭐ **and since 2026-08-21 a HELD stick roams instead**, through
//! `MenuControlFrame::nav`, which is the one non-edge direction on that frame
//! and exists for exactly this. The snap did not go away: it is what a d-pad
//! and a keyboard still get, and what a stick falls back to when it is flicked
//! rather than held.
//!
//! ⚠ **each cursor has ONE position and no separate "focused element".** What it
//! is over is re-derived from the rectangles every frame. Two representations of
//! "where the cursor is" is how a highlight and a click end up disagreeing about
//! which portrait was chosen.
//!
//! [`MenuControlFrame`]: ambition_platformer2d::input::MenuControlFrame

use bevy::prelude::{Entity, Vec2};

/// A screen rectangle, in LOGICAL window pixels with a top-left origin — the
/// same space `Window::cursor_position` reports and the same one
/// `Node { position_type: Absolute, left, top }` writes back into.
///
/// ⚠ these come from [`super::layout`], NOT from reading nodes back out. A
/// `bevy_ui` node's screen rect is `ComputedNode` (PHYSICAL px) +
/// `UiGlobalTransform`, its plain `GlobalTransform` is identity for UI nodes,
/// and none of it is measured until `PostUpdate` — three separate ways for a
/// hit test to read a plausible zero. Deriving the rectangle and POSITIONING the
/// node from it has none of those failure modes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl HitRect {
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn size(self) -> Vec2 {
        self.max - self.min
    }

    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// A rect a layout pass has not measured yet.
    ///
    /// ⚠ a freshly spawned node reads ZERO until layout runs in `PostUpdate`,
    /// so every consumer here has to be able to say "not yet" rather than
    /// treat the origin as a real position.
    pub fn is_unmeasured(self) -> bool {
        self.size().x <= 0.0 || self.size().y <= 0.0
    }
}

/// One thing the cursor can be over.
#[derive(Clone, Copy, Debug)]
pub struct CursorTarget {
    pub entity: Entity,
    pub rect: HitRect,
}

/// **What the cursor is over**, chosen by containment and then by distance.
///
/// ⚠ `filter().min_by()` rather than `find()`: the cards overlap the pool row
/// by a few pixels at some window sizes, and a `find` over an ambiguous
/// predicate answers confidently and wrongly. Nearest-centre is a rule that
/// stays right when two rects genuinely overlap.
pub fn hovered(point: Vec2, targets: &[CursorTarget]) -> Option<Entity> {
    targets
        .iter()
        .filter(|target| !target.rect.is_unmeasured() && target.rect.contains(point))
        .min_by(|a, b| {
            let da = a.rect.center().distance_squared(point);
            let db = b.rect.center().distance_squared(point);
            da.total_cmp(&db)
        })
        .map(|target| target.entity)
}

/// **The nearest target in a direction**, for arrows, d-pads and sticks.
///
/// `direction` is any non-zero vector in screen space (y grows DOWNWARD, like
/// the rest of this module). The cost is `along + 2 * across`: distance in the
/// pressed direction, plus twice the sideways error, which is the rule that
/// makes "right" prefer the next portrait in the row over the one diagonally
/// below it while still crossing to the next row when the row runs out.
///
/// Returns `None` when nothing lies that way — a cursor at the right edge
/// pressing right STAYS, which is the correct answer and not a failure.
pub fn snap(from: Vec2, direction: Vec2, targets: &[CursorTarget]) -> Option<Entity> {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO {
        return None;
    }
    let sideways = Vec2::new(-direction.y, direction.x);
    targets
        .iter()
        .filter(|target| !target.rect.is_unmeasured())
        .filter_map(|target| {
            let offset = target.rect.center() - from;
            let along = offset.dot(direction);
            // A target the cursor is already on has `along <= 0` and drops out,
            // which is what stops a press from selecting the thing under it.
            if along <= MIN_SNAP_TRAVEL_PX {
                return None;
            }
            let across = offset.dot(sideways).abs();
            Some((target.entity, along + across * SIDEWAYS_PENALTY))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity)
}

/// How far a target's centre must be in the pressed direction to count as
/// "that way". Without a floor, two rects whose centres share a row make each
/// other's `along` a rounding error and the cursor oscillates.
const MIN_SNAP_TRAVEL_PX: f32 = 1.0;

/// Sideways error costs double. See [`snap`].
const SIDEWAYS_PENALTY: f32 = 2.0;

/// **Where one seat's pointer is and what is in its hand.**
///
/// ⚠ **a VALUE since 2026-08-21, not a resource.** It was one shared cursor —
/// *"Jon asked for a cursor, not for four of them"* — and four people at four
/// pads took turns with it like a mouse. Smash gives every player their own
/// hand and they all move at once; Jon's call, 2026-08-20, was to go there. See
/// [`SelectCursors`], which owns one of these per seat.
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectCursor {
    /// Logical window pixels, top-left origin.
    pub position: Vec2,
    /// The slot whose token is in hand, if any.
    pub carrying: Option<usize>,
    /// Where the carried token was picked up, so a mouse RELEASE can tell a
    /// drag from a click. See [`Self::release_should_drop`].
    pub grabbed_at: Vec2,
    /// Whether anything has ever positioned this cursor. A cursor parked at
    /// the origin looks identical to one nobody has moved, and the difference
    /// decides whether the first arrow press snaps from the corner or from the
    /// middle of the screen.
    pub placed: bool,
}

impl SelectCursor {
    pub fn move_to(&mut self, position: Vec2) {
        self.position = position;
        self.placed = true;
    }

    pub fn grab(&mut self, slot: usize) {
        self.carrying = Some(slot);
        self.grabbed_at = self.position;
    }

    pub fn drop_it(&mut self) -> Option<usize> {
        self.carrying.take()
    }

    /// **Does letting go of the mouse button mean "put it down"?**
    ///
    /// Only if the pointer actually travelled. Both idioms have to work on the
    /// same screen: a mouse user DRAGS (press, move, release), and a pad user
    /// CLICKS TWICE (there is no release edge in the menu frame at all). A
    /// release that has not moved is therefore the first half of a two-click
    /// place, and dropping the token there would make every pad pick-up
    /// immediately put the token back.
    pub fn release_should_drop(&self) -> bool {
        self.carrying.is_some() && self.position.distance(self.grabbed_at) > DRAG_SLOP_PX
    }
}

/// **FOUR CURSORS, ONE PER SEAT** — the model every Smash has.
///
/// ⭐ **indexed by SEAT, which is also the slot and also the device index.**
/// One numbering, because two that have to agree eventually disagree; the same
/// rule `SeatMenuFrames` follows, and the reason a seat's pad, a seat's token
/// and a seat's cursor never need a translation table between them.
///
/// ⚠ **every seat has a cursor whether or not anybody is in it.** An absent
/// seat's cursor costs two floats and is simply not drawn — the alternative is
/// an `Option` every reader unwraps, and a seat that joins mid-lobby would then
/// have to invent a position from somewhere.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct SelectCursors {
    seats: [SelectCursor; crate::select::MAX_SMASH_SEATS],
}

impl SelectCursors {
    pub fn seat(&self, seat: usize) -> &SelectCursor {
        &self.seats[seat.min(crate::select::MAX_SMASH_SEATS - 1)]
    }

    pub fn seat_mut(&mut self, seat: usize) -> &mut SelectCursor {
        &mut self.seats[seat.min(crate::select::MAX_SMASH_SEATS - 1)]
    }

    /// Every seat's cursor, in seat order — the order a reader must draw and
    /// arbitrate in if two of them are ever to be compared.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &SelectCursor)> {
        self.seats.iter().enumerate()
    }

    /// **Which seat is carrying `slot`'s token, if any.**
    ///
    /// ⚠ a seat carries its OWN token and nobody else's, so this is a lookup
    /// rather than a search — but it is written as one so that the day a seat
    /// may hand a token over, the callers do not have to change.
    pub fn carrier_of(&self, slot: usize) -> Option<usize> {
        self.iter()
            .find(|(_, cursor)| cursor.carrying == Some(slot))
            .map(|(seat, _)| seat)
    }
}

/// How far the pointer must travel between press and release for it to be a
/// drag rather than a click. Mirrors `ROW_TAP_SLOP_PX` in `ambition_ui_nav`,
/// which solved the same ambiguity for list rows.
pub const DRAG_SLOP_PX: f32 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: u32, x: f32, y: f32) -> CursorTarget {
        CursorTarget {
            entity: Entity::from_raw_u32(id).expect("a test entity id"),
            rect: HitRect::from_center_size(Vec2::new(x, y), Vec2::splat(40.0)),
        }
    }

    /// A 3-wide row: pressing right takes the next one along, not the far one.
    #[test]
    fn right_takes_the_next_target_along_the_row() {
        let row = [
            target(1, 100.0, 100.0),
            target(2, 200.0, 100.0),
            target(3, 300.0, 100.0),
        ];
        assert_eq!(
            snap(Vec2::new(100.0, 100.0), Vec2::X, &row),
            Some(row[1].entity)
        );
    }

    /// **Sideways error costs double**, so a target directly right beats one
    /// that is nearer in a straight line but a row down.
    #[test]
    fn a_target_dead_ahead_beats_a_nearer_diagonal_one() {
        let targets = [
            target(1, 260.0, 100.0), // 160 ahead, dead on
            target(2, 190.0, 210.0), // 90 ahead but 110 across
        ];
        assert_eq!(
            snap(Vec2::new(100.0, 100.0), Vec2::X, &targets),
            Some(targets[0].entity),
            "the cursor left the row it was travelling along"
        );
    }

    /// Nothing that way is a cursor that STAYS, not an error and not a wrap.
    #[test]
    fn the_edge_of_the_screen_snaps_nowhere() {
        let row = [target(1, 100.0, 100.0)];
        assert_eq!(snap(Vec2::new(100.0, 100.0), Vec2::X, &row), None);
        assert_eq!(snap(Vec2::new(400.0, 100.0), Vec2::X, &row), None);
    }

    /// ⚠ y grows DOWNWARD. "Down" must reach the slot cards, which are BELOW
    /// the grid — an inverted axis here would make the whole bottom third
    /// unreachable from a d-pad and nothing else would notice.
    #[test]
    fn down_moves_toward_the_bottom_of_the_screen() {
        let targets = [target(1, 100.0, 40.0), target(2, 100.0, 400.0)];
        assert_eq!(
            snap(Vec2::new(100.0, 100.0), Vec2::Y, &targets),
            Some(targets[1].entity),
            "down snapped upward, so the slot cards are unreachable without a mouse"
        );
    }

    /// An unmeasured rect is not a target. A freshly spawned node reads zero
    /// until layout runs, and a zero rect at the origin is the nearest thing to
    /// everything.
    #[test]
    fn an_unmeasured_node_is_never_snapped_to() {
        let unmeasured = CursorTarget {
            entity: Entity::from_raw_u32(9).expect("a test entity id"),
            rect: HitRect::from_center_size(Vec2::ZERO, Vec2::ZERO),
        };
        let targets = [unmeasured, target(1, 300.0, 100.0)];
        assert_eq!(
            snap(Vec2::new(100.0, 100.0), Vec2::X, &targets),
            Some(targets[1].entity)
        );
        assert_eq!(hovered(Vec2::ZERO, &targets), None);
    }

    /// Overlapping rects resolve by nearest centre rather than by whichever the
    /// query happened to yield first.
    #[test]
    fn overlapping_targets_resolve_to_the_nearest_centre() {
        let targets = [target(1, 100.0, 100.0), target(2, 120.0, 100.0)];
        assert_eq!(
            hovered(Vec2::new(118.0, 100.0), &targets),
            Some(targets[1].entity)
        );
        assert_eq!(
            hovered(Vec2::new(96.0, 100.0), &targets),
            Some(targets[0].entity)
        );
    }

    /// **A pad's pick-up must not put the token straight back down.** A release
    /// that has not travelled is the first half of a two-click place.
    #[test]
    fn a_release_that_did_not_travel_keeps_the_token_in_hand() {
        let mut cursor = SelectCursor::default();
        cursor.move_to(Vec2::new(50.0, 50.0));
        cursor.grab(2);
        assert!(!cursor.release_should_drop());

        cursor.move_to(Vec2::new(50.0 + DRAG_SLOP_PX + 1.0, 50.0));
        assert!(cursor.release_should_drop());
        assert_eq!(cursor.drop_it(), Some(2));
        assert_eq!(cursor.carrying, None);
    }
}
