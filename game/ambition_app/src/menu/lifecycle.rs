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

#[derive(Default)]
pub(crate) struct MenuPauseState {
    scope: Option<ambition_platformer2d::platformer::lifecycle::SessionScopeId>,
    initialized: bool,
    resume: Option<GameMode>,
}

/// Apply the combined pause claim after all menu input and actions.
pub(crate) fn aggregate_menu_pause(
    mut menus: Query<&mut InventoryUiState>,
    mode: Res<State<GameMode>>,
    mut next: ResMut<NextState<GameMode>>,
    scope: ambition_platformer2d::platformer::lifecycle::LiveSessionScope,
    ownership: Option<Res<ambition_platformer2d::rollback::RollbackSessionOwnership>>,
    mut pause: Local<MenuPauseState>,
) {
    let scope = scope.get();
    if pause.initialized && pause.scope != scope {
        for mut menu in &mut menus { menu.close(); }
        pause.resume = None;
        pause.scope = scope;
        return;
    }
    pause.initialized = true;
    pause.scope = scope;
    // A local surface cannot pause a remote timeline. Its input context
    // captures that participant's controls while the shared simulation runs.
    let remote = ownership.is_some_and(|ownership| ownership.has_remote_peers());
    let open = !remote && menus.iter().any(|menu| menu.visible);
    if open {
        match mode.get() {
            GameMode::Playing | GameMode::Dialogue => {
                pause.resume = Some(*mode.get());
                ambition_platformer2d::platformer::world_log::note_game_mode_request(GameMode::Paused, "local_menu_pause");
                next.set(GameMode::Paused);
            }
            GameMode::Paused => {
                if pause.resume.is_none() { pause.resume = Some(GameMode::Paused); }
            }
            _ => {}
        }
    } else if let Some(resume) = pause.resume.take() {
        if *mode.get() == GameMode::Paused && resume != GameMode::Paused {
            ambition_platformer2d::platformer::world_log::note_game_mode_request(resume, "local_menu_release");
            next.set(resume);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn route_menu_lifecycle(
    mut input: ResMut<MenuControlFrame>,
    backend: Res<InventoryUiBackend>,
    mut overlay: Single<&mut InventoryUiState>,
    mode: Res<State<GameMode>>,
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
                overlay.close();
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
            overlay.close();
            play_ui(&mut sfx, ids::UI_MENU_CLOSE);
            return;
        }
    } else {
        if !super::model::primary_overlay_may_open(*mode.get()) { return; }
        overlay.visible = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::inventory_ui::InventoryMenu;
    use ambition_platformer2d::input::ParticipantId;
    use ambition_platformer2d::platformer::lifecycle::{SessionRoot, SessionScopeId};
    use ambition_platformer2d::rollback::RollbackSessionOwnership;

    fn pause_app(base: GameMode) -> (App, [Entity; 2]) {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0)));
        app.insert_state(base);
        app.add_systems(PostUpdate, aggregate_menu_pause);
        let menus = [ParticipantId::PRIMARY, ParticipantId::SECONDARY].map(|owner| {
            app.world_mut().spawn(InventoryMenu { owner }).id()
        });
        (app, menus)
    }

    fn visible(app: &mut App, menu: Entity, visible: bool) {
        app.world_mut().get_mut::<InventoryUiState>(menu).unwrap().visible = visible;
    }

    fn settle(app: &mut App) { app.update(); app.update(); }
    fn mode(app: &App) -> GameMode { *app.world().resource::<State<GameMode>>().get() }

    #[test]
    fn only_the_last_menu_close_restores_the_underlying_mode() {
        for base in [GameMode::Playing, GameMode::Dialogue, GameMode::Paused] {
            for first in 0..2 {
                let (mut app, menus) = pause_app(base);
                visible(&mut app, menus[first], true);
                settle(&mut app);
                assert_eq!(mode(&app), GameMode::Paused);
                visible(&mut app, menus[1 - first], true);
                settle(&mut app);
                visible(&mut app, menus[first], false);
                settle(&mut app);
                assert_eq!(mode(&app), GameMode::Paused, "the other menu still holds pause");
                app.world_mut().entity_mut(menus[1 - first]).despawn();
                settle(&mut app);
                assert_eq!(mode(&app), base, "removal releases the last menu claim");
            }
        }
    }

    #[test]
    fn a_room_transition_keeps_its_mode_and_does_not_release_other_menus() {
        let (mut app, menus) = pause_app(GameMode::Dialogue);
        for menu in menus { visible(&mut app, menu, true); }
        settle(&mut app);
        app.world_mut().resource_mut::<NextState<GameMode>>().set(GameMode::RoomTransition);
        settle(&mut app);
        visible(&mut app, menus[0], false);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::RoomTransition);
        assert!(app.world().get::<InventoryUiState>(menus[1]).unwrap().visible);
        app.world_mut().resource_mut::<NextState<GameMode>>().set(GameMode::Playing);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Paused, "the remaining menu still holds pause");
        visible(&mut app, menus[1], false);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Playing, "the transition replaced the old dialogue mode");
    }

    #[test]
    fn session_retirement_closes_menus_without_restoring_an_old_mode() {
        let (mut app, menus) = pause_app(GameMode::Dialogue);
        let root = app.world_mut().spawn(SessionRoot(SessionScopeId(1))).id();
        app.update();
        visible(&mut app, menus[0], true);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Paused);
        app.world_mut().entity_mut(root).despawn();
        app.world_mut().resource_mut::<NextState<GameMode>>().set(GameMode::Playing);
        settle(&mut app);
        assert!(!app.world().get::<InventoryUiState>(menus[0]).unwrap().visible);
        assert_eq!(mode(&app), GameMode::Playing);
        app.world_mut().spawn(SessionRoot(SessionScopeId(2)));
        settle(&mut app);
        visible(&mut app, menus[1], true);
        settle(&mut app);
        visible(&mut app, menus[1], false);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Playing);
    }

    #[test]
    fn a_local_menu_does_not_pause_a_remote_timeline() {
        let (mut app, menus) = pause_app(GameMode::Playing);
        app.insert_resource(RollbackSessionOwnership::Peer);
        visible(&mut app, menus[0], true);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Playing);
        app.world_mut().remove_resource::<RollbackSessionOwnership>();
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Paused);
        app.insert_resource(RollbackSessionOwnership::Peer);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Playing, "a remote timeline releases a local hold");
        assert!(app.world().get::<InventoryUiState>(menus[0]).unwrap().visible);
    }
}
