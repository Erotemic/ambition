//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_portal::{PortalChannel, PortalChannelColor};

/// The c136/c137 shape: a thin-wall doorway pair, opposed faces 32px
/// apart (material midpoint x = 516).
fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
    let left = PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(500.0, 300.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    let right = PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(532.0, 300.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    (left, right)
}

fn viewer_at(x: f32) -> PortalViewer {
    PortalViewer {
        present: true,
        eye: Vec2::new(x, 300.0),
        half_size: Vec2::new(12.0, 20.0),
        occluders: Vec::new(),
    }
}

/// Exactly one of a pair's two panes is dominant, and the handoff sits at
/// the material midpoint.
#[test]
fn pane_dominance_is_antisymmetric_with_midpoint_zero() {
    let (left, right) = thin_wall_pair();
    for x in [460.0, 505.0, 516.0, 525.0, 570.0] {
        let eye = Vec2::new(x, 300.0);
        let a = pane_dominance(&left, &right, eye);
        let b = pane_dominance(&right, &left, eye);
        assert!(
            (a + b).abs() < 1e-4,
            "dominance must be antisymmetric at x={x}: {a} vs {b}"
        );
    }
    assert!(
        pane_dominance(&left, &right, Vec2::new(516.0, 300.0)).abs() < 1e-4,
        "the handoff point is the material midpoint"
    );
    assert!(pane_dominance(&left, &right, Vec2::new(460.0, 300.0)) > 0.0);
    assert!(pane_dominance(&right, &left, Vec2::new(570.0, 300.0)) > 0.0);
}

/// Walking decisively across the doorway hands the top pane over exactly
/// ONCE — the old radial-distance bias was near-tied through the whole
/// seam and could alternate the opaque panes per frame (the c136/c137
/// crossing flicker).
#[test]
fn thin_wall_pane_winner_flips_exactly_once_walking_across() {
    let (left, right) = thin_wall_pair();
    let config = PortalViewConeConfig::default();
    let mut left_dom: Option<bool> = None;
    let mut right_dom: Option<bool> = None;
    let mut flips = 0usize;
    let mut prev_top: Option<bool> = None;
    let mut x = 460.0;
    while x <= 570.0 {
        let v = viewer_at(x);
        let (zl, dl) = pane_z(&config, Some(&v), &left, &right, left_dom);
        let (zr, dr) = pane_z(&config, Some(&v), &right, &left, right_dom);
        left_dom = Some(dl);
        right_dom = Some(dr);
        assert!(
            (zl - zr).abs() > 1e-4,
            "the two panes must never tie in z (x={x}): {zl} vs {zr}"
        );
        let left_on_top = zl > zr;
        if let Some(prev) = prev_top {
            if prev != left_on_top {
                flips += 1;
                assert!(
                    (505.0..=527.0).contains(&x),
                    "the pane handoff belongs near the material midpoint, got x={x}"
                );
            }
        }
        prev_top = Some(left_on_top);
        x += 1.0;
    }
    assert_eq!(flips, 1, "one crossing, one pane handoff");
}

/// Sub-pixel eye jitter around the midpoint must not alternate the panes:
/// within the hysteresis band the previous winner is kept (standing
/// still inside the doorway — Jon's screenshot — stays stable).
#[test]
fn midpoint_jitter_keeps_the_previous_pane_winner() {
    let (left, right) = thin_wall_pair();
    let config = PortalViewConeConfig::default();
    // Approach decisively from the left, then jitter ±1.5px at the seam.
    let mut left_dom: Option<bool> = None;
    let mut right_dom: Option<bool> = None;
    for x in [480.0, 500.0, 514.0] {
        let v = viewer_at(x);
        left_dom = Some(pane_z(&config, Some(&v), &left, &right, left_dom).1);
        right_dom = Some(pane_z(&config, Some(&v), &right, &left, right_dom).1);
    }
    for i in 0..20 {
        let x = 516.0 + if i % 2 == 0 { -1.5 } else { 1.5 };
        let v = viewer_at(x);
        let (zl, dl) = pane_z(&config, Some(&v), &left, &right, left_dom);
        let (zr, dr) = pane_z(&config, Some(&v), &right, &left, right_dom);
        left_dom = Some(dl);
        right_dom = Some(dr);
        assert!(
            zl > zr,
            "jitter inside the hysteresis band must keep the left pane on top (i={i})"
        );
    }
}

/// Same-plane pairs have identically zero dominance; the proximity ramp
/// still orders them (nearer window on top), as before.
#[test]
fn same_plane_pair_still_orders_by_proximity() {
    let up = Vec2::new(0.0, -1.0);
    let a = PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(254.0, 880.0),
        up,
        Vec2::new(46.0, 9.0),
    );
    let b = PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(554.0, 880.0),
        up,
        Vec2::new(46.0, 9.0),
    );
    let eye = Vec2::new(300.0, 840.0);
    assert!(
        pane_dominance(&a, &b, eye).abs() < 1e-4,
        "same-plane fronts coincide"
    );
    let config = PortalViewConeConfig::default();
    let v = PortalViewer {
        present: true,
        eye,
        half_size: Vec2::new(12.0, 20.0),
        occluders: Vec::new(),
    };
    let (za, _) = pane_z(&config, Some(&v), &a, &b, None);
    let (zb, _) = pane_z(&config, Some(&v), &b, &a, None);
    assert!(za > zb, "the nearer same-plane window draws on top");
}
