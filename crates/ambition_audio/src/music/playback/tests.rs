use super::*;

#[derive(Default)]
struct Output {
    handles: Assets<AudioInstance>,
    sources: Assets<KiraAudioSource>,
    states: HashMap<bevy::asset::AssetId<AudioInstance>, VoiceObservation>,
    resumed: Vec<Handle<AudioInstance>>,
    stopped: Vec<Handle<AudioInstance>>,
    gains: Vec<(Handle<AudioInstance>, f32)>,
    queued: bool,
    suspended: bool,
}

impl MusicVoiceOutput for Output {
    fn shape(&self, _: &Handle<AudioInstance>, source: &Handle<KiraAudioSource>) -> Option<MusicSourceShape> {
        Some(self.sources.get(source.id()).map(MusicSourceShape::decoded).unwrap_or(MusicSourceShape { sample_rate: 48000, frames: 192000 }))
    }

    fn ready_to_start(&self) -> bool {
        !self.suspended
    }
    fn ready(&self, _: &Handle<KiraAudioSource>) -> bool {
        true
    }
    fn submit(&mut self, _: MusicPlaybackOccurrence, _: &Handle<KiraAudioSource>, _: bool) -> Handle<AudioInstance> {
        let handle = self.handles.reserve_handle();
        self.states.insert(
            handle.id(),
            if self.queued {
                VoiceObservation::Queued
            } else {
                VoiceObservation::Paused
            },
        );
        handle
    }
    fn observe(&self, handle: &Handle<AudioInstance>) -> VoiceObservation {
        self.states
            .get(&handle.id())
            .copied()
            .unwrap_or(VoiceObservation::Absent)
    }
    fn resume(&mut self, handle: &Handle<AudioInstance>, _start: Option<f64>, _fade: f32) {
        assert_eq!(self.observe(handle), VoiceObservation::Paused);
        self.resumed.push(handle.clone());
        self.states
            .insert(handle.id(), VoiceObservation::Playing(0.0));
    }
    fn gain(&mut self, handle: &Handle<AudioInstance>, gain: f32) {
        self.gains.push((handle.clone(), gain));
    }
    fn stop(&mut self, handle: &Handle<AudioInstance>) {
        self.stopped.push(handle.clone());
        self.states.insert(handle.id(), VoiceObservation::Stopped);
    }
    fn release(&mut self, handle: &Handle<AudioInstance>) {
        self.states.remove(&handle.id());
    }
}

impl Output {
    fn request(&self, id: &str, layers: usize, looped: bool) -> MusicPlaybackRequest {
        MusicPlaybackRequest {
            start_after: None,
            entry: MusicEntry(1),
            goal_revision: 1,
            product: MusicProduct::Track {
                provider: "game".into(),
                generation: 1,
                id: id.into(),
            },
            state_id: None,
            phase_policy: MusicPhasePolicy::default(),
            looped,
            fade_seconds: 0.0,
            voices: (0..layers)
                .map(|slot| MusicVoiceSource {
                    expected_shape: None,
                    source: self.sources.reserve_handle(),
                    gain: 1.0,
                    slot,
                })
                .collect(),
        }
    }
}

#[test]
fn fresh_playback_cannot_be_stopped_by_either_previous_storage_slot() {
    for replacements in [0, 1] {
        let mut playback = MusicPlayback::default();
        let mut output = Output::default();
        playback.request(output.request("first", 2, true));
        playback.advance(&mut output, 0.0, 1.0);
        playback.advance(&mut output, 0.1, 1.0);
        if replacements == 1 {
            playback.request(output.request("second", 2, true));
            for frame in 2..7 {
                playback.advance(&mut output, frame as f64 * 0.1, 1.0);
            }
        }
        playback.silence();
        playback.request(output.request("destination", 3, true));
        for frame in 7..12 {
            playback.advance(&mut output, frame as f64 * 0.1, 1.0);
        }
        let current = playback.current().unwrap();
        assert_eq!(
            current.product,
            MusicProduct::Track {
                provider: "game".into(),
                generation: 1,
                id: "destination".into()
            }
        );
        assert_eq!(current.phase, MusicGroupPhase::Running);
        assert!(
            current
                .instances()
                .all(|handle| output.resumed.contains(handle) && !output.stopped.contains(handle))
        );
        assert_eq!(playback.reserved_voices(), 3);
    }
}

#[test]
fn late_canceled_submissions_keep_their_reservations_and_only_stop_their_own_voices() {
    let mut playback = MusicPlayback::default();
    let mut output = Output {
        queued: true,
        ..default()
    };
    playback.request(output.request("old", 6, true));
    playback.advance(&mut output, 0.0, 1.0);
    let old: Vec<_> = playback
        .groups()
        .next()
        .unwrap()
        .instances()
        .cloned()
        .collect();
    output.queued = false;
    playback.request(output.request("second", 6, true));
    playback.advance(&mut output, 0.1, 1.0);
    playback.advance(&mut output, 0.2, 1.0);
    let second: Vec<_> = playback.current().unwrap().instances().cloned().collect();
    playback.request(output.request("latest", 6, true));
    playback.advance(&mut output, 10.0, 1.0);
    assert_eq!(playback.reserved_voices(), 12);
    assert_eq!(output.resumed.len(), 6);
    for handle in &old {
        output.states.insert(handle.id(), VoiceObservation::Paused);
    }
    playback.advance(&mut output, 10.1, 1.0);
    assert!(
        old.iter()
            .all(|handle| output.stopped.contains(handle) && !output.resumed.contains(handle))
    );
    assert!(second.iter().all(|handle| !output.stopped.contains(handle)));
    for frame in 102..108 {
        playback.advance(&mut output, frame as f64 * 0.1, 1.0);
        assert!(playback.reserved_voices() <= 12);
        assert!(playback.groups().count() <= 2);
    }
    assert_eq!(
        playback.current().unwrap().product,
        MusicProduct::Track {
            provider: "game".into(),
            generation: 1,
            id: "latest".into()
        }
    );
}

#[test]
fn partial_admission_retires_the_group_and_exhausts_a_bounded_retry() {
    let mut playback = MusicPlayback::default();
    let mut output = Output::default();
    playback.request(output.request("layers", 2, true));
    playback.advance(&mut output, 0.0, 1.0);
    for attempt in 0..2 {
        let group = playback
            .groups()
            .find(|group| group.phase == MusicGroupPhase::Submitted)
            .unwrap();
        let handle = group.instances().next().unwrap().clone();
        output.states.insert(handle.id(), VoiceObservation::Absent);
        playback.advance(&mut output, 0.1 + f64::from(attempt), 1.0);
    }
    playback.advance(&mut output, 3.0, 1.0);
    assert!(output.resumed.is_empty());
    assert_eq!(output.stopped.len(), 2);
    assert_eq!(playback.reserved_voices(), 0);
    assert!(playback.outcome().is_terminal());
    assert_eq!(
        playback.last_terminal.unwrap().1,
        MusicTerminalReason::AdmissionAbsent
    );
}

#[test]
fn stopped_loops_retry_but_finished_one_shots_do_not_restart() {
    for looped in [false, true] {
        let mut playback = MusicPlayback::default();
        let mut output = Output::default();
        let request = output.request("music", 1, looped);
        playback.request(request.clone());
        playback.advance(&mut output, 0.0, 1.0);
        playback.advance(&mut output, 0.1, 1.0);
        let handle = playback
            .current()
            .unwrap()
            .instances()
            .next()
            .unwrap()
            .clone();
        output.states.insert(handle.id(), VoiceObservation::Stopped);
        playback.advance(&mut output, 1.0, 1.0);
        for frame in 11..20 {
            playback.request(request.clone());
            playback.advance(&mut output, frame as f64 * 0.1, 1.0);
        }
        assert_eq!(output.resumed.len(), if looped { 2 } else { 1 });
        assert_eq!(playback.current().is_some(), looped);
    }
}

#[test]
fn mix_changes_reach_both_groups_without_resetting_their_transition_envelopes() {
    let mut playback = MusicPlayback::default();
    let mut output = Output::default();
    let mut first = output.request("first", 1, true);
    first.fade_seconds = 1.0;
    playback.request(first);
    playback.advance(&mut output, 0.0, 1.0);
    playback.advance(&mut output, 0.1, 1.0);
    let old = playback
        .current()
        .unwrap()
        .instances()
        .next()
        .unwrap()
        .clone();
    let mut second = output.request("second", 1, true);
    second.fade_seconds = 1.0;
    playback.request(second);
    playback.advance(&mut output, 0.6, 1.0);
    playback.advance(&mut output, 0.7, 1.0);
    let new = playback
        .current()
        .unwrap()
        .instances()
        .next()
        .unwrap()
        .clone();
    playback.advance(&mut output, 1.2, 0.4);
    let latest = |handle: &Handle<AudioInstance>| {
        output
            .gains
            .iter()
            .rev()
            .find(|(id, _)| id == handle)
            .unwrap()
            .1
    };
    assert!((latest(&old) - 0.12).abs() < 1e-6);
    assert!((latest(&new) - 0.2).abs() < 1e-6);
    for frame in 13..25 {
        playback.advance(&mut output, frame as f64 * 0.1, 0.4);
    }
    let controls = output.gains.len();
    playback.advance(&mut output, 2.5, 0.4);
    assert_eq!(output.gains.len(), controls);
}

#[test]
fn a_suspended_device_keeps_submission_and_admission_local() {
    let mut playback = MusicPlayback::default();
    let mut output = Output {
        suspended: true,
        ..default()
    };
    playback.request(output.request("opening", 2, true));
    playback.advance(&mut output, 0.0, 1.0);
    playback.advance(&mut output, 10.0, 1.0);
    assert_eq!(playback.reserved_voices(), 0);
    output.suspended = false;
    playback.advance(&mut output, 10.1, 1.0);
    output.suspended = true;
    playback.advance(&mut output, 20.0, 1.0);
    assert!(output.resumed.is_empty());
    output.suspended = false;
    playback.advance(&mut output, 20.1, 1.0);
    assert_eq!(output.resumed.len(), 2);
    assert!(playback.desired_is_running());
    assert!(playback.last_terminal.is_none());
}

#[test]
fn admission_timeout_does_not_forget_unresolved_voice_reservations() {
    let mut playback = MusicPlayback::default();
    let mut output = Output {
        queued: true,
        ..default()
    };
    playback.request(output.request("blocked", 6, true));
    playback.advance(&mut output, 0.0, 1.0);
    playback.advance(&mut output, 5.0, 1.0);
    playback.advance(&mut output, 10.0, 1.0);
    assert_eq!(playback.reserved_voices(), 12);
    assert!(playback.outcome().is_terminal());
    assert_eq!(
        playback.last_terminal.unwrap().1,
        MusicTerminalReason::AdmissionTimeout
    );
    playback.request(output.request("latest", 6, true));
    playback.advance(&mut output, 11.0, 1.0);
    assert_eq!(playback.reserved_voices(), 12);
    assert!(output.resumed.is_empty());
}

#[test]
fn an_outgoing_sections_mix_cannot_overwrite_its_successor() {
    let mut playback = MusicPlayback::default();
    let mut output = Output::default();
    let mut ending = output.request("ending", 1, false);
    ending.product = MusicProduct::Section {
        generation: 1,
        provider: "game".into(),
        cue: "fight".into(),
        section: "ending".into(),
    };
    playback.request(ending);
    playback.advance(&mut output, 0.0, 1.0);
    playback.advance(&mut output, 0.1, 1.0);
    playback.request(output.request("room", 1, true));
    playback.advance(&mut output, 1.0, 1.0);
    playback.advance(&mut output, 1.1, 1.0);
    let room = playback
        .current()
        .unwrap()
        .instances()
        .next()
        .unwrap()
        .clone();
    playback.mix_current_section(MusicEntry(1), &[0.1; MAX_LAYERS]);
    playback.advance(&mut output, 1.2, 1.0);
    assert_eq!(
        output
            .gains
            .iter()
            .rev()
            .find(|(handle, _)| *handle == room)
            .unwrap()
            .1,
        1.0
    );
}

#[test]
fn a_fresh_entry_resets_completion_with_identical_one_voice_media() {
    let mut playback = MusicPlayback::default();
    let mut output = Output::default();
    let mut request = output.request("one_shot", 1, false);
    request.entry = playback.new_entry();
    playback.request(request.clone());
    playback.advance(&mut output, 0.0, 1.0);
    playback.advance(&mut output, 0.1, 1.0);
    let old = playback
        .current()
        .unwrap()
        .instances()
        .next()
        .unwrap()
        .clone();
    output.states.insert(old.id(), VoiceObservation::Stopped);
    playback.advance(&mut output, 1.0, 1.0);
    assert_eq!(playback.outcome(), MusicRequestOutcome::Completed);
    playback.request(request.clone());
    playback.advance(&mut output, 2.0, 1.0);
    assert!(playback.current().is_none());
    request.entry = playback.new_entry();
    playback.request(request.clone());
    playback.advance(&mut output, 3.0, 1.0);
    playback.advance(&mut output, 3.1, 1.0);
    output.states.insert(old.id(), VoiceObservation::Stopped);
    playback.advance(&mut output, 4.0, 1.0);
    assert_eq!(playback.current().unwrap().entry, request.entry);
    assert_eq!(playback.outcome(), MusicRequestOutcome::Pending);
    assert_eq!(playback.reserved_voices(), 1);
}

#[test]
fn transport_uses_all_native_voice_cursors_and_stops_when_one_voice_pauses() {
    let mut playback = MusicPlayback::default();
    let mut output = Output::default();
    playback.request(output.request("loop", 2, true));
    playback.advance(&mut output, 0.0, 1.0);
    playback.advance(&mut output, 0.1, 1.0);
    let voices: Vec<_> = playback.current().unwrap().instances().cloned().collect();
    output
        .states
        .insert(voices[0].id(), VoiceObservation::Playing(0.75));
    output
        .states
        .insert(voices[1].id(), VoiceObservation::Playing(0.5));
    playback.advance(&mut output, 300.0, 1.0);
    assert_eq!(playback.current().unwrap().position_seconds, Some(0.5));
    output
        .states
        .insert(voices[1].id(), VoiceObservation::Paused);
    playback.advance(&mut output, 600.0, 1.0);
    assert_eq!(playback.current().unwrap().position_seconds, None);
    output
        .states
        .insert(voices[1].id(), VoiceObservation::Playing(0.6));
    playback.advance(&mut output, 900.0, 1.0);
    assert_eq!(playback.current().unwrap().position_seconds, Some(0.6));
    assert_eq!(output.resumed.len(), 2);
}
