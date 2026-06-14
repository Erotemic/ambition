    //! Pure-logic coverage for the settings model: the menu's page graph,
    //! row tables, action routing, and label formatting. `apply_action`
    //! itself is World-coupled (it pokes the primary `Window`), but the
    //! helpers it delegates to are pure and are where the navigation bugs
    //! would actually live, so they are what we pin here.
    use super::*;

    #[test]
    fn display_mode_cycle_is_a_three_step_loop() {
        let start = DisplayModeKind::Windowed;
        let a = next_display_mode(start);
        let b = next_display_mode(a);
        let c = next_display_mode(b);
        assert_eq!(c, start, "three Next presses return to the start");
        assert_ne!(start, a);
        assert_ne!(a, b);
        assert_ne!(start, b);
    }

    #[test]
    fn display_mode_prev_inverts_next() {
        for kind in [
            DisplayModeKind::Windowed,
            DisplayModeKind::Borderless,
            DisplayModeKind::Fullscreen,
        ] {
            assert_eq!(prev_display_mode(next_display_mode(kind)), kind);
            assert_eq!(next_display_mode(prev_display_mode(kind)), kind);
        }
    }

    #[test]
    fn page_nav_outcome_opens_each_subpage_and_back_pops() {
        use SettingsItem as I;
        use SettingsOutcome::{OpenPage, PopPage};
        assert_eq!(
            page_nav_outcome(I::OpenVideo),
            Some(OpenPage(SettingsPage::Video))
        );
        assert_eq!(
            page_nav_outcome(I::OpenShaders),
            Some(OpenPage(SettingsPage::Shaders))
        );
        assert_eq!(
            page_nav_outcome(I::OpenAudio),
            Some(OpenPage(SettingsPage::Audio))
        );
        assert_eq!(
            page_nav_outcome(I::OpenControls),
            Some(OpenPage(SettingsPage::Controls))
        );
        assert_eq!(
            page_nav_outcome(I::OpenGameplay),
            Some(OpenPage(SettingsPage::Gameplay))
        );
        assert_eq!(
            page_nav_outcome(I::OpenDeveloper),
            Some(OpenPage(SettingsPage::Developer))
        );
        assert_eq!(page_nav_outcome(I::Back), Some(PopPage));
        // A content row (cycles a value) is not a page-navigation row.
        assert_eq!(page_nav_outcome(I::DisplayMode), None);
    }

    #[test]
    fn page_nav_label_present_iff_nav_row() {
        assert_eq!(page_nav_label(SettingsItem::OpenVideo), Some("Video >"));
        assert_eq!(page_nav_label(SettingsItem::Back), Some("Back"));
        assert_eq!(page_nav_label(SettingsItem::DisplayMode), None);
    }

    #[test]
    fn every_page_has_rows_terminated_by_back_with_no_dupes() {
        for &page in SettingsPage::ALL {
            let rows = SettingsItem::rows_for(page);
            assert!(!rows.is_empty(), "{page:?} has no rows");
            assert_eq!(
                *rows.last().unwrap(),
                SettingsItem::Back,
                "{page:?} should end with a Back row",
            );
            for (i, a) in rows.iter().enumerate() {
                for b in &rows[i + 1..] {
                    assert_ne!(a, b, "{page:?} lists {a:?} twice");
                }
            }
        }
    }

    #[test]
    fn top_page_exposes_reset_and_subpage_entries() {
        let top = SettingsItem::rows_for(SettingsPage::Top);
        for required in [
            SettingsItem::OpenVideo,
            SettingsItem::OpenAudio,
            SettingsItem::OpenControls,
            SettingsItem::OpenGameplay,
            SettingsItem::OpenDeveloper,
            SettingsItem::ResetAllSettings,
        ] {
            assert!(top.contains(&required), "Top page missing {required:?}");
        }
    }

    #[test]
    fn page_titles_are_unique_and_nonempty() {
        let titles: Vec<&str> = SettingsPage::ALL.iter().map(|p| p.title()).collect();
        assert_eq!(titles.len(), SettingsPage::ALL.len());
        for t in &titles {
            assert!(!t.is_empty(), "empty page title");
        }
        for (i, a) in titles.iter().enumerate() {
            for b in &titles[i + 1..] {
                assert_ne!(a, b, "duplicate page title {a:?}");
            }
        }
    }

    #[test]
    fn apply_cycle_routes_prev_and_next() {
        let dec = |x: i32| x - 1;
        let inc = |x: i32| x + 1;
        let mut v = 10;
        apply_cycle(SettingsAction::Next, &mut v, dec, inc);
        assert_eq!(v, 11);
        apply_cycle(SettingsAction::Prev, &mut v, dec, inc);
        assert_eq!(v, 10);
        apply_cycle(SettingsAction::Confirm, &mut v, dec, inc);
        assert_eq!(v, 11, "Confirm advances like Next");
    }

    #[test]
    fn format_helpers_have_the_expected_shape() {
        assert_eq!(
            format_cycle("Camera View", "Wide"),
            "Camera View: Wide  < / >"
        );
        assert_eq!(format_toggle("FPS Overlay", true), "FPS Overlay: on");
        assert_eq!(format_toggle("FPS Overlay", false), "FPS Overlay: off");
    }
