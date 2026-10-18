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

/// Every entity under the ring's faces, sorted: a census that changes when any
/// plane is despawned or spawned.
fn descendants(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut faces = world.query_filtered::<Entity, With<AmbitionMenuPage<Page>>>();
    let faces: Vec<Entity> = faces.iter(world).collect();
    let mut children = world.query::<&Children>();
    let mut found = Vec::new();
    for face in faces {
        let mut stack = vec![face];
        while let Some(entity) = stack.pop() {
            if let Ok(kids) = children.get(world, entity) {
                stack.extend(kids.iter());
                found.extend(kids.iter());
            }
        }
    }
    found.sort();
    found
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

/// A change to one page's content keeps every face standing: the changed node
/// is rewritten on the entity already drawing it. This is the scroll / drill /
/// pick-up-an-item path, and respawning here is what blinked.
#[test]
fn a_content_change_keeps_every_face_and_its_nodes() {
    let mut app = test_app();
    let before = faces(&mut app);
    let planes_before = descendants(&mut app);

    let mut published = all_pages(0.5);
    published[2] = page(Page::Quest, 0.9);
    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(published, Page::Items);
    app.update();

    assert_eq!(
        faces(&mut app),
        before,
        "a content change must not respawn a face"
    );
    assert_eq!(
        descendants(&mut app),
        planes_before,
        "a recoloured panel is the same panel: no entity may be despawned or spawned",
    );
}

/// A page turn respawns nothing. `active` is baked into depth bands and control
/// markers, and both are re-derived in place on the faces whose flag moved.
#[test]
fn a_page_turn_moves_the_active_face_in_place() {
    let mut app = test_app();
    let before = faces(&mut app);
    let planes_before = descendants(&mut app);

    let mut pages = app
        .world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>();
    pages.replace_pages(all_pages(0.5), Page::Map);
    app.update();

    assert_eq!(
        faces(&mut app),
        before,
        "a page turn must not respawn a face"
    );
    assert_eq!(descendants(&mut app), planes_before);
    let world = app.world_mut();
    let mut marked = world.query_filtered::<&ChildOf, With<super::KaleidoscopeActiveFaceControl>>();
    let parents: Vec<Entity> = marked.iter(world).map(ChildOf::parent).collect();
    let map_face = before.iter().find(|(id, _)| *id == Page::Map).unwrap().1;
    assert!(
        !parents.is_empty(),
        "the active face's control must carry the marker"
    );
    assert!(
        parents.iter().all(|parent| *parent == map_face),
        "only the new active face's controls are highlight-eligible",
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

/// A solid plane is born in the alpha mode the fade sweep would give it, so a
/// rebuild never flips a material's pipeline key one schedule later. The flip
/// was a frame in which the plane sat in no render phase under Bevy 0.19.
///
/// This was once credited with ending the System-face flash on scroll. It did
/// not: a capture on 2026-09-27 still showed a newly SPAWNED solid plane (born
/// Opaque) drawing nothing on its first frame. The flash ended when a scroll
/// stopped spawning planes (`reconcile_tests`).
#[test]
fn a_freshly_rebuilt_solid_plane_is_already_opaque() {
    let mut app = test_app();
    let world = app.world_mut();
    let mut query = world.query::<(&super::KaleidoscopeFade, &MeshMaterial3d<StandardMaterial>)>();
    let handles: Vec<Handle<StandardMaterial>> = query
        .iter(world)
        .map(|(_, material)| material.0.clone())
        .collect();
    assert!(
        !handles.is_empty(),
        "the fixture's pages spawn at least one faded plane"
    );
    let materials = world.resource::<Assets<StandardMaterial>>();
    let mut solid = 0;
    for handle in &handles {
        let material = materials
            .get(handle)
            .expect("a spawned plane's material exists");
        if material.base_color_texture.is_none() {
            solid += 1;
            assert_eq!(
                material.alpha_mode,
                AlphaMode::Opaque,
                "a solid plane must be spawned Opaque — the mode the fade sweep would set — \
                 not Blend-then-corrected"
            );
        }
    }
    assert!(
        solid > 0,
        "the fixture's panels are solid planes; none were found"
    );
}

/// The cube draws the colours its config states (`KaleidoscopeMenuConfig::look`),
/// not the renderer's own: an item cell, a control that cannot be used, a
/// scroll bar's track and thumb, and the face's frame. And a scroll bar keeps
/// its track colour after the cursor changes its state.
#[test]
fn the_cube_draws_the_look_its_config_states() {
    use ambition_menu::MenuControlKind;
    use super::{KaleidoscopeControlStyle, KaleidoscopeLook};

    // Each colour differs from the renderer's own and from each other.
    let tone = |n: f32| Color::srgba(0.05 * n, 0.5, 1.0 - 0.05 * n, 0.9);
    let look = KaleidoscopeLook {
        selected: tone(1.0),
        important: tone(2.0),
        item: tone(3.0),
        action: tone(4.0),
        control: tone(5.0),
        disabled: tone(6.0),
        scrollbar_track: tone(7.0),
        scrollbar_thumb: tone(8.0),
        edge_frame: tone(9.0),
        icon_disabled: tone(10.0),
        icon_selected: tone(11.0),
    };
    let mut page = MenuPageModel::new(Page::Items, "page", MenuColor::rgba(0.1, 0.1, 0.1, 1.0));
    let rect = |y: f32| MenuRect { x: 10.0, y, w: 20.0, h: 8.0 };
    page.control(rect(10.0), MenuControlKind::Item, "cell", None, false, false, Some(Action::Open(Page::Map)));
    page.control(rect(20.0), MenuControlKind::Action, "none", None, false, false, None);
    page.scrollbar(MenuRect { x: 80.0, y: 10.0, w: 2.0, h: 60.0 }, 0.0, 0.4);
    let open = Some(Action::Open(Page::Quest));
    page.control(rect(30.0), MenuControlKind::Tab, "tab", None, false, false, open);
    page.control(rect(40.0), MenuControlKind::Slot, "kept", None, false, true, open);
    page.control_with_icon(rect(50.0), MenuControlKind::Item, "picked", None, Some("picked.png"), true, false, open);
    page.control_with_icon(rect(60.0), MenuControlKind::Item, "unowned", None, Some("unowned.png"), false, false, None);

    let mut app = App::new();
    // The icons load, so the asset server needs its task pool.
    app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Mesh>();
    app.init_asset::<Image>();
    app.insert_resource(KaleidoscopeMenuConfig { look: look.clone(), ..Default::default() });
    app.insert_resource(ActiveMenuPages::<Page, Action> {
        pages: vec![page],
        active: Some(Page::Items),
        visible: true,
        version: 1,
    });
    app.world_mut().spawn(MenuRing);
    app.add_systems(Update, (rebuild_cube_faces::<Page, Action>, super::sync_control_focus_visuals).chain());
    app.update();

    let colour_of = |app: &mut App, entity: Entity| {
        let handle = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).unwrap().0.clone();
        app.world().resource::<Assets<StandardMaterial>>().get(&handle).unwrap().base_color
    };
    let controls: Vec<(Entity, MenuControlKind, bool)> = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &KaleidoscopeControlStyle)>();
        query.iter(world).map(|(entity, style)| (entity, style.kind, style.disabled)).collect()
    };
    let control = |kind, disabled| {
        controls
            .iter()
            .find(|(_, k, d)| *k == kind && *d == disabled)
            .map(|(entity, ..)| *entity)
            .unwrap_or_else(|| panic!("no {kind:?} control (disabled {disabled})"))
    };
    let (item, none, bar) = (
        control(MenuControlKind::Item, false),
        control(MenuControlKind::Action, true),
        control(MenuControlKind::Scrollbar, false),
    );
    assert_eq!(colour_of(&mut app, item), look.item, "an item cell");
    assert_eq!(colour_of(&mut app, none), look.disabled, "a control that cannot be used");
    assert_eq!(colour_of(&mut app, bar), look.scrollbar_track, "a scroll bar's track");
    let thumb = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &Name)>();
        query.iter(world).find(|(_, name)| name.as_str() == "scrollbar thumb").map(|(entity, _)| entity)
    }
    .expect("the scroll bar draws its thumb");
    assert_eq!(colour_of(&mut app, thumb), look.scrollbar_thumb, "a scroll bar's thumb");
    assert_eq!(colour_of(&mut app, control(MenuControlKind::Tab, false)), look.control, "any other control");
    assert_eq!(colour_of(&mut app, control(MenuControlKind::Slot, false)), look.important, "an important control");
    let materials: Vec<(Color, bool)> = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .iter()
        .map(|(_, material)| (material.base_color, material.base_color_texture.is_some()))
        .collect();
    assert!(materials.contains(&(look.edge_frame, false)), "the face's frame is not the config's colour");
    assert!(materials.contains(&(look.selected, false)), "the control under the cursor is not the config's colour");
    assert!(materials.contains(&(look.action, false)), "the page-turn arrows are not the config's colour");
    assert!(materials.contains(&(look.icon_selected, true)), "the icon under the cursor is not the config's tint");
    assert!(materials.contains(&(look.icon_disabled, true)), "an icon that cannot be used is not the config's tint");

    // The cursor's state changes on the bar and leaves it: it keeps its track.
    app.world_mut().get_mut::<super::MenuVisualState>(bar).unwrap().set_changed();
    app.update();
    assert_eq!(colour_of(&mut app, bar), look.scrollbar_track, "a scroll bar after its state changed");
}
