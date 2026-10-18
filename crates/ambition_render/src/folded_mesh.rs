//! The one road for a mesh built on the CPU: lightning, lasers, any line or
//! ribbon an effect lays out as triangles. A producer writes its triangles
//! into a [`FoldedMesh`], in the world, as if no portal were there; this
//! module writes the mesh that is drawn, and it is the only writer.
//!
//! So such a mesh works with portals by construction. Each frame its
//! triangles are cut where the room's portal pairs cut a volume
//! (`PortalPassages::pieces`, the resolver's own rule for a strike): the part
//! that reaches through a pair from its front is drawn out of the other end.
//! The part behind a pane from the viewer (the far side of a portal's wall)
//! is not drawn inside the pane, which shows the view through it instead
//! (`pane_relation`, the far-side compositor's rule for a sprite). A mesh
//! with no pair near it is drawn as it was built, at no cost.
//!
//! Its material only composites the colours its triangles carry: added
//! (light) or blended. No shader draws a shape.

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    math::Vec2,
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_core::frame::thread::{HalfPlane, VolumePiece};
#[cfg(feature = "portal_render")]
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomOf, LiveRooms};
use ambition_platformer2d_shared_tangle::portal_passages::PortalPassages;

/// Triangles with a colour at each corner (linear RGBA), world space (+y
/// down).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tris {
    pub positions: Vec<Vec2>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl Tris {
    pub fn vertex(&mut self, at: Vec2, color: [f32; 4]) -> u32 {
        self.positions.push(at);
        self.colors.push(color);
        (self.positions.len() - 1) as u32
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// The box round every corner, or `None` for no triangles.
    pub fn bounds(&self) -> Option<ae::Aabb> {
        let mut used = self.indices.iter().map(|&i| self.positions[i as usize]);
        let first = used.next()?;
        let (lo, hi) = used.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)));
        Some(ae::Aabb { min: lo, max: hi })
    }

    fn triangles(&self) -> impl Iterator<Item = [(Vec2, [f32; 4]); 3]> + '_ {
        self.indices.chunks_exact(3).map(|t| {
            let corner = |i: u32| (self.positions[i as usize], self.colors[i as usize]);
            [corner(t[0]), corner(t[1]), corner(t[2])]
        })
    }

    /// Add the convex polygon `poly` as a fan.
    fn polygon(&mut self, poly: &[(Vec2, [f32; 4])]) {
        if poly.len() < 3 {
            return;
        }
        let first = self.positions.len() as u32;
        for &(at, color) in poly {
            self.vertex(at, color);
        }
        for k in 1..poly.len() as u32 - 1 {
            self.indices.extend_from_slice(&[first, first + k, first + k + 1]);
        }
    }
}

/// How a [`FoldedMesh`]'s colours meet what is under them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Composite {
    /// Added: light. Where the device has no material for it, blended.
    #[default]
    Add,
    Blend,
}

/// A mesh built on the CPU, drawn through the portals of its room.
///
/// `tris` are in the world, unfolded. `root` is where the geometry comes
/// from (a ray's start, a body's middle): a pair cuts it only from the
/// pair's front, as seen from there. `carrier` is the body it is carried
/// with, if any: the pair that body straddles cuts it first.
#[derive(Component, Clone, Debug, Default)]
pub struct FoldedMesh {
    pub tris: Tris,
    pub root: Vec2,
    pub carrier: Option<Entity>,
    pub z: f32,
    pub composite: Composite,
}

/// The material of an added mesh: its colours, added to what is under them.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct AddedLightMaterial {
    /// Multiplies every colour.
    #[uniform(0)]
    pub strength: Vec4,
}

impl Material2d for AddedLightMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/folded_mesh.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let add = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(add);
            }
        }
        Ok(())
    }
}

#[derive(Resource)]
struct FoldMaterials {
    added: Option<Handle<AddedLightMaterial>>,
    blended: Handle<ColorMaterial>,
}

/// On a [`FoldedMesh`] that has been drawn: the mesh it writes.
#[derive(Component)]
struct FoldedMeshDrawn(Handle<Mesh>);

/// The set the writer runs in: a producer of a [`FoldedMesh`] runs before it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FoldedMeshSync;

/// Installs the writer. A host with no renderer installs nothing.
pub struct FoldedMeshPlugin;

impl Plugin for FoldedMeshPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() {
            return;
        }
        #[cfg(not(target_os = "android"))]
        let added = {
            bevy::asset::embedded_asset!(app, "folded_mesh.wgsl");
            app.add_plugins(Material2dPlugin::<AddedLightMaterial>::default());
            app.world_mut()
                .get_resource_mut::<Assets<AddedLightMaterial>>()
                .map(|mut materials| materials.add(AddedLightMaterial { strength: Vec4::ONE }))
        };
        #[cfg(target_os = "android")]
        let added = None;
        let Some(blended) = app
            .world_mut()
            .get_resource_mut::<Assets<ColorMaterial>>()
            .map(|mut materials| materials.add(ColorMaterial::from_color(Color::WHITE)))
        else {
            return;
        };
        app.insert_resource(FoldMaterials { added, blended });
        use ambition_platformer2d_shared_tangle::lifecycle::session_world_exists;
        app.configure_sets(
            Update,
            FoldedMeshSync
                // After the body-owned drawables: a mesh that rides a body is
                // folded where the body is drawn this frame.
                .after(crate::rendering::BodyOwnedDrawableSync)
                .run_if(session_world_exists),
        );
        app.add_systems(Update, write_folded_meshes.in_set(FoldedMeshSync));
    }
}

/// `poly` (a convex polygon) on the inward side of `plane`.
fn clip(poly: &[(Vec2, [f32; 4])], plane: &HalfPlane) -> Vec<(Vec2, [f32; 4])> {
    let side = |p: Vec2| (p - plane.point).dot(plane.inward);
    let mut out = Vec::with_capacity(poly.len() + 2);
    for (i, &(a, ca)) in poly.iter().enumerate() {
        let (b, cb) = poly[(i + 1) % poly.len()];
        let (sa, sb) = (side(a), side(b));
        if sa >= 0.0 {
            out.push((a, ca));
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            let t = sa / (sa - sb);
            let mix = std::array::from_fn(|k| ca[k] + (cb[k] - ca[k]) * t);
            out.push((a.lerp(b, t), mix));
        }
    }
    out
}

fn clip_all(mut poly: Vec<(Vec2, [f32; 4])>, planes: &[HalfPlane]) -> Vec<(Vec2, [f32; 4])> {
    for plane in planes {
        if poly.len() < 3 {
            break;
        }
        poly = clip(&poly, plane);
    }
    poly
}

/// `tris` where the portal pieces put them: each piece's part of each
/// triangle, carried into the piece's chart. One whole piece is `tris` as
/// built.
pub fn fold(tris: &Tris, pieces: &[VolumePiece]) -> Tris {
    if let [piece] = pieces {
        if piece.bounds.is_empty() && !piece.is_carried() {
            return tris.clone();
        }
    }
    let mut out = Tris::default();
    for piece in pieces {
        for tri in tris.triangles() {
            let carried: Vec<_> = tri.iter().map(|&(p, c)| (piece.carry(p), c)).collect();
            out.polygon(&clip_all(carried, &piece.bounds));
        }
    }
    out
}

/// `tris` with the rectangle `min..max` cut out of each triangle whose
/// middle `covered` says is behind it: the pane draws there.
pub fn uncover(tris: &Tris, min: Vec2, max: Vec2, covered: impl Fn(Vec2) -> bool) -> Tris {
    let mut out = Tris::default();
    let outside = [
        // Left of it; right of it; above it and below it, between the two.
        vec![HalfPlane { point: min, inward: Vec2::NEG_X }],
        vec![HalfPlane { point: max, inward: Vec2::X }],
        vec![
            HalfPlane { point: min, inward: Vec2::X },
            HalfPlane { point: max, inward: Vec2::NEG_X },
            HalfPlane { point: min, inward: Vec2::NEG_Y },
        ],
        vec![
            HalfPlane { point: min, inward: Vec2::X },
            HalfPlane { point: max, inward: Vec2::NEG_X },
            HalfPlane { point: max, inward: Vec2::Y },
        ],
    ];
    for tri in tris.triangles() {
        let lo = tri.iter().fold(Vec2::splat(f32::INFINITY), |m, (p, _)| m.min(*p));
        let hi = tri.iter().fold(Vec2::splat(f32::NEG_INFINITY), |m, (p, _)| m.max(*p));
        let middle = (tri[0].0 + tri[1].0 + tri[2].0) / 3.0;
        let overlaps = lo.x <= max.x && hi.x >= min.x && lo.y <= max.y && hi.y >= min.y;
        if !overlaps || !covered(middle) {
            out.polygon(&tri);
            continue;
        }
        for planes in &outside {
            out.polygon(&clip_all(tri.to_vec(), planes));
        }
    }
    out
}

/// A drawable mesh of `tris` for the render world. Nothing to draw is one
/// invisible triangle, so the mesh is never empty.
fn mesh_of(tris: &Tris) -> Mesh {
    let (positions, colors, indices) = if tris.is_empty() {
        (vec![[0.0f32; 3]; 3], vec![[0.0f32; 4]; 3], vec![0, 1, 2])
    } else {
        // Mesh space: world, y up (the room's offset is the entity's).
        let positions = tris.positions.iter().map(|p| [p.x, -p.y, 0.0]).collect();
        (positions, tris.colors.clone(), tris.indices.clone())
    };
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// The far side: what of a mesh in `room` is behind a pane from its eye.
#[cfg(feature = "portal_render")]
#[derive(bevy::ecs::system::SystemParam)]
struct Panes<'w, 's> {
    frames: ambition_portal2d_presentation::PortalFrames<'w, 's>,
    portals: Query<'w, 's, (Entity, &'static ambition_portal2d_presentation::PlacedPortal)>,
    viewers: Option<Res<'w, ambition_portal2d_presentation::PortalViewers>>,
}

#[cfg(feature = "portal_render")]
impl Panes<'_, '_> {
    fn uncover(&self, tris: Tris, room: Option<LiveRoomInstance>) -> Tris {
        let Some(viewer) = self.viewers.as_deref().and_then(|v| v.in_room(room)).filter(|v| v.present) else {
            return tris;
        };
        let Some(bounds) = tris.bounds() else {
            return tris;
        };
        let by_room = self.frames.portals_by_room(self.portals.iter());
        let mut tris = tris;
        for pane in by_room.in_room(room) {
            // A pane that covers some of it from the eye; each triangle is
            // then judged by its own middle, as a sprite is by its centre.
            let relation = ambition_portal2d_presentation::pane_relation(pane, viewer.eye, bounds.min, bounds.max, false);
            if relation == ambition_portal2d_presentation::PaneRelation::Disjoint {
                continue;
            }
            let (min, max) = ambition_portal2d_presentation::pane_cover_rect(pane);
            tris = uncover(&tris, min, max, |middle| ambition_portal2d_presentation::behind_pane(pane, viewer.eye, middle));
        }
        tris
    }
}

/// Fold each changed [`FoldedMesh`] through its room's portals and write the
/// mesh it is drawn with.
#[allow(clippy::too_many_arguments)]
fn write_folded_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<FoldMaterials>,
    world: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    rooms: LiveRooms,
    passages: Option<Res<PortalPassages>>,
    #[cfg(feature = "portal_render")] panes: Panes,
    mut folded: Query<(Entity, &FoldedMesh, Option<&FoldedMeshDrawn>, &mut Transform), Changed<FoldedMesh>>,
) {
    for (entity, mesh, drawn, mut transform) in &mut folded {
        let Some(room_world) = world.of(entity) else {
            continue;
        };
        let room = rooms.of(entity);
        let tris = match (passages.as_deref(), mesh.tris.bounds()) {
            (Some(passages), Some(bounds)) if passages.cuts_in(room, mesh.carrier) => {
                let pieces = passages.pieces(&ae::CombatVolume::aabb(bounds), mesh.carrier, mesh.root, room);
                fold(&mesh.tris, &pieces)
            }
            _ => mesh.tris.clone(),
        };
        #[cfg(feature = "portal_render")]
        let tris = panes.uncover(tris, room);
        transform.translation = world_to_bevy(&room_world.0, Vec2::ZERO, mesh.z);
        match drawn {
            Some(FoldedMeshDrawn(handle)) => {
                if let Some(mut target) = meshes.get_mut(handle) {
                    *target = mesh_of(&tris);
                }
            }
            None => {
                let handle = meshes.add(mesh_of(&tris));
                let mut entity = commands.entity(entity);
                entity.insert((Mesh2d(handle.clone()), NoFrustumCulling, FoldedMeshDrawn(handle)));
                match (&materials.added, mesh.composite) {
                    (Some(added), Composite::Add) => entity.insert(MeshMaterial2d(added.clone())),
                    _ => entity.insert(MeshMaterial2d(materials.blended.clone())),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::frame::thread::{partition, PieceBudget, PortalLink};
    use ambition_platformer2d_core::frame::{MapConvention, PortalAperture, PortalFrame};

    fn quad(min: Vec2, max: Vec2) -> Tris {
        let mut tris = Tris::default();
        let c = [1.0; 4];
        let a = tris.vertex(min, c);
        let b = tris.vertex(Vec2::new(max.x, min.y), c);
        let d = tris.vertex(max, c);
        let e = tris.vertex(Vec2::new(min.x, max.y), c);
        tris.indices.extend_from_slice(&[a, b, d, a, d, e]);
        tris
    }

    fn area(tris: &Tris) -> f32 {
        tris.triangles()
            .map(|[(a, _), (b, _), (c, _)]| ((b - a).perp_dot(c - a) * 0.5).abs())
            .sum()
    }

    /// A wall portal at x 100 facing -x, its partner on a floor at (400, 300)
    /// facing up: a band from x 0 to 160 reaches 60 through it.
    fn pieces_of(tris: &Tris) -> Vec<VolumePiece> {
        let enter = PortalAperture { frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), Vec2::new(-1.0, 0.0)), half_length: 40.0 };
        let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(400.0, 300.0), Vec2::new(0.0, -1.0)), half_length: 40.0 };
        let links = PortalLink::both_ways(0, (enter, exit));
        let bounds = tris.bounds().unwrap();
        partition(&ae::CombatVolume::aabb(bounds), Vec2::ZERO, &links, None, PieceBudget::SINGLE_HOP, MapConvention::Reflection)
            .pieces
    }

    #[test]
    fn a_mesh_with_no_pair_near_it_is_drawn_as_built() {
        let band = quad(Vec2::new(0.0, -5.0), Vec2::new(60.0, 5.0));
        assert_eq!(fold(&band, &pieces_of(&band)), band);
    }

    /// ⭐ What reaches through a pair is drawn out of its other end, and
    /// nothing is lost or doubled.
    #[test]
    fn a_band_through_a_pair_comes_out_of_the_other_end() {
        let band = quad(Vec2::new(0.0, -5.0), Vec2::new(160.0, 5.0));
        let folded = fold(&band, &pieces_of(&band));
        assert!((area(&folded) - area(&band)).abs() < 1.0, "{} vs {}", area(&folded), area(&band));
        assert!(folded.positions.iter().all(|p| p.x <= 100.0 + 1e-3 || p.y <= 300.0 + 1e-3), "nothing is left behind the wall");
        let out: Vec<Vec2> = folded.positions.iter().copied().filter(|p| p.x > 300.0).collect();
        assert!(!out.is_empty(), "some of it is out of the floor portal");
        assert!(out.iter().all(|p| (p.x - 400.0).abs() <= 6.0 && p.y >= 239.0), "60 up out of the exit, 10 wide: {out:?}");
    }

    #[test]
    fn a_pane_covers_only_what_is_behind_it() {
        let band = quad(Vec2::new(0.0, 0.0), Vec2::new(100.0, 10.0));
        let min = Vec2::new(40.0, -20.0);
        let max = Vec2::new(60.0, 20.0);
        let none = uncover(&band, min, max, |_| false);
        assert!((area(&none) - 1000.0).abs() < 1e-3);
        let cut = uncover(&band, min, max, |_| true);
        assert!((area(&cut) - 800.0).abs() < 1e-2, "the 20 wide strip under the pane is gone: {}", area(&cut));
        assert!(cut.positions.iter().all(|p| p.x <= 40.0 + 1e-3 || p.x >= 60.0 - 1e-3));
    }
}
