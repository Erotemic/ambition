//! Pointer input for the shared map viewport.

use bevy::picking::{
    events::{self, Pointer, Press},
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    PickingSystems,
};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use ambition_platformer2d::inventory_ui::{InventoryMenu, InventoryUiState};
use ambition_platformer2d::menu::{
    backend::InventoryUiBackend,
    map::browser::{MapAction, MapBrowser},
    render::bevy_ui::logical_node_rect,
    ActiveMenuPages, AmbitionMenuControl, MenuInteraction, MenuSurface,
};

use super::{
    effects::MenuBodySource,
    map_page::{MapPageSource, MAP_FRAME},
    model::{MenuPage, MenuPageAction},
    regions::{MenuRegion, MenuWheelInput},
};

#[derive(Component, Default)]
pub(crate) struct MapGesture {
    first: Option<Contact>,
    second: Option<Contact>,
    was_active: bool,
}

struct Contact {
    pointer: PointerId,
    origin: Vec2,
    last: Location,
    moved: bool,
}

impl Contact {
    fn from_press(event: &Pointer<Press>) -> Self {
        Self {
            pointer: event.pointer_id,
            origin: event.pointer_location.position,
            last: event.pointer_location.clone(),
            moved: false,
        }
    }
}

#[derive(Clone, Copy)]
struct TouchPair {
    midpoint: Vec2,
    span: f32,
}

impl MapGesture {
    fn contacts(&self) -> impl Iterator<Item = &Contact> {
        self.first.iter().chain(self.second.iter())
    }

    fn pair(&self) -> Option<TouchPair> {
        let a = self.first.as_ref()?.last.position;
        let b = self.second.as_ref()?.last.position;
        Some(TouchPair {
            midpoint: (a + b) * 0.5,
            span: a.distance(b),
        })
    }

    fn contact_mut(&mut self, pointer: PointerId) -> Option<&mut Contact> {
        self.first
            .iter_mut()
            .chain(self.second.iter_mut())
            .find(|contact| contact.pointer == pointer)
    }

    fn release(&mut self, pointer: PointerId) {
        if self
            .first
            .as_ref()
            .is_some_and(|contact| contact.pointer == pointer)
        {
            self.first = self.second.take();
        } else if self
            .second
            .as_ref()
            .is_some_and(|contact| contact.pointer == pointer)
        {
            self.second = None;
        }
    }
}

pub(crate) fn install(app: &mut App) {
    let reset = reset_inactive_maps
        .after(super::MenuActionsApplied)
        .before(super::system_cache::cache_system_menu);
    #[cfg(all(feature = "bevy_ui_menu", feature = "input"))]
    let reset = reset.after(super::grid_backend::grid_menu_tab_activated);
    app.add_message::<PointerInput>()
        .add_observer(press)
        .add_systems(
            PreUpdate,
            cancel_taps_on_motion
                .after(PickingSystems::ProcessInput)
                .before(events::pointer_events),
        )
        .add_systems(
            Update,
            update
                .after(ambition_platformer2d::input::MenuNavConsume)
                .before(super::MenuActionsApplied),
        )
        .add_systems(Update, reset);
}

fn is_active(state: &InventoryUiState, pages: &ActiveMenuPages<MenuPage, MenuPageAction>) -> bool {
    state.visible && pages.active == Some(MenuPage::Map)
}

fn reset_inactive_maps(
    mut menus: Query<(
        &InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut MapBrowser,
        &mut MapGesture,
    )>,
) {
    for (state, pages, mut browser, mut gesture) in &mut menus {
        let active = is_active(state, pages);
        if !active && gesture.was_active {
            *browser = MapBrowser::default();
            *gesture = MapGesture::default();
        } else if active != gesture.was_active {
            gesture.was_active = active;
        }
    }
}

fn press(
    event: On<Pointer<Press>>,
    controls: Query<(&AmbitionMenuControl<MenuPageAction>, &MenuSurface)>,
    mut menus: Query<(
        &InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut MapGesture,
        &mut MenuInteraction<MenuPageAction>,
    )>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok((control, surface)) = controls.get(event.entity) else {
        return;
    };
    if !matches!(
        control.action,
        Some(MenuPageAction::Map(MapAction::Open(_) | MapAction::Canvas))
    ) {
        return;
    }
    let Ok((state, pages, mut gesture, mut interaction)) = menus.get_mut(surface.0) else {
        return;
    };
    if !is_active(state, pages) {
        return;
    }
    if gesture
        .contacts()
        .any(|contact| contact.pointer == event.pointer_id)
    {
        return;
    }
    let can_pinch = gesture.second.is_none()
        && event.pointer_id.is_touch()
        && gesture.first.as_ref().is_some_and(|first| {
            first.pointer.is_touch() && first.last.target == event.pointer_location.target
        });
    if let Some(first) = &mut gesture.first {
        interaction.suppress_pointer(event.pointer_id);
        if can_pinch {
            interaction.suppress_pointer(first.pointer);
            first.moved = true;
            let mut second = Contact::from_press(&event);
            second.moved = true;
            gesture.second = Some(second);
        }
    } else {
        gesture.first = Some(Contact::from_press(&event));
    }
}

fn cancel_taps_on_motion(
    mut inputs: MessageReader<PointerInput>,
    mut menus: Query<(&MapGesture, &mut MenuInteraction<MenuPageAction>)>,
) {
    for event in inputs.read() {
        if matches!(
            event.action,
            PointerAction::Move { .. } | PointerAction::Release(PointerButton::Primary)
        ) {
            for (gesture, mut interaction) in &mut menus {
                if gesture
                    .contacts()
                    .any(|contact| contact.pointer == event.pointer_id)
                {
                    interaction.move_pointer(event.pointer_id, event.location.position);
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update(
    mut pointer: MessageReader<PointerInput>,
    mut wheel: MenuWheelInput,
    source: MapPageSource,
    backend: Res<InventoryUiBackend>,
    bodies: MenuBodySource,
    canvas_nodes: Query<(
        Entity,
        &MenuSurface,
        &AmbitionMenuControl<MenuPageAction>,
        Option<(&ComputedNode, &UiGlobalTransform, &ComputedUiTargetCamera)>,
    )>,
    cameras: Query<&Camera>,
    #[cfg(feature = "kaleidoscope_menu")]
    cube_geometry: ambition_menu_kaleidoscope::KaleidoscopeControlGeometry,
    mut menus: Query<(
        Entity,
        &InventoryMenu,
        &InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &MenuRegion,
        &mut MapBrowser,
        &mut MapGesture,
        &mut MenuInteraction<MenuPageAction>,
    )>,
) {
    wheel.read();
    let inputs: Vec<_> = pointer.read().collect();
    for (menu, owner, state, pages, region, mut browser, mut gesture, mut interaction) in &mut menus
    {
        let active = is_active(state, pages);
        let scroll = wheel.lines_for(menu);
        let mut drag_delta = Vec2::ZERO;
        let mut pinch: Option<(TouchPair, TouchPair)> = None;
        for event in &inputs {
            match event.action {
                PointerAction::Move { .. } if active => {
                    let before = gesture.pair();
                    let Some(contact) = gesture
                        .contact_mut(event.pointer_id)
                        .filter(|contact| contact.last.target == event.location.target)
                    else {
                        continue;
                    };
                    let at = event.location.position;
                    if contact.moved
                        || at.distance(contact.origin)
                            > ambition_platformer2d::ui_nav::ROW_TAP_SLOP_PX
                    {
                        if before.is_none() {
                            let previous = if contact.moved {
                                contact.last.position
                            } else {
                                contact.origin
                            };
                            drag_delta += previous - at;
                        }
                        contact.moved = true;
                    }
                    contact.last.position = at;
                    if let (Some(before), Some(after)) = (before, gesture.pair()) {
                        pinch.get_or_insert((before, after)).1 = after;
                    }
                }
                PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel => {
                    gesture.release(event.pointer_id);
                    interaction.clear_pointer_suppression(event.pointer_id);
                }
                _ => {}
            }
        }
        if !active || (drag_delta == Vec2::ZERO && scroll == Vec2::ZERO && pinch.is_none()) {
            continue;
        }
        let Some(rect) = region.0 else {
            continue;
        };
        let frame = canvas_nodes
            .iter()
            .find_map(|(_entity, surface, control, grid_node)| {
                if surface.0 != menu
                    || control.action != Some(MenuPageAction::Map(MapAction::Canvas))
                {
                    return None;
                }
                if backend.effective() == InventoryUiBackend::Grid {
                    let (node, transform, target) = grid_node?;
                    return logical_node_rect(node, transform, cameras.get(target.get()?).ok()?);
                }
                #[cfg(feature = "kaleidoscope_menu")]
                return cube_geometry.logical_rect(_entity);
                #[cfg(not(feature = "kaleidoscope_menu"))]
                None
            })
            .unwrap_or_else(|| {
                let min = rect.min + rect.size() * Vec2::new(MAP_FRAME.x, MAP_FRAME.y) * 0.01;
                Rect::from_corners(
                    min,
                    min + rect.size() * Vec2::new(MAP_FRAME.w, MAP_FRAME.h) * 0.01,
                )
            });
        let frame_size = frame.size().max(Vec2::ONE);
        let aspect = super::map_page::frame_aspect(backend.effective(), Some(region));
        let body = bodies.read(owner.owner).map(|body| body.entity);
        if let Some((before, after)) = pinch {
            let scale = if before.span >= 1.0 && after.span >= 1.0 {
                after.span / before.span
            } else {
                1.0
            };
            source.pinch_viewport(
                &mut browser,
                body,
                aspect,
                (before.midpoint - frame.min) / frame_size,
                (after.midpoint - frame.min) / frame_size,
                scale,
            );
        }
        let pan = (drag_delta - Vec2::new(scroll.x * 40.0, 0.0)) / frame_size;
        if pan != Vec2::ZERO || scroll.y != 0.0 {
            source.adjust_viewport(&mut browser, body, pan, scroll.y.clamp(-8.0, 8.0), aspect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::test_support as fixture;
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::menu::map::browser::MapBrowserView;
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::mouse::MouseWheel;
    use bevy::picking::backend::HitData;

    #[test]
    fn map_touch_pair_zooms_without_taps_and_reopening_follows_the_owner() {
        for backend in [
            InventoryUiBackend::Grid,
            InventoryUiBackend::LunexKaleidoscope,
        ]
        .into_iter()
        .filter(|backend| backend.is_available())
        {
            let mut app = App::new();
            app.insert_resource(backend).add_message::<MouseWheel>();
            install(&mut app);
            match backend {
                InventoryUiBackend::Grid => {
                    ambition_platformer2d::menu::render::bevy_ui::install_bevy_ui_menu_actions::<
                        MenuPageAction,
                    >(&mut app)
                }
                InventoryUiBackend::LunexKaleidoscope => {
                    #[cfg(feature = "kaleidoscope_menu")]
                    app.add_observer(super::super::kaleidoscope_app::kaleidoscope_pointer_press);
                }
            }
            let rooms = fixture::two_room_map(&mut app);
            let body = app
                .world_mut()
                .spawn((DrivingParticipant(PlayerSlot(0)), InRoomInstance(rooms[0])))
                .id();
            fixture::spawn_inventory_menu(
                &mut app,
                InventoryUiState {
                    visible: true,
                    ..default()
                },
            );
            let menu = fixture::menu_id(app.world());
            let region = MenuRegion(Some(Rect::from_corners(Vec2::ZERO, Vec2::splat(1000.0))));
            app.world_mut().entity_mut(menu).insert(region);
            fixture::active_pages_mut(app.world_mut()).active = Some(MenuPage::Map);
            let aspect = super::super::map_page::frame_aspect(backend, Some(&region));
            let view = |app: &mut App| -> MapBrowserView {
                app.world_mut()
                    .run_system_once(move |source: MapPageSource, browsers: Query<&MapBrowser>| {
                        source
                            .view(Some(body), Some(browsers.get(menu).unwrap()), aspect)
                            .unwrap()
                    })
                    .unwrap()
            };
            app.update();
            let initial = view(&mut app);
            assert_eq!(initial.title, "ALPHA");
            let action = MenuPageAction::Map(MapAction::Open(initial.entries[0].id));
            let target = fixture::spawn_control(&mut app, action);
            app.world_mut().entity_mut(target).insert(Button);
            let centre = 10.0
                * Vec2::new(
                    MAP_FRAME.x + MAP_FRAME.w * 0.5,
                    MAP_FRAME.y + MAP_FRAME.h * 0.5,
                );
            let fingers = [PointerId::Touch(1), PointerId::Touch(2)];
            let offsets = [-40.0, 40.0];
            let at = |x| fixture::pointer_location_at(centre + Vec2::new(x, 0.0));
            let press = |app: &mut App, id, x| {
                app.world_mut().trigger(Pointer::new(
                    id,
                    at(x),
                    Press {
                        button: PointerButton::Primary,
                        hit: HitData::new(target, 0.0, None, None),
                        count: 1,
                    },
                    target,
                ));
            };
            for (id, x) in fingers.into_iter().zip(offsets) {
                press(&mut app, id, x);
            }
            let move_pair = |app: &mut App, before: f32, after: f32| {
                for (id, x) in fingers.into_iter().zip(offsets) {
                    app.world_mut().write_message(PointerInput::new(
                        id,
                        at(x * after),
                        PointerAction::Move {
                            delta: Vec2::new(x * (after - before), 0.0),
                        },
                    ));
                }
                app.update();
            };
            move_pair(&mut app, 1.0, 2.0);
            let width = |view: MapBrowserView| view.entries[0].pieces[0].width();
            let zoomed = width(view(&mut app));
            assert!(zoomed > width(initial));
            move_pair(&mut app, 2.0, 1.0);
            assert!(width(view(&mut app)) < zoomed);
            for id in fingers {
                let mut interaction =
                    fixture::menu_component_mut::<MenuInteraction<MenuPageAction>>(app.world_mut());
                assert_eq!(interaction.release_pointer(id), None);
                app.world_mut().write_message(PointerInput::new(
                    id,
                    at(0.0),
                    PointerAction::Cancel,
                ));
            }
            app.update();
            press(&mut app, fingers[0], 0.0);
            assert_eq!(
                fixture::menu_component_mut::<MenuInteraction<MenuPageAction>>(app.world_mut())
                    .release_pointer(fingers[0]),
                Some(action)
            );
            app.world_mut()
                .run_system_once(
                    move |source: MapPageSource, mut browsers: Query<&mut MapBrowser>| {
                        source.apply(
                            &mut browsers.get_mut(menu).unwrap(),
                            Some(body),
                            MapAction::Up,
                            aspect,
                        );
                    },
                )
                .unwrap();
            assert!(!view(&mut app).room_detail);
            fixture::active_pages_mut(app.world_mut()).active = Some(MenuPage::Items);
            app.update();
            app.world_mut()
                .entity_mut(body)
                .insert(InRoomInstance(rooms[1]));
            fixture::active_pages_mut(app.world_mut()).active = Some(MenuPage::Map);
            app.update();
            assert_eq!(view(&mut app).title, "BETA");
        }
    }
}
