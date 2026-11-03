//! Menu admission, input latches and linked presentation lifetime.

use super::*;
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::input::{InputParticipant, MenuControlFrame};
use ambition_platformer2d::inventory_ui::InventoryMenu;
use ambition_platformer2d::platformer::lifecycle::{SessionRoot, SessionScopeId, SessionScopedEntity};
use ambition_platformer2d::rollback::LocalPlayers;
use ambition_platformer2d::session::participants::SessionParticipants;

fn admission_app(backend: InventoryUiBackend) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0)));
    app.insert_state(GameMode::Playing);
    app.insert_resource(backend);
    app.insert_resource(LocalPlayers(vec![0, 1]));
    app.init_resource::<MapMenuState>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<PauseRequested>();
    install_session_menus(&mut app);
    app.add_systems(Update, route_menu_lifecycle.in_set(ambition_platformer2d::input::MenuNavConsume));
    let mut members = SessionParticipants::default();
    for slot in [0, 1, 2] { members.admit(PlayerSlot(slot)); }
    let root = app.world_mut().spawn((SessionRoot(SessionScopeId(1)), members)).id();
    (app, root)
}

fn menu_entities(app: &mut App) -> std::collections::BTreeMap<u8, Entity> {
    let world = app.world_mut();
    world.query_filtered::<(Entity, &InventoryMenu), With<super::super::MenuInstance>>()
        .iter(world).map(|(entity, menu)| (menu.owner.slot(), entity)).collect()
}

#[test]
fn local_members_keep_their_menus_without_bodies_or_devices() {
    let (mut app, _) = admission_app(InventoryUiBackend::Grid);
    let offered = app.world_mut().spawn(InputParticipant::with_id(ParticipantId(3))).id();
    let admitted_device = app.world_mut().spawn(InputParticipant::with_id(ParticipantId(1))).id();
    app.update();
    let before = menu_entities(&mut app);
    assert_eq!(before.keys().copied().collect::<Vec<_>>(), vec![0, 1]);
    for entity in before.values() {
        assert_eq!(app.world().get::<SessionScopedEntity>(*entity).unwrap().0, SessionScopeId(1));
        assert!(app.world().get::<ambition_platformer2d::rollback::Rollback>(*entity).is_none());
    }
    app.world_mut().entity_mut(offered).despawn();
    app.world_mut().entity_mut(admitted_device).despawn();
    app.update();
    assert_eq!(menu_entities(&mut app), before);

    app.world_mut().resource_mut::<LocalPlayers>().0 = vec![1, 2];
    app.update();
    let after = menu_entities(&mut app);
    assert_eq!(after.keys().copied().collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(after[&1], before[&1]);
    assert!(app.world().get_entity(before[&0]).is_err());
}

#[test]
fn participant_and_session_retirement_remove_linked_presentations() {
    use ambition_platformer2d::menu::render::MenuPresentationOf;
    let (mut app, root) = admission_app(InventoryUiBackend::Grid);
    app.update();
    let menu = menu_entities(&mut app)[&1];
    let presentation = app.world_mut().spawn(MenuPresentationOf(menu)).id();
    let child = app.world_mut().spawn(ChildOf(presentation)).id();
    app.world_mut().get_mut::<SessionParticipants>(root).unwrap().remove(PlayerSlot(1));
    app.update();
    for entity in [menu, presentation, child] {
        assert!(app.world().get_entity(entity).is_err());
    }
    app.world_mut().get_mut::<SessionParticipants>(root).unwrap().admit(PlayerSlot(1));
    app.update();
    let replacement = menu_entities(&mut app)[&1];
    assert!(!app.world().get::<InventoryUiState>(replacement).unwrap().visible);
    app.world_mut().entity_mut(root).despawn();
    let mut members = SessionParticipants::default();
    members.admit(PlayerSlot(0));
    let next_root = app.world_mut().spawn((SessionRoot(SessionScopeId(2)), members)).id();
    app.update();
    let next = menu_entities(&mut app);
    assert_eq!(next.len(), 1);
    assert_eq!(app.world().get::<SessionScopedEntity>(next[&0]).unwrap().0, SessionScopeId(2));
    assert!(app.world().get_entity(replacement).is_err());
    app.world_mut().entity_mut(next_root).despawn();
    app.update();
    assert!(menu_entities(&mut app).is_empty());
}

#[test]
fn the_joining_start_does_not_open_either_menu() {
    for backend in [InventoryUiBackend::Grid, InventoryUiBackend::LunexKaleidoscope]
        .into_iter().filter(|backend| backend.is_available())
    {
        let (mut app, _) = admission_app(backend);
        for slot in [0, 1] {
            app.world_mut().resource_mut::<SeatMenuFrames>().for_seat_mut(slot).start = true;
        }
        app.update();
        let menus = menu_entities(&mut app);
        for menu in menus.values() {
            assert!(!app.world().get::<InventoryUiState>(*menu).unwrap().visible);
        }
        app.world_mut().resource_mut::<SeatMenuFrames>().for_seat_mut(1).start = false;
        app.update();
        app.world_mut().resource_mut::<SeatMenuFrames>().for_seat_mut(1).start = true;
        app.update();
        assert!(app.world().get::<InventoryUiState>(menus[&1]).unwrap().visible);
        assert!(!app.world().get::<InventoryUiState>(menus[&0]).unwrap().visible);
    }
}

#[test]
fn rebind_capture_owns_its_menu_shortcuts_and_leaves_the_other_menu_usable() {
    let (mut app, _) = admission_app(InventoryUiBackend::Grid);
    app.update();
    let menus = menu_entities(&mut app);
    app.world_mut().get_mut::<InventoryUiState>(menus[&1]).unwrap().visible = true;
    app.world_mut().get_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>(menus[&1]).unwrap()
        .active = Some(MenuPage::System);
    app.world_mut().get_mut::<RebindCapture>(menus[&1]).unwrap().arm(0);
    {
        let mut frames = app.world_mut().resource_mut::<SeatMenuFrames>();
        frames.for_seat_mut(0).inventory = true;
        let captured = frames.for_seat_mut(1);
        captured.start = true;
        captured.inventory = true;
        captured.map = true;
    }
    app.update();
    assert!(app.world().get::<InventoryUiState>(menus[&1]).unwrap().visible);
    assert_eq!(app.world().get::<ActiveMenuPages<MenuPage, MenuPageAction>>(menus[&1]).unwrap().active, Some(MenuPage::System));
    assert!(app.world().get::<InventoryUiState>(menus[&0]).unwrap().visible);
    assert_eq!(app.world().get::<ActiveMenuPages<MenuPage, MenuPageAction>>(menus[&0]).unwrap().active, Some(MenuPage::Items));
}

#[test]
fn another_surface_keeps_its_frames_and_prevents_menu_opening() {
    use ambition_platformer2d::input::{ContextClaim, LOADING_CONTEXT, ParticipantContexts, participant::context_priority};
    let (mut app, _) = admission_app(InventoryUiBackend::Grid);
    let mut contexts = ParticipantContexts::default();
    contexts.declare(ContextClaim::capturing(LOADING_CONTEXT, context_priority::LOADING));
    let participant = app.world_mut().spawn((InputParticipant::with_id(ParticipantId::SECONDARY), contexts)).id();
    app.update();
    app.world_mut().resource_mut::<SeatMenuFrames>().set(0, MenuControlFrame { start: true, ..default() });
    let input = MenuControlFrame { start: true, select: true, ..default() };
    app.world_mut().resource_mut::<SeatMenuFrames>().set(1, input);
    app.update();
    let menus = menu_entities(&mut app);
    assert!(app.world().get::<InventoryUiState>(menus[&0]).unwrap().visible);
    assert!(!app.world().get::<InventoryUiState>(menus[&1]).unwrap().visible);
    assert_eq!(app.world().resource::<SeatMenuFrames>().for_seat(1), input);
    app.world_mut().get_mut::<ParticipantContexts>(participant).unwrap().retract(LOADING_CONTEXT);
    app.update();
    assert!(!app.world().get::<InventoryUiState>(menus[&1]).unwrap().visible, "a held Start is not a new press");
    app.world_mut().resource_mut::<SeatMenuFrames>().set(1, MenuControlFrame::default());
    app.update();
    app.world_mut().resource_mut::<SeatMenuFrames>().set(1, input);
    app.update();
    assert!(app.world().get::<InventoryUiState>(menus[&1]).unwrap().visible);
}
