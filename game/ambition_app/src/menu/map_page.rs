//! The Map page of the pause menu (the cube's face and the grid's tab).
//!
//! The map is the LDtk layout: the region of rooms joined by edge exits that
//! the player is in (`ambition_menu::map::view`), each room its world
//! rectangle, visited rooms drawn, the ways on from them dim, the doors
//! marked, the residents and shrines of the visited rooms pinned. Inside a
//! building, the region outside it, its door marked. The page is panels and
//! text, so both backends draw it as they draw any page.
//!
//! The map is the engine's; how it looks is the game's. [`build_map_page`]
//! is the host's plain placeholder, and a game draws its own map by
//! inserting a [`MapPagePresentation`] built on the same [`MapView`].

use ambition_platformer2d::menu::{MenuColor, MenuPageModel, MenuRect, MenuTextAlign};
use ambition_platformer2d::menu::map::view::{map_label, map_view, MapPinKind, MapView};
use ambition_platformer2d::menu::map::MapMenuState;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::model::{add_edge_buttons, line, menu_look, MenuPage, MenuPageAction};

/// The map's frame on the page, in percent of the page's width and height.
pub(crate) const MAP_FRAME: MenuRect = MenuRect::new(14.0, 20.0, 72.0, 62.0);

/// The frame's width over its height on screen: the page is 3:2 (the cube's
/// face, `MenuCubeGeometry::oot_like`), so a percent of width is 1.5 percent
/// of height.
pub(crate) const MAP_FRAME_ASPECT: f32 = MAP_FRAME.w * 1.5 / MAP_FRAME.h;

/// A room's name on the map, in percent of the page's height.
const MAP_LABEL_SIZE: f32 = 2.0;

/// The narrowest room, in percent of the page's width, that shows its name.
const MAP_LABEL_MIN_WIDTH: f32 = 9.0;

/// A room's title fitted to `width` percent of the page: its name
/// (`map_label`), cut with an ellipsis when it is still too long. A character
/// of the label (capitals, the widest) is at most about 0.72 of its height
/// wide, and a percent of width is 1.5 percent of height on the 3:2 page.
fn short_title(title: &str, width: f32) -> String {
    let title = map_label(title);
    let title = title.as_str();
    let fits = (width * 1.5 / (MAP_LABEL_SIZE * 0.72)).floor().max(3.0) as usize;
    if title.chars().count() <= fits {
        return title.to_string();
    }
    let cut: String = title.chars().take(fits.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// Builds the Map page from the map of the player's region (`None` off the
/// map).
pub type MapPageBuilder = Arc<dyn Fn(Option<&MapView>) -> MenuPageModel<MenuPage, MenuPageAction> + Send + Sync>;

/// How a game draws its Map page. Without one, the host's placeholder
/// ([`build_map_page`]).
#[derive(Resource, Clone)]
pub struct MapPagePresentation(pub MapPageBuilder);

impl MapPagePresentation {
    pub fn new(build: impl Fn(Option<&MapView>) -> MenuPageModel<MenuPage, MenuPageAction> + Send + Sync + 'static) -> Self {
        Self(Arc::new(build))
    }
}

/// What the Map page is built from: the map's rooms and visits, the room the
/// player is in, and the game's presentation.
#[derive(SystemParam)]
pub(crate) struct MapPageSource<'w, 's> {
    map: Option<Res<'w, MapMenuState>>,
    presentation: Option<Res<'w, MapPagePresentation>>,
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

    /// The Map page for `view`, as the game draws it.
    pub(crate) fn page(&self, view: Option<&MapView>) -> MenuPageModel<MenuPage, MenuPageAction> {
        match &self.presentation {
            Some(presentation) => (presentation.0)(view),
            None => build_map_page(view),
        }
    }

    /// Whether the map's rooms or visits, or the game's presentation, changed
    /// this frame.
    pub(crate) fn changed(&self) -> bool {
        self.map.as_ref().is_some_and(|map| map.is_changed()) || self.presentation_changed()
    }

    /// Whether the game's presentation was inserted or replaced this frame:
    /// the one change a by-value [`MapView`] key cannot see.
    pub(crate) fn presentation_changed(&self) -> bool {
        self.presentation.as_ref().is_some_and(|p| p.is_changed())
    }
}

fn colour([r, g, b, a]: [f32; 4]) -> MenuColor {
    MenuColor::rgba(r, g, b, a)
}

/// The host's placeholder Map page: the region's rooms in the frame, the
/// title of the room the player is in under it.
pub fn build_map_page(view: Option<&MapView>) -> MenuPageModel<MenuPage, MenuPageAction> {
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
    // The names of the visited rooms big enough to hold one, under the
    // doors' marks so neither hides the other.
    for room in view.rooms.iter().filter(|room| room.visited) {
        let w = room.size.x * f.w;
        if w < MAP_LABEL_MIN_WIDTH || room.size.y * f.h < MAP_LABEL_SIZE * 1.6 {
            continue;
        }
        let cx = f.x + (room.min.x + room.size.x * 0.5) * f.w;
        let cy = f.y + (room.min.y + room.size.y * 0.5) * f.h - MAP_LABEL_SIZE * 0.5;
        let colour_key = if room.here { map_look.label_here } else { map_look.label };
        model.text(cx, cy, MAP_LABEL_SIZE, short_title(&room.title, w), MenuTextAlign::Center, colour(colour_key));
    }
    // The pins, under the doors' marks.
    for pin in &view.pins {
        let (w, fill) = match pin.kind {
            MapPinKind::Resident => (0.55, map_look.pin_resident),
            MapPinKind::Shrine => (0.9, map_look.pin_shrine),
        };
        let h = w * 1.5;
        model.panel(MenuRect::new(f.x + pin.at.x * f.w - w * 0.5, f.y + pin.at.y * f.h - h * 0.5, w, h), colour(fill), None);
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
    // The sides the region goes on past.
    let arrows = [
        (view.beyond.left, f.x - 2.6, f.y + f.h * 0.5, "<"),
        (view.beyond.right, f.x + f.w + 2.6, f.y + f.h * 0.5, ">"),
        (view.beyond.up, 50.0, f.y - 4.5, "^"),
        (view.beyond.down, 50.0, f.y + f.h + 1.6, "v"),
    ];
    for (_, x, y, arrow) in arrows.into_iter().filter(|(on, ..)| *on) {
        model.text(x, y - 1.5, 3.0, arrow.to_string(), MenuTextAlign::Center, colour(map_look.label));
    }
    let here = match &view.inside {
        Some(inside) => format!("{} ({})", inside, line("menu.map.inside")),
        None => view.here_title.clone(),
    };
    model.text(50.0, 88.0, 2.2, here, MenuTextAlign::Center, colour(map_look.label));
    add_edge_buttons(&mut model, MenuPage::Map);
    model
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    use super::{short_title, MapPagePresentation, MapPageSource};
    use crate::menu::model::MenuPage;
    use ambition_platformer2d::menu::{MenuColor, MenuPageModel};

    /// The page the Map tab shows: the host's placeholder, until the game
    /// inserts its own presentation.
    fn page_title(app: &mut App) -> String {
        app.world_mut().run_system_once(|source: MapPageSource| source.page(None).title).expect("the system runs")
    }

    #[test]
    fn a_game_draws_its_own_map_page_and_the_host_draws_a_plain_one() {
        let mut app = App::new();
        let placeholder = page_title(&mut app);
        app.insert_resource(MapPagePresentation::new(|_| {
            MenuPageModel::new(MenuPage::Map, "THE GAME'S MAP", MenuColor::rgba(0.0, 0.0, 0.0, 1.0))
        }));
        assert_eq!(page_title(&mut app), "THE GAME'S MAP");
        assert_ne!(placeholder, "THE GAME'S MAP");
    }

    #[test]
    fn a_room_name_drops_its_article_and_fits_its_room() {
        assert_eq!(short_title("THE QUAD", 20.0), "QUAD");
        assert_eq!(short_title("THE GATE DECK: EVERYTHING THE RING BRINGS STOPS AT CUSTOMS", 20.0), "GATE DECK");
        assert_eq!(short_title("Castle Godwin: the gatehouse.", 20.0), "Gatehouse");
        assert_eq!(short_title("THE PIRATE PORT: THE GATE PLAZA", 20.0), "GATE PLAZA");
        let cut = short_title("THE WRECKERS' REEF AND ITS LONG SHOAL", 9.0);
        assert!(cut.ends_with('…'), "{cut}");
        assert!(cut.chars().count() <= 12, "{cut}");
    }
}
