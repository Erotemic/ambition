//! Procedural audio for Ambition sandbox feedback and music.
//!
//! The current implementation synthesizes tiny WAV files into in-memory Bevy
//! `AudioSource` handles. This keeps the project assetless while leaving a clear
//! seam for a future Kira/CPAL-backed `ambition_audio` crate. The background
//! track is intentionally chip-style rather than a droning ambience pad.

use bevy::audio::AudioSource;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Resource)]
pub struct SoundBank {
    jump: Handle<AudioSource>,
    double_jump: Handle<AudioSource>,
    dash: Handle<AudioSource>,
    blink: Handle<AudioSource>,
    precision_blink: Handle<AudioSource>,
    slash: Handle<AudioSource>,
    hit: Handle<AudioSource>,
    pogo: Handle<AudioSource>,
    reset: Handle<AudioSource>,
    death: Handle<AudioSource>,
    respawn: Handle<AudioSource>,
    ambience: Handle<AudioSource>,
}

impl SoundBank {
    pub fn new(audio_sources: &mut Assets<AudioSource>) -> Self {
        let mut add = |spec: SynthSpec| audio_sources.add(AudioSource { bytes: synth_wav_bytes(spec, 44_100).into() });
        Self {
            jump: add(SynthSpec::jump()),
            double_jump: add(SynthSpec::double_jump()),
            dash: add(SynthSpec::dash()),
            blink: add(SynthSpec::blink()),
            precision_blink: add(SynthSpec::precision_blink()),
            slash: add(SynthSpec::slash()),
            hit: add(SynthSpec::hit()),
            pogo: add(SynthSpec::pogo()),
            reset: add(SynthSpec::reset()),
            death: add(SynthSpec::death()),
            respawn: add(SynthSpec::respawn()),
            ambience: audio_sources.add(AudioSource { bytes: synth_retro_theme_wav_bytes(44_100).into() }),
        }
    }

    pub fn ambience(&self) -> Handle<AudioSource> {
        self.ambience.clone()
    }

    pub fn get(&self, cue: SoundCue) -> Handle<AudioSource> {
        match cue {
            SoundCue::Jump => self.jump.clone(),
            SoundCue::DoubleJump => self.double_jump.clone(),
            SoundCue::Dash => self.dash.clone(),
            SoundCue::Blink => self.blink.clone(),
            SoundCue::PrecisionBlink => self.precision_blink.clone(),
            SoundCue::Slash => self.slash.clone(),
            SoundCue::Hit => self.hit.clone(),
            SoundCue::Pogo => self.pogo.clone(),
            SoundCue::Reset => self.reset.clone(),
            SoundCue::Death => self.death.clone(),
            SoundCue::Respawn => self.respawn.clone(),
        }
    }
}

pub fn play_sound(commands: &mut Commands, bank: &SoundBank, cue: SoundCue) {
    commands.spawn((
        AudioPlayer::new(bank.get(cue)),
        PlaybackSettings::DESPAWN,
    ));
}

/// Start the generated background music loop.
///
/// This intentionally uses Bevy's built-in audio for now. Once we need
/// cross-fades, parameter automation, or layered adaptive music, this is the
/// seam where Kira should replace the current simple playback path. The public
/// function keeps its original name so the call sites do not churn yet.
pub fn play_ambience(commands: &mut Commands, bank: &SoundBank) {
    commands.spawn((
        AudioPlayer::new(bank.ambience()),
        PlaybackSettings::LOOP,
    ));
}

#[derive(Clone, Copy, Debug)]
enum Waveform {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug)]
struct SynthSpec {
    waveform: Waveform,
    frequency: f32,
    frequency_end: f32,
    duration: f32,
    volume: f32,
    attack: f32,
    release: f32,
    noise: f32,
}

impl SynthSpec {
    fn jump() -> Self {
        Self::tone(Waveform::Sine, 460.0, 720.0, 0.085, 0.22)
    }
    fn double_jump() -> Self {
        Self::tone(Waveform::Triangle, 520.0, 940.0, 0.115, 0.22)
    }
    fn dash() -> Self {
        Self::tone(Waveform::Saw, 260.0, 110.0, 0.105, 0.18)
    }
    fn blink() -> Self {
        Self::tone(Waveform::Triangle, 740.0, 260.0, 0.090, 0.18)
    }
    fn precision_blink() -> Self {
        Self {
            noise: 0.08,
            ..Self::tone(Waveform::Sine, 880.0, 180.0, 0.160, 0.20)
        }
    }
    fn slash() -> Self {
        Self::tone(Waveform::Square, 620.0, 340.0, 0.075, 0.16)
    }
    fn hit() -> Self {
        Self {
            noise: 0.44,
            ..Self::tone(Waveform::Triangle, 220.0, 88.0, 0.105, 0.26)
        }
    }
    fn pogo() -> Self {
        Self::tone(Waveform::Sine, 360.0, 880.0, 0.105, 0.22)
    }
    fn reset() -> Self {
        Self::tone(Waveform::Sine, 160.0, 90.0, 0.150, 0.16)
    }
    fn death() -> Self {
        Self {
            noise: 0.18,
            ..Self::tone(Waveform::Saw, 140.0, 48.0, 0.220, 0.24)
        }
    }
    fn respawn() -> Self {
        Self::tone(Waveform::Triangle, 440.0, 660.0, 0.145, 0.20)
    }
    fn tone(waveform: Waveform, frequency: f32, frequency_end: f32, duration: f32, volume: f32) -> Self {
        Self {
            waveform,
            frequency,
            frequency_end,
            duration,
            volume,
            attack: 0.003,
            release: 0.045,
            noise: 0.0,
        }
    }
}

fn sample_wave(phase: f32, waveform: Waveform) -> f32 {
    let p = phase.fract();
    match waveform {
        Waveform::Sine => (p * TAU).sin(),
        Waveform::Square => {
            if p < 0.5 { 1.0 } else { -1.0 }
        }
        Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Waveform::Saw => 2.0 * p - 1.0,
    }
}

fn envelope(index: usize, length: usize, attack: usize, release: usize) -> f32 {
    if attack > 0 && index < attack {
        return index as f32 / attack as f32;
    }
    if release > 0 && index >= length.saturating_sub(release) {
        return (length.saturating_sub(index)) as f32 / release as f32;
    }
    1.0
}

fn synth_wav_bytes(spec: SynthSpec, sample_rate: u32) -> Vec<u8> {
    let sample_count = ((spec.duration * sample_rate as f32).max(1.0)) as usize;
    let attack = (spec.attack * sample_rate as f32) as usize;
    let release = (spec.release * sample_rate as f32) as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut phase = 0.0f32;
    let mut noise_state = 0x1234_5678u32;
    for i in 0..sample_count {
        let t = if sample_count > 1 {
            i as f32 / (sample_count - 1) as f32
        } else {
            0.0
        };
        let freq = spec.frequency + (spec.frequency_end - spec.frequency) * t;
        phase += freq / sample_rate as f32;
        let mut sample = sample_wave(phase, spec.waveform);
        if spec.noise > 0.0 {
            noise_state = noise_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (((noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0;
            sample = sample * (1.0 - spec.noise) + n * spec.noise;
        }
        sample *= envelope(i, sample_count, attack, release) * spec.volume;
        let v = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        pcm.push(v);
        pcm.push(v);
    }
    let data_bytes = (pcm.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2 * 2).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in pcm {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn synth_retro_theme_wav_bytes(sample_rate: u32) -> Vec<u8> {
    let bpm = 96.0f32;
    let seconds_per_beat = 60.0 / bpm;
    let total_beats = 32.0f32;
    let seconds = total_beats * seconds_per_beat;
    let sample_count = (seconds * sample_rate as f32).round() as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut noise_state = 0x45d9_f3bu32;

    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let loop_beat = (t / seconds_per_beat).rem_euclid(total_beats);
        let seam_fade = (loop_beat.min(total_beats - loop_beat) * 8.0).clamp(0.0, 1.0);
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        mix_stereo(
            &mut left,
            &mut right,
            note_sequence_voice(&RETRO_LEAD, loop_beat, seconds_per_beat, 523.25, Waveform::Square, 0.375, t),
            -0.18,
        );
        mix_stereo(
            &mut left,
            &mut right,
            retro_arpeggio(loop_beat, seconds_per_beat, t),
            0.22,
        );
        mix_stereo(
            &mut left,
            &mut right,
            retro_bass(loop_beat, seconds_per_beat),
            0.0,
        );
        mix_stereo(
            &mut left,
            &mut right,
            retro_soft_drums(loop_beat, seconds_per_beat, t, &mut noise_state),
            0.08,
        );

        // Leave plenty of headroom; the SFX should still read clearly over music.
        left = (left * seam_fade * 0.72).clamp(-1.0, 1.0);
        right = (right * seam_fade * 0.72).clamp(-1.0, 1.0);
        pcm.push((left * i16::MAX as f32) as i16);
        pcm.push((right * i16::MAX as f32) as i16);
    }

    wav_bytes_from_stereo_i16(&pcm, sample_rate)
}

const RETRO_LEAD: [(f32, f32, i32, f32); 31] = [
    (0.00, 0.75, 0, 0.090),
    (1.00, 0.75, 4, 0.082),
    (2.00, 1.50, 7, 0.086),
    (4.00, 0.75, 9, 0.084),
    (5.00, 0.75, 7, 0.080),
    (6.00, 1.50, 4, 0.082),
    (8.00, 0.50, 2, 0.078),
    (8.75, 0.50, 4, 0.078),
    (9.50, 1.00, 7, 0.084),
    (11.00, 0.75, 12, 0.088),
    (12.00, 0.75, 11, 0.082),
    (13.00, 0.75, 7, 0.080),
    (14.00, 1.50, 4, 0.080),
    (16.00, 0.75, 7, 0.086),
    (17.00, 0.75, 9, 0.086),
    (18.00, 1.50, 12, 0.090),
    (20.00, 0.50, 14, 0.082),
    (20.75, 0.50, 12, 0.082),
    (21.50, 1.00, 9, 0.084),
    (23.00, 0.75, 7, 0.080),
    (24.00, 0.75, 4, 0.082),
    (25.00, 0.75, 7, 0.084),
    (26.00, 0.75, 9, 0.086),
    (27.00, 0.75, 12, 0.088),
    (28.00, 0.50, 11, 0.082),
    (28.75, 0.50, 9, 0.082),
    (29.50, 0.50, 7, 0.080),
    (30.25, 0.50, 4, 0.078),
    (31.00, 0.50, 2, 0.074),
    (31.50, 0.25, 0, 0.070),
    (31.75, 0.20, -12, 0.060),
];

const RETRO_CHORDS: [[i32; 4]; 8] = [
    [0, 4, 7, 12],
    [-5, -1, 2, 7],
    [-3, 0, 4, 9],
    [-7, -3, 0, 5],
    [0, 4, 7, 11],
    [2, 5, 9, 14],
    [-5, -1, 2, 7],
    [-7, -3, 0, 7],
];

const RETRO_BASS_ROOTS: [i32; 8] = [0, -5, -3, -7, 0, 2, -5, -7];

fn note_sequence_voice(
    events: &[(f32, f32, i32, f32)],
    loop_beat: f32,
    seconds_per_beat: f32,
    root_hz: f32,
    waveform: Waveform,
    duty: f32,
    time_seconds: f32,
) -> f32 {
    for event in events {
        let (start, duration_beats, semitone, volume) = *event;
        if loop_beat >= start && loop_beat < start + duration_beats {
            let local_time = (loop_beat - start) * seconds_per_beat;
            let duration = duration_beats * seconds_per_beat;
            let freq = semitone_frequency(root_hz, semitone);
            let vibrato = 1.0 + 0.0025 * (TAU * 5.4 * time_seconds).sin();
            let phase = freq * local_time * vibrato;
            return chip_wave(phase, waveform, duty) * note_envelope(local_time, duration, 0.006, 0.070) * volume;
        }
    }
    0.0
}

fn retro_arpeggio(loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    let bar = ((loop_beat / 4.0).floor() as usize).min(RETRO_CHORDS.len() - 1);
    let step = (loop_beat * 2.0).floor();
    let step_index = step as usize % 4;
    let local_time = (loop_beat - step * 0.5) * seconds_per_beat;
    let freq = semitone_frequency(523.25, RETRO_CHORDS[bar][step_index]);
    let shimmer = 1.0 + 0.0015 * (TAU * 6.0 * time_seconds).sin();
    chip_wave(freq * local_time * shimmer, Waveform::Square, 0.25)
        * note_envelope(local_time, 0.5 * seconds_per_beat, 0.003, 0.050)
        * 0.036
}

fn retro_bass(loop_beat: f32, seconds_per_beat: f32) -> f32 {
    let bar = ((loop_beat / 4.0).floor() as usize).min(RETRO_BASS_ROOTS.len() - 1);
    let beat_in_bar = loop_beat - (bar as f32 * 4.0);
    let beat_floor = beat_in_bar.floor();
    let local_time = (beat_in_bar - beat_floor) * seconds_per_beat;
    let chord_root = RETRO_BASS_ROOTS[bar];
    let semitone = if beat_floor as i32 % 2 == 0 { chord_root } else { chord_root + 7 };
    chip_wave(semitone_frequency(130.81, semitone) * local_time, Waveform::Triangle, 0.5)
        * note_envelope(local_time, 0.92 * seconds_per_beat, 0.004, 0.090)
        * 0.075
}

fn retro_soft_drums(
    loop_beat: f32,
    seconds_per_beat: f32,
    time_seconds: f32,
    noise_state: &mut u32,
) -> f32 {
    *noise_state = (*noise_state).wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let noise = (((*noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0;
    let beat_floor = loop_beat.floor();
    let beat_frac = loop_beat - beat_floor;
    let beat_in_bar = beat_floor as i32 % 4;
    let mut sample = 0.0f32;

    if beat_frac < 0.16 && (beat_in_bar == 0 || beat_in_bar == 2) {
        let local_time = beat_frac * seconds_per_beat;
        let env = (1.0 - beat_frac / 0.16).clamp(0.0, 1.0).powf(2.0);
        let freq = if beat_in_bar == 0 { 78.0 } else { 62.0 };
        sample += (TAU * (freq - 18.0 * beat_frac) * local_time).sin() * env * 0.040;
    }

    if beat_frac < 0.13 && beat_in_bar == 2 {
        let env = (1.0 - beat_frac / 0.13).clamp(0.0, 1.0).powf(2.0);
        sample += noise * env * 0.030;
    }

    let eighth_frac = (loop_beat * 2.0).fract();
    if eighth_frac < 0.07 {
        let env = (1.0 - eighth_frac / 0.07).clamp(0.0, 1.0).powf(2.0);
        // Slight periodic lift keeps the hat from becoming a harsh metronome.
        let sway = 0.65 + 0.35 * (TAU * time_seconds / 8.0).sin().abs();
        sample += noise * env * 0.010 * sway;
    }

    sample
}

fn semitone_frequency(root_hz: f32, semitone: i32) -> f32 {
    root_hz * 2.0f32.powf(semitone as f32 / 12.0)
}

fn chip_wave(phase: f32, waveform: Waveform, duty: f32) -> f32 {
    let p = phase.fract();
    match waveform {
        Waveform::Sine => (p * TAU).sin(),
        Waveform::Square => {
            if p < duty.clamp(0.05, 0.95) { 1.0 } else { -1.0 }
        }
        Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Waveform::Saw => 2.0 * p - 1.0,
    }
}

fn note_envelope(local_time: f32, duration: f32, attack: f32, release: f32) -> f32 {
    if duration <= 0.0 || local_time < 0.0 || local_time > duration {
        return 0.0;
    }
    if attack > 0.0 && local_time < attack {
        return (local_time / attack).clamp(0.0, 1.0);
    }
    let release_start = (duration - release).max(attack);
    if release > 0.0 && local_time > release_start {
        return ((duration - local_time) / release).clamp(0.0, 1.0);
    }
    1.0
}

fn mix_stereo(left: &mut f32, right: &mut f32, sample: f32, pan: f32) {
    let pan = pan.clamp(-1.0, 1.0);
    *left += sample * (1.0 - pan * 0.35);
    *right += sample * (1.0 + pan * 0.35);
}

fn wav_bytes_from_stereo_i16(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_bytes = (pcm.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2 * 2).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in pcm {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
