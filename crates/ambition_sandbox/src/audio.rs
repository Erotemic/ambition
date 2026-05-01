//! Procedural audio for Ambition sandbox feedback and music.
//!
//! The synthesizer still produces in-memory WAV assets at startup, but the
//! actual cue frequencies, envelopes, and lo-fi music pattern now live in the
//! RON sandbox manifest instead of being hard-coded in this module.

use std::f32::consts::TAU;

use bevy::audio::AudioSource;
use bevy::prelude::*;

use crate::data::{AudioSpec, MusicSpec, NoteSpec, SfxSpec, SoundCueKey, WaveformSpec};

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
    pub fn new(audio_sources: &mut Assets<AudioSource>, spec: &AudioSpec) -> Self {
        let sample_rate = spec.sample_rate.max(8_000);
        let mut add = |cue: SoundCue| {
            let sfx = find_sfx(spec, cue);
            audio_sources.add(AudioSource { bytes: synth_wav_bytes(sfx, sample_rate).into() })
        };
        Self {
            jump: add(SoundCue::Jump),
            double_jump: add(SoundCue::DoubleJump),
            dash: add(SoundCue::Dash),
            blink: add(SoundCue::Blink),
            precision_blink: add(SoundCue::PrecisionBlink),
            slash: add(SoundCue::Slash),
            hit: add(SoundCue::Hit),
            pogo: add(SoundCue::Pogo),
            reset: add(SoundCue::Reset),
            death: add(SoundCue::Death),
            respawn: add(SoundCue::Respawn),
            ambience: audio_sources.add(AudioSource { bytes: synth_lofi_theme_wav_bytes(&spec.music, sample_rate).into() }),
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

fn find_sfx(spec: &AudioSpec, cue: SoundCue) -> SfxSpec {
    let key = SoundCueKey::from(cue);
    spec.sfx.iter().copied().find(|candidate| candidate.cue == key).unwrap_or_else(|| fallback_sfx(key))
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

pub fn play_sound(commands: &mut Commands, bank: &SoundBank, cue: SoundCue) {
    commands.spawn((AudioPlayer::new(bank.get(cue)), PlaybackSettings::DESPAWN));
}

/// Start the generated background music loop.
pub fn play_ambience(commands: &mut Commands, bank: &SoundBank) {
    commands.spawn((AudioPlayer::new(bank.ambience()), PlaybackSettings::LOOP));
}

fn sample_wave(phase: f32, waveform: WaveformSpec) -> f32 {
    let p = phase.fract();
    match waveform {
        WaveformSpec::Sine => (p * TAU).sin(),
        WaveformSpec::Square => if p < 0.5 { 1.0 } else { -1.0 },
        WaveformSpec::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        WaveformSpec::Saw => 2.0 * p - 1.0,
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

fn synth_wav_bytes(spec: SfxSpec, sample_rate: u32) -> Vec<u8> {
    let sample_count = ((spec.duration * sample_rate as f32).max(1.0)) as usize;
    let attack = (spec.attack * sample_rate as f32) as usize;
    let release = (spec.release * sample_rate as f32) as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut phase = 0.0f32;
    let mut noise_state = 0x1234_5678u32;
    for i in 0..sample_count {
        let t = if sample_count > 1 { i as f32 / (sample_count - 1) as f32 } else { 0.0 };
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
    wav_bytes_from_stereo_i16(&pcm, sample_rate)
}

fn synth_lofi_theme_wav_bytes(spec: &MusicSpec, sample_rate: u32) -> Vec<u8> {
    let bpm = spec.bpm.max(1.0);
    let seconds_per_beat = 60.0 / bpm;
    let total_beats = spec.total_beats.max(1.0);
    let seconds = total_beats * seconds_per_beat;
    let sample_count = (seconds * sample_rate as f32).round() as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut noise_state = 0x45d9_f3bu32;
    let mut lowpass_left = 0.0f32;
    let mut lowpass_right = 0.0f32;

    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let loop_beat = (t / seconds_per_beat).rem_euclid(total_beats);
        let seam_fade = (loop_beat.min(total_beats - loop_beat) * 3.0).clamp(0.0, 1.0);
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        mix_stereo(&mut left, &mut right, lofi_chord_pad(spec, loop_beat, seconds_per_beat, t), -0.10);
        mix_stereo(&mut left, &mut right, note_sequence_voice(&spec.lead, loop_beat, seconds_per_beat, spec.root_hz, WaveformSpec::Triangle, 0.5, t) * spec.gains.lead, 0.12);
        mix_stereo(&mut left, &mut right, lofi_soft_keys(spec, loop_beat, seconds_per_beat, t), 0.18);
        mix_stereo(&mut left, &mut right, lofi_bass(spec, loop_beat, seconds_per_beat, t), -0.04);
        mix_stereo(&mut left, &mut right, lofi_dusty_drums(loop_beat, seconds_per_beat, t, &mut noise_state) * spec.gains.drums, 0.02);

        left += tape_hiss(&mut noise_state) * spec.tape_hiss;
        right += tape_hiss(&mut noise_state) * spec.tape_hiss;

        let alpha = spec.lowpass_alpha.clamp(0.001, 1.0);
        lowpass_left += alpha * (left - lowpass_left);
        lowpass_right += alpha * (right - lowpass_right);
        left = soft_clip(lowpass_left * seam_fade * spec.master_gain).clamp(-1.0, 1.0);
        right = soft_clip(lowpass_right * seam_fade * spec.master_gain).clamp(-1.0, 1.0);
        pcm.push((left * i16::MAX as f32) as i16);
        pcm.push((right * i16::MAX as f32) as i16);
    }

    wav_bytes_from_stereo_i16(&pcm, sample_rate)
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
        let wow = 1.0 + 0.002 * (TAU * 0.31 * time_seconds).sin();
        let freq = semitone_frequency(root_hz, note.semitone) * wow;
        let phase = freq * local_time;
        let rounded = chip_wave(phase, waveform, duty) * 0.82 + (phase.fract() * TAU).sin() * 0.18;
        sample += rounded * note_envelope(local_time, duration, 0.045, 0.260) * note.volume;
    }
    sample
}

fn lofi_chord_pad(spec: &MusicSpec, loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    if spec.chords.is_empty() {
        return 0.0;
    }
    let bar = ((loop_beat / 4.0).floor() as usize).min(spec.chords.len() - 1);
    let local_time = (loop_beat - (bar as f32 * 4.0)) * seconds_per_beat;
    let duration = 4.0 * seconds_per_beat;
    let mut sample = 0.0f32;

    for (voice, semitone) in spec.chords[bar].iter().enumerate() {
        let detune = 1.0 + (voice as f32 - 1.5) * 0.0015;
        let wow = 1.0 + 0.0025 * (TAU * (0.09 + voice as f32 * 0.017) * time_seconds).sin();
        let freq = semitone_frequency(spec.root_hz, *semitone) * detune * wow;
        let phase = freq * local_time;
        let tri = chip_wave(phase, WaveformSpec::Triangle, 0.5);
        let sine = (phase.fract() * TAU).sin();
        sample += (tri * 0.45 + sine * 0.55) * spec.gains.chord_pad;
    }

    sample * note_envelope(local_time, duration, 0.180, 0.700)
}

fn lofi_soft_keys(spec: &MusicSpec, loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    if spec.chords.is_empty() {
        return 0.0;
    }
    let bar = ((loop_beat / 4.0).floor() as usize).min(spec.chords.len() - 1);
    let half_step = (loop_beat * 2.0).floor() as i32;
    if half_step.rem_euclid(4) != 1 {
        return 0.0;
    }

    let step_start = half_step as f32 * 0.5;
    let local_time = (loop_beat - step_start) * seconds_per_beat;
    let step_index = ((half_step / 2) as usize + bar) % 4;
    let semitone = spec.chords[bar][step_index];
    let freq = semitone_frequency(spec.key_root_hz, semitone + 12);
    let wobble = 1.0 + 0.0030 * (TAU * 0.65 * time_seconds).sin();
    let phase = freq * local_time * wobble;
    let rounded = chip_wave(phase, WaveformSpec::Triangle, 0.5) * 0.70 + (phase.fract() * TAU).sin() * 0.30;
    rounded * note_envelope(local_time, 0.42 * seconds_per_beat, 0.025, 0.180) * spec.gains.soft_keys
}

fn lofi_bass(spec: &MusicSpec, loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    if spec.bass_roots.is_empty() {
        return 0.0;
    }
    let bar = ((loop_beat / 4.0).floor() as usize).min(spec.bass_roots.len() - 1);
    let beat_in_bar = loop_beat - (bar as f32 * 4.0);
    let beat_floor = beat_in_bar.floor();
    let local_time = (beat_in_bar - beat_floor) * seconds_per_beat;
    let chord_root = spec.bass_roots[bar];
    let semitone = match beat_floor as i32 {
        0 => chord_root,
        1 => chord_root,
        2 => chord_root + 7,
        _ => chord_root,
    };
    let freq = semitone_frequency(spec.bass_root_hz, semitone) * (1.0 + 0.0015 * (TAU * 0.22 * time_seconds).sin());
    (freq * local_time * TAU).sin()
        * note_envelope(local_time, 0.86 * seconds_per_beat, 0.020, 0.210)
        * spec.gains.bass
}

fn lofi_dusty_drums(loop_beat: f32, seconds_per_beat: f32, time_seconds: f32, noise_state: &mut u32) -> f32 {
    *noise_state = (*noise_state).wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let noise = (((*noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0;
    let beat_floor = loop_beat.floor();
    let beat_frac = loop_beat - beat_floor;
    let beat_in_bar = beat_floor as i32 % 4;
    let mut sample = 0.0f32;

    if beat_frac < 0.20 && beat_in_bar == 0 {
        let local_time = beat_frac * seconds_per_beat;
        let env = (1.0 - beat_frac / 0.20).clamp(0.0, 1.0).powf(2.4);
        sample += (TAU * (52.0 - 12.0 * beat_frac) * local_time).sin() * env * 0.050;
    }
    if beat_frac < 0.18 && beat_in_bar == 2 {
        let env = (1.0 - beat_frac / 0.18).clamp(0.0, 1.0).powf(2.2);
        let body = (TAU * 145.0 * beat_frac * seconds_per_beat).sin() * env * 0.012;
        sample += noise * env * 0.018 + body;
    }
    let quarter_frac = loop_beat.fract();
    if quarter_frac < 0.10 {
        let env = (1.0 - quarter_frac / 0.10).clamp(0.0, 1.0).powf(2.0);
        let sway = 0.55 + 0.45 * (TAU * time_seconds / 10.0).sin().abs();
        sample += noise * env * 0.0035 * sway;
    }
    sample
}

fn tape_hiss(noise_state: &mut u32) -> f32 {
    *noise_state = (*noise_state).wrapping_mul(1_103_515_245).wrapping_add(12_345);
    (((*noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0
}

fn soft_clip(sample: f32) -> f32 {
    sample / (1.0 + sample.abs() * 0.35)
}

fn semitone_frequency(root_hz: f32, semitone: i32) -> f32 {
    root_hz * 2.0f32.powf(semitone as f32 / 12.0)
}

fn chip_wave(phase: f32, waveform: WaveformSpec, duty: f32) -> f32 {
    let p = phase.fract();
    match waveform {
        WaveformSpec::Sine => (p * TAU).sin(),
        WaveformSpec::Square => if p < duty.clamp(0.05, 0.95) { 1.0 } else { -1.0 },
        WaveformSpec::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        WaveformSpec::Saw => 2.0 * p - 1.0,
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
