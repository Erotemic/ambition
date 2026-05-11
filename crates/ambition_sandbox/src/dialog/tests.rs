use super::ui::DIALOG_VISIBLE_OPTIONS;
use super::*;
use crate::ui_nav::visible_window_start;

#[test]
fn default_state_is_inactive() {
    let s = DialogState::default();
    assert!(!s.active());
}

#[test]
fn start_activates_dialogue() {
    let mut s = DialogState::default();
    s.start("guide", "Guide");
    assert!(s.active());
    let title = s.title();
    assert!(!title.is_empty());
    // Title format is "{speaker} — {mode_label}" when a node
    // exists; otherwise "{npc_name} — dialogue". Either way the
    // separator is present.
    assert!(title.contains('—') || title.contains("dialogue"));
}

#[test]
fn close_deactivates() {
    let mut s = DialogState::default();
    s.start("guide", "Guide");
    s.close();
    assert!(!s.active());
}

#[test]
fn body_returns_routing_bug_message_when_no_node() {
    let mut s = DialogState::default();
    s.start("nonexistent_dialogue_id_for_test", "X");
    // The node index is 0 but the mode for an unknown id may
    // route to a fallback set; either way `body()` must return
    // SOME string (not panic).
    let body = s.body();
    assert!(!body.is_empty());
}

#[test]
fn selected_option_starts_at_zero() {
    let mut s = DialogState::default();
    s.start("guide", "Guide");
    assert_eq!(s.selected_option(), 0);
}

#[test]
fn visible_dialog_window_keeps_selected_option_in_range() {
    assert_eq!(visible_window_start(0, 8, DIALOG_VISIBLE_OPTIONS), 0);
    assert_eq!(visible_window_start(4, 8, DIALOG_VISIBLE_OPTIONS), 2);
    assert_eq!(visible_window_start(7, 8, DIALOG_VISIBLE_OPTIONS), 4);
}
