use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use bevy::platform::collections::HashMap;

pub(crate) trait LiveRoomRow {
    fn room(&self) -> Option<LiveRoomInstance>;
}

/// Keep one row per authored ID and live room. Reuse each ID's allocation
/// until the last occurrence goes away.
#[derive(Clone, Debug)]
pub(crate) struct LiveRoomRows<T> {
    rows: HashMap<String, Vec<(T, u64)>>,
    generation: u64,
}

impl<T> Default for LiveRoomRows<T> {
    fn default() -> Self {
        Self { rows: HashMap::default(), generation: 0 }
    }
}

impl<T: LiveRoomRow> LiveRoomRows<T> {
    pub(crate) fn get(&self, id: &str) -> Option<&T> {
        let rows = self.rows.get(id)?;
        let [(row, _)] = rows.as_slice() else {
            return None;
        };
        Some(row)
    }

    pub(crate) fn get_in(&self, id: &str, room: Option<LiveRoomInstance>) -> Option<&T> {
        self.rows.get(id)?.iter().map(|(row, _)| row).find(|row| row.room() == room)
    }

    pub(crate) fn contains(&self, id: &str) -> bool {
        self.rows.contains_key(id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        // AMBITION_REVIEW(determinism): presentation reads these rows.
        self.rows.iter().flat_map(|(id, rows)| rows.iter().map(move |(row, _)| (id.as_str(), row)))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.rows.values().map(Vec::len).sum()
    }

    pub(crate) fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub(crate) fn end_rebuild(&mut self) {
        let generation = self.generation;
        self.rows.retain(|_, rows| {
            rows.retain(|(_, stamp)| *stamp == generation);
            !rows.is_empty()
        });
    }

    pub(crate) fn insert(&mut self, id: &str, row: T) {
        self.refresh(id, row.room(), |_, _| false, || row);
    }

    pub(crate) fn insert_first(&mut self, id: &str, row: T) {
        self.refresh(id, row.room(), |_, written| written, || row);
    }

    /// Update a row in place, or build a new row when the update returns false.
    pub(crate) fn refresh(
        &mut self,
        id: &str,
        room: Option<LiveRoomInstance>,
        update: impl FnOnce(&mut T, bool) -> bool,
        build: impl FnOnce() -> T,
    ) {
        let generation = self.generation;
        if let Some(rows) = self.rows.get_mut(id) {
            if let Some((row, stamp)) = rows.iter_mut().find(|(row, _)| row.room() == room) {
                if !update(row, *stamp == generation) {
                    *row = build();
                }
                *stamp = generation;
            } else {
                rows.push((build(), generation));
            }
        } else {
            self.rows.insert(id.to_owned(), vec![(build(), generation)]);
        }
    }
}
