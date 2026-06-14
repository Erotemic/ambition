    use super::*;
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

        let mut thumb_q = app.world_mut().query::<&BevyUiMenuScrollbarThumb>();
        assert_eq!(
            thumb_q.iter(app.world()).count(),
            1,
            "a scrolling scrollbar draws a thumb"
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
