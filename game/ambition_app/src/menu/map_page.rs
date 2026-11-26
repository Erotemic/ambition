//! The map page shared by the Cube and Grid menus.

use ambition_platformer2d::menu::map::browser::{
    MapAccess, MapAction, MapAtlas, MapBrowser, MapBrowserView, MapHierarchy,
};
use ambition_platformer2d::menu::map::view::{map_label, MapPinKind};
use ambition_platformer2d::menu::map::MapMenuState;
use ambition_platformer2d::menu::{
    MenuColor, MenuControlKind, MenuPageModel, MenuRect, MenuTextAlign,
};
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::model::{add_edge_buttons, line, menu_look, MenuPage, MenuPageAction};

/// The map's frame on the page, in percent of the page's width and height.
pub(crate) const MAP_FRAME: MenuRect = MenuRect::new(14.0, 26.0, 72.0, 54.0);

/// The frame's width over its height on screen: the page is 3:2 (the cube's
/// face, `MenuCubeGeometry::oot_like`), so a percent of width is 1.5 percent
/// of height.
pub(crate) const MAP_FRAME_ASPECT: f32 = MAP_FRAME.w * 1.5 / MAP_FRAME.h;

#[cfg(any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
pub(crate) fn frame_aspect(
    backend: ambition_platformer2d::menu::backend::InventoryUiBackend,
    region: Option<&super::regions::MenuRegion>,
) -> f32 {
    use ambition_platformer2d::menu::{
        backend::InventoryUiBackend, render::bevy_ui::menu_page_size,
    };

    if backend == InventoryUiBackend::Grid {
        if let Some(rect) = region.and_then(|region| region.0) {
            let size = menu_page_size(rect.size());
            return size.x * MAP_FRAME.w / (size.y * MAP_FRAME.h);
        }
    }
    MAP_FRAME_ASPECT
}

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

/// Build a menu page from the active map level.
pub type MapPageBuilder =
    Arc<dyn Fn(Option<&MapBrowserView>) -> MenuPageModel<MenuPage, MenuPageAction> + Send + Sync>;

/// Replace the shared map page presentation for a game.
#[derive(Resource, Clone)]
pub struct MapPagePresentation(pub MapPageBuilder);

impl MapPagePresentation {
    pub fn new(
        build: impl Fn(Option<&MapBrowserView>) -> MenuPageModel<MenuPage, MenuPageAction>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self(Arc::new(build))
    }
}

/// What the Map page is built from: the map's rooms and visits, the room the
/// player is in, and the game's presentation.
#[derive(SystemParam)]
pub(crate) struct MapPageSource<'w, 's> {
    map: Option<Res<'w, MapMenuState>>,
    hierarchy: Option<Res<'w, MapHierarchy>>,
    rollback: Option<Res<'w, ambition_platformer2d::rollback::RollbackSessionOwnership>>,
    network: Option<Res<'w, ambition_platformer2d::platformer::lifecycle::NetworkMenuPause>>,
    presentation: Option<Res<'w, MapPagePresentation>>,
    // Optional: a menu in an app with no rooms (a menu test, a shell with no
    // session) still draws its other pages; the Map page then shows no map.
    room: Option<ambition_platformer2d::world::rooms::LiveRoomSpecs<'w, 's>>,
}

impl MapPageSource<'_, '_> {
    /// Get the live room of the supplied body.
    #[cfg(feature = "bevy_ui_menu")]
    pub(crate) fn room(
        &self,
        body: Option<Entity>,
    ) -> Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance> {
        self.room.as_ref()?.live().of(body?)
    }

    fn current(&self, body: Option<Entity>) -> Option<&str> {
        Some(self.room.as_ref()?.spec_of(body?)?.id.as_str())
    }

    fn access<'a>(&'a self, map: &'a MapMenuState, body: Option<Entity>) -> MapAccess<'a> {
        let current_room = self.current(body);
        let developer = cfg!(feature = "dev_tools");
        MapAccess {
            current_room,
            visited: &map.visited,
            reveal_all: developer && map.reveal_all,
            can_travel: developer
                && current_room.is_some()
                && self.network.is_none()
                && !self
                    .rollback
                    .as_ref()
                    .is_some_and(|owner| owner.has_remote_peers()),
        }
    }

    pub(crate) fn view(
        &self,
        body: Option<Entity>,
        browser: Option<&MapBrowser>,
        aspect: f32,
    ) -> Option<MapBrowserView> {
        let map = self.map.as_ref()?;
        if map.rooms.is_empty() {
            return None;
        }
        let atlas = MapAtlas::new(&map.rooms, self.hierarchy.as_deref());
        Some(
            browser
                .unwrap_or(&MapBrowser::default())
                .view(&atlas, self.access(map, body), aspect),
        )
    }

    #[cfg(all(
        feature = "input",
        any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")
    ))]
    pub(crate) fn apply(
        &self,
        browser: &mut MapBrowser,
        body: Option<Entity>,
        action: MapAction,
        aspect: f32,
    ) -> ambition_platformer2d::menu::map::browser::MapOutcome<'_> {
        use ambition_platformer2d::menu::map::browser::MapOutcome;
        let Some(map) = self.map.as_ref().filter(|map| !map.rooms.is_empty()) else {
            return if action == MapAction::Up {
                MapOutcome::Close
            } else {
                MapOutcome::Stay
            };
        };
        let atlas = MapAtlas::new(&map.rooms, self.hierarchy.as_deref());
        browser.apply(&atlas, self.access(map, body), aspect, action)
    }

    #[cfg(all(
        feature = "input",
        any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")
    ))]
    pub(crate) fn adjust_viewport(
        &self,
        browser: &mut MapBrowser,
        body: Option<Entity>,
        pan: Vec2,
        zoom: f32,
        aspect: f32,
    ) {
        if let Some(map) = self.map.as_ref() {
            browser.adjust_viewport(
                &MapAtlas::new(&map.rooms, self.hierarchy.as_deref()),
                self.access(map, body),
                aspect,
                pan,
                zoom,
            );
        }
    }

    #[cfg(all(feature = "input", any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")))]
    pub(crate) fn pinch_viewport(
        &self,
        browser: &mut MapBrowser,
        body: Option<Entity>,
        aspect: f32,
        from: Vec2,
        to: Vec2,
        scale: f32,
    ) {
        if let Some(map) = self.map.as_ref() {
            browser.pinch_viewport(
                &MapAtlas::new(&map.rooms, self.hierarchy.as_deref()),
                self.access(map, body),
                aspect,
                from,
                to,
                scale,
            );
        }
    }

    /// The Map page for `view`, as the game draws it.
    pub(crate) fn page(
        &self,
        view: Option<&MapBrowserView>,
    ) -> MenuPageModel<MenuPage, MenuPageAction> {
        match &self.presentation {
            Some(presentation) => (presentation.0)(view),
            None => build_map_page(view),
        }
    }

    /// Whether the map's rooms or visits, or the game's presentation, changed
    /// this frame.
    #[cfg(feature = "bevy_ui_menu")]
    pub(crate) fn changed(&self) -> bool {
        self.map.as_ref().is_some_and(|map| map.is_changed())
            || self.presentation_changed()
            || self
                .hierarchy
                .as_ref()
                .is_some_and(|hierarchy| hierarchy.is_changed())
            || self
                .network
                .as_ref()
                .is_some_and(|network| network.is_changed())
            || self
                .rollback
                .as_ref()
                .is_some_and(|rollback| rollback.is_changed())
            || self
                .room
                .as_ref()
                .is_some_and(|rooms| rooms.set_is_changed())
    }

    /// Whether the game's presentation was inserted or replaced this frame:
    /// the one change a by-value [`MapBrowserView`] key cannot see.
    pub(crate) fn presentation_changed(&self) -> bool {
        self.presentation.as_ref().is_some_and(|p| p.is_changed())
    }
}

#[cfg(any(
    feature = "kaleidoscope_menu",
    all(feature = "input", feature = "bevy_ui_menu")
))]
pub(crate) fn navigate(
    menu: Entity,
    input: &ambition_platformer2d::input::MenuControlFrame,
    actions: &mut MessageWriter<ambition_platformer2d::menu::MenuActionActivated<MenuPageAction>>,
) {
    let dx = i8::from(input.right) - i8::from(input.left);
    let dy = i8::from(input.down) - i8::from(input.up);
    let action = if input.back {
        MapAction::Up
    } else if dx != 0 || dy != 0 {
        MapAction::Move(dx, dy)
    } else if input.select {
        MapAction::Confirm
    } else {
        return;
    };
    actions.write(ambition_platformer2d::menu::MenuActionActivated {
        menu,
        action: MenuPageAction::Map(action),
    });
}

#[cfg(all(
    feature = "input",
    any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")
))]
#[derive(SystemParam)]
pub(crate) struct MapTravel<'w, 's> {
    bodies: ambition_platformer2d::platformer::lifecycle::LiveBodies<'w, 's>,
    in_play: Query<
        'w,
        's,
        (),
        (
            With<ambition_platformer2d::engine_core::BodyKinematics>,
            Without<ambition_platformer2d::combat::death_rules::OutOfPlay>,
            Without<ambition_platformer2d::session::rooms::passage::ZonePassage>,
        ),
    >,
    pending:
        Option<Res<'w, ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>>,
    requests: Option<
        ambition_platformer2d::platformer::host_intents::HostIntentWriter<
            'w,
            's,
            ambition_platformer2d::session::rooms::RoomTravelRequested,
        >,
    >,
}

#[cfg(all(
    feature = "input",
    any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")
))]
impl MapTravel<'_, '_> {
    pub(crate) fn request(
        &mut self,
        source: &MapPageSource,
        body: Option<super::effects::MenuBodyFacts>,
        target: &str,
    ) -> bool {
        use ambition_platformer2d::session::rooms::RoomTravelRequested;

        let Some(body) = body.filter(|body| self.in_play.contains(body.entity)) else {
            return false;
        };
        let Some(rooms) = source
            .room
            .as_ref()
            .filter(|rooms| rooms.spec_of(body.entity).is_some())
        else {
            return false;
        };
        let Some(subject) = self
            .bodies
            .id_of(body.entity)
            .filter(|id| id.room.is_some())
        else {
            return false;
        };
        if rooms.rooms().room_index_by_id(target).is_none()
            || self
                .pending
                .as_ref()
                .is_some_and(|pending| pending.peek().is_some())
        {
            return false;
        }
        let Some(requests) = self.requests.as_mut() else {
            return false;
        };
        requests.write(RoomTravelRequested {
            body: subject,
            destination: target.to_owned(),
        });
        true
    }
}

fn colour([r, g, b, a]: [f32; 4]) -> MenuColor {
    MenuColor::rgba(r, g, b, a)
}

/// Build the default map page for both menu renderers.
pub fn build_map_page(view: Option<&MapBrowserView>) -> MenuPageModel<MenuPage, MenuPageAction> {
    let look = menu_look();
    let map_look = look.map;
    let mut model = MenuPageModel::new(MenuPage::Map, line("menu.map.title"), colour(look.page));
    add_edge_buttons(&mut model, MenuPage::Map);
    let Some(view) = view else {
        model.text(
            50.0,
            8.0,
            4.0,
            line("menu.map.title"),
            MenuTextAlign::Center,
            colour(look.heading),
        );
        model.text(
            50.0,
            50.0,
            2.4,
            line("menu.map.unmapped"),
            MenuTextAlign::Center,
            colour(look.placeholder_body),
        );
        return model;
    };
    let title = match &view.parent_title {
        Some(parent) => format!("{parent} / {}", view.title),
        None => view.title.clone(),
    };
    model.text(
        50.0,
        5.0,
        3.3,
        title,
        MenuTextAlign::Center,
        colour(look.heading),
    );

    let f = MAP_FRAME;
    model.layered_panel(
        f,
        colour(map_look.frame),
        Some(MenuPageAction::Map(MapAction::Canvas)),
        0.0,
    );
    let mut entries: Vec<_> = view.entries.iter().collect();
    entries.sort_by_key(|entry| (entry.selected, entry.here));
    let mut tiles = Vec::new();
    for entry in entries {
        let label = entry
            .pieces
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
            .map(|(index, _)| index);
        tiles.extend(
            entry
                .pieces
                .iter()
                .enumerate()
                .map(|(index, &piece)| (entry, piece, label == Some(index))),
        );
    }
    let count = tiles.len().max(1) as f32;
    for (index, &(entry, rect, label)) in tiles.iter().enumerate() {
        let rect = MenuRect::new(
            f.x + rect.min.x * f.w,
            f.y + rect.min.y * f.h,
            rect.width() * f.w,
            rect.height() * f.h,
        );
        let fill = if entry.here {
            map_look.room_here
        } else if !entry.visited {
            map_look.room_unvisited
        } else {
            map_look.room
        };
        let layer = 0.08 + 0.56 * index as f32 / count;
        let action = (!view.room_detail).then_some(MenuPageAction::Map(MapAction::Open(entry.id)));
        let inset = MenuRect::new(
            rect.x + (rect.w * 0.08).min(0.22),
            rect.y + (rect.h * 0.08).min(0.32),
            rect.w - (rect.w * 0.16).min(0.44),
            rect.h - (rect.h * 0.16).min(0.64),
        );
        if entry.selected && !view.room_detail {
            model.layered_panel(rect, colour(look.heading), action, layer);
            model.layered_panel(inset, colour(fill), action, layer + 0.28 / count);
        } else {
            model.layered_panel(inset, colour(fill), action, layer);
        }
        if label
            && entry.visited
            && inset.w >= MAP_LABEL_MIN_WIDTH
            && inset.h >= MAP_LABEL_SIZE * 1.6
        {
            let label = short_title(&entry.title, inset.w);
            let anchor = Vec2::new(inset.x + inset.w * 0.5, inset.y + inset.h * 0.5);
            let half_width = label.chars().count() as f32 * MAP_LABEL_SIZE * 0.72 / 3.0;
            let label_rect = Rect::from_center_size(
                Vec2::new((anchor.x - f.x) / f.w, (anchor.y - f.y) / f.h),
                Vec2::new(half_width * 2.0 / f.w, MAP_LABEL_SIZE / f.h),
            );
            // Hide a label where a room in front of it covers the text.
            if !tiles[index + 1..]
                .iter()
                .any(|(_, rect, _)| !rect.intersect(label_rect).is_empty())
            {
                model.text(
                    anchor.x,
                    anchor.y - MAP_LABEL_SIZE * 0.5,
                    MAP_LABEL_SIZE,
                    label,
                    MenuTextAlign::Center,
                    colour(if entry.here {
                        map_look.label_here
                    } else {
                        map_look.label
                    }),
                );
            }
        }
    }
    let marks = view
        .pins
        .iter()
        .map(|pin| {
            let (size, fill) = match pin.kind {
                MapPinKind::Resident => (0.55, map_look.pin_resident),
                MapPinKind::Shrine => (0.9, map_look.pin_shrine),
            };
            (pin.at, size, fill)
        })
        .chain(view.doors.iter().map(|at| (*at, 0.7, map_look.door)));
    let count = (view.pins.len() + view.doors.len()).max(1) as f32;
    for (i, (at, w, fill)) in marks.enumerate() {
        let centre = Vec2::new(f.x + at.x * f.w, f.y + at.y * f.h);
        let mark = Rect::from_center_size(centre, Vec2::new(w, w * 1.5)).intersect(
            Rect::from_corners(Vec2::new(f.x, f.y), Vec2::new(f.x + f.w, f.y + f.h)),
        );
        if !mark.is_empty() {
            model.layered_panel(
                MenuRect::new(mark.min.x, mark.min.y, mark.width(), mark.height()),
                colour(fill),
                None,
                0.7 + 0.3 * i as f32 / count,
            );
        }
    }

    let mut x = f.x;
    for &tool in &view.tools {
        let (label, w) = match tool {
            MapAction::Up => (line("menu.map.back"), 9.0),
            MapAction::Zoom(-1) => ("−".into(), 5.0),
            MapAction::Fit => (line("menu.map.fit"), 5.0),
            MapAction::Zoom(1) => ("+".into(), 5.0),
            MapAction::Pan(-1, 0) => ("←".into(), 5.0),
            MapAction::Pan(0, -1) => ("↑".into(), 5.0),
            MapAction::Pan(0, 1) => ("↓".into(), 5.0),
            MapAction::Pan(1, 0) => ("→".into(), 5.0),
            MapAction::Browse(-1) => (line("menu.map.previous"), 8.0),
            MapAction::Browse(1) => (line("menu.map.next"), 8.0),
            MapAction::Travel => (line("menu.map.travel"), 16.0),
            _ => continue,
        };
        model.control(
            MenuRect::new(x, 14.0, w, 8.0),
            MenuControlKind::Action,
            label,
            None,
            view.focused_tool == Some(tool),
            tool == MapAction::Travel,
            Some(MenuPageAction::Map(tool)),
        );
        x += w + 1.0;
    }

    if let Some(entry) = view.entries.iter().find(|entry| entry.selected) {
        let label = if entry.here {
            format!("{} · {}", entry.title, line("menu.map.here"))
        } else {
            entry.title.clone()
        };
        model.text(
            50.0,
            84.0,
            2.6,
            label,
            MenuTextAlign::Center,
            colour(map_look.label),
        );
    } else {
        model.text(
            50.0,
            50.0,
            2.4,
            line("menu.map.empty"),
            MenuTextAlign::Center,
            colour(map_look.label),
        );
    }
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
        app.world_mut()
            .run_system_once(|source: MapPageSource| source.page(None).title)
            .expect("the system runs")
    }

    #[test]
    fn a_game_draws_its_own_map_page_and_the_host_draws_a_plain_one() {
        let mut app = App::new();
        let placeholder = page_title(&mut app);
        app.insert_resource(MapPagePresentation::new(|_| {
            MenuPageModel::new(
                MenuPage::Map,
                "THE GAME'S MAP",
                MenuColor::rgba(0.0, 0.0, 0.0, 1.0),
            )
        }));
        assert_eq!(page_title(&mut app), "THE GAME'S MAP");
        assert_ne!(placeholder, "THE GAME'S MAP");
    }

    #[test]
    fn a_room_name_drops_its_article_and_fits_its_room() {
        assert_eq!(short_title("THE QUAD", 20.0), "QUAD");
        assert_eq!(
            short_title(
                "THE GATE DECK: EVERYTHING THE RING BRINGS STOPS AT CUSTOMS",
                20.0
            ),
            "GATE DECK"
        );
        assert_eq!(
            short_title("Castle Godwin: the gatehouse.", 20.0),
            "Gatehouse"
        );
        assert_eq!(
            short_title("THE PIRATE PORT: THE GATE PLAZA", 20.0),
            "GATE PLAZA"
        );
        let cut = short_title("THE WRECKERS' REEF AND ITS LONG SHOAL", 9.0);
        assert!(cut.ends_with('…'), "{cut}");
        assert!(cut.chars().count() <= 12, "{cut}");
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::input::ParticipantId;
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    use bevy::ecs::system::RunSystemOnce;

    fn titles(app: &mut App) -> [Option<String>; 2] {
        app.world_mut()
            .run_system_once(
                |source: MapPageSource, bodies: crate::menu::effects::MenuBodySource| {
                    [ParticipantId(0), ParticipantId(1)].map(|owner| {
                        let body = Some(bodies.read(owner)?.entity);
                        source
                            .view(body, None, MAP_FRAME_ASPECT)?
                            .entries
                            .into_iter()
                            .find(|entry| entry.here)
                            .map(|entry| entry.title)
                    })
                },
            )
            .unwrap()
    }

    #[test]
    fn each_menu_map_follows_its_controlled_body_between_live_rooms() {
        let mut app = App::new();
        let rooms = crate::menu::test_support::two_room_map(&mut app);
        app.world_mut()
            .spawn((DrivingParticipant(PlayerSlot(0)), InRoomInstance(rooms[0])));
        let second = app
            .world_mut()
            .spawn((DrivingParticipant(PlayerSlot(1)), InRoomInstance(rooms[1])))
            .id();
        assert_eq!(
            titles(&mut app),
            [Some("ALPHA".into()), Some("BETA".into())]
        );
        app.world_mut()
            .entity_mut(second)
            .insert(InRoomInstance(rooms[0]));
        assert_eq!(
            titles(&mut app),
            [Some("ALPHA".into()), Some("ALPHA".into())]
        );
        app.world_mut()
            .entity_mut(second)
            .remove::<DrivingParticipant>();
        let next = app
            .world_mut()
            .spawn((DrivingParticipant(PlayerSlot(1)), InRoomInstance(rooms[1])))
            .id();
        assert_eq!(
            titles(&mut app),
            [Some("ALPHA".into()), Some("BETA".into())]
        );
        let duplicate = app
            .world_mut()
            .spawn((DrivingParticipant(PlayerSlot(1)), InRoomInstance(rooms[0])))
            .id();
        assert_eq!(titles(&mut app), [Some("ALPHA".into()), None]);
        app.world_mut().despawn(next);
        app.world_mut().despawn(duplicate);
        assert_eq!(titles(&mut app), [Some("ALPHA".into()), None]);
    }
}
