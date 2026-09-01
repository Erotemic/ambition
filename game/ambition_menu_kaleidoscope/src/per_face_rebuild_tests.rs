//! The rebuild narrows to the faces whose content moved.
//!
//! These assert on ENTITY IDENTITY, not on a frame time: a face that was left
//! alone keeps its `Entity`, a face that was rebuilt gets a new one. That is a
//! count, and it survives a slow machine, a busy machine, and a GPU nobody has.

use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use ambition_menu::{ActiveMenuPages, AmbitionMenuPage, MenuColor, MenuPageModel, MenuRect};

use super::{rebuild_cube_faces, KaleidoscopeMenuConfig, MenuRing};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Items,
    Map,
    Quest,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Open(Page),
}

const PAGES: [Page; 4] = [Page::Items, Page::Map, Page::Quest, Page::System];

/// A page with one panel whose colour carries `tag`, so a content change is a
/// one-field edit rather than a structural one.
fn page(id: Page, tag: f32) -> MenuPageModel<Page, Action> {
    let mut model = MenuPageModel::new(id, "page", MenuColor::rgba(0.1, 0.1, 0.1, 1.0));
    model.panel(
        MenuRect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
        MenuColor::rgba(tag, 0.2, 0.3, 1.0),
        Some(Action::Open(id)),
    );
    model
}

fn all_pages(tag: f32) -> Vec<MenuPageModel<Page, Action>> {
    PAGES.iter().map(|id| page(*id, tag)).collect()
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Mesh>();
    app.init_asset::<Image>();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.insert_resource(ActiveMenuPages::<Page, Action> {
        pages: all_pages(0.5),
        active: Some(Page::Items),
        visible: true,
        version: 1,
    });
    app.world_mut().spawn(MenuRing);
    app.add_systems(Update, rebuild_cube_faces::<Page, Action>);
    app.update();
    app
}

/// The live face entity for each page, in `PAGES` order.
fn faces(app: &mut App) -> Vec<(Page, Entity)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &AmbitionMenuPage<Page>)>();
    let mut found: Vec<(Page, Entity)> = query
        .iter(world)
        .map(|(entity, page)| (page.id, entity))
        .collect();
    found.sort_by_key(|(id, _)| PAGES.iter().position(|p| p == id).unwrap());
    found
}

/// Publishing pages that compare EQUAL must not respawn anything, even though the
/// version bump says "something changed". Without the per-face comparison the
/// version alone despawns and rebuilds all four faces.
#[test]
fn an_equal_republish_rebuilds_no_face() {
    let mut app = test_app();
    let before = faces(&mut app);
    assert_eq!(before.len(), 4, "four pages published, four faces expected");

    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(all_pages(0.5), Page::Items);
    app.update();

    assert_eq!(
        faces(&mut app),
        before,
        "an equal republish must leave every face standing",
    );
}

/// A change to ONE page's content rebuilds exactly that page's face. This is the
/// scroll / drill / pick-up-an-item path: the other three faces are untouched.
#[test]
fn a_one_page_content_change_rebuilds_only_that_face() {
    let mut app = test_app();
    let before = faces(&mut app);

    let mut published = all_pages(0.5);
    published[2] = page(Page::Quest, 0.9);
    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(published, Page::Items);
    app.update();

    let after = faces(&mut app);
    assert_eq!(after.len(), 4);
    for (i, ((page_before, before), (page_after, after))) in
        before.iter().zip(after.iter()).enumerate()
    {
        assert_eq!(page_before, page_after, "page order must be preserved");
        if i == 2 {
            assert_ne!(
                before, after,
                "the page whose content changed must be rebuilt",
            );
        } else {
            assert_eq!(
                before, after,
                "{page_after:?} did not change and must not be rebuilt",
            );
        }
    }
}

/// A page turn rebuilds TWO faces — the one that stopped being active and the one
/// that started — because `active` is baked into each face's depth bands and
/// control markers. The two faces that were not involved stay.
#[test]
fn a_page_turn_rebuilds_only_the_two_faces_whose_active_flag_moved() {
    let mut app = test_app();
    let before = faces(&mut app);

    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(all_pages(0.5), Page::Map);
    app.update();

    let after = faces(&mut app);
    let changed: Vec<Page> = before
        .iter()
        .zip(after.iter())
        .filter(|((_, before), (_, after))| before != after)
        .map(|((id, _), _)| *id)
        .collect();
    assert_eq!(
        changed,
        vec![Page::Items, Page::Map],
        "only the outgoing and incoming active faces carry a changed `active` flag",
    );
}

/// The config decides every face's geometry and styling at spawn time, so a change
/// to it invalidates all of them at once — the one case that is still wholesale.
#[test]
fn a_config_change_rebuilds_every_face() {
    let mut app = test_app();
    let before = faces(&mut app);

    let mut config = app.world_mut().resource_mut::<KaleidoscopeMenuConfig>();
    config.inside_x_flip = -config.inside_x_flip;
    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(all_pages(0.5), Page::Items);
    app.update();

    let after = faces(&mut app);
    for ((id, before), (_, after)) in before.iter().zip(after.iter()) {
        assert_ne!(before, after, "{id:?} must be rebuilt for the new config");
    }
}

/// Dropping a page retires its face; adding one spawns a face for it. The ring
/// must never keep a face for a page nobody publishes any more.
#[test]
fn a_page_leaving_the_publication_retires_its_face() {
    let mut app = test_app();

    let published: Vec<MenuPageModel<Page, Action>> = all_pages(0.5)
        .into_iter()
        .filter(|model| model.id != Page::Quest)
        .collect();
    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(published, Page::Items);
    app.update();

    let live: Vec<Page> = faces(&mut app).into_iter().map(|(id, _)| id).collect();
    assert_eq!(live, vec![Page::Items, Page::Map, Page::System]);
}
