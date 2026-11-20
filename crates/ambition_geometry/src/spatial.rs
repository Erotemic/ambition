//! Reusable bounds and exact spatial queries over a rebuilt snapshot.

use crate::{Aabb, Vec2};

/// Bounds supplied by a spatial object.
pub trait Spatial {
    fn spatial_bounds(&self) -> Aabb;
    fn swept_bounds(&self) -> Aabb {
        self.spatial_bounds()
    }
}

impl<T: Spatial + ?Sized> Spatial for &T {
    fn spatial_bounds(&self) -> Aabb {
        (**self).spatial_bounds()
    }
    fn swept_bounds(&self) -> Aabb {
        (**self).swept_bounds()
    }
}

impl Spatial for Aabb {
    fn spatial_bounds(&self) -> Aabb {
        *self
    }
}

/// A query can reject an entire subtree through its application-owned summary.
/// `may_match` must return true if any item in that subtree can be accepted.
pub trait SpatialFilter<T, S> {
    fn accepts(&mut self, item: &T) -> bool;
    fn may_match(&mut self, _summary: &S) -> bool {
        true
    }
}

/// The input row whose bounds are not finite or have reversed extents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidSpatialBounds {
    pub index: usize,
}

impl std::fmt::Display for InvalidSpatialBounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "spatial item {} has nonfinite or reversed bounds",
            self.index
        )
    }
}
impl std::error::Error for InvalidSpatialBounds {}

struct Entry<T> {
    item: T,
    bounds: Aabb,
    point: bool,
}
struct Node<S> {
    range: std::ops::Range<usize>,
    children: [Option<usize>; 2],
    bounds: Aabb,
    summary: S,
}

/// A balanced bounds tree for a snapshot of moving or fixed objects.
/// Rebuilds reuse storage. Nearest ties use input order, independent of tree
/// traversal. Supply items in semantic identity order when ties affect state.
pub struct SpatialTree<T, S = ()> {
    entries: Vec<Entry<T>>,
    nodes: Vec<Node<S>>,
    order: Vec<usize>,
    root: Option<usize>,
}

impl<T, S> Default for SpatialTree<T, S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            nodes: Vec::new(),
            order: Vec::new(),
            root: None,
        }
    }
}

impl<T: Spatial> SpatialTree<T> {
    /// Rebuild a tree without application summaries.
    pub fn rebuild(
        &mut self,
        items: impl IntoIterator<Item = T>,
    ) -> Result<(), InvalidSpatialBounds> {
        self.rebuild_with_summary(items, |_| (), |_, _| {})
    }

    /// Find the nearest accepted bounds, with squared Euclidean distance.
    pub fn nearest(&self, point: Vec2, accepts: impl FnMut(&T) -> bool) -> Option<(&T, f32)> {
        struct Filter<F>(F);
        impl<T, F: FnMut(&T) -> bool> SpatialFilter<T, ()> for Filter<F> {
            fn accepts(&mut self, item: &T) -> bool {
                (self.0)(item)
            }
        }
        self.nearest_filtered(point, &mut Filter(accepts))
    }
}

impl<T: Spatial, S> SpatialTree<T, S> {
    /// Rebuild all bounds and summaries from current input. A failed rebuild
    /// leaves the tree empty. `merge` must include both summaries in its result.
    pub fn rebuild_with_summary(
        &mut self,
        items: impl IntoIterator<Item = T>,
        summarize: impl Fn(&T) -> S,
        merge: impl Fn(&S, &S) -> S,
    ) -> Result<(), InvalidSpatialBounds> {
        self.root = None;
        self.entries.clear();
        self.nodes.clear();
        self.order.clear();
        for (index, item) in items.into_iter().enumerate() {
            let bounds = item.spatial_bounds();
            if !valid(bounds) {
                self.entries.clear();
                return Err(InvalidSpatialBounds { index });
            }
            self.entries.push(Entry {
                item,
                bounds,
                point: bounds.min == bounds.max,
            });
            self.order.push(index);
        }
        self.root = build(
            &self.entries,
            &mut self.order,
            &mut self.nodes,
            0,
            0,
            &summarize,
            &merge,
        );
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find the nearest accepted bounds. Nonfinite query points have no result.
    /// Summary rejection must be conservative. Equal distances use input order.
    pub fn nearest_filtered(
        &self,
        point: Vec2,
        filter: &mut impl SpatialFilter<T, S>,
    ) -> Option<(&T, f32)> {
        if !point.is_finite() {
            return None;
        }
        let mut best = None;
        if let Some(root) = self.root {
            self.visit_nearest(root, point, filter, &mut best);
        }
        best.map(|(index, distance)| (&self.entries[index].item, distance))
    }

    fn visit_nearest(
        &self,
        index: usize,
        point: Vec2,
        filter: &mut impl SpatialFilter<T, S>,
        best: &mut Option<(usize, f32)>,
    ) {
        let node = &self.nodes[index];
        if best.is_some_and(|(_, distance)| distance_to_bounds(point, node.bounds) > distance)
            || !filter.may_match(&node.summary)
        {
            return;
        }
        for &rank in &self.order[node.range.clone()] {
            let entry = &self.entries[rank];
            let distance = if entry.point {
                squared_distance(entry.bounds.min, point)
            } else {
                distance_to_bounds(point, entry.bounds)
            };
            if best.is_none_or(|(before, d)| distance < d || (distance == d && rank < before))
                && filter.accepts(&entry.item)
            {
                *best = Some((rank, distance));
            }
        }
        let mut children = node.children;
        if let [Some(a), Some(b)] = children {
            if distance_to_bounds(point, self.nodes[b].bounds)
                < distance_to_bounds(point, self.nodes[a].bounds)
            {
                children.swap(0, 1);
            }
        }
        for child in children.into_iter().flatten() {
            self.visit_nearest(child, point, filter, best);
        }
    }

    /// Visit all bounds that touch or overlap the query. Visits use tree order;
    /// a consumer that needs another order must order its own results.
    pub fn query_bounds(&self, bounds: Aabb, mut visit: impl FnMut(&T)) {
        if !valid(bounds) {
            return;
        }
        if let Some(root) = self.root {
            self.visit_bounds(root, bounds, &mut visit);
        }
    }

    fn visit_bounds(&self, index: usize, bounds: Aabb, visit: &mut impl FnMut(&T)) {
        let node = &self.nodes[index];
        if !overlaps(node.bounds, bounds) {
            return;
        }
        for &rank in &self.order[node.range.clone()] {
            let entry = &self.entries[rank];
            if overlaps(entry.bounds, bounds) {
                visit(&entry.item);
            }
        }
        for child in node.children.into_iter().flatten() {
            self.visit_bounds(child, bounds, visit);
        }
    }
}

fn valid(bounds: Aabb) -> bool {
    bounds.min.is_finite() && bounds.max.is_finite() && bounds.min.cmple(bounds.max).all()
}
fn overlaps(a: Aabb, b: Aabb) -> bool {
    a.min.cmple(b.max).all() && b.min.cmple(a.max).all()
}
fn distance_to_bounds(point: Vec2, bounds: Aabb) -> f32 {
    let closest = point.clamp(bounds.min, bounds.max);
    squared_distance(closest, point)
}
fn squared_distance(a: Vec2, b: Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn build<T, S>(
    entries: &[Entry<T>],
    order: &mut [usize],
    nodes: &mut Vec<Node<S>>,
    axis: usize,
    offset: usize,
    summarize: &impl Fn(&T) -> S,
    merge: &impl Fn(&S, &S) -> S,
) -> Option<usize> {
    if order.is_empty() {
        return None;
    }
    let (range, children, bounds, summary) = if order.len() <= 32 {
        let mut bounds = entries[order[0]].bounds;
        let mut summary = summarize(&entries[order[0]].item);
        for &rank in &order[1..] {
            bounds.min = bounds.min.min(entries[rank].bounds.min);
            bounds.max = bounds.max.max(entries[rank].bounds.max);
            summary = merge(&summary, &summarize(&entries[rank].item));
        }
        (offset..offset + order.len(), [None, None], bounds, summary)
    } else {
        let middle = order.len() / 2;
        let center = |index: usize| {
            entries[index].bounds.min[axis] * 0.5 + entries[index].bounds.max[axis] * 0.5
        };
        order.select_nth_unstable_by(middle, |&a, &b| {
            center(a).total_cmp(&center(b)).then(a.cmp(&b))
        });
        let (left, right) = order.split_at_mut(middle);
        let left = build(entries, left, nodes, 1 - axis, offset, summarize, merge).unwrap();
        let right = build(
            entries,
            right,
            nodes,
            1 - axis,
            offset + middle,
            summarize,
            merge,
        )
        .unwrap();
        let bounds = Aabb {
            min: nodes[left].bounds.min.min(nodes[right].bounds.min),
            max: nodes[left].bounds.max.max(nodes[right].bounds.max),
        };
        (
            0..0,
            [Some(left), Some(right)],
            bounds,
            merge(&nodes[left].summary, &nodes[right].summary),
        )
    };
    let index = nodes.len();
    nodes.push(Node {
        range,
        children,
        bounds,
        summary,
    });
    Some(index)
}

#[cfg(test)]
mod tests;
