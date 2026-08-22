//! Host-published boundary between confirmed and speculative simulation frames.
//!
//! Irreversible observers such as persistence, audio, and presentation effects
//! use [`ConfirmedFrameBoundary`] to avoid committing speculative results. Low
//! crates can depend on this timeline fact without naming a rollback backend.
//! When the resource is absent, there is no rollback host and frames are treated
//! as confirmed.

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
