//! The map page shared by the Cube and Grid menus.

use ambition_platformer2d::menu::map::browser::{
    MapAccess, MapAction, MapAtlas, MapBrowser, MapBrowserView, MapHierarchy,
};
use ambition_platformer2d::menu::map::MapMenuState;
use ambition_platformer2d::menu::MenuPageModel;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::model::{MenuPage, MenuPageAction};

mod presentation;
pub use presentation::build_map_page;
pub(crate) use presentation::MAP_FRAME;

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

    #[cfg(all(
        feature = "input",
        any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")
    ))]
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    use super::{presentation::short_title, MapPagePresentation, MapPageSource};
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
