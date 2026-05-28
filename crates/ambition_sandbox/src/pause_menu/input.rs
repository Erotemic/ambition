use super::*;

use crate::ui_nav::ListCursor;

/// `Start` input opens/closes the pause menu by toggling `GameMode`.
///
/// Reads from `MenuControlFrame`, the semantic menu-input seam. Keyboard,
/// gamepad, mouse/touch buttons, and touch gestures all fold into that
/// resource before this system runs.
#[cfg(feature = "input")]
pub fn pause_menu_toggle(
    menu: Res<MenuControlFrame>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut inventory: ResMut<InventoryUiState>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !menu.start {
        return;
    }
    match mode.get() {
        GameMode::Playing => {
            state.page = PauseMenuPage::Top;
            state.selected = 0;
            state.stack.clear();
            next_mode.set(GameMode::Paused);
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::UI_PAUSE_OPEN,
                pos: crate::engine_core::Vec2::ZERO,
            });
        }
        GameMode::Paused => {
            inventory.visible = false;
            next_mode.set(GameMode::Playing);
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::UI_PAUSE_CLOSE,
                pos: crate::engine_core::Vec2::ZERO,
            });
        }
        _ => {}
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub fn pause_menu_navigate(
    menu: Res<MenuControlFrame>,
    mode: Res<State<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<InventoryUiState>,
    mut exit: MessageWriter<AppExit>,
    mut display_state: ResMut<DisplayModeState>,
    mut user_settings: ResMut<UserSettings>,
    mut reset_request: ResMut<crate::runtime::reset::SandboxResetRequested>,
    mut dev_toggles: DevToggleParams,
    windows: Query<&mut Window, With<PrimaryWindow>>,
    #[cfg(feature = "audio")] mut library: ResMut<AudioLibrary>,
    #[cfg(feature = "audio")] asset_server: Res<AssetServer>,
    #[cfg(feature = "audio")] mut music_state: ResMut<MusicPlaybackState>,
    #[cfg(feature = "audio")] mut radio: ResMut<RadioStationState>,
    #[cfg(feature = "audio")] music_channel: Res<AudioChannel<MusicChannel>>,
) {
    if !matches!(mode.get(), GameMode::Paused) {
        return;
    }
    if inventory.visible {
        return;
    }
    let mut frame = MenuInputFrame {
        up: menu.up,
        down: menu.down,
        left: menu.left,
        right: menu.right,
        select: menu.select,
        back: menu.back,
        start: menu.start,
    };
    apply_vertical_scroll(&mut frame, menu.vertical_scroll_steps());

    // Fold pointer-driven confirms into the frame, and clear any
    // armed pointer state when the user navigates with kbd / gamepad
    // (touching a different row already cleared/replaced it).
    if state.pointer_confirm {
        frame.select = true;
        state.pointer_confirm = false;
    }
    if frame.any_directional() || frame.back || menu.scroll_y.abs() >= 0.5 {
        state.pointer_armed = None;
    }

    let preset_count = KeyboardPreset::presets().len();

    // MenuBack always pops; if we're already at Top it closes the menu.
    if frame.back {
        match state.page {
            PauseMenuPage::Top => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuPage::Settings(SettingsPage::Top) | PauseMenuPage::Radio => {
                state.page = PauseMenuPage::Top;
                state.selected = 0;
                state.stack.clear();
            }
            _ => {
                state.pop_page();
            }
        }
        return;
    }

    match state.page {
        PauseMenuPage::Top => {
            handle_top_input(
                frame,
                &mut state,
                &mut next_mode,
                &mut inventory,
                &mut exit,
                &mut reset_request,
                #[cfg(feature = "audio")]
                &mut library,
                #[cfg(feature = "audio")]
                &asset_server,
                #[cfg(feature = "audio")]
                &mut music_state,
                #[cfg(feature = "audio")]
                &mut radio,
                #[cfg(feature = "audio")]
                &music_channel,
            );
        }
        PauseMenuPage::Settings(page) => {
            let mut cluster_item = dev_toggles.player_q.single_mut().ok();
            let live_movement_refs = cluster_item.as_mut().map(|item| {
                let clusters = item.as_clusters_mut();
                // Pass individual cluster refs through; the settings
                // code drives `apply_player_body_profile` and
                // `apply_movement_profile` directly.
                (
                    clusters.kinematics,
                    &*clusters.abilities,
                    clusters.dash,
                    clusters.jump,
                )
            });
            handle_settings_page_input(
                frame,
                page,
                &mut state,
                &mut user_settings,
                &mut display_state,
                windows,
                preset_count,
                &mut dev_toggles.dev_state,
                &mut dev_toggles.developer,
                &mut dev_toggles.editable_tuning,
                &mut dev_toggles.ldtk_reload,
                live_movement_refs,
            );
        }
        PauseMenuPage::Radio => {
            #[cfg(feature = "audio")]
            handle_radio_input(
                frame,
                &mut state,
                &mut library,
                &asset_server,
                &mut radio,
                &mut music_state,
                &music_channel,
            );
            #[cfg(not(feature = "audio"))]
            {
                if frame.back || frame.select {
                    state.pop_page();
                }
            }
        }
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn handle_top_input(
    nav: MenuInputFrame,
    state: &mut PauseMenuState,
    next_mode: &mut NextState<GameMode>,
    inventory: &mut InventoryUiState,
    exit: &mut MessageWriter<AppExit>,
    reset_request: &mut crate::runtime::reset::SandboxResetRequested,
    #[cfg(feature = "audio")] library: &mut AudioLibrary,
    #[cfg(feature = "audio")] asset_server: &AssetServer,
    #[cfg(feature = "audio")] music_state: &mut MusicPlaybackState,
    #[cfg(feature = "audio")] radio: &mut RadioStationState,
    #[cfg(feature = "audio")] music_channel: &AudioChannel<MusicChannel>,
) {
    let items = PauseMenuItem::ALL;
    let mut cursor = ListCursor::new(state.selected, items.len());
    cursor.apply_directional(nav.up, nav.down);
    state.selected = cursor.selected();

    let item = items[state.selected];

    #[cfg(feature = "audio")]
    if item == PauseMenuItem::MusicTrack {
        let active = radio
            .selected_track()
            .unwrap_or(music_state.active_track.as_str());
        let next_track = if nav.left {
            library.previous_track_id(active)
        } else if nav.right {
            library.next_track_id(active)
        } else {
            None
        };
        if let Some(next_track) = next_track.map(str::to_string) {
            set_radio_track(
                library,
                asset_server,
                radio,
                music_state,
                music_channel,
                &next_track,
            );
        }
    }

    if nav.select {
        match item {
            PauseMenuItem::Resume => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Settings => {
                state.enter_page(PauseMenuPage::Settings(SettingsPage::Top));
            }
            PauseMenuItem::MusicTrack => {
                #[cfg(feature = "audio")]
                {
                    state.enter_page(PauseMenuPage::Radio);
                    let active = radio
                        .selected_track()
                        .unwrap_or(music_state.active_track.as_str());
                    state.selected = library.track_index(active).unwrap_or(0);
                }
                #[cfg(not(feature = "audio"))]
                {
                    state.enter_page(PauseMenuPage::Radio);
                    state.selected = 0;
                }
            }
            PauseMenuItem::Inventory => {
                inventory.visible = true;
                inventory.selected = 0;
                inventory.opened_from_pause = true;
            }
            PauseMenuItem::ResetSandbox => {
                // Queue the reset and return to gameplay so the
                // processor system can run on the next frame. The
                // banner ("SANDBOX RESET") confirms the action.
                reset_request.request();
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(all(feature = "input", feature = "audio"))]
fn handle_radio_input(
    nav: MenuInputFrame,
    state: &mut PauseMenuState,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    radio: &mut RadioStationState,
    music_state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
) {
    let count = library.track_count();
    if count == 0 {
        return;
    }
    let mut cursor = ListCursor::new(state.selected, count);
    cursor.apply_directional(nav.up || nav.left, nav.down || nav.right);
    state.selected = cursor.selected();
    if nav.select || nav.left || nav.right {
        let track_id = library
            .track_at(state.selected)
            .map(|track| track.id.clone());
        if let Some(track_id) = track_id {
            set_radio_track(
                library,
                asset_server,
                radio,
                music_state,
                music_channel,
                &track_id,
            );
        }
    } else if nav.up || nav.down {
        // Up/down move the highlight without confirming. Warm the
        // highlighted track's handle so the next confirm doesn't
        // hitch on cold asset IO.
        let preload_id = library
            .track_at(state.selected)
            .map(|track| track.id.clone());
        if let Some(preload_id) = preload_id {
            library.preload_track(&preload_id, asset_server);
        }
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn handle_settings_page_input(
    nav: MenuInputFrame,
    page: SettingsPage,
    state: &mut PauseMenuState,
    user_settings: &mut UserSettings,
    display_state: &mut DisplayModeState,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    preset_count: usize,
    dev_state: &mut SandboxDevState,
    developer: &mut DeveloperTools,
    editable_tuning: &mut crate::dev::dev_tools::EditableMovementTuning,
    ldtk_reload: &mut LdtkHotReloadState,
    live_movement_refs: Option<(
        &mut crate::player::PlayerKinematics,
        &crate::player::PlayerAbilities,
        &mut crate::player::PlayerDashState,
        &mut crate::player::PlayerJumpState,
    )>,
) {
    let rows = SettingsItem::rows_for(page);
    if rows.is_empty() {
        return;
    }
    let mut cursor = ListCursor::new(state.selected, rows.len());
    cursor.apply_directional(nav.up, nav.down);
    state.selected = cursor.selected();
    let item = rows[state.selected];

    let action = if nav.left {
        Some(SettingsAction::Prev)
    } else if nav.right {
        Some(SettingsAction::Next)
    } else if nav.select {
        Some(SettingsAction::Confirm)
    } else {
        None
    };
    if let Some(action) = action {
        let outcome = handle_settings_action(
            item,
            action,
            user_settings,
            display_state,
            &mut windows,
            preset_count,
            dev_state,
            developer,
            editable_tuning,
            ldtk_reload,
            live_movement_refs,
        );
        match outcome {
            SettingsOutcome::Stay => {}
            SettingsOutcome::OpenPage(next_page) => {
                state.enter_page(PauseMenuPage::Settings(next_page));
            }
            SettingsOutcome::PopPage => {
                state.pop_page();
            }
        }
    }
}
