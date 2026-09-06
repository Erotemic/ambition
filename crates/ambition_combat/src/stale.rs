//! Move staling — the history that makes a repeated answer worth less.
//!
//! The movement kernel never read a field of it.
//!
//! the behaviour was already opt-in (`DeclaredCombatRules::stale_step`
//! of `0.0` is no staling), and that is exactly why the STATE did not need to
//! be global: a rule being switchable is not a reason for its storage to be
//! everywhere. `ActorMoveset` now `#[require]`s it, so the bodies that carry
//! a history are the bodies that can land a move.

/// hashes, not ids, and that is what makes it rollback state at all. A
/// `Vec<String>` of move names would be a heap allocation per body per tick to
/// save and restore; nine `u32`s are a POD component the snapshot copies like
/// any other. The hash is only ever compared to another hash — nothing reads it
/// back as a name — so a collision costs one move a staleness it did not earn
/// and nothing else.
///
/// it records what LANDED, not what was thrown. A whiffed move is not
/// stale: staling exists to stop one good answer being the only answer, and a
/// move that missed did not answer anything.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyStaleMoves {
    /// Nine is the genre's own queue length.
    pub recent: [u32; 9],
    pub next: u8,
}

impl BodyStaleMoves {
    /// How many of the last nine landings were this same move.
    pub fn occurrences(&self, move_hash: u32) -> u32 {
        if move_hash == 0 {
            return 0;
        }
        self.recent.iter().filter(|h| **h == move_hash).count() as u32
    }

    /// Record one landing.
    pub fn record(&mut self, move_hash: u32) {
        if move_hash == 0 {
            return;
        }
        let slot = (self.next as usize) % self.recent.len();
        self.recent[slot] = move_hash;
        self.next = ((slot + 1) % self.recent.len()) as u8;
    }
}

/// The hash a move id is remembered by. FNV-1a, written out rather than
/// taken from `DefaultHasher`, because a rollback comparison must give the same
/// answer in every process and `RandomState` is seeded per process.
pub fn stale_move_hash(move_id: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in move_id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    // `0` is the empty slot, so a move that hashes to it takes the next value.
    if hash == 0 {
        1
    } else {
        hash
    }
}

#[cfg(test)]
mod stale_move_tests {
    use super::*;

    /// THE QUEUE REMEMBERS NINE AND FORGETS THE TENTH.
    #[test]
    fn the_ring_holds_nine_landings_and_then_rolls() {
        let jab = stale_move_hash("jab");
        let smash = stale_move_hash("fsmash");
        let mut queue = BodyStaleMoves::default();
        assert_eq!(queue.occurrences(jab), 0);

        for _ in 0..9 {
            queue.record(jab);
        }
        assert_eq!(queue.occurrences(jab), 9, "the ring did not fill");

        // nine more of something else pushes every jab out — which is the
        // mechanic: vary your answers and the old one recovers.
        for _ in 0..9 {
            queue.record(smash);
        }
        assert_eq!(queue.occurrences(jab), 0, "a worn move never recovered");
        assert_eq!(queue.occurrences(smash), 9);
    }

    /// THE HASH IS STABLE AND NEVER COLLIDES WITH THE EMPTY SLOT.
    ///
    /// FNV-1a written out rather than `DefaultHasher`, because `RandomState`
    /// is seeded per process and a rollback comparison must give the same answer
    /// in every one of them.
    #[test]
    fn the_move_hash_is_deterministic_and_never_zero() {
        assert_eq!(stale_move_hash("jab"), stale_move_hash("jab"));
        assert_ne!(stale_move_hash("jab"), stale_move_hash("fsmash"));
        assert_ne!(stale_move_hash(""), 0, "the empty id took the empty slot");
        // and an unrecorded slot is never counted.
        assert_eq!(BodyStaleMoves::default().occurrences(0), 0);
    }
}
