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

/// Render z for a portal's window biased by viewer proximity, so the portal
/// you are closest to draws ON TOP of the others (inverse-distance, bounded by
/// `span` and kept under the rim gap). No viewer ⇒ base z.
pub(crate) fn proximity_z(
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
    portal_pos: Vec2,
) -> f32 {
    let dist = viewer
        .filter(|v| v.present)
        .map_or(f32::INFINITY, |v| v.eye.distance(portal_pos));
    config.z + config.z_proximity_span / (1.0 + dist / 200.0)
}
