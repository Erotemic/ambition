//! ECS audio-environment layer.
//!
//! Gameplay publishes an [`AudioEnvironmentMode`]; this module smooths the mix
//! on wall-clock time. Underwater currently ducks music and SFX volume but does
//! not filter high frequencies because the current `bevy_kira_audio` backend
//! does not expose track effects. Direct-Kira migration sites use
//! `TODO: kira_underwater_filter_backend`.

use bevy::prelude::*;

#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::{AudioChannel, AudioControl};

#[cfg(feature = "audio")]
use ambition_audio::library::{amplitude_to_decibels, MusicChannel, SfxChannel};

/// Coarse classification of the player's acoustic surroundings. The
/// gameplay layer picks one of these; the audio layer is responsible
/// for translating it into a smoothed mix change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioEnvironmentMode {
    /// Open-air mix: no environmental coloration applied.
    #[default]
    Normal,
    /// Submerged. Currently volume-ducked; low-pass filtering requires the
    /// direct-Kira backend described in the module docs.
    Underwater,
}

/// ECS resource describing "what should the world sound like right
/// now". `target` is set by gameplay (typically
/// [`detect_audio_environment`] reading the player's
/// [`ambition_platformer2d_core::BodyEnvironmentContact::water`] cluster field).
/// `wetness` is smoothed toward `target_wetness()` by
/// [`smooth_audio_environment`] using wall-clock dt, so the transition
/// keeps progressing while the world is paused or in bullet-time
/// (audio buses always run on the wall clock — see
/// `PresentationTime::wall_dt` docs).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct AudioEnvironment {
    pub target: AudioEnvironmentMode,
    /// 0.0 = fully dry / open-air, 1.0 = full target effect.
    pub wetness: f32,
    /// Approximate seconds to traverse the full 0→1 range. 0.35 s
    /// lands inside the brief's 200–600 ms band.
    pub transition_secs: f32,
}

impl Default for AudioEnvironment {
    fn default() -> Self {
        Self {
            target: AudioEnvironmentMode::Normal,
            wetness: 0.0,
            transition_secs: 0.35,
        }
    }
}

impl AudioEnvironment {
    pub fn target_wetness(&self) -> f32 {
        match self.target {
            AudioEnvironmentMode::Normal => 0.0,
            AudioEnvironmentMode::Underwater => 1.0,
        }
    }

    /// Deterministically advance `wetness` toward `target_wetness()`.
    ///
    /// Uses a saturating linear step rather than a per-frame
    /// exponential so the transition is frame-rate-independent and
    /// pinnable in unit tests. Stepping with `dt == transition_secs`
    /// reaches the target in exactly one call.
    pub fn advance(&mut self, dt: f32) {
        let target = self.target_wetness();
        if dt <= 0.0 || self.transition_secs <= 0.0 {
            self.wetness = target;
            return;
        }
        let step = (dt / self.transition_secs).clamp(0.0, 1.0);
        let delta = target - self.wetness;
        self.wetness += delta * step;
        if (self.wetness - target).abs() < 1e-4 {
            self.wetness = target;
        }
        self.wetness = self.wetness.clamp(0.0, 1.0);
    }

    /// Multiplier applied on top of `effective_music()` when writing
    /// the music-channel volume. 1.0 = unmodified; lower = ducked.
    /// `0.40` underwater ≈ -8 dB, which gives a clearly "muffled"
    /// impression on top of the existing mix.
    pub fn music_attenuation(&self) -> f32 {
        lerp(1.0, 0.40, self.wetness)
    }

    /// Multiplier applied on top of `effective_sfx()`. SFX is ducked
    /// less aggressively than music so footstep/combat impacts still
    /// read while submerged.
    pub fn sfx_attenuation(&self) -> f32 {
        lerp(1.0, 0.55, self.wetness)
    }
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Drive [`AudioEnvironment::target`] from the primary player's
/// `WaterContact`. The gameplay simulator (`movement::simulation`)
/// is the canonical writer of `water_contact`; this system simply
/// observes it so audio stays in lockstep with the existing source
/// of truth and we never invent a parallel "is underwater" flag.
///
/// `submersion >= 0.5` is the threshold for "head under" — barely
/// touching the surface shouldn't flip the mix, but standing mostly
/// inside the volume should.
#[cfg(feature = "audio")]
pub fn detect_audio_environment(
    mut env: ResMut<AudioEnvironment>,
    primary: Query<
        &ambition_platformer2d_core::BodyEnvironmentContact,
        With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
    >,
) {
    let underwater = primary
        .iter()
        .next()
        .and_then(|env_contact| env_contact.water)
        .is_some_and(|contact| contact.submersion >= 0.5);
    let next = if underwater {
        AudioEnvironmentMode::Underwater
    } else {
        AudioEnvironmentMode::Normal
    };
    if env.target != next {
        env.target = next;
    }
}

/// Smooth `AudioEnvironment::wetness` toward its target using
/// wall-clock dt. Audio buses are intentionally on the wall clock
/// (see `PresentationTime::wall_dt`) so the underwater transition keeps
/// moving when the sim is paused or slowed by bullet-time.
#[cfg(feature = "audio")]
pub fn smooth_audio_environment(
    time: ambition_time::PresentationTime,
    mut env: ResMut<AudioEnvironment>,
) {
    env.advance(time.wall_dt());
}

/// Single writer for the music + SFX channel volumes. Combines user
/// mixer settings with the smoothed [`AudioEnvironment`] attenuation
/// so:
///
/// * changing volume sliders while submerged still works (the
///   re-compose runs whenever either input changes);
/// * the gameplay layer never reaches into Kira directly;
/// * one cache key (`Local<Option<(...)>>`) gates the per-frame
///   write — we don't spam `set_volume` once steady-state is reached.
///
/// `TODO: kira_underwater_filter_backend` — the volume attenuation
/// computed here is the placeholder until the bevy_kira_audio
/// wrapper grows track-level effect access (or we swap to a direct-
/// Kira layer per `docs/systems/audio-underwater.md`). The real underwater
/// effect should be a Kira `FilterBuilder` (LowPass, cutoff tweened
/// from ~20 kHz to ~800 Hz against `wetness`), not a level reduction.
/// This function is the exact swap point: replace the
/// `effective_music * music_attenuation` arithmetic with a call into
/// the filter handle, and leave the user-mixer composition intact.
#[cfg(feature = "audio")]
pub fn apply_audio_environment(
    settings: Res<ambition_persistence::settings::UserSettings>,
    env: Res<AudioEnvironment>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    mut last: Local<Option<(ambition_persistence::settings::AudioSettings, f32)>>,
) {
    let current_settings = settings.audio;
    let wetness = env.wetness;
    let key = (current_settings, quantize_wetness(wetness));
    if last.as_ref().map(|(a, w)| (*a, *w)) == Some(key) {
        return;
    }
    let music_db =
        amplitude_to_decibels(current_settings.effective_music() * env.music_attenuation());
    let sfx_db = amplitude_to_decibels(current_settings.effective_sfx() * env.sfx_attenuation());
    music_channel.set_volume(music_db);
    sfx_channel.set_volume(sfx_db);
    *last = Some(key);
}

/// 64-step granularity is finer than the ear can resolve over an 8 dB range while still cutting
/// roughly two orders of magnitude of redundant writes.
#[cfg_attr(not(feature = "audio"), allow(dead_code))]
#[inline]
fn quantize_wetness(wetness: f32) -> f32 {
    (wetness.clamp(0.0, 1.0) * 64.0).round() / 64.0
}

#[cfg(test)]
mod tests;
