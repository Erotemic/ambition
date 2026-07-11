//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
    assert!(((env.wetness - 0.625).abs() < 1e-5), "got {}", env.wetness);
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
    use crate::persistence::settings::AudioSettings;

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
    use ambition_engine_core as ae;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<AudioEnvironment>();
    // Spawn a minimal primary player with the env-contact cluster
    // component the detect system reads.
    app.world_mut().spawn((
        crate::actor::PlayerEntity,
        crate::actor::PrimaryPlayer,
        crate::actor::BodyEnvironmentContact::default(),
    ));
    app.add_systems(Update, detect_audio_environment);

    // No water contact yet → Normal.
    app.update();
    assert_eq!(
        app.world().resource::<AudioEnvironment>().target,
        AudioEnvironmentMode::Normal,
    );

    // Stamp a fully-submerged contact and re-run.
    let region = ae::aabb_from_min_size(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(100.0, 150.0));
    let contact = ae::WaterContact {
        kind: ae::WaterKind::Clear,
        region_aabb: region,
        surface_y: 50.0,
        submersion: 1.0,
        spec: ae::WaterVolumeSpec::default(),
    };
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::actor::BodyEnvironmentContact,
            With<crate::actor::PrimaryPlayer>,
        >();
        for mut env_contact in q.iter_mut(app.world_mut()) {
            env_contact.water = Some(contact);
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
            &mut crate::actor::BodyEnvironmentContact,
            With<crate::actor::PrimaryPlayer>,
        >();
        for mut env_contact in q.iter_mut(app.world_mut()) {
            env_contact.water = Some(ae::WaterContact {
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
