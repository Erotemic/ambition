//! Bounded music occurrences and exact voice control.

use super::*;
use bevy::asset::AssetApp;

const MAX_GROUPS: usize = 2;
const ADMISSION_TIMEOUT_SECONDS: f64 = 5.0;
const MAX_RETRIES: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicPlaybackOccurrence(pub u64);

/// A selected entry has its own completion and retry scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicEntry(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicProduct {
    Track {
        provider: String,
        generation: u64,
        id: String,
    },
    Section {
        provider: String,
        generation: u64,
        cue: String,
        section: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MusicVoiceSource {
    pub source: Handle<KiraAudioSource>,
    pub gain: f32,
    pub slot: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MusicPlaybackRequest {
    pub entry: MusicEntry,
    pub goal_revision: u64,
    pub product: MusicProduct,
    pub voices: Vec<MusicVoiceSource>,
    pub looped: bool,
    pub fade_seconds: f32,
}

impl MusicPlaybackRequest {
    fn can_continue(&self, other: &Self) -> bool {
        self.entry == other.entry
            && self.product == other.product
            && self.looped == other.looped
            && self.voices.len() == other.voices.len()
            && self
                .voices
                .iter()
                .zip(&other.voices)
                .all(|(a, b)| a.source == b.source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicTerminalReason {
    Completed,
    Replaced,
    Canceled,
    AdmissionAbsent,
    AdmissionTimeout,
    UnexpectedStop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicRequestOutcome {
    #[default]
    Pending,
    Completed,
    Failed(MusicTerminalReason),
}
impl MusicRequestOutcome {
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicGroupPhase {
    Submitted,
    Running,
    Retiring,
    Canceled,
}

#[derive(Clone, Debug)]
struct MusicVoice {
    handle: Handle<AudioInstance>,
    gain: f32,
    last_gain: Option<f32>,
    stop_sent: bool,
}

#[derive(Clone, Copy, Debug)]
struct Envelope {
    from: f32,
    to: f32,
    began: f64,
    seconds: f32,
}

impl Envelope {
    fn value(self, now: f64) -> f32 {
        let fraction = if self.seconds <= 0.0 {
            1.0
        } else {
            ((now - self.began) as f32 / self.seconds).clamp(0.0, 1.0)
        };
        self.from + (self.to - self.from) * fraction
    }

    fn done(self, now: f64) -> bool {
        now - self.began >= f64::from(self.seconds)
    }
}

#[derive(Clone, Debug)]
pub struct MusicPlaybackGroup {
    pub entry: MusicEntry,
    pub occurrence: MusicPlaybackOccurrence,
    pub product: MusicProduct,
    pub phase: MusicGroupPhase,
    pub position_seconds: Option<f64>,
    request: MusicPlaybackRequest,
    voices: Vec<MusicVoice>,
    submitted_at: f64,
    envelope: Envelope,
    terminal: Option<MusicTerminalReason>,
}

impl MusicPlaybackGroup {
    pub fn goal_revision(&self) -> u64 {
        self.request.goal_revision
    }

    pub fn reserved_voices(&self) -> usize {
        self.voices.len()
    }

    pub fn instances(&self) -> impl Iterator<Item = &Handle<AudioInstance>> {
        self.voices.iter().map(|voice| &voice.handle)
    }

    pub fn nonzero_gain_voice_count(&self) -> usize {
        if self.phase != MusicGroupPhase::Running {
            return 0;
        }
        self.voices
            .iter()
            .filter(|voice| voice.last_gain.is_some_and(|gain| gain > 0.001))
            .count()
    }
}

/// One pending choice and at most two submitted, running, or retiring groups.
#[derive(Resource, Default, Debug)]
pub struct MusicPlayback {
    desired: Option<MusicPlaybackRequest>,
    groups: [Option<MusicPlaybackGroup>; MAX_GROUPS],
    current: Option<MusicPlaybackOccurrence>,
    current_completion: Option<(MusicEntry, MusicProduct)>,
    next_occurrence: u64,
    next_entry: u64,
    pub revision: u64,
    retries: u8,
    outcome: MusicRequestOutcome,
    pub last_terminal: Option<(MusicPlaybackOccurrence, MusicTerminalReason)>,
}

impl MusicPlayback {
    pub(crate) fn new_entry(&mut self) -> MusicEntry {
        self.next_entry = self
            .next_entry
            .checked_add(1)
            .expect("music entry identity exhausted");
        MusicEntry(self.next_entry)
    }

    pub fn outcome(&self) -> MusicRequestOutcome {
        self.outcome
    }

    pub(super) fn completed_product(&self) -> Option<(MusicEntry, &MusicProduct)> {
        self.current_completion
            .as_ref()
            .map(|(entry, product)| (*entry, product))
    }

    /// Withdraw unsent work. An admitted outgoing group can finish or be replaced.
    pub(crate) fn defer(&mut self) {
        if self.desired.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
        self.outcome = MusicRequestOutcome::Pending;
        for group in self
            .groups
            .iter_mut()
            .flatten()
            .filter(|group| group.phase == MusicGroupPhase::Submitted)
        {
            group.phase = MusicGroupPhase::Canceled;
            group.terminal = Some(MusicTerminalReason::Canceled);
        }
    }

    pub fn groups(&self) -> impl Iterator<Item = &MusicPlaybackGroup> {
        self.groups.iter().filter_map(Option::as_ref)
    }

    pub fn current(&self) -> Option<&MusicPlaybackGroup> {
        self.groups()
            .find(|group| Some(group.occurrence) == self.current)
    }

    pub fn desired_is_running(&self) -> bool {
        self.current().is_some_and(|group| {
            group.phase == MusicGroupPhase::Running
                && self
                    .desired
                    .as_ref()
                    .is_some_and(|request| request.can_continue(&group.request))
        })
    }

    pub fn reserved_voices(&self) -> usize {
        self.groups().map(MusicPlaybackGroup::reserved_voices).sum()
    }

    pub(crate) fn request(&mut self, request: MusicPlaybackRequest) {
        assert!(!request.voices.is_empty() && request.voices.len() <= MAX_LAYERS);
        if self
            .desired
            .as_ref()
            .is_some_and(|old| old.can_continue(&request))
        {
            for group in self.groups.iter_mut().flatten() {
                if group.request.can_continue(&request)
                    && matches!(
                        group.phase,
                        MusicGroupPhase::Running | MusicGroupPhase::Submitted
                    )
                {
                    group.request.goal_revision = request.goal_revision;
                    for (voice, source) in group.voices.iter_mut().zip(&request.voices) {
                        voice.gain = source.gain;
                    }
                }
            }
            self.desired = Some(request);
            return;
        }
        self.desired = Some(request);
        self.revision = self.revision.wrapping_add(1);
        self.retries = 0;
        self.outcome = MusicRequestOutcome::Pending;
    }

    pub(crate) fn set_section_gains(
        &mut self,
        goal_revision: u64,
        provider: &str,
        cue: &str,
        section: &str,
        gains: &[f32],
    ) {
        let Some(request) = &mut self.desired else {
            return;
        };
        if !matches!(&request.product, MusicProduct::Section { provider: p, cue: c, section: s, .. }
            if p == provider && c == cue && s == section)
        {
            return;
        }
        request.goal_revision = goal_revision;
        for voice in &mut request.voices {
            voice.gain = gains[voice.slot];
        }
        for group in self.groups.iter_mut().flatten() {
            if group.request.can_continue(request)
                && matches!(
                    group.phase,
                    MusicGroupPhase::Running | MusicGroupPhase::Submitted
                )
            {
                group.request.goal_revision = goal_revision;
                for (voice, source) in group.voices.iter_mut().zip(&request.voices) {
                    voice.gain = source.gain;
                }
            }
        }
    }

    pub(crate) fn silence(&mut self) {
        self.current_completion = None;
        self.outcome = MusicRequestOutcome::Pending;
        if self.desired.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
        self.current = None;
        for group in self.groups.iter_mut().flatten() {
            group.phase = MusicGroupPhase::Canceled;
            group.terminal = Some(MusicTerminalReason::Canceled);
        }
    }

    fn failed(&mut self, occurrence: MusicPlaybackOccurrence, reason: MusicTerminalReason) {
        self.last_terminal = Some((occurrence, reason));
        if self.retries < MAX_RETRIES {
            self.retries += 1;
        } else {
            self.outcome = MusicRequestOutcome::Failed(reason);
        }
    }

    fn advance(&mut self, output: &mut impl MusicVoiceOutput, now: f64, master: f32) {
        for slot in 0..MAX_GROUPS {
            let Some(mut group) = self.groups[slot].take() else {
                continue;
            };
            let desired = self
                .desired
                .as_ref()
                .is_some_and(|request| request.can_continue(&group.request));
            if group.phase == MusicGroupPhase::Submitted && !desired {
                group.phase = MusicGroupPhase::Canceled;
                group.terminal = Some(MusicTerminalReason::Canceled);
            }
            let mut observations = [VoiceObservation::Absent; MAX_LAYERS];
            for (state, voice) in observations.iter_mut().zip(&group.voices) {
                *state = output.observe(&voice.handle);
            }
            let observations = &observations[..group.voices.len()];
            if !output.ready_to_start() && group.phase == MusicGroupPhase::Submitted {
                group.submitted_at = now;
            }
            if group.phase == MusicGroupPhase::Submitted {
                if observations.iter().any(|state| {
                    matches!(state, VoiceObservation::Absent | VoiceObservation::Stopped)
                }) {
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(MusicTerminalReason::AdmissionAbsent);
                    self.failed(group.occurrence, MusicTerminalReason::AdmissionAbsent);
                } else if observations
                    .iter()
                    .all(|state| matches!(state, VoiceObservation::Paused))
                    && output.ready_to_start()
                {
                    if let Some(old) = self.current {
                        for previous in self
                            .groups
                            .iter_mut()
                            .flatten()
                            .filter(|other| other.occurrence == old)
                        {
                            previous.phase = MusicGroupPhase::Retiring;
                            previous.terminal = Some(MusicTerminalReason::Replaced);
                            previous.envelope = Envelope {
                                from: previous.envelope.value(now),
                                to: 0.0,
                                began: now,
                                seconds: group.request.fade_seconds,
                            };
                        }
                    }
                    group.phase = MusicGroupPhase::Running;
                    group.envelope.began = now;
                    self.current_completion = None;
                    self.current = Some(group.occurrence);
                    for voice in &group.voices {
                        output.resume(&voice.handle);
                    }
                } else if now - group.submitted_at >= ADMISSION_TIMEOUT_SECONDS {
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(MusicTerminalReason::AdmissionTimeout);
                    self.failed(group.occurrence, MusicTerminalReason::AdmissionTimeout);
                }
            } else if group.phase == MusicGroupPhase::Running {
                group.position_seconds = observations
                    .iter()
                    .filter_map(|state| match state {
                        VoiceObservation::Playing(position) => Some(*position),
                        _ => None,
                    })
                    .reduce(f64::min);
                let terminal = observations.iter().any(|state| {
                    matches!(state, VoiceObservation::Absent | VoiceObservation::Stopped)
                });
                if terminal {
                    let complete = !group.request.looped
                        && observations
                            .iter()
                            .all(|state| matches!(state, VoiceObservation::Stopped));
                    let reason = if complete {
                        MusicTerminalReason::Completed
                    } else {
                        MusicTerminalReason::UnexpectedStop
                    };
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(reason);
                    if self.current == Some(group.occurrence) {
                        if complete {
                            self.current_completion = Some((group.entry, group.product.clone()));
                        }
                        self.current = None;
                    }
                    if desired {
                        if complete {
                            self.outcome = MusicRequestOutcome::Completed;
                            self.last_terminal = Some((group.occurrence, reason));
                        } else {
                            self.failed(group.occurrence, reason);
                        }
                    }
                }
            }
            if group.phase == MusicGroupPhase::Retiring && group.envelope.done(now) {
                group.phase = MusicGroupPhase::Canceled;
            }
            if group.phase == MusicGroupPhase::Canceled {
                let mut accounted = true;
                for (voice, observation) in group.voices.iter_mut().zip(observations) {
                    match observation {
                        VoiceObservation::Queued => accounted = false,
                        VoiceObservation::Absent | VoiceObservation::Stopped => {
                            output.release(&voice.handle)
                        }
                        _ => {
                            accounted = false;
                            if !voice.stop_sent {
                                output.stop(&voice.handle);
                                voice.stop_sent = true;
                            }
                        }
                    }
                }
                if accounted {
                    if self.current == Some(group.occurrence) {
                        self.current = None;
                    }
                    if let Some(reason) = group.terminal {
                        self.last_terminal = Some((group.occurrence, reason));
                    }
                    continue;
                }
            } else if matches!(
                group.phase,
                MusicGroupPhase::Running | MusicGroupPhase::Retiring
            ) {
                let envelope = group.envelope.value(now);
                for voice in &mut group.voices {
                    let gain = master * voice.gain * envelope;
                    if voice.last_gain != Some(gain) {
                        output.gain(&voice.handle, gain);
                        voice.last_gain = Some(gain);
                    }
                }
            }
            self.groups[slot] = Some(group);
        }
        let Some(request) = self.desired.as_ref() else {
            return;
        };
        if !output.ready_to_start()
            || self.outcome.is_terminal()
            || self.groups().any(|group| {
                group.request.can_continue(request)
                    && matches!(
                        group.phase,
                        MusicGroupPhase::Running | MusicGroupPhase::Submitted
                    )
            })
        {
            return;
        }
        let Some(slot) = self.groups.iter().position(Option::is_none) else {
            return;
        };
        if !request
            .voices
            .iter()
            .all(|voice| output.ready(&voice.source))
        {
            return;
        }
        self.next_occurrence = self
            .next_occurrence
            .checked_add(1)
            .expect("music occurrence space exhausted");
        let occurrence = MusicPlaybackOccurrence(self.next_occurrence);
        let voices = request
            .voices
            .iter()
            .map(|source| MusicVoice {
                handle: output.submit(&source.source, request.looped),
                gain: source.gain,
                last_gain: None,
                stop_sent: false,
            })
            .collect();
        self.groups[slot] = Some(MusicPlaybackGroup {
            entry: request.entry,
            occurrence,
            product: request.product.clone(),
            phase: MusicGroupPhase::Submitted,
            position_seconds: None,
            request: request.clone(),
            voices,
            submitted_at: now,
            envelope: Envelope {
                from: 0.0,
                to: 1.0,
                began: now,
                seconds: request.fade_seconds,
            },
            terminal: None,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum VoiceObservation {
    Queued,
    Absent,
    Paused,
    Playing(f64),
    Stopping,
    Stopped,
}

trait MusicVoiceOutput {
    fn ready_to_start(&self) -> bool {
        true
    }
    fn ready(&self, source: &Handle<KiraAudioSource>) -> bool;
    fn submit(&mut self, source: &Handle<KiraAudioSource>, looped: bool) -> Handle<AudioInstance>;
    fn observe(&self, handle: &Handle<AudioInstance>) -> VoiceObservation;
    fn resume(&mut self, handle: &Handle<AudioInstance>);
    fn gain(&mut self, handle: &Handle<AudioInstance>, gain: f32);
    fn stop(&mut self, handle: &Handle<AudioInstance>);
    fn release(&mut self, handle: &Handle<AudioInstance>);
}

#[derive(Debug)]
struct RecordedVoice {
    started: Option<f64>,
    duration: f64,
    looped: bool,
    stopped: bool,
}

/// Bounded output state for hosts without a device drain.
#[derive(Resource, Default, Debug)]
pub struct RecordedMusicOutput {
    voices: HashMap<bevy::asset::AssetId<AudioInstance>, RecordedVoice>,
    pub submitted: u64,
    pub resumed: u64,
    pub gain_changes: u64,
    pub stopped: u64,
}

struct KiraMusicOutput<'a> {
    channels: &'a DynamicAudioChannels,
    decoded: &'a Assets<KiraAudioSource>,
    instances: &'a mut Assets<AudioInstance>,
    recording: &'a mut RecordedMusicOutput,
    mode: crate::output::AudioOutputMode,
    now: f64,
    can_start: bool,
}

const OUTPUT_CHANNEL: &str = "ambition.music.occurrences";

impl MusicVoiceOutput for KiraMusicOutput<'_> {
    fn ready_to_start(&self) -> bool {
        self.can_start
    }
    fn ready(&self, source: &Handle<KiraAudioSource>) -> bool {
        self.decoded.contains(source.id())
    }

    fn submit(&mut self, source: &Handle<KiraAudioSource>, looped: bool) -> Handle<AudioInstance> {
        self.recording.submitted += 1;
        if self.mode.emits_to_device() {
            let mut command = self.channels.channel(OUTPUT_CHANNEL).play(source.clone());
            command.paused().with_volume(amplitude_to_decibels(0.0));
            if looped {
                command.looped();
            }
            command.handle()
        } else {
            let handle = self.instances.reserve_handle();
            let sound = &self
                .decoded
                .get(source)
                .expect("decoded before submission")
                .sound;
            self.recording.voices.insert(
                handle.id(),
                RecordedVoice {
                    started: None,
                    duration: sound.duration().as_secs_f64(),
                    looped,
                    stopped: false,
                },
            );
            handle
        }
    }

    fn observe(&self, handle: &Handle<AudioInstance>) -> VoiceObservation {
        if !self.mode.emits_to_device() {
            return match self.recording.voices.get(&handle.id()) {
                None => VoiceObservation::Absent,
                Some(voice) if voice.stopped => VoiceObservation::Stopped,
                Some(voice) => match voice.started {
                    None => VoiceObservation::Paused,
                    Some(started) if !voice.looped && self.now - started >= voice.duration => {
                        VoiceObservation::Stopped
                    }
                    Some(started) => VoiceObservation::Playing(self.now - started),
                },
            };
        }
        match self.instances.get(handle).map(AudioInstance::state) {
            Some(PlaybackState::Playing { position }) => VoiceObservation::Playing(position),
            Some(PlaybackState::Paused { .. }) => VoiceObservation::Paused,
            Some(PlaybackState::Pausing { .. } | PlaybackState::Stopping { .. }) => {
                VoiceObservation::Stopping
            }
            Some(PlaybackState::Stopped) => VoiceObservation::Stopped,
            _ if self.channels.channel(OUTPUT_CHANNEL).state(handle) == PlaybackState::Queued => {
                VoiceObservation::Queued
            }
            _ => VoiceObservation::Absent,
        }
    }

    fn resume(&mut self, handle: &Handle<AudioInstance>) {
        self.recording.resumed += 1;
        if self.mode.emits_to_device() {
            if let Some(mut voice) = self.instances.get_mut(handle) {
                voice.resume(AudioTween::default());
            }
        } else if let Some(voice) = self.recording.voices.get_mut(&handle.id()) {
            voice.started = Some(self.now);
        }
    }

    fn gain(&mut self, handle: &Handle<AudioInstance>, gain: f32) {
        self.recording.gain_changes += 1;
        if self.mode.emits_to_device() {
            if let Some(mut voice) = self.instances.get_mut(handle) {
                voice.set_decibels(amplitude_to_decibels(gain), AudioTween::default());
            }
        }
    }

    fn stop(&mut self, handle: &Handle<AudioInstance>) {
        self.recording.stopped += 1;
        if self.mode.emits_to_device() {
            if let Some(mut voice) = self.instances.get_mut(handle) {
                voice.stop(AudioTween::default());
            }
        } else if let Some(voice) = self.recording.voices.get_mut(&handle.id()) {
            voice.stopped = true;
        }
    }

    fn release(&mut self, handle: &Handle<AudioInstance>) {
        self.recording.voices.remove(&handle.id());
    }
}

/// Observe the last drain before submitting new work. Device timing is best effort.
pub fn advance_music_playback(
    mut playback: ResMut<MusicPlayback>,
    mut recording: ResMut<RecordedMusicOutput>,
    channels: Res<DynamicAudioChannels>,
    decoded: Res<Assets<KiraAudioSource>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    time: Res<Time<Real>>,
    mix: Res<MusicMix>,
    output: Option<Res<crate::output::AudioOutputMode>>,
    mut base: Option<ResMut<MusicPlaybackState>>,
    mut director: Option<ResMut<MusicDirectorState>>,
    unlock: Option<Res<crate::web_unlock::AudioUnlockState>>,
    conditions: Option<Res<crate::output::AudioPlaybackConditions>>,
) {
    let mode = output.as_deref().copied().unwrap_or_default();
    let can_start = !mode.emits_to_device()
        || (unlock.as_ref().is_none_or(|state| state.unlocked)
            && conditions
                .as_ref()
                .is_none_or(|conditions| !conditions.device_suspended));
    let mut sink = KiraMusicOutput {
        channels: &channels,
        decoded: &decoded,
        instances: &mut instances,
        recording: &mut recording,
        mode,
        can_start,
        now: time.elapsed_secs_f64(),
    };
    playback.advance(&mut sink, time.elapsed_secs_f64(), mix.effective_music());
    if let Some(director) = director.as_deref_mut() {
        if let Some(group) = playback.current().filter(|group| {
            group.phase == MusicGroupPhase::Running && Some(group.entry) == director.entry
        }) {
            if let Some(selection) = director.selection.clone() {
                director.admitted = Some(AdmittedMusic {
                    entry: group.entry,
                    selection,
                });
            }
        }
        if director.admitted.as_ref().is_some_and(|admitted| {
            !playback.groups().any(|group| {
                group.entry == admitted.entry && group.phase == MusicGroupPhase::Running
            })
        }) {
            director.admitted = None;
        }
        director.transport_completed = playback.completed_product().is_some_and(|(entry, product)|
            Some(entry) == director.entry && matches!(product,
                MusicProduct::Section { cue, section, .. } if director.active_cue_id.as_ref() == Some(cue) && director.current_section_id.as_ref() == Some(section)));
    }
    if let Some(base) = base.as_deref_mut() {
        let track = base.active_track().to_owned();
        base.set_instance(playback.current().and_then(|group| match &group.product {
            MusicProduct::Track { id, .. } if *id == track => group.instances().next().cloned(),
            _ => None,
        }));
    }
}

pub fn add_music_playback(app: &mut App) {
    app.init_resource::<MusicPlayback>()
        .init_resource::<RecordedMusicOutput>()
        .init_resource::<MusicMix>()
        .add_systems(
            PostUpdate,
            advance_music_playback
                .after(bevy_kira_audio::AudioSystemSet::PlayTypedChannels)
                .after(bevy_kira_audio::AudioSystemSet::PlayDynamicChannels),
        );
    if !app.world().contains_resource::<Assets<AudioInstance>>() {
        app.init_asset::<AudioInstance>();
    }
    let mut channels = app
        .world_mut()
        .get_resource_or_init::<DynamicAudioChannels>();
    if !channels.is_channel(OUTPUT_CHANNEL) {
        channels.create_channel(OUTPUT_CHANNEL);
    }
}

#[cfg(test)]
mod tests;
