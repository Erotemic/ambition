use super::model::{MAX_ROWS, RADIO_VISIBLE_ROWS};
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
fn radio_page_is_windowed_for_mobile_sized_menus() {
    // The radio track catalog is already large enough that showing one row per
    // track overflows the pause panel on small/mobile displays. Keep the
    // visible radio rows capped below the backing row-slot pool so the shared
    // windowing helpers scroll the list instead of rendering every track.
    assert!(RADIO_VISIBLE_ROWS < MAX_ROWS);
    assert_eq!(RADIO_VISIBLE_ROWS, 8);
    assert_eq!(
        crate::ui_nav::windowed_title("Radio", 12, 26, RADIO_VISIBLE_ROWS),
        "Radio — 13/26"
    );
    assert_eq!(visible_row_index(0, 12, 26, RADIO_VISIBLE_ROWS), Some(8));
    assert_eq!(
        visible_row_index(RADIO_VISIBLE_ROWS - 1, 12, 26, RADIO_VISIBLE_ROWS),
        Some(15)
    );
    assert_eq!(
        visible_row_index(RADIO_VISIBLE_ROWS, 12, 26, RADIO_VISIBLE_ROWS),
        None
    );
}
