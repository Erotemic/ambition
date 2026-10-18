//! A list shared between worlds, with a layer of its own on top: the room's
//! blocks (or lines) as one shared base, and per world a few of them hidden
//! and a few more added. What a body in a portal pair meets is the room's
//! world with the hole it opened and the far side it is glued to
//! (`docs/planning/engine/portals-a-body-in-two-places.md`): that world is a
//! clone of the room's (one shared base, nothing copied) and a small layer,
//! so it costs the same in a room of ten blocks or ten thousand.
//!
//! A reader sees one list, in order: the base without what is hidden, then
//! what is added. It reads it as it reads a `Vec` (`iter`, `len`, `[i]`,
//! `get`). A writer that changes the list as a whole (`push`, `retain`,
//! `[i] = ..`) gets its own copy of the base with the layer folded in, as a
//! `Vec` it owns would be.

use std::sync::{Arc, OnceLock};

use crate::{Aabb, Vec2};

/// A shared base list, a set of its entries hidden, and entries added.
#[derive(Clone)]
pub struct Layered<T> {
    base: Arc<Base<T>>,
    /// Indices into `base`, sorted, each once.
    hidden: Vec<u32>,
    added: Vec<T>,
}

impl<T> Default for Layered<T> {
    fn default() -> Self {
        Self { base: Arc::new(Base::new(Vec::new())), hidden: Vec::new(), added: Vec::new() }
    }
}

impl<T> From<Vec<T>> for Layered<T> {
    fn from(base: Vec<T>) -> Self {
        Self { base: Arc::new(Base::new(base)), hidden: Vec::new(), added: Vec::new() }
    }
}

impl<T> FromIterator<T> for Layered<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl<T> Layered<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.base.len() - self.hidden.len() + self.added.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> LayeredIter<'_, T> {
        LayeredIter { base: self.base.iter().enumerate(), hidden: &self.hidden, added: self.added.iter(), left: self.len() }
    }

    /// Where the `i`th entry a reader sees is: in the base, or added.
    fn locate(&self, i: usize) -> Option<Slot> {
        if self.hidden.is_empty() {
            return if i < self.base.len() {
                Some(Slot::Base(i))
            } else {
                (i - self.base.len() < self.added.len()).then(|| Slot::Added(i - self.base.len()))
            };
        }
        let shown = self.base.len() - self.hidden.len();
        if i >= shown {
            return (i - shown < self.added.len()).then(|| Slot::Added(i - shown));
        }
        // The base index with `i` shown entries before it: step past each
        // hidden index at or before the candidate.
        let mut at = i;
        for hidden in &self.hidden {
            if (*hidden as usize) <= at {
                at += 1;
            } else {
                break;
            }
        }
        Some(Slot::Base(at))
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        match self.locate(i)? {
            Slot::Base(at) => self.base.get(at),
            Slot::Added(at) => self.added.get(at),
        }
    }

    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    pub fn last(&self) -> Option<&T> {
        self.len().checked_sub(1).and_then(|i| self.get(i))
    }

    /// Hide the `i`th entry a reader sees, from this list only: the base is
    /// shared and stays as it is. Added entries are removed.
    pub fn hide(&mut self, i: usize) {
        match self.locate(i) {
            Some(Slot::Base(at)) => {
                if let Err(pos) = self.hidden.binary_search(&(at as u32)) {
                    self.hidden.insert(pos, at as u32);
                }
            }
            Some(Slot::Added(at)) => {
                self.added.remove(at);
            }
            None => {}
        }
    }

    /// Add an entry after every other, to this list only.
    pub fn add(&mut self, value: T) {
        self.added.push(value);
    }

    /// Add entries after every other, to this list only.
    pub fn add_all(&mut self, values: impl IntoIterator<Item = T>) {
        self.added.extend(values);
    }

    /// Hide every entry `hide` picks, from this list only.
    pub fn hide_where(&mut self, mut hide: impl FnMut(&T) -> bool) {
        let mut hidden = self.hidden.iter().peekable();
        let mut newly: Vec<u32> = Vec::new();
        for (i, value) in self.base.iter().enumerate() {
            if hidden.peek().is_some_and(|h| **h as usize == i) {
                hidden.next();
                continue;
            }
            if hide(value) {
                newly.push(i as u32);
            }
        }
        if !newly.is_empty() {
            self.hidden.extend(newly);
            self.hidden.sort_unstable();
        }
        self.added.retain(|value| !hide(value));
    }

    /// Does this list have a layer of its own over its base?
    pub fn is_layered(&self) -> bool {
        !self.hidden.is_empty() || !self.added.is_empty()
    }

    /// Do the two share one base (a clone and its source)?
    pub fn shares_base_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.base, &other.base)
    }
}

impl<T: Clone> Layered<T> {
    /// The list as a `Vec` of its own: the layer folded into a copy of the
    /// base (no copy when the base is not shared and there is no layer).
    pub fn to_mut(&mut self) -> &mut Vec<T> {
        if self.is_layered() {
            let folded: Vec<T> = self.iter().cloned().collect();
            *self = Self::from(folded);
        }
        let base = Arc::make_mut(&mut self.base);
        // What is written may move: the index is made again when asked.
        base.grid = OnceLock::new();
        &mut base.items
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }

    pub fn into_vec(self) -> Vec<T> {
        if !self.is_layered() {
            return Arc::try_unwrap(self.base).map_or_else(|shared| shared.items.clone(), |base| base.items);
        }
        self.to_vec()
    }

    pub fn push(&mut self, value: T) {
        self.to_mut().push(value);
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.to_mut().extend(iter);
    }

    pub fn retain(&mut self, keep: impl FnMut(&T) -> bool) {
        self.to_mut().retain(keep);
    }

    pub fn sort_by(&mut self, compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        self.to_mut().sort_by(compare);
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn insert(&mut self, at: usize, value: T) {
        self.to_mut().insert(at, value);
    }

    pub fn remove(&mut self, at: usize) -> T {
        self.to_mut().remove(at)
    }

    pub fn truncate(&mut self, len: usize) {
        self.to_mut().truncate(len);
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.to_mut().get_mut(i)
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.to_mut().last_mut()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.to_mut().iter_mut()
    }
}

/// The shared list, and an index of where its entries are, made the first
/// time something asks what is near a place.
struct Base<T> {
    items: Vec<T>,
    grid: OnceLock<Grid>,
}

impl<T> Base<T> {
    fn new(items: Vec<T>) -> Self {
        Self { items, grid: OnceLock::new() }
    }
}

impl<T: Clone> Clone for Base<T> {
    fn clone(&self) -> Self {
        Self::new(self.items.clone())
    }
}

impl<T> std::ops::Deref for Base<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Vec<T> {
        &self.items
    }
}

/// An entry that is somewhere: a block's box, a line's bounds.
pub trait Spatial {
    fn spatial_bounds(&self) -> Aabb;
}

/// Where the entries of a list are: each grid cell, the entries whose bounds
/// reach it, in list order.
struct Grid {
    cell: f32,
    cells: std::collections::HashMap<(i32, i32), Vec<u32>>,
    /// Entries too big to file by cell (more than this many cells): asked
    /// every time.
    large: Vec<u32>,
}

const GRID_CELL: f32 = 128.0;
const GRID_LARGE_CELLS: i64 = 256;

impl Grid {
    fn cells_of(&self, b: Aabb) -> (i32, i32, i32, i32) {
        let lo = (b.min / self.cell).floor();
        let hi = (b.max / self.cell).floor();
        (lo.x as i32, lo.y as i32, hi.x as i32, hi.y as i32)
    }

    fn build<T: Spatial>(items: &[T]) -> Self {
        let mut grid = Self { cell: GRID_CELL, cells: Default::default(), large: Vec::new() };
        for (i, item) in items.iter().enumerate() {
            let (x0, y0, x1, y1) = grid.cells_of(item.spatial_bounds());
            if i64::from(x1 - x0 + 1) * i64::from(y1 - y0 + 1) > GRID_LARGE_CELLS {
                grid.large.push(i as u32);
                continue;
            }
            for x in x0..=x1 {
                for y in y0..=y1 {
                    grid.cells.entry((x, y)).or_default().push(i as u32);
                }
            }
        }
        grid
    }

    /// The entries whose cells meet `region`, each once, in list order.
    fn near(&self, region: Aabb) -> Vec<u32> {
        let (x0, y0, x1, y1) = self.cells_of(region);
        let mut found: Vec<u32> = self.large.clone();
        if i64::from(x1 - x0 + 1) * i64::from(y1 - y0 + 1) > GRID_LARGE_CELLS * 16 {
            // A region the size of the room: every entry.
            // AMBITION_REVIEW(determinism): the hash order does not get out;
            // `found` is sorted and made unique below.
            found.extend(self.cells.values().flatten().copied());
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(cell) = self.cells.get(&(x, y)) {
                        found.extend_from_slice(cell);
                    }
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }
}

impl<T: Spatial> Layered<T> {
    /// The base entries near `region`, not hidden, by base index.
    fn base_near(&self, region: Aabb) -> Vec<u32> {
        let grid = self.base.grid.get_or_init(|| Grid::build(&self.base.items));
        let mut near = grid.near(region);
        near.retain(|i| self.hidden.binary_search(i).is_err() && touches(self.base.items[*i as usize].spatial_bounds(), region));
        near
    }

    /// The entries whose bounds overlap `region`, in list order. A grid over
    /// the shared base (made once for it) finds them, so it costs what is
    /// near, not what the list holds.
    pub fn near(&self, region: Aabb) -> Vec<&T> {
        let mut near: Vec<&T> = self.base_near(region).into_iter().map(|i| &self.base.items[i as usize]).collect();
        near.extend(self.added.iter().filter(|value| touches(value.spatial_bounds(), region)));
        near
    }

    /// Hide the entries near `region` that `hide` picks.
    pub fn hide_near(&mut self, region: Aabb, mut hide: impl FnMut(&T) -> bool) {
        let picked: Vec<u32> =
            self.base_near(region).into_iter().filter(|i| hide(&self.base.items[*i as usize])).collect();
        if !picked.is_empty() {
            self.hidden.extend(picked);
            self.hidden.sort_unstable();
        }
        self.added.retain(|value| !(touches(value.spatial_bounds(), region) && hide(value)));
    }
}

/// Do the two boxes share a point (a flat line's box has no area)?
fn touches(a: Aabb, b: Aabb) -> bool {
    a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
}

/// The box around `points`.
pub fn bounds_of(points: &[Vec2]) -> Aabb {
    let (lo, hi) = points
        .iter()
        .fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    Aabb::new((lo + hi) * 0.5, ((hi - lo) * 0.5).max(Vec2::ZERO))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Base(usize),
    Added(usize),
}

/// The entries a reader sees, in order.
pub struct LayeredIter<'a, T> {
    base: std::iter::Enumerate<std::slice::Iter<'a, T>>,
    hidden: &'a [u32],
    added: std::slice::Iter<'a, T>,
    left: usize,
}

impl<'a, T> Iterator for LayeredIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        for (i, value) in self.base.by_ref() {
            if let Some((first, rest)) = self.hidden.split_first() {
                if *first as usize == i {
                    self.hidden = rest;
                    continue;
                }
            }
            self.left -= 1;
            return Some(value);
        }
        let value = self.added.next()?;
        self.left -= 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<T> ExactSizeIterator for LayeredIter<'_, T> {}

impl<'a, T> DoubleEndedIterator for LayeredIter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        if let Some(value) = self.added.next_back() {
            self.left -= 1;
            return Some(value);
        }
        while let Some((i, value)) = self.base.next_back() {
            if let Some((last, rest)) = self.hidden.split_last() {
                if *last as usize == i {
                    self.hidden = rest;
                    continue;
                }
            }
            self.left -= 1;
            return Some(value);
        }
        None
    }
}

impl<'a, T> IntoIterator for &'a Layered<T> {
    type Item = &'a T;
    type IntoIter = LayeredIter<'a, T>;

    fn into_iter(self) -> LayeredIter<'a, T> {
        self.iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut Layered<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> std::slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T: Clone> IntoIterator for Layered<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> std::vec::IntoIter<T> {
        self.into_vec().into_iter()
    }
}

impl<T> std::ops::Index<usize> for Layered<T> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        self.get(i).unwrap_or_else(|| panic!("index {i} out of a list of {}", self.len()))
    }
}

impl<T: Clone> std::ops::IndexMut<usize> for Layered<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.to_mut()[i]
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Layered<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: PartialEq> PartialEq for Layered<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: serde::Serialize> serde::Serialize for Layered<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Layered<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<T>::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layer_hides_and_adds_without_touching_the_shared_base() {
        let room: Layered<u32> = (0..6).collect();
        let mut body = room.clone();
        assert!(body.shares_base_with(&room));
        body.hide(1);
        // Where a reader sees it: 3 is the third now.
        body.hide(2);
        body.add(10);
        body.add(11);
        assert_eq!(body.iter().copied().collect::<Vec<_>>(), vec![0, 2, 4, 5, 10, 11]);
        assert_eq!(body.iter().rev().copied().collect::<Vec<_>>(), vec![11, 10, 5, 4, 2, 0]);
        assert_eq!(body.len(), 6);
        assert_eq!((0..body.len()).map(|i| body[i]).collect::<Vec<_>>(), vec![0, 2, 4, 5, 10, 11]);
        assert_eq!(body.get(6), None);
        // Hiding an entry by where a reader sees it.
        body.hide(1);
        assert_eq!(body.to_vec(), vec![0, 4, 5, 10, 11]);
        body.hide(4);
        assert_eq!(body.to_vec(), vec![0, 4, 5, 10]);
        assert!(body.shares_base_with(&room), "the layer copied the base");
        assert_eq!(room.to_vec(), vec![0, 1, 2, 3, 4, 5]);
        // What is near a place, by the shared index.
        #[derive(Clone, Debug, PartialEq)]
        struct At(f32);
        impl Spatial for At {
            fn spatial_bounds(&self) -> Aabb {
                Aabb::new(crate::Vec2::new(self.0, 0.0), crate::Vec2::splat(1.0))
            }
        }
        let row: Layered<At> = (0..2000).map(|i| At(i as f32 * 10.0)).collect();
        let mut seen = row.clone();
        seen.add(At(505.0));
        let near = |list: &Layered<At>| list.near(Aabb::new(crate::Vec2::new(505.0, 0.0), crate::Vec2::splat(10.0))).into_iter().map(|a| a.0).collect::<Vec<_>>();
        assert_eq!(near(&seen), vec![500.0, 510.0, 505.0]);
        seen.hide_near(Aabb::new(crate::Vec2::new(505.0, 0.0), crate::Vec2::splat(10.0)), |a| a.0 != 510.0);
        assert_eq!(near(&seen), vec![510.0]);
        assert_eq!(seen.len(), 1999);
        assert_eq!(near(&row), vec![500.0, 510.0]);
        // A write folds the layer into a base of its own.
        body.push(12);
        assert_eq!(body.to_vec(), vec![0, 4, 5, 10, 12]);
        assert!(!body.shares_base_with(&room));
        assert_eq!(room.to_vec(), vec![0, 1, 2, 3, 4, 5]);
    }
}
