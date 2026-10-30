//! The Map page of the pause menu (the cube's face and the grid's tab).
//!
//! The map is the LDtk layout: the region of rooms joined by edge exits that
//! the player is in (`ambition_menu::map::view`), each room its world
//! rectangle, visited rooms drawn, the ways on from them dim, the doors
//! marked. Inside a building, the region outside it, its door marked. The
//! page is panels and text, so both backends draw it as they draw any page.

use ambition_platformer2d::menu::{MenuColor, MenuPageModel, MenuRect, MenuTextAlign};
use ambition_platformer2d::menu::map::view::{map_view, MapView};
use ambition_platformer2d::menu::map::MapMenuState;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::model::{add_edge_buttons, line, menu_look, MenuPage, MenuPageAction};

/// The map's frame on the page, in percent of the page's width and height.
pub(crate) const MAP_FRAME: MenuRect = MenuRect::new(14.0, 20.0, 72.0, 62.0);

/// The frame's width over its height on screen: the page is 3:2 (the cube's
/// face, `MenuCubeGeometry::oot_like`), so a percent of width is 1.5 percent
/// of height.
pub(crate) const MAP_FRAME_ASPECT: f32 = MAP_FRAME.w * 1.5 / MAP_FRAME.h;

/// What the Map page is built from: the map's rooms and visits, and the room
/// the player is in.
#[derive(SystemParam)]
pub(crate) struct MapPageSource<'w, 's> {
    map: Option<Res<'w, MapMenuState>>,
    // Optional: a menu in an app with no rooms (a menu test, a shell with no
    // session) still draws its other pages; the Map page then shows no map.
    room: Option<ambition_platformer2d::world::rooms::PrimaryLiveRoomSpec<'w, 's>>,
}

impl MapPageSource<'_, '_> {
    /// The map of the player's region, or `None` off the map.
    pub(crate) fn view(&self) -> Option<MapView> {
        let map = self.map.as_ref()?;
        let current = self.room.as_ref()?.spec()?.id.clone();
        map_view(map, &current, MAP_FRAME_ASPECT)
    }

    /// Whether the map's rooms or visits changed this frame.
    pub(crate) fn changed(&self) -> bool {
        self.map.as_ref().is_some_and(|map| map.is_changed())
    }
}

fn colour([r, g, b, a]: [f32; 4]) -> MenuColor {
    MenuColor::rgba(r, g, b, a)
}

/// The Map page: the region's rooms in the frame, the title of the room the
/// player is in under it.
pub(crate) fn build_map_page(view: Option<&MapView>) -> MenuPageModel<MenuPage, MenuPageAction> {
    let look = menu_look();
    let map_look = look.map;
    let mut model = MenuPageModel::new(MenuPage::Map, line("menu.map.title"), colour(look.page));
    model.text(50.0, 8.0, 4.0, line("menu.map.title"), MenuTextAlign::Center, colour(look.heading));
    let Some(view) = view else {
        model.text(50.0, 50.0, 1.7, line("menu.map.unmapped"), MenuTextAlign::Center, colour(look.placeholder_body));
        add_edge_buttons(&mut model, MenuPage::Map);
        return model;
    };
    let f = MAP_FRAME;
    model.panel(MenuRect::new(f.x - 1.0, f.y - 1.5, f.w + 2.0, f.h + 3.0), colour(map_look.frame), None);
    // Each room inset a little, so rooms that touch show the join between
    // them; the room the player is in drawn last, over its neighbours.
    let gap_x = 0.18;
    let gap_y = gap_x * 1.5;
    let mut rooms: Vec<_> = view.rooms.iter().collect();
    rooms.sort_by_key(|room| room.here);
    for room in rooms {
        let rect = MenuRect::new(
            f.x + room.min.x * f.w + gap_x,
            f.y + room.min.y * f.h + gap_y,
            (room.size.x * f.w - 2.0 * gap_x).max(0.4),
            (room.size.y * f.h - 2.0 * gap_y).max(0.6),
        );
        let fill = if room.here {
            map_look.room_here
        } else if room.visited {
            map_look.room
        } else {
            map_look.room_unvisited
        };
        model.panel(rect, colour(fill), None);
    }
    for (i, at) in view.doors.iter().enumerate() {
        let marked = view.marked_door == Some(i);
        let (w, h) = if marked { (1.4, 2.1) } else { (0.7, 1.05) };
        model.panel(
            MenuRect::new(f.x + at.x * f.w - w * 0.5, f.y + at.y * f.h - h * 0.5, w, h),
            colour(if marked { map_look.door_marked } else { map_look.door }),
            None,
        );
    }
    let here = match &view.inside {
        Some(inside) => format!("{} ({})", inside, line("menu.map.inside")),
        None => view.here_title.clone(),
    };
    model.text(50.0, 88.0, 2.2, here, MenuTextAlign::Center, colour(map_look.label));
    add_edge_buttons(&mut model, MenuPage::Map);
    model
}
