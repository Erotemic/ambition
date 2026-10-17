//! Procedural lightning: bolts between two ends and auras on bodies, drawn
//! from [`VfxMessage::Lightning`] and [`VfxMessage::LightningAura`] rows.
//!
//! The shape is geometry built here each flicker ([`bolt`]): a jagged trunk
//! and its forks, or arcs round a body's ellipse, laid out as triangles with
//! a colour at each corner. Its material only adds those colours to what is
//! under them; it draws no shape of its own.
//!
//! Each row is a LEASE (`ambition_vfx::lightning`): one entity per key, its
//! ends moved by each row that renews it, faded out when the rows stop.
//!
//! Quality is presentation policy. With the screen-shader budget the bolt
//! has its halo, its forks and its light where it lands, added on top; with
//! none (or on Android, which has no custom material) it is its white-hot
//! core alone, blended, and an aura is not drawn. A bolt is a tell, so the
//! core is drawn on every device.
//!
//! Not yet portal-composited: a bolt that crosses a portal is drawn whole on
//! the near side.

pub mod bolt;

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
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
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomOf, LiveRooms, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::presented_pose::PresentedPose;
use ambition_vfx::vfx::{VfxInRoom, VfxMessage};
use ambition_vfx::{LightningEnd, LightningKey, LightningLook};

use crate::quality::ResolvedVisualQuality;
use bolt::{Rng, Strand, Tris};

/// Over bodies and the other effects.
const Z: f32 = WORLD_Z_FX + 3.0;

/// Seconds a bolt takes to fade once its lease runs out.
const FADE_S: f32 = 0.12;

/// The material of every bolt: the mesh's colours, ADDED to what is under
/// them (light adds; it does not cover).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct LightningMaterial {
    /// Multiplies every colour; `(1, 1, 1, 1)` draws the mesh as built.
    #[uniform(0)]
    pub strength: Vec4,
}

impl Material2d for LightningMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/lightning/lightning.wgsl".into()
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

/// The two materials a bolt is drawn with: added light where the device has
/// the material, plain blending where it does not.
#[derive(Resource)]
struct LightningMaterials {
    added: Option<Handle<LightningMaterial>>,
    plain: Handle<ColorMaterial>,
}

/// What this device draws of lightning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    /// The halo, the light where a bolt lands, and auras.
    pub rich: bool,
    /// Of a look's forks (and an aura's arcs), how many.
    pub forks: f32,
}

impl Detail {
    pub fn of(quality: Option<&ResolvedVisualQuality>) -> Detail {
        match quality {
            None => Detail { rich: true, forks: 1.0 },
            Some(q) if !q.budget.shaders.draws_screen_shaders() => Detail { rich: false, forks: 0.0 },
            Some(q) => Detail { rich: true, forks: q.budget.particles.spawn_rate_scale.clamp(0.0, 1.0) },
        }
    }

    fn forks(&self, look: &LightningLook) -> usize {
        (look.forks as f32 * self.forks).round() as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Bolt { from: LightningEnd, to: LightningEnd },
    Aura { owner: Entity, center: ae::Vec2, half: ae::Vec2 },
}

/// One leased bolt or aura.
#[derive(Component)]
pub struct LightningVisual {
    key: LightningKey,
    shape: Shape,
    look: LightningLook,
    /// Seconds left of the lease.
    hold: f32,
    age: f32,
    /// 1 while held; down to 0 once the lease runs out.
    fade: f32,
    since_path: f32,
    rng: Rng,
    strands: Vec<Strand>,
    intensity: f32,
    /// Where the body ends were last seen, for a body that has gone.
    last_seen: [Option<ae::Vec2>; 2],
    mesh: Handle<Mesh>,
}

/// Installs lightning. A host with no renderer installs nothing: its rows
/// drain unread.
pub struct LightningPlugin;

impl Plugin for LightningPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() {
            return;
        }
        #[cfg(not(target_os = "android"))]
        let added = {
            bevy::asset::embedded_asset!(app, "lightning.wgsl");
            app.add_plugins(Material2dPlugin::<LightningMaterial>::default());
            app.world_mut()
                .get_resource_mut::<Assets<LightningMaterial>>()
                .map(|mut materials| materials.add(LightningMaterial { strength: Vec4::ONE }))
        };
        #[cfg(target_os = "android")]
        let added = None;
        let Some(plain) = app
            .world_mut()
            .get_resource_mut::<Assets<ColorMaterial>>()
            .map(|mut materials| materials.add(ColorMaterial::from_color(Color::WHITE)))
        else {
            return;
        };
        app.insert_resource(LightningMaterials { added, plain });
        use ambition_platformer2d_shared_tangle::lifecycle::session_world_exists;
        use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
        app.add_systems(
            Update,
            (receive_lightning, animate_lightning)
                .chain()
                .after(Platformer2dSimulationPhase::CoreSimulation)
                // A body end is where its body is drawn this frame.
                .after(crate::rendering::actors::sync_visuals)
                .run_if(session_world_exists),
        );
    }
}

/// A mesh of `tris`, for the render world only (each frame's is a new one).
/// Nothing to draw is one invisible triangle, so the mesh is never empty.
fn mesh_of(mut tris: Tris) -> Mesh {
    if tris.indices.is_empty() {
        tris = Tris { positions: vec![[0.0; 3]; 3], colors: vec![[0.0; 4]; 3], indices: vec![0, 1, 2] };
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, tris.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, tris.colors);
    mesh.insert_indices(Indices::U32(tris.indices));
    mesh
}

/// Renew the lease of each row's key, or start one.
fn receive_lightning(
    mut commands: Commands,
    mut messages: MessageReader<VfxInRoom>,
    rooms: LiveRooms,
    active_session: Option<Res<ActiveSessionScope>>,
    materials: Res<LightningMaterials>,
    quality: Option<Res<ResolvedVisualQuality>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut live: Query<&mut LightningVisual>,
) {
    let session = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let detail = Detail::of(quality.as_deref());
    let mut started: Vec<LightningKey> = Vec::new();
    for message in messages.read() {
        let (key, shape, look, hold) = match message.vfx {
            VfxMessage::Lightning { key, from, to, look, hold } => (key, Shape::Bolt { from, to }, look, hold),
            VfxMessage::LightningAura { key, owner, center, half, look, hold } => {
                (key, Shape::Aura { owner, center, half }, look, hold)
            }
            _ => continue,
        };
        if let Some(mut visual) = live.iter_mut().find(|visual| visual.key == key) {
            visual.shape = shape;
            visual.look = look;
            visual.hold = visual.hold.max(hold);
            visual.fade = 1.0;
            continue;
        }
        // A second row for a lease started this frame: the first holds it.
        if started.contains(&key) {
            continue;
        }
        let (Some(session), Some(room)) = (session, message.room.or_else(|| rooms.sole())) else {
            continue;
        };
        started.push(key);
        let mesh = meshes.add(mesh_of(Tris::default()));
        let visual = LightningVisual {
            key,
            shape,
            look,
            hold,
            age: 0.0,
            fade: 1.0,
            since_path: f32::INFINITY,
            rng: Rng::new(key.0),
            strands: Vec::new(),
            intensity: 1.0,
            last_seen: [None, None],
            mesh: mesh.clone(),
        };
        let mut entity = commands.spawn_session_scoped(
            session.in_room(Some(room)),
            (Name::new("VFX lightning"), Mesh2d(mesh), Transform::default(), NoFrustumCulling, visual),
        );
        match (&materials.added, detail.rich) {
            (Some(added), true) => entity.insert(MeshMaterial2d(added.clone())),
            _ => entity.insert(MeshMaterial2d(materials.plain.clone())),
        };
    }
}

/// Where `end` is now, remembering a body end in `seen`.
fn resolve(end: LightningEnd, owners: &Query<&PresentedPose>, seen: &mut Option<ae::Vec2>) -> Option<ae::Vec2> {
    match end {
        LightningEnd::World(at) => Some(at),
        LightningEnd::Body { owner, offset } => {
            if let Ok(pose) = owners.get(owner) {
                *seen = Some(pose.presented() + offset);
            }
            *seen
        }
    }
}

fn linear(c: [f32; 4], alpha: f32) -> [f32; 4] {
    let l = Color::srgba(c[0], c[1], c[2], c[3]).to_linear();
    [l.red, l.green, l.blue, (l.alpha * alpha).clamp(0.0, 1.0)]
}

/// Age, fade, re-path and re-mesh every bolt and aura.
fn animate_lightning(
    mut commands: Commands,
    time: Res<Time>,
    quality: Option<Res<ResolvedVisualQuality>>,
    world: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    owners: Query<&PresentedPose>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visuals: Query<(Entity, &mut LightningVisual, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let detail = Detail::of(quality.as_deref());
    for (entity, mut v, mut transform) in &mut visuals {
        v.age += dt;
        v.hold -= dt;
        v.since_path += dt;
        if v.hold <= 0.0 {
            v.fade -= dt / FADE_S;
            if v.fade <= 0.0 {
                meshes.remove(&v.mesh);
                commands.entity(entity).despawn();
                continue;
            }
        }
        let Some(room) = world.of(entity) else {
            continue;
        };
        let v = &mut *v;
        // The frame: where its (0, 0) is in the world, and its two axes.
        let frame = match v.shape {
            Shape::Bolt { from, to } => {
                let [seen_from, seen_to] = &mut v.last_seen;
                match (resolve(from, &owners, seen_from), resolve(to, &owners, seen_to)) {
                    (Some(a), Some(b)) => Some((a, b - a, (b - a).perp())),
                    _ => None,
                }
            }
            Shape::Aura { owner, center, half } => {
                resolve(LightningEnd::Body { owner, offset: center }, &owners, &mut v.last_seen[0])
                    .map(|c| (c, ae::Vec2::new(half.x, 0.0), ae::Vec2::new(0.0, half.y)))
            }
        };
        let Some((origin, ax, ay)) = frame else {
            continue;
        };
        if v.since_path >= v.look.flicker_s {
            v.since_path = 0.0;
            let forks = detail.forks(&v.look);
            v.strands = match v.shape {
                Shape::Bolt { .. } => bolt::bolt(&mut v.rng, ax.length(), v.look.jag, forks),
                Shape::Aura { .. } if detail.rich => bolt::aura(&mut v.rng, v.look.jag, forks.max(1)),
                Shape::Aura { .. } => Vec::new(),
            };
            v.intensity = v.rng.range(0.7, 1.0);
        }
        transform.translation = world_to_bevy(&room.0, origin, Z);
        // Frame point -> mesh space (world offset from the origin, y up).
        let place = |p: Vec2| {
            let w = ax * p.x + ay * p.y;
            Vec2::new(w.x, -w.y)
        };
        let head = if v.look.grow_s > 0.0 { (v.age / v.look.grow_s).min(1.0) } else { 1.0 };
        let k = v.intensity * v.fade.clamp(0.0, 1.0);
        let look = v.look;
        let mut tris = Tris::default();
        for strand in &v.strands {
            if !strand.trunk && strand.appears_at > head {
                continue;
            }
            let points = if strand.trunk { bolt::grown(&strand.points, head) } else { strand.points.clone() };
            let points: Vec<Vec2> = points.into_iter().map(place).collect();
            let widths: Vec<f32> = strand.widths.iter().take(points.len()).map(|w| w * look.width).collect();
            if detail.rich {
                let glow: Vec<f32> = widths.iter().map(|w| w * look.glow_scale).collect();
                tris.ribbon(&points, &glow, linear(look.glow, k), linear(look.glow, 0.0));
            }
            tris.ribbon(&points, &widths, linear(look.core, k), linear(look.core, 0.3 * k));
        }
        if detail.rich && look.contact_glow > 0.0 {
            let radius = look.contact_glow * (0.75 + 0.35 * v.intensity);
            let glow = linear(look.glow, 0.8 * k);
            match v.shape {
                Shape::Bolt { .. } => {
                    tris.glow(Vec2::ZERO, radius * 0.45, glow, linear(look.glow, 0.0));
                    if head >= 1.0 {
                        tris.glow(place(Vec2::X), radius, glow, linear(look.glow, 0.0));
                    }
                }
                Shape::Aura { .. } => tris.glow(Vec2::ZERO, radius, glow, linear(look.glow, 0.0)),
            }
        }
        if let Some(mut mesh) = meshes.get_mut(&v.mesh) {
            *mesh = mesh_of(tris);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_persistence::settings::video::{VisualQualityBudget, VisualQualityProfile};

    fn at(profile: VisualQualityProfile) -> Detail {
        Detail::of(Some(&ResolvedVisualQuality { profile, budget: VisualQualityBudget::for_profile(profile) }))
    }

    /// The core is the tell, so every device draws it; the rest is on top.
    #[test]
    fn a_device_with_no_shader_budget_draws_the_core_alone() {
        let potato = at(VisualQualityProfile::Potato);
        assert!(!potato.rich);
        assert_eq!(potato.forks(&LightningLook::STRIKE), 0);
        let high = at(VisualQualityProfile::High);
        assert!(high.rich);
        assert_eq!(high.forks(&LightningLook::STRIKE), LightningLook::STRIKE.forks as usize);
        assert!(at(VisualQualityProfile::Low).forks(&LightningLook::FORCE) < high.forks(&LightningLook::FORCE));
    }
}
