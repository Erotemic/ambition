//! How many rows of a frozen relation carry a key.
//!
//! THE LAW: rows are counted, not distinct values. One row is
//! [`Cardinality::Unique`]; a second row with the same key is
//! [`Cardinality::Ambiguous`], even when it repeats the first value exactly.
//! A key with no row is [`Cardinality::Missing`]. A value that is itself
//! optional keeps `Unique(None)` apart from `Missing`: one row that says
//! "nothing yet" is not the absence of a row.
//!
//! This is not the [`crate::Resolver`] law (repeated aliases of one
//! declaration are allowed there) nor registry classification (an equal
//! registration is idempotent there). Domain errors, visibility and lifetime
//! stay with each owner; this module only counts.
//!
//! The relation is frozen. After two rows collide, removing one cannot tell
//! which value remains, so an owner rebuilds at its own cut rather than
//! editing in place.

use std::borrow::Borrow;
use std::collections::BTreeMap;

/// How many rows carry a key: none, exactly one (with its value), or more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cardinality<V> {
    Missing,
    Unique(V),
    Ambiguous,
}

impl<V> Cardinality<V> {
    /// The value of a unique row.
    pub fn unique(self) -> Option<V> {
        match self {
            Self::Unique(value) => Some(value),
            Self::Missing | Self::Ambiguous => None,
        }
    }

    pub fn map<W>(self, f: impl FnOnce(V) -> W) -> Cardinality<W> {
        match self {
            Self::Missing => Cardinality::Missing,
            Self::Unique(value) => Cardinality::Unique(f(value)),
            Self::Ambiguous => Cardinality::Ambiguous,
        }
    }
}

/// The cardinality of `key` among `rows`, by scanning them: the reference
/// that [`FrozenLookup`] must agree with, and the cheapest answer for one
/// question over a small relation.
pub fn scan<K: PartialEq, V>(rows: impl IntoIterator<Item = (K, V)>, key: &K) -> Cardinality<V> {
    let mut found = rows.into_iter().filter(|(row, _)| row == key);
    match (found.next(), found.next()) {
        (None, _) => Cardinality::Missing,
        (Some((_, value)), None) => Cardinality::Unique(value),
        (Some(_), Some(_)) => Cardinality::Ambiguous,
    }
}

/// A frozen relation indexed by key, answering [`Cardinality`] for each
/// lookup by the law above. Lookup never exposes iteration order.
#[derive(Clone, Debug)]
pub struct FrozenLookup<K, V>(BTreeMap<K, Option<V>>);

impl<K, V> Default for FrozenLookup<K, V> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<K: Ord, V> FrozenLookup<K, V> {
    /// Index `rows`. A key's second row marks it ambiguous for good.
    pub fn from_rows(rows: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut index = BTreeMap::new();
        for (key, value) in rows {
            index.entry(key).and_modify(|found: &mut Option<V>| *found = None).or_insert(Some(value));
        }
        Self(index)
    }

    /// How many rows carry `key`.
    pub fn get<Q>(&self, key: &Q) -> Cardinality<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.0.get(key) {
            None => Cardinality::Missing,
            Some(Some(value)) => Cardinality::Unique(value),
            Some(None) => Cardinality::Ambiguous,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The index and the scan agree on every key of a relation with missing,
    /// unique, repeated-identical and conflicting rows, and an optional value
    /// keeps `Unique(None)` apart from `Missing`.
    #[test]
    fn the_index_counts_rows_as_the_scan_does() {
        let rows = [(1, Some('a')), (2, Some('b')), (2, Some('b')), (3, Some('c')), (3, Some('d')), (4, None)];
        let index = FrozenLookup::from_rows(rows);
        for key in 0..6 {
            assert_eq!(index.get(&key).map(|value| *value), scan(rows, &key), "key {key}");
        }
        assert_eq!(index.get(&1), Cardinality::Unique(&Some('a')));
        assert_eq!(index.get(&2), Cardinality::Ambiguous, "a repeated identical row is still a second row");
        assert_eq!(index.get(&3), Cardinality::Ambiguous);
        assert_eq!(index.get(&4), Cardinality::Unique(&None), "one row that says nothing yet");
        assert_eq!(index.get(&0), Cardinality::Missing);
    }
}
