//! The one road for a mesh built on the CPU: lightning, lasers, any line or
//! ribbon an effect lays out as triangles. A producer writes its triangles
//! into a [`FoldedMesh`], in the world, as if no portal were there; this
//! module writes the mesh that is drawn, and it is the only writer.
//!
//! So such a mesh works with portals by construction. Each frame its
//! triangles are cut where the room's portal pairs cut a volume
//! (`BodyGeometry::pieces`, the resolver's own rule for a strike): the part
//! that reaches through a pair from its front is drawn out of the other end.
//! The part behind a portal's face is under its window, which draws over the
//! play plane (`PORTAL_WINDOW_Z`). A mesh with no pair near it is drawn as it
//! was built, at no cost.
//!
//! Its material only composites the colours its triangles carry: added
//! (light) or blended. No shader draws a shape.

use ambition_platformer2d_shared_tangle::body::geometry::PartSource;
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::{NoFrustumCulling, RenderLayers, VisibilitySystems},
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
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomOf, LiveRooms};
use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;

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
    pub(crate) fn polygon(&mut self, poly: &[(Vec2, [f32; 4])]) {
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
/// pair's front, as seen from there. `source` selects the material chart,
/// active transit and traversal policy.
#[derive(Component, Clone, Debug, Default)]
pub struct FoldedMesh {
    pub tris: Tris,
    pub root: Vec2,
    pub source: PartSource,
    pub z: f32,
    pub composite: Composite,
}

/// The material of an added mesh: its colours, added to what is under them.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct AddedLightMaterial {
    /// Multiplies every colour.
    #[uniform(0)]
    pub strength: Vec4,
}

impl AddedLightMaterial {
    pub(crate) const BLEND: BlendState = BlendState {
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
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(Self::BLEND);
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

/// Prepared geometry and material for a planar draw.
#[derive(Component)]
pub(crate) struct PreparedPlanarMesh<M: Asset> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<M>,
}

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
            .map(|mut materials| {
                materials.add(ColorMaterial {
                    color: Color::WHITE,
                    alpha_mode: AlphaMode2d::Blend,
                    ..default()
                })
            })
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
        app.add_systems(Update, prepare_folded_meshes.in_set(FoldedMeshSync));
        app.add_systems(
            PostUpdate,
            draw_folded_meshes_2d
                .run_if(any_with_component::<FoldedMesh>)
                .after(bevy::camera::CameraUpdateSystems)
                .after(crate::rendering::view_isolation::isolate_live_rooms)
                .before(VisibilitySystems::CheckVisibility),
        );
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
pub fn fold(tris: &Tris, pieces: &[VolumePiece]) -> Result<Tris, ae::frame::NumericFault> {
    if let [piece] = pieces {
        if piece.source_bounds().is_empty() && !piece.is_carried() {
            return Ok(tris.clone());
        }
    }
    let mut out = Tris::default();
    for piece in pieces {
        let root_bounds = piece.root_bounds()?;
        for tri in tris.triangles() {
            let clipped = clip_all(tri.to_vec(), &root_bounds);
            let carried = clipped.into_iter().map(|(point, color)| piece.carry(point).map(|point| (point, color)))
                .collect::<Result<Vec<_>, _>>()?;
            out.polygon(&carried);
        }
    }
    Ok(out)
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

/// Prepare one mesh for all outputs that draw the effect.
#[allow(clippy::too_many_arguments)]
fn prepare_folded_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<FoldMaterials>,
    world: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    rooms: LiveRooms,
    passages: Option<Res<BodyGeometry>>,
    carriers: Query<&ae::BodyKinematics>,
    mut folded: Query<(
        Entity, &FoldedMesh, Option<&PreparedPlanarMesh<AddedLightMaterial>>,
        Option<&PreparedPlanarMesh<ColorMaterial>>, &mut Transform,
    ), Changed<FoldedMesh>>,
) {
    for (entity, mesh, added, blended, mut transform) in &mut folded {
        let Some(room_world) = world.of(entity) else {
            continue;
        };
        let room = rooms.of(entity);
        let cuts = passages.as_deref().filter(|geometry| geometry.cuts_in(room, mesh.source.carrier()));
        let folded_tris;
        let tris = match cuts.and_then(|geometry| Some((geometry, mesh.tris.bounds()?))) {
            Some((passages, bounds)) => {
                let volume = ae::CombatVolume::aabb(bounds);
                let pieces = match mesh.source {
                    PartSource::World { owner } => passages.world_pieces(&volume, owner, mesh.root, room),
                    PartSource::Body(body) => {
                        let Ok(carrier) = carriers.get(body) else { continue; };
                        let Ok(material) = carrier.material_map() else { continue; };
                        passages.body_world_pieces(&volume, body, material, mesh.root, room)
                    }
                };
                let Ok(pieces) = pieces else { continue; };
                let Ok(tris) = fold(&mesh.tris, &pieces) else { continue; };
                folded_tris = tris;
                &folded_tris
            }
            None => &mesh.tris,
        };
        let translation = world_to_bevy(&room_world.0, Vec2::ZERO, mesh.z);
        if transform.translation != translation {
            transform.translation = translation;
        }
        let previous = added.map(|draw| &draw.mesh).or_else(|| blended.map(|draw| &draw.mesh));
        let handle = match previous {
            Some(handle) if meshes.contains(handle.id()) => {
                *meshes.get_mut(handle).unwrap() = mesh_of(tris);
                handle.clone()
            }
            _ => meshes.add(mesh_of(tris)),
        };
        match (&materials.added, mesh.composite) {
            (Some(material), Composite::Add) => {
                if added.is_none_or(|draw| draw.mesh != handle || draw.material != *material) {
                    commands.entity(entity)
                        .remove::<PreparedPlanarMesh<ColorMaterial>>()
                        .insert((PreparedPlanarMesh { mesh: handle, material: material.clone() }, NoFrustumCulling));
                }
            }
            _ => {
                if blended.is_none_or(|draw| draw.mesh != handle || draw.material != materials.blended) {
                    commands.entity(entity)
                        .remove::<PreparedPlanarMesh<AddedLightMaterial>>()
                        .insert((PreparedPlanarMesh { mesh: handle, material: materials.blended.clone() }, NoFrustumCulling));
                }
            }
        }
    }
}

/// Bind a mesh when a 2D camera uses its layers. Portal captures can use the
/// mesh while the main output draws in 2.5D.
fn draw_folded_meshes_2d(
    mut commands: Commands,
    cameras: Query<(&Camera, Option<&RenderLayers>), With<Camera2d>>,
    effects: Query<(
        Entity, Option<&RenderLayers>, Option<&Mesh2d>,
        Option<&PreparedPlanarMesh<AddedLightMaterial>>, Option<&PreparedPlanarMesh<ColorMaterial>>,
        Option<&MeshMaterial2d<AddedLightMaterial>>, Option<&MeshMaterial2d<ColorMaterial>>,
    ), With<FoldedMesh>>,
) {
    let masks: Vec<_> = cameras.iter().filter_map(|(camera, mask)| {
        camera.is_active.then(|| mask.unwrap_or_default())
    }).collect();
    for (id, layers, drawn, added, blended, light, color) in &effects {
        let wanted = masks.iter().any(|mask| mask.intersects(layers.unwrap_or_default()));
        if !wanted {
            if drawn.is_some() {
                commands.entity(id).remove::<(Mesh2d, MeshMaterial2d<AddedLightMaterial>, MeshMaterial2d<ColorMaterial>)>();
            }
        } else if let Some(draw) = added {
            if drawn.is_none_or(|mesh| mesh.0 != draw.mesh) || light.is_none_or(|material| material.0 != draw.material) {
                commands.entity(id)
                    .remove::<MeshMaterial2d<ColorMaterial>>()
                    .insert((Mesh2d(draw.mesh.clone()), MeshMaterial2d(draw.material.clone())));
            }
        } else if let Some(draw) = blended {
            if drawn.is_none_or(|mesh| mesh.0 != draw.mesh) || color.is_none_or(|material| material.0 != draw.material) {
                commands.entity(id)
                    .remove::<MeshMaterial2d<AddedLightMaterial>>()
                    .insert((Mesh2d(draw.mesh.clone()), MeshMaterial2d(draw.material.clone())));
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

    #[test]
    fn folded_geometry_survives_the_2d_capture_binding() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        let world = app.world_mut();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            world,
            ae::RoomGeometry(ae::World::new(
                "effect room", Vec2::splat(256.0), Vec2::ZERO, Vec::new(),
            )),
        );
        world.insert_resource(FoldMaterials {
            added: Some(Handle::default()),
            blended: Handle::default(),
        });
        let layer = RenderLayers::layer(7);
        let effect = world.spawn((
            FoldedMesh {
                tris: quad(Vec2::ZERO, Vec2::splat(36.0)),
                composite: Composite::Blend,
                ..default()
            },
            Transform::default(),
            layer.clone(),
        )).id();
        world.spawn((Camera2d, RenderLayers::layer(8)));
        app.add_systems(Update, (prepare_folded_meshes, draw_folded_meshes_2d).chain());

        app.update();
        let mesh = app.world().get::<PreparedPlanarMesh<ColorMaterial>>(effect).unwrap().mesh.clone();
        assert!(app.world().get::<Mesh2d>(effect).is_none());

        let capture = app.world_mut().spawn((Camera2d, layer)).id();
        app.update();
        assert_eq!(app.world().get::<Mesh2d>(effect).unwrap().0, mesh);
        assert!(app.world().get::<MeshMaterial2d<ColorMaterial>>(effect).is_some());

        app.world_mut().get_mut::<FoldedMesh>(effect).unwrap().composite = Composite::Add;
        app.update();
        assert_eq!(app.world().get::<Mesh2d>(effect).unwrap().0, mesh);
        assert!(app.world().get::<MeshMaterial2d<ColorMaterial>>(effect).is_none());
        assert!(app.world().get::<MeshMaterial2d<AddedLightMaterial>>(effect).is_some());

        app.world_mut().get_mut::<Camera>(capture).unwrap().is_active = false;
        app.update();
        assert!(app.world().get::<Mesh2d>(effect).is_none());
        assert_eq!(app.world().get::<PreparedPlanarMesh<AddedLightMaterial>>(effect).unwrap().mesh, mesh);
    }

    /// A wall portal at x 100 facing -x, its partner on a floor at (400, 300)
    /// facing up: a band from x 0 to 160 reaches 60 through it.
    fn pieces_of(tris: &Tris) -> Vec<VolumePiece> {
        let enter = PortalAperture { frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), Vec2::new(-1.0, 0.0)), half_length: 40.0 };
        let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(400.0, 300.0), Vec2::new(0.0, -1.0)), half_length: 40.0 };
        let links = PortalLink::both_ways(0, (enter, exit));
        let bounds = tris.bounds().unwrap();
        partition(ambition_platformer2d_core::frame::thread::MaterialPrimitive::world(&ae::CombatVolume::aabb(bounds)), Vec2::ZERO, &links, None, PieceBudget::SINGLE_HOP, MapConvention::Reflection)
            .unwrap()
    }

    #[test]
    fn a_mesh_with_no_pair_near_it_is_drawn_as_built() {
        let band = quad(Vec2::new(0.0, -5.0), Vec2::new(60.0, 5.0));
        assert_eq!(fold(&band, &pieces_of(&band)).unwrap(), band);
    }

    /// ⭐ What reaches through a pair is drawn out of its other end, and
    /// nothing is lost or doubled.
    #[test]
    fn a_band_through_a_pair_comes_out_of_the_other_end() {
        let band = quad(Vec2::new(0.0, -5.0), Vec2::new(160.0, 5.0));
        let folded = fold(&band, &pieces_of(&band)).unwrap();
        assert!((area(&folded) - area(&band)).abs() < 1.0, "{} vs {}", area(&folded), area(&band));
        assert!(folded.positions.iter().all(|p| p.x <= 100.0 + 1e-3 || p.y <= 300.0 + 1e-3), "nothing is left behind the wall");
        let out: Vec<Vec2> = folded.positions.iter().copied().filter(|p| p.x > 300.0).collect();
        assert!(!out.is_empty(), "some of it is out of the floor portal");
        assert!(out.iter().all(|p| (p.x - 400.0).abs() <= 6.0 && p.y >= 239.0), "60 up out of the exit, 10 wide: {out:?}");
    }

}
