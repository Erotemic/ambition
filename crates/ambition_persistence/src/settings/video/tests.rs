//! Unit tests for video settings: shader-strength clamping and related nudges.

use super::*;

#[test]
fn screen_shader_strength_stays_clamped() {
    let mut shaders = ScreenShaderSettings::default();
    shaders.nudge_strength(10.0);
    assert_eq!(shaders.strength, 1.0);
    shaders.nudge_strength(-10.0);
    assert_eq!(shaders.strength, 0.0);
}

#[test]
fn flash_intensity_cycles() {
    let order = [
        FlashIntensity::On,
        FlashIntensity::Reduced,
        FlashIntensity::Off,
        FlashIntensity::On,
    ];
    let mut current = order[0];
    for expected in &order[1..] {
        current = current.next();
        assert_eq!(current, *expected);
    }
}

#[test]
fn colorblind_mode_cycles_through_all() {
    let mut visited = std::collections::HashSet::new();
    let mut cur = ColorblindMode::Off;
    for _ in 0..ColorblindMode::ALL.len() {
        visited.insert(cur);
        cur = cur.next();
    }
    assert_eq!(visited.len(), ColorblindMode::ALL.len());
}

#[test]
fn flash_multiplier_clamps() {
    assert_eq!(FlashIntensity::On.multiplier(), 1.0);
    assert_eq!(FlashIntensity::Off.multiplier(), 0.0);
    assert!(FlashIntensity::Reduced.multiplier() > 0.0);
    assert!(FlashIntensity::Reduced.multiplier() < 1.0);
}

#[test]
fn flash_intensity_cycles_through_all() {
    let mut visited: Vec<FlashIntensity> = Vec::new();
    let mut cur = FlashIntensity::On;
    for _ in 0..FlashIntensity::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), FlashIntensity::ALL.len());
}

#[test]
fn camera_zoom_preset_scales_are_positive_finite() {
    for preset in CameraZoomPreset::ALL {
        let scale = preset.scale();
        assert!(scale > 0.0 && scale.is_finite());
    }
}

#[test]
fn camera_zoom_preset_cycles_through_all() {
    let mut visited: Vec<CameraZoomPreset> = Vec::new();
    let mut cur = CameraZoomPreset::Combat;
    for _ in 0..CameraZoomPreset::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), CameraZoomPreset::ALL.len());
}

#[test]
fn camera_aspect_policy_cycles_through_all() {
    let mut visited: Vec<CameraAspectPolicy> = Vec::new();
    let mut cur = CameraAspectPolicy::FitDesign;
    for _ in 0..CameraAspectPolicy::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), CameraAspectPolicy::ALL.len());
}

#[test]
fn combat_framing_biases_up_without_horizontal_tap_lookahead() {
    let (dx, dy) = CameraFramingPreset::Combat.target_offset(800.0, 450.0, 1.0);
    assert_eq!(dx, 0.0);
    assert!(dy < 0.0);
    let (dx_left, _) = CameraFramingPreset::Combat.target_offset(800.0, 450.0, -1.0);
    assert_eq!(dx_left, 0.0);
}

#[test]
fn flash_intensity_prev_next_round_trip() {
    let f = FlashIntensity::Reduced;
    assert_eq!(f.next().prev(), f);
}

#[test]
fn visual_quality_profile_table_matches_android_starting_budget() {
    let low = VisualQualityBudget::for_profile(VisualQualityProfile::Low);
    assert_eq!(low.portal.max_resolution, 384);
    assert_eq!(low.portal.recursion_depth, 0);
    assert!(!low.portal.include_parallax);
    assert_eq!(low.sprites.resolution_scale, TextureResolutionScale::Half);
    assert_eq!(low.parallax.max_layers, Some(2));

    let ultra = VisualQualityBudget::for_profile(VisualQualityProfile::Ultra);
    assert_eq!(ultra.portal.max_active_captures, 4);
    assert_eq!(ultra.backgrounds.max_texture_resolution, 4096);
    assert_eq!(ultra.particles.max_particles, 1024);
}

#[test]
fn potato_profile_strips_everything_to_the_bare_minimum() {
    let potato = VisualQualityBudget::for_profile(VisualQualityProfile::Potato);
    // Smallest possible portal capture, no recursion / parallax, throttled hard.
    assert_eq!(potato.portal.max_resolution, 128);
    assert_eq!(potato.portal.recursion_depth, 0);
    assert!(!potato.portal.include_parallax);
    assert!(potato.portal.min_refresh_interval_s > 0.0);
    // Tiniest textures for sprites + backgrounds.
    assert_eq!(
        potato.sprites.resolution_scale,
        TextureResolutionScale::Potato
    );
    assert!(potato.sprites.prefer_scaled_variants);
    assert_eq!(
        potato.backgrounds.resolution_scale,
        TextureResolutionScale::Potato
    );
    // Parallax off, shaders off, almost no particles.
    assert!(!potato.parallax.enabled);
    assert_eq!(potato.shaders.screen_shader_scale, 0.0);
    assert!(!potato.shaders.allow_expensive_materials);
    assert!(potato.particles.max_particles <= 32);

    // Potato is the floor: it is no heavier than Low on the levers that matter.
    let low = VisualQualityBudget::for_profile(VisualQualityProfile::Low);
    assert!(potato.portal.max_resolution <= low.portal.max_resolution);
    assert!(potato.particles.max_particles <= low.particles.max_particles);
}

#[test]
fn visual_quality_profile_cycles_through_all_including_potato() {
    assert!(VisualQualityProfile::ALL.contains(&VisualQualityProfile::Potato));
    let mut visited: Vec<VisualQualityProfile> = Vec::new();
    let mut cur = VisualQualityProfile::High;
    for _ in 0..VisualQualityProfile::ALL.len() {
        if !visited.contains(&cur) {
            visited.push(cur);
        }
        cur = cur.next();
    }
    assert_eq!(visited.len(), VisualQualityProfile::ALL.len());
    // next/prev round-trips around the new first variant.
    assert_eq!(
        VisualQualityProfile::Potato.next().prev(),
        VisualQualityProfile::Potato
    );
}

#[test]
fn custom_visual_quality_resolves_to_stored_budget() {
    let mut settings = VisualQualitySettings::default();
    settings.profile = VisualQualityProfile::Custom;
    settings.custom.portal.max_resolution = 333;
    assert_eq!(settings.resolved_budget().portal.max_resolution, 333);
}

#[test]
fn texture_resolution_scale_owns_variant_folder_names() {
    assert_eq!(
        TextureResolutionScale::Half.asset_subdir("custom_sprites"),
        "custom_sprites_0_5x"
    );
    assert_eq!(
        TextureResolutionScale::Quarter.asset_subdir("sprites"),
        "sprites_0_25x"
    );
    assert_eq!(
        TextureResolutionScale::Half.parallax_subdir(),
        "backgrounds/parallax_layers_0_5x"
    );
    assert_eq!(
        TextureResolutionScale::Full.asset_subdir("sprites"),
        "sprites"
    );
    assert_eq!(
        TextureResolutionScale::Potato.asset_subdir("sprites"),
        "sprites_potato"
    );
    assert_eq!(
        TextureResolutionScale::Potato.parallax_subdir(),
        "backgrounds/parallax_layers_potato"
    );
    assert_eq!(
        TextureResolutionScale::Potato.asset_id_suffix(),
        Some("potato")
    );
    // Every below-Full tier is a generated variant; Full is never in the list.
    assert_eq!(TextureResolutionScale::MANIFEST_VARIANTS.len(), 3);
    assert!(TextureResolutionScale::MANIFEST_VARIANTS.contains(&TextureResolutionScale::Potato));
    assert!(!TextureResolutionScale::MANIFEST_VARIANTS.contains(&TextureResolutionScale::Full));
}
