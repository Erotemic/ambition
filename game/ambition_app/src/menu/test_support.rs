//! Backend fixtures send picking events to explicit entities.
//!
//! These helpers do not run hit testing. Use a rendered capture with touch
//! input or a real window to check that the visible control receives a tap.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Move, Over, Pointer, Press, Release};
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::prelude::*;

use ambition_platformer2d::menu::{AmbitionMenuControl, MenuControlKind, MenuFocusKey};

use crate::menu::model::MenuPageAction;

pub(crate) fn pointer_location_at(position: Vec2) -> Location {
    Location {
        target: NormalizedRenderTarget::None {
            width: 1,
            height: 1,
        },
        position,
    }
}

pub(crate) fn pointer_location() -> Location {
    pointer_location_at(Vec2::ZERO)
}

pub(crate) fn spawn_control(app: &mut App, action: MenuPageAction) -> Entity {
    let menu = menu_id(app.world());
    app.world_mut()
        .spawn((
            ambition_platformer2d::menu::MenuSurface(menu),
            AmbitionMenuControl::<MenuPageAction> {
                kind: MenuControlKind::OptionToggle,
                action: Some(action),
                focus: MenuFocusKey::default(),
            },
        ))
        .id()
}

pub(crate) fn trigger_press(app: &mut App, entity: Entity) {
    let event = Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
            // bevy 0.19: consecutive-press counter; a single synthetic press is 1.
            count: 1,
        },
        entity,
    );
    if let Some(mut messages) = app.world_mut().get_resource_mut::<Messages<_>>() {
        messages.write(event.clone());
    }
    app.world_mut().trigger(event);
}

pub(crate) fn trigger_release(app: &mut App, entity: Entity) {
    let event = Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Release {
            button: PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    );
    if let Some(mut messages) = app.world_mut().get_resource_mut::<Messages<_>>() {
        messages.write(event.clone());
    }
    if let Some(mut messages) = app.world_mut().get_resource_mut::<Messages<bevy::picking::pointer::PointerInput>>() {
        messages.write(bevy::picking::pointer::PointerInput::new(
            PointerId::Mouse,
            pointer_location(),
            bevy::picking::pointer::PointerAction::Release(PointerButton::Primary),
        ));
    }
    app.world_mut().trigger(event);
}

pub(crate) fn trigger_over(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Over {
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    ));
}

pub(crate) fn trigger_move(app: &mut App, entity: Entity, delta: Vec2) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Move {
            hit: HitData::new(entity, 0.0, None, None),
            delta,
        },
        entity,
    ));
}

pub(crate) fn click_control(app: &mut App, action: MenuPageAction) {
    let entity = spawn_control(app, action);
    trigger_press(app, entity);
    trigger_release(app, entity);
    app.update();
}

/// The single inventory surface used by these backend fixtures.
pub(crate) fn inventory_state(world: &World) -> &ambition_platformer2d::inventory_ui::InventoryUiState {
    world.try_query::<&ambition_platformer2d::inventory_ui::InventoryUiState>()
        .expect("the fixture has inventory state").single(world).expect("one fixture menu")
}

pub(crate) fn inventory_state_mut(world: &mut World) -> Mut<'_, ambition_platformer2d::inventory_ui::InventoryUiState> {
    let entity = world.query_filtered::<Entity, With<ambition_platformer2d::inventory_ui::InventoryUiState>>()
        .single(world).expect("one fixture menu");
    world.get_mut(entity).unwrap()
}

pub(crate) fn spawn_inventory_menu(
    app: &mut App,
    state: ambition_platformer2d::inventory_ui::InventoryUiState,
) {
    app.world_mut().spawn(
        (
            crate::menu::MenuInstance,
            ambition_platformer2d::inventory_ui::InventoryMenu {
                owner: ambition_platformer2d::input::ParticipantId::PRIMARY,
            },
            state,
            ambition_platformer2d::menu::ActiveMenuPages::<
                crate::menu::model::MenuPage,
                MenuPageAction,
            >::default(),
        ),
    );
}

type Pages = ambition_platformer2d::menu::ActiveMenuPages<crate::menu::model::MenuPage, MenuPageAction>;

pub(crate) fn active_pages(world: &World) -> &Pages {
    world.try_query::<&Pages>().expect("fixture page model")
        .single(world).expect("one fixture menu")
}

pub(crate) fn active_pages_mut(world: &mut World) -> Mut<'_, Pages> {
    let entity = world.query_filtered::<Entity, With<Pages>>().single(world).expect("one fixture menu");
    world.get_mut(entity).unwrap()
}

pub(crate) fn menu_id(world: &World) -> Entity {
    world
        .try_query_filtered::<Entity, With<ambition_platformer2d::inventory_ui::InventoryMenu>>()
        .expect("fixture menu owner")
        .single(world)
        .expect("one fixture menu")
}

pub(crate) fn scroll_drag(app: &mut App, fraction: f32) {
    let menu = menu_id(app.world());
    app.world_mut()
        .write_message(ambition_platformer2d::menu::MenuScrollDragged { menu, fraction });
}

pub(crate) fn two_room_map(app: &mut App) -> [ambition_platformer2d::platformer::lifecycle::LiveRoomInstance; 2] {
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::platformer::lifecycle::{insert_session_world_component, spawn_live_room, LiveRoomInstance};
    use ambition_platformer2d::world::rooms::{LiveRoomDefinition, RoomSet, RoomSpec};
    use ambition_platformer2d::menu::map::{MapMenuState, MapRoomNode};
    let definitions: Vec<_> = ["alpha", "beta"].into_iter().map(|id| RoomSpec::new(id, ae::World::new(
        id, ae::Vec2::splat(400.0), ae::Vec2::splat(50.0), Vec::new(),
    ))).collect();
    insert_session_world_component(app.world_mut(), RoomSet::from_parts_or_panic("alpha", definitions, Vec::new()));
    let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for (index, room) in rooms.into_iter().enumerate() {
        spawn_live_room(app.world_mut(), room, LiveRoomDefinition::from_index(index));
    }
    let mut map = MapMenuState::default();
    map.visited.extend(["alpha".into(), "beta".into()]);
    map.rooms = ["alpha", "beta"].into_iter().enumerate().map(|(index, id)| MapRoomNode {
        id: id.into(), title: id.to_uppercase(), world: "fixture".into(),
        world_min: ae::Vec2::new(index as f32 * 400.0, 0.0), world_size: ae::Vec2::splat(400.0),
        edge_exits: vec![if index == 0 { "beta" } else { "alpha" }.into()],
        ..Default::default()
    }).collect();
    app.insert_resource(map);
    rooms
}

pub(crate) fn menu_component<T: Component>(world: &World) -> &T {
    world.get::<T>(menu_id(world)).expect("fixture menu component")
}

pub(crate) fn menu_component_mut<T: Component<Mutability = bevy::ecs::component::Mutable>>(world: &mut World) -> Mut<'_, T> {
    let menu = menu_id(world);
    world.get_mut::<T>(menu).expect("fixture menu component")
}

pub(crate) fn set_menu_frame(
    world: &mut World,
    frame: ambition_platformer2d::input::MenuControlFrame,
) {
    let owner = world
        .get::<ambition_platformer2d::inventory_ui::InventoryMenu>(menu_id(world))
        .unwrap()
        .owner;
    world
        .resource_mut::<ambition_platformer2d::input::SeatMenuFrames>()
        .set(owner.slot(), frame);
}

pub(crate) fn menu_frame_mut(
    world: &mut World,
) -> Mut<'_, ambition_platformer2d::input::MenuControlFrame> {
    let owner = world
        .get::<ambition_platformer2d::inventory_ui::InventoryMenu>(menu_id(world))
        .unwrap()
        .owner;
    world
        .resource_mut::<ambition_platformer2d::input::SeatMenuFrames>()
        .map_unchanged(|frames| frames.for_seat_mut(owner.slot()))
}


#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
pub(crate) fn add_scroll_window(app: &mut App) -> Entity {
    let menu = menu_id(app.world());
    app.world_mut().get_mut::<super::regions::MenuRegion>(menu).unwrap().0 =
        Some(Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)));
    let mut window = Window { resolution: (1280, 720).into(), ..default() };
    window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
    app.world_mut().spawn((window, bevy::window::PrimaryWindow)).id()
}

#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
pub(crate) fn wheel(app: &mut App, y: f32) {
    let window = app.world_mut().query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world()).unwrap();
    app.world_mut().write_message(bevy::input::mouse::MouseWheel {
        unit: bevy::input::mouse::MouseScrollUnit::Line,
        x: 0.0,
        y,
        window,
        phase: bevy::input::touch::TouchPhase::Moved,
    });
}


#[cfg(feature = "input")]
pub(crate) fn spawn_binding_participant(app: &mut App, owner: ambition_platformer2d::input::ParticipantId, recipe: ambition_platformer2d::input::BindingRecipe) -> Entity {
    app.world_mut().spawn((
        ambition_platformer2d::input::InputParticipant::with_id(owner),
        recipe.build(),
        recipe,
        leafwing_input_manager::prelude::ActionState::<ambition_platformer2d::input::Platformer2dInputActionMonolith>::default(),
    )).id()
}
