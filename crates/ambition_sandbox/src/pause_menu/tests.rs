use super::*;

#[test]
fn pause_menu_state_default_is_top_page_zero() {
    let s = PauseMenuState::default();
    assert!(matches!(s.page, PauseMenuPage::Top));
    assert_eq!(s.selected, 0);
}

#[test]
fn enter_page_pushes_onto_stack() {
    let mut s = PauseMenuState {
        selected: 3,
        page: PauseMenuPage::Top,
        stack: Vec::new(),
        pointer_armed: None,
        pointer_confirm: false,
        focus: crate::ui_nav::MenuFocusState::default(),
    };
    s.enter_page(PauseMenuPage::Settings(SettingsPage::Top));
    assert!(matches!(s.page, PauseMenuPage::Settings(SettingsPage::Top)));
    assert_eq!(s.selected, 0);
    assert_eq!(s.stack.len(), 1);
    s.pop_page();
    assert!(matches!(s.page, PauseMenuPage::Top));
    assert!(s.stack.is_empty());
}

#[test]
fn pause_menu_item_all_includes_settings() {
    assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::Settings));
}

/// `ResetSandbox` is the user-facing entry point for the
/// "wipe the save and rebuild the runtime" flow. Pin it here so
/// a future menu-shape refactor can't silently drop it.
#[test]
fn pause_menu_item_all_includes_reset_sandbox() {
    assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::ResetSandbox));
    assert_eq!(PauseMenuItem::ResetSandbox.static_label(), "Reset Sandbox");
}

/// `MenuSettingsItem` is the public re-export so other modules can
/// query rows by tag without crossing the private boundary.
#[test]
fn menu_settings_item_is_settings_item() {
    let _ = MenuSettingsItem::DisplayMode;
}

/// Stage 3a parity: the pause menu and the shared settings IR must agree on
/// the label, value, and apply behaviour for every option they both surface.
///
/// These tests pin that the two frontends (bevy-UI pause menu + 3D cube) cannot
/// drift on the overlapping option set: the pause rows that map to a
/// [`SettingsOptionId`] derive their label/value from `settings_menu_model` and
/// apply through `apply_settings_option`. If a new shared option appears (or a
/// mapped option's behaviour diverges) one of these fails.
#[cfg(feature = "input")]
mod shared_ir_parity {
    use crate::persistence::settings::{
        apply_action, apply_settings_option, settings_menu_model, SettingsAction, SettingsItem,
        SettingsOption, SettingsOptionId, SettingsOptionKind, SettingsPage, UserSettings,
    };

    /// Every pause-menu row surfaced anywhere in the menu (across all pages).
    fn all_surfaced_items() -> Vec<SettingsItem> {
        let mut items = Vec::new();
        for &page in SettingsPage::ALL {
            for &item in SettingsItem::rows_for(page) {
                if !items.contains(&item) {
                    items.push(item);
                }
            }
        }
        items
    }

    /// Every shared option id the IR actually produces (i.e. every category
    /// option). `Close` is a renderer pseudo-option not built into the model
    /// and not surfaced by the pause menu, so it is excluded.
    fn model_option_ids(settings: &UserSettings) -> Vec<SettingsOptionId> {
        settings_menu_model(settings)
            .categories
            .iter()
            .flat_map(|c| c.options.iter())
            .map(|o| o.id)
            .collect()
    }

    fn shared_option(id: SettingsOptionId, settings: &UserSettings) -> SettingsOption {
        settings_menu_model(settings)
            .categories
            .iter()
            .flat_map(|c| c.options.iter())
            .find(|o| o.id == id)
            .cloned()
            .unwrap_or_else(|| panic!("IR has no option for {id:?}"))
    }

    /// The pause menu's decoration of an IR option: cycle/slider rows get the
    /// `< / >` arrows, toggle/action rows do not. Mirrors
    /// `model::pause_label_from_shared`; duplicated here on purpose so the test
    /// fails if that rendering rule silently changes.
    fn expected_pause_label(opt: &SettingsOption) -> String {
        match opt.kind {
            SettingsOptionKind::Cycle { .. } | SettingsOptionKind::Slider { .. } => {
                format!("{}: {}  < / >", opt.label, opt.value_label)
            }
            // A valueless action row (e.g. "Reset Filter Defaults") reads as a
            // bare label.
            SettingsOptionKind::Action if opt.value_label.is_empty() => opt.label.clone(),
            SettingsOptionKind::Toggle(_) | SettingsOptionKind::Action => {
                format!("{}: {}", opt.label, opt.value_label)
            }
        }
    }

    /// A handful of non-default states so the parity check covers live values,
    /// not just defaults.
    fn sample_settings() -> Vec<UserSettings> {
        let mut s1 = UserSettings::default();
        s1.video.show_fps = !s1.video.show_fps;
        s1.audio.muted = true;
        s1.audio.master_volume = 0.35;
        s1.controls.left_stick_deadzone = 0.18;
        s1.controls.trigger_press_threshold = 0.7;
        s1.gameplay.player_damage_multiplier = 1.5;
        s1.gameplay.assist = s1.gameplay.assist.toggle();

        let mut s2 = UserSettings::default();
        s2.audio.master_volume = 1.0;
        s2.controls.right_stick_deadzone = 0.6;
        s2.controls.trigger_release_threshold = 0.9;

        // Stage 3b: exercise the migrated shaders + keyboard preset across live,
        // non-default values so the cycle wrap and shader clamps are covered.
        let mut s3 = UserSettings::default();
        s3.controls.keyboard_preset_index = 3; // last preset; +1 wraps to 0
        s3.video.shaders.strength = 0.5;
        s3.video.shaders.crt_strength = 0.9; // near clamp ceiling
        s3.video.shaders.crt_scanlines = 0.05;
        s3.video.shaders.film_grain_size = 8.0; // range ceiling
        s3.video.shaders.film_grain_fps = 60.0; // range ceiling
        s3.video.shaders.deep_dream_strength = 0.0; // exercise the auto-arm side-effect
        s3.video.shaders.vignette_strength = 0.95;

        vec![UserSettings::default(), s1, s2, s3]
    }

    /// Every mapped pause row's id is one the IR model actually builds, so the
    /// label/apply lookups can never miss. Implicitly checks the mapping is
    /// "live" (no stale id that the model dropped).
    #[test]
    fn mapped_ids_are_all_produced_by_the_model() {
        let settings = UserSettings::default();
        let produced = model_option_ids(&settings);
        for item in all_surfaced_items() {
            if let Some(id) = item.shared_option_id() {
                assert!(
                    produced.contains(&id),
                    "{item:?} maps to {id:?} but the IR model never builds it"
                );
            }
        }
    }

    /// The mapping is exhaustive over the OVERLAP: every shared option the IR
    /// models is surfaced by at least one pause-menu row that maps to it. Adding
    /// a new `SettingsOptionId` the pause menu shows without mapping it here
    /// fails this test (the whole point: the frontends can't silently diverge).
    #[test]
    fn every_model_option_is_mapped_by_some_pause_row() {
        let settings = UserSettings::default();
        let mapped: Vec<SettingsOptionId> = all_surfaced_items()
            .into_iter()
            .filter_map(|i| i.shared_option_id())
            .collect();
        for id in model_option_ids(&settings) {
            assert!(
                mapped.contains(&id),
                "shared option {id:?} is not surfaced/mapped by any pause-menu row"
            );
        }
    }

    /// For every mapped row, the pause menu's rendered label == the shared IR's
    /// label+value (with the pause menu's `< / >` decoration), across several
    /// settings states. This is the anti-drift pin for display text.
    #[test]
    fn labels_match_the_shared_ir() {
        for settings in sample_settings() {
            for item in all_surfaced_items() {
                let Some(id) = item.shared_option_id() else {
                    continue;
                };
                let opt = shared_option(id, &settings);
                assert_eq!(
                    item.label(&settings),
                    expected_pause_label(&opt),
                    "{item:?} label drifted from shared IR {id:?}"
                );
            }
        }
    }

    /// For every mapped row, applying a directional step through the real
    /// pause-menu path (`apply_action`) leaves `UserSettings` identical to
    /// applying it through the shared IR (`apply_settings_option`). This is the
    /// anti-drift pin for mutation. Run inside a minimal World because
    /// `apply_action` borrows the live primary `Window` (only `DisplayMode`
    /// actually pokes it; the post-state of `UserSettings` is what we compare).
    #[test]
    fn apply_matches_the_shared_ir() {
        use bevy::ecs::system::SystemState;
        use bevy::prelude::*;
        use bevy::window::PrimaryWindow;

        for item in all_surfaced_items() {
            let Some(id) = item.shared_option_id() else {
                continue;
            };
            for (action, dir) in [
                (SettingsAction::Prev, -1),
                (SettingsAction::Next, 1),
                (SettingsAction::Confirm, 0),
            ] {
                for base in sample_settings() {
                    // Expected: shared IR mutation.
                    let mut expected = base.clone();
                    apply_settings_option(id, dir, &mut expected);

                    // Actual: drive the real pause-menu apply path in a World.
                    let mut world = World::new();
                    world.spawn((Window::default(), PrimaryWindow));
                    world.insert_resource(base.clone());
                    world.insert_resource(crate::host::windowing::DisplayModeState::default());
                    world.insert_resource(crate::SandboxDevState::default());
                    world.insert_resource(crate::dev::dev_tools::DeveloperTools::default());
                    world.insert_resource(crate::dev::dev_tools::EditableMovementTuning::default());
                    world.insert_resource(crate::ldtk_world::LdtkHotReloadState::default());

                    let mut state: SystemState<(
                        ResMut<UserSettings>,
                        ResMut<crate::host::windowing::DisplayModeState>,
                        Query<&mut Window, With<PrimaryWindow>>,
                        ResMut<crate::SandboxDevState>,
                        ResMut<crate::dev::dev_tools::DeveloperTools>,
                        ResMut<crate::dev::dev_tools::EditableMovementTuning>,
                        ResMut<crate::ldtk_world::LdtkHotReloadState>,
                    )> = SystemState::new(&mut world);
                    {
                        let (
                            mut settings,
                            mut display_state,
                            mut windows,
                            mut dev_state,
                            mut developer,
                            mut editable_tuning,
                            mut ldtk_reload,
                        ) = state.get_mut(&mut world);
                        apply_action(
                            item,
                            action,
                            &mut settings,
                            &mut display_state,
                            &mut windows,
                            // keyboard_preset_count: the fixed
                            // `KeyboardPreset::presets().len()`. KeyboardPreset is
                            // now a mapped row that routes through the shared IR,
                            // which wraps modulo the same count, so this matches.
                            4,
                            &mut dev_state,
                            &mut developer,
                            &mut editable_tuning,
                            &mut ldtk_reload,
                            None,
                        );
                    }
                    state.apply(&mut world);

                    let actual = world.resource::<UserSettings>().clone();
                    assert_eq!(
                        actual, expected,
                        "{item:?} ({action:?}) diverged from shared IR {id:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn visible_window_tracks_selected_row_without_overflow() {
    assert_eq!(crate::ui_nav::visible_window_start(0, 12, 5), 0);
    assert_eq!(crate::ui_nav::visible_window_start(4, 12, 5), 2);
    assert_eq!(crate::ui_nav::visible_window_start(11, 12, 5), 7);
    assert_eq!(visible_row_index(0, 11, 12, 5), Some(7));
    assert_eq!(visible_row_index(4, 11, 12, 5), Some(11));
    assert_eq!(visible_row_index(5, 11, 12, 5), None);
}

#[test]
fn radio_rows_are_windowed_for_mobile_panels() {
    assert!(super::model::RADIO_VISIBLE_ROWS < super::model::MAX_ROWS);
    assert_eq!(super::model::RADIO_VISIBLE_ROWS, 8);
    let cursor = crate::ui_nav::ListCursor::new(12, 26);
    assert_eq!(
        cursor.windowed_title("Radio", super::model::RADIO_VISIBLE_ROWS),
        "Radio — 13/26"
    );
    assert_eq!(
        cursor.visible_row_for_slot(0, super::model::RADIO_VISIBLE_ROWS),
        Some(8)
    );
    assert_eq!(
        cursor.visible_row_for_slot(7, super::model::RADIO_VISIBLE_ROWS),
        Some(15)
    );
    assert_eq!(
        cursor.visible_row_for_slot(8, super::model::RADIO_VISIBLE_ROWS),
        None
    );
}
