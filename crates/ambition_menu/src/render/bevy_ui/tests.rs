use super::*;
use crate::scrollbar_thumb_layout;
use crate::{MenuColor, MenuFocusKey};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Inventory,
    System,
    Map,
    Quest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Equip,
    Setting,
}

fn tab_set() -> Vec<BevyUiMenuTabSpec<Page>> {
    vec![
        BevyUiMenuTabSpec::new(Page::Inventory, "Inventory"),
        BevyUiMenuTabSpec::new(Page::System, "System"),
        BevyUiMenuTabSpec::new(Page::Map, "Map"),
        BevyUiMenuTabSpec::new(Page::Quest, "Quest"),
    ]
}

/// A page with two actionable controls + a non-actionable label, and a
/// scrolling scrollbar. Returns the page plus the focus key of the first
/// control so tests can request it focused.
fn sample_page() -> (MenuPageModel<Page, Action>, MenuFocusKey) {
    let mut page = MenuPageModel::new(Page::Inventory, "Inventory", MenuColor::BLUE_PANEL);
    page.text(
        50.0,
        4.0,
        5.0,
        "Inventory",
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );
    let r0 = MenuRect::new(10.0, 20.0, 30.0, 8.0);
    let r1 = MenuRect::new(10.0, 30.0, 30.0, 8.0);
    page.control(
        r0,
        MenuControlKind::Item,
        "Health",
        None,
        false,
        false,
        Some(Action::Equip),
    );
    page.control(
        r1,
        MenuControlKind::Action,
        "Audio",
        None,
        false,
        false,
        Some(Action::Setting),
    );
    // A label with no action (not actionable).
    page.control(
        MenuRect::new(10.0, 40.0, 30.0, 8.0),
        MenuControlKind::Decoration,
        "Label",
        None,
        false,
        false,
        None,
    );
    // A scrolling scrollbar (size < 1 → thumb drawn).
    page.scrollbar(MenuRect::new(92.0, 20.0, 4.0, 60.0), 0.25, 0.5);
    let focus0 = focus_key_for(r0);
    (page, focus0)
}

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app
}

/// Queue the spawn, run one update so the command applies, then assert.
fn spawn_view(app: &mut App, active_tab: usize, focused: Option<MenuFocusKey>) {
    let (page, _) = sample_page();
    let tabs = tab_set();
    app.world_mut().commands().queue(move |world: &mut World| {
        let view = BevyUiMenuView {
            tabs: &tabs,
            active_tab,
            page: &page,
            focused,
            focused_tab: None,
        };
        let mut commands = world.commands();
        spawn_bevy_ui_menu(&mut commands, &view);
    });
    app.update();
}

#[test]
fn spawns_one_tab_button_per_tab_with_active_flagged() {
    let mut app = build_app();
    spawn_view(&mut app, 1, None);

    let mut q = app.world_mut().query::<&BevyUiMenuTab>();
    let mut tabs: Vec<_> = q.iter(app.world()).copied().collect();
    tabs.sort_by_key(|t| t.index);
    assert_eq!(tabs.len(), 4, "one button per tab");
    let active: Vec<usize> = tabs.iter().filter(|t| t.active).map(|t| t.index).collect();
    assert_eq!(active, vec![1], "exactly the active tab is flagged");
}

#[test]
fn selected_and_highlighted_are_distinct_colors() {
    // Highlighted (cursor/hover), selected (equipped/active), and both
    // together must all have different backgrounds.
    let k = MenuControlKind::Item;
    let highlighted = control_bg(k, true, false, false);
    let selected = control_bg(k, false, true, false);
    let both = control_bg(k, true, true, false);
    let plain = control_bg(k, false, false, false);
    assert_ne!(highlighted, selected, "highlighted ≠ selected");
    assert_ne!(highlighted, both, "highlighted ≠ selected+highlighted");
    assert_ne!(selected, both, "selected ≠ selected+highlighted");
    assert_ne!(selected, plain, "selected ≠ plain");
    assert_ne!(highlighted, plain, "highlighted ≠ plain");
}

#[test]
fn focused_tab_is_flagged_on_the_tab_button() {
    // When the view reports a focused tab (keyboard on the tab bar), that tab
    // button carries `focused: true` and no other does.
    let mut app = build_app();
    let (page, _) = sample_page();
    let tabs = tab_set();
    app.world_mut().commands().queue(move |world: &mut World| {
        let view = BevyUiMenuView {
            tabs: &tabs,
            active_tab: 0,
            page: &page,
            focused: None,
            focused_tab: Some(2),
        };
        let mut commands = world.commands();
        spawn_bevy_ui_menu(&mut commands, &view);
    });
    app.update();

    let mut q = app.world_mut().query::<&BevyUiMenuTab>();
    let focused: Vec<usize> = q
        .iter(app.world())
        .filter(|t| t.focused)
        .map(|t| t.index)
        .collect();
    assert_eq!(focused, vec![2], "exactly the focused tab is flagged");
}

#[test]
fn controls_present_tagged_with_action_and_focus_key() {
    let mut app = build_app();
    spawn_view(&mut app, 0, None);

    let mut q = app.world_mut().query::<&AmbitionMenuControl<Action>>();
    let controls: Vec<_> = q.iter(app.world()).cloned().collect();
    // 2 actionable + 1 label + 1 scrollbar = 4 control entities.
    assert_eq!(controls.len(), 4);
    let actions: Vec<Action> = controls.iter().filter_map(|c| c.action).collect();
    assert!(actions.contains(&Action::Equip));
    assert!(actions.contains(&Action::Setting));
    // The item control carries the focus key derived from its rect.
    let item = controls
        .iter()
        .find(|c| c.action == Some(Action::Equip))
        .unwrap();
    assert_eq!(
        item.focus,
        focus_key_for(MenuRect::new(10.0, 20.0, 30.0, 8.0))
    );
}

/// The Equip row's entity in the spawned sample page.
fn equip_row(app: &mut App) -> Entity {
    let mut q = app
        .world_mut()
        .query::<(Entity, &AmbitionMenuControl<Action>)>();
    q.iter(app.world())
        .find_map(|(entity, control)| (control.action == Some(Action::Equip)).then_some(entity))
        .expect("sample page has an Equip row")
}

fn set_interaction(app: &mut App, entity: Entity, interaction: Interaction) {
    app.world_mut().entity_mut(entity).insert(interaction);
    app.update();
}

fn drain_activations(app: &mut App) -> Vec<crate::MenuActionActivated<Action>> {
    app.world_mut()
        .resource_mut::<Messages<crate::MenuActionActivated<Action>>>()
        .drain()
        .collect()
}

#[test]
fn a_row_activates_when_the_pointer_comes_up_on_it_not_when_it_goes_down() {
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    spawn_view(&mut app, 0, None);
    let entity = equip_row(&mut app);

    set_interaction(&mut app, entity, Interaction::Pressed);
    assert!(
        drain_activations(&mut app).is_empty(),
        "going down on a row is not choosing it"
    );

    // A held touch must not fire while it is held, however many frames pass.
    app.update();
    assert!(drain_activations(&mut app).is_empty());

    // Bevy reports a release OVER the control as a return to `Hovered`.
    set_interaction(&mut app, entity, Interaction::Hovered);
    assert_eq!(
        drain_activations(&mut app),
        vec![crate::MenuActionActivated {
            action: Action::Equip,
        }],
        "coming up on the row is",
    );

    app.update();
    assert!(
        drain_activations(&mut app).is_empty(),
        "and it fires once, not for every frame the pointer rests there"
    );

    // The next press arms again.
    set_interaction(&mut app, entity, Interaction::Pressed);
    set_interaction(&mut app, entity, Interaction::Hovered);
    assert_eq!(drain_activations(&mut app).len(), 1);
}

#[test]
fn a_press_that_leaves_the_row_activates_nothing() {
    // A leave and a release are both `Interaction::None`. The bridge must not
    // treat a leave as a release, or dragging on a list activates rows.
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    spawn_view(&mut app, 0, None);
    let entity = equip_row(&mut app);

    set_interaction(&mut app, entity, Interaction::Pressed);
    set_interaction(&mut app, entity, Interaction::None);
    assert!(
        drain_activations(&mut app).is_empty(),
        "the pointer left the row; it chose nothing"
    );

    // And the abandoned arm does not fire later when the pointer wanders back.
    set_interaction(&mut app, entity, Interaction::Hovered);
    assert!(drain_activations(&mut app).is_empty());
}

#[test]
fn an_arm_survives_the_page_respawning_under_the_finger() {
    // The arm is keyed by control, not entity: a page rebuild puts press and
    // release on two different entities for one control.
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    spawn_view(&mut app, 0, None);
    let entity = equip_row(&mut app);
    set_interaction(&mut app, entity, Interaction::Pressed);

    // The page respawns: every control is a new entity, and for a frame the
    // armed one is not in the world at all.
    let roots: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<BevyUiMenuRoot>>();
        q.iter(app.world()).collect()
    };
    for root in roots {
        app.world_mut().entity_mut(root).despawn();
    }
    app.update();
    assert!(
        drain_activations(&mut app).is_empty(),
        "a control vanishing is not a release"
    );

    spawn_view(&mut app, 0, None);
    let respawned = equip_row(&mut app);
    assert_ne!(respawned, entity, "the rebuild really did move the entity");
    set_interaction(&mut app, respawned, Interaction::Hovered);
    assert_eq!(
        drain_activations(&mut app),
        vec![crate::MenuActionActivated {
            action: Action::Equip,
        }],
        "the finger came up on the control it pressed, whatever entity now draws it",
    );
}

#[test]
fn interaction_pressed_ignores_disabled_rows() {
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    spawn_view(&mut app, 0, None);

    let entity = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &AmbitionMenuControl<Action>)>();
        q.iter(app.world())
            .find_map(|(entity, control)| control.action.is_none().then_some(entity))
            .expect("sample page has a disabled decoration row")
    };
    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Pressed);
    app.update();

    assert!(
        app.world_mut()
            .resource_mut::<Messages<crate::MenuActionActivated<Action>>>()
            .drain()
            .next()
            .is_none(),
        "a pickable row with no action remains non-activating",
    );
}

#[test]
fn a_tab_activates_on_the_way_up_like_every_other_control() {
    // A finger that lands on a tab and slides is moving the page, so tabs
    // activate on release, like rows.
    let mut app = build_app();
    install_bevy_ui_menu_tabs(&mut app);
    spawn_view(&mut app, 0, None);

    let entity = {
        let mut q = app.world_mut().query::<(Entity, &BevyUiMenuTab)>();
        q.iter(app.world())
            .find_map(|(entity, tab)| (tab.index == 2).then_some(entity))
            .expect("sample view has tab 2")
    };
    let drain = |app: &mut App| -> Vec<crate::MenuTabActivated> {
        app.world_mut()
            .resource_mut::<Messages<crate::MenuTabActivated>>()
            .drain()
            .collect()
    };

    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Pressed);
    app.update();
    assert!(drain(&mut app).is_empty(), "down is not a tab change");

    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Hovered);
    app.update();
    assert_eq!(drain(&mut app), vec![crate::MenuTabActivated { index: 2 }]);

    // A press that slides off the tab changes nothing.
    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Pressed);
    app.update();
    app.world_mut().entity_mut(entity).insert(Interaction::None);
    app.update();
    assert!(drain(&mut app).is_empty(), "the finger left the tab");
}

#[test]
fn focused_control_is_flagged_and_only_one() {
    let mut app = build_app();
    let (_, focus0) = sample_page();
    spawn_view(&mut app, 0, Some(focus0));

    let mut focused_q = app
        .world_mut()
        .query::<(&BevyUiMenuFocused, &AmbitionMenuControl<Action>)>();
    let flagged: Vec<_> = focused_q.iter(app.world()).collect();
    assert_eq!(flagged.len(), 1, "exactly one focused control");
    assert_eq!(flagged[0].1.action, Some(Action::Equip));

    let mut vs_q = app
        .world_mut()
        .query::<(&BevyUiMenuFocused, &MenuVisualState)>();
    let (_, vs) = vs_q.single(app.world()).unwrap();
    assert!(vs.focused, "focused control's visual state is focused");
}

#[test]
fn scrollbar_spawns_track_and_thumb_with_right_fraction() {
    let mut app = build_app();
    spawn_view(&mut app, 0, None);

    let mut bar_q = app.world_mut().query::<&BevyUiMenuScrollbar>();
    let bars: Vec<_> = bar_q.iter(app.world()).copied().collect();
    assert_eq!(bars.len(), 1, "one scrollbar track");
    assert_eq!(
        bars[0].thumb,
        ScrollThumb {
            start: 0.25,
            size: 0.5
        }
    );

    let mut thumb_q = app
        .world_mut()
        .query_filtered::<&Pickable, With<BevyUiMenuScrollbarThumb>>();
    let thumbs: Vec<_> = thumb_q.iter(app.world()).collect();
    assert_eq!(thumbs.len(), 1, "a scrolling scrollbar draws a thumb");
    // The thumb is not pickable, so a grab on it falls through to the track;
    // otherwise click-drag breaks.
    assert!(
        !thumbs[0].is_hoverable && !thumbs[0].should_block_lower,
        "scrollbar thumb must be Pickable::IGNORE so the track owns the drag",
    );
}

#[test]
fn full_size_scrollbar_draws_no_thumb() {
    let mut app = build_app();
    let mut page: MenuPageModel<Page, Action> =
        MenuPageModel::new(Page::System, "System", MenuColor::BLUE_PANEL);
    // size >= 1 → list fits → no thumb.
    page.scrollbar(MenuRect::new(92.0, 20.0, 4.0, 60.0), 0.0, 1.0);
    let tabs = tab_set();
    app.world_mut().commands().queue(move |world: &mut World| {
        let view = BevyUiMenuView {
            tabs: &tabs,
            active_tab: 1,
            page: &page,
            focused: None,
            focused_tab: None,
        };
        let mut commands = world.commands();
        spawn_bevy_ui_menu(&mut commands, &view);
    });
    app.update();

    let mut bar_q = app.world_mut().query::<&BevyUiMenuScrollbar>();
    assert_eq!(bar_q.iter(app.world()).count(), 1);
    let mut thumb_q = app.world_mut().query::<&BevyUiMenuScrollbarThumb>();
    assert_eq!(
        thumb_q.iter(app.world()).count(),
        0,
        "a non-scrolling list draws no thumb"
    );
}

#[test]
fn item_cell_with_icon_spawns_an_image_node() {
    // An owned item cell with an icon path renders an `ImageNode` when an
    // `AssetServer` is available, like the cube.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>();

    let mut page: MenuPageModel<Page, Action> =
        MenuPageModel::new(Page::Inventory, "Inventory", MenuColor::BLUE_PANEL);
    page.control_with_icon(
        MenuRect::new(10.0, 20.0, 12.0, 12.0),
        MenuControlKind::Item,
        "Health",
        None,
        Some("items/health.png"),
        false,
        false,
        Some(Action::Equip),
    );
    let tabs = tab_set();
    app.world_mut().commands().queue(move |world: &mut World| {
        let view = BevyUiMenuView {
            tabs: &tabs,
            active_tab: 0,
            page: &page,
            focused: None,
            focused_tab: None,
        };
        let assets = world.get_resource::<AssetServer>().cloned();
        let mut commands = world.commands();
        spawn_bevy_ui_menu_with_assets(&mut commands, &view, assets.as_ref());
    });
    app.update();

    let mut icon_q = app.world_mut().query::<&ImageNode>();
    assert_eq!(
        icon_q.iter(app.world()).count(),
        1,
        "an item cell with an icon spawns one ImageNode"
    );
}

#[test]
fn item_cell_without_assets_falls_back_to_label() {
    // With no AssetServer (the headless path), an icon cell renders its label
    // and no ImageNode.
    let mut app = build_app();
    let mut page: MenuPageModel<Page, Action> =
        MenuPageModel::new(Page::Inventory, "Inventory", MenuColor::BLUE_PANEL);
    page.control_with_icon(
        MenuRect::new(10.0, 20.0, 12.0, 12.0),
        MenuControlKind::Item,
        "Health",
        None,
        Some("items/health.png"),
        false,
        false,
        Some(Action::Equip),
    );
    let tabs = tab_set();
    app.world_mut().commands().queue(move |world: &mut World| {
        let view = BevyUiMenuView {
            tabs: &tabs,
            active_tab: 0,
            page: &page,
            focused: None,
            focused_tab: None,
        };
        let mut commands = world.commands();
        spawn_bevy_ui_menu(&mut commands, &view);
    });
    app.update();

    let mut icon_q = app.world_mut().query::<&ImageNode>();
    assert_eq!(
        icon_q.iter(app.world()).count(),
        0,
        "no assets → no ImageNode"
    );
}

#[test]
fn thumb_layout_clamps_and_places_within_track() {
    // Top window → thumb at top.
    let (top, h) = scrollbar_thumb_layout(ScrollThumb {
        start: 0.0,
        size: 0.5,
    });
    assert!(top.abs() < 1e-6);
    assert!((h - 0.5).abs() < 1e-6);
    // Bottom window → thumb flush with bottom (top == 1 - height).
    let (top, h) = scrollbar_thumb_layout(ScrollThumb {
        start: 1.0,
        size: 0.5,
    });
    assert!((top + h - 1.0).abs() < 1e-6);
    // Tiny thumb floored grabbable.
    let (_, h) = scrollbar_thumb_layout(ScrollThumb {
        start: 0.5,
        size: 0.0,
    });
    assert!(h >= 0.08 - 1e-6);
}

/// The track-rect to fraction mapping the scrollbar observers use: top is 0,
/// middle 0.5, bottom 1; off the ends clamps; a zero-height (unmeasured)
/// track gives `None`.
#[test]
fn scrollbar_fraction_maps_pointer_into_track() {
    // Track spans screen y in [100, 300] (top 100, height 200).
    assert_eq!(scrollbar_fraction_from_rect(100.0, 200.0, 100.0), Some(0.0));
    assert_eq!(scrollbar_fraction_from_rect(100.0, 200.0, 200.0), Some(0.5));
    assert_eq!(scrollbar_fraction_from_rect(100.0, 200.0, 300.0), Some(1.0));
    // Off the ends clamps into 0..=1.
    assert_eq!(scrollbar_fraction_from_rect(100.0, 200.0, 50.0), Some(0.0));
    assert_eq!(scrollbar_fraction_from_rect(100.0, 200.0, 999.0), Some(1.0));
    // An unmeasured track (no layout pass yet) yields None.
    assert_eq!(scrollbar_fraction_from_rect(0.0, 0.0, 50.0), None);
}

/// A centered line is centered on its container, not anchored at the center.
///
/// A text node with no width shrinks to its content, so `left: Percent(50)`
/// puts its left edge at the middle, and `Justify::Center` inside a box as
/// wide as the line does nothing. The line would draw to the right.
#[test]
fn a_centred_text_node_spans_its_container_instead_of_starting_at_the_anchor() {
    use super::spawn::text_node;
    use bevy::ui::Val;

    let centred = text_node(50.0, 92.0, MenuTextAlign::Center);
    assert_eq!(
        (centred.left, centred.width),
        (Val::Percent(0.0), Val::Percent(100.0)),
        "a centred line must SPAN the container so justification has room to \
         centre it; anchoring at 50% makes `Justify::Center` a no-op"
    );

    // Right-aligned text ends at its anchor.
    let right = text_node(90.0, 10.0, MenuTextAlign::Right);
    assert_eq!(
        (right.left, right.width),
        (Val::Percent(0.0), Val::Percent(90.0))
    );

    // Left is the one case where the anchor is a left edge.
    let left = text_node(12.0, 10.0, MenuTextAlign::Left);
    assert_eq!(left.left, Val::Percent(12.0));
}

/// The authored percentage reaches the engine as a viewport unit, and follows
/// the viewport when it changes.
///
/// `MenuNode::Text`'s `size` is percent of viewport height; used as pixels it
/// draws text a few pixels tall. The backend spawns `FontSize::Vh`.
///
/// Runs no Ambition system except the spawn. Resolution uses Bevy's
/// `propagate_ui_target_cameras` and `FontSize::eval`, as the text pipeline
/// does. It measures two viewports: one checks the arithmetic, the second
/// checks that the size tracks the target and was not fixed at spawn.
#[test]
fn menu_text_is_sized_as_a_percentage_of_the_live_viewport() {
    use bevy::camera::{Camera, ComputedCameraValues, RenderTargetInfo};
    use bevy::prelude::*;
    use bevy::ui::{IsDefaultUiCamera, UiScale};

    /// The authored size of the sample page's one `MenuNode::Text`.
    const AUTHORED_PERCENT: f32 = 5.0;

    // A UI camera with a stated render-target size and no window or render
    // app. `ComputedCameraValues` is public so a headless test can set one,
    // as `bevy_ui`'s own `propagate_ui_target_cameras` tests do.
    fn target(app: &mut App, camera: Entity, physical_height: u32) {
        app.world_mut()
            .entity_mut(camera)
            .get_mut::<Camera>()
            .expect("the camera keeps its Camera component")
            .computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(1920, physical_height),
                scale_factor: 1.0,
            }),
            ..default()
        };
    }

    fn text_sizes(app: &mut App) -> Vec<f32> {
        let rem = app.world().resource::<bevy::text::RemSize>().0;
        let mut query = app
            .world_mut()
            .query::<(&TextFont, &bevy::ui::ComputedUiRenderTargetInfo)>();
        query
            .iter(app.world())
            // "Not Px", not "is Vh": control labels keep Bevy's default
            // `FontSize::Px(20.0)`. Filtering for `Vh` would drop a wrong
            // unit (`Vw`) from the set instead of failing on it.
            .filter(|(font, _)| !matches!(font.font_size, FontSize::Px(_)))
            .map(|(font, target)| font.font_size.eval(target.logical_size(), rem))
            .collect()
    }

    let mut app = build_app();
    app.add_plugins((
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        // `UiPlugin` schedules `ui_focus_system`, which reads mouse buttons.
        bevy::input::InputPlugin,
        // ... and, under this crate's `bevy_picking` feature,
        // `UiPickingPlugin`. `PickingPlugin` is required: in a workspace
        // build, feature unification enables `bevy/ui_picking`, and `UiPlugin`
        // then runs `widget::viewport_picking`, which reads `Res<HoverMap>`.
        // `-p ambition_menu` alone does not show this.
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::picking::input::PointerInputPlugin,
        bevy::window::WindowPlugin::default(),
        bevy::text::TextPlugin,
        bevy::ui::UiPlugin::default(),
    ));
    app.init_resource::<UiScale>();
    // `update_image_content_size_system` reads it; no plugin here registers it.
    app.init_asset::<bevy::image::TextureAtlasLayout>();
    let camera = app.world_mut().spawn((Camera2d, IsDefaultUiCamera)).id();

    // The real spawn path, as in every other test here.
    spawn_view(&mut app, 0, None);

    target(&mut app, camera, 1080);
    app.update();
    assert_eq!(
        text_sizes(&mut app),
        vec![AUTHORED_PERCENT / 100.0 * 1080.0],
        "the sample page's one text node is authored at {AUTHORED_PERCENT}% of \
         viewport height, so on a 1080-tall target it must resolve to {:.1}px",
        AUTHORED_PERCENT / 100.0 * 1080.0
    );

    // The second viewport. Nothing respawns; only the target changes.
    target(&mut app, camera, 2160);
    app.update();
    assert_eq!(
        text_sizes(&mut app),
        vec![AUTHORED_PERCENT / 100.0 * 2160.0],
        "the size did not follow the viewport, which is the only reason the \
         authored unit is a percentage rather than a number of pixels"
    );
}

/// The Setting row's entity in the spawned sample page.
fn setting_row(app: &mut App) -> Entity {
    let mut q = app
        .world_mut()
        .query::<(Entity, &AmbitionMenuControl<Action>)>();
    q.iter(app.world())
        .find_map(|(entity, control)| (control.action == Some(Action::Setting)).then_some(entity))
        .expect("sample page has a Setting row")
}

/// One tap: press, then release over the same row.
fn tap(app: &mut App, entity: Entity) -> Vec<crate::MenuActionActivated<Action>> {
    set_interaction(app, entity, Interaction::Pressed);
    set_interaction(app, entity, Interaction::Hovered);
    drain_activations(app)
}

#[test]
fn the_destructive_guard_reaches_a_pointer_menu_and_leaves_its_neighbours_alone() {
    // `MenuTapMode::SingleTapWithDestructiveGuard` is the default. Menus drawn
    // by this bridge (including the pause menu's Quit rows) must honor it.
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    // No `UserSettings` resource: absent is the default policy, which is the
    // one under test.
    app.insert_resource(crate::MenuDestructiveActions::<Action>::new(|action| {
        matches!(action, Action::Equip)
    }));
    spawn_view(&mut app, 0, None);
    let equip = equip_row(&mut app);
    let setting = setting_row(&mut app);

    assert_eq!(
        tap(&mut app, setting),
        vec![crate::MenuActionActivated {
            action: Action::Setting
        }],
        "a reversible row still costs one tap; a guard everywhere is only a tax"
    );

    assert!(
        tap(&mut app, equip).is_empty(),
        "the first tap on the destructive row arms it and nothing more"
    );
    assert_eq!(
        tap(&mut app, equip),
        vec![crate::MenuActionActivated {
            action: Action::Equip
        }],
        "the second tap on the SAME row is the answer to the guard"
    );

    // The arm does not survive a visit elsewhere: an armed Quit the user left
    // must not fire on their return tap.
    assert!(tap(&mut app, equip).is_empty(), "arm again");
    assert_eq!(tap(&mut app, setting).len(), 1);
    assert!(
        tap(&mut app, equip).is_empty(),
        "touching another row abandoned the pending confirm"
    );
}

/// Two rows that do the same thing are still two rows.
///
/// The destructive arm is keyed by `MenuFocusKey`, not by action. Keyed by
/// action, arming destructive row A would let row B (with an equal action)
/// fire on its first tap. In a pause menu, that is Quit to Desktop answering
/// a guard armed somewhere else.
#[test]
fn two_destructive_rows_with_the_same_action_arm_separately() {
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    app.insert_resource(crate::MenuDestructiveActions::<Action>::new(|action| {
        matches!(action, Action::Equip)
    }));

    // A page whose two actionable rows carry ONE action and two focus keys.
    let mut page = MenuPageModel::new(Page::Inventory, "Twins", MenuColor::BLUE_PANEL);
    let a = MenuRect::new(10.0, 20.0, 30.0, 8.0);
    let b = MenuRect::new(10.0, 30.0, 30.0, 8.0);
    for rect in [a, b] {
        page.control(
            rect,
            MenuControlKind::Action,
            "Quit",
            None,
            false,
            false,
            Some(Action::Equip),
        );
    }
    let tabs = tab_set();
    app.world_mut().commands().queue(move |world: &mut World| {
        let view = BevyUiMenuView {
            tabs: &tabs,
            active_tab: 0,
            page: &page,
            focused: None,
            focused_tab: None,
        };
        let mut commands = world.commands();
        spawn_bevy_ui_menu(&mut commands, &view);
    });
    app.update();

    let (key_a, key_b) = (focus_key_for(a), focus_key_for(b));
    assert_ne!(key_a, key_b, "the two rows must be distinguishable at all");
    let row = |app: &mut App, key: MenuFocusKey| {
        let mut q = app
            .world_mut()
            .query::<(Entity, &AmbitionMenuControl<Action>)>();
        q.iter(app.world())
            .find_map(|(entity, control)| (control.focus == key).then_some(entity))
            .expect("both twin rows are in the world")
    };
    let (row_a, row_b) = (row(&mut app, key_a), row(&mut app, key_b));

    assert!(tap(&mut app, row_a).is_empty(), "A's first tap arms A");
    assert!(
        tap(&mut app, row_b).is_empty(),
        "B fired on its FIRST tap because it does the same thing as the armed A"
    );
    assert_eq!(
        tap(&mut app, row_b).len(),
        1,
        "B's own second tap is what answers B's guard"
    );
}

#[test]
fn a_menu_that_registers_no_destructive_rows_is_unguarded() {
    // The default for every host: a menu with no irreversible row registers
    // nothing and keeps single-tap.
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    spawn_view(&mut app, 0, None);
    let equip = equip_row(&mut app);

    assert_eq!(
        tap(&mut app, equip).len(),
        1,
        "no registration, no guard, one tap"
    );
}

#[test]
fn single_tap_mode_answers_for_the_destructive_row_too() {
    // The guard follows the user's setting. A user who chose `SingleTap` gets
    // no second tap, even on Quit.
    let mut app = build_app();
    install_bevy_ui_menu_actions::<Action>(&mut app);
    app.insert_resource(crate::MenuDestructiveActions::<Action>::new(|action| {
        matches!(action, Action::Equip)
    }));
    let mut settings = ambition_persistence::settings::UserSettings::default();
    settings.controls.menu_tap_mode = ambition_input::settings::MenuTapMode::SingleTap;
    app.insert_resource(settings);
    spawn_view(&mut app, 0, None);
    let equip = equip_row(&mut app);

    assert_eq!(
        tap(&mut app, equip).len(),
        1,
        "the configured policy wins over the row's riskiness"
    );
}

/// The tab bar republishes (respawns its nodes) constantly, so a press and
/// its release often land on different entities with the same tab index.
/// The press must survive that. The release arm matches on the tab index,
/// which a respawn keeps; this test pins that.
#[test]
fn a_press_that_survives_a_tab_bar_republish_still_activates() {
    let mut app = build_app();
    install_bevy_ui_menu_tabs(&mut app);
    spawn_view(&mut app, 0, None);

    let tab_of = |app: &mut App, index: usize| -> Entity {
        let mut q = app.world_mut().query::<(Entity, &BevyUiMenuTab)>();
        q.iter(app.world())
            .find_map(|(entity, tab)| (tab.index == index).then_some(entity))
            .expect("the sample view has this tab")
    };
    let drain = |app: &mut App| -> Vec<crate::MenuTabActivated> {
        app.world_mut()
            .resource_mut::<Messages<crate::MenuTabActivated>>()
            .drain()
            .collect()
    };

    let pressed = tab_of(&mut app, 2);
    app.world_mut()
        .entity_mut(pressed)
        .insert(Interaction::Pressed);
    app.update();
    assert!(drain(&mut app).is_empty(), "premise: down is not a tab change");

    // The republish: this tab's node is despawned and rebuilt, so the release
    // lands on a different entity with the same index.
    app.world_mut().despawn(pressed);
    let fresh = app
        .world_mut()
        .spawn((
            Button,
            Interaction::None,
            BevyUiMenuTab {
                index: 2,
                active: false,
                focused: false,
            },
        ))
        .id();
    app.update();
    assert_ne!(fresh, pressed, "premise: the republish produced a new entity");

    app.world_mut().entity_mut(fresh).insert(Interaction::Hovered);
    app.update();
    assert_eq!(
        drain(&mut app).into_iter().map(|m| m.index).collect::<Vec<_>>(),
        vec![2],
        "a press that survived a republish must still activate the tab it began on"
    );
}
