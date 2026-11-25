//! Shared lifecycle for the inventory menu presentations.

use bevy::prelude::*;
use ambition_platformer2d::game_shell::PauseRequested;
use ambition_platformer2d::input::{SeatMenuFrames, ParticipantId};
use ambition_platformer2d::inventory_ui::InventoryUiState;
use ambition_platformer2d::menu::{ActiveMenuPages, backend::InventoryUiBackend, map::MapMenuState};
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::sfx::{SfxWriter, ids};

use super::kaleidoscope_app::{KaleidoscopeCursor, KaleidoscopeSystemNav, RebindCapture, play_ui};
use super::model::{MenuFocus, MenuPage, MenuPageAction, OpenOverlayKey};
use super::quality_confirm::VisualQualityConfirmState;

pub(crate) fn install_session_menus(app: &mut App) {
    app.init_resource::<SeatMenuFrames>();
    #[cfg(feature = "input")]
    app.init_resource::<ambition_platformer2d::input::MenuShortcutPages>()
        .add_systems(Update, publish_shortcut_pages.in_set(ambition_platformer2d::input::InputSet::ResolveContext));
    app.add_systems(Update, sync_session_menus
        .after(ambition_platformer2d::input::InputSet::Route)
        .before(ambition_platformer2d::input::MenuNavConsume));
}

#[cfg(feature = "input")]
fn publish_shortcut_pages(
    room: ambition_platformer2d::runtime::CurrentRoom,
    backend: Option<Res<InventoryUiBackend>>,
    mut pages: ResMut<ambition_platformer2d::input::MenuShortcutPages>,
) {
    let available = backend.is_some_and(|backend| backend.effective().is_available())
        && ambition_platformer2d::runtime::in_base_mode(room);
    pages.inventory = available;
    pages.map = available;
}

/// Menu lifetime follows session admission and local input authority.
fn sync_session_menus(
    mut commands: Commands,
    scope: ambition_platformer2d::platformer::lifecycle::LiveSessionScope,
    roots: Query<(
        &ambition_platformer2d::platformer::lifecycle::SessionRoot,
        &ambition_platformer2d::session::participants::SessionParticipants,
    )>,
    local: Option<Res<ambition_platformer2d::rollback::LocalPlayers>>,
    frames: Res<SeatMenuFrames>,
    menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        Option<&ambition_platformer2d::platformer::lifecycle::SessionScopedEntity>,
    ), With<super::MenuInstance>>,
) {
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::platformer::lifecycle::SpawnSessionScopedExt;

    let scope = scope.get();
    let members = scope.and_then(|scope| {
        roots.iter().find(|(root, _)| root.0 == scope).map(|(_, members)| members)
    });
    let is_local = |slot: PlayerSlot| local.as_ref()
        .is_none_or(|local| local.0.contains(&usize::from(slot.0)));
    for (entity, menu, owner) in &menus {
        let slot = PlayerSlot(menu.owner.slot());
        if owner.map(|owner| owner.0) != scope
            || !members.is_some_and(|members| members.contains(slot))
            || !is_local(slot)
        {
            commands.entity(entity).despawn();
        }
    }
    let (Some(scope), Some(members)) = (scope, members) else { return; };
    for slot in members.iter().filter(|slot| is_local(*slot)) {
        let owner = ParticipantId(slot.0);
        if menus.iter().any(|(_, menu, scoped)| {
            menu.owner == owner && scoped.is_some_and(|scoped| scoped.0 == scope)
        }) {
            continue;
        }
        commands.spawn_in_session(scope, (
            super::MenuInstance,
            ambition_platformer2d::inventory_ui::InventoryMenu { owner },
            ActiveMenuPages::<MenuPage, MenuPageAction>::default(),
            MenuLifecycleState { last_start: frames.for_seat(slot.0).start },
        ));
    }
}

#[derive(Default)]
pub(crate) struct MenuPauseState {
    scope: Option<ambition_platformer2d::platformer::lifecycle::SessionScopeId>,
    initialized: bool,
    hold: ambition_platformer2d::platformer::lifecycle::MenuPauseHold,
}

/// Apply the combined pause claim after all menu input and actions.
pub(crate) fn aggregate_menu_pause(
    mut menus: Query<(&ambition_platformer2d::inventory_ui::InventoryMenu, &mut InventoryUiState)>,
    roster: Option<Res<ambition_platformer2d::platformer::lifecycle::MenuPauseRoster>>,
    mut votes: Option<ResMut<ambition_platformer2d::platformer::lifecycle::LocalMenuPauseVotes>>,
    network: Option<Res<ambition_platformer2d::platformer::lifecycle::NetworkMenuPause>>,
    mode: Res<State<GameMode>>,
    mut next: ResMut<NextState<GameMode>>,
    scope: ambition_platformer2d::platformer::lifecycle::LiveSessionScope,
    ownership: Option<Res<ambition_platformer2d::rollback::RollbackSessionOwnership>>,
    mut pause: Local<MenuPauseState>,
) {
    let scope = scope.get();
    if pause.initialized && pause.scope != scope {
        for (_, mut menu) in &mut menus { menu.close(); }
        if let Some(votes) = votes.as_mut() { votes.publish(scope, []); }
        pause.hold = Default::default();
        pause.scope = scope;
        return;
    }
    pause.initialized = true;
    pause.scope = scope;
    if let Some(votes) = votes.as_mut() {
        votes.publish(scope, menus.iter().filter(|(_, state)| state.visible).map(|(menu, _)| menu.owner.slot()));
    }
    // Use the admitted roster, not the number of controllers or visible menus.
    // Without the host roster, isolated menu fixtures still exercise the
    // same nonempty unanimity rule. Remote votes must come from an agreed
    // control-plane state; a local request cannot stop a peer timeline.
    let remote = network.is_some() || ownership.is_some_and(|ownership| ownership.has_remote_peers());
    let open = !remote && match roster.as_deref() {
        Some(roster) => roster.scope == scope && roster.session_pause(|slot| {
            menus.iter().any(|(menu, state)| menu.owner.slot() == slot && state.visible)
        }),
        None => !menus.is_empty() && menus.iter().all(|(_, state)| state.visible),
    };
    let requested = match &*next {
        NextState::Pending(mode) | NextState::PendingIfNeq(mode) => Some(*mode),
        NextState::Unchanged => None,
    };
    if let Some(target) = pause.hold.update(open, *mode.get(), requested) {
        ambition_platformer2d::platformer::world_log::note_game_mode_request(target, "menu_pause_vote");
        next.set(target);
    }
}

/// The Start edge state of one inventory surface.
#[derive(Component, Default)]
pub(crate) struct MenuLifecycleState {
    last_start: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn route_menu_lifecycle(
    access: ambition_platformer2d::input::participant::ParticipantContextAccess,
    mut frames: ResMut<SeatMenuFrames>,
    backend: Res<InventoryUiBackend>,
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        &mut MenuLifecycleState,
        &mut InventoryUiState,
        &mut ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut KaleidoscopeCursor,
        &mut KaleidoscopeSystemNav,
        &mut VisualQualityConfirmState,
        &RebindCapture,
    )>,
    mode: Res<State<GameMode>>,
    mut map: ResMut<MapMenuState>,
    mut sfx: SfxWriter,
    mut pause: MessageReader<PauseRequested>,
    #[cfg(feature = "bevy_ui_menu")] mut grid: Query<&mut super::grid_backend::GridMenuTabState>,
) {
    if !backend.effective().is_available() {
        return;
    }
    let platform_pause = pause.read().count() > 0;
    for (
        menu_entity,
        owner,
        mut latch,
        mut overlay,
        mut pages,
        mut cursor,
        mut system,
        mut quality,
        capture,
    ) in &mut menus
    {
        let input = frames.for_seat(owner.owner.slot());
        let start = input.start && !latch.last_start;
        latch.last_start = input.start;
        let platform_pause = platform_pause && owner.owner == ParticipantId::PRIMARY;
        if !platform_pause && !super::inventory_input_allowed(&access, owner.owner) {
            continue;
        }
        if overlay.visible && capture.armed().is_some() {
            *frames.for_seat_mut(owner.owner.slot()) = Default::default();
            continue;
        }
        if start || platform_pause || input.inventory || input.map {
            // The opening or closing press belongs only to this menu lifecycle.
            // A dedicated Start can share a press with another menu control.
            let frame = frames.for_seat_mut(owner.owner.slot());
            frame.consume_nav_edges();
            frame.select_held = false;
            frame.back_held = false;
        }

        let page = if platform_pause {
            if overlay.visible {
                continue;
            }
            MenuPage::System
        } else if start {
            if overlay.visible {
                quality.cancel();
                overlay.close();
                play_ui(&mut sfx, ids::UI_MENU_CLOSE);
                continue;
            }
            MenuPage::System
        } else if input.inventory {
            MenuPage::Items
        } else if input.map {
            MenuPage::Map
        } else {
            continue;
        };

        if overlay.visible {
            if super::model::open_overlay_key(page, pages.active) == OpenOverlayKey::Close {
                quality.cancel();
                overlay.close();
                play_ui(&mut sfx, ids::UI_MENU_CLOSE);
                continue;
            }
        } else {
            if !super::model::overlay_may_open(*mode.get()) {
                continue;
            }
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
        if let Ok(mut grid) = grid.get_mut(menu_entity) {
            grid.reset_navigation();
        }
    }
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
    fn unanimous_menu_pause_restores_the_underlying_mode_when_any_player_resumes() {
        for base in [GameMode::Playing, GameMode::Dialogue, GameMode::Cutscene, GameMode::Paused] {
            for first in 0..2 {
                let (mut app, menus) = pause_app(base);
                visible(&mut app, menus[first], true);
                settle(&mut app);
                assert_eq!(mode(&app), base);
                visible(&mut app, menus[1 - first], true);
                settle(&mut app);
                assert_eq!(mode(&app), GameMode::Paused);
                visible(&mut app, menus[first], false);
                settle(&mut app);
                assert_eq!(mode(&app), base);
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
        assert_eq!(mode(&app), GameMode::Playing, "one player resumed");
        visible(&mut app, menus[1], false);
        settle(&mut app);
        assert_eq!(mode(&app), GameMode::Playing, "the transition replaced the old dialogue mode");
    }

    #[test]
    fn a_mode_request_in_the_open_frame_is_not_replaced_by_menu_pause() {
        #[derive(Resource)]
        struct RequestedMode(Option<GameMode>);
        for (base, requested) in [
            (GameMode::Playing, GameMode::RoomTransition),
            (GameMode::Dialogue, GameMode::Playing),
            (GameMode::Playing, GameMode::Paused),
        ] {
            let (mut app, menus) = pause_app(base);
            app.world_mut().entity_mut(menus[1]).despawn();
            app.insert_resource(RequestedMode(Some(requested)));
            app.add_systems(Update, |mut request: ResMut<RequestedMode>, mut next: ResMut<NextState<GameMode>>| {
                if let Some(request) = request.0.take() { next.set(request); }
            });
            visible(&mut app, menus[0], true);
            settle(&mut app);
            assert_eq!(mode(&app), if requested == GameMode::RoomTransition {
                GameMode::RoomTransition
            } else { GameMode::Paused });
            visible(&mut app, menus[0], false);
            settle(&mut app);
            assert_eq!(mode(&app), requested, "closing cannot restore a mode that the other request replaced");
        }
    }

    #[test]
    fn session_retirement_closes_menus_without_restoring_an_old_mode() {
        let (mut app, menus) = pause_app(GameMode::Dialogue);
        let root = app.world_mut().spawn(SessionRoot(SessionScopeId(1))).id();
        app.update();
        visible(&mut app, menus[0], true);
        visible(&mut app, menus[1], true);
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
        visible(&mut app, menus[1], true);
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

#[cfg(test)]
mod admission_tests;
