use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestPage {
    Items,
    System,
}

#[test]
fn unchanged_version_and_active_page_do_not_invalidate_faces() {
    let pages = ActiveMenuPages::<TestPage, ()> {
        pages: Vec::new(),
        active: Some(TestPage::Items),
        visible: true,
        version: 7,
    };

    assert!(!renderer_page_identity_changed(
        Some(7),
        Some(&TestPage::Items),
        &pages,
    ));
}

#[test]
fn version_or_active_page_changes_invalidate_faces() {
    let pages = ActiveMenuPages::<TestPage, ()> {
        pages: Vec::new(),
        active: Some(TestPage::System),
        visible: true,
        version: 8,
    };

    assert!(renderer_page_identity_changed(
        Some(7),
        Some(&TestPage::System),
        &pages,
    ));
    assert!(renderer_page_identity_changed(
        Some(8),
        Some(&TestPage::Items),
        &pages,
    ));
    assert!(renderer_page_identity_changed(None, None, &pages));
}
