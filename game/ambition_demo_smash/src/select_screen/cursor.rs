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

    /// ⛔ **PRIVATE, and that is the invariant.** One token has at most one
    /// carrier, and a rule enforced by every caller remembering it is a rule
    /// with no owner — the screen let any human grab any CPU token, and two
    /// cursors carrying the same one differ from one cursor only in which of
    /// them the renderer happened to draw. Go through
    /// [`SelectCursors::try_grab`], which is where the question is answered.
    fn grab(&mut self, slot: usize) {
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
/// ⭐ **indexed by INPUT SEAT** — the same key `SeatMenuFrames` uses, because a
/// cursor is a HAND and a hand belongs to a person.
///
/// ⚠ **the current seating policy maps that key onto a local-source index, and
/// that is a policy rather than an identity.** The ordinal counts the sources
/// taken up on this machine, so a lone player on pad three is seat zero; this
/// table does not claim an input seat, a physical device and a match slot are
/// the same thing.
///
/// ⛔⛔ **and a MATCH SLOT is a third numbering, which this is not.** A seat's
/// cursor, a seat's pad and a seat's menu frame agree; the CARD that seat
/// drives is whichever one names its source, and
/// [`SmashSelect::slot_driven_by`](crate::select::SmashSelect::slot_driven_by)
/// is the only honest way to ask. This doc used to say all three were one
/// numbering, and the select screen believed it: a roster with a CPU between
/// two people routed the second person onto the CPU's card.
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
    /// ⛔⛔ **an out-of-range seat resolves to NO seat, never to a neighbour.**
    ///
    /// This used to clamp with `seat.min(MAX - 1)` under a doc that said,
    /// correctly, that *"clamping hands one person another's cursor"* — loud in
    /// debug and silently wrong in release, which is the build where a stranger
    /// moving your cursor is not survivable. An invalid identity is not a
    /// nearby identity, and a menu that answers for the wrong person is worse
    /// than one that answers for nobody (GPT review, 2026-08-22).
    ///
    /// ⚠ **no caller pays for this.** Every production reader runs inside
    /// `for seat in 0..MAX_SMASH_SEATS`, so the `Option` is always `Some` and
    /// the change is free where it matters; what it removes is the ability for
    /// a future caller to pass a seat it got from data and be handed somebody.
    pub fn seat(&self, seat: usize) -> Option<&SelectCursor> {
        self.seats.get(seat)
    }

    pub fn seat_mut(&mut self, seat: usize) -> Option<&mut SelectCursor> {
        self.seats.get_mut(seat)
    }

    /// Every seat's cursor, in seat order — the order a reader must draw and
    /// arbitrate in if two of them are ever to be compared.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &SelectCursor)> {
        self.seats.iter().enumerate()
    }

    /// **Which seat is carrying `slot`'s token, if any.**
    ///
    /// ⚠ **a search, not a lookup** — a seat may carry a CPU's token as well as
    /// its own, so the carrier of slot 2 is not seat 2. [`Self::try_grab`] is
    /// what keeps the answer singular; without it this returned the first of
    /// however many cursors held the same token, and the renderer drew one of
    /// them.
    pub fn carrier_of(&self, slot: usize) -> Option<usize> {
        self.iter()
            .find(|(_, cursor)| cursor.carrying == Some(slot))
            .map(|(seat, _)| seat)
    }

    /// **Take one token if both sides are free.**
    ///
    /// This owns the two mechanical invariants of carrying: a token has at most
    /// one carrier, and a cursor has at most one token. Which token a particular
    /// player is ALLOWED to grab is character-select policy and stays in the
    /// screen state machine rather than leaking into this geometry helper.
    ///
    /// Re-grabbing the token already in this hand succeeds and re-arms
    /// `grabbed_at`, which is what distinguishes a drag from a click.
    pub fn try_grab(&mut self, seat: usize, slot: usize) -> bool {
        // ⛔ a seat that does not exist grabs NOTHING — it does not grab on the
        // last seat's behalf, which is what the clamped accessor used to do.
        let Some(cursor) = self.seat(seat) else {
            return false;
        };
        if cursor.carrying.is_some_and(|held| held != slot) {
            return false;
        }
        match self.carrier_of(slot) {
            Some(holder) if holder != seat => false,
            _ => {
                // `seat` indexed successfully above.
                if let Some(cursor) = self.seat_mut(seat) {
                    cursor.grab(slot);
                }
                true
            }
        }
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

    #[test]
    fn a_cursor_cannot_replace_the_token_it_is_already_carrying() {
        let mut cursors = SelectCursors::default();
        assert!(cursors.try_grab(0, 1));
        assert!(!cursors.try_grab(0, 2));
        assert_eq!(cursors.seat(0).expect("seat 0").carrying, Some(1));
        assert_eq!(cursors.carrier_of(1), Some(0));
        assert_eq!(cursors.carrier_of(2), None);
    }

    /// ⛔⛔ **an out-of-range seat is NOBODY, not the nearest somebody.**
    ///
    /// This table clamped with `seat.min(MAX - 1)`, so seat 7 of 4 moved player
    /// 4's cursor and grabbed with player 4's hand — an identity bug converted
    /// into wrong input, in release builds only. The falsifier is that the last
    /// seat must be UNTOUCHED by a request that named a seat past the end.
    #[test]
    fn a_seat_past_the_end_resolves_to_nobody_rather_than_the_last_seat() {
        let mut cursors = SelectCursors::default();
        let past_the_end = crate::select::MAX_SMASH_SEATS + 3;

        assert!(cursors.seat(past_the_end).is_none());
        assert!(cursors.seat_mut(past_the_end).is_none());

        // ⭐ the part that actually bit: a WRITE through the clamp landed on a
        // real person. `try_grab` must refuse, and the last seat must still be
        // empty afterwards.
        assert!(
            !cursors.try_grab(past_the_end, 1),
            "a seat that does not exist grabbed a token"
        );
        assert_eq!(
            cursors
                .seat(crate::select::MAX_SMASH_SEATS - 1)
                .expect("the last seat exists")
                .carrying,
            None,
            "the out-of-range grab landed on the last seat's cursor"
        );
        assert_eq!(cursors.carrier_of(1), None);
    }

    #[test]
    fn one_token_cannot_have_two_carriers() {
        let mut cursors = SelectCursors::default();
        assert!(cursors.try_grab(0, 2));
        assert!(!cursors.try_grab(1, 2));
        assert_eq!(cursors.carrier_of(2), Some(0));
        assert_eq!(cursors.seat(1).expect("seat 1").carrying, None);
    }
}
