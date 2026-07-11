//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn rect(x: i32, y: i32, w: i32, h: i32, page: u32, off: (i32, i32)) -> FrameRect {
    FrameRect {
        x,
        y,
        w,
        h,
        page,
        off,
        anchors: Default::default(),
    }
}

fn row(animation: &str, row_index: u32, page: u32, rects: Vec<FrameRect>) -> SheetRow {
    SheetRow {
        animation: animation.to_string(),
        row_index,
        frame_count: rects.len() as u32,
        duration_ms: 100,
        duration_secs: 0.1,
        page,
        rects,
    }
}

fn record(frame_w: u32, frame_h: u32, rows: Vec<SheetRow>) -> SheetRecord {
    SheetRecord {
        target: "t".into(),
        image: "t.png".into(),
        images: vec![],
        label_width: 0,
        frame_width: frame_w,
        frame_height: frame_h,
        y_offset: 0,
        body_metrics: None,
        tuning: None,
        rows,
    }
}

/// A freely-packed sheet scatters one animation's frames across pages.
/// `flat_index_in_page` must be a page-local index that exactly addresses
/// the layout `atlas_page` produces for that page.
#[test]
fn flat_index_agrees_with_atlas_page_when_frames_span_pages() {
    // idle: f0→page0, f1→page1.  walk: f0→page1, f1→page0.
    let rec = record(
        8,
        8,
        vec![
            row(
                "idle",
                0,
                0,
                vec![rect(0, 0, 8, 8, 0, (0, 0)), rect(0, 0, 8, 8, 1, (0, 0))],
            ),
            row(
                "walk",
                1,
                0,
                vec![rect(8, 0, 8, 8, 1, (0, 0)), rect(8, 0, 8, 8, 0, (0, 0))],
            ),
        ],
    );
    assert_eq!(rec.page_count(), 2);

    let n0 = rec.atlas_page(0, 0).rects.len();
    let n1 = rec.atlas_page(1, 0).rects.len();
    assert_eq!(n0, 2, "page 0: idle.f0 + walk.f1");
    assert_eq!(n1, 2, "page 1: idle.f1 + walk.f0");

    // (row, frame) -> (expected page). Indices must be unique within a page
    // and cover the layout.
    let cases = [(0usize, 0usize, 0u32), (0, 1, 1), (1, 0, 1), (1, 1, 0)];
    let mut seen: std::collections::HashMap<u32, Vec<usize>> = Default::default();
    for (ri, f, want_page) in cases {
        assert_eq!(rec.frame_page_of(ri, f), want_page, "row {ri} f{f}");
        let idx = rec.flat_index_in_page(ri, f);
        let len = if want_page == 0 { n0 } else { n1 };
        assert!(idx < len, "row {ri} f{f} index {idx} out of range {len}");
        seen.entry(want_page).or_default().push(idx);
    }
    for (page, mut idxs) in seen {
        idxs.sort();
        idxs.dedup();
        let len = if page == 0 { n0 } else { n1 };
        assert_eq!(idxs.len(), len, "page {page} indices unique + cover layout");
    }
}

/// A grid row (no rects) addresses cells off the frame stride, and the
/// single-page flat index is the global index.
#[test]
fn grid_fallback_addresses_stride_cells() {
    let mut rec = record(
        16,
        16,
        vec![row("idle", 0, 0, vec![]), row("walk", 1, 0, vec![])],
    );
    rec.rows[0].frame_count = 3;
    rec.rows[1].frame_count = 2;
    rec.label_width = 4;
    let page = rec.atlas_page(0, 0);
    assert_eq!(page.rects.len(), 5, "3 + 2 grid cells");
    // First idle cell starts at label_width.
    assert_eq!(page.rects[0].min, UVec2::new(4, 0));
    // walk row sits one frame_height down.
    assert_eq!(page.rects[3].min, UVec2::new(4, 16));
    assert_eq!(rec.flat_index_in_page(1, 0), 3, "walk.f0 is global index 3");
}

/// Trim round-trips: a trimmed frame reports its stored size + offset; an
/// untrimmed frame is the identity.
#[test]
fn frame_trim_reports_offset_and_size() {
    let rec = record(
        128,
        128,
        vec![row(
            "idle",
            0,
            0,
            vec![
                rect(2, 2, 80, 100, 0, (20, 14)),
                rect(90, 2, 128, 128, 0, (0, 0)),
            ],
        )],
    );
    assert!(rec.is_trimmed());
    let t0 = rec.frame_trim(0, 0);
    assert_eq!(t0.offset, IVec2::new(20, 14));
    assert_eq!(t0.trimmed, UVec2::new(80, 100));
    assert_eq!(t0.logical, UVec2::new(128, 128));
    assert!(!t0.is_identity());
    let t1 = rec.frame_trim(0, 1);
    assert!(t1.is_identity());
}

/// `trimmed_render` is the identity for an untrimmed frame and keeps a
/// logical point fixed in world space for a trimmed one.
#[test]
fn trimmed_render_identity_and_fixed_point() {
    let logical = UVec2::new(384, 529);
    let base_size = Vec2::new(120.0, 165.0);
    let base_anchor = Vec2::new(0.0, -0.3);
    let (s, a) = trimmed_render(&FrameTrim::identity(logical), base_size, base_anchor);
    assert!((s - base_size).length() < 1e-3);
    assert!((a - base_anchor).length() < 1e-4);

    // A trimmed sub-rect must map the logical-frame centre to the same world
    // position the full frame would.
    let (ox, oy, tw, th) = (100i32, 80i32, 180u32, 360u32);
    let trim = FrameTrim {
        offset: IVec2::new(ox, oy),
        trimmed: UVec2::new(tw, th),
        logical,
    };
    let (size, anchor) = trimmed_render(&trim, base_size, base_anchor);
    let world = |sub: (f32, f32, f32, f32), size: Vec2, anchor: Vec2, px: f32, py: f32| {
        let (sox, soy, stw, sth) = sub;
        let nx = (px - sox) / stw - 0.5;
        let ny = 0.5 - (py - soy) / sth;
        -anchor * size + Vec2::new(nx * size.x, ny * size.y)
    };
    let (lw, lh) = (logical.x as f32, logical.y as f32);
    let (px, py) = (ox as f32 + tw as f32 / 2.0, oy as f32 + th as f32 / 2.0);
    let full = world((0.0, 0.0, lw, lh), base_size, base_anchor, px, py);
    let trimmed = world(
        (ox as f32, oy as f32, tw as f32, th as f32),
        size,
        anchor,
        px,
        py,
    );
    assert!(
        (full - trimmed).length() < 1e-2,
        "full={full:?} trimmed={trimmed:?}"
    );
}
