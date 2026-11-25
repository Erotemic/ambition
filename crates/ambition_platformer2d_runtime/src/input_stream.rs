//! Input-stream capture (netcode N0.2) — the one place a session's input is
//! recorded.
//!
//! The artifact is [`ambition_platformer2d_core::InputStream`]; this is the sim-side
//! recorder that fills it. Replay fixtures, RL trajectories, desync forensics,
//! and (later) the lockstep wire all read the same recording, so they cannot
//! disagree about what the player did.
//!
//! It records each seat's ADMITTED input (IN1): the frame the backend or driver published for
//! this tick, after sample coalescing and before the simulation interprets or spends it. A
//! reset consumer clears `reset_pressed`, gestures write `fast_fall_pressed`; recording after
//! them lost the cause of what they did, and a replay re-injects at this same cut, so those
//! stages run again on the replay and reach the same result. The cut is neither the device
//! sample (render-rate, not yet a tick) nor the consumed frame.

use bevy::prelude::{Res, ResMut, Resource, SystemSet};

use ambition_characters::control::{SeatRawFrames, SlotControlLatches, SlotControls};
use ambition_platformer2d_core::InputStream;
use ambition_time::SimTick;

/// Records every tick's admitted seat input into an [`InputStream`].
///
/// Disarmed by default and cheap to leave installed: the recording system is
/// gated on [`Self::is_recording`], so an unarmed recorder costs one resource
/// read per tick and nothing else.
#[derive(Resource, Clone, Debug, Default)]
pub struct InputStreamRecorder {
    stream: Option<InputStream>,
    slot_count: u8,
}

impl InputStreamRecorder {
    /// Begin recording `slot_count` slots at `tick_hz`. Discards any previous
    /// recording — a recorder holds one session.
    pub fn arm(&mut self, tick_hz: u32, slot_count: u8) {
        self.stream = Some(InputStream::recording_at(tick_hz));
        self.slot_count = slot_count.max(1);
    }

    /// Begin recording the local single-player slot.
    pub fn arm_single_player(&mut self, tick_hz: u32) {
        self.arm(tick_hz, 1);
    }

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }

    /// The recording so far.
    pub fn stream(&self) -> Option<&InputStream> {
        self.stream.as_ref()
    }

    /// Stop recording and take the stream.
    pub fn finish(&mut self) -> Option<InputStream> {
        self.stream.take()
    }
}

/// Bevy run condition: is a recording in progress?
pub fn input_stream_recording(recorder: Option<Res<InputStreamRecorder>>) -> bool {
    recorder.is_some_and(|r| r.is_recording())
}

/// The point in `PlayerInput` where every seat's input for this tick is admitted and not yet
/// interpreted: after the host publishes it (`LocalInputPublication`, or a rollback backend
/// before the simulation), before the room-replay admission, the reset consumer and
/// `PlayerInputSet::Device`. A stage that edits a seat frame orders after this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AdmittedSeatInput;

/// Append this tick's admitted seat input to the armed recording, at [`AdmittedSeatInput`].
///
/// `seat_frame_this_tick` names which table holds this tick's input on this host: a latch or
/// rollback host has published `SlotControls`; a latchless host has the driver's raw row and
/// publishes it later. A resimulated tick records again and replaces the speculative tail.
pub fn record_input_stream(
    tick: Res<SimTick>,
    latches: Option<Res<SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    slots: Res<SlotControls>,
    raw: Res<SeatRawFrames>,
    mut recorder: ResMut<InputStreamRecorder>,
) {
    let slot_count = recorder.slot_count;
    let Some(stream) = recorder.stream.as_mut() else {
        return;
    };
    stream.push(
        tick.get(),
        (0..slot_count).map(|slot| {
            ambition_platformer2d_actor::control::seat_frame_this_tick(
                latches.as_deref(),
                rollback.as_deref(),
                &slots,
                &raw,
                ambition_characters::control::PlayerSlot(slot),
            )
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::ControlFrame;

    #[test]
    fn an_unarmed_recorder_is_not_recording() {
        let recorder = InputStreamRecorder::default();
        assert!(!recorder.is_recording());
        assert!(recorder.stream().is_none());
    }

    #[test]
    fn arming_starts_a_fresh_contiguous_recording() {
        let mut recorder = InputStreamRecorder::default();
        recorder.arm_single_player(60);
        assert!(recorder.is_recording());

        let stream = recorder.stream.as_mut().expect("armed");
        stream.push(0, [ControlFrame::default()]);
        stream.push(1, [ControlFrame::default()]);

        let finished = recorder.finish().expect("a recording");
        assert_eq!(finished.validate(), Ok(()));
        assert_eq!(finished.len(), 2);
        assert_eq!(finished.slot_count(), Some(1));
        assert!(!recorder.is_recording(), "finish() disarms");
    }

    /// Re-arming discards the previous session rather than concatenating two
    /// unrelated timelines into one non-contiguous stream.
    #[test]
    fn re_arming_discards_the_previous_recording() {
        let mut recorder = InputStreamRecorder::default();
        recorder.arm_single_player(60);
        recorder
            .stream
            .as_mut()
            .unwrap()
            .push(0, [ControlFrame::default()]);
        recorder.arm_single_player(60);
        assert!(recorder.stream().expect("armed").is_empty());
    }

    /// A zero slot count records nobody. Clamp rather than produce a stream
    /// whose every frame fails `validate()` with `NoSlots`.
    #[test]
    fn a_recorder_always_records_at_least_one_slot() {
        let mut recorder = InputStreamRecorder::default();
        recorder.arm(60, 0);
        assert_eq!(recorder.slot_count, 1);
    }
}
