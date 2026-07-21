//! Which simulation frames are settled, and which are still a guess.
//!
//! A rollback host runs the simulation *speculatively*. It advances a frame
//! using a prediction of what a remote peer did, and when the real input
//! arrives it rewinds and runs that frame again with the truth. A frame is
//! **confirmed** once every player's real input for it is known: no future
//! message can change it, so it will never be simulated a third time.
//!
//! Everything inside the simulation is happy to be re-run — that is what makes
//! rollback correct. The problem is everything *outside* it. A sound is played
//! once and cannot be unplayed; a particle is spawned once; a save file is
//! written once. Those effects must be keyed to the confirmed timeline rather
//! than to whatever the simulation currently believes.
//!
//! [`ConfirmedFrameBoundary`] is the host's published answer to "where is that
//! line right now". It lives in this crate beside [`crate::ControlFrame`] and
//! [`crate::InputStream`] because it describes the same timeline those do — the
//! sim tick — and because it must be readable by low crates (persistence, the
//! forensic trace) that cannot and should not name a rollback session.
//!
//! # The absent case is the common case
//!
//! Ordinary render-frame and fixed-tick hosts never install this resource.
//! Absent means "no speculation happens here", so every consumer treats every
//! frame as confirmed and behaves exactly as it did before this type existed.
//! Read it as `Option<Res<ConfirmedFrameBoundary>>` and default to *release*.

use bevy_ecs::resource::Resource;

/// The host's view of the rollback timeline.
///
/// Published once per simulated frame by the rollback bridge, from the GGRS
/// session's own frame counters. See the module docs for why the absent case
/// means "confirm everything".
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConfirmedFrameBoundary {
    /// The frame the simulation just advanced. May be a prediction.
    pub current: i32,
    /// The newest frame that can never be simulated again. `-1` before any
    /// frame has been confirmed, matching GGRS's own convention.
    pub confirmed: i32,
    /// Bumped whenever the host installs a different session. Pending work
    /// stamped with an older generation belongs to a timeline that no longer
    /// exists and must be discarded rather than released.
    pub session: u64,
}

impl ConfirmedFrameBoundary {
    /// True when `frame` can never be simulated again.
    pub const fn is_confirmed(&self, frame: i32) -> bool {
        frame <= self.confirmed
    }

    /// True when the live world state itself is confirmed — nothing is
    /// currently predicted, so reading the world is reading settled truth.
    ///
    /// This is the condition persistence needs: a save file written while
    /// `current` is ahead of `confirmed` records a guess.
    pub const fn fully_confirmed(&self) -> bool {
        self.confirmed >= self.current
    }
}

/// Run condition: the world holds no predicted state right now.
///
/// Absent resource → no rollback host → always true. Use for irreversible
/// host-side writes (disk, network) that must never record speculation.
pub fn world_state_is_confirmed(
    boundary: Option<bevy_ecs::system::Res<ConfirmedFrameBoundary>>,
) -> bool {
    boundary.is_none_or(|boundary| boundary.fully_confirmed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_at_or_below_the_line_is_confirmed() {
        let boundary = ConfirmedFrameBoundary {
            current: 9,
            confirmed: 5,
            session: 0,
        };
        assert!(boundary.is_confirmed(4));
        assert!(
            boundary.is_confirmed(5),
            "the boundary frame itself is settled"
        );
        assert!(!boundary.is_confirmed(6));
        assert!(!boundary.is_confirmed(9), "the live frame is still a guess");
    }

    /// GGRS reports -1 before anything is confirmed. Frame 0 must not slip
    /// through on an off-by-one.
    #[test]
    fn nothing_is_confirmed_before_the_first_confirmation() {
        let boundary = ConfirmedFrameBoundary {
            current: 0,
            confirmed: -1,
            session: 0,
        };
        assert!(!boundary.is_confirmed(0));
        assert!(!boundary.fully_confirmed());
    }

    #[test]
    fn the_world_is_confirmed_only_when_nothing_is_predicted() {
        let caught_up = ConfirmedFrameBoundary {
            current: 7,
            confirmed: 7,
            session: 0,
        };
        assert!(caught_up.fully_confirmed());

        let predicting = ConfirmedFrameBoundary {
            current: 8,
            confirmed: 7,
            session: 0,
        };
        assert!(!predicting.fully_confirmed());
    }

    /// The whole point of the absent case: a fixed-tick game must be
    /// unaffected by this type existing.
    #[test]
    fn no_rollback_host_confirms_everything() {
        assert!(world_state_is_confirmed(None));
    }
}
