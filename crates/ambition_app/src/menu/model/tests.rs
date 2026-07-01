use super::*;

#[test]
fn items_face_wires_all_24_slots_from_our_catalog() {
    let owned = OwnedItems::default();
    let spec = items_spec(&owned, None);
    assert_eq!(
        spec.cells.len(),
        ambition_gameplay_core::items::ITEM_COUNT,
        "the cube's items face has one cell per inventory slot (24)"
    );
    // Slots are in grid order; labels are wrapped from our catalog.
    for (idx, cell) in spec.cells.iter().enumerate() {
        assert_eq!(cell.slot.0, idx);
        assert_eq!(cell.label, cell_label(Item::ALL[idx].display_name()));
    }
}

#[test]
fn owned_and_equipped_flags_reflect_inventory_state() {
    let mut owned = OwnedItems::default();
    owned.grant(Item::Blink, 1);
    let spec = items_spec(&owned, Some(Item::Blink));
    let blink = &spec.cells[Item::Blink.index()];
    assert!(blink.owned, "granted item reads owned");
    assert!(blink.equipped, "equipped item reads equipped");
    assert!(blink.action.is_some(), "owned item has an action");
    // An un-granted item is unowned + actionless.
    let unowned = spec.cells.iter().find(|c| !c.owned).expect("some unowned");
    assert!(unowned.action.is_none());
}

#[test]
fn cell_labels_wrap_and_stay_short() {
    // Long names must wrap to <= LABEL_MAX_LINES lines, each <= LABEL_WRAP_COLS
    // chars, so they never bleed across neighbouring cells.
    let label = cell_label("Puppy-Slug Gun");
    let lines: Vec<&str> = label.split('\n').collect();
    assert!(
        lines.len() <= LABEL_MAX_LINES,
        "label wraps to few lines: {label:?}"
    );
    for line in lines {
        assert!(
            line.chars().count() <= LABEL_WRAP_COLS,
            "line fits the cell: {line:?}"
        );
    }
}

#[test]
fn item_cells_carry_a_sprite_icon_when_one_exists_else_fall_back_to_text() {
    // Items with authored art emit an `icon` on their grid control; items
    // without art carry `None` (the lib then renders the text label).
    let owned = OwnedItems::default();
    let page = build_items_page(&owned, None);
    // Item-grid controls are emitted in catalog slot order, so the icon list
    // lines up 1:1 with `Item::ALL`.
    let icons: Vec<Option<String>> = page
        .nodes
        .iter()
        .filter_map(|n| match n {
            ambition_menu::MenuNode::Control {
                kind: MenuControlKind::Item,
                icon,
                ..
            } => Some(icon.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(icons.len(), Item::ALL.len(), "one control per catalog item");
    for (item, icon) in Item::ALL.into_iter().zip(icons.iter()) {
        match item.icon_path() {
            Some(path) => assert_eq!(
                icon.as_deref(),
                Some(path),
                "{item:?} should carry its sprite icon"
            ),
            None => assert!(icon.is_none(), "{item:?} has no art → text fallback"),
        }
    }
    // Sanity: at least one of each (a real sprite + a real text fallback).
    assert!(icons.iter().any(|i| i.is_some()), "some items have icons");
    assert!(
        icons.iter().any(|i| i.is_none()),
        "some items fall back to text"
    );
}

#[test]
fn items_page_has_one_detail_panel_not_per_cell_descriptions() {
    // Regression for the "24 overlapping descriptions" mush: NO grid cell may
    // carry the full item description as its detail text.
    let owned = OwnedItems::default();
    let page = build_items_page(&owned, None);
    for node in &page.nodes {
        if let ambition_menu::MenuNode::Control {
            detail: Some(d),
            kind,
            ..
        } = node
        {
            if *kind == MenuControlKind::Item {
                assert!(
                    !d.contains(Item::Blink.description()),
                    "grid cell must not render the full description: {d:?}"
                );
            }
        }
    }
    // The detail panel is now cursor-INDEPENDENT page data: it reserves a fixed
    // set of EMPTY dynamic-text slots (filled in place from the live cursor),
    // so the page itself never bakes the description (a hover would otherwise
    // rebuild the face and drop a `Pointer<Click>` — the deferred Bug 2).
    let has_dynamic_slots = page
        .nodes
        .iter()
        .filter(|n| matches!(n, ambition_menu::MenuNode::DynamicText { .. }))
        .count();
    assert!(
        has_dynamic_slots >= ITEMS_DETAIL_BODY_LINES as usize,
        "items detail panel reserves dynamic-text slots for in-place fill"
    );
    // The in-place text for a focused item renders its description (this is what
    // `kaleidoscope_sync_detail_text` writes into the dynamic slots each move).
    let slot_text = items_detail_slot_text(&owned, None, MenuFocus::Item(Item::Blink.index()));
    let joined: String = slot_text
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        joined.contains(Item::Blink.description()),
        "focused item's description is supplied by the in-place detail slots: {joined:?}"
    );
}

#[test]
fn system_page_top_level_shows_entry_list() {
    let settings = UserSettings::default();
    let focus = MenuFocus::System(0);
    // No entry open -> the top-level view is the SYSTEM entry list.
    let page = build_system_page(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
        focus,
        0,
        None,
    );
    let entries = page
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::OpenSystemEntry(_)),
                    ..
                }
            )
        })
        .count();
    // Radio + Video + Audio + Controls + Gameplay + Language always drill in
    // (6 rows; Shaders is no longer a top-level entry — it rides under Video).
    // Reset All Settings is always present but is an Action (no drill). The 7
    // top-level rows (6 drill entries + the Reset All Settings action) now
    // overflow the SYSTEM_VISIBLE_ROWS (6) window (Fix 2); the first window still
    // shows all 6 drill entries, so exactly 6 drill rows are emitted.
    let expected_drill = 6;
    assert_eq!(
        entries, expected_drill,
        "one drill row per non-action entry"
    );
    // No raw settings toggles leak at the top level.
    let has_setting = page.nodes.iter().any(|n| {
        matches!(
            n,
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::System(_)),
                ..
            }
        )
    });
    assert!(!has_setting, "entry list does not show raw setting toggles");
    // Edge buttons are present so rotation still works.
    let has_edges = page.nodes.iter().any(|n| {
        matches!(
            n,
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::ChangePage(_)),
                ..
            }
        )
    });
    assert!(has_edges, "System page keeps the L/R edge buttons");
}

#[test]
fn value_rows_get_decrease_and_increase_click_zones() {
    // Fix 2: a drilled-in Video screen has value rows (Slider/Cycle, e.g. Camera
    // Zoom). Each such row in the visible window gets a ◀ (dir -1) + ▶ (dir +1)
    // click zone so touch/mouse users can step both ways.
    let settings = UserSettings::default();
    let page = build_system_page(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
        MenuFocus::System(0),
        0,
        Some(SystemMenuEntryId::Video),
    );
    let steps: Vec<(SettingsOptionId, i32)> = page
        .nodes
        .iter()
        .filter_map(|n| match n {
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::SystemStep(o, dir)),
                ..
            } => Some((*o, *dir)),
            _ => None,
        })
        .collect();
    assert!(
        !steps.is_empty(),
        "a value-style settings screen emits step zones"
    );
    // Every step zone is exactly -1 or +1.
    assert!(
        steps.iter().all(|(_, d)| *d == -1 || *d == 1),
        "step zones carry dir -1 or +1: {steps:?}"
    );
    // Every option that has a step zone has BOTH a decrease (-1) and increase (+1).
    let sys_model = SystemMenuModel::build(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let mut value_opts: Vec<SettingsOptionId> = Vec::new();
    for (o, _) in &steps {
        if !value_opts.contains(o) {
            value_opts.push(*o);
        }
    }
    for opt in value_opts {
        assert!(
            is_value_setting_row(&sys_model, opt),
            "{opt:?} with a step zone is a value (Slider/Cycle) row"
        );
        assert!(
            steps.contains(&(opt, -1)),
            "{opt:?} has a decrease (dir -1) zone"
        );
        assert!(
            steps.contains(&(opt, 1)),
            "{opt:?} has an increase (dir +1) zone"
        );
    }
    // A non-value row (DisplayMode is a Cycle, but a pure Toggle like ShowFps is
    // NOT a value row in the keyboard's LEFT/RIGHT sense the way Slider/Cycle are)
    // never gets a step zone unless it is genuinely a value kind.
    assert!(
        !steps
            .iter()
            .any(|(o, _)| !is_value_setting_row(&sys_model, *o)),
        "only value rows get step zones"
    );
}

#[test]
fn system_page_drilled_into_video_shows_curated_options_and_back() {
    let mut settings = UserSettings::default();
    settings.video.show_fps = true;
    let focus = MenuFocus::System(0);
    // Drill into Video -> its curated options (from the SYSTEM IR) + a Back row.
    let page = build_system_page(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
        focus,
        0,
        Some(SystemMenuEntryId::Video),
    );
    let options: Vec<_> = page
        .nodes
        .iter()
        .filter_map(|n| match n {
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::System(o)),
                ..
            } => Some(*o),
            _ => None,
        })
        .collect();
    // The Video screen leads with the transactional Quality Profile row, then the
    // basic video rows. Shader rows still ride under Video later in the full list.
    assert_eq!(
        &options[..3],
        &[
            SettingsOptionId::VisualQuality,
            SettingsOptionId::DisplayMode,
            SettingsOptionId::CameraZoom,
        ]
    );
    // Shaders are reachable under Video: the FULL row list (pre-window) carries
    // every shader option as a Setting row.
    let sys_model = SystemMenuModel::build(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let all_rows = system_rows(&sys_model, Some(SystemMenuEntryId::Video));
    for shader in [
        SettingsOptionId::ShaderStrength,
        SettingsOptionId::ShaderVignetteStrength,
    ] {
        assert!(
            all_rows.contains(&SystemRow::Setting(shader)),
            "{shader:?} is reachable under Video"
        );
    }

    // The FPS Overlay row reflects the ON state we set above. ShowFps now sits
    // past the first visible window (the full player-facing Video set leads the
    // screen), so verify the live label off the IR rather than the windowed page.
    let video_entry = sys_model.entry(SystemMenuEntryId::Video).unwrap();
    let ambition_gameplay_core::menu::ir::system::SystemMenuTarget::Settings(opts) =
        &video_entry.target
    else {
        panic!("video drills into a settings screen");
    };
    let fps = opts
        .iter()
        .find(|o| o.id == SettingsOptionId::ShowFps)
        .expect("ShowFps is on the Video screen");
    assert_eq!(fps.value_label, "ON", "FPS Overlay reflects the ON state");

    // A Back row drills out to the entry list. The Video screen now overflows
    // the visible window (24 rows), so Back is the LAST row in the full list
    // rather than always on the first window; assert it via the row list.
    assert_eq!(
        all_rows.last(),
        Some(&SystemRow::Back),
        "an open entry ends with a Back row"
    );
    // Scrolling to the end brings the Back row into the rendered window.
    let end_start = system_max_window_start(all_rows.len());
    let page_end = build_system_page(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
        MenuFocus::System(all_rows.len() - 1),
        end_start,
        Some(SystemMenuEntryId::Video),
    );
    let has_back = page_end.nodes.iter().any(|n| {
        matches!(
            n,
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::CloseSystemEntry),
                ..
            }
        )
    });
    assert!(has_back, "scrolling to the end renders the Back row");
}

#[test]
fn system_setting_label_tracks_settings_changes() {
    let mut settings = UserSettings::default();
    let model0 = SystemMenuModel::build(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let off = system_row_label(&model0, SystemRow::Setting(SettingsOptionId::QuestHud));
    settings.gameplay.quest_hud_visible = !settings.gameplay.quest_hud_visible;
    let model1 = SystemMenuModel::build(
        &settings,
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let on = system_row_label(&model1, SystemRow::Setting(SettingsOptionId::QuestHud));
    assert_ne!(off, on, "toggling the setting changes the row label");
}

#[test]
fn map_and_quest_edge_buttons_are_focusable() {
    // Fix 1: placeholder pages (Map / Quest) build real, focusable L/R edge
    // buttons. The focus HIGHLIGHT is now applied in place from the live cursor
    // (`kaleidoscope_sync_focus_visuals`) rather than baked into the page data,
    // so the page only needs to emit the two clickable edge controls; landing on
    // one after a page turn highlights it without a rebuild.
    for page in [MenuPage::Map, MenuPage::Quest] {
        let model = placeholder_page(page, "T", "body");
        // Both edge buttons exist as Action controls with a ChangePage action.
        let edges: Vec<_> = model
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    ambition_menu::MenuNode::Control {
                        kind: MenuControlKind::Action,
                        action: Some(MenuPageAction::ChangePage(_)),
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(edges.len(), 2, "{page:?} has both L/R edge buttons");
        // The page data is cursor-independent: NO edge button is baked selected.
        let any_baked_selected = model.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    selected: true,
                    action: Some(MenuPageAction::ChangePage(_)),
                    ..
                }
            )
        });
        assert!(
            !any_baked_selected,
            "{page:?} edge highlight is applied in place, not baked"
        );
    }
}

#[test]
fn short_system_screens_show_every_row_without_an_indicator() {
    // Fix 3/4: a screen that fits shows all rows and adds NO scroll indicator.
    let rows: Vec<SystemRow> = (0..SYSTEM_VISIBLE_ROWS)
        .map(|i| SystemRow::Option(SystemOptionId::Radio(i)))
        .collect();
    let (window, indicator) = system_visible_window(&rows, 0);
    assert_eq!(window.len(), rows.len(), "all rows visible when they fit");
    assert!(indicator.is_none(), "no indicator for a short screen");
    // Absolute indices are identity for a non-windowed list.
    for (slot, (abs, _)) in window.iter().enumerate() {
        assert_eq!(slot, *abs);
    }
}

#[test]
fn long_system_screens_window_the_list_and_follow_the_cursor() {
    // Fix 3/4: a Radio-sized screen (26 rows) windows to SYSTEM_VISIBLE_ROWS and
    // the window follows the cursor, mapping windowed slots back to absolute rows.
    let total = 26usize;
    let rows: Vec<SystemRow> = (0..total)
        .map(|i| SystemRow::Option(SystemOptionId::Radio(i)))
        .collect();

    // Cursor at the top: window starts at 0, indicator reads "1/26".
    let (window, indicator) = system_visible_window(&rows, 0);
    assert_eq!(window.len(), SYSTEM_VISIBLE_ROWS, "list is windowed");
    assert_eq!(window.first().unwrap().0, 0);
    assert_eq!(indicator.as_deref(), Some("1/26"));

    // Cursor mid-list: the focused absolute row stays inside the rendered window.
    let focused = 13;
    let (window, indicator) = system_visible_window(&rows, focused);
    assert_eq!(window.len(), SYSTEM_VISIBLE_ROWS);
    assert!(
        window.iter().any(|(abs, _)| *abs == focused),
        "the focused row scrolls into the visible window"
    );
    assert_eq!(
        indicator.as_deref(),
        Some("14/26"),
        "1-based n/total indicator"
    );

    // Cursor at the bottom: the window clamps to the list end (no overflow).
    let (window, _) = system_visible_window(&rows, total - 1);
    assert_eq!(window.len(), SYSTEM_VISIBLE_ROWS);
    assert_eq!(window.last().unwrap().0, total - 1, "last row reachable");
}

#[test]
fn long_system_page_renders_only_a_window_of_clickable_rows() {
    // The built System page for a long Radio screen renders exactly
    // SYSTEM_VISIBLE_ROWS option controls (all clickable), not all 26.
    let settings = UserSettings::default();
    let radio = RadioSnapshot {
        stations: (0..26).map(|i| (i, format!("Station {i}"))).collect(),
        active: Some(0),
    };
    let focus = MenuFocus::System(13);
    let sys_model = SystemMenuModel::build(&settings, &radio, &DevSnapshot::default());
    let rows = system_rows(&sys_model, Some(SystemMenuEntryId::Radio));
    // Cursor-derived window (no override) keeps the focused station in view.
    let window_start = system_window_start(&rows, focus);
    let page = build_system_page(
        &settings,
        &radio,
        &DevSnapshot::default(),
        focus,
        window_start,
        Some(SystemMenuEntryId::Radio),
    );
    let option_rows = page
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::SystemOption(_)),
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        option_rows, SYSTEM_VISIBLE_ROWS,
        "a long Radio screen renders only the visible window of station rows"
    );
    // The window includes the focused station (index 13). The highlight itself
    // is applied IN PLACE from the live cursor (not baked as `selected`), so the
    // page only needs to RENDER the focused row inside the window.
    let has_focused = page.nodes.iter().any(|n| {
        matches!(
            n,
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::SystemOption(SystemOptionId::Radio(13))),
                ..
            }
        )
    });
    assert!(
        has_focused,
        "the focused station scrolls into the visible window"
    );
}

#[test]
fn scrollbar_thumb_geometry_reflects_window_and_total() {
    // Fix 1: thumb size = visible/total; start = window fraction of travel.
    // 26-row list, 6 visible: size = 6/26 ≈ 0.2308.
    let total = 26usize;
    let (start_top, size) = system_scrollbar_thumb(0, total);
    assert!(
        (size - SYSTEM_VISIBLE_ROWS as f32 / total as f32).abs() < 1e-4,
        "thumb size = visible/total: {size}"
    );
    assert!(
        size < 1.0,
        "an overflowing list scrolls (thumb < full track)"
    );
    assert!(
        (start_top - 0.0).abs() < 1e-4,
        "top window → thumb at the top"
    );

    // Bottom window → thumb at the bottom (start == 1.0).
    let max = system_max_window_start(total);
    let (start_bottom, _) = system_scrollbar_thumb(max, total);
    assert!(
        (start_bottom - 1.0).abs() < 1e-4,
        "bottom window → thumb start 1.0: {start_bottom}"
    );

    // A mid window lands between.
    let (start_mid, _) = system_scrollbar_thumb(max / 2, total);
    assert!(
        start_mid > 0.0 && start_mid < 1.0,
        "mid window → thumb mid-track: {start_mid}"
    );
}

#[test]
fn long_system_page_emits_one_scrollbar_node_with_thumb() {
    // Fix 1: a long Radio screen emits exactly one Scrollbar control carrying the
    // thumb geometry (the lib draws the track + thumb from it).
    let settings = UserSettings::default();
    let radio = RadioSnapshot {
        stations: (0..26).map(|i| (i, format!("Station {i}"))).collect(),
        active: Some(0),
    };
    let page = build_system_page(
        &settings,
        &radio,
        &DevSnapshot::default(),
        MenuFocus::System(0),
        0,
        Some(SystemMenuEntryId::Radio),
    );
    let thumbs: Vec<_> = page
        .nodes
        .iter()
        .filter_map(|n| match n {
            ambition_menu::MenuNode::Control {
                kind: MenuControlKind::Scrollbar,
                thumb: Some(t),
                ..
            } => Some(*t),
            _ => None,
        })
        .collect();
    assert_eq!(thumbs.len(), 1, "exactly one scrollbar node with a thumb");
    assert!(thumbs[0].size < 1.0, "thumb shows the list scrolls");
    assert!(
        (thumbs[0].start - 0.0).abs() < 1e-4,
        "top window → thumb top"
    );

    // A short screen emits NO scrollbar node. A 3-station Radio screen (3 rows +
    // Back = 4) fits inside SYSTEM_VISIBLE_ROWS, so no scrollbar is drawn. (The
    // TOP-LEVEL entry list has 7 rows and now overflows the 6-row window — Fix 2 —
    // so it is no longer a valid "fits" case; drill into a short screen instead.)
    let short_radio = RadioSnapshot {
        stations: (0..3).map(|i| (i, format!("Station {i}"))).collect(),
        active: Some(0),
    };
    let short_page = build_system_page(
        &settings,
        &short_radio,
        &DevSnapshot::default(),
        MenuFocus::System(0),
        0,
        Some(SystemMenuEntryId::Radio),
    );
    let any_scrollbar = short_page.nodes.iter().any(|n| {
        matches!(
            n,
            ambition_menu::MenuNode::Control {
                kind: MenuControlKind::Scrollbar,
                ..
            }
        )
    });
    assert!(!any_scrollbar, "a fitting list draws no scrollbar");
}

#[test]
fn viewer_left_button_turns_to_the_right_neighbor() {
    // Pressing LEFT rotates the cube left, bringing the +1 ring neighbour to
    // front (matches the demo's page_on_viewer_left = index + 1).
    assert_eq!(MenuPage::Items.on_viewer_left(), MenuPage::Map);
    assert_eq!(MenuPage::Items.on_viewer_right(), MenuPage::System);
    let owned = OwnedItems::default();
    let page = build_items_page(&owned, None);
    let left = page.nodes.iter().find_map(|n| match n {
        ambition_menu::MenuNode::Control {
            action: Some(MenuPageAction::ChangePage(p)),
            rect,
            ..
        } if rect.x < 10.0 => Some(*p),
        _ => None,
    });
    assert_eq!(
        left,
        Some(MenuPage::Map),
        "left edge button turns to viewer-left page"
    );
}
