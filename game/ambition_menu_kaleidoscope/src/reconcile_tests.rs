//! A scrolled list is rewritten in place, and only a node whose entity tree
//! changes is respawned.
//!
//! Asserted on entity identity: a respawned plane has a new `Entity`. Under
//! Bevy 0.19 a new solid plane draws nothing on its first frame, so every
//! entity these tests keep is a plane that does not blink.

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_lunex::prelude::Text3d;

use ambition_menu::{
    ActiveMenuPages, AmbitionMenuPage, MenuColor, MenuControlKind, MenuNode, MenuPageModel,
    MenuRect, MenuTextAlign, ScrollThumb,
};

use super::{rebuild_cube_faces, KaleidoscopeMenuConfig, MenuRing, RenderedNode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    System,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Row(usize),
}

const ROWS: [&str; 9] = [
    "Radio", "Video", "Audio", "Controls", "Gameplay", "Language", "Reset", "Quit", "Credits",
];
const VISIBLE: usize = 6;

/// The System list as the host publishes it with the window starting at `start`:
/// a title, `VISIBLE` rows, and a scrollbar whose thumb follows the window.
fn system_page(start: usize, detail_on: Option<usize>) -> MenuPageModel<Page, Action> {
    let mut model = MenuPageModel::new(Page::System, "System", MenuColor::BLUE_PANEL);
    model.nodes.push(MenuNode::Text {
        x: 50.0,
        y: 8.0,
        size: 5.0,
        text: format!("SYSTEM {}/{}", start + 1, ROWS.len()),
        align: MenuTextAlign::Center,
        color: MenuColor::WHITE,
    });
    for slot in 0..VISIBLE {
        let row = start + slot;
        model.nodes.push(MenuNode::Control {
            rect: MenuRect::new(10.0, 16.0 + slot as f32 * 9.0, 70.0, 8.0),
            kind: MenuControlKind::OptionToggle,
            label: ROWS[row].to_string(),
            detail: (detail_on == Some(slot)).then(|| "on".to_string()),
            icon: None,
            selected: false,
            important: false,
            action: Some(Action::Row(row)),
            thumb: None,
        });
    }
    model.nodes.push(MenuNode::Control {
        rect: MenuRect::new(84.0, 16.0, 3.0, 54.0),
        kind: MenuControlKind::Scrollbar,
        label: String::new(),
        detail: None,
        icon: None,
        selected: false,
        important: false,
        action: None,
        thumb: Some(ScrollThumb {
            start: start as f32 / (ROWS.len() - VISIBLE) as f32,
            size: VISIBLE as f32 / ROWS.len() as f32,
        }),
    });
    model
}

fn test_app(model: MenuPageModel<Page, Action>) -> App {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Mesh>();
    app.init_asset::<Image>();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.insert_resource(ActiveMenuPages::<Page, Action> {
        pages: vec![model],
        active: Some(Page::System),
        visible: true,
        version: 1,
    });
    app.world_mut().spawn(MenuRing);
    app.add_systems(Update, rebuild_cube_faces::<Page, Action>);
    app.update();
    app
}

fn publish(app: &mut App, model: MenuPageModel<Page, Action>) {
    app.world_mut()
        .resource_mut::<ActiveMenuPages<Page, Action>>()
        .replace_pages(vec![model], Page::System);
    app.update();
}

/// Every entity under the face, sorted.
fn census(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut faces = world.query_filtered::<Entity, With<AmbitionMenuPage<Page>>>();
    let face = faces.single(world).expect("one page, one face");
    let mut children = world.query::<&Children>();
    let mut found = vec![face];
    let mut stack = vec![face];
    while let Some(entity) = stack.pop() {
        if let Ok(kids) = children.get(world, entity) {
            stack.extend(kids.iter());
            found.extend(kids.iter());
        }
    }
    found.sort();
    found
}

/// The node entity drawing model node `index`.
fn node_entity(app: &mut App, index: usize) -> Entity {
    let world = app.world_mut();
    let mut nodes = world.query::<(Entity, &RenderedNode<Action>)>();
    nodes
        .iter(world)
        .find(|(_, node)| node.index == index)
        .map(|(entity, _)| entity)
        .expect("every published node is drawn")
}

fn texts(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut texts = world.query::<&Text3d>();
    let mut found: Vec<String> = texts
        .iter(world)
        .filter_map(|text| text.get_single().map(str::to_string))
        .collect();
    found.sort();
    found
}

/// Scrolling the list one row rewrites the title, every row label and the thumb
/// on the entities already drawing them. Nothing is despawned or spawned.
#[test]
fn a_scroll_step_spawns_and_despawns_nothing() {
    let mut app = test_app(system_page(0, None));
    let before = census(&mut app);
    assert!(
        texts(&mut app).contains(&"Radio".to_string()),
        "the premise: row 0 is drawn"
    );

    publish(&mut app, system_page(1, None));

    assert_eq!(
        census(&mut app),
        before,
        "a scroll step must reuse every entity"
    );
    let drawn = texts(&mut app);
    assert!(
        drawn.contains(&"SYSTEM 2/9".to_string()),
        "the title moved: {drawn:?}"
    );
    assert!(
        drawn.contains(&"Reset".to_string()),
        "the new last row is drawn: {drawn:?}"
    );
    assert!(
        !drawn.contains(&"Radio".to_string()),
        "the row scrolled off is gone: {drawn:?}"
    );
}

/// The thumb follows the scroll on its own plane.
#[test]
fn a_scroll_step_moves_the_thumb_in_place() {
    let mut app = test_app(system_page(0, None));
    let thumb_layout = |app: &mut App| {
        let world = app.world_mut();
        let mut thumbs = world.query::<(&Name, Entity, &bevy_lunex::prelude::UiLayout)>();
        let (_, entity, layout) = thumbs
            .iter(world)
            .find(|(name, _, _)| name.as_str() == "scrollbar thumb")
            .expect("a list longer than its window draws a thumb");
        (entity, format!("{layout:?}"))
    };
    let (thumb, top) = thumb_layout(&mut app);

    publish(&mut app, system_page(3, None));

    let (thumb_after, bottom) = thumb_layout(&mut app);
    assert_eq!(thumb_after, thumb, "the thumb is the same plane");
    assert_ne!(bottom, top, "and it moved");
}

/// A node whose entity tree changes — a row gaining a detail line — is the one
/// node respawned. Its neighbours, and the face, stay.
#[test]
fn a_shape_change_respawns_only_that_node() {
    let mut app = test_app(system_page(0, None));
    let rows_before: Vec<Entity> = (0..=VISIBLE + 1)
        .map(|i| node_entity(&mut app, i))
        .collect();

    publish(&mut app, system_page(0, Some(2)));

    let rows_after: Vec<Entity> = (0..=VISIBLE + 1)
        .map(|i| node_entity(&mut app, i))
        .collect();
    for (index, (before, after)) in rows_before.iter().zip(&rows_after).enumerate() {
        // Node 0 is the title; the row with slot 2 is node 3.
        if index == 3 {
            assert_ne!(
                before, after,
                "the row that gained a detail line is respawned"
            );
        } else {
            assert_eq!(
                before, after,
                "node {index} kept its shape and must be kept"
            );
        }
    }
    assert!(
        texts(&mut app).contains(&"on".to_string()),
        "the new detail line is drawn"
    );
}

/// A shorter publication retires the nodes past its end, and a longer one spawns
/// them, without touching the nodes both share.
#[test]
fn nodes_past_the_end_are_retired_and_new_ones_spawned() {
    let mut app = test_app(system_page(0, None));
    let title = node_entity(&mut app, 0);

    let mut shorter = system_page(0, None);
    shorter.nodes.truncate(3);
    publish(&mut app, shorter);
    let world = app.world_mut();
    let drawn = world.query::<&RenderedNode<Action>>().iter(world).count();
    assert_eq!(drawn, 3, "the nodes past the new end are despawned");
    assert_eq!(node_entity(&mut app, 0), title);

    publish(&mut app, system_page(0, None));
    let world = app.world_mut();
    let drawn = world.query::<&RenderedNode<Action>>().iter(world).count();
    assert_eq!(drawn, VISIBLE + 2, "the returning nodes are spawned again");
    assert_eq!(node_entity(&mut app, 0), title);
}
