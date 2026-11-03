//! Menu regions shared by camera and node presentation.

use ambition_platformer2d::input::InputOwner;
use ambition_platformer2d::inventory_ui::InventoryMenu;
use ambition_platformer2d::menu::MenuSurface;
use ambition_platformer2d::render::ui_surface::ParticipantSurface;
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default, PartialEq)]
pub(crate) struct MenuRegion(pub Option<Rect>);

/// Wheel input for the visible menu under the mouse in its display window.
#[cfg(any(feature = "input", feature = "kaleidoscope_menu"))]
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuWheelInput<'w, 's> {
    events: MessageReader<'w, 's, bevy::input::mouse::MouseWheel>,
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    menus: Query<
        'w,
        's,
        (
            Entity,
            &'static MenuRegion,
            &'static ambition_platformer2d::inventory_ui::InventoryUiState,
        ),
        With<InventoryMenu>,
    >,
    steps: Local<'s, Vec<(Entity, i32)>>,
}

#[cfg(any(feature = "input", feature = "kaleidoscope_menu"))]
impl MenuWheelInput<'_, '_> {
    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.steps.clear();
    }

    pub(crate) fn read(&mut self) {
        self.steps.clear();
        for event in self.events.read() {
            let Some(position) = self
                .windows
                .get(event.window)
                .ok()
                .and_then(Window::cursor_position)
            else {
                continue;
            };
            let mut targets = self.menus.iter().filter(|(_, region, state)| {
                state.visible
                    && region.0.is_some_and(|rect| {
                        position.cmpge(rect.min).all() && position.cmplt(rect.max).all()
                    })
            });
            let Some((menu, _, _)) = targets.next() else {
                continue;
            };
            if targets.next().is_some() {
                continue;
            }
            let delta = if event.y > 0.0 {
                -1
            } else if event.y < 0.0 {
                1
            } else {
                0
            };
            if let Some((_, steps)) = self.steps.iter_mut().find(|(owner, _)| *owner == menu) {
                *steps += delta;
            } else {
                self.steps.push((menu, delta));
            }
        }
    }

    pub(crate) fn steps_for(&self, menu: Entity) -> i32 {
        self.steps
            .iter()
            .find(|(owner, _)| *owner == menu)
            .map_or(0, |(_, steps)| *steps)
    }
}

pub(crate) fn resolve_menu_regions(
    surface: ParticipantSurface,
    owners: Query<(Entity, &InventoryMenu)>,
    mut regions: Query<&mut MenuRegion>,
    mut ordered: Local<Vec<(u8, Entity, Option<Rect>)>>,
) {
    ordered.clear();
    ordered.extend(owners.iter().map(|(entity, menu)| {
        (
            menu.owner.slot(),
            entity,
            surface.rect(Some(InputOwner::Participant(menu.owner))),
        )
    }));
    // A sole local menu owns the display, including gameplay framing margins.
    if let [(_, _, region)] = ordered.as_mut_slice() {
        *region = region.map(|_| surface.display_rect());
    }
    ordered.sort_by_key(|(owner, entity, _)| (*owner, *entity));
    for (index, (owner, menu, region)) in ordered.iter().enumerate() {
        let Ok(mut target) = regions.get_mut(*menu) else {
            continue;
        };
        let wanted = region.and_then(|region| {
            if ordered
                .iter()
                .filter(|(other, _, _)| other == owner)
                .count()
                != 1
            {
                return None;
            }
            let count = ordered
                .iter()
                .filter(|(_, _, other)| *other == Some(region))
                .count();
            let ordinal = ordered[..index]
                .iter()
                .filter(|(_, _, other)| *other == Some(region))
                .count();
            let width = region.width() / count as f32;
            let min = region.min + Vec2::new(width * ordinal as f32, 0.0);
            Some(Rect::from_corners(
                min,
                Vec2::new(min.x + width, region.max.y),
            ))
        });
        if target.0 != wanted {
            target.0 = wanted;
        }
    }
}

pub(crate) fn place_menu_nodes(
    surface: ParticipantSurface,
    regions: Query<&MenuRegion>,
    mut roots: Query<
        (&MenuSurface, &mut Node),
        With<ambition_platformer2d::menu::render::bevy_ui::BevyUiMenuRoot>,
    >,
) {
    for (owner, mut node) in &mut roots {
        let Ok(region) = regions.get(owner.0) else {
            continue;
        };
        let mut wanted = node.clone();
        if let Some(rect) = region.0 {
            surface.place(&mut wanted, rect);
            wanted.display = Display::Flex;
        } else {
            wanted.display = Display::None;
        }
        if *node != wanted {
            *node = wanted;
        }
    }
}

#[cfg(feature = "kaleidoscope_menu")]
pub(crate) fn place_cube_regions(
    surface: ParticipantSurface,
    regions: Query<&MenuRegion>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<
        (&MenuSurface, &mut Camera),
        With<ambition_menu_kaleidoscope::KaleidoscopePauseCamera>,
    >,
    mut scrims: Query<(&MenuSurface, &mut Node), With<super::kaleidoscope_app::KaleidoscopeScrim>>,
) {
    let scale = windows.single().map_or(1.0, Window::scale_factor);
    for (owner, mut camera) in &mut cameras {
        let Some(rect) = regions.get(owner.0).ok().and_then(|region| region.0) else {
            if camera.is_active {
                camera.is_active = false;
            }
            continue;
        };
        let min = (rect.min * scale).round().as_uvec2();
        let max = (rect.max * scale).round().as_uvec2();
        let viewport = bevy::camera::Viewport {
            physical_position: min,
            physical_size: max - min,
            ..default()
        };
        if camera.viewport.as_ref().is_none_or(|current| {
            current.physical_position != viewport.physical_position
                || current.physical_size != viewport.physical_size
        }) {
            camera.viewport = Some(viewport);
        }
    }
    for (owner, mut node) in &mut scrims {
        let rect = regions.get(owner.0).ok().and_then(|region| region.0);
        let mut wanted = node.clone();
        if let Some(rect) = rect {
            surface.place(&mut wanted, rect);
            wanted.display = Display::Flex;
        } else {
            wanted.display = Display::None;
        }
        if *node != wanted {
            *node = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::input::ParticipantId;
    use ambition_platformer2d::sim_view::{LocalView, LocalViewId, ViewParticipant, ViewPlacement};

    #[test]
    fn menu_regions_share_the_dialogue_view_rule_and_split_a_shared_view() {
        let mut app = App::new();
        app.add_systems(Update, resolve_menu_regions);
        let menus = [ParticipantId::PRIMARY, ParticipantId::SECONDARY].map(|owner| {
            app.world_mut()
                .spawn((super::super::MenuInstance, InventoryMenu { owner }))
                .id()
        });
        let views = [0, 1].map(|seat| {
            app.world_mut()
                .spawn((
                    LocalView,
                    LocalViewId(seat),
                    ViewParticipant(PlayerSlot(seat as u8)),
                    ViewPlacement::column(seat as usize, 2),
                ))
                .id()
        });
        app.update();
        let rectangles =
            |app: &App| menus.map(|menu| app.world().get::<MenuRegion>(menu).unwrap().0.unwrap());
        let split = rectangles(&app);
        assert!(split[0].max.x <= split[1].min.x);
        app.world_mut().despawn(views[1]);
        app.world_mut()
            .entity_mut(views[0])
            .remove::<ViewParticipant>();
        app.world_mut()
            .entity_mut(views[0])
            .insert(ViewPlacement::default());
        app.update();
        assert_eq!(rectangles(&app), split);
        app.world_mut().despawn(menus[1]);
        app.update();
        assert_eq!(
            app.world()
                .get::<MenuRegion>(menus[0])
                .unwrap()
                .0
                .unwrap()
                .width(),
            split[0].width() + split[1].width()
        );
    }
    #[test]
    fn a_sole_menu_uses_the_display_and_shared_menus_keep_their_reading_regions() {
        use ambition_platformer2d::presentation::gameplay_presentation::{
            ResolvedGameplayPresentation, ScreenRect,
        };
        let mut app = App::new();
        app.add_systems(Update, resolve_menu_regions);
        let display = Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0));
        let reading = Rect::from_corners(Vec2::new(222.0, 0.0), Vec2::new(1035.0, 720.0));
        app.insert_resource(ResolvedGameplayPresentation {
            display_rect: ScreenRect::from_corners(display.min, display.max),
            display_safe_rect: ScreenRect::from_corners(reading.min, reading.max),
            gameplay_rect: ScreenRect::from_corners(display.min, display.max),
            ..default()
        });
        app.world_mut().spawn((LocalView, LocalViewId(0), ViewPlacement::default()));
        let first = app.world_mut().spawn((super::super::MenuInstance,
            InventoryMenu { owner: ParticipantId::PRIMARY })).id();
        app.update();
        assert_eq!(app.world().get::<MenuRegion>(first).unwrap().0, Some(display));
        let second = app.world_mut().spawn((super::super::MenuInstance,
            InventoryMenu { owner: ParticipantId::SECONDARY })).id();
        app.update();
        let left = app.world().get::<MenuRegion>(first).unwrap().0.unwrap();
        let right = app.world().get::<MenuRegion>(second).unwrap().0.unwrap();
        assert_eq!(left.min, reading.min);
        assert_eq!(left.max.x, right.min.x);
        assert_eq!(right.max, reading.max);
        app.world_mut().despawn(second);
        app.update();
        assert_eq!(app.world().get::<MenuRegion>(first).unwrap().0, Some(display));
    }

    #[cfg(feature = "kaleidoscope_menu")]
    #[test]
    fn adjacent_menu_viewports_share_one_rounded_edge_at_fractional_scale() {
        let mut app = App::new();
        app.add_systems(Update, place_cube_regions);
        let mut window = Window::default();
        window.resolution.set_scale_factor_override(Some(1.25));
        app.world_mut().spawn((window, bevy::window::PrimaryWindow));
        let cameras = [0.0, 320.5].map(|x| {
            let menu = app
                .world_mut()
                .spawn(MenuRegion(Some(Rect::from_corners(
                    Vec2::new(x, 0.0),
                    Vec2::new(x + 320.5, 360.0),
                ))))
                .id();
            app.world_mut()
                .spawn((
                    MenuSurface(menu),
                    Camera::default(),
                    ambition_menu_kaleidoscope::KaleidoscopePauseCamera,
                ))
                .id()
        });
        app.update();
        let left = app
            .world()
            .get::<Camera>(cameras[0])
            .unwrap()
            .viewport
            .as_ref()
            .unwrap();
        let right = app
            .world()
            .get::<Camera>(cameras[1])
            .unwrap()
            .viewport
            .as_ref()
            .unwrap();
        assert_eq!(
            left.physical_position.x + left.physical_size.x,
            right.physical_position.x
        );
    }
}
