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
    // The non-dev prefix is always present in this fixed order. Shaders is no longer a
    // top-level entry (it rides under Video); Reset All Settings, Quit to Title, and Quit to
    // Desktop are always present (the two exits sit right after Reset All Settings — Title
    // above Desktop
    assert_eq!(
        &ids[..9],
        &[
            SystemMenuEntryId::Radio,
            SystemMenuEntryId::Video,
            SystemMenuEntryId::Audio,
            SystemMenuEntryId::Controls,
            SystemMenuEntryId::Gameplay,
            SystemMenuEntryId::Language,
            SystemMenuEntryId::ResetAllSettings,
            SystemMenuEntryId::QuitToHome,
            SystemMenuEntryId::Quit,
        ]
    );
    if DEV_BUILD {
        assert_eq!(
            &ids[9..],
            &[
                SystemMenuEntryId::Developer,
                SystemMenuEntryId::ResetNewGame
            ]
        );
    } else {
        assert_eq!(
            ids.len(),
            9,
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
    // The two exits sit in order after Reset All Settings: Quit to Title, then
    // Quit to Desktop.
    let pos = |id| model.entries.iter().position(|e| e.id == id).unwrap();
    let reset_pos = pos(SystemMenuEntryId::ResetAllSettings);
    assert_eq!(pos(SystemMenuEntryId::QuitToHome), reset_pos + 1);
    assert_eq!(pos(SystemMenuEntryId::Quit), reset_pos + 2);
}

#[test]
fn quit_to_home_is_an_always_present_action_entry_above_quit() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let entry = model
        .entry(SystemMenuEntryId::QuitToHome)
        .expect("Quit to Title is always surfaced");
    assert_eq!(entry.label, "Quit to Title");
    assert_eq!(
        entry.target,
        SystemMenuTarget::Action(SystemMenuAction::QuitToHome),
        "Quit to Title fires an immediate action (retire session -> title), no screen"
    );
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
                    | SettingsOptionId::Vsync
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
    // cube renders the same control the grid does. (The leading 10 basic Video
    // rows — 7 basic + FramePacing + Vsync + VisualQuality — are cycles/toggles, so
    // only the shader tail is checked.)
    for o in options.iter().skip(10) {
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
    // Video exposes the complete player-facing display/camera/accessibility/FPS
    // set; shader rows follow these basic settings.
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
    // Controls: every stick/trigger/burst/menu row the pause menu shows.
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
        SettingsOptionId::BurstInputMode,
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
    // Resource-backed rows sourced from DeveloperRuntimeState / WorldSourceHotReload, not
    // DeveloperTools, are part of the Developer screen vocabulary.
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

/// Every option id either reaches a system settings screen, or says why it does not.
///
/// `settings/apply.rs` matches the enum exhaustively, so each option must say
/// what it does. `settings/build.rs` pushes rows explicitly, so a wired option
/// could appear on no screen and other tests would still pass.
///
/// Three guards split the work:
/// `scripts/check_enum_all_constants_are_complete.py` (run by `--maintenance`)
/// decides whether a variant is on `ALL`; this test decides where it appears;
/// `apply.rs`'s exhaustive match decides what changing it does.
#[test]
fn every_settings_option_id_reaches_a_screen() {
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    // A Vec rather than a set: `SettingsOptionId` is not `Ord`/`Hash`, and the
    // membership test below is over 59 ids once — the linear scan is free and adding
    // a derive to a production enum to satisfy a test would be the tail wagging.
    let mut reachable: Vec<SettingsOptionId> = Vec::new();
    for entry in &model.entries {
        if let SystemMenuTarget::Settings(options) = &entry.target {
            for option in options {
                if !reachable.contains(&option.id) {
                    reachable.push(option.id);
                }
            }
        }
    }
    // Guard against a vacuous pass: the default build reaches 58 of 59 ids.
    assert!(
        reachable.len() > 50,
        "only {} settings ids are reachable — the model did not build, and the \
         per-id assertions below are vacuous",
        reachable.len()
    );

    for id in SettingsOptionId::ALL {
        // The catch-all arm defaults to `true`, so a new id on `ALL` fails below
        // unless a player can reach it or an explicit arm says why not. An id
        // missing from `ALL` is caught by
        // `scripts/check_enum_all_constants_are_complete.py`.
        let must_reach = match id {
            // A momentary Close / Back action, not a setting with a value, so it is
            // not a row on any settings screen.
            SettingsOptionId::Close => false,
            _other => true,
        };
        assert_eq!(
            reachable.contains(&id),
            must_reach,
            "{id:?}: reachable={} but this test expects {must_reach}. A setting the \
             player cannot reach is wired but invisible, which is the defect this \
             guard exists for; if it is deliberately not a screen row, say so in the \
             match above.",
            reachable.contains(&id)
        );
    }
}
