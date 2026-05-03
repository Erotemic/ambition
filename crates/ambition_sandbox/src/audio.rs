//! Procedural audio for Ambition sandbox feedback and music.
//!
//! The sandbox renders procedural sound effects and declarative lo-fi music
//! into in-memory Kira static sound assets at visible startup. Kira owns the
//! playback backend, channels, fades, and looping; the RON data remains the
//! source of truth for cue shapes and music arrangements.

use ambition_engine as ae;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween, Frame,
    StaticSoundData, StaticSoundSettings,
};
use fundsp::audiounit::AudioUnit;
use fundsp::prelude as dsp;
use std::sync::Arc;
use std::time::Duration;

use crate::data::{AudioSpec, MusicSpec, NoteSpec, SfxSpec, SoundCueKey, WaveformSpec};

pub const ORIGINAL_TRACK_ID: &str = "original_lofi_loop";

#[derive(Resource)]
pub struct MusicChannel;

#[derive(Resource)]
pub struct SfxChannel;

/// Typed sandbox-side audio message (Bevy 0.18 buffered-message API; the
/// pre-0.18 `Event`/`EventReader` names moved to observer-style one-shots).
///
/// Simulation systems emit `SfxMessage` values into a per-frame `Vec` (the
/// "Vec collector" pattern documented in `docs/events_refactor_plan.md`).
/// `sandbox_update` drains the Vec into the `Messages<SfxMessage>` resource
/// at end-of-frame, and the presentation-side `audio_play_sfx_messages`
/// system reads it and plays the actual sound. Headless builds omit the
/// audio subscriber; messages accumulate until drained, costing nothing
/// visible.
///
/// Each variant carries `pos` to set up future spatialized audio without
/// another refactor; today's audio playback ignores it.
#[derive(Message, Clone, Copy, Debug)]
pub enum SfxMessage {
    Jump { pos: ae::Vec2 },
    DoubleJump { pos: ae::Vec2 },
    Dash { pos: ae::Vec2 },
    Blink { pos: ae::Vec2, precision: bool },
    Pogo { pos: ae::Vec2 },
    Slash { pos: ae::Vec2 },
    Hit { pos: ae::Vec2 },
    Death { pos: ae::Vec2 },
    Reset { pos: ae::Vec2 },
}

impl SfxMessage {
    pub fn cue(self) -> SoundCue {
        match self {
            SfxMessage::Jump { .. } => SoundCue::Jump,
            SfxMessage::DoubleJump { .. } => SoundCue::DoubleJump,
            SfxMessage::Dash { .. } => SoundCue::Dash,
            SfxMessage::Blink {
                precision: false, ..
            } => SoundCue::Blink,
            SfxMessage::Blink {
                precision: true, ..
            } => SoundCue::PrecisionBlink,
            SfxMessage::Pogo { .. } => SoundCue::Pogo,
            SfxMessage::Slash { .. } => SoundCue::Slash,
            SfxMessage::Hit { .. } => SoundCue::Hit,
            SfxMessage::Death { .. } => SoundCue::Death,
            SfxMessage::Reset { .. } => SoundCue::Reset,
        }
    }
}

/// Presentation-side subscriber. Reads `SfxMessage`s and plays the actual
/// sound through Kira's SFX channel. Skipped in headless builds.
pub fn audio_play_sfx_messages(
    mut messages: MessageReader<SfxMessage>,
    library: Res<AudioLibrary>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
) {
    for message in messages.read() {
        sfx_channel.play(library.sfx_handle(message.cue()));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundCue {
    Jump,
    DoubleJump,
    Dash,
    Blink,
    PrecisionBlink,
    Slash,
    Hit,
    Pogo,
    Reset,
    Death,
    Respawn,
}

impl SoundCue {
    const ALL: [Self; 11] = [
        Self::Jump,
        Self::DoubleJump,
        Self::Dash,
        Self::Blink,
        Self::PrecisionBlink,
        Self::Slash,
        Self::Hit,
        Self::Pogo,
        Self::Reset,
        Self::Death,
        Self::Respawn,
    ];
}

impl From<SoundCue> for SoundCueKey {
    fn from(value: SoundCue) -> Self {
        match value {
            SoundCue::Jump => Self::Jump,
            SoundCue::DoubleJump => Self::DoubleJump,
            SoundCue::Dash => Self::Dash,
            SoundCue::Blink => Self::Blink,
            SoundCue::PrecisionBlink => Self::PrecisionBlink,
            SoundCue::Slash => Self::Slash,
            SoundCue::Hit => Self::Hit,
            SoundCue::Pogo => Self::Pogo,
            SoundCue::Reset => Self::Reset,
            SoundCue::Death => Self::Death,
            SoundCue::Respawn => Self::Respawn,
        }
    }
}

#[derive(Clone)]
pub struct MusicTrackRuntime {
    pub id: String,
    pub display_name: String,
    pub handle: Handle<KiraAudioSource>,
    pub duration_seconds: f32,
}

#[derive(Resource)]
pub struct AudioLibrary {
    sfx: HashMap<SoundCue, Handle<KiraAudioSource>>,
    fallback_sfx: Handle<KiraAudioSource>,
    music_tracks: Vec<MusicTrackRuntime>,
}

impl AudioLibrary {
    pub fn new(audio_sources: &mut Assets<KiraAudioSource>, spec: &AudioSpec) -> Self {
        if let Err(error) = spec.validate() {
            warn!("invalid audio spec: {error}");
        }
        let sample_rate = spec.sample_rate.max(8_000);
        let mut sfx = HashMap::default();
        for cue in SoundCue::ALL {
            let sfx_spec = find_sfx(spec, cue);
            sfx.insert(
                cue,
                add_rendered_audio(audio_sources, render_sfx(sfx_spec, sample_rate)),
            );
        }
        let fallback_sfx = sfx.get(&SoundCue::Jump).cloned().unwrap_or_else(|| {
            add_rendered_audio(
                audio_sources,
                render_sfx(fallback_sfx(SoundCueKey::Jump), sample_rate),
            )
        });

        let music_tracks = spec
            .music_tracks
            .iter()
            .map(|track| MusicTrackRuntime {
                id: track.id.clone(),
                display_name: track.display_name.clone(),
                handle: add_rendered_audio(
                    audio_sources,
                    render_lofi_theme(&track.arrangement, sample_rate),
                ),
                duration_seconds: track.arrangement.duration_seconds(),
            })
            .collect();

        Self {
            sfx,
            fallback_sfx,
            music_tracks,
        }
    }

    pub fn sfx_handle(&self, cue: SoundCue) -> Handle<KiraAudioSource> {
        self.sfx
            .get(&cue)
            .cloned()
            .unwrap_or_else(|| self.fallback_sfx.clone())
    }

    pub fn track(&self, id: &str) -> Option<&MusicTrackRuntime> {
        self.music_tracks.iter().find(|track| track.id == id)
    }

    pub fn track_count(&self) -> usize {
        self.music_tracks.len()
    }

    pub fn default_track_id<'a>(&'a self, configured: &'a str) -> Option<&'a str> {
        if self.track(configured).is_some() {
            Some(configured)
        } else if self.track(ORIGINAL_TRACK_ID).is_some() {
            warn!(
                "default music track '{configured}' is missing; falling back to '{ORIGINAL_TRACK_ID}'"
            );
            Some(ORIGINAL_TRACK_ID)
        } else {
            let fallback = self.music_tracks.first().map(|track| track.id.as_str());
            if let Some(fallback) = fallback {
                warn!(
                    "default music track '{configured}' is missing; falling back to '{fallback}'"
                );
            }
            fallback
        }
    }

    pub fn display_name(&self, id: &str) -> &str {
        self.track(id)
            .map(|track| track.display_name.as_str())
            .unwrap_or("Unknown Track")
    }

    pub fn next_track_id(&self, active: &str) -> Option<&str> {
        self.track_offset(active, 1)
    }

    pub fn previous_track_id(&self, active: &str) -> Option<&str> {
        self.track_offset(active, -1)
    }

    fn track_offset(&self, active: &str, offset: isize) -> Option<&str> {
        if self.music_tracks.is_empty() {
            return None;
        }
        let index = self
            .music_tracks
            .iter()
            .position(|track| track.id == active)
            .unwrap_or(0);
        let len = self.music_tracks.len() as isize;
        let next = (index as isize + offset).rem_euclid(len) as usize;
        Some(self.music_tracks[next].id.as_str())
    }
}

#[derive(Resource, Clone, Debug)]
pub struct MusicPlaybackState {
    pub active_track: String,
}

impl MusicPlaybackState {
    pub fn from_audio_spec(spec: &AudioSpec, library: &AudioLibrary) -> Self {
        let active_track = library
            .default_track_id(&spec.default_music_track)
            .unwrap_or_default()
            .to_string();
        Self { active_track }
    }

    pub fn active_display_name<'a>(&self, library: &'a AudioLibrary) -> &'a str {
        library.display_name(&self.active_track)
    }
}

pub fn start_default_music(
    library: Res<AudioLibrary>,
    state: Res<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
) {
    play_music_track(&library, &state.active_track, &music_channel);
}

pub fn switch_to_music_track(
    library: &AudioLibrary,
    state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
    next_track: &str,
) {
    if library.track(next_track).is_none() {
        warn!("cannot switch to missing music track '{next_track}'");
        return;
    }
    state.active_track = next_track.to_string();
    music_channel.stop().fade_out(AudioTween::new(
        Duration::from_millis(180),
        AudioEasing::OutPowi(2),
    ));
    play_music_track(library, next_track, music_channel);
}

fn play_music_track(
    library: &AudioLibrary,
    track_id: &str,
    music_channel: &AudioChannel<MusicChannel>,
) {
    let Some(track) = library.track(track_id) else {
        warn!("cannot play missing music track '{track_id}'");
        return;
    };
    music_channel
        .play(track.handle.clone())
        .looped()
        .fade_in(AudioTween::new(
            Duration::from_millis(220),
            AudioEasing::InPowi(2),
        ));
}

#[derive(Clone, Debug)]
pub struct RenderedAudio {
    pub sample_rate: u32,
    pub frames: Vec<Frame>,
}

impl RenderedAudio {
    pub fn duration_seconds(&self) -> f32 {
        self.frames.len() as f32 / self.sample_rate as f32
    }

    fn into_source(self) -> KiraAudioSource {
        KiraAudioSource {
            sound: StaticSoundData {
                sample_rate: self.sample_rate,
                frames: Arc::from(self.frames.into_boxed_slice()),
                settings: StaticSoundSettings::default(),
                slice: None,
            },
        }
    }
}

fn add_rendered_audio(
    audio_sources: &mut Assets<KiraAudioSource>,
    rendered: RenderedAudio,
) -> Handle<KiraAudioSource> {
    audio_sources.add(rendered.into_source())
}

fn find_sfx(spec: &AudioSpec, cue: SoundCue) -> SfxSpec {
    let key = SoundCueKey::from(cue);
    spec.sfx
        .iter()
        .copied()
        .find(|candidate| candidate.cue == key)
        .unwrap_or_else(|| fallback_sfx(key))
}

fn fallback_sfx(cue: SoundCueKey) -> SfxSpec {
    let (waveform, frequency, frequency_end, duration, volume, noise) = match cue {
        SoundCueKey::Jump => (WaveformSpec::Sine, 460.0, 720.0, 0.085, 0.22, 0.0),
        SoundCueKey::DoubleJump => (WaveformSpec::Triangle, 520.0, 940.0, 0.115, 0.22, 0.0),
        SoundCueKey::Dash => (WaveformSpec::Saw, 260.0, 110.0, 0.105, 0.18, 0.0),
        SoundCueKey::Blink => (WaveformSpec::Triangle, 740.0, 260.0, 0.090, 0.18, 0.0),
        SoundCueKey::PrecisionBlink => (WaveformSpec::Sine, 880.0, 180.0, 0.160, 0.20, 0.08),
        SoundCueKey::Slash => (WaveformSpec::Square, 620.0, 340.0, 0.075, 0.16, 0.0),
        SoundCueKey::Hit => (WaveformSpec::Triangle, 220.0, 88.0, 0.105, 0.26, 0.44),
        SoundCueKey::Pogo => (WaveformSpec::Sine, 360.0, 880.0, 0.105, 0.22, 0.0),
        SoundCueKey::Reset => (WaveformSpec::Sine, 160.0, 90.0, 0.150, 0.16, 0.0),
        SoundCueKey::Death => (WaveformSpec::Saw, 140.0, 48.0, 0.220, 0.24, 0.18),
        SoundCueKey::Respawn => (WaveformSpec::Triangle, 440.0, 660.0, 0.145, 0.20, 0.0),
    };
    SfxSpec {
        cue,
        waveform,
        frequency,
        frequency_end,
        duration,
        volume,
        attack: 0.003,
        release: 0.045,
        noise,
    }
}

fn render_sfx(spec: SfxSpec, sample_rate: u32) -> RenderedAudio {
    match spec.waveform {
        WaveformSpec::Sine => render_sfx_with_fundsp_osc(spec, sample_rate, dsp::sine::<f32>()),
        WaveformSpec::Square => render_sfx_with_fundsp_osc(spec, sample_rate, dsp::square()),
        WaveformSpec::Triangle => render_sfx_with_fundsp_osc(spec, sample_rate, dsp::triangle()),
        WaveformSpec::Saw => render_sfx_with_fundsp_osc(spec, sample_rate, dsp::soft_saw()),
    }
}

fn render_sfx_with_fundsp_osc(
    mut spec: SfxSpec,
    sample_rate: u32,
    mut oscillator: impl AudioUnit,
) -> RenderedAudio {
    let sample_count = ((spec.duration * sample_rate as f32).max(1.0)) as usize;
    let attack = (spec.attack * sample_rate as f32) as usize;
    let release = (spec.release * sample_rate as f32) as usize;
    let mut frames = Vec::with_capacity(sample_count);

    oscillator.set_sample_rate(sample_rate as f64);
    oscillator.reset();

    let mut noise = dsp::white();
    noise.set_sample_rate(sample_rate as f64);
    noise.reset();

    let mut body_filter = dsp::lowpole_hz(8_000.0);
    body_filter.set_sample_rate(sample_rate as f64);
    body_filter.reset();

    let mut noise_filter = dsp::lowpole_hz(2_200.0);
    noise_filter.set_sample_rate(sample_rate as f64);
    noise_filter.reset();

    spec.volume = spec.volume.clamp(0.0, 1.0);
    spec.noise = spec.noise.clamp(0.0, 1.0);

    for i in 0..sample_count {
        let t = if sample_count > 1 {
            i as f32 / (sample_count - 1) as f32
        } else {
            0.0
        };
        let freq = (spec.frequency + (spec.frequency_end - spec.frequency) * t).max(1.0);
        let tone = body_filter.filter_mono(oscillator.filter_mono(freq));
        let dust = noise_filter.filter_mono(noise.get_mono());
        let mut sample = tone * (1.0 - spec.noise) + dust * spec.noise;
        sample *= envelope(i, sample_count, attack, release) * spec.volume;
        let sample = dsp::clamp11(sample);
        frames.push(Frame::new(sample, sample));
    }
    RenderedAudio {
        sample_rate,
        frames,
    }
}

fn envelope(index: usize, length: usize, attack: usize, release: usize) -> f32 {
    if attack > 0 && index < attack {
        return dsp::smooth5(index as f32 / attack as f32);
    }
    if release > 0 && index >= length.saturating_sub(release) {
        return dsp::smooth5((length.saturating_sub(index)) as f32 / release as f32);
    }
    1.0
}

fn render_lofi_theme(spec: &MusicSpec, sample_rate: u32) -> RenderedAudio {
    let bpm = spec.bpm.max(1.0);
    let seconds_per_beat = 60.0 / bpm;
    let total_beats = spec.total_beats.max(1.0);
    let seconds = total_beats * seconds_per_beat;
    let sample_count = (seconds * sample_rate as f32).round() as usize;
    let mut frames = Vec::with_capacity(sample_count);

    let mut drum_noise = dsp::white();
    drum_noise.set_sample_rate(sample_rate as f64);
    drum_noise.reset();
    let mut drum_filter = dsp::lowpole_hz(2_800.0);
    drum_filter.set_sample_rate(sample_rate as f64);
    drum_filter.reset();

    let mut hiss_left = dsp::pink::<f32>();
    hiss_left.set_sample_rate(sample_rate as f64);
    hiss_left.reset();
    let mut hiss_right = dsp::pink::<f32>();
    hiss_right.set_sample_rate(sample_rate as f64);
    hiss_right.reset();

    let cutoff = (700.0 + spec.lowpass_alpha.clamp(0.001, 1.0) * 9_500.0).clamp(500.0, 12_000.0);
    let mut lowpass_left = dsp::lowpole_hz(cutoff);
    lowpass_left.set_sample_rate(sample_rate as f64);
    lowpass_left.reset();
    let mut lowpass_right = dsp::lowpole_hz(cutoff * 0.97);
    lowpass_right.set_sample_rate(sample_rate as f64);
    lowpass_right.reset();

    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let loop_beat = (t / seconds_per_beat).rem_euclid(total_beats);
        let loop_edge = loop_beat.min(total_beats - loop_beat);
        let loop_fade = dsp::smooth5((loop_edge * 3.0).clamp(0.0, 1.0));
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        mix_stereo(
            &mut left,
            &mut right,
            lofi_chord_pad(spec, loop_beat, seconds_per_beat, t),
            -0.10,
        );
        mix_stereo(
            &mut left,
            &mut right,
            note_sequence_voice(
                &spec.lead,
                loop_beat,
                seconds_per_beat,
                spec.root_hz,
                WaveformSpec::Triangle,
                0.5,
                t,
            ) * spec.gains.lead,
            0.12,
        );
        mix_stereo(
            &mut left,
            &mut right,
            lofi_soft_keys(spec, loop_beat, seconds_per_beat, t),
            0.18,
        );
        mix_stereo(
            &mut left,
            &mut right,
            lofi_bass(spec, loop_beat, seconds_per_beat, t),
            -0.04,
        );

        let drum_noise_sample = drum_filter.filter_mono(drum_noise.get_mono());
        mix_stereo(
            &mut left,
            &mut right,
            lofi_dusty_drums(loop_beat, seconds_per_beat, t, drum_noise_sample) * spec.gains.drums,
            0.02,
        );

        left += hiss_left.get_mono() * spec.tape_hiss;
        right += hiss_right.get_mono() * spec.tape_hiss;

        left = lowpass_left.filter_mono(left) * loop_fade * spec.master_gain;
        right = lowpass_right.filter_mono(right) * loop_fade * spec.master_gain;
        frames.push(Frame::new(
            dsp::clamp11(dsp_soft_clip(left)),
            dsp::clamp11(dsp_soft_clip(right)),
        ));
    }

    RenderedAudio {
        sample_rate,
        frames,
    }
}

fn note_sequence_voice(
    notes: &[NoteSpec],
    loop_beat: f32,
    seconds_per_beat: f32,
    root_hz: f32,
    waveform: WaveformSpec,
    duty: f32,
    time_seconds: f32,
) -> f32 {
    let mut sample = 0.0f32;
    for note in notes {
        let end_beat = note.start + note.duration;
        if loop_beat < note.start || loop_beat >= end_beat {
            continue;
        }
        let local_time = (loop_beat - note.start) * seconds_per_beat;
        let duration = note.duration * seconds_per_beat;
        let wow = 1.0 + 0.002 * dsp::sin_hz(0.31, time_seconds);
        let freq = semitone_frequency(root_hz, note.semitone) * wow;
        let rounded = fundsp_wave_at(freq, local_time, waveform, duty) * 0.82
            + dsp::sin_hz(freq, local_time) * 0.18;
        sample += rounded * note_envelope(local_time, duration, 0.045, 0.260) * note.volume;
    }
    sample
}

fn lofi_chord_pad(
    spec: &MusicSpec,
    loop_beat: f32,
    seconds_per_beat: f32,
    time_seconds: f32,
) -> f32 {
    if spec.chords.is_empty() {
        return 0.0;
    }
    let bar = music_bar_index(loop_beat);
    let chord = spec.chords[bar % spec.chords.len()];
    let local_time = beat_in_bar(loop_beat) * seconds_per_beat;
    let duration = 4.0 * seconds_per_beat;
    let mut sample = 0.0f32;

    for (voice, semitone) in chord.iter().enumerate() {
        let detune = 1.0 + (voice as f32 - 1.5) * 0.0015;
        let wow = 1.0 + 0.0025 * dsp::sin_hz(0.09 + voice as f32 * 0.017, time_seconds);
        let freq = semitone_frequency(spec.root_hz, *semitone) * detune * wow;
        let tri = fundsp_wave_at(freq, local_time, WaveformSpec::Triangle, 0.5);
        let sine = dsp::sin_hz(freq, local_time);
        sample += (tri * 0.45 + sine * 0.55) * spec.gains.chord_pad;
    }

    sample * note_envelope(local_time, duration, 0.180, 0.700)
}

fn lofi_soft_keys(
    spec: &MusicSpec,
    loop_beat: f32,
    seconds_per_beat: f32,
    time_seconds: f32,
) -> f32 {
    if spec.chords.is_empty() {
        return 0.0;
    }
    let bar = music_bar_index(loop_beat);
    let chord = spec.chords[bar % spec.chords.len()];
    let half_step = (loop_beat * 2.0).floor() as i32;
    if half_step.rem_euclid(4) != 1 {
        return 0.0;
    }

    let step_start = half_step as f32 * 0.5;
    let local_time = (loop_beat - step_start) * seconds_per_beat;
    let step_index = ((half_step / 2) as usize + bar) % 4;
    let semitone = chord[step_index];
    let freq = semitone_frequency(spec.key_root_hz, semitone + 12);
    let wobble = 1.0 + 0.0030 * dsp::sin_hz(0.65, time_seconds);
    let rounded = fundsp_wave_at(freq * wobble, local_time, WaveformSpec::Triangle, 0.5) * 0.70
        + dsp::sin_hz(freq * wobble, local_time) * 0.30;
    rounded
        * note_envelope(local_time, 0.42 * seconds_per_beat, 0.025, 0.180)
        * spec.gains.soft_keys
}

fn lofi_bass(spec: &MusicSpec, loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    if spec.bass_roots.is_empty() {
        return 0.0;
    }
    let bar = music_bar_index(loop_beat);
    let beat_in_bar = beat_in_bar(loop_beat);
    let beat_floor = beat_in_bar.floor();
    let local_time = (beat_in_bar - beat_floor) * seconds_per_beat;
    let chord_root = spec.bass_roots[bar % spec.bass_roots.len()];
    let semitone = match beat_floor as i32 {
        0 => chord_root,
        1 => chord_root,
        2 => chord_root + 7,
        _ => chord_root,
    };
    let freq = semitone_frequency(spec.bass_root_hz, semitone)
        * (1.0 + 0.0015 * dsp::sin_hz(0.22, time_seconds));
    dsp::sin_hz(freq, local_time)
        * note_envelope(local_time, 0.86 * seconds_per_beat, 0.020, 0.210)
        * spec.gains.bass
}

fn lofi_dusty_drums(loop_beat: f32, seconds_per_beat: f32, time_seconds: f32, noise: f32) -> f32 {
    let beat_floor = loop_beat.floor();
    let beat_frac = loop_beat - beat_floor;
    let beat_in_bar = beat_floor as i32 % 4;
    let bar = music_bar_index(loop_beat);
    let phrase_bar = bar % 16;
    let drums_out = phrase_bar == 15;
    let hat_drop = matches!(phrase_bar, 7 | 14 | 15);
    let mut sample = 0.0f32;

    if !drums_out && beat_frac < 0.20 && beat_in_bar == 0 {
        let local_time = beat_frac * seconds_per_beat;
        let env = (1.0 - beat_frac / 0.20).clamp(0.0, 1.0).powf(2.4);
        let weight = if phrase_bar >= 8 { 0.042 } else { 0.050 };
        sample += dsp::sin_hz(52.0 - 12.0 * beat_frac, local_time) * env * weight;
    }
    if !drums_out && beat_frac < 0.18 && beat_in_bar == 2 {
        let env = (1.0 - beat_frac / 0.18).clamp(0.0, 1.0).powf(2.2);
        let body = dsp::sin_hz(145.0, beat_frac * seconds_per_beat) * env * 0.012;
        let snare_weight = if phrase_bar == 11 { 0.012 } else { 0.018 };
        sample += noise * env * snare_weight + body;
    }
    let quarter_frac = loop_beat.fract();
    if !hat_drop && quarter_frac < 0.10 {
        let env = (1.0 - quarter_frac / 0.10).clamp(0.0, 1.0).powf(2.0);
        let sway = 0.55 + 0.45 * dsp::sin_hz(0.1, time_seconds).abs();
        let long_cycle = 0.70 + 0.30 * dsp::sin_hz(0.025, time_seconds).abs();
        sample += noise * env * 0.0035 * sway * long_cycle;
    }
    sample
}

fn music_bar_index(loop_beat: f32) -> usize {
    (loop_beat / 4.0).floor().max(0.0) as usize
}

fn beat_in_bar(loop_beat: f32) -> f32 {
    loop_beat - music_bar_index(loop_beat) as f32 * 4.0
}

fn semitone_frequency(root_hz: f32, semitone: i32) -> f32 {
    root_hz * 2.0f32.powf(semitone as f32 / 12.0)
}

fn fundsp_wave_at(freq: f32, time_seconds: f32, waveform: WaveformSpec, duty: f32) -> f32 {
    match waveform {
        WaveformSpec::Sine => dsp::sin_hz(freq, time_seconds),
        WaveformSpec::Square => {
            let phase = (freq * time_seconds).fract();
            if phase < duty.clamp(0.05, 0.95) {
                1.0
            } else {
                -1.0
            }
        }
        WaveformSpec::Triangle => dsp::tri_hz(freq, time_seconds),
        WaveformSpec::Saw => 2.0 * (freq * time_seconds).fract() - 1.0,
    }
}

fn note_envelope(local_time: f32, duration: f32, attack: f32, release: f32) -> f32 {
    if duration <= 0.0 || local_time < 0.0 || local_time > duration {
        return 0.0;
    }
    if attack > 0.0 && local_time < attack {
        return dsp::smooth5((local_time / attack).clamp(0.0, 1.0));
    }
    let release_start = (duration - release).max(attack);
    if release > 0.0 && local_time > release_start {
        return dsp::smooth5(((duration - local_time) / release).clamp(0.0, 1.0));
    }
    1.0
}

fn mix_stereo(left: &mut f32, right: &mut f32, sample: f32, pan: f32) {
    let pan = pan.clamp(-1.0, 1.0);
    *left += sample * (1.0 - pan * 0.35);
    *right += sample * (1.0 + pan * 0.35);
}

fn dsp_soft_clip(sample: f32) -> f32 {
    dsp::softsign(sample * 1.25) / dsp::softsign(1.25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::SandboxDataSpec;

    #[test]
    fn sfx_message_maps_to_sound_cue() {
        let pos = ae::Vec2::ZERO;
        assert_eq!(SfxMessage::Jump { pos }.cue(), SoundCue::Jump);
        assert_eq!(SfxMessage::DoubleJump { pos }.cue(), SoundCue::DoubleJump);
        assert_eq!(SfxMessage::Dash { pos }.cue(), SoundCue::Dash);
        assert_eq!(
            SfxMessage::Blink {
                pos,
                precision: false
            }
            .cue(),
            SoundCue::Blink
        );
        assert_eq!(
            SfxMessage::Blink {
                pos,
                precision: true
            }
            .cue(),
            SoundCue::PrecisionBlink
        );
        assert_eq!(SfxMessage::Pogo { pos }.cue(), SoundCue::Pogo);
        assert_eq!(SfxMessage::Slash { pos }.cue(), SoundCue::Slash);
        assert_eq!(SfxMessage::Hit { pos }.cue(), SoundCue::Hit);
        assert_eq!(SfxMessage::Death { pos }.cue(), SoundCue::Death);
        assert_eq!(SfxMessage::Reset { pos }.cue(), SoundCue::Reset);
    }

    #[test]
    fn sfx_message_carries_position() {
        let pos = ae::Vec2::new(120.0, 64.0);
        if let SfxMessage::Hit { pos: at } = (SfxMessage::Hit { pos }) {
            assert_eq!(at, pos);
        } else {
            panic!("variant pattern match failed");
        }
    }

    #[test]
    fn embedded_music_renders_expected_durations() {
        let spec = SandboxDataSpec::load_embedded();
        let sample_rate = 8_000;
        let original = &spec
            .audio
            .track(ORIGINAL_TRACK_ID)
            .expect("original track exists")
            .arrangement;
        let long = &spec
            .audio
            .track("long_lofi_drift")
            .expect("long track exists")
            .arrangement;

        let original_render = render_lofi_theme(original, sample_rate);
        let long_render = render_lofi_theme(long, sample_rate);
        assert!((original_render.duration_seconds() - original.duration_seconds()).abs() < 0.01);
        assert!((long_render.duration_seconds() - long.duration_seconds()).abs() < 0.01);
        assert!(long_render.frames.len() > original_render.frames.len() * 3);
    }

    #[test]
    fn long_track_authors_full_chord_and_bass_phrase() {
        let spec = SandboxDataSpec::load_embedded();
        let long = &spec
            .audio
            .track("long_lofi_drift")
            .expect("long track exists")
            .arrangement;
        assert_eq!(long.chords.len(), long.bar_count());
        assert_eq!(long.bass_roots.len(), long.bar_count());
        assert!(long.chords.windows(2).any(|pair| pair[0] != pair[1]));
        assert!(long.bass_roots.windows(2).any(|pair| pair[0] != pair[1]));
        assert_ne!(long.chords[0], *long.chords.last().unwrap());
        assert_ne!(long.bass_roots[0], *long.bass_roots.last().unwrap());
    }

    #[test]
    fn music_track_order_cycles() {
        let spec = SandboxDataSpec::load_embedded();
        let mut assets = Assets::<KiraAudioSource>::default();
        let library = AudioLibrary::new(&mut assets, &spec.audio);
        assert_eq!(library.track_count(), 2);
        assert_eq!(
            library.next_track_id("long_lofi_drift"),
            Some(ORIGINAL_TRACK_ID)
        );
        assert_eq!(
            library.previous_track_id("long_lofi_drift"),
            Some(ORIGINAL_TRACK_ID)
        );
        assert_eq!(
            library.next_track_id(ORIGINAL_TRACK_ID),
            Some("long_lofi_drift")
        );
    }
}
