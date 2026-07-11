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
mod tests;
