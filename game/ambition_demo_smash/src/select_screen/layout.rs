//! **Where everything on the select screen IS.**
//!
//! One pure function from a viewport size to a set of rectangles. The Bevy
//! nodes are positioned from it and the cursor hit-tests against it, so "what
//! you clicked" and "what you saw" cannot disagree — they are the same numbers.
//!
//! ## Why this is arithmetic and not flexbox
//!
//! The obvious build is a flex tree, and then hit-testing reads each node's
//! `ComputedNode` back out. That has two costs the moment a virtual cursor is
//! involved, and both were going to be paid:
//!
//! * **`bevy_ui` measures in `PostUpdate`**, so a freshly spawned node reads a
//!   ZERO rect for a frame — the cursor is over nothing exactly when the screen
//!   appears, and a zero rect at the origin is the nearest thing to everything.
//! * **it is only testable with a renderer.** The demo's own app is headless
//!   (`add_headless_foundation` — no `WindowPlugin`, no `UiPlugin`), which is
//!   what lets a test press buttons at all. A screen whose geometry only exists
//!   under a GPU is a screen whose geometry is never checked.
//!
//! Deriving the rectangles instead makes both go away, and costs one screenful
//! of arithmetic that is itself unit-testable. ⚠ the nodes are therefore
//! ABSOLUTELY positioned from this: a flex parent that also had opinions would
//! be the second authority this exists to avoid.
//!
//! ## Jon's proportions, named
//!
//! *"a grid of portraits … on the top 65% of the screen. The bottom 35% of the
//! screen should be 4 participant slot cards."* [`GRID_FRACTION`] is that 65%
//! and it is the only place it appears.

use bevy::prelude::Vec2;

use super::SelectTarget;
use super::cursor::HitRect;
use crate::select::MAX_SMASH_SEATS;

/// Jon's split: portraits above, cards below.
pub const GRID_FRACTION: f32 = 0.65;

/// The viewport a screen with no window is laid out for.
///
/// ⚠ **not a guess and not zero.** A headless app has no `Window`, and laying
/// out against `Vec2::ZERO` collapses every rectangle to a point — after which
/// every hit test answers "nothing" and every test of this screen passes
/// vacuously. 1280x720 is the size the demo's own tests and `capture_scene`
/// both use.
pub const HEADLESS_VIEWPORT: Vec2 = Vec2::new(1280.0, 720.0);

const MARGIN: f32 = 14.0;
const GAP: f32 = 10.0;
const TITLE_H: f32 = 36.0;
const POOL_H: f32 = 44.0;
const CARD_GAP: f32 = 8.0;
const ROLE_BUTTON_H: f32 = 30.0;
const START_W: f32 = 150.0;
const START_H: f32 = 34.0;

/// The token and cursor diameters. A token is a sphere, per Jon; in `bevy_ui` a
/// circle is a square with `BorderRadius::MAX`.
pub const TOKEN_PX: f32 = 26.0;
pub const CURSOR_PX: f32 = 22.0;

/// The most columns the grid will use before it starts adding rows. Six
/// 1280-wide columns are still a readable portrait; more is a row of stamps.
const MAX_COLUMNS: usize = 6;

/// Every rectangle on the screen, derived from the viewport and the roster size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectLayout {
    pub viewport: Vec2,
    pub characters: usize,
    pub columns: usize,
    pub rows: usize,
    cell: Vec2,
    grid_origin: Vec2,
}

impl SelectLayout {
    pub fn new(viewport: Vec2, characters: usize) -> Self {
        let viewport = Vec2::new(viewport.x.max(320.0), viewport.y.max(240.0));
        let characters = characters.max(1);
        let columns = characters.min(MAX_COLUMNS);
        let rows = characters.div_ceil(columns);

        let grid_top = TITLE_H;
        let grid_bottom = viewport.y * GRID_FRACTION - POOL_H;
        let area = Vec2::new(
            viewport.x - MARGIN * 2.0,
            (grid_bottom - grid_top).max(40.0),
        );
        let cell = Vec2::new(
            (area.x - GAP * (columns as f32 - 1.0)) / columns as f32,
            (area.y - GAP * (rows as f32 - 1.0)) / rows as f32,
        );
        // Portraits are authored 256x320, so a cell wider than 0.8 of its own
        // height is empty space either side of every face. Narrow the cell
        // rather than stretch the art — a select screen where the fighters are
        // subtly the wrong shape is worse than one with wider gutters.
        let cell = Vec2::new(cell.x.min(cell.y * 0.86), cell.y);
        let used_w = cell.x * columns as f32 + GAP * (columns as f32 - 1.0);
        let used_h = cell.y * rows as f32 + GAP * (rows as f32 - 1.0);
        let grid_origin = Vec2::new(
            (viewport.x - used_w) * 0.5,
            grid_top + (area.y - used_h) * 0.5,
        );

        Self {
            viewport,
            characters,
            columns,
            rows,
            cell,
            grid_origin,
        }
    }

    /// Laid out for whatever window exists, or for [`HEADLESS_VIEWPORT`].
    pub fn for_viewport(viewport: Option<Vec2>, characters: usize) -> Self {
        Self::new(viewport.unwrap_or(HEADLESS_VIEWPORT), characters)
    }

    pub fn title(&self) -> HitRect {
        HitRect {
            min: Vec2::new(0.0, 2.0),
            max: Vec2::new(self.viewport.x, TITLE_H),
        }
    }

    /// One portrait's cell. `None` past the end of the roster, rather than a
    /// wrapped or clamped cell — an index nobody authored is a bug to see, not
    /// a rectangle to invent.
    pub fn portrait(&self, index: usize) -> Option<HitRect> {
        if index >= self.characters {
            return None;
        }
        let column = index % self.columns;
        let row = index / self.columns;
        let min = self.grid_origin
            + Vec2::new(
                column as f32 * (self.cell.x + GAP),
                row as f32 * (self.cell.y + GAP),
            );
        Some(HitRect {
            min,
            max: min + self.cell,
        })
    }

    /// The strip under the grid holding the unplaced tokens, the prompt and
    /// START.
    fn pool(&self) -> HitRect {
        let bottom = self.viewport.y * GRID_FRACTION;
        HitRect {
            min: Vec2::new(MARGIN, bottom - POOL_H),
            max: Vec2::new(self.viewport.x - MARGIN, bottom),
        }
    }

    /// Where a slot's token rests when it is on nobody. Centred as a group, so
    /// four resting tokens read as a row of pieces waiting rather than as four
    /// unrelated dots.
    pub fn token_home(&self, slot: usize) -> HitRect {
        let pool = self.pool();
        let span = TOKEN_PX * MAX_SMASH_SEATS as f32 + GAP * (MAX_SMASH_SEATS as f32 - 1.0);
        let left = pool.center().x - span * 0.5 + slot as f32 * (TOKEN_PX + GAP);
        HitRect::from_center_size(
            Vec2::new(left + TOKEN_PX * 0.5, pool.center().y),
            Vec2::splat(TOKEN_PX),
        )
    }

    pub fn start_button(&self) -> HitRect {
        let pool = self.pool();
        HitRect::from_center_size(
            Vec2::new(pool.max.x - START_W * 0.5, pool.center().y),
            Vec2::new(START_W, START_H),
        )
    }

    pub fn prompt(&self) -> HitRect {
        let pool = self.pool();
        HitRect {
            min: Vec2::new(pool.min.x, pool.center().y - 10.0),
            max: Vec2::new(pool.min.x + 520.0, pool.center().y + 10.0),
        }
    }

    /// One participant card. Four equal columns across Jon's bottom 35%.
    pub fn card(&self, slot: usize) -> HitRect {
        let top = self.viewport.y * GRID_FRACTION;
        let width = (self.viewport.x - MARGIN * 2.0 - CARD_GAP * (MAX_SMASH_SEATS as f32 - 1.0))
            / MAX_SMASH_SEATS as f32;
        let min = Vec2::new(MARGIN + slot as f32 * (width + CARD_GAP), top + CARD_GAP);
        HitRect {
            min,
            max: Vec2::new(min.x + width, self.viewport.y - CARD_GAP),
        }
    }

    /// The button that cycles one card between controller / CPU / absent.
    pub fn role_button(&self, slot: usize) -> HitRect {
        let card = self.card(slot);
        let width = card.size().x * 0.88;
        HitRect::from_center_size(
            Vec2::new(card.center().x, card.min.y + 32.0 + ROLE_BUTTON_H * 0.5),
            Vec2::new(width, ROLE_BUTTON_H),
        )
    }

    /// The chosen fighter's portrait on a card.
    pub fn card_portrait(&self, slot: usize) -> HitRect {
        let card = self.card(slot);
        let height = (card.size().y - 90.0).clamp(30.0, 120.0);
        HitRect::from_center_size(
            Vec2::new(card.center().x, card.min.y + 78.0 + height * 0.5),
            Vec2::new(height * 0.8, height),
        )
    }

    /// **Everything the cursor can act on, in a stable order.**
    ///
    /// ⚠ order is part of the contract. Ties in `snap` and `hovered` are broken
    /// by the first candidate at equal cost, and a set assembled in a different
    /// order on a different run would resolve those ties differently — which is
    /// the same class of defect as reading an unordered Bevy query.
    pub fn targets(&self) -> Vec<(SelectTarget, HitRect)> {
        let mut targets = Vec::with_capacity(self.characters + MAX_SMASH_SEATS * 2 + 1);
        for index in 0..self.characters {
            if let Some(rect) = self.portrait(index) {
                targets.push((SelectTarget::Portrait(index), rect));
            }
        }
        for slot in 0..MAX_SMASH_SEATS {
            targets.push((SelectTarget::RoleButton(slot), self.role_button(slot)));
        }
        for slot in 0..MAX_SMASH_SEATS {
            targets.push((SelectTarget::Token(slot), self.token_home(slot)));
        }
        targets.push((SelectTarget::Start, self.start_button()));
        targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SmashRoster;

    fn wide() -> SelectLayout {
        SelectLayout::new(Vec2::new(1280.0, 720.0), SmashRoster::default().len())
    }

    /// **Jon's 65/35 split, checked where it is claimed.** The grid lives above
    /// the line and the cards below it, with nothing crossing.
    #[test]
    fn the_grid_is_the_top_65_percent_and_the_cards_the_bottom_35() {
        let layout = wide();
        let line = 720.0 * GRID_FRACTION;
        for index in 0..layout.characters {
            let cell = layout.portrait(index).expect("an authored portrait");
            assert!(
                cell.max.y <= line,
                "portrait {index} reaches {} and the cards start at {line}",
                cell.max.y
            );
        }
        for slot in 0..MAX_SMASH_SEATS {
            assert!(
                layout.card(slot).min.y >= line,
                "slot card {slot} starts above the 65% line"
            );
        }
        assert!(layout.start_button().max.y <= line);
    }

    /// Nothing overlaps anything else it is not inside. ⛔ two targets sharing
    /// pixels is a click that lands on whichever the tie-break happened to
    /// prefer, and it is invisible until somebody misses a portrait.
    #[test]
    fn no_two_clickable_targets_overlap() {
        let layout = wide();
        let targets = layout.targets();
        for (i, (a_kind, a)) in targets.iter().enumerate() {
            for (b_kind, b) in targets.iter().skip(i + 1) {
                let overlaps = a.min.x < b.max.x
                    && b.min.x < a.max.x
                    && a.min.y < b.max.y
                    && b.min.y < a.max.y;
                assert!(
                    !overlaps,
                    "{a_kind:?} and {b_kind:?} share pixels: {a:?} vs {b:?}"
                );
            }
        }
    }

    /// Everything is inside the window. A card off the bottom edge is a card
    /// nobody can click, and it would still pass every other test here.
    #[test]
    fn every_target_is_on_screen() {
        for size in [
            Vec2::new(1280.0, 720.0),
            Vec2::new(1920.0, 1080.0),
            Vec2::new(640.0, 360.0),
            Vec2::new(2340.0, 1080.0),
        ] {
            let layout = SelectLayout::new(size, SmashRoster::default().len());
            for (kind, rect) in layout.targets() {
                assert!(
                    rect.min.x >= -0.5
                        && rect.min.y >= -0.5
                        && rect.max.x <= layout.viewport.x + 0.5
                        && rect.max.y <= layout.viewport.y + 0.5,
                    "{kind:?} is off a {size:?} screen: {rect:?}"
                );
                assert!(!rect.is_unmeasured(), "{kind:?} has no area at {size:?}");
            }
        }
    }

    /// **Every fighter has a cell**, and nobody past the end has one.
    #[test]
    fn the_grid_holds_exactly_the_roster() {
        let layout = wide();
        assert_eq!(layout.characters, SmashRoster::default().len());
        assert!(layout.portrait(layout.characters).is_none());
        assert_eq!(
            layout.columns * layout.rows >= layout.characters,
            true,
            "the grid has fewer cells than fighters"
        );
    }

    /// ⚠ **a viewport nobody set must not collapse the screen to a point.** A
    /// headless app has no window; laying out against zero would make every hit
    /// test answer "nothing" and every test of this screen pass over an empty
    /// box.
    #[test]
    fn a_missing_window_lays_out_against_a_real_size() {
        let layout = SelectLayout::for_viewport(None, 8);
        assert_eq!(layout.viewport, HEADLESS_VIEWPORT);
        assert!(!layout.portrait(0).expect("a first cell").is_unmeasured());
        let degenerate = SelectLayout::new(Vec2::ZERO, 8);
        assert!(
            !degenerate
                .portrait(0)
                .expect("a first cell")
                .is_unmeasured()
        );
    }

    /// The cards are in reading order and the same width. A couch reads the
    /// cards left to right and the tokens have to match.
    #[test]
    fn the_four_cards_run_left_to_right_at_one_width() {
        let layout = wide();
        let widths: Vec<f32> = (0..MAX_SMASH_SEATS)
            .map(|slot| layout.card(slot).size().x)
            .collect();
        for width in &widths {
            assert!((width - widths[0]).abs() < 0.01);
        }
        for slot in 1..MAX_SMASH_SEATS {
            assert!(layout.card(slot).min.x > layout.card(slot - 1).min.x);
            assert!(layout.token_home(slot).min.x > layout.token_home(slot - 1).min.x);
        }
    }
}
