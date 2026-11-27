//! Hold diagnostic results until their simulation frame is confirmed.

use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) struct PendingFrames<T> {
    session: Option<u64>,
    frames: BTreeMap<i32, T>,
}

impl<T> Default for PendingFrames<T> {
    fn default() -> Self {
        Self {
            session: None,
            frames: BTreeMap::new(),
        }
    }
}

impl<T> PendingFrames<T> {
    /// Clear pending results when the session or confirmation mode changes.
    /// Return true when the caller must reset its diagnostic state.
    pub(super) fn select_session(&mut self, session: Option<u64>) -> bool {
        if session == self.session {
            return false;
        }
        self.session = session;
        self.frames.clear();
        true
    }

    /// Replace the result for a frame, including a correction with no anomaly.
    pub(super) fn replace(&mut self, frame: i32, result: T) {
        self.frames.insert(frame, result);
    }

    /// Remove confirmed results in frame order.
    pub(super) fn take_confirmed(&mut self, through: i32) -> Vec<T> {
        self.frames
            .extract_if(..=through, |_, _| true)
            .map(|(_, value)| value)
            .collect()
    }
}
