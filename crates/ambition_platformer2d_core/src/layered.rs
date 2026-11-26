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

use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::{Aabb, Vec2};

/// An unknown envelope retains every candidate for exact contact.
pub(crate) const UNBOUNDED: Aabb = Aabb { min: Vec2::NEG_INFINITY, max: Vec2::INFINITY };

/// A shared base list, a set of its entries hidden, and entries added.
#[derive(Clone)]
pub struct Layered<T> {
    base: Arc<Base<T>>,
    layer: Option<Arc<Layer<T>>>,
    reads: Option<Arc<Mutex<LayerReads>>>,
}

struct Layer<T> {
    /// Indices into `base`, sorted, each once.
    hidden: Vec<u32>,
    added: Vec<T>,
    bounds: [OnceLock<Vec<Aabb>>; 2],
    has_motion: OnceLock<bool>,
}

impl<T: Clone> Clone for Layer<T> {
    fn clone(&self) -> Self {
        Self {
            hidden: self.hidden.clone(), added: self.added.clone(),
            bounds: Default::default(), has_motion: OnceLock::new(),
        }
    }
}

#[derive(Default, Clone, Copy)]
struct LayerReads {
    area: Option<Aabb>,
    read_count: bool,
    read_all: bool,
}

impl<T> Default for Layered<T> {
    fn default() -> Self {
        Self::from(Vec::new())
    }
}

impl<T> From<Vec<T>> for Layered<T> {
    fn from(base: Vec<T>) -> Self {
        Self { base: Arc::new(Base::new(base)), layer: None, reads: None }
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
        self.note_read(|reads| reads.read_count = true);
        self.base.len() - self.hidden().len() + self.added().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> LayeredIter<'_, T> {
        self.note_read(|reads| reads.read_all = true);
        LayeredIter { base: self.base.iter().enumerate(), hidden: self.hidden(), added: self.added().iter(), left: self.len() }
    }

    fn note_read(&self, note: impl FnOnce(&mut LayerReads)) {
        if let Some(reads) = &self.reads {
            note(&mut reads.lock().expect("terrain read scope"));
        }
    }

    /// Record the terrain reads of the private copy used to prove a step.
    pub(crate) fn record_reads(&mut self) {
        self.reads = Some(Arc::default());
    }

    fn hidden(&self) -> &[u32] {
        self.layer.as_deref().map_or(&[], |layer| layer.hidden.as_slice())
    }

    fn added(&self) -> &[T] {
        self.layer.as_deref().map_or(&[], |layer| layer.added.as_slice())
    }

    /// Where the `i`th entry a reader sees is: in the base, or added.
    fn locate(&self, i: usize) -> Option<Slot> {
        let hidden = self.hidden();
        if hidden.is_empty() {
            return if i < self.base.len() {
                Some(Slot::Base(i))
            } else {
                (i - self.base.len() < self.added().len()).then(|| Slot::Added(i - self.base.len()))
            };
        }
        let shown = self.base.len() - hidden.len();
        if i >= shown {
            return (i - shown < self.added().len()).then(|| Slot::Added(i - shown));
        }
        // The base index with `i` shown entries before it: step past each
        // hidden index at or before the candidate.
        let mut at = i;
        for hidden in hidden {
            if (*hidden as usize) <= at {
                at += 1;
            } else {
                break;
            }
        }
        Some(Slot::Base(at))
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        self.note_read(|reads| reads.read_all = true);
        match self.locate(i)? {
            Slot::Base(at) => self.base.get(at),
            Slot::Added(at) => self.added().get(at),
        }
    }

    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    pub fn last(&self) -> Option<&T> {
        self.len().checked_sub(1).and_then(|i| self.get(i))
    }

    /// Does this list have a layer of its own over its base?
    pub fn is_layered(&self) -> bool {
        !self.hidden().is_empty() || !self.added().is_empty()
    }

    /// Do the two share one base (a clone and its source)?
    pub fn shares_base_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.base, &other.base)
    }

}

/// Identity of a published list. A write changes the identity of the part it changes.
pub(crate) struct LayeredIdentity<T> {
    base: Weak<Base<T>>,
    layer: Option<Weak<Layer<T>>>,
    outside_layer: Option<OutsideLayer>,
}

/// The step's reads found no added terrain in this area. Hidden base entries
/// and observed list counts must still match.
struct OutsideLayer {
    area: Option<Aabb>,
    hidden: Vec<u32>,
    count: Option<usize>,
}

impl<T: Spatial> LayeredIdentity<T> {
    pub(crate) fn matches(&self, list: &Layered<T>) -> bool {
        if !std::ptr::eq(self.base.as_ptr(), Arc::as_ptr(&list.base)) {
            return false;
        }
        self.layer.as_ref().map(Weak::as_ptr) == list.layer.as_ref().map(Arc::as_ptr)
            || self.outside_layer.as_ref().is_some_and(|scope| {
                scope.hidden == list.hidden() && scope.count.is_none_or(|count| count == list.len())
                    && !list.added_touches(scope.area)
            })
    }
}

impl<T: Clone> Layered<T> {
    fn layer_mut(&mut self) -> &mut Layer<T> {
        let layer = Arc::make_mut(self.layer.get_or_insert_with(|| Arc::new(Layer {
            hidden: Vec::new(), added: Vec::new(), bounds: Default::default(),
            has_motion: OnceLock::new(),
        })));
        layer.bounds = Default::default();
        layer.has_motion = OnceLock::new();
        layer
    }

    /// Hide the `i`th entry a reader sees, from this list only: the base is
    /// shared and stays as it is. Added entries are removed.
    pub fn hide(&mut self, i: usize) {
        match self.locate(i) {
            Some(Slot::Base(at)) => {
                let hidden = &mut self.layer_mut().hidden;
                if let Err(pos) = hidden.binary_search(&(at as u32)) {
                    hidden.insert(pos, at as u32);
                }
            }
            Some(Slot::Added(at)) => {
                self.layer_mut().added.remove(at);
            }
            None => {}
        }
    }

    /// Add an entry after every other, to this list only.
    pub fn add(&mut self, value: T) {
        self.layer_mut().added.push(value);
    }

    /// Add entries after every other, to this list only.
    pub fn add_all(&mut self, values: impl IntoIterator<Item = T>) {
        let mut values = values.into_iter().peekable();
        if values.peek().is_some() {
            self.layer_mut().added.extend(values);
        }
    }

    /// Hide every entry `hide` picks, from this list only.
    pub fn hide_where(&mut self, mut hide: impl FnMut(&T) -> bool) {
        let mut hidden = self.hidden().iter().peekable();
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
            let hidden = &mut self.layer_mut().hidden;
            hidden.extend(newly);
            hidden.sort_unstable();
        }
        if !self.added().is_empty() {
            self.layer_mut().added.retain(|value| !hide(value));
        }
    }

    /// The list as a `Vec` of its own: the layer folded into a copy of the
    /// base (no copy when the base is not shared and there is no layer).
    pub fn to_mut(&mut self) -> &mut Vec<T> {
        if self.is_layered() {
            let folded: Vec<T> = self.iter().cloned().collect();
            *self = Self::from(folded);
        }
        let base = Arc::make_mut(&mut self.base);
        // What is written may move: the index is made again when asked.
        base.grids = [OnceLock::new(),OnceLock::new()];
        base.has_motion = OnceLock::new();
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
    grids: [OnceLock<BoundIndex>; 2],
    has_motion: OnceLock<bool>,
}

struct BoundIndex {
    bounds: Vec<Aabb>,
    grid: RangeGrid,
}

impl<T> Base<T> {
    fn new(items: Vec<T>) -> Self {
        Self { items, grids:[OnceLock::new(),OnceLock::new()], has_motion: OnceLock::new() }
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

use ambition_geometry::spatial::{touches, RangeGrid, Spatial};

/// Geometry with a published path in the current step.
pub(crate) trait SpatialMotion: Spatial {
    fn moves(&self) -> bool;
}

impl<T: Spatial> Layered<T> {
    pub(crate) fn moving_near(&self, region: Aabb, eligible: impl Fn(&T) -> bool) -> bool
    where
        T: SpatialMotion,
    {
        let base_moves = *self.base.has_motion.get_or_init(|| self.base.iter().any(T::moves));
        let added_moves = self.layer.as_ref().is_some_and(|layer| {
            *layer.has_motion.get_or_init(|| layer.added.iter().any(T::moves))
        });
        // A new moving entry in this region can invalidate a settled step.
        if !base_moves {
            self.note_region_read(region);
        }
        (base_moves && self.base_near(region, true)
            .map(|i| &self.base.items[i as usize]).any(|item| item.moves() && eligible(item)))
            || (added_moves && self.additions_near(region, true)
                .any(|(_, item)| item.moves() && eligible(item)))
    }

    pub(crate) fn identity(&self) -> LayeredIdentity<T> {
        let outside_layer = self.reads.as_ref().and_then(|observed| {
            let reads = *observed.lock().expect("terrain read scope");
            (!reads.read_all && !self.added_touches(reads.area)).then(|| OutsideLayer {
                area: reads.area,
                hidden: self.hidden().to_vec(),
                count: reads.read_count.then(|| self.base.len() - self.hidden().len() + self.added().len()),
            })
        });
        LayeredIdentity {
            base: Arc::downgrade(&self.base),
            layer: self.layer.as_ref().map(Arc::downgrade),
            outside_layer,
        }
    }

    fn added_bounds(&self, swept: bool) -> &[Aabb] {
        self.layer.as_ref().map_or(&[], |layer| {
            layer.bounds[usize::from(swept)].get_or_init(|| layer.added.iter()
                .map(|item| if swept { item.swept_bounds() } else { item.spatial_bounds() }).collect())
        })
    }

    fn added_touches(&self, area: Option<Aabb>) -> bool {
        area.is_some_and(|area| self.added_bounds(true).iter().any(|bounds| touches(*bounds, area)))
    }

    fn note_region_read(&self, region: Aabb) {
        self.note_read(|reads| {
            if !region.min.is_finite() || !region.max.is_finite() || !region.min.cmple(region.max).all() {
                reads.read_all = true;
                return;
            }
            reads.area = Some(reads.area.map_or(region, |prior| Aabb {
                min: prior.min.min(region.min), max: prior.max.max(region.max),
            }));
        });
    }

    /// Visible base entries with bounds that touch `region`.
    fn base_near(&self, region: Aabb, swept: bool) -> impl Iterator<Item = u32> + '_ {
        self.note_region_read(region);
        let cache = &self.base.grids[usize::from(swept)];
        let index = cache.get_or_init(|| {
            let bounds: Vec<_> = self.base.items.iter()
                .map(|item| if swept { item.swept_bounds() } else { item.spatial_bounds() })
                .collect();
            let grid = RangeGrid::build(bounds.iter().copied());
            BoundIndex { bounds, grid }
        });
        let hidden = self.hidden();
        index.grid.near(region)
            .filter(move |i| hidden.binary_search(i).is_err() && touches(index.bounds[*i as usize], region))
    }

    fn additions_near(&self, region: Aabb, swept: bool) -> impl Iterator<Item = (usize, &T)> + '_ {
        self.added().iter().zip(self.added_bounds(swept)).enumerate()
            .filter_map(move |(i, (value, bounds))| touches(*bounds, region).then_some((i, value)))
    }

    fn near_items(&self, region: Aabb, swept: bool) -> impl Iterator<Item = &T> + '_ {
        self.base_near(region, swept).map(|i| &self.base.items[i as usize])
            .chain(self.additions_near(region, swept).map(|(_, value)| value))
    }

    /// The entries whose bounds overlap `region`, in list order. A grid over
    /// the shared base (made once for it) finds them, so it costs what is
    /// near, not what the list holds.
    pub fn near(&self, region: Aabb) -> Vec<&T> {
        self.near_items(region, false).collect()
    }

    pub(crate) fn in_region(&self, region: Option<Aabb>) -> impl Iterator<Item = &T> {
        let nearby = region.map(|area| self.near_items(area, false));
        let all = region.is_none().then(|| self.iter());
        nearby.into_iter().flatten().chain(all.into_iter().flatten())
    }

    /// Entries whose bounds meet the region during their published motion.
    pub fn swept_near(&self, region: Aabb) -> impl Iterator<Item = &T> + '_ {
        self.near_items(region, true)
    }

    /// Keep the logical index used by contacts and surface routes.
    pub fn indexed_near(&self, region: Aabb) -> Vec<(usize,&T)> {
        self.indexed_candidates(region, false).collect()
    }

    /// Keep contact indices while testing the full published motion.
    pub fn indexed_swept_near(&self, region: Aabb) -> impl Iterator<Item = (usize, &T)> + '_ {
        self.indexed_candidates(region, true)
    }

    fn indexed_candidates(&self, region: Aabb, swept: bool) -> impl Iterator<Item = (usize, &T)> + '_ {
        let hidden = self.hidden();
        let near = self.base_near(region, swept).map(move |i| {
            let skipped = hidden.partition_point(|hidden| *hidden < i);
            (i as usize-skipped,&self.base.items[i as usize])
        });
        let base_len = self.base.items.len()-hidden.len();
        near.chain(self.additions_near(region, swept).map(move |(i, value)| (base_len + i, value)))
    }

    /// Hide the entries near `region` that `hide` picks.
    pub fn hide_near(&mut self, region: Aabb, mut hide: impl FnMut(&T) -> bool) where T: Clone {
        let picked: Vec<u32> =
            self.base_near(region,false).filter(|i| hide(&self.base.items[*i as usize])).collect();
        if !picked.is_empty() {
            let hidden = &mut self.layer_mut().hidden;
            hidden.extend(picked);
            hidden.sort_unstable();
        }
        if !self.added().is_empty() {
            self.layer_mut().added.retain(|value| !(touches(value.spatial_bounds(), region) && hide(value)));
        }
    }
}

/// The box around `points`.
pub fn bounds_of(points: &[Vec2]) -> Aabb {
    if points.is_empty() { return Aabb::new(Vec2::ZERO, Vec2::ZERO); }
    if points.iter().any(|point| !point.is_finite()) { return UNBOUNDED; }
    let (lo, hi) = points
        .iter()
        .fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    Aabb { min: lo, max: hi }
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
    fn spatial_queries_match_visible_list_order_for_local_and_global_regions() {
        #[derive(Clone)]
        struct At { id: usize, bounds: Aabb, swept: Aabb }
        impl Spatial for At {
            fn spatial_bounds(&self) -> Aabb { self.bounds }
            fn swept_bounds(&self) -> Aabb { self.swept }
        }
        let mut entries: Layered<At> = (0..1000).map(|id| {
            let min = Vec2::new((id % 50) as f32 * 64.0 - 1600.0, (id / 50) as f32 * 64.0 - 640.0);
            let bounds = Aabb { min, max: min + Vec2::splat(128.0) };
            At { id, bounds, swept: Aabb { min: min - Vec2::splat(256.0), max: bounds.max } }
        }).chain(std::iter::once(At { id: 1000, bounds: UNBOUNDED, swept: UNBOUNDED })).collect();
        // Build both shared grids before a layer hides and adds entries.
        entries.near(UNBOUNDED);
        assert_eq!(entries.swept_near(UNBOUNDED).count(), 1001);
        for index in [999, 500, 0] { entries.hide(index); }
        entries.add(At { id: 1001, bounds: UNBOUNDED, swept: UNBOUNDED });
        entries.add(At { id: 1002, bounds: Aabb { min: Vec2::ZERO, max: Vec2::ZERO }, swept: UNBOUNDED });
        let mut regions = vec![UNBOUNDED, Aabb { min: Vec2::splat(-100000.0), max: Vec2::splat(100000.0) }];
        for i in -20..20 {
            let min = Vec2::new(i as f32 * 128.0, i as f32 * 32.0);
            regions.push(Aabb { min, max: min });
            regions.push(Aabb { min, max: min + Vec2::splat(300.0) });
        }
        for region in regions {
            for swept in [false, true] {
                let expected: Vec<_> = entries.iter().enumerate()
                    .filter(|(_, entry)| touches(if swept { entry.swept } else { entry.bounds }, region))
                    .map(|(index, entry)| (index, entry.id)).collect();
                let actual: Vec<_> = entries.indexed_candidates(region, swept)
                    .map(|(index, entry)| (index, entry.id)).collect();
                assert_eq!(actual, expected, "region={region:?}, swept={swept}");
            }
        }
    }

    #[test]
    fn a_thin_line_keeps_its_far_endpoint_in_the_index() {
        let end = Vec2::new(16_777_218.0, 0.0);
        let lines: Layered<crate::SurfaceChain> = vec![crate::SurfaceChain::open(
            "thin line", vec![Vec2::new(16_777_216.0, 0.0), end],
        )].into();
        let mut found = lines.indexed_swept_near(Aabb { min: end, max: end });
        assert_eq!(found.next().map(|(index, _)| index), Some(0));
        assert!(found.next().is_none());
    }

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
        let mut moved = row.clone();
        moved.get_mut(50).unwrap().0 = 9000.0;
        assert_eq!(near(&moved), vec![510.0]);
        assert_eq!(near(&row), vec![500.0, 510.0]);
        // A write folds the layer into a base of its own.
        body.push(12);
        assert_eq!(body.to_vec(), vec![0, 4, 5, 10, 12]);
        assert!(!body.shares_base_with(&room));
        assert_eq!(room.to_vec(), vec![0, 1, 2, 3, 4, 5]);
    }
}
