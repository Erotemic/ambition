//! **Input-stream capture** (netcode N0.2) — the one place a session's input is
//! recorded.
//!
//! The artifact is [`ambition_engine_core::InputStream`]; this is the sim-side
//! recorder that fills it. Replay fixtures, RL trajectories, desync forensics,
//! and (later) the lockstep wire all read the same recording, so they cannot
//! disagree about what the player did.
//!
//! It records `SlotControls` AFTER the input phase has finalized them — the
//! frame the simulation actually consumed, not the frame the device produced.
//! Those differ: gesture recognition (double-tap fast-fall), portal input warp,
//! and the fixed-tick latch all rewrite the frame between device and sim. A
//! recording of the device frame would replay into a different trajectory.

use bevy::prelude::{Res, ResMut, Resource};

use ambition_characters::brain::SlotControls;
use ambition_engine_core::InputStream;
use ambition_time::SimTick;

/// Records every tick's finalized [`SlotControls`] into an [`InputStream`].
///
/// Disarmed by default and cheap to leave installed: the recording system is
/// gated on [`Self::is_recording`], so an unarmed recorder costs one resource
/// read per tick and nothing else.
#[derive(Resource, Debug, Default)]
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

/// Append this tick's finalized slot input to the armed recording.
///
/// Registered in `SandboxSet::PlayerInput` immediately after
/// `populate_slot_controls`, which is the moment `SlotControls` becomes the
/// input this tick will be simulated with.
pub fn record_input_stream(
    tick: Res<SimTick>,
    slots: Res<SlotControls>,
    mut recorder: ResMut<InputStreamRecorder>,
) {
    let slot_count = recorder.slot_count;
    let Some(stream) = recorder.stream.as_mut() else {
        return;
    };
    stream.push(
        tick.get(),
        (0..slot_count).map(|slot| slots.get(ambition_characters::brain::PlayerSlot(slot))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core::ControlFrame;

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
