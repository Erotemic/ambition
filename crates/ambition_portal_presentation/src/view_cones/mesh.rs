//! Cone render-mesh construction (triangulation + UVs) and the proximity-fade
//! ramp helpers the sync system applies.
//!
//! Split out of the former 1098-line `view_cones.rs` (2026-06-15).

use super::*;

pub(crate) fn make_mesh(render: &ConeRender) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    apply_mesh(&mut mesh, render);
    mesh
}

pub(crate) fn apply_mesh(mesh: &mut Mesh, render: &ConeRender) {
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, render.uvs.clone());
    mesh.insert_indices(Indices::U32(render.indices.clone()));
}

/// A hidden-rig placeholder mesh (degenerate; the rig is invisible until its
/// first visible frame fills it in).
pub(crate) fn placeholder_mesh() -> Mesh {
    make_mesh(&ConeRender {
        positions: vec![[0.0; 3]; 3],
        uvs: vec![[0.0; 2]; 3],
        indices: vec![0, 1, 2],
        centroid: Vec3::ZERO,
        cam_center: Vec3::ZERO,
        entry_poly_world: vec![Vec2::ZERO; 3],
        mapped_source_vertices: vec![Vec2::ZERO; 3],
        source_min: Vec2::ZERO,
        source_max: Vec2::ONE,
        source_size: Vec2::ONE,
    })
}

/// Smoothstep shaping for the temporal blend — the "squished logit" feel:
/// flat near both ends, fast through the middle.
pub(crate) fn smooth01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Pairwise pane-dominance score: positive when the viewer is on THIS
/// portal's side of the pair's shared material — the signed front distance to
/// this face minus the partner's. Antisymmetric (exactly one of a pair's two
/// windows scores positive) and zero exactly at the material midpoint of an
/// opposed-face thin-wall pair; identically zero for a same-plane pair, whose
/// fronts coincide (those fall through to the proximity ramp).
pub(crate) fn pane_dominance(portal: &PlacedPortal, partner: &PlacedPortal, eye: Vec2) -> f32 {
    (eye - portal.pos).dot(portal.normal.normalize_or_zero())
        - (eye - partner.pos).dot(partner.normal.normalize_or_zero())
}

/// Hysteresis band (world px) around the dominance midpoint: within it the
/// previous winner is kept, so sub-pixel eye jitter while standing in a
/// thin-wall seam cannot alternate two overlapping opaque panes
/// frame-to-frame (the c136/c137 crossing/standing flicker). Crossing
/// decisively still hands the pane over exactly once, at the material
/// midpoint — the same place the `window_eye` handoff crossfades.
pub(crate) const PANE_DOMINANCE_BAND: f32 = 6.0;

/// Render z for a portal's window and the sticky dominance state to carry on
/// the rig. Two terms inside the declared `z_proximity_span` band:
///
/// - a PAIRWISE winner bonus (60%): between a pair's own two overlapping
///   panes, the one whose face the viewer is in front of draws on top —
///   decided by [`pane_dominance`] with [`PANE_DOMINANCE_BAND`] hysteresis,
///   NOT by radial distance, which is near-tied everywhere around a thin-wall
///   seam and flipped the opaque panes per frame;
/// - the proximity ramp (40%): across DIFFERENT pairs (and within same-plane
///   pairs, where dominance is identically zero) the nearer window still
///   draws over farther ones, as before.
///
/// No viewer ⇒ base z, previous winner kept.
pub(crate) fn pane_z(
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
    portal: &PlacedPortal,
    partner: &PlacedPortal,
    prev_dominant: Option<bool>,
) -> (f32, bool) {
    let Some(v) = viewer.filter(|v| v.present) else {
        return (config.z, prev_dominant.unwrap_or(false));
    };
    let score = pane_dominance(portal, partner, v.eye);
    let dominant = if score.abs() < PANE_DOMINANCE_BAND {
        prev_dominant.unwrap_or(score >= 0.0)
    } else {
        score > 0.0
    };
    let prox = 1.0 / (1.0 + v.eye.distance(portal.pos) / 200.0);
    let winner = if dominant { 1.0 } else { 0.0 };
    let z = config.z + config.z_proximity_span * (0.4 * prox + 0.6 * winner);
    (z, dominant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal::{PortalChannel, PortalChannelColor};

    /// The c136/c137 shape: a thin-wall doorway pair, opposed faces 32px
    /// apart (material midpoint x = 516).
    fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
        let left = PlacedPortal {
            channel: PortalChannel::Authored(PortalChannelColor::Purple),
            pos: Vec2::new(500.0, 300.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        let right = PlacedPortal {
            channel: PortalChannel::Authored(PortalChannelColor::Yellow),
            pos: Vec2::new(532.0, 300.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
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
        let a = PlacedPortal {
            channel: PortalChannel::Authored(PortalChannelColor::Purple),
            pos: Vec2::new(254.0, 880.0),
            normal: up,
            half_extent: Vec2::new(46.0, 9.0),
        };
        let b = PlacedPortal {
            channel: PortalChannel::Authored(PortalChannelColor::Yellow),
            pos: Vec2::new(554.0, 880.0),
            normal: up,
            half_extent: Vec2::new(46.0, 9.0),
        };
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
}
