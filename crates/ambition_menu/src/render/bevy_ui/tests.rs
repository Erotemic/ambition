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
    // Fix 2: highlighted (cursor/hover), selected (equipped/active), and the two
    // together must all read as DIFFERENT control backgrounds.
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
    // Fix 4: when the view reports a focused tab (keyboard on the tab bar), that
    // tab button carries `focused: true` and no other does.
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
    // a leave and a release are the same `Interaction::None`, and treating
    // one as the other is what made dragging on a list dangerous.
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
    // the reason the arm is keyed on the ACTION: a menu page rebuilds its
    // controls, so press and release land on two different entities for one
    // control. That is the `Pointer<Click>` failure this bridge must not have.
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
    // A tab bar is a strip of touch targets along the top of a scrollable page:
    // a finger that lands on one and slides is moving the page. Leaving tabs on
    // press-activate beside rows on release-activate is the drift a shared
    // renderer exists to prevent.
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
    // The thumb must be non-pickable so a grab on the thumb falls through to the track — otherwise
    // click-drag breaks.
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
    // Fix 3: an owned item cell carrying an icon path renders an `ImageNode`
    // (the sprite icon) when an `AssetServer` is available, like the cube does.
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
    // With no AssetServer (the cube/headless path), an icon cell still renders
    // its label and NO ImageNode — the renderer degrades gracefully.
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

/// Feature C: the pure track-rect → fraction mapping the `bevy_ui` scrollbar
/// observers use. A pointer at the track top is 0, mid is 0.5, bottom is 1; off
/// the ends clamps; a zero-height (unmeasured) track yields `None`.
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

/// A CENTRED line is centred on its container, not anchored at the centre.
///
/// A text node with no width shrinks to its content, so `left: Percent(50)` puts
/// the node's LEFT EDGE at the middle and the line runs off to the right —
/// `Justify::Center` then centres the line inside a box exactly as wide as the
/// line, which does nothing. Every "centred" heading and footer in the shell was
/// drawn to the RIGHT of where it was asked to be.
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

    // Right-aligned spans up to its anchor, so the line ENDS there.
    let right = text_node(90.0, 10.0, MenuTextAlign::Right);
    assert_eq!(
        (right.left, right.width),
        (Val::Percent(0.0), Val::Percent(90.0))
    );

    // Left is the one case where the anchor genuinely is a left edge.
    let left = text_node(12.0, 10.0, MenuTextAlign::Left);
    assert_eq!(left.left, Val::Percent(12.0));
}

/// The size a menu is SPAWNED at is never the size it is presented at, and
/// the gap has to close inside one frame.
///
/// This asserts the closing, not the schedule: one update, spawned and
/// corrected, whatever set it ends up in.
#[test]
fn text_spawned_this_frame_is_already_the_windows_size_when_the_frame_ends() {
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, Window, WindowResolution};

    const WINDOW_HEIGHT: f32 = 720.0;
    let fraction = crate::MenuTextHeightFraction(5.0);

    let mut app = App::new();
    install_bevy_ui_menu_text_scaling(&mut app);
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(1280, WINDOW_HEIGHT as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    // Spawned from a system, in `Update`, exactly like every real menu rebuild:
    // the entity does not exist until this frame's commands are applied.
    app.add_systems(Update, move |mut commands: Commands| {
        commands.spawn((
            Text::new("Ambition"),
            TextFont {
                font_size: FontSize::Px(fraction.reference_pixels()),
                ..default()
            },
            fraction,
        ));
    });

    app.update();

    let mut query = app.world_mut().query::<&TextFont>();
    let sizes: Vec<FontSize> = query.iter(app.world()).map(|font| font.font_size).collect();
    assert_eq!(
        sizes,
        vec![FontSize::Px(fraction.pixels_at(WINDOW_HEIGHT))],
        "the node still carries its {:.1}px reference size at the end of the frame \
         it was spawned in — that size is what the player sees flash",
        fraction.reference_pixels(),
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
    // `MenuTapMode::SingleTapWithDestructiveGuard` is the SHIPPED DEFAULT, and
    // its stated reason is a stray touch on Quit. It was reaching only the rows
    // routed through `ambition_ui_nav`; every menu drawn by this bridge — the
    // pause menu and its Quit rows included — activated on the first release.
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

    // And the arm does not survive going somewhere else in between: an armed
    // Quit that the user walked away from must not fire on their return tap.
    assert!(tap(&mut app, equip).is_empty(), "arm again");
    assert_eq!(tap(&mut app, setting).len(), 1);
    assert!(
        tap(&mut app, equip).is_empty(),
        "touching another row abandoned the pending confirm"
    );
}

/// ⛔⛔ TWO ROWS THAT DO THE SAME THING ARE STILL TWO ROWS.
///
/// The destructive arm was keyed by `format!("{action:?}")`, then by `Action`
/// itself. The second removed a `Debug` dependency and was still one layer too
/// coarse: an action says what a row DOES. Arm destructive row A, tap
/// destructive row B once, and B — carrying an EQUAL action — reads as already
/// armed and fires on the first tap. In a pause menu that is *Quit to Desktop*
/// answering a guard the user armed somewhere else.
///
/// ⭐ `MenuFocusKey` is the identity the menu already carries, and it is
/// distinct here while the action is not — which is exactly the case neither
/// earlier key could tell apart.
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
    // The default for every existing host, and the reason this change needed no
    // edit at 21 `MenuPage::control` call sites: a menu with no irreversible row
    // registers nothing and keeps single-tap throughout.
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
    // The guard is the user's setting, not this bridge's opinion: someone who
    // has chosen `SingleTap` has said they do not want the second tap, and a
    // Quit row is exactly where a hardcoded policy would override them.
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
