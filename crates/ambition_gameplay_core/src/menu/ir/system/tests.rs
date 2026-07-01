//! Unit tests for the SYSTEM IR: top-level entry order, dev-build gating, the
//! curated per-screen settings subsets, and the Radio / Language / Developer
//! screen contents built by `SystemMenuModel::build`.

use super::*;

#[test]
fn top_level_order_and_dev_gating() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let ids: Vec<_> = model.entries.iter().map(|e| e.id).collect();
    // The non-dev prefix is always present in this fixed order. Shaders is no
    // longer a top-level entry (it rides under Video); Reset All Settings and
    // Quit to Desktop are always present (Quit sits right after Reset All
    // Settings, before the dev-only entries).
    assert_eq!(
        &ids[..8],
        &[
            SystemMenuEntryId::Radio,
            SystemMenuEntryId::Video,
            SystemMenuEntryId::Audio,
            SystemMenuEntryId::Controls,
            SystemMenuEntryId::Gameplay,
            SystemMenuEntryId::Language,
            SystemMenuEntryId::ResetAllSettings,
            SystemMenuEntryId::Quit,
        ]
    );
    if DEV_BUILD {
        assert_eq!(
            &ids[8..],
            &[
                SystemMenuEntryId::Developer,
                SystemMenuEntryId::ResetSandbox
            ]
        );
    } else {
        assert_eq!(
            ids.len(),
            8,
            "non-dev builds omit Developer + Reset Sandbox"
        );
    }
}

#[test]
fn reset_all_settings_is_an_always_present_action_entry() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let entry = model
        .entry(SystemMenuEntryId::ResetAllSettings)
        .expect("Reset All Settings is always surfaced");
    assert_eq!(
        entry.target,
        SystemMenuTarget::Action(SystemMenuAction::ResetAllSettings),
        "Reset All Settings fires an immediate action (no screen)"
    );
}

#[test]
fn quit_is_an_always_present_action_entry_after_reset_all() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let entry = model
        .entry(SystemMenuEntryId::Quit)
        .expect("Quit to Desktop is always surfaced");
    assert_eq!(entry.label, "Quit to Desktop");
    assert_eq!(
        entry.target,
        SystemMenuTarget::Action(SystemMenuAction::Quit),
        "Quit fires an immediate action (no screen)"
    );
    // Quit sits immediately after Reset All Settings.
    let reset_pos = model
        .entries
        .iter()
        .position(|e| e.id == SystemMenuEntryId::ResetAllSettings)
        .unwrap();
    let quit_pos = model
        .entries
        .iter()
        .position(|e| e.id == SystemMenuEntryId::Quit)
        .unwrap();
    assert_eq!(quit_pos, reset_pos + 1);
}

#[test]
fn video_screen_is_the_curated_subset() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let video = model.entry(SystemMenuEntryId::Video).unwrap();
    let SystemMenuTarget::Settings(options) = &video.target else {
        panic!("video drills into a settings screen");
    };
    let ids: Vec<_> = options.iter().map(|o| o.id).collect();
    // The basic Video rows lead the screen (now the FULL player-facing set in
    // pause-menu page order); the shader subpage follows. `VisualQuality` (the
    // one-global-profile→budget selector from the visual-quality-profiles feature)
    // leads the basic rows.
    assert_eq!(
        &ids[..8],
        &[
            SettingsOptionId::VisualQuality,
            SettingsOptionId::DisplayMode,
            SettingsOptionId::CameraZoom,
            SettingsOptionId::CameraAspect,
            SettingsOptionId::CameraFraming,
            SettingsOptionId::Flashes,
            SettingsOptionId::Colorblind,
            SettingsOptionId::ShowFps,
        ]
    );
}

#[test]
fn shaders_screen_reaches_every_shader_option() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    // Shaders now live UNDER Video (flat, after the basic Video rows) — there
    // is no separate Shaders entry. Assert every shader option is reachable on
    // the Video screen, in pause-menu order.
    let video = model.entry(SystemMenuEntryId::Video).unwrap();
    let SystemMenuTarget::Settings(options) = &video.target else {
        panic!("video drills into a settings screen");
    };
    let shader_ids: Vec<_> = options
        .iter()
        .map(|o| o.id)
        .filter(|id| {
            !matches!(
                id,
                SettingsOptionId::DisplayMode
                    | SettingsOptionId::CameraZoom
                    | SettingsOptionId::CameraAspect
                    | SettingsOptionId::CameraFraming
                    | SettingsOptionId::Flashes
                    | SettingsOptionId::Colorblind
                    | SettingsOptionId::ShowFps
                    | SettingsOptionId::FramePacing
                    | SettingsOptionId::VisualQuality
            )
        })
        .collect();
    // The whole `Video > Shaders` pause-menu subpage is reachable on the cube,
    // now nested under Video.
    assert_eq!(
        shader_ids,
        vec![
            SettingsOptionId::ShaderStrength,
            SettingsOptionId::ShaderCrtStrength,
            SettingsOptionId::ShaderCrtScanlines,
            SettingsOptionId::ShaderCrtMask,
            SettingsOptionId::ShaderCrtCurvature,
            SettingsOptionId::ShaderCrtBloom,
            SettingsOptionId::ShaderCrtChroma,
            SettingsOptionId::ShaderFilmGrainStrength,
            SettingsOptionId::ShaderFilmGrainSize,
            SettingsOptionId::ShaderFilmGrainFps,
            SettingsOptionId::ShaderFilmGrainLumaBias,
            SettingsOptionId::ShaderRobotDeathStrength,
            SettingsOptionId::ShaderRobotStatic,
            SettingsOptionId::ShaderRobotTear,
            SettingsOptionId::ShaderRobotDesaturate,
            SettingsOptionId::ShaderRobotScanlines,
            SettingsOptionId::ShaderUnderwaterStrength,
            SettingsOptionId::ShaderUnderwaterDistortion,
            SettingsOptionId::ShaderDeepDreamStrength,
            SettingsOptionId::ShaderVignetteStrength,
        ]
    );
    // Each shader option carries a live slider value label (e.g. "0%") so the
    // cube renders the same control the grid does. (The leading 9 basic Video
    // rows — 7 basic + FramePacing + VisualQuality — are cycles/toggles, so only
    // the shader tail is checked.)
    for o in options.iter().skip(9) {
        assert!(matches!(o.kind, SettingsOptionKind::Slider { .. }));
    }
}

/// Pull the curated settings-option ids for a category off a built model.
fn screen_ids(model: &SystemMenuModel, id: SystemMenuEntryId) -> Vec<SettingsOptionId> {
    let SystemMenuTarget::Settings(options) = &model.entry(id).unwrap().target else {
        panic!("{id:?} drills into a settings screen");
    };
    options.iter().map(|o| o.id).collect()
}

#[test]
fn system_screens_surface_every_player_facing_setting() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    // Video: the full basic player-facing set (display/camera/accessibility/FPS)
    // — every row the old pause-menu Video page shows (shaders ride after).
    let video = screen_ids(&model, SystemMenuEntryId::Video);
    for id in [
        SettingsOptionId::DisplayMode,
        SettingsOptionId::CameraZoom,
        SettingsOptionId::CameraAspect,
        SettingsOptionId::CameraFraming,
        SettingsOptionId::Flashes,
        SettingsOptionId::Colorblind,
        SettingsOptionId::ShowFps,
        SettingsOptionId::VisualQuality,
    ] {
        assert!(video.contains(&id), "Video screen is missing {id:?}");
    }
    // Audio: the full set.
    let audio = screen_ids(&model, SystemMenuEntryId::Audio);
    for id in [
        SettingsOptionId::MasterVolume,
        SettingsOptionId::MusicVolume,
        SettingsOptionId::SfxVolume,
        SettingsOptionId::Mute,
    ] {
        assert!(audio.contains(&id), "Audio screen is missing {id:?}");
    }
    // Controls: every stick/trigger/dash/menu row the pause menu shows.
    let controls = screen_ids(&model, SystemMenuEntryId::Controls);
    for id in [
        SettingsOptionId::KeyboardPreset,
        SettingsOptionId::ControllerProfile,
        SettingsOptionId::LeftStickDeadzone,
        SettingsOptionId::RightStickDeadzone,
        SettingsOptionId::TriggerPress,
        SettingsOptionId::TriggerRelease,
        SettingsOptionId::DpadMenuNav,
        SettingsOptionId::InvertAimY,
        SettingsOptionId::DashInputMode,
        SettingsOptionId::TouchControls,
        SettingsOptionId::MenuTapMode,
        SettingsOptionId::ResetControlFiltering,
    ] {
        assert!(controls.contains(&id), "Controls screen is missing {id:?}");
    }
    // Gameplay: difficulty/assist/damage plus the HUD + trace toggles.
    let gameplay = screen_ids(&model, SystemMenuEntryId::Gameplay);
    for id in [
        SettingsOptionId::Difficulty,
        SettingsOptionId::Assist,
        SettingsOptionId::PlayerDamage,
        SettingsOptionId::DebugHud,
        SettingsOptionId::QuestHud,
        SettingsOptionId::TraceAutoDump,
    ] {
        assert!(gameplay.contains(&id), "Gameplay screen is missing {id:?}");
    }
}

#[test]
fn developer_screen_surfaces_resource_backed_extra_toggles() {
    // The F1/F2/F12 rows (sourced from SandboxDevState / LdtkHotReloadState, not
    // DeveloperTools) are now part of the Developer screen vocabulary.
    for id in [
        DevToggleId::DebugOverlay,
        DevToggleId::SlowMotion,
        DevToggleId::LdtkAutoApply,
    ] {
        assert!(
            DevToggleId::ALL.contains(&id),
            "{id:?} is a surfaced Developer toggle"
        );
        assert!(!id.is_cycle(), "{id:?} is a toggle, not a cycle");
    }
    // Resource-backed cycles surfaced on the Developer screen. These are not
    // mirrored into `DeveloperTools`; each owning resource remains the single
    // source of truth for its default/current value.
    for id in [
        DevToggleId::PortalEffect,
        DevToggleId::PortalCamera,
        DevToggleId::Gravity,
    ] {
        assert!(DevToggleId::ALL.contains(&id));
        assert!(id.is_cycle(), "{id:?} is a cycle");
    }
    assert_eq!(DevToggleId::ALL.len(), 22);
}

#[test]
fn controls_screen_reaches_keyboard_preset_and_reset() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let controls = model.entry(SystemMenuEntryId::Controls).unwrap();
    let SystemMenuTarget::Settings(options) = &controls.target else {
        panic!("controls drills into a settings screen");
    };
    let ids: Vec<_> = options.iter().map(|o| o.id).collect();
    assert!(ids.contains(&SettingsOptionId::KeyboardPreset));
    assert!(ids.contains(&SettingsOptionId::ResetControlFiltering));
}

#[test]
fn radio_screen_marks_the_active_station() {
    let radio = RadioSnapshot {
        stations: vec![(0, "A".into()), (1, "B".into())],
        active: Some(1),
    };
    let model = SystemMenuModel::build(&UserSettings::default(), &radio, &DevSnapshot::default());
    let SystemMenuTarget::Radio(rows) = &model.entry(SystemMenuEntryId::Radio).unwrap().target
    else {
        panic!("radio screen");
    };
    assert_eq!(rows.len(), 2);
    assert!(!rows[0].active);
    assert!(rows[1].active, "the active station is flagged");
}

#[test]
fn language_stub_only_english_available() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let SystemMenuTarget::Language(rows) =
        &model.entry(SystemMenuEntryId::Language).unwrap().target
    else {
        panic!("language screen");
    };
    assert_eq!(rows.len(), LocaleId::ALL.len());
    let english = rows.iter().find(|r| r.id == LocaleId::English).unwrap();
    assert!(english.available && english.active);
    assert!(
        rows.iter().filter(|r| r.available).count() == 1,
        "only English is selectable in the stub"
    );
}
