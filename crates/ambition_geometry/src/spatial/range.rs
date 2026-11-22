//! A uniform grid of bounds: which entries of a list may overlap a region.
//!
//! [`RangeGrid::near`] is CONSERVATIVE: it returns every entry whose bounds
//! touch the region (each once, in list order), and possibly others whose
//! cells meet it. A consumer that needs the exact answer filters the
//! candidates with [`touches`]; [`scan_touching`] is that exact answer by a
//! direct scan, the reference both must agree with. Visibility, overlays and
//! what an entry means stay with the consumer.

use crate::Aabb;

/// Do the two boxes share a point? Inclusive: a flat line's box has no area
/// and still touches what it lies on.
pub fn touches(a: Aabb, b: Aabb) -> bool {
    a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
}

/// The entries of `bounds` that touch `region`, by index, in list order: the
/// reference for [`RangeGrid`] over valid boxes (`min <= max`).
pub fn scan_touching(bounds: impl IntoIterator<Item = Aabb>, region: Aabb) -> Vec<u32> {
    bounds.into_iter().enumerate().filter(|(_, b)| touches(*b, region)).map(|(i, _)| i as u32).collect()
}

/// Where the entries of a list are: each grid cell, the entries whose bounds
/// reach it, in list order.
#[derive(Clone, Debug)]
pub struct RangeGrid {
    cell: f32,
    cells: Vec<((i32, i32), Vec<u32>)>,
    /// Indexed entries, once, in source order.
    all: Vec<u32>,
    /// Entries too big to file by cell (more than this many cells): asked
    /// every time.
    large: Vec<u32>,
}

const GRID_CELL: f32 = 128.0;
const GRID_LARGE_CELLS: i64 = 256;

impl RangeGrid {
    fn cells_of(&self, b: Aabb) -> (i32, i32, i32, i32) {
        let lo = (b.min / self.cell).floor();
        let hi = (b.max / self.cell).floor();
        (lo.x as i32, lo.y as i32, hi.x as i32, hi.y as i32)
    }

    /// Index a list by its entries' `bounds`, in list order. The bounds are
    /// expected valid (`min <= max`); an inverted box is filed by the cells
    /// its corners name, and one whose corners name no cell range is never a
    /// candidate.
    pub fn build(bounds: impl IntoIterator<Item = Aabb>) -> Self {
        let mut grid = Self { cell: GRID_CELL, cells: Default::default(), all: Vec::new(), large: Vec::new() };
        let mut cells = std::collections::BTreeMap::<(i32, i32), Vec<u32>>::new();
        for (i, b) in bounds.into_iter().enumerate() {
            let (x0, y0, x1, y1) = grid.cells_of(b);
            let large = (i64::from(x1) - i64::from(x0) + 1).saturating_mul(i64::from(y1) - i64::from(y0) + 1) > GRID_LARGE_CELLS;
            if large || (x0 <= x1 && y0 <= y1) {
                grid.all.push(i as u32);
            }
            if large {
                grid.large.push(i as u32);
                continue;
            }
            for x in x0..=x1 {
                for y in y0..=y1 {
                    cells.entry((x, y)).or_default().push(i as u32);
                }
            }
        }
        grid.cells = cells.into_iter().collect();
        grid
    }

    /// The entries whose cells meet `region`, each once, in list order: a
    /// superset of those that touch it.
    pub fn near(&self, region: Aabb) -> Vec<u32> {
        let (x0, y0, x1, y1) = self.cells_of(region);
        if (i64::from(x1) - i64::from(x0) + 1).saturating_mul(i64::from(y1) - i64::from(y0) + 1) > GRID_LARGE_CELLS * 16 {
            return self.all.clone();
        }
        let mut found: Vec<u32> = self.large.clone();
        for x in x0..=x1 {
            let first = self.cells.binary_search_by_key(&(x, y0), |(key, _)| *key);
            if y0 == y1 {
                if let Ok(index) = first {
                    found.extend_from_slice(&self.cells[index].1);
                }
                continue;
            }
            let first = first.unwrap_or_else(|insertion| insertion);
            for (key, entries) in &self.cells[first..] {
                if key.0 != x || key.1 > y1 {
                    break;
                }
                found.extend_from_slice(entries);
            }
        }
        // Cell lookup order must not change a consumer's order.
        found.sort_unstable();
        found.dedup();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    /// The grid's candidates, filtered by `touches`, are the scan's answer for
    /// small, cell-straddling, flat and large entries, and for regions small,
    /// flat, wide and huge.
    #[test]
    fn the_grid_filtered_by_touch_is_the_scan() {
        let b = |x: f32, y: f32, w: f32, h: f32| Aabb { min: Vec2::new(x, y), max: Vec2::new(x + w, y + h) };
        let mut bounds = vec![
            b(0.0, 0.0, 10.0, 10.0),
            b(120.0, 120.0, 20.0, 20.0),
            b(-300.0, 50.0, 600.0, 0.0),
            b(-5000.0, -5000.0, 10000.0, 10000.0),
        ];
        for i in 0..60 {
            let f = i as f32;
            bounds.push(b((f * 97.0) % 1500.0 - 700.0, (f * 53.0) % 900.0 - 400.0, 5.0 + f % 40.0, 3.0 + (f * 7.0) % 60.0));
        }
        let grid = RangeGrid::build(bounds.iter().copied());
        let regions = [
            b(0.0, 0.0, 1.0, 1.0),
            b(127.0, 127.0, 2.0, 2.0),
            b(-1000.0, 50.0, 2000.0, 0.0),
            b(-200.0, -200.0, 400.0, 400.0),
            b(-1.0e6, -1.0e6, 2.0e6, 2.0e6),
        ];
        for region in regions {
            let exact: Vec<u32> = grid.near(region).into_iter().filter(|i| touches(bounds[*i as usize], region)).collect();
            assert_eq!(exact, scan_touching(bounds.iter().copied(), region), "{region:?}");
        }
    }
}
