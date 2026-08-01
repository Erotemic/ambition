//! **The input stream** (netcode N0.2) — the per-tick input artifact.
//!
//! One type serves four jobs that were each about to grow their own format:
//!
//! - **replay fixtures** — record a session, replay it, assert zero divergence;
//! - **RL trajectories** — the action sequence half of `(observation, action)`;
//! - **desync forensics** — the input both peers agreed on, so a divergence can
//!   be blamed on the sim rather than the input;
//! - **the wire format**, later — N2 lockstep peers exchange exactly this.
//!
//! Because the same bytes must eventually cross a network, the encoding obeys
//! the Q4 guardrails (see `docs/adr/0023-same-build-determinism.md`): explicit
//! field order, no platform-width-dependent types (`u64` ticks, `u32` version —
//! never `usize`), and a version stamp checked on load.
//!
//! # The timeline
//!
//! Frames key on `SimTick` — the sim step index, not a wall-clock instant and
//! not a rendered frame. A stream is contiguous: tick `t`, `t+1`, … with no
//! gaps, because a gap is not "no input", it is a missing decision.
//!
//! # Slots
//!
//! Each frame carries one `ControlFrame` per player slot, in slot order. A
//! single-player stream has one. This is why the artifact is `SlotControls`
//! per tick rather than one `ControlFrame` per tick: local multiplayer (N1) and
//! lockstep (N2) are the same recording.

use serde::{Deserialize, Serialize};

use crate::ControlFrame;

/// Bump when the meaning of an existing field changes, or a field is removed.
///
/// ADDING a field does not need a bump: `ControlFrame` deserializes with
/// `#[serde(default)]`, so an older stream loads with the new field neutral —
/// which is exactly what an older recording meant by it.
pub const INPUT_STREAM_VERSION: u32 = 1;

/// Everything wrong with a stream, said precisely enough to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputStreamError {
    /// Recorded by a build whose stream format this one cannot read.
    UnsupportedVersion { found: u32, expected: u32 },
    /// A tick rate of zero has no timeline.
    ZeroTickHz,
    /// Ticks must be contiguous and increasing: a gap is a missing decision,
    /// not an absence of one.
    NonContiguousTick {
        index: u32,
        expected: u64,
        found: u64,
    },
    /// Every frame must carry the same number of slots — a session does not
    /// gain or lose a player mid-stream without saying so.
    SlotCountChanged {
        index: u32,
        expected: u32,
        found: u32,
    },
    /// A frame with no slots records nobody's input.
    NoSlots,
}

impl core::fmt::Display for InputStreamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "input stream version {found} is not readable by this build (expects {expected})"
            ),
            Self::ZeroTickHz => write!(f, "input stream has a tick rate of 0 Hz"),
            Self::NonContiguousTick {
                index,
                expected,
                found,
            } => write!(
                f,
                "input stream frame {index} is tick {found}, expected {expected} \
                 (ticks must be contiguous — a gap is a missing decision)"
            ),
            Self::SlotCountChanged {
                index,
                expected,
                found,
            } => write!(
                f,
                "input stream frame {index} has {found} slot(s), expected {expected}"
            ),
            Self::NoSlots => write!(f, "input stream frames record no slots"),
        }
    }
}

impl std::error::Error for InputStreamError {}

/// One sim tick's input: every slot's control frame, in slot order.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct InputStreamFrame {
    /// The `SimTick` this input is consumed on.
    pub tick: u64,
    /// One entry per player slot, index == slot. Never empty.
    pub slots: Vec<ControlFrame>,
}

/// A contiguous recording of per-tick input. See the module docs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InputStream {
    /// [`INPUT_STREAM_VERSION`] at record time. Checked by [`Self::validate`].
    pub version: u32,
    /// The sim tick rate this was recorded at. A stream replayed at a different
    /// rate is a different trajectory, so the rate travels with the input.
    pub tick_hz: u32,
    /// Contiguous, increasing by one.
    pub frames: Vec<InputStreamFrame>,
}

impl InputStream {
    /// An empty stream to record into.
    pub fn recording_at(tick_hz: u32) -> Self {
        Self {
            version: INPUT_STREAM_VERSION,
            tick_hz,
            frames: Vec::new(),
        }
    }

    /// Append one tick's slots. The caller is the sim, so the tick is whatever
    /// `SimTick` says; [`Self::validate`] is what catches a caller that lied.
    pub fn push(&mut self, tick: u64, slots: impl IntoIterator<Item = ControlFrame>) {
        self.frames.push(InputStreamFrame {
            tick,
            slots: slots.into_iter().collect(),
        });
    }

    /// The first recorded tick, if any.
    pub fn start_tick(&self) -> Option<u64> {
        self.frames.first().map(|f| f.tick)
    }

    /// Number of recorded ticks.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// How many slots each frame carries. `None` for an empty stream.
    pub fn slot_count(&self) -> Option<usize> {
        self.frames.first().map(|f| f.slots.len())
    }

    /// The input for one tick, if it is in range.
    pub fn frame(&self, tick: u64) -> Option<&InputStreamFrame> {
        let start = self.start_tick()?;
        let offset = tick.checked_sub(start)?;
        self.frames.get(usize::try_from(offset).ok()?)
    }

    /// One slot's frame at one tick, neutral when the slot is absent.
    pub fn control(&self, tick: u64, slot: u8) -> ControlFrame {
        self.frame(tick)
            .and_then(|f| f.slots.get(usize::from(slot)))
            .copied()
            .unwrap_or_default()
    }

    /// Slot 0's frames in tick order — the single-player replay path.
    pub fn primary_frames(&self) -> impl Iterator<Item = ControlFrame> + '_ {
        self.frames
            .iter()
            .map(|f| f.slots.first().copied().unwrap_or_default())
    }

    /// Everything a reader must check before trusting a stream off disk or off
    /// the wire. An empty stream is valid — it is a session with no ticks.
    pub fn validate(&self) -> Result<(), InputStreamError> {
        if self.version != INPUT_STREAM_VERSION {
            return Err(InputStreamError::UnsupportedVersion {
                found: self.version,
                expected: INPUT_STREAM_VERSION,
            });
        }
        if self.tick_hz == 0 {
            return Err(InputStreamError::ZeroTickHz);
        }
        let Some(first) = self.frames.first() else {
            return Ok(());
        };
        if first.slots.is_empty() {
            return Err(InputStreamError::NoSlots);
        }
        let slot_count = first.slots.len();
        for (i, frame) in self.frames.iter().enumerate() {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            let expected = first.tick + i as u64;
            if frame.tick != expected {
                return Err(InputStreamError::NonContiguousTick {
                    index,
                    expected,
                    found: frame.tick,
                });
            }
            if frame.slots.len() != slot_count {
                return Err(InputStreamError::SlotCountChanged {
                    index,
                    expected: u32::try_from(slot_count).unwrap_or(u32::MAX),
                    found: u32::try_from(frame.slots.len()).unwrap_or(u32::MAX),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jump() -> ControlFrame {
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            axis_x: -0.5,
            ..ControlFrame::default()
        }
    }

    fn stream() -> InputStream {
        let mut s = InputStream::recording_at(60);
        s.push(0, [ControlFrame::default()]);
        s.push(1, [jump()]);
        s.push(2, [ControlFrame::default()]);
        s
    }

    #[test]
    fn a_recorded_stream_validates() {
        assert_eq!(stream().validate(), Ok(()));
        assert_eq!(stream().len(), 3);
        assert_eq!(stream().start_tick(), Some(0));
        assert_eq!(stream().slot_count(), Some(1));
    }

    /// An empty stream is a session with no ticks — valid, not broken.
    #[test]
    fn an_empty_stream_validates() {
        assert_eq!(InputStream::recording_at(60).validate(), Ok(()));
    }

    #[test]
    fn json_round_trips_every_field_exactly() {
        let original = stream();
        let text = serde_json::to_string(&original).expect("serializes");
        let decoded: InputStream = serde_json::from_str(&text).expect("deserializes");
        assert_eq!(decoded, original, "the stream is the input, byte for byte");
        assert_eq!(decoded.control(1, 0).axis_x, -0.5);
        assert!(decoded.control(1, 0).jump_pressed);
    }

    /// A stream from a build that recorded fields this one has never heard of,
    /// or that omitted fields added since, must still load — with the missing
    /// ones NEUTRAL, which is what the old recording meant by them.
    #[test]
    fn an_older_stream_loads_with_new_fields_neutral() {
        let text = r#"{
            "version": 1,
            "tick_hz": 60,
            "frames": [{ "tick": 7, "slots": [{ "axis_x": 1.0, "jump_pressed": true }] }]
        }"#;
        let decoded: InputStream = serde_json::from_str(text).expect("deserializes");
        assert_eq!(decoded.validate(), Ok(()));
        let frame = decoded.control(7, 0);
        assert_eq!(frame.axis_x, 1.0);
        assert!(frame.jump_pressed);
        assert!(!frame.shield_held, "an unrecorded field is neutral");
        assert_eq!(frame.aim_y, 0.0);
    }

    #[test]
    fn a_future_version_is_refused_rather_than_misread() {
        let mut s = stream();
        s.version = INPUT_STREAM_VERSION + 1;
        assert_eq!(
            s.validate(),
            Err(InputStreamError::UnsupportedVersion {
                found: INPUT_STREAM_VERSION + 1,
                expected: INPUT_STREAM_VERSION,
            })
        );
    }

    /// A gap is a missing decision, not an absence of one. Silently replaying
    /// across it would produce a trajectory nobody recorded.
    #[test]
    fn a_tick_gap_is_an_error() {
        let mut s = InputStream::recording_at(60);
        s.push(0, [ControlFrame::default()]);
        s.push(2, [ControlFrame::default()]);
        assert_eq!(
            s.validate(),
            Err(InputStreamError::NonContiguousTick {
                index: 1,
                expected: 1,
                found: 2,
            })
        );
    }

    #[test]
    fn a_stream_that_gains_a_player_mid_recording_is_an_error() {
        let mut s = InputStream::recording_at(60);
        s.push(0, [ControlFrame::default()]);
        s.push(1, [ControlFrame::default(), ControlFrame::default()]);
        assert!(matches!(
            s.validate(),
            Err(InputStreamError::SlotCountChanged { .. })
        ));
    }

    #[test]
    fn a_zero_hz_stream_has_no_timeline() {
        let mut s = InputStream::recording_at(0);
        s.push(0, [ControlFrame::default()]);
        assert_eq!(s.validate(), Err(InputStreamError::ZeroTickHz));
    }

    /// Reading outside the recording is neutral input, not a panic: a replay
    /// that outlives its stream simply stops being driven.
    #[test]
    fn reading_past_the_end_is_neutral() {
        let s = stream();
        assert_eq!(s.control(99, 0), ControlFrame::default());
        assert_eq!(s.control(1, 3), ControlFrame::default());
        assert!(s.frame(99).is_none());
    }

    #[test]
    fn primary_frames_walks_slot_zero_in_tick_order() {
        let frames: Vec<_> = stream().primary_frames().collect();
        assert_eq!(frames.len(), 3);
        assert!(frames[1].jump_pressed);
        assert!(!frames[0].jump_pressed);
    }
}
