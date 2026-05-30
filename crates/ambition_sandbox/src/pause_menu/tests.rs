use super::*;

#[test]
fn pause_menu_state_default_is_top_page_zero() {
    let s = PauseMenuState::default();
    assert!(matches!(s.page, PauseMenuPage::Top));
    assert_eq!(s.selected, 0);
}

#[test]
fn enter_page_pushes_onto_stack() {
    let mut s = PauseMenuState {
        selected: 3,
        page: PauseMenuPage::Top,
        stack: Vec::new(),
        pointer_armed: None,
        pointer_confirm: false,
        focus: crate::ui_nav::MenuFocusState::default(),
    };
    s.enter_page(PauseMenuPage::Settings(SettingsPage::Top));
    assert!(matches!(s.page, PauseMenuPage::Settings(SettingsPage::Top)));
    assert_eq!(s.selected, 0);
    assert_eq!(s.stack.len(), 1);
    s.pop_page();
    assert!(matches!(s.page, PauseMenuPage::Top));
    assert!(s.stack.is_empty());
}

#[test]
fn pause_menu_item_all_includes_settings() {
    assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::Settings));
}

/// `ResetSandbox` is the user-facing entry point for the
/// "wipe the save and rebuild the runtime" flow. Pin it here so
/// a future menu-shape refactor can't silently drop it.
#[test]
fn pause_menu_item_all_includes_reset_sandbox() {
    assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::ResetSandbox));
    assert_eq!(PauseMenuItem::ResetSandbox.static_label(), "Reset Sandbox");
}

/// `MenuSettingsItem` is the public re-export so other modules can
/// query rows by tag without crossing the private boundary.
#[test]
fn menu_settings_item_is_settings_item() {
    let _ = MenuSettingsItem::DisplayMode;
}

#[test]
fn visible_window_tracks_selected_row_without_overflow() {
    assert_eq!(crate::ui_nav::visible_window_start(0, 12, 5), 0);
    assert_eq!(crate::ui_nav::visible_window_start(4, 12, 5), 2);
    assert_eq!(crate::ui_nav::visible_window_start(11, 12, 5), 7);
    assert_eq!(visible_row_index(0, 11, 12, 5), Some(7));
    assert_eq!(visible_row_index(4, 11, 12, 5), Some(11));
    assert_eq!(visible_row_index(5, 11, 12, 5), None);
}

#[test]
fn radio_rows_are_windowed_for_mobile_panels() {
    assert!(super::model::RADIO_VISIBLE_ROWS < super::model::MAX_ROWS);
    assert_eq!(super::model::RADIO_VISIBLE_ROWS, 8);
    let cursor = crate::ui_nav::ListCursor::new(12, 26);
    assert_eq!(
        cursor.windowed_title("Radio", super::model::RADIO_VISIBLE_ROWS),
        "Radio — 13/26"
    );
    assert_eq!(
        cursor.visible_row_for_slot(0, super::model::RADIO_VISIBLE_ROWS),
        Some(8)
    );
    assert_eq!(
        cursor.visible_row_for_slot(7, super::model::RADIO_VISIBLE_ROWS),
        Some(15)
    );
    assert_eq!(
        cursor.visible_row_for_slot(8, super::model::RADIO_VISIBLE_ROWS),
        None
    );
}
