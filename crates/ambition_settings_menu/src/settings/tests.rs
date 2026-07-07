//! Unit tests for the settings IR: model build (category order, live value
//! labels) and `apply_settings_option` mutation/close behaviour.

use super::*;
use ambition_persistence::settings::UserSettings;

#[test]
fn model_has_the_four_categories_in_order() {
    let model = settings_menu_model(&UserSettings::default());
    let ids: Vec<_> = model.categories.iter().map(|c| c.id).collect();
    assert_eq!(ids, SettingsCategoryId::ALL.to_vec());
    for cat in &model.categories {
        assert!(!cat.options.is_empty(), "{:?} has options", cat.id);
        assert_eq!(cat.label, cat.id.label());
    }
}

#[test]
fn value_labels_are_live() {
    let mut settings = UserSettings::default();
    settings.video.show_fps = false;
    let model = settings_menu_model(&settings);
    let fps = model
        .category(SettingsCategoryId::Video)
        .unwrap()
        .options
        .iter()
        .find(|o| o.id == SettingsOptionId::ShowFps)
        .unwrap();
    assert_eq!(fps.value_label, "OFF");
    assert!(matches!(fps.kind, SettingsOptionKind::Toggle(false)));
}

#[test]
fn apply_toggles_a_bool_field() {
    let mut settings = UserSettings::default();
    let before = settings.gameplay.quest_hud_visible;
    let closed = apply_settings_option(SettingsOptionId::QuestHud, 0, &mut settings);
    assert!(!closed);
    assert_ne!(settings.gameplay.quest_hud_visible, before);
}

#[test]
fn apply_steps_a_slider_and_cycles_an_enum() {
    let mut settings = UserSettings::default();
    let v0 = settings.audio.master_volume;
    apply_settings_option(SettingsOptionId::MasterVolume, 1, &mut settings);
    assert!(settings.audio.master_volume >= v0);

    let z0 = settings.video.camera_zoom;
    apply_settings_option(SettingsOptionId::CameraZoom, 1, &mut settings);
    assert_eq!(settings.video.camera_zoom, z0.next());
}

#[test]
fn close_option_reports_close() {
    let mut settings = UserSettings::default();
    assert!(apply_settings_option(
        SettingsOptionId::Close,
        0,
        &mut settings
    ));
}

#[test]
fn slider_value_label_is_percent() {
    let model = settings_menu_model(&UserSettings::default());
    let master = model
        .category(SettingsCategoryId::Audio)
        .unwrap()
        .options
        .iter()
        .find(|o| o.id == SettingsOptionId::MasterVolume)
        .unwrap();
    assert!(master.value_label.ends_with('%'));
    assert!(matches!(master.kind, SettingsOptionKind::Slider { .. }));
}
