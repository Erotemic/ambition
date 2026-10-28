//! The portal-clip material: a `Material2d` that draws one texture-accurate
//! piece of a sprite mid-portal-transit, discarding every fragment behind
//! a world-space clip half-plane.
//!
//! This is the render-side realization of the Core invariant in
//! [`ambition_portal2d::pieces`]: a body straddling a portal pair is ONE logical
//! object with TWO spatial pieces. The carry ([`crate::carry`]) draws what a
//! pair carries as sibling mesh quads with this material: the `here` slice
//! clipped to the front of the entry plane, the `through` slice clipped to the
//! front of the exit plane (plus the exit aperture span). The portal map is an
//! isometry, so the slices tile continuously across the seam: nothing pops
//! when the position snaps at the centroid crossing, and the sunk slice never
//! draws over the far side of a thin wall. The far-side compositor draws a
//! body's uncovered remainder with it too.
//!
//! Clipping runs in the fragment shader against final render-world positions,
//! so it is exact for any anchor, trim rect, flip, roll, or scale — the
//! `Sprite.rect` alternative would have to re-derive all of those per frame.
//! The quad + atlas-frame UV mapping follows the hit-flash overlay pattern
//! (`ambition_render::rendering::hit_flash`), the established way to draw "the
//! sprite's current frame" as a mesh.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::PortalWorldFrame;

/// A clip half-plane that rejects nothing (zero normal = disabled in-shader).
pub const CLIP_PLANE_OFF: Vec4 = Vec4::ZERO;

/// Material2d for one portal-clipped sprite piece.
///
/// Bindings follow the WebGL2-friendly convention of the hit-flash /
/// deep-dream overlays: plain `vec4` uniforms, no struct UBOs, no arrays.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct PortalClipMaterial {
    /// Current atlas frame as a UV rect on the sprite sheet:
    /// `(min.x, min.y, max.x, max.y)` normalized.
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// `(flip_x, flip_y, silhouette, premultiplied)`. `flip_x > 0.5` mirrors
    /// the frame horizontally; `silhouette > 0.5` paints `tint.rgb` masked by
    /// the sample's alpha and `tint.a` -- the look a declared non-sprite
    /// drawable asks for; `premultiplied > 0.5`: the texture's colour is
    /// premultiplied by its alpha (a camera's capture over a clear of
    /// nothing), and the shader divides it out before it is blended.
    #[uniform(1)]
    pub control: Vec4,
    /// Sprite tint (linear RGBA), multiplied into the sample.
    #[uniform(2)]
    pub tint: Vec4,
    /// Clip half-planes `(point.xy, normal.xy)` in render-world space;
    /// fragments with `dot(p - point, normal) < 0` are discarded. Zero
    /// normal disables a plane ([`CLIP_PLANE_OFF`]).
    #[uniform(3)]
    pub clip0: Vec4,
    #[uniform(4)]
    pub clip1: Vec4,
    #[uniform(5)]
    pub clip2: Vec4,
    #[uniform(8)]
    pub clip3: Vec4,
    /// A piece carried through two pairs is bounded by more planes.
    #[uniform(9)]
    pub clip4: Vec4,
    #[uniform(10)]
    pub clip5: Vec4,
    #[texture(6)]
    #[sampler(7)]
    pub color_texture: Handle<Image>,
}

/// How many half planes a piece can be clipped by.
pub const CLIP_PLANES: usize = 6;

impl PortalClipMaterial {
    /// A piece of `color_texture`'s frame `uv_rect`, clipped by `planes`
    /// (render-space, [`clip_plane_render`]); the planes past
    /// [`CLIP_PLANES`] are dropped.
    pub fn new(uv_rect: Vec4, control: Vec4, tint: Vec4, planes: &[Vec4], color_texture: Handle<Image>) -> Self {
        let mut clip = [CLIP_PLANE_OFF; CLIP_PLANES];
        for (slot, plane) in clip.iter_mut().zip(planes) {
            *slot = *plane;
        }
        let [clip0, clip1, clip2, clip3, clip4, clip5] = clip;
        Self { uv_rect, control, tint, clip0, clip1, clip2, clip3, clip4, clip5, color_texture }
    }

    fn same_as(&self, other: &Self) -> bool {
        self.uv_rect == other.uv_rect
            && self.control == other.control
            && self.tint == other.tint
            && [self.clip0, self.clip1, self.clip2, self.clip3, self.clip4, self.clip5]
                == [other.clip0, other.clip1, other.clip2, other.clip3, other.clip4, other.clip5]
            && self.color_texture == other.color_texture
    }
}

/// The clipped quads one system draws, kept from frame to frame: a frame
/// takes the quads it needs in order, sets each one's material and pose, and
/// hides the rest. A quad and its material are made once, when a frame first
/// needs that many, and reused after; nothing is spawned or added per frame.
#[derive(Default)]
pub struct ClipQuadPool {
    slots: Vec<(Entity, Handle<PortalClipMaterial>)>,
    used: usize,
    /// Quads made, ever.
    made: usize,
}

impl ClipQuadPool {
    /// Start a frame. `alive` says which quads still exist (a room that
    /// unloads takes its quads with it).
    pub fn begin(&mut self, alive: impl Fn(Entity) -> bool) {
        self.slots.retain(|(entity, _)| alive(*entity));
        self.used = 0;
    }

    /// Draw one piece with the next quad. `with` is what else the quad
    /// carries this frame (its marker, its room's stamp).
    pub fn draw(
        &mut self,
        commands: &mut Commands,
        materials: &mut Assets<PortalClipMaterial>,
        mesh: &Handle<Mesh>,
        material: PortalClipMaterial,
        transform: Transform,
        with: impl Bundle,
        name: &'static str,
    ) -> Entity {
        let entity = match self.slots.get(self.used) {
            Some((entity, handle)) => {
                if materials.get(handle).is_none_or(|old| !old.same_as(&material)) {
                    materials.insert(handle.id(), material).ok();
                }
                commands.entity(*entity).insert((transform, Visibility::Inherited, with));
                *entity
            }
            None => {
                let handle = materials.add(material);
                let entity = commands
                    .spawn((Mesh2d(mesh.clone()), MeshMaterial2d(handle.clone()), transform, Visibility::Inherited, with, Name::new(name)))
                    .id();
                self.slots.push((entity, handle));
                self.made += 1;
                entity
            }
        };
        self.used += 1;
        entity
    }

    /// Quads made, ever: a quad made on a frame may not be drawn on it.
    pub fn made(&self) -> usize {
        self.made
    }

    /// Make `count` quads before the first frame needs them, hidden, each
    /// with `with`.
    pub fn warm(
        &mut self,
        commands: &mut Commands,
        materials: &mut Assets<PortalClipMaterial>,
        mesh: &Handle<Mesh>,
        count: usize,
        with: impl Bundle + Clone,
    ) {
        while self.slots.len() < count {
            let handle = materials.add(PortalClipMaterial::new(Vec4::new(0.0, 0.0, 1.0, 1.0), Vec4::ZERO, Vec4::ZERO, &[], Handle::default()));
            let entity = commands
                .spawn((Mesh2d(mesh.clone()), MeshMaterial2d(handle.clone()), Transform::default(), Visibility::Hidden, with.clone(), Name::new("Portal clip quad")))
                .id();
            self.slots.push((entity, handle));
        }
    }

    /// End a frame: the quads it did not take are hidden.
    pub fn end(&mut self, commands: &mut Commands) {
        for (entity, _) in &self.slots[self.used..] {
            commands.entity(*entity).insert(Visibility::Hidden);
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

/// The unit quad every clipped piece is drawn with, made at startup with
/// the warm-up below: a mesh made on the frame of the first cut was not
/// drawn on that frame.
#[derive(Resource, Clone)]
pub struct PortalPieceQuad(pub Handle<Mesh>);

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
    commands.insert_resource(PortalPieceQuad(quad.clone()));
    commands.spawn((
        Mesh2d(quad),
        MeshMaterial2d(materials.add(PortalClipMaterial::new(
            Vec4::new(0.0, 0.0, 1.0, 1.0),
            Vec4::ZERO,
            Vec4::ZERO,
            &[],
            Handle::default(),
        ))),
        Transform::default(),
        bevy::camera::visibility::NoFrustumCulling,
        Name::new("Portal clip material warm-up"),
    ));
}

/// An engine-space half-plane (point + outward normal) as the shader's
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
        let quad = app.world().resource::<PortalPieceQuad>();
        let meshes = app.world().resource::<Assets<Mesh>>();
        assert!(meshes.get(&quad.0).is_some());
        assert_eq!(meshes.len(), 1);
    }
}
