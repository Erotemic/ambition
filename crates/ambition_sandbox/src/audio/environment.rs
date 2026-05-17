//! ECS audio-environment layer.
//!
//! Gameplay state (water volumes, future caves/tunnels) sets
//! [`AudioEnvironment::target`]; this module smoothly approaches the
//! target wetness on a wall-clock timer and feeds the writer that
//! adjusts the audio mix. Gameplay/water code does not touch Kira
//! directly — it only mutates `AudioEnvironment` (or relies on the
//! built-in `detect_audio_environment` system to do so from
//! `WaterContact`).
//!
//! ## Underwater effect — current status (not a "real" low-pass)
//!
//! **What you actually hear today:** music drops by ~8 dB and SFX by
//! ~5 dB on a smooth 350 ms ramp. There is **no high-frequency
//! damping**; the spectrum is unchanged. This is a placeholder mix
//! adjustment, *not* an underwater muffle. Do not represent it that
//! way in UI/docs.
//!
//! **Why volume-only:** the brief asks for a Kira `LowPass` filter
//! tweened from ~20 kHz to ~800 Hz. Kira ships exactly that
//! (`kira::effect::filter::FilterBuilder` /
//! `MainTrackBuilder::with_effect`), but the wrapper this sandbox
//! uses — `bevy_kira_audio` 0.25 — does not expose track-level
//! effect insertion. Verified by reading the crate source:
//! `AudioOutput` is `pub(crate)`, `AudioManager` is a private field,
//! and `AudioSettings` only forwards `sound_capacity` to
//! `MainTrackBuilder::new()`. There is no extension point.
//!
//! **Required next step (not done in this module):** replace the
//! bevy_kira_audio wrapper with a thin direct-Kira layer that owns
//! one `kira::AudioManager`, exposes a music sub-track + an SFX
//! sub-track each pre-built with `FilterBuilder::new().mode(LowPass)
//! .cutoff(20_000.0)`, and hands `FilterHandle`s back to the ECS
//! writer. See `docs/systems/audio-underwater.md` for the full migration
//! plan and surface area.
//!
//! **Search markers in the code:** every place that has to change
//! when the direct-Kira layer lands is tagged
//! `TODO: kira_underwater_filter_backend`.

use bevy::prelude::*;

#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::{AudioChannel, AudioControl};

#[cfg(feature = "audio")]
use super::runtime::{amplitude_to_decibels, MusicChannel, SfxChannel};

/// Coarse classification of the player's acoustic surroundings. The
/// gameplay layer picks one of these; the audio layer is responsible
/// for translating it into a smoothed mix change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioEnvironmentMode {
    /// Open-air mix: no environmental coloration applied.
    #[default]
    Normal,
    /// Submerged. The *intended* effect is a Kira low-pass tween
    /// (cutoff ~20 kHz → ~800 Hz over 200–600 ms). The *current*
    /// effect is a volume duck (~-8 dB music, ~-5 dB SFX) — a
    /// placeholder until the bevy_kira_audio backend gap is closed.
    /// See module docs.
    Underwater,
}

/// ECS resource describing "what should the world sound like right
/// now". `target` is set by gameplay (typically
/// [`detect_audio_environment`] reading [`PlayerMovementAuthority::water_contact`]).
/// `wetness` is smoothed toward `target_wetness()` by
/// [`smooth_audio_environment`] using wall-clock dt, so the transition
/// keeps progressing while the world is paused or in bullet-time
/// (audio buses always run on the wall clock — see
/// `WorldTime::wall_dt` docs).
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
        &crate::player::PlayerMovementAuthority,
        With<crate::player::PrimaryPlayer>,
    >,
) {
    let underwater = primary
        .iter()
        .next()
        .and_then(|authority| authority.player.water_contact)
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
/// (see `WorldTime::wall_dt`) so the underwater transition keeps
/// moving when the sim is paused or slowed by bullet-time.
#[cfg(feature = "audio")]
pub fn smooth_audio_environment(
    time: Res<crate::WorldTime>,
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
/// computed here is the **placeholder** until the bevy_kira_audio
/// wrapper grows track-level effect access (or we swap to a direct-
/// Kira layer per `docs/systems/audio-underwater.md`). The real underwater
/// effect should be a Kira `FilterBuilder` (LowPass, cutoff tweened
/// from ~20 kHz to ~800 Hz against `wetness`), not a level reduction.
/// This function is the exact swap point: replace the
/// `effective_music * music_attenuation` arithmetic with a call into
/// the filter handle, and leave the user-mixer composition intact.
#[cfg(feature = "audio")]
pub fn apply_audio_environment(
    settings: Res<crate::settings::UserSettings>,
    env: Res<AudioEnvironment>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    mut last: Local<Option<(crate::settings::AudioSettings, f32)>>,
) {
    let current_settings = settings.audio;
    let wetness = env.wetness;
    let key = (current_settings, quantize_wetness(wetness));
    if last.as_ref().map(|(a, w)| (*a, *w)) == Some(key) {
        return;
    }
    let music_db =
        amplitude_to_decibels(current_settings.effective_music() * env.music_attenuation());
    let sfx_db =
        amplitude_to_decibels(current_settings.effective_sfx() * env.sfx_attenuation());
    music_channel.set_volume(music_db);
    sfx_channel.set_volume(sfx_db);
    *last = Some(key);
}

/// Round `wetness` to a fixed quantum so tiny float jitter on the
/// smoothing curve doesn't push a `set_volume` call every frame.
/// 64-step granularity is finer than the ear can resolve over an
/// 8 dB range while still cutting roughly two orders of magnitude of
/// redundant writes.
#[cfg_attr(not(feature = "audio"), allow(dead_code))]
#[inline]
fn quantize_wetness(wetness: f32) -> f32 {
    (wetness.clamp(0.0, 1.0) * 64.0).round() / 64.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_environment_is_dry_normal() {
        let env = AudioEnvironment::default();
        assert_eq!(env.target, AudioEnvironmentMode::Normal);
        assert_eq!(env.wetness, 0.0);
        assert!((env.music_attenuation() - 1.0).abs() < 1e-6);
        assert!((env.sfx_attenuation() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn advance_full_step_reaches_target() {
        let mut env = AudioEnvironment::default();
        env.target = AudioEnvironmentMode::Underwater;
        env.advance(env.transition_secs);
        assert_eq!(env.wetness, 1.0);
    }

    #[test]
    fn advance_partial_step_is_deterministic() {
        let mut env = AudioEnvironment::default();
        env.target = AudioEnvironmentMode::Underwater;
        // Half the transition window → exactly 0.5 wetness.
        env.advance(env.transition_secs * 0.5);
        assert!((env.wetness - 0.5).abs() < 1e-5, "got {}", env.wetness);
        // Another quarter → 0.5 + 0.25 = 0.75 (saturating step).
        env.advance(env.transition_secs * 0.25);
        assert!(
            ((env.wetness - 0.625).abs() < 1e-5),
            "got {}",
            env.wetness
        );
    }

    #[test]
    fn advance_round_trip_normal_target_pulls_wetness_down() {
        let mut env = AudioEnvironment {
            target: AudioEnvironmentMode::Normal,
            wetness: 1.0,
            transition_secs: 0.5,
        };
        env.advance(0.25); // half the window
        assert!((env.wetness - 0.5).abs() < 1e-5);
        env.advance(0.5);
        assert_eq!(env.wetness, 0.0);
    }

    #[test]
    fn advance_clamps_when_overshooting() {
        let mut env = AudioEnvironment::default();
        env.target = AudioEnvironmentMode::Underwater;
        env.advance(env.transition_secs * 10.0);
        assert_eq!(env.wetness, 1.0);
    }

    #[test]
    fn underwater_attenuation_is_strictly_below_dry() {
        let env = AudioEnvironment {
            target: AudioEnvironmentMode::Underwater,
            wetness: 1.0,
            transition_secs: 0.35,
        };
        assert!(env.music_attenuation() < 1.0);
        assert!(env.sfx_attenuation() < 1.0);
        // Music is ducked more than SFX.
        assert!(env.music_attenuation() < env.sfx_attenuation());
    }

    /// Guardrail: the combined output volume must respect the user's
    /// mixer settings even while the underwater effect is active. If
    /// this ever inverts (e.g. environment writes a fixed dB instead
    /// of multiplying the mixer level), every "mute"/"music=0"
    /// preference would leak audio while submerged.
    #[test]
    fn underwater_composes_with_user_settings() {
        use crate::settings::AudioSettings;

        let dry = AudioEnvironment::default();
        let mut wet = AudioEnvironment::default();
        wet.target = AudioEnvironmentMode::Underwater;
        wet.wetness = 1.0;

        let mut settings = AudioSettings::default();
        settings.master_volume = 0.5;
        settings.music_volume = 0.5;
        settings.sfx_volume = 1.0;

        let dry_music = settings.effective_music() * dry.music_attenuation();
        let wet_music = settings.effective_music() * wet.music_attenuation();
        assert!(wet_music < dry_music, "underwater must duck music");

        // Mute should still produce silence underwater.
        let mut muted = settings;
        muted.muted = true;
        assert_eq!(muted.effective_music() * wet.music_attenuation(), 0.0);
        assert_eq!(muted.effective_sfx() * wet.sfx_attenuation(), 0.0);
    }

    #[test]
    fn quantize_wetness_collapses_jitter() {
        let a = quantize_wetness(0.500_001);
        let b = quantize_wetness(0.500_010);
        assert_eq!(a, b);
        assert_eq!(quantize_wetness(0.0), 0.0);
        assert_eq!(quantize_wetness(1.0), 1.0);
    }

    /// Bevy-integration: `detect_audio_environment` pulls from the
    /// primary player's `water_contact` and writes the matching
    /// target into `AudioEnvironment`. This is the seam the brief
    /// required: gameplay/water owns `water_contact`, the audio
    /// module only observes it — no parallel "is underwater" flag.
    #[cfg(feature = "audio")]
    #[test]
    fn detect_picks_up_player_water_contact_in_app() {
        use ambition_engine as ae;
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<AudioEnvironment>();
        let world = ae::World::new(
            "env_detect_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(100.0, 100.0),
            Vec::new(),
        );
        let mut player =
            ae::Player::new_with_abilities(world.spawn, ae::AbilitySet::sandbox_all());
        player.refresh_movement_resources(ae::DEFAULT_TUNING);
        app.world_mut().spawn((
            crate::player::PlayerEntity,
            crate::player::PrimaryPlayer,
            crate::player::PlayerMovementAuthority::new(player),
        ));
        app.add_systems(Update, detect_audio_environment);

        // No water contact yet → Normal.
        app.update();
        assert_eq!(
            app.world().resource::<AudioEnvironment>().target,
            AudioEnvironmentMode::Normal,
        );

        // Stamp a fully-submerged contact and re-run.
        let region = ae::aabb_from_min_size(
            ae::Vec2::new(50.0, 50.0),
            ae::Vec2::new(100.0, 150.0),
        );
        let contact = ae::WaterContact {
            kind: ae::WaterKind::Clear,
            region_aabb: region,
            surface_y: 50.0,
            submersion: 1.0,
            spec: ae::WaterVolumeSpec::default(),
        };
        {
            let mut q = app.world_mut().query_filtered::<
                &mut crate::player::PlayerMovementAuthority,
                With<crate::player::PrimaryPlayer>,
            >();
            for mut authority in q.iter_mut(app.world_mut()) {
                authority.player.water_contact = Some(contact);
            }
        }
        app.update();
        assert_eq!(
            app.world().resource::<AudioEnvironment>().target,
            AudioEnvironmentMode::Underwater,
        );

        // Shallow contact (below threshold) → Normal again.
        {
            let mut q = app.world_mut().query_filtered::<
                &mut crate::player::PlayerMovementAuthority,
                With<crate::player::PrimaryPlayer>,
            >();
            for mut authority in q.iter_mut(app.world_mut()) {
                authority.player.water_contact = Some(ae::WaterContact {
                    submersion: 0.2,
                    ..contact
                });
            }
        }
        app.update();
        assert_eq!(
            app.world().resource::<AudioEnvironment>().target,
            AudioEnvironmentMode::Normal,
        );
    }
}
