//! Textured portal pieces with complete mesh clipping.

use bevy::asset::{embedded_asset, RenderAssetUsages};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::PortalWorldFrame;

/// Texture and colour for a clipped material region.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct PortalClipMaterial {
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// Horizontal flip, vertical flip, silhouette and premultiplied alpha.
    #[uniform(1)]
    pub control: Vec4,
    #[uniform(2)]
    pub tint: Vec4,
    #[texture(6)]
    #[sampler(7)]
    pub color_texture: Handle<Image>,
}

impl PortalClipMaterial {
    pub fn new(uv_rect: Vec4, control: Vec4, tint: Vec4, color_texture: Handle<Image>) -> Self {
        Self { uv_rect, control, tint, color_texture }
    }

    fn same_as(&self, other: &Self) -> bool {
        self.uv_rect == other.uv_rect && self.control == other.control
            && self.tint == other.tint && self.color_texture == other.color_texture
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PieceMesh {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl PieceMesh {
    /// Clip in double precision. UVs remain in the source rectangle.
    fn clipped(transform: &Transform, planes: &[Vec4]) -> Self {
        use bevy::math::DVec2;
        let x = (transform.rotation * Vec3::X * transform.scale.x).truncate().as_dvec2();
        let y = (transform.rotation * Vec3::Y * transform.scale.y).truncate().as_dvec2();
        let origin = transform.translation.truncate().as_dvec2();
        let mut polygon = vec![DVec2::new(-0.5, -0.5), DVec2::new(0.5, -0.5), DVec2::new(0.5, 0.5), DVec2::new(-0.5, 0.5)];
        for plane in planes {
            let normal = DVec2::new(f64::from(plane.z), f64::from(plane.w));
            let point = plane.truncate().truncate().as_dvec2();
            let side = |p: DVec2| (origin + x * p.x + y * p.y - point).dot(normal);
            let Some(&last) = polygon.last() else { break; };
            let mut previous = last;
            let mut before = side(previous);
            let mut clipped = Vec::with_capacity(polygon.len() + 1);
            for current in polygon {
                let after = side(current);
                if (before >= 0.0) != (after >= 0.0) {
                    clipped.push(previous + (current - previous) * (before / (before - after)));
                }
                if after >= 0.0 { clipped.push(current); }
                previous = current;
                before = after;
            }
            polygon = clipped;
        }
        let mut mesh = Self::default();
        if polygon.len() < 3 { return mesh; }
        for point in polygon {
            mesh.positions.push([point.x as f32, point.y as f32, 0.0]);
            mesh.uvs.push([(point.x + 0.5) as f32, (0.5 - point.y) as f32]);
        }
        for i in 1..mesh.positions.len() as u32 - 1 {
            mesh.indices.extend([0, i, i + 1]);
        }
        mesh
    }

    fn mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; self.positions.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

struct ClipSlot {
    entity: Entity,
    material: Handle<PortalClipMaterial>,
    mesh: Handle<Mesh>,
    geometry: PieceMesh,
}

/// Reuse each piece's entity, mesh and texture material across frames.
#[derive(Default)]
pub struct ClipQuadPool {
    slots: Vec<ClipSlot>,
    used: usize,
    made: usize,
}

impl ClipQuadPool {
    pub fn begin(&mut self, alive: impl Fn(Entity) -> bool) {
        self.slots.retain(|slot| alive(slot.entity));
        self.used = 0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        commands: &mut Commands,
        materials: &mut Assets<PortalClipMaterial>,
        meshes: &mut Assets<Mesh>,
        material: PortalClipMaterial,
        transform: Transform,
        planes: &[Vec4],
        with: impl Bundle,
        name: &'static str,
    ) -> Entity {
        let geometry = PieceMesh::clipped(&transform, planes);
        let entity = match self.slots.get_mut(self.used) {
            Some(slot) => {
                if materials.get(&slot.material).is_none_or(|old| !old.same_as(&material)) {
                    materials.insert(slot.material.id(), material).ok();
                }
                if slot.geometry != geometry {
                    meshes.insert(slot.mesh.id(), geometry.mesh()).ok();
                    slot.geometry = geometry;
                }
                commands.entity(slot.entity).insert((transform, Visibility::Inherited, with));
                slot.entity
            }
            None => {
                let material = materials.add(material);
                let mesh = meshes.add(geometry.mesh());
                let entity = commands.spawn((Mesh2d(mesh.clone()), MeshMaterial2d(material.clone()),
                    transform, Visibility::Inherited, with, Name::new(name))).id();
                self.slots.push(ClipSlot { entity, material, mesh, geometry });
                self.made += 1;
                entity
            }
        };
        self.used += 1;
        entity
    }

    pub fn made(&self) -> usize { self.made }

    pub fn warm(
        &mut self,
        commands: &mut Commands,
        materials: &mut Assets<PortalClipMaterial>,
        meshes: &mut Assets<Mesh>,
        count: usize,
        with: impl Bundle + Clone,
    ) {
        while self.slots.len() < count {
            let material = materials.add(PortalClipMaterial::new(Vec4::new(0.0, 0.0, 1.0, 1.0), Vec4::ZERO, Vec4::ZERO, Handle::default()));
            let geometry = PieceMesh::default();
            let mesh = meshes.add(geometry.mesh());
            let entity = commands.spawn((Mesh2d(mesh.clone()), MeshMaterial2d(material.clone()),
                Transform::default(), Visibility::Hidden, with.clone(), Name::new("Portal piece mesh"))).id();
            self.slots.push(ClipSlot { entity, material, mesh, geometry });
        }
    }

    /// Release bindings to a retired capture, including hidden pooled pieces.
    pub(crate) fn forget_image(&mut self, image: &Handle<Image>, materials: &mut Assets<PortalClipMaterial>) {
        for slot in &self.slots {
            if materials.get(&slot.material).is_some_and(|material| material.color_texture == *image) {
                if let Some(mut material) = materials.get_mut(&slot.material) {
                    material.color_texture = Handle::default();
                }
            }
        }
    }

    pub fn end(&mut self, commands: &mut Commands) {
        for slot in &self.slots[self.used..] {
            commands.entity(slot.entity).insert(Visibility::Hidden);
        }
    }
}

impl Material2d for PortalClipMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_portal2d_presentation/shaders/portal_clip.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Register the clip material when the host supplies its asset prerequisites.
/// Asset-only hosts do not install the material or warm a mesh quad.
pub(crate) fn add_portal_clip_material_plugin(app: &mut App) {
    // Shader registration needs the embedded registry. Quad creation also needs
    // mesh assets; an asset registry alone does not supply them.
    if !app.world().contains_resource::<Assets<Mesh>>()
        || !app.world().contains_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
    {
        return;
    }
    // Ensure, do not add: `add_plugins` panics on a duplicate, and both the
    // transit pieces and the far-side compositor call this (both on by
    // default), without knowing about each other.
    if app.is_plugin_added::<Material2dPlugin<PortalClipMaterial>>() {
        return;
    }
    embedded_asset!(app, "shaders/portal_clip.wgsl");
    app.add_plugins(Material2dPlugin::<PortalClipMaterial>::default());
    app.add_systems(Startup, warm_the_clip_material);
}

/// A clear quad of the clip material, drawn for the whole run, so its
/// pipeline is compiled before the first piece needs it. A pipeline is
/// compiled the first time something is drawn with it, in the background:
/// until then nothing with it is drawn. The first body that crossed a portal
/// in a session was hidden whole on the frame it was first cut, and its
/// pieces were not drawn (measured with the seam lab: one frame with none of
/// the gauge, on the first crossing of each run and on no later one).
fn warm_the_clip_material(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PortalClipMaterial>>,
) {
    let quad = meshes.add(Rectangle::default());
    commands.spawn((
        Mesh2d(quad),
        MeshMaterial2d(materials.add(PortalClipMaterial::new(
            Vec4::new(0.0, 0.0, 1.0, 1.0),
            Vec4::ZERO,
            Vec4::ZERO,
            Handle::default(),
        ))),
        Transform::default(),
        bevy::camera::visibility::NoFrustumCulling,
        Name::new("Portal clip material warm-up"),
    ));
}

/// An engine-space half-plane (point + inward normal) as a mesh clip
/// render-space `(point.xy, normal.xy)` uniform. Positions go through the one
/// canonical engine→Bevy adapter ([`PortalWorldFrame::to_render`]); directions
/// only flip y (engine is y-down, render is y-up).
pub fn clip_plane_render(frame: &PortalWorldFrame, point: Vec2, normal: Vec2) -> Vec4 {
    let p = frame.to_render(point, 0.0);
    Vec4::new(p.x, p.y, normal.x, -normal.y)
}

// `SpriteFrameBasis` / `sprite_frame_basis` live in `ambition_sprite_fx` (they
// are not portal concepts) and are re-exported so callers are unchanged.
pub use ambition_sprite_fx::{sprite_frame_basis, SpriteFrameBasis};

/// Pose a piece quad so it draws exactly where the source sprite would: the
/// unit rect is scaled to the drawn size, and the sprite's anchor becomes a
/// rotated world offset (a mesh quad is center-origin; a sprite pivots on its
/// anchor). `base` carries translation (incl. z), rotation, and any extra
/// sprite scale.
pub fn clip_piece_transform(base: &Transform, anchor: Vec2, size: Vec2) -> Transform {
    let offset = base.rotation * (-anchor * size * base.scale.truncate()).extend(0.0);
    Transform {
        translation: base.translation + offset,
        rotation: base.rotation,
        scale: (size * base.scale.truncate()).extend(1.0),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod idempotency_tests {
    use super::*;
    use bevy::asset::AssetPlugin;

    /// Repeated installation must create one usable warm-up quad.
    #[test]
    fn adding_the_clip_material_twice_is_safe() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), bevy::mesh::MeshPlugin));
        add_portal_clip_material_plugin(&mut app);
        add_portal_clip_material_plugin(&mut app);
        app.update();
        let quad = app.world_mut().query::<&Mesh2d>().single(app.world()).unwrap().0.clone();
        let meshes = app.world().resource::<Assets<Mesh>>();
        assert!(meshes.get(&quad).is_some());
        assert_eq!(meshes.len(), 1);
    }
}
