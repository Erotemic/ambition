//! Pure select-screen geometry shared by rendering and cursor hit-testing.
//!
//! Rectangles are derived directly from the viewport rather than read back from
//! Bevy UI layout, so rendering, hit-testing, and headless tests use one geometry
//! authority. [`GRID_FRACTION`] allocates the portrait grid above participant
//! cards.

use bevy::prelude::Vec2;

use super::SelectTarget;
use super::cursor::HitRect;
use crate::select::MAX_SMASH_SEATS;

pub const GRID_FRACTION: f32 = 0.65;

/// Viewport used by headless select-screen tests and capture tooling.
pub const HEADLESS_VIEWPORT: Vec2 = Vec2::new(1280.0, 720.0);

const MARGIN: f32 = 14.0;
const GAP: f32 = 10.0;
/// Title clearance for the shell's top-right controls.
const TITLE_H: f32 = 64.0;
const CONTROL_STRIP_H: f32 = 44.0;
const CARD_GAP: f32 = 8.0;
const ROLE_BUTTON_H: f32 = 30.0;
const START_W: f32 = 150.0;
/// The stage cycle's width. Narrower than START because it is a setting rather
/// than the commit, and still well over [`MIN_TOUCH_PX`].
const STAGE_W: f32 = 132.0;
const START_H: f32 = 34.0;
/// Back control width, paired with the start control.
const BACK_W: f32 = 150.0;
const BACK_H: f32 = 32.0;

/// Drawn token/cursor sizes; touch hit regions are expanded separately.
pub const TOKEN_PX: f32 = 26.0;
pub const CURSOR_PX: f32 = 22.0;

/// Minimum touch hit size, matching the repository's touch-input layout.
pub const MIN_TOUCH_PX: f32 = 44.0;

/// Upper bound on columns; [`SelectLayout::new`] may choose fewer to preserve
/// [`MIN_TOUCH_PX`].
const MAX_COLUMNS: usize = 6;

/// Every rectangle on the screen, derived from the viewport and the roster size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectLayout {
    pub viewport: Vec2,
    pub characters: usize,
    pub columns: usize,
    pub rows: usize,
    /// Which page of the grid is showing, already clamped into `0..pages`.
    pub page: usize,
    /// How many pages the roster needs at this size. `1` on a monitor.
    pub pages: usize,
    cell: Vec2,
    grid_origin: Vec2,
}

/// The cell a `columns x rows` grid gets, after both aspect clamps.
///
/// Free of `self` because [`SelectLayout::new`] has to ask it about grids it
/// has not chosen yet. Portraits are authored 256x320, so a cell wider than
/// 0.86 of its height is empty space either side of every face — and a cell
/// TALLER than its width allows is a stripe. Clamping only the first (which is
/// what this did until ) is why a width floor could be satisfied and
/// then silently undone one line later.
fn cell_size(area: Vec2, columns: usize, rows: usize) -> Vec2 {
    let raw = Vec2::new(
        (area.x - GAP * (columns as f32 - 1.0)) / columns as f32,
        (area.y - GAP * (rows as f32 - 1.0)) / rows as f32,
    );
    let width = raw.x.min(raw.y * PORTRAIT_ASPECT);
    Vec2::new(width, raw.y.min(width / PORTRAIT_ASPECT))
}

/// A portrait's width as a fraction of its height. See [`cell_size`].
const PORTRAIT_ASPECT: f32 = 0.86;

impl SelectLayout {
    pub fn new(viewport: Vec2, characters: usize) -> Self {
        Self::paged(viewport, characters, 0)
    }

    /// The layout showing `page` of the grid.
    pub fn paged(viewport: Vec2, characters: usize, page: usize) -> Self {
        let viewport = Vec2::new(viewport.x.max(320.0), viewport.y.max(240.0));
        let characters = characters.max(1);

        let grid_top = TITLE_H;
        let grid_bottom = viewport.y * GRID_FRACTION - CONTROL_STRIP_H;
        let area = Vec2::new(
            viewport.x - MARGIN * 2.0,
            (grid_bottom - grid_top).max(40.0),
        );

        // THE MOST FIGHTERS A THUMB CAN STILL HIT, AND THE REST ONTO A PAGE.
        //
        // eighteen portraits do not fit on a phone, and no constant makes
        // them. At 844x390 the grid band is 145px tall; three rows of it are
        // 42px cells, and the portrait aspect then drags the WIDTH down to 36 —
        // both under [`MIN_TOUCH_PX`]. Shrinking the chrome to buy the
        // difference gets to 44.5px, which is squeaking past a floor rather than
        // clearing it. The honest answer is fewer fighters on screen at once.
        //
        // so this searches for the grid that shows the MOST cells while every
        // cell still clears the floor, and pages whatever is left over. A
        // monitor's search finds the whole roster and reports one page, so the
        // desktop screen is unchanged and the paging never appears.
        //
        // never more rows than the roster needs — without that cap a tall
        // phone would lay out seven rows for eighteen fighters and leave four
        // empty, which is a grid that fits by describing fighters nobody has.
        let mut columns = 1usize;
        let mut rows = 1usize;
        let mut best = 0usize;
        for candidate_columns in 1..=MAX_COLUMNS.min(characters) {
            for candidate_rows in 1..=characters.div_ceil(candidate_columns) {
                let cell = cell_size(area, candidate_columns, candidate_rows);
                if cell.x < MIN_TOUCH_PX || cell.y < MIN_TOUCH_PX {
                    continue;
                }
                // FIGHTERS shown, not cells offered. Scoring the capacity
                // instead put four fighters in a 3x2 — six cells, two of them
                // describing nobody — because six beats four. A grid is as good
                // as the roster it shows.
                let shown = (candidate_columns * candidate_rows).min(characters);
                // Ties go to the SHALLOWER grid: the same fighters in fewer
                // rows is the "fewest rows the cap allows" rule this screen has
                // always had, and it is what keeps a four-fighter roster on one
                // line instead of folding it into a square.
                if shown > best || (shown == best && candidate_rows < rows) {
                    best = shown;
                    columns = candidate_columns;
                    rows = candidate_rows;
                }
            }
        }
        // a viewport too small for even one hittable cell still lays out.
        // Returning nothing here would collapse every rectangle and make every
        // hit test answer "nothing" — the vacuum [`HEADLESS_VIEWPORT`] exists to
        // avoid. One cell, under the floor, is a visible problem; no cells is an
        // invisible one.
        if best == 0 {
            columns = 1;
            rows = 1;
        }

        let per_page = columns * rows;
        let pages = characters.div_ceil(per_page);
        let page = page.min(pages - 1);

        // BALANCE THE ROWS OF THE LAST PAGE'S GRID. Eight fighters under a
        // plain `min(n, 6)` wrap 6 + 2, which reads as a grid with two strays
        // rather than as a roster. Only worth doing when everything fits on one
        // page — rebalancing a paged grid would change the cell size per page.
        //
        // this only ever REMOVES columns (`ceil(n / ceil(n / c)) <= c`),
        // so the floor chosen above survives the balancing.
        let (columns, rows) = if pages == 1 {
            let rows = characters.div_ceil(columns);
            (characters.div_ceil(rows), rows)
        } else {
            (columns, rows)
        };

        let cell = cell_size(area, columns, rows);
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
            page,
            pages,
            cell,
            grid_origin,
        }
    }

    /// One portrait cell's size in pixels.
    ///
    /// ⭐ THE CURSOR'S SPEED IS SCALED FROM THIS, not from the viewport, which is
    /// the whole reason it is public. A cursor measured in screen fractions gets
    /// faster relative to the things it is selecting as the grid grows, and
    /// slower as it shrinks; measured in CELLS it always takes the same time to
    /// cross one portrait, which is the distance the player is actually
    /// thinking in. See `CURSOR_CELLS_PER_SECOND`.
    pub fn cell(&self) -> Vec2 {
        self.cell
    }

    /// How many portraits one page shows.
    pub fn per_page(&self) -> usize {
        self.columns * self.rows
    }

    /// The roster indices this page draws, `first..end`.
    pub fn page_range(&self) -> std::ops::Range<usize> {
        let first = self.page * self.per_page();
        first..(first + self.per_page()).min(self.characters)
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

    /// One portrait's cell. `None` now also means "on another page", not only "past the end
    /// of the roster". A placed token on another page is hidden here while the slot card
    /// continues to show the fighter that was chosen.
    pub fn portrait(&self, index: usize) -> Option<HitRect> {
        if !self.page_range().contains(&index) {
            return None;
        }
        let on_page = index - self.page * self.per_page();
        let column = on_page % self.columns;
        let row = on_page / self.columns;
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

    /// The strip under the grid holding page controls, the prompt and START.
    fn control_strip(&self) -> HitRect {
        let bottom = self.viewport.y * GRID_FRACTION;
        HitRect {
            min: Vec2::new(MARGIN, bottom - CONTROL_STRIP_H),
            max: Vec2::new(self.viewport.x - MARGIN, bottom),
        }
    }

    /// The diameter a token is DRAWN at, scaled to the grid it sits on.
    ///
    /// A phone's cells are a third of a monitor's, and a token sized for the
    /// monitor covers the face underneath it. this is deliberately allowed to
    /// fall below [`MIN_TOUCH_PX`] — what a finger hits is [`Self::touchable`],
    /// and conflating the two is how a select screen ends up either unhittable
    /// or covered in blobs.
    pub fn token_px(&self) -> f32 {
        (self.cell.y * 0.30).clamp(20.0, TOKEN_PX)
    }

    /// The diameter a cursor is drawn at, in the token's proportion.
    pub fn cursor_px(&self) -> f32 {
        self.token_px() * (CURSOR_PX / TOKEN_PX)
    }

    /// Grow a rect about its own centre to the touch floor.
    ///
    /// the one place the floor is applied, so "drawn here" and "hittable
    /// there" stay one derivation. A rect already big enough comes back
    /// unchanged, so this is safe to wrap anything in.
    pub fn touchable(rect: HitRect) -> HitRect {
        let size = rect.size();
        HitRect::from_center_size(
            rect.center(),
            Vec2::new(size.x.max(MIN_TOUCH_PX), size.y.max(MIN_TOUCH_PX)),
        )
    }

    /// A page arrow, at the LEFT of the control strip, opposite START.
    ///
    /// sized to [`MIN_TOUCH_PX`] rather than to the strip's height: the strip
    /// shrinks with the viewport and these are the two controls a phone needs
    /// MOST, since a phone is the only place the grid pages at all.
    pub fn page_button(&self, next: bool) -> HitRect {
        let strip = self.control_strip();
        let size = Vec2::splat(MIN_TOUCH_PX);
        let left = strip.min.x + if next { MIN_TOUCH_PX + GAP } else { 0.0 };
        HitRect::from_center_size(
            Vec2::new(left + MIN_TOUCH_PX * 0.5, strip.center().y),
            size,
        )
    }

    /// The stage cycle, immediately left of START in the same strip.
    ///
    /// ⚠ Beside START rather than in the grid: choosing a stage is a MATCH
    /// decision like pressing start, not a per-seat one like picking a fighter,
    /// and putting it among the portraits would put it inside the region a
    /// cursor sweeps while choosing a character.
    pub fn stage_button(&self) -> HitRect {
        let strip = self.control_strip();
        HitRect::from_center_size(
            Vec2::new(
                strip.max.x - START_W - GAP - STAGE_W * 0.5,
                strip.center().y,
            ),
            Vec2::new(STAGE_W, START_H),
        )
    }

    pub fn start_button(&self) -> HitRect {
        let strip = self.control_strip();
        HitRect::from_center_size(
            Vec2::new(strip.max.x - START_W * 0.5, strip.center().y),
            Vec2::new(START_W, START_H),
        )
    }

    /// The way out of the lobby, at the LEFT end of the title strip.
    ///
    /// left, because the host owns the top-RIGHT corner. [`TITLE_H`]'s own
    /// comment says so: the shell draws Menu and Back there over whatever route
    /// is up, and a second Back under them would be two buttons fighting for one
    /// thumb. This strip is otherwise empty — the title text is centred — so the
    /// button costs no portrait and no card.
    pub fn back_button(&self) -> HitRect {
        let strip = self.title();
        HitRect::from_center_size(
            Vec2::new(MARGIN + BACK_W * 0.5, strip.center().y),
            Vec2::new(BACK_W, BACK_H),
        )
    }

    pub fn prompt(&self) -> HitRect {
        let strip = self.control_strip();
        HitRect {
            min: Vec2::new(strip.min.x, strip.center().y - 10.0),
            max: Vec2::new(strip.min.x + 520.0, strip.center().y + 10.0),
        }
    }

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

    /// Everything the cursor can act on, in a stable order.
    ///
    /// order is part of the contract.
    pub fn targets(&self) -> Vec<(SelectTarget, HitRect)> {
        let mut targets = Vec::with_capacity(self.characters + MAX_SMASH_SEATS + 4);
        for index in 0..self.characters {
            if let Some(rect) = self.portrait(index) {
                targets.push((SelectTarget::Portrait(index), rect));
            }
        }
        for slot in 0..MAX_SMASH_SEATS {
            targets.push((SelectTarget::RoleButton(slot), self.role_button(slot)));
        }
        targets.push((SelectTarget::Start, self.start_button()));
        // only when there IS another page. A button that turns to nowhere
        // is a target the cursor can land on and a finger can miss the grid for.
        if self.pages > 1 {
            targets.push((SelectTarget::PagePrev, self.page_button(false)));
            targets.push((SelectTarget::PageNext, self.page_button(true)));
        }
        // APPENDED, so every portrait keeps the position it already had.
        // The cursor names a target by its INDEX in this list, and inserting
        // anywhere but the end would silently re-point every walkthrough,
        // capture and test that reaches a cell by number.
        targets.push((SelectTarget::Back, self.back_button()));
        // ⚠ AFTER `Back`, for the same reason `Back` is last: this arrived after
        // every existing index was already spoken for. The order is the
        // contract, and "append" is the only edit to it that costs nothing.
        targets.push((SelectTarget::Stage, self.stage_button()));
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

    /// EVERY PORTRAIT IS REACHABLE FROM EVERY OTHER BY THE D-PAD ALONE.
    ///
    /// ⛔ **This is the property Jon's complaint was about** — *"the controls
    /// don't feel good, they are very hard to use with a gamepad"* — and the
    /// existing snap tests could not see it: they check one hop over a handful
    /// of synthetic rectangles, and unreachability is a property of the WHOLE
    /// graph. A grid where one portrait can only be entered and never left, or
    /// where a column is cut off from its neighbour, passes every one-hop test
    /// and is unusable on a pad.
    ///
    /// ⇒ Walks the real layout's real targets as a directed graph — four
    /// directions from each portrait, `cursor::snap` deciding each edge — and
    /// asserts the portraits form ONE strongly-reachable set from any start.
    #[test]
    fn every_portrait_is_reachable_from_every_other_by_the_dpad() {
        use crate::select_screen::cursor::{snap, CursorTarget};
        use bevy::prelude::Entity;
        use std::collections::{HashSet, VecDeque};

        // ⚠ The FULL roster. `wide()` uses `SmashRoster::default()`, which is
        // two characters — a grid of two cannot express a dead end, and the
        // size assertion below caught that fixture before it passed vacuously.
        let layout = SelectLayout::new(Vec2::new(1280.0, 720.0), roster());
        let targets = layout.targets();
        let rects: Vec<CursorTarget> = targets
            .iter()
            .enumerate()
            .filter_map(|(index, (_, rect))| {
                Some(CursorTarget {
                    entity: Entity::from_raw_u32(index as u32)?,
                    rect: *rect,
                })
            })
            .collect();
        let portraits: Vec<usize> = targets
            .iter()
            .enumerate()
            .filter(|(_, (kind, _))| matches!(kind, SelectTarget::Portrait(_)))
            .map(|(index, _)| index)
            .collect();
        assert!(
            portraits.len() > 4,
            "a grid with {} portraits cannot exercise this",
            portraits.len()
        );

        let dirs = [
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, -1.0),
        ];
        // Reachability from ONE portrait is enough for the property that
        // matters: the graph is symmetric under direction reversal, so a set
        // every portrait can be reached from is a set every portrait can reach.
        let start = portraits[0];
        let mut seen: HashSet<usize> = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            let from = rects[at].rect.center();
            for dir in dirs {
                let Some(next) = snap(from, dir, &rects) else {
                    continue;
                };
                let index = rects
                    .iter()
                    .position(|t| t.entity == next)
                    .expect("snap returns one of the targets it was given");
                if seen.insert(index) {
                    queue.push_back(index);
                }
            }
        }

        let unreachable: Vec<usize> = portraits
            .iter()
            .copied()
            .filter(|index| !seen.contains(index))
            .collect();
        assert!(
            unreachable.is_empty(),
            "{} of {} portraits cannot be reached from portrait {start} by any \
             sequence of d-pad presses: {unreachable:?}. A cell nobody can steer \
             to is a fighter nobody can pick with a pad.",
            unreachable.len(),
            portraits.len()
        );
    }

    /// A phone held sideways — the viewport this screen was unusable at.
    ///
    /// the FULL roster, not `SmashRoster::default()`. The default is this
    /// demo's own two stand-ins, which fit on a postage stamp; sizing a phone
    /// against two fighters would have every assertion below pass over a screen
    /// nobody will ever see. [`crate::select::SMASH_ROSTER`] is what a host
    /// composes.
    fn phone() -> SelectLayout {
        SelectLayout::new(PHONE_LANDSCAPE, roster())
    }

    fn roster() -> usize {
        crate::select::SMASH_ROSTER.len()
    }

    const PHONE_LANDSCAPE: Vec2 = Vec2::new(844.0, 390.0);

    /// EVERY PORTRAIT ON A PHONE IS BIG ENOUGH TO HIT.
    #[test]
    fn a_phone_shows_no_portrait_smaller_than_a_thumb() {
        let layout = phone();
        let mut checked = 0;
        for index in 0..layout.characters {
            let Some(rect) = layout.portrait(index) else {
                continue;
            };
            let size = rect.size();
            assert!(
                size.x >= MIN_TOUCH_PX && size.y >= MIN_TOUCH_PX,
                "portrait {index} is {size:?} on a phone, under the {MIN_TOUCH_PX}px floor"
            );
            checked += 1;
        }
        assert!(checked > 0, "a phone showed no portraits at all");
    }

    /// The token drawing may be compact, but its hit target still receives the
    /// same touch floor as every other direct-manipulation target. Token
    /// placement itself belongs to the select state, not to this layout.
    #[test]
    fn a_phone_offers_a_thumb_sized_token_hit_target() {
        let layout = phone();
        let drawn = HitRect::from_center_size(Vec2::ZERO, Vec2::splat(layout.token_px()));
        let size = SelectLayout::touchable(drawn).size();
        assert!(
            size.x >= MIN_TOUCH_PX && size.y >= MIN_TOUCH_PX,
            "a token is {size:?} to a finger"
        );
    }

    /// A PHONE PAGES THE ROSTER; A MONITOR DOES NOT.
    #[test]
    fn the_roster_pages_on_a_phone_and_fits_on_a_monitor() {
        let desktop = SelectLayout::new(Vec2::new(1280.0, 720.0), roster());
        assert_eq!(desktop.pages, 1, "the desktop grid grew pages");
        assert!(
            phone().pages > 1,
            "a phone claimed to fit the whole roster at a hittable size"
        );
    }

    /// EVERY FIGHTER IS ON EXACTLY ONE PAGE.
    ///
    /// a paged grid whose pages do not cover the roster hides a character
    /// nothing else would report — the screen looks complete and one fighter is
    /// unpickable.
    #[test]
    fn the_pages_cover_the_roster_exactly_once() {
        let characters = roster();
        let mut seen = vec![0usize; characters];
        let pages = phone().pages;
        for page in 0..pages {
            let layout = SelectLayout::paged(PHONE_LANDSCAPE, characters, page);
            for index in layout.page_range() {
                seen[index] += 1;
                assert!(
                    layout.portrait(index).is_some(),
                    "fighter {index} is in page {page}'s range but has no cell"
                );
            }
        }
        for (index, count) in seen.iter().enumerate() {
            assert_eq!(*count, 1, "fighter {index} appears on {count} pages");
        }
    }

    /// A page past the end shows the last one rather than an empty grid.
    #[test]
    fn a_page_past_the_end_clamps() {
        let characters = roster();
        let layout = SelectLayout::paged(PHONE_LANDSCAPE, characters, 99);
        assert_eq!(layout.page, layout.pages - 1);
        assert!(!layout.page_range().is_empty());
    }

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

    /// Nothing overlaps anything else it is not inside. two targets sharing
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

    /// Every fighter has a cell, and nobody past the end has one.
    #[test]
    fn the_grid_holds_exactly_the_roster() {
        let layout = wide();
        assert_eq!(layout.characters, SmashRoster::default().len());
        assert!(layout.portrait(layout.characters).is_none());
        assert!(
            layout.columns * layout.rows >= layout.characters,
            "the grid has fewer cells than fighters"
        );
    }

    /// The rows are BALANCED, or a roster reads as a grid with strays.
    ///
    /// found by looking at a capture: eight fighters under a plain
    /// `min(n, 6)` wrapped 6 + 2.
    #[test]
    fn the_grid_spreads_evenly_rather_than_filling_rows_to_the_cap() {
        for (characters, expected) in [(1, (1, 1)), (4, (4, 1)), (6, (6, 1)), (8, (4, 2)), (9, (5, 2)), (13, (5, 3))] {
            let layout = SelectLayout::new(Vec2::new(1280.0, 720.0), characters);
            assert_eq!(
                (layout.columns, layout.rows),
                expected,
                "{characters} fighters laid out {}x{}",
                layout.columns,
                layout.rows
            );
            let stragglers = layout.columns * layout.rows - characters;
            assert!(
                stragglers < layout.rows,
                "{characters} fighters left {stragglers} empty cells across \
                 {} rows, so the last row is a stub",
                layout.rows
            );
        }
    }

    /// a viewport nobody set must not collapse the screen to a point. A
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
        }
    }
}
