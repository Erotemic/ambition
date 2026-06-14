    use crate::scrollbar_thumb_layout;
    use crate::ScrollThumb;

    /// Fix 1: the lib maps the host's thumb fractions onto a track-relative
    /// `(y, size)`: size = visible fraction (floored grabbable), y rides the
    /// remaining travel so a top window sits at 0 and a bottom window flush with the
    /// track bottom (`y + size == 1`).
    #[test]
    fn thumb_layout_tracks_start_and_size() {
        // 6/26 visible, top window.
        let size = 6.0 / 26.0;
        let (y_top, s) = scrollbar_thumb_layout(ScrollThumb { start: 0.0, size });
        assert!((s - size).abs() < 1e-4, "size = visible fraction");
        assert!(y_top.abs() < 1e-4, "top window → thumb at the top");

        // Bottom window: thumb flush with the track bottom.
        let (y_bot, s) = scrollbar_thumb_layout(ScrollThumb { start: 1.0, size });
        assert!(
            (y_bot + s - 1.0).abs() < 1e-4,
            "bottom window → thumb at bottom"
        );

        // A tiny visible fraction is floored to a grabbable minimum (8%).
        let (_, s_min) = scrollbar_thumb_layout(ScrollThumb {
            start: 0.0,
            size: 0.01,
        });
        assert!(
            (s_min - 0.08).abs() < 1e-4,
            "thumb floored grabbable: {s_min}"
        );
    }
