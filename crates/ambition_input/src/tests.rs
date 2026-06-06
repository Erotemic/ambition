use super::*;
use crate::settings::ControlSettings;

#[test]
fn analog_drift_below_deadzone_zeros_movement() {
    // Simulated worn Xbox 360 controller with a small +Y bias.
    let (x, y) = ControlSettings::apply_deadzone(0.04, 0.06, 0.18);
    assert_eq!((x, y), (0.0, 0.0));
    // The same drift fed to analog_to_dir must not pick a direction.
    assert!(analog_to_dir(x, y, 0.5).is_none());
}

#[test]
fn keyboard_preset_presets_returns_four_unique_ids() {
    let presets = KeyboardPreset::presets();
    assert_eq!(presets.len(), 4);
    // Every preset id is unique.
    for (i, a) in presets.iter().enumerate() {
        for b in &presets[i + 1..] {
            assert_ne!(a.id, b.id);
        }
    }
}

#[test]
fn keyboard_preset_movement_label_matches_id_family() {
    // Both Arrows variants produce "Arrow keys"; both WASD variants
    // produce "WASD".
    assert_eq!(KeyboardPreset::arrows_zxc().movement_label(), "Arrow keys");
    assert_eq!(KeyboardPreset::arrows_qwer().movement_label(), "Arrow keys");
    assert_eq!(KeyboardPreset::wasd_jkl().movement_label(), "WASD");
    assert_eq!(KeyboardPreset::wasd_uipo().movement_label(), "WASD");
}

#[test]
fn analog_to_dir_picks_dominant_axis() {
    assert_eq!(analog_to_dir(0.8, 0.1, 0.5), Some(MenuDir::Right));
    assert_eq!(analog_to_dir(-0.8, -0.1, 0.5), Some(MenuDir::Left));
    // +y is up in the leafwing convention used here.
    assert_eq!(analog_to_dir(0.1, 0.8, 0.5), Some(MenuDir::Up));
    assert_eq!(analog_to_dir(0.1, -0.8, 0.5), Some(MenuDir::Down));
}

#[test]
fn menu_state_emits_first_press_then_waits_for_initial_delay() {
    let mut state = MenuInputState::default();
    // First frame holding Down: emit immediately.
    let f = state.step(
        false,
        false,
        false,
        false,
        Some(MenuDir::Down),
        false,
        false,
        false,
        0.016,
        0.30,
        0.10,
    );
    assert!(f.down);
    // Continuing to hold for less than the initial delay must not
    // re-emit.
    let mut emits = 0;
    for _ in 0..5 {
        let f = state.step(
            false,
            false,
            false,
            false,
            Some(MenuDir::Down),
            false,
            false,
            false,
            0.016,
            0.30,
            0.10,
        );
        if f.down {
            emits += 1;
        }
    }
    assert_eq!(emits, 0, "should not repeat before initial delay elapses");
}

#[test]
fn menu_state_repeats_after_initial_delay() {
    let mut state = MenuInputState::default();
    // First push to start the hold.
    let _ = state.step(
        false,
        false,
        false,
        false,
        Some(MenuDir::Right),
        false,
        false,
        false,
        0.016,
        0.10,
        0.05,
    );
    let mut emits = 0;
    for _ in 0..40 {
        let f = state.step(
            false,
            false,
            false,
            false,
            Some(MenuDir::Right),
            false,
            false,
            false,
            0.016,
            0.10,
            0.05,
        );
        if f.right {
            emits += 1;
        }
    }
    assert!(emits >= 4, "expected several repeat ticks; got {emits}");
}

#[test]
fn cardinal_edges_pass_through_without_repeat_state() {
    let mut state = MenuInputState::default();
    // D-pad / arrow keys edge fires on one frame but does not start
    // an analog hold.
    let f = state.step(
        true, false, false, false, None, false, false, false, 0.016, 0.30, 0.10,
    );
    assert!(f.up);
    let f = state.step(
        false, false, false, false, None, false, false, false, 0.016, 0.30, 0.10,
    );
    assert!(!f.any_directional());
}

#[test]
fn menu_state_select_passes_through() {
    let mut state = MenuInputState::default();
    let f = state.step(
        false, false, false, false, None, true, false, false, 0.016, 0.30, 0.10,
    );
    assert!(f.select);
    assert!(!f.any_directional());
}

#[test]
fn menu_control_scroll_steps_round_and_clamp() {
    let frame = MenuControlFrame {
        scroll_y: -2.4,
        ..Default::default()
    };
    assert_eq!(frame.vertical_scroll_steps(), -2);
    let frame = MenuControlFrame {
        scroll_y: 42.0,
        ..Default::default()
    };
    assert_eq!(frame.vertical_scroll_steps(), 6);
}

#[test]
fn menu_state_back_passes_through() {
    let mut state = MenuInputState::default();
    let f = state.step(
        false, false, false, false, None, false, true, false, 0.016, 0.30, 0.10,
    );
    assert!(f.back);
}
