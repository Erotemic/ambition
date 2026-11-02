//! Shared lifecycle for the inventory menu presentations.

use bevy::prelude::*;
use ambition_platformer2d::game_shell::PauseRequested;
use ambition_platformer2d::input::MenuControlFrame;
use ambition_platformer2d::inventory_ui::InventoryUiState;
use ambition_platformer2d::menu::{ActiveMenuPages, backend::InventoryUiBackend, map::MapMenuState};
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::sfx::{SfxWriter, ids};

use super::kaleidoscope_app::{KaleidoscopeCursor, KaleidoscopeSystemNav, close_system_entry, play_ui};
use super::model::{MenuFocus, MenuPage, MenuPageAction, OpenOverlayKey};
use super::quality_confirm::VisualQualityConfirmState;

#[allow(clippy::too_many_arguments)]
pub(crate) fn route_menu_lifecycle(
    mut input: ResMut<MenuControlFrame>,
    backend: Res<InventoryUiBackend>,
    mut overlay: ResMut<InventoryUiState>,
    mode: Res<State<GameMode>>,
    mut next: ResMut<NextState<GameMode>>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system: ResMut<KaleidoscopeSystemNav>,
    mut quality: ResMut<VisualQualityConfirmState>,
    mut map: ResMut<MapMenuState>,
    mut sfx: SfxWriter,
    mut last_start: Local<bool>,
    mut pause: MessageReader<PauseRequested>,
    #[cfg(feature = "bevy_ui_menu")]
    mut grid: Option<ResMut<super::grid_backend::GridMenuTabState>>,
) {
    if !backend.effective().is_available() { return; }
    let platform_pause = pause.read().count() > 0;
    let start = input.start && !*last_start;
    *last_start = input.start;
    if start { input.back = false; }

    let page = if platform_pause {
        if overlay.visible { return; }
        MenuPage::System
    } else if start {
        if overlay.visible {
            quality.cancel();
            if system.open_entry.is_some() {
                close_system_entry(&mut system, &mut cursor);
                play_ui(&mut sfx, ids::UI_MENU_BACK);
                #[cfg(feature = "bevy_ui_menu")]
                if let Some(grid) = grid.as_mut() { grid.reset_navigation(); }
            } else {
                close_menu(&mut overlay, mode.get(), &mut next);
                play_ui(&mut sfx, ids::UI_MENU_CLOSE);
            }
            return;
        }
        MenuPage::System
    } else if input.inventory {
        MenuPage::Items
    } else if input.map {
        MenuPage::Map
    } else { return; };

    if overlay.visible {
        if super::model::open_overlay_key(page, pages.active) == OpenOverlayKey::Close {
            quality.cancel();
            close_menu(&mut overlay, mode.get(), &mut next);
            play_ui(&mut sfx, ids::UI_MENU_CLOSE);
            return;
        }
    } else {
        if !super::model::primary_overlay_may_open(*mode.get()) { return; }
        overlay.visible = true;
        if let Some(request) = super::model::mode_on_overlay_open(&mut overlay, *mode.get()) {
            ambition_platformer2d::platformer::world_log::note_game_mode_request(request, "menu_open");
            next.set(request);
        }
        play_ui(&mut sfx, ids::UI_MENU_OPEN);
    }
    pages.active = Some(page);
    system.open_entry = None;
    cursor.reset_pointer();
    cursor.mark_keyboard(match page {
        MenuPage::Items => MenuFocus::Item(0),
        MenuPage::System => MenuFocus::System(0),
        MenuPage::Map | MenuPage::Quest => MenuFocus::EdgeLeft,
    });
    map.open = false;
    #[cfg(feature = "bevy_ui_menu")]
    if let Some(grid) = grid.as_mut() { grid.reset_navigation(); }
}

pub(crate) fn close_menu(
    overlay: &mut InventoryUiState,
    mode: &GameMode,
    next: &mut NextState<GameMode>,
) {
    overlay.close();
    if let Some(request) = super::model::mode_on_overlay_close(overlay, *mode) {
        ambition_platformer2d::platformer::world_log::note_game_mode_request(request, "menu_close");
        next.set(request);
    }
}
