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
