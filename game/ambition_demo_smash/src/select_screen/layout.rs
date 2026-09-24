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
/// The stage cycle's width: narrower than START (a setting, not the commit),
/// and well over [`MIN_TOUCH_PX`].
const STAGE_W: f32 = 132.0;
/// The stocks cycle's width: narrower than the stage's (a one-digit label),
/// and well over [`MIN_TOUCH_PX`].
const STOCKS_W: f32 = 108.0;
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
/// Free of `self` because [`SelectLayout::new`] asks it about grids it has not
/// chosen yet. Portraits are authored 256x320, so a cell wider than 0.86 of its
/// height leaves empty space beside every face, and a taller cell is a stripe.
/// Both clamps are needed, or a width floor can be undone one line later.
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

        // The most fighters a thumb can still hit; the rest go onto pages.
        //
        // Eighteen portraits do not fit on a phone: at 844x390 the grid band
        // is 145px, and three rows give cells under [`MIN_TOUCH_PX`]. So search
        // for the grid that shows the most cells while every cell clears the
        // floor, and page the rest. On a monitor the search finds the whole
        // roster on one page.
        //
        // Never more rows than the roster needs, or a tall phone would lay out
        // empty rows.
        let mut columns = 1usize;
        let mut rows = 1usize;
        let mut best = 0usize;
        for candidate_columns in 1..=MAX_COLUMNS.min(characters) {
            for candidate_rows in 1..=characters.div_ceil(candidate_columns) {
                let cell = cell_size(area, candidate_columns, candidate_rows);
                if cell.x < MIN_TOUCH_PX || cell.y < MIN_TOUCH_PX {
                    continue;
                }
                // Score fighters shown, not cells offered, or four fighters
                // would get a 3x2 with two empty cells.
                let shown = (candidate_columns * candidate_rows).min(characters);
                // Ties go to the shallower grid, so a four-fighter roster stays
                // on one line.
                if shown > best || (shown == best && candidate_rows < rows) {
                    best = shown;
                    columns = candidate_columns;
                    rows = candidate_rows;
                }
            }
        }
        // A viewport too small for one hittable cell still lays out. With no
        // cells, every rect would collapse and every hit test answer "nothing"
        // (the problem [`HEADLESS_VIEWPORT`] avoids). One undersized cell is a
        // visible problem.
        if best == 0 {
            columns = 1;
            rows = 1;
        }

        let per_page = columns * rows;
        let pages = characters.div_ceil(per_page);
        let page = page.min(pages - 1);

        // Balance the rows: eight fighters under `min(n, 6)` wrap 6 + 2. Only
        // when everything fits on one page, or the cell size would change per
        // page. This only removes columns (`ceil(n / ceil(n / c)) <= c`), so
        // the floor chosen above survives.
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
    /// Public because the cursor speed is scaled from it
    /// (`CURSOR_CELLS_PER_SECOND`): crossing one portrait always takes the same
    /// time, whatever the grid size.
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

    /// One portrait's cell. `None` means past the roster or on another page. A
    /// placed token on another page is hidden, while the slot card still shows
    /// the chosen fighter.
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

    /// The diameter a token is drawn at, scaled to the grid it sits on.
    ///
    /// A phone's cells are a third of a monitor's, so a fixed token would cover
    /// the face. It may fall below [`MIN_TOUCH_PX`]; what a finger hits is
    /// [`Self::touchable`].
    pub fn token_px(&self) -> f32 {
        (self.cell.y * 0.30).clamp(20.0, TOKEN_PX)
    }

    /// The diameter a cursor is drawn at, in the token's proportion.
    pub fn cursor_px(&self) -> f32 {
        self.token_px() * (CURSOR_PX / TOKEN_PX)
    }

    /// Grow a rect about its own centre to the touch floor.
    ///
    /// The one place the floor is applied, so "drawn here" and "hittable there"
    /// share a derivation. A rect already big enough is unchanged.
    pub fn touchable(rect: HitRect) -> HitRect {
        let size = rect.size();
        HitRect::from_center_size(
            rect.center(),
            Vec2::new(size.x.max(MIN_TOUCH_PX), size.y.max(MIN_TOUCH_PX)),
        )
    }

    /// A page arrow, at the left of the control strip, opposite START.
    ///
    /// Sized to [`MIN_TOUCH_PX`], not the strip height: the strip shrinks with
    /// the viewport, and only a phone pages the grid.
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
    /// Beside START, not in the grid: the stage is a match decision, and it
    /// must stay out of the area a cursor sweeps while choosing a fighter.
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

    /// The stocks cycle, left of the stage cycle: both are match decisions,
    /// read in order: how many stocks, on which stage, then GO.
    pub fn stocks_button(&self) -> HitRect {
        let strip = self.control_strip();
        HitRect::from_center_size(
            Vec2::new(
                strip.max.x - START_W - GAP - STAGE_W - GAP - STOCKS_W * 0.5,
                strip.center().y,
            ),
            Vec2::new(STOCKS_W, START_H),
        )
    }

    pub fn start_button(&self) -> HitRect {
        let strip = self.control_strip();
        HitRect::from_center_size(
            Vec2::new(strip.max.x - START_W * 0.5, strip.center().y),
            Vec2::new(START_W, START_H),
        )
    }

    /// The way out of the lobby, at the left end of the title strip.
    ///
    /// Left, because the shell draws Menu and Back in the top-right corner
    /// (see [`TITLE_H`]). The title text is centred, so this costs no portrait
    /// or card.
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
    /// Everything the cursor can act on, in a stable order. The order is part
    /// of the contract.
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
        // Only when there is another page; otherwise the arrows are dead
        // targets.
        if self.pages > 1 {
            targets.push((SelectTarget::PagePrev, self.page_button(false)));
            targets.push((SelectTarget::PageNext, self.page_button(true)));
        }
        // Append only: the cursor names a target by its index here, so an
        // insert would re-point every walkthrough, capture and test.
        targets.push((SelectTarget::Back, self.back_button()));
        // Appended after `Back`, by the same contract.
        targets.push((SelectTarget::Stage, self.stage_button()));
        // Appended after `Stage`, by the same contract.
        targets.push((SelectTarget::Stocks, self.stocks_button()));
        targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::SmashRoster;

    fn wide() -> SelectLayout {
        SelectLayout::new(Vec2::new(1280.0, 720.0), roster())
    }

    /// Every portrait is reachable from every other by the d-pad alone.
    ///
    /// One-hop snap tests cannot see this: a portrait that can be entered but
    /// not left passes them. This walks the real layout's targets as a directed
    /// graph (four directions per portrait, `cursor::snap` for each edge) and
    /// asserts all portraits are reachable.
    #[test]
    fn every_portrait_is_reachable_from_every_other_by_the_dpad() {
        use crate::select_screen::cursor::{snap, CursorTarget};
        use bevy::prelude::Entity;
        use std::collections::{HashSet, VecDeque};

        // The full roster: a two-character grid cannot express a dead end.
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
        // Reachability from one portrait is enough: the graph is symmetric
        // under direction reversal.
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

    /// How many presses it takes to cross the grid: reachable is not the same
    /// as reasonable.
    ///
    /// The measured diameter is 8 on the 23-cell desktop grid. The bound is 12,
    /// deliberately loose, so it fires on a real regression and not on a layout
    /// tweak.
    #[test]
    fn crossing_the_grid_stays_within_a_handful_of_presses() {
        use crate::select_screen::cursor::{snap, CursorTarget};
        use bevy::prelude::Entity;
        use std::collections::{HashMap, VecDeque};

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

        let dirs = [
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, -1.0),
        ];
        let mut worst = 0usize;
        for &start in &portraits {
            let mut dist: HashMap<usize, usize> = HashMap::from([(start, 0)]);
            let mut queue = VecDeque::from([start]);
            while let Some(at) = queue.pop_front() {
                let d = dist[&at];
                let from = rects[at].rect.center();
                for dir in dirs {
                    let Some(next) = snap(from, dir, &rects) else {
                        continue;
                    };
                    let index = rects
                        .iter()
                        .position(|t| t.entity == next)
                        .expect("snap returns one of the targets it was given");
                    dist.entry(index).or_insert_with(|| {
                        queue.push_back(index);
                        d + 1
                    });
                }
            }
            for &p in &portraits {
                worst = worst.max(dist.get(&p).copied().unwrap_or(usize::MAX));
            }
        }

        assert!(
            worst <= 12,
            "the farthest two portraits are {worst} d-pad presses apart on a \
             {}-cell grid. Reachable is not the same as reasonable, and a player \
             holding a pad feels the difference before they can name it.",
            portraits.len()
        );
        // Anti-vacuity: the grid must take several presses to cross.
        assert!(
            worst >= 3,
            "the whole grid is {worst} presses across, so the bound above cannot \
             fail for any layout and is not measuring anything"
        );
    }

    /// A phone held sideways, with the full roster (see [`roster`]); the
    /// default two stand-ins would fit anywhere.
    fn phone() -> SelectLayout {
        SelectLayout::new(PHONE_LANDSCAPE, roster())
    }

    /// The widest grid any composition can build: the wish list plus random.
    ///
    /// Production passes `fighters.cell_count()` (one per fighter, plus the
    /// [`crate::select::SlotPick::Random`] cell). This is an upper bound: the
    /// assembled roster is 4 cells in the standalone demo, 9 or more in the
    /// composed app. The widest grid is the stress case (most pages, tightest
    /// packing); [`the_demo_sized_grid_also_packs_without_overlap`] covers the
    /// small end.
    fn roster() -> usize {
        // Mirrors `SmashRoster::cell_count()`; `SMASH_ROSTER` is a slice, so
        // the `+ 1` for random is spelled out.
        crate::select::SMASH_ROSTER.len() + 1
    }

    const PHONE_LANDSCAPE: Vec2 = Vec2::new(844.0, 390.0);

    /// Every portrait on a phone is big enough to hit.
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

    /// The token may be drawn small, but its hit target gets the touch floor.
    /// (This is close to a tautology: `touchable` clamps to the floor.)
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

    /// The other end of the range: the grid the standalone demo builds (four
    /// cells). Larger portraits can hit the control strip from the other
    /// direction, so neither end implies the other.
    #[test]
    fn the_demo_sized_grid_also_packs_without_overlap() {
        // 3 seatable fighters + random, as `SmashRoster::assemble` gives in the
        // standalone composition.
        let layout = SelectLayout::new(PHONE_LANDSCAPE, 4);
        let targets: Vec<_> = layout
            .targets()
            .into_iter()
            .map(|(kind, rect)| (kind, SelectLayout::touchable(rect)))
            .collect();
        const EPS: f32 = 0.5;
        for (i, (a_kind, a)) in targets.iter().enumerate() {
            for (b_kind, b) in targets.iter().skip(i + 1) {
                let dx = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
                let dy = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0);
                assert!(
                    dx.min(dy) <= EPS,
                    "on the DEMO-sized grid the touch boxes of {a_kind:?} and \
                     {b_kind:?} overlap by {:.2}px — the wide-grid test cannot \
                     see this, because four big portraits collide differently \
                     from twenty-four small ones",
                    dx.min(dy)
                );
            }
        }
    }

    /// On a phone, no two touchable targets overlap.
    ///
    /// `no_two_clickable_targets_overlap` checks drawn rects on a desktop. A
    /// finger hits [`SelectLayout::touchable`], which expands small targets to
    /// [`MIN_TOUCH_PX`], so two small separate targets can share expanded
    /// pixels.
    #[test]
    fn a_phone_leaves_no_two_touch_targets_fighting_over_the_same_pixels() {
        let layout = phone();
        let targets: Vec<_> = layout
            .targets()
            .into_iter()
            .map(|(kind, rect)| (kind, SelectLayout::touchable(rect)))
            .collect();
        // An epsilon separates "overlap" from "adjacent": targets that abut at
        // a grid line can overlap by float rounding (0.00002 px).
        const EPS: f32 = 0.5;
        let mut tightest = f32::INFINITY;
        for (i, (a_kind, a)) in targets.iter().enumerate() {
            for (b_kind, b) in targets.iter().skip(i + 1) {
                // Positive on each axis means overlap on that axis. The boxes
                // overlap only when both are positive; the smaller is the depth.
                let depth_x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
                let depth_y = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0);
                let depth = depth_x.min(depth_y);
                tightest = tightest.min(depth);
                assert!(
                    depth <= EPS,
                    "on a phone the TOUCH boxes of {a_kind:?} and {b_kind:?} \
                     overlap by {depth:.2}px ({a:?} vs {b:?}) — both may be drawn \
                     apart, but the {MIN_TOUCH_PX}px floor expands them into each \
                     other, so one of the two is untappable"
                );
            }
        }

        // The layout has no room to spare: `START_H` (34px) is under the 44px
        // floor, so the strip buttons are expanded, and the stage button
        // reaches exactly the grid line. This fails if the margin goes
        // properly negative.
        assert!(
            tightest <= EPS,
            "the tightest touch-box pair is {tightest:.2}px apart, which should be \
             impossible given the assertion above"
        );
    }

    /// A phone pages the roster; a monitor does not.
    #[test]
    fn the_roster_pages_on_a_phone_and_fits_on_a_monitor() {
        let desktop = SelectLayout::new(Vec2::new(1280.0, 720.0), roster());
        assert_eq!(desktop.pages, 1, "the desktop grid grew pages");
        assert!(
            phone().pages > 1,
            "a phone claimed to fit the whole roster at a hittable size"
        );
    }

    /// Every fighter is on exactly one page; otherwise a fighter is silently
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

    /// Nothing overlaps anything it is not inside: shared pixels send a click
    /// to whichever target the tie-break prefers.
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

    /// Everything is inside the window: a card off the bottom edge cannot be
    /// clicked.
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

    /// The last cell is the random cell, and it is drawn.
    ///
    /// It is the cell most likely to be pushed into the control strip or off a
    /// page. This pins that the layout draws it and the roster answers it with
    /// [`SlotPick::Random`].
    #[test]
    fn the_last_grid_cell_is_random_and_is_drawn() {
        let layout = wide();
        let last = layout.characters - 1;

        // It is drawn: a grid sized for the cell but not laying it out would
        // pass every count assertion.
        assert!(
            layout.portrait(last).is_some(),
            "the last cell has no portrait rect, so the random square is sized \
             for and never placed"
        );
        assert!(
            layout.portrait(layout.characters).is_none(),
            "the grid laid out a cell past its own count"
        );

        // It is random, not a fighter: `SmashRoster::cell` answers `Random` at
        // `len()`, the last index of a `cell_count()`-sized grid.
        let roster = SmashRoster::default();
        assert_eq!(
            roster.cell(roster.len()),
            Some(crate::select::SlotPick::Random),
            "the cell past the last fighter is not the random cell"
        );
        assert!(
            matches!(roster.cell(roster.len() - 1), Some(crate::select::SlotPick::Fighter(_))),
            "the cell before it is not a fighter, so the boundary is off by one"
        );
        assert_eq!(
            roster.cell(roster.cell_count()),
            None,
            "a cell past the grid answered something"
        );
    }

    /// Every fighter has a cell, and nobody past the end has one.
    #[test]
    fn the_grid_holds_the_roster_plus_the_random_cell() {
        let layout = wide();
        // Compare with the fixture's own source (`roster()`), not
        // `SmashRoster::default()` (the two stand-ins): the layout holds the
        // cells it was asked for, one per entry plus random.
        assert_eq!(layout.characters, roster());
        assert_eq!(
            roster(),
            crate::select::SMASH_ROSTER.len() + 1,
            "the fixture's cell count drifted from the wish list plus random"
        );
        assert!(layout.portrait(layout.characters).is_none());
        assert!(
            layout.columns * layout.rows >= layout.characters,
            "the grid has fewer cells than fighters"
        );
    }

    /// The rows are balanced: eight fighters under a plain `min(n, 6)` would
    /// wrap 6 + 2.
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

    /// A missing window must not collapse the screen to a point: a headless
    /// app would hit-test nothing and every test would pass over an empty box.
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

    /// The cards run left to right at one width, the order a couch reads them.
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
