//! Four pointers, and every device that drives one.
//!
//! Everything here is a plain value over rectangles: no Bevy queries, no
//! entities, no assumption about what the rectangles are. [`super`] measures
//! the screen into [`HitRect`]s once per frame and asks these functions where
//! the cursor should go.
//!
//! ## Why a tap snaps and a held stick does not
//!
//! A mouse or finger reports a position, so the cursor takes it. A d-pad
//! reports a direction as a just-pressed edge, so the cursor moves to the
//! nearest target in that direction ([`snap`]). Every stop is on something
//! clickable, and the whole screen is reachable in a bounded number of presses.
//!
//! Each cursor has one position and no separate "focused element". What it is
//! over is re-derived from the rectangles every frame, so a highlight and a
//! click cannot disagree.
//!
//! [`MenuControlFrame`]: ambition_platformer2d::input::MenuControlFrame

use bevy::prelude::{Entity, Vec2};

/// A screen rectangle, in logical window pixels with a top-left origin: the
/// space `Window::cursor_position` reports and
/// `Node { position_type: Absolute, left, top }` writes.
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

    /// A newly spawned node reads zero until layout runs in `PostUpdate`, so
    /// consumers must treat that as "not yet", not as the origin.
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

/// What the cursor is over, chosen by containment and then by distance.
///
/// `filter().min_by()`, not `find()`: cards overlap the pool row by a few
/// pixels at some window sizes, and nearest-centre stays correct when rects
/// overlap.
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

/// The nearest target in a direction, for arrows, d-pads and sticks.
///
/// `direction` is any non-zero vector in screen space (y grows downward). The
/// cost is `along + 2 * across`, so "right" prefers the next portrait in the
/// row over the one diagonally below, and still crosses rows at a row's end.
///
/// Returns `None` when nothing lies that way: a cursor at the right edge
/// pressing right stays.
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
            // A target the cursor is already on has `along <= 0` and drops
            // out, so a press does not select the thing under it.
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
/// "that way". Without a floor, two targets in one row can make the cursor
/// oscillate.
const MIN_SNAP_TRAVEL_PX: f32 = 1.0;

/// Sideways error costs double. See [`snap`].
const SIDEWAYS_PENALTY: f32 = 2.0;

/// Where one seat's pointer is and what is in its hand.
///
/// See [`SelectCursors`], which owns one of these per seat.
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectCursor {
    /// Logical window pixels, top-left origin.
    pub position: Vec2,
    /// The slot whose token is in hand, if any.
    pub carrying: Option<usize>,
    /// Where the carried token was picked up, so a mouse release can tell a
    /// drag from a click. See [`Self::release_should_drop`].
    pub grabbed_at: Vec2,
    /// Whether anything has positioned this cursor. It decides whether the
    /// first arrow press snaps from the corner or from the screen's middle.
    pub placed: bool,
    /// Speed built up by holding a direction. See [`CursorRamp`]. Per seat,
    /// because it belongs to one person's hand.
    pub ramp: CursorRamp,
}

impl SelectCursor {
    pub fn move_to(&mut self, position: Vec2) {
        self.position = position;
        self.placed = true;
    }

    /// Private: one token has at most one carrier. Go through
    /// [`SelectCursors::try_grab`], which enforces it.
    fn grab(&mut self, slot: usize) {
        self.carrying = Some(slot);
        self.grabbed_at = self.position;
    }

    pub fn drop_it(&mut self) -> Option<usize> {
        self.carrying.take()
    }

    /// Does letting go of the mouse button mean "put it down"?
    ///
    /// Only if the pointer travelled. A mouse user drags (press, move,
    /// release); a pad user clicks twice (the menu frame has no release edge).
    /// A release without travel is the first half of a two-click place.
    pub fn release_should_drop(&self) -> bool {
        self.carrying.is_some() && self.position.distance(self.grabbed_at) > DRAG_SLOP_PX
    }
}

/// Four cursors, one per seat.
///
/// Indexed by input seat, the same key `SeatMenuFrames` uses: a cursor is a
/// hand, and a hand belongs to a person. The seating policy maps that key to a
/// local-source index, so a lone player on pad three is seat zero. A match
/// slot is a third numbering; use
/// [`SmashSelect::slot_driven_by`](crate::select::SmashSelect::slot_driven_by)
/// to find the card a seat drives.
///
/// Every seat has a cursor whether or not anyone is in it; an absent seat's
/// cursor is not drawn. A seat that joins mid-lobby then already has one.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct SelectCursors {
    seats: [SelectCursor; crate::select::MAX_SMASH_SEATS],
}

impl SelectCursors {
    /// An out-of-range seat resolves to `None`, not to another player's
    /// cursor. Production readers stay inside `0..MAX_SMASH_SEATS`.
    pub fn seat(&self, seat: usize) -> Option<&SelectCursor> {
        self.seats.get(seat)
    }

    pub fn seat_mut(&mut self, seat: usize) -> Option<&mut SelectCursor> {
        self.seats.get_mut(seat)
    }

    /// Every seat's cursor, in seat order: the order to draw and arbitrate in.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &SelectCursor)> {
        self.seats.iter().enumerate()
    }

    /// Which seat is carrying `slot`'s token, if any.
    ///
    /// A search, not a lookup: a seat may carry a CPU's token, so the carrier
    /// of slot 2 is not seat 2. [`Self::try_grab`] keeps the answer singular.
    pub fn carrier_of(&self, slot: usize) -> Option<usize> {
        self.iter()
            .find(|(_, cursor)| cursor.carrying == Some(slot))
            .map(|(seat, _)| seat)
    }

    /// Take one token if both sides are free.
    ///
    /// This owns the two mechanical invariants: a token has at most one
    /// carrier, and a cursor has at most one token. Which token a player may
    /// grab is select-screen policy.
    ///
    /// Re-grabbing the token already in this hand succeeds and re-arms
    /// `grabbed_at`, which is what distinguishes a drag from a click.
    pub fn try_grab(&mut self, seat: usize, slot: usize) -> bool {
        // A seat that does not exist grabs nothing.
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

    /// Sideways error costs double, so a target directly right beats one
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

    /// Nothing that way means the cursor stays: not an error and not a wrap.
    #[test]
    fn the_edge_of_the_screen_snaps_nowhere() {
        let row = [target(1, 100.0, 100.0)];
        assert_eq!(snap(Vec2::new(100.0, 100.0), Vec2::X, &row), None);
        assert_eq!(snap(Vec2::new(400.0, 100.0), Vec2::X, &row), None);
    }

    /// y grows downward. "Down" must reach the slot cards below the grid.
    #[test]
    fn down_moves_toward_the_bottom_of_the_screen() {
        let targets = [target(1, 100.0, 40.0), target(2, 100.0, 400.0)];
        assert_eq!(
            snap(Vec2::new(100.0, 100.0), Vec2::Y, &targets),
            Some(targets[1].entity),
            "down snapped upward, so the slot cards are unreachable without a mouse"
        );
    }

    /// An unmeasured rect is not a target: a zero rect at the origin would be
    /// nearest to everything.
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

    /// Overlapping rects resolve by nearest centre, not by query order.
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

    /// A pad's pick-up must not put the token straight back down. A release
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

    /// An out-of-range seat is nobody, not the nearest somebody. A request
    /// naming a seat past the end must leave the last seat untouched.
    #[test]
    fn a_seat_past_the_end_resolves_to_nobody_rather_than_the_last_seat() {
        let mut cursors = SelectCursors::default();
        let past_the_end = crate::select::MAX_SMASH_SEATS + 3;

        assert!(cursors.seat(past_the_end).is_none());
        assert!(cursors.seat_mut(past_the_end).is_none());

        // A write must not land on a real person: `try_grab` refuses, and the
        // last seat stays empty.
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

/// How many portrait cells a fully deflected stick crosses per second.
///
/// A speed in cells, not a fraction of the viewport: it does not depend on
/// aspect ratio (one scalar for both axes), scales with the grid, and is easy
/// to reason about (4.5 portraits a second at full deflection).
///
/// This is the starting speed. A committed push builds up to
/// [`RAMP_MAX_MULTIPLIER`] times this; see [`CursorRamp`]. Short movements, most
/// of them, get this base.
///
/// Cells are 0.86 as wide as they are tall ([`PORTRAIT_ASPECT`]), so crossing a
/// cell vertically takes about 16% longer than horizontally.
pub const CURSOR_CELLS_PER_SECOND: f32 = 4.5;

/// The longest frame the cursor will integrate over.
///
/// A render hitch must not move the hand: without this, one 100ms stall moves
/// 0.4 of a cell in one unseen frame. The cursor is a pointer, not a physics
/// body.
pub const MAX_CURSOR_DT: f32 = 1.0 / 30.0;

/// How far the stick must be pushed before the cursor builds any speed at all.
///
/// A gentle push never accelerates, however long it is held: it means "place
/// the hand". Only a committed push builds speed.
pub const RAMP_ARM_DEFLECTION: f32 = 0.6;

/// How long a committed push is held at base speed before it starts building.
///
/// So a correction never ramps: nudging one portrait over takes a fraction of
/// this.
pub const RAMP_ARM_SECONDS: f32 = 0.18;

/// How long the build takes, once it starts.
pub const RAMP_TO_FULL_SECONDS: f32 = 0.6;

/// Top speed, as a multiple of [`CURSOR_CELLS_PER_SECOND`].
///
/// Reached only after about 0.78s of continuous committed push, longer than
/// most grid crossings; the cursor usually runs on the middle of the curve.
pub const RAMP_MAX_MULTIPLIER: f32 = 2.2;

/// Speed built up by holding a direction, so the cursor is unhurried over short
/// distances and quick over long ones.
///
/// One speed cannot both cross an eighteen-portrait grid quickly and stop
/// precisely on a portrait, so speed builds while a direction is held, just
/// enough that the player does not notice.
///
/// Smoothstep: zero slope at both ends, so there is no visible gear change.
/// A linear ramp has corners that feel like a lurch.
///
/// It resets on a reversal, so the return after an overshoot starts at base
/// speed and the player does not oscillate. A right-angle turn is not a
/// reversal (the test is `dot < 0`), so turning a corner keeps the speed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CursorRamp {
    /// Seconds of continuous committed push in the current direction.
    held_seconds: f32,
    /// The unit direction the build belongs to, so a reversal can be seen.
    direction: Vec2,
}

impl CursorRamp {
    /// Fold this frame in, and report the multiplier to apply to base speed.
    pub fn advance(&mut self, analog: Vec2, dt: f32) -> f32 {
        let magnitude = analog.length();
        if magnitude < RAMP_ARM_DEFLECTION {
            *self = Self::default();
            return 1.0;
        }
        let heading = analog / magnitude;
        if self.direction != Vec2::ZERO && self.direction.dot(heading) < 0.0 {
            *self = Self::default();
            // The reversal frame starts the new gesture, building from zero.
        }
        self.direction = heading;
        self.held_seconds += dt.clamp(0.0, MAX_CURSOR_DT);
        self.multiplier()
    }

    /// The current multiplier, without advancing.
    pub fn multiplier(&self) -> f32 {
        let building = (self.held_seconds - RAMP_ARM_SECONDS).max(0.0);
        let t = (building / RAMP_TO_FULL_SECONDS).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        1.0 + eased * (RAMP_MAX_MULTIPLIER - 1.0)
    }

    /// Whether any speed has been built. Diagnostics and tests, not gameplay.
    pub fn is_building(&self) -> bool {
        self.held_seconds > RAMP_ARM_SECONDS
    }
}

/// How far the cursor travels this frame for one stick deflection.
///
/// Speed is proportional to magnitude squared, so a half deflection moves at a
/// quarter speed: small pushes place the hand, full pushes cross the grid.
///
/// The curve bends speed, not direction: `direction` keeps the stick's angle.
/// There is no sticky-target magnetism; it makes the cursor fight the player.
///
/// Two multipliers compose: the curve reads how hard the stick is pushed, and
/// `ramp` reads how long.
pub fn cursor_travel(analog: Vec2, cell: Vec2, dt: f32, ramp: f32) -> Vec2 {
    let magnitude = analog.length().min(1.0);
    if magnitude <= 0.0 {
        return Vec2::ZERO;
    }
    let direction = analog / analog.length();
    let speed = magnitude * magnitude * CURSOR_CELLS_PER_SECOND * cell.x * ramp;
    direction * speed * dt.clamp(0.0, MAX_CURSOR_DT)
}

/// The cursor's velocity model, asked about the properties it was reshaped for.
#[cfg(test)]
mod cursor_travel_tests {
    use super::*;
    use crate::select_screen::layout::SelectLayout;

    /// These arms test the base model (deflection, units, aspect, dt clamp),
    /// so they use no built-up speed. The ramp has its own module.
    const NO_RAMP: f32 = 1.0;

    /// A 16:9 monitor and a 4:3 one, same roster.
    fn wide() -> SelectLayout {
        SelectLayout::new(Vec2::new(1920.0, 1080.0), 8)
    }
    fn tall() -> SelectLayout {
        SelectLayout::new(Vec2::new(1024.0, 768.0), 8)
    }

    /// Both axes move at the same rate: one scalar from the cell, not
    /// `viewport.x` for both.
    #[test]
    fn a_full_deflection_travels_the_same_distance_up_as_it_does_across() {
        let cell = wide().cell();
        let across = cursor_travel(Vec2::X, cell, 1.0 / 60.0, NO_RAMP).length();
        let down = cursor_travel(Vec2::Y, cell, 1.0 / 60.0, NO_RAMP).length();
        assert!(
            (across - down).abs() < 0.001,
            "a vertical push travelled {down:.2}px and a horizontal one {across:.2}px"
        );
    }

    /// The speed follows the portraits, not the window: crossing a cell takes
    /// the same time on any screen shape.
    #[test]
    fn crossing_one_portrait_takes_the_same_time_on_any_screen_shape() {
        let seconds_per_cell = |layout: SelectLayout| {
            let cell = layout.cell();
            let per_frame = cursor_travel(Vec2::X, cell, 1.0 / 60.0, NO_RAMP).length();
            cell.x / (per_frame * 60.0)
        };
        let a = seconds_per_cell(wide());
        let b = seconds_per_cell(tall());
        assert!(
            (a - b).abs() < 0.001,
            "one portrait took {a:.3}s to cross on a 16:9 screen and {b:.3}s on a 4:3 one"
        );
        assert!(
            (a - 1.0 / CURSOR_CELLS_PER_SECOND).abs() < 0.001,
            "a full deflection crossed a cell in {a:.3}s, not the \
             1/{CURSOR_CELLS_PER_SECOND} the constant promises"
        );
    }

    /// A half deflection is much slower than half speed, so placing the hand
    /// and crossing the grid are different gestures.
    #[test]
    fn a_half_deflection_moves_at_a_quarter_speed_not_half() {
        let cell = wide().cell();
        let full = cursor_travel(Vec2::X, cell, 1.0 / 60.0, NO_RAMP).length();
        let half = cursor_travel(Vec2::X * 0.5, cell, 1.0 / 60.0, NO_RAMP).length();
        assert!(
            (half / full - 0.25).abs() < 0.01,
            "a half deflection moved at {:.3} of full speed",
            half / full
        );
        // A fifth of the stick is a twenty-fifth of the speed.
        let nudge = cursor_travel(Vec2::X * 0.2, cell, 1.0 / 60.0, NO_RAMP).length();
        assert!(
            (nudge / full - 0.04).abs() < 0.01,
            "a fifth deflection moved at {:.3} of full speed",
            nudge / full
        );
    }

    /// The curve bends speed, not direction: a 45-degree push stays 45 degrees.
    #[test]
    fn the_response_curve_does_not_bend_the_direction_the_stick_is_pointing() {
        let cell = wide().cell();
        for stick in [
            Vec2::new(1.0, 1.0).normalize() * 0.4,
            Vec2::new(3.0, 1.0).normalize() * 0.9,
            Vec2::new(-1.0, 2.0).normalize() * 0.6,
        ] {
            let travel = cursor_travel(stick, cell, 1.0 / 60.0, NO_RAMP);
            let angle = travel.normalize().dot(stick.normalize());
            assert!(
                angle > 0.9999,
                "a stick at {stick:?} moved the cursor along {travel:?}"
            );
        }
    }

    /// A render hitch must not move the hand: a 100ms stall is clamped.
    #[test]
    fn a_long_frame_is_clamped_so_a_hitch_does_not_throw_the_cursor() {
        let cell = wide().cell();
        let hitch = cursor_travel(Vec2::X, cell, 0.100, NO_RAMP).length();
        let capped = cursor_travel(Vec2::X, cell, MAX_CURSOR_DT, NO_RAMP).length();
        assert!(
            (hitch - capped).abs() < 0.001,
            "a 100ms frame travelled {hitch:.1}px against the {capped:.1}px cap"
        );
        // An ordinary frame is not clamped, or the cap would be the speed.
        let ordinary = cursor_travel(Vec2::X, cell, 1.0 / 120.0, NO_RAMP).length();
        assert!(
            ordinary < capped * 0.5,
            "a 120Hz frame travelled {ordinary:.2}px, which is the cap, not the rate"
        );
    }

    /// A stick at rest moves nothing, without dividing by zero.
    #[test]
    fn a_stick_at_rest_moves_the_cursor_nowhere() {
        assert_eq!(
            cursor_travel(Vec2::ZERO, wide().cell(), 1.0 / 60.0, NO_RAMP),
            Vec2::ZERO
        );
    }

    /// Records both speeds against the old one (`1.15 * viewport.x`, about
    /// 2208px/s at 1920 wide, from a standing start): the base is about half of
    /// it; the top, reached only after about 0.78s of committed push, is a
    /// little above it. Writing down the top speed stops it creeping up.
    #[test]
    fn the_base_speed_is_half_the_rate_it_replaced_and_the_top_is_reached_slowly() {
        let layout = wide();
        let cell = layout.cell();
        let old_px_per_second = layout.viewport.x * 1.15;
        let dt = 1.0 / 60.0;
        let base = cursor_travel(Vec2::X, cell, dt, NO_RAMP).length() / dt;
        let top = cursor_travel(Vec2::X, cell, dt, RAMP_MAX_MULTIPLIER).length() / dt;
        println!(
            "cell {cell:?}: base {base:.0}px/s, top {top:.0}px/s (old was {old_px_per_second:.0})"
        );
        assert!(
            base < old_px_per_second * 0.6,
            "the starting speed is {base:.0}px/s against the old {old_px_per_second:.0} — \
             a short movement should be markedly calmer than the rate that was \
             reported as unusable"
        );
        assert!(
            top > old_px_per_second,
            "top speed is {top:.0}px/s, under the old rate — then the ramp is \
             not buying a faster journey and the base could simply be raised"
        );

        // A short push runs at the base rate, which makes the top speed safe.
        let mut ramp = CursorRamp::default();
        let correction = ramp.advance(Vec2::X, 0.12);
        assert_eq!(
            correction, 1.0,
            "a 120ms nudge already had built-up speed, so every small \
             repositioning inherits a journey's momentum"
        );
    }
}

/// The hold ramp: unhurried over short distances, quick over long ones.
#[cfg(test)]
mod cursor_ramp_tests {
    use super::*;

    /// Hold a full deflection for `seconds` in 60Hz frames, reporting the
    /// multiplier at the end.
    fn after_holding(seconds: f32) -> f32 {
        let mut ramp = CursorRamp::default();
        let dt = 1.0 / 60.0;
        let mut multiplier = 1.0;
        let frames = (seconds / dt).round() as i32;
        for _ in 0..frames {
            multiplier = ramp.advance(Vec2::X, dt);
        }
        multiplier
    }

    /// The shape: slow at first, faster later, then it stops building.
    #[test]
    fn speed_starts_at_the_base_rate_builds_while_held_and_then_stops_building() {
        let start = after_holding(0.05);
        let middle = after_holding(0.5);
        let end = after_holding(1.5);
        let much_later = after_holding(4.0);
        assert_eq!(start, 1.0, "a push already had speed before it was held");
        assert!(
            middle > start && middle < end,
            "held 0.05s/{start:.2}x, 0.5s/{middle:.2}x, 1.5s/{end:.2}x — not a build"
        );
        assert!(
            (end - RAMP_MAX_MULTIPLIER).abs() < 0.01,
            "a long hold reached {end:.2}x, not the {RAMP_MAX_MULTIPLIER}x the constant promises"
        );
        assert_eq!(
            much_later, end,
            "the ramp kept climbing past its stated maximum"
        );
    }

    /// A correction never ramps: nudging one portrait over ends well inside
    /// the arming delay.
    #[test]
    fn a_short_correction_never_leaves_the_base_speed() {
        for nudge in [0.016_f32, 0.05, 0.1, RAMP_ARM_SECONDS - 0.01] {
            assert_eq!(
                after_holding(nudge),
                1.0,
                "a {nudge:.3}s push had already started building speed"
            );
        }
    }

    /// A reversal drops the speed, so a correction does not inherit the
    /// overshoot's momentum.
    #[test]
    fn reversing_direction_gives_the_return_trip_base_speed_again() {
        let mut ramp = CursorRamp::default();
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            ramp.advance(Vec2::X, dt);
        }
        assert!(
            ramp.multiplier() > 1.5,
            "the sweep did not build enough speed for this arm to mean anything"
        );
        let back = ramp.advance(-Vec2::X, dt);
        assert_eq!(
            back, 1.0,
            "flicking back after an overshoot inherited {back:.2}x of the \
             speed that caused the overshoot"
        );
    }

    /// A right-angle turn is not a reversal: sweeping a row then a column keeps
    /// the speed.
    #[test]
    fn a_right_angle_turn_keeps_the_speed_it_built() {
        let mut ramp = CursorRamp::default();
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            ramp.advance(Vec2::X, dt);
        }
        let built = ramp.multiplier();
        let turned = ramp.advance(Vec2::Y, dt);
        assert!(
            turned >= built,
            "turning ninety degrees dropped the build from {built:.2}x to {turned:.2}x"
        );
    }

    /// A gentle push never accelerates, however long it is held.
    #[test]
    fn a_deflection_under_the_arming_threshold_never_builds_speed() {
        let mut ramp = CursorRamp::default();
        let dt = 1.0 / 60.0;
        let mut multiplier = 1.0;
        for _ in 0..180 {
            multiplier = ramp.advance(Vec2::X * (RAMP_ARM_DEFLECTION - 0.05), dt);
        }
        assert_eq!(
            multiplier, 1.0,
            "three seconds of a careful half-push accelerated to {multiplier:.2}x"
        );
        assert!(!ramp.is_building());
    }

    /// Releasing the stick forgets the build, or the next push starts fast.
    #[test]
    fn letting_go_forgets_the_speed_that_was_built() {
        let mut ramp = CursorRamp::default();
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            ramp.advance(Vec2::X, dt);
        }
        assert!(ramp.is_building());
        assert_eq!(ramp.advance(Vec2::ZERO, dt), 1.0);
        assert_eq!(
            ramp.advance(Vec2::X, dt),
            1.0,
            "the push after a release started with inherited speed"
        );
    }

    /// Smoothstep: the build has zero slope at both ends, so there is no frame
    /// where the cursor visibly changes gear.
    #[test]
    fn the_build_has_no_corner_at_either_end() {
        let dt = 1.0 / 60.0;
        let samples: Vec<f32> = (0..120)
            .map(|frame| after_holding(frame as f32 * dt))
            .collect();
        let jumps: Vec<f32> = samples.windows(2).map(|w| w[1] - w[0]).collect();
        // The first and last frames of the build move far less than its
        // middle, which a corner would not do.
        let biggest = jumps.iter().cloned().fold(0.0_f32, f32::max);
        let arming_frame = (RAMP_ARM_SECONDS / dt).ceil() as usize;
        let full_frame = ((RAMP_ARM_SECONDS + RAMP_TO_FULL_SECONDS) / dt).floor() as usize;
        assert!(
            jumps[arming_frame] < biggest * 0.25,
            "the build starts with a {:.4} step against a {biggest:.4} peak — that is a gear change",
            jumps[arming_frame]
        );
        assert!(
            jumps[full_frame - 1] < biggest * 0.25,
            "the build tops out with a {:.4} step against a {biggest:.4} peak",
            jumps[full_frame - 1]
        );
    }

    /// How long a full-stick sweep takes to cross a grid row, with and without
    /// the ramp. That difference is what the feature buys.
    #[test]
    fn a_full_row_sweep_is_meaningfully_quicker_than_it_would_be_without_the_ramp() {
        use crate::select_screen::layout::SelectLayout;
        let layout = SelectLayout::new(Vec2::new(1920.0, 1080.0), 8);
        let cell = layout.cell();
        let row = cell.x * 6.0;
        let dt = 1.0 / 60.0;

        let seconds_to_cross = |ramped: bool| {
            let mut ramp = CursorRamp::default();
            let mut travelled = 0.0;
            let mut elapsed = 0.0;
            while travelled < row && elapsed < 10.0 {
                let multiplier = ramp.advance(Vec2::X, dt);
                let multiplier = if ramped { multiplier } else { 1.0 };
                travelled += cursor_travel(Vec2::X, cell, dt, multiplier).length();
                elapsed += dt;
            }
            elapsed
        };
        let with_ramp = seconds_to_cross(true);
        let without = seconds_to_cross(false);
        println!("a six-cell row: {with_ramp:.2}s with the ramp, {without:.2}s without");
        assert!(
            with_ramp < without * 0.8,
            "the ramp saved only {:.2}s of a {without:.2}s sweep — not worth a \
             second speed the player has to learn",
            without - with_ramp
        );
        assert!(
            with_ramp > 0.5,
            "a full row goes by in {with_ramp:.2}s, which is quick enough to \
             overshoot the whole grid"
        );
    }

    /// The clamp that protects the travel integral protects the ramp too: a
    /// render hitch must not fast-forward the build either.
    #[test]
    fn a_long_frame_does_not_fast_forward_the_build() {
        let mut hitched = CursorRamp::default();
        let mut steady = CursorRamp::default();
        hitched.advance(Vec2::X, 1.0);
        steady.advance(Vec2::X, MAX_CURSOR_DT);
        assert_eq!(
            hitched.multiplier(),
            steady.multiplier(),
            "a one-second frame built more speed than the clamp allows"
        );
    }
}
