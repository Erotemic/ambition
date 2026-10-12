//! The barrier of Eternal Cosmic Mary-O.
//!
//! Jon, 2026-10-11: "I want her shader to make it looks like there is a
//! barrier around her, on her border, (which should exclude her wings ...).
//! So the shader is like an invincible barrier and communicates to the player
//! her aura is impenetrable."
//!
//! Presentation only, and content-owned. In the form she is drawn from parts
//! (`mary_o_v2_cosmic` of the sprite renderer), and her wings are parts of
//! their own. The part renderer says where the parts of her body are
//! ([`PartSetRequest`] for [`BODY_TRACKS`]), and one quad over her draws
//! light on the border of their shape (`shaders/cosmic_barrier.wgsl`). The
//! quad draws nothing on her, and nothing about her wings.
//!
//! The light about her (`cosmic_aura.rs`) stays under her, and the quasar
//! draws over her while she holds one (`quasar_shader.rs`): three layers.
//!
//! A device with no budget for shaders draws none, a headless composition
//! installs none, and she has no barrier in a row that is not of parts (her
//! transformation).

use bevy::{
    asset::embedded_asset,
    camera::visibility::RenderLayers,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d::render::rendering::actors::rigged::{
    drive_rigged_presentations, PartSetDraws, PartSetRequest,
};
use ambition_platformer2d::render::rendering::PlayerVisual;

/// The tracks of her flipbook that are her body: her hair and her figure.
/// Her wings, the ring about her head and the star of her crown are not
/// (`_mary_o_v2_cosmic_art.BODY_TRACKS` of the sprite renderer).
pub const BODY_TRACKS: [&str; 2] = ["overlay:hair", "overlay:body"];
/// Sheet pixels to one pixel of her art: her frame is 216 px for a stage of
/// 72 px (`mary_o_v2_cosmic.STAGE`). The barrier is drawn in pixels of her art.
const ART_PIXEL: f32 = 3.0;
/// How far from her body the barrier goes, in pixels of her art: its line,
/// its band, and the soft band about that.
const REACH_PIXELS: f32 = 3.0;
/// The most parts the shader reads.
const MOST_PARTS: usize = 4;
/// Over each part of her, by more than the depth of all her parts.
const BARRIER_Z: f32 = 0.012;

/// The uniforms of `cosmic_barrier.wgsl`, in its order.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct BarrierUniform {
    /// The quad under her feet: its low corner and its size, in sheet pixels.
    pub quad: Vec4,
    /// Each part: its low corner and its size, in the same space.
    pub part_box: [Vec4; MOST_PARTS],
    /// Each part on its page: `(u0, v0, u1, v1)`.
    pub part_uv: [Vec4; MOST_PARTS],
    /// `x`: how many parts. `y`: [`ART_PIXEL`]. `z`: the strength.
    pub control: Vec4,
    /// A corner of the pixels of her art: the barrier is in step with them.
    pub grid: Vec4,
    /// The line next to her, the band about it, and the soft band about that.
    pub line: Vec4,
    pub band: Vec4,
    pub glow: Vec4,
}

/// The material of the barrier.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MaryOCosmicBarrierMaterial {
    #[uniform(0)]
    pub barrier: BarrierUniform,
    #[texture(1)]
    #[sampler(2)]
    pub page: Handle<Image>,
}

impl Material2d for MaryOCosmicBarrierMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_demo_mary_o/shaders/cosmic_barrier.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource, Default)]
struct CosmicBarrierInstalled;

/// The unit quad the barrier scales.
#[derive(Resource)]
struct CosmicBarrierQuad(Handle<Mesh>);

/// On her visual while it has its barrier: the quad.
#[derive(Component)]
struct HasCosmicBarrier(Entity);

/// Whether this device draws the barrier: it is a shader.
fn barrier_is_in_budget(quality: Option<Res<ResolvedVisualQuality>>) -> bool {
    quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders())
}

/// Install the material and the system that gives her the barrier.
/// Idempotent. An app that does not draw installs nothing.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<CosmicBarrierInstalled>() {
        return;
    }
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }
    app.insert_resource(CosmicBarrierInstalled);
    embedded_asset!(app, "shaders/cosmic_barrier.wgsl");
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<MaryOCosmicBarrierMaterial>::default());
    }
    if let Some(mut meshes) = app.world_mut().get_resource_mut::<Assets<Mesh>>() {
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        app.insert_resource(CosmicBarrierQuad(quad));
    }
    app.add_systems(
        Update,
        (
            raise_her_barrier
                // The parts of this frame, not of the last.
                .after(drive_rigged_presentations)
                .run_if(resource_exists::<CosmicBarrierQuad>)
                .run_if(resource_exists::<Assets<MaryOCosmicBarrierMaterial>>),
            drop_the_barrier_of_a_body_that_is_gone,
        ),
    );
}

/// What the barrier draws this frame: its uniforms, the page of her parts,
/// and the quad under the place of her feet. `None` when no part of her body
/// is drawn from parts, or its page is not loaded.
///
/// A part is read where it stands with no turn and no scale of its own: the
/// layers of her body have none. `page_size` is the size of the page in texels.
fn barrier_of(draws: &PartSetDraws, page_size: impl Fn(&Handle<Image>) -> Option<Vec2>) -> Option<(BarrierUniform, Handle<Image>, Transform)> {
    let page = draws.parts.first()?.image.clone();
    let texels = page_size(&page)?;
    let mut uniform = BarrierUniform::default();
    let (mut low, mut high) = (Vec2::MAX, Vec2::MIN);
    let mut count = 0;
    for part in draws.parts.iter().filter(|part| part.image == page).take(MOST_PARTS) {
        let pivot = part.local.translation.truncate();
        // A sprite's anchor is the point of it at its place.
        let corner = pivot - (part.anchor + Vec2::splat(0.5)) * part.size;
        uniform.part_box[count] = corner.extend(part.size.x).extend(part.size.y);
        uniform.part_uv[count] = Vec4::new(
            part.rect.min.x / texels.x,
            part.rect.min.y / texels.y,
            part.rect.max.x / texels.x,
            part.rect.max.y / texels.y,
        );
        low = low.min(corner);
        high = high.max(corner + part.size);
        // Her figure is the last of them, and it is drawn in whole pixels of
        // her art, so its corner is a corner of those pixels.
        uniform.grid = corner.extend(0.0).extend(0.0);
        count += 1;
    }
    let reach = Vec2::splat((REACH_PIXELS + 1.0) * ART_PIXEL);
    let (low, size) = (low - reach, high - low + reach * 2.0);
    uniform.quad = low.extend(size.x).extend(size.y);
    uniform.control = Vec4::new(count as f32, ART_PIXEL, 1.0, 0.0);
    // Display colours: a white line, a cyan band, a pink glow.
    uniform.line = Vec4::new(1.0, 1.0, 1.0, 0.95);
    uniform.band = Vec4::new(0.45, 0.92, 1.0, 0.62);
    uniform.glow = Vec4::new(1.0, 0.56, 0.86, 0.3);
    let middle = low + size * 0.5;
    let quad = draws
        .place
        .mul_transform(Transform::from_translation(middle.extend(BARRIER_Z)).with_scale(size.extend(1.0)));
    Some((uniform, page, quad))
}

/// Give her the barrier while she is in the form and the device has a budget
/// for it, keep it on the parts of her body, and take it away after.
#[allow(clippy::type_complexity)]
fn raise_her_barrier(
    mut commands: Commands,
    quad: Res<CosmicBarrierQuad>,
    mut materials: ResMut<Assets<MaryOCosmicBarrierMaterial>>,
    images: Res<Assets<Image>>,
    quality: Option<Res<ResolvedVisualQuality>>,
    bodies: Query<
        (
            Entity,
            &WornCharacter,
            Option<&PartSetDraws>,
            Has<PartSetRequest>,
            Option<&HasCosmicBarrier>,
            Option<&RenderLayers>,
        ),
        With<PlayerVisual>,
    >,
    mut quads: Query<(&mut Transform, &mut Visibility, &MeshMaterial2d<MaryOCosmicBarrierMaterial>)>,
) {
    let in_budget = barrier_is_in_budget(quality);
    for (body, worn, draws, asks, has, layers) in &bodies {
        let wanted = in_budget && crate::cosmic::is_cosmic(worn.id());
        if !wanted {
            if asks {
                commands.entity(body).try_remove::<(PartSetRequest, PartSetDraws)>();
            }
            if let Some(has) = has {
                commands.entity(has.0).try_despawn();
                commands.entity(body).try_remove::<HasCosmicBarrier>();
            }
            continue;
        }
        if !asks {
            commands.entity(body).try_insert(PartSetRequest {
                tracks: BODY_TRACKS.iter().map(|track| track.to_string()).collect(),
            });
        }
        let drawn = draws.and_then(|draws| barrier_of(draws, |page| images.get(page).map(|image| image.size_f32())));
        match (drawn, has) {
            (Some((uniform, page, place)), None) => {
                let material = materials.add(MaryOCosmicBarrierMaterial { barrier: uniform, page });
                let mut spawned = commands.spawn((
                    Mesh2d(quad.0.clone()),
                    MeshMaterial2d(material),
                    place,
                    Name::new("mary-o cosmic barrier"),
                ));
                // Where she is drawn: a room that is not live, a view of one seat.
                if let Some(layers) = layers {
                    spawned.insert(layers.clone());
                }
                let spawned = spawned.id();
                commands.entity(body).try_insert(HasCosmicBarrier(spawned));
            }
            (Some((uniform, page, place)), Some(has)) => {
                let Ok((mut transform, mut visibility, material)) = quads.get_mut(has.0) else {
                    continue;
                };
                if *transform != place {
                    *transform = place;
                }
                visibility.set_if_neq(Visibility::Inherited);
                let stale = materials.get(&material.0).is_some_and(|now| now.barrier != uniform || now.page != page);
                if stale {
                    if let Some(mut now) = materials.get_mut(&material.0) {
                        now.barrier = uniform;
                        now.page = page;
                    }
                }
            }
            (None, Some(has)) => {
                // A row that is not of parts: no barrier while it plays.
                if let Ok((_, mut visibility, _)) = quads.get_mut(has.0) {
                    visibility.set_if_neq(Visibility::Hidden);
                }
            }
            (None, None) => {}
        }
    }
}

/// The barrier of a body that is gone goes with it: the quad is not its child.
fn drop_the_barrier_of_a_body_that_is_gone(
    mut commands: Commands,
    mut gone: RemovedComponents<HasCosmicBarrier>,
    owners: Query<&HasCosmicBarrier>,
    quads: Query<Entity, With<MeshMaterial2d<MaryOCosmicBarrierMaterial>>>,
) {
    if gone.read().count() == 0 {
        return;
    }
    for quad in &quads {
        if !owners.iter().any(|has| has.0 == quad) {
            commands.entity(quad).try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::render::rendering::actors::rigged::PlacedPart;

    #[test]
    fn a_headless_app_installs_no_barrier() {
        let mut app = App::new();
        install(&mut app);
        assert!(!app.world().contains_resource::<CosmicBarrierInstalled>());
    }

    /// The quad holds each part of her body and the reach of the barrier
    /// about them, and a part is where its pivot and its anchor put it.
    #[test]
    fn the_quad_holds_her_body_and_the_reach_of_the_barrier() {
        let page = Handle::<Image>::default();
        let part = |at: Vec2, size: Vec2| PlacedPart {
            image: page.clone(),
            rect: Rect::new(10.0, 20.0, 10.0 + size.x, 20.0 + size.y),
            size,
            // The top left of the part is its pivot.
            anchor: Vec2::new(-0.5, 0.5),
            local: Transform::from_translation(at.extend(0.0)),
        };
        let draws = PartSetDraws {
            place: Transform::from_xyz(100.0, 50.0, 3.0),
            parts: vec![part(Vec2::new(-30.0, 90.0), Vec2::new(48.0, 75.0)), part(Vec2::new(-21.0, 96.0), Vec2::new(54.0, 96.0))],
        };
        let (uniform, _, quad) = barrier_of(&draws, |_| Some(Vec2::new(640.0, 541.0))).unwrap();
        assert_eq!(uniform.control.x, 2.0);
        // The figure: its top left at (-21, 96), so its low corner at (-21, 0).
        assert_eq!(uniform.part_box[1], Vec4::new(-21.0, 0.0, 54.0, 96.0));
        assert_eq!(uniform.grid.truncate().truncate(), Vec2::new(-21.0, 0.0));
        let reach = (REACH_PIXELS + 1.0) * ART_PIXEL;
        assert_eq!(uniform.quad, Vec4::new(-30.0 - reach, -reach, 63.0 + 2.0 * reach, 96.0 + 2.0 * reach));
        assert_eq!(quad.translation.z, 3.0 + BARRIER_Z);
        assert!(barrier_of(&PartSetDraws::default(), |_| Some(Vec2::ONE)).is_none());
        assert!(barrier_of(&draws, |_| None).is_none());
    }
}
