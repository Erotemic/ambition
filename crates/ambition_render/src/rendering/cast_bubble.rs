//! The off-screen bubble: a cast member wholly outside the view is shown in a
//! ring at the view's edge, with a live view of the member inside it.
//!
//! A platform fighter's camera stops short of the blast line
//! (`CastFramingTuning::bounds`), so a launched fighter leaves the frame while
//! it is still in play. Without the bubble, the player cannot see the fighter
//! they are recovering with. The game declares the bubble
//! (`CastFramingTuning::bubble`); with no declaration, nothing here runs.
//!
//! Each bubble is a camera that draws the gameplay layers, centred on the
//! member, into a texture. The texture is drawn on a disc on the front HUD
//! layer, inside a ring in the member's colour. The world is drawn by its own
//! cameras, so a portal, a parallax-free stage and every body show in the
//! bubble as they show in the view; nothing is copied.
//!
//! Presentation only: it reads [`BodyPoseView`], never a simulation component.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use ambition_platformer2d_shared_tangle::camera_layers::{
    FrontHudCamera, MainCamera, FRONT_HUD_LAYER, PARALLAX_BACKGROUND_LAYER,
};
use ambition_platformer2d_shared_tangle::markers::{CastFramingTuning, FramedCast};
use ambition_sim_view::{BodyPoseView, CameraViewState};

/// The texels across a bubble's texture. Twice a typical diameter, so the
/// bubble stays sharp on a high-density display.
const BUBBLE_TEXELS: u32 = 192;

/// The bubble cameras draw before every camera that shows their texture.
const BUBBLE_CAMERA_ORDER: isize = -500;

/// The ring's width, as a fraction of the bubble's radius.
const RING_FRACTION: f32 = 0.12;

/// The space between a bubble and the view's edge, in logical pixels.
const EDGE_GAP_PX: f32 = 6.0;

/// One bubble: its camera, its texture, and the disc that shows it.
struct BubbleSlot {
    camera: Entity,
    disc: Entity,
    ring_material: Handle<ColorMaterial>,
}

/// The bubbles made so far, one per cast position. Made on first need and
/// kept; an unused bubble is hidden and its camera rests.
#[derive(Resource, Default)]
pub struct CastBubbles {
    slots: Vec<BubbleSlot>,
}

/// Marks a bubble's camera.
#[derive(Component)]
pub struct CastBubbleCamera;

/// Marks a bubble's disc (the ring is its child).
#[derive(Component)]
pub struct CastBubbleDisc;

/// Run only for a game that declares a bubble, or while one is still shown.
pub fn bubbles_are_asked_for_or_drawn(
    tuning: Option<Res<CastFramingTuning>>,
    bubbles: Option<Res<CastBubbles>>,
) -> bool {
    tuning.is_some_and(|tuning| tuning.bubble.is_some())
        || bubbles.is_some_and(|bubbles| !bubbles.slots.is_empty())
}

/// Where the bubble for a member sits, relative to the view's centre, in
/// logical pixels (+y up). `None` while any part of the member is in the view.
///
/// `half_view` is half the view's size, `at` is the member's centre and
/// `half_body` half its size, all in logical pixels from the view's centre.
/// The bubble sits on the line from the centre to the member, held inside the
/// view by its radius and a small gap.
pub fn bubble_centre(half_view: Vec2, at: Vec2, half_body: Vec2, radius: f32) -> Option<Vec2> {
    let outside = at.x.abs() - half_body.x > half_view.x || at.y.abs() - half_body.y > half_view.y;
    if !outside {
        return None;
    }
    let room = (half_view - Vec2::splat(radius + EDGE_GAP_PX)).max(Vec2::ZERO);
    let reach = |room: f32, along: f32| if along.abs() > f32::EPSILON { room / along.abs() } else { f32::INFINITY };
    let t = reach(room.x, at.x).min(reach(room.y, at.y));
    Some(at * t)
}

/// Show each cast member that is wholly outside the view in its bubble.
#[allow(clippy::too_many_arguments)]
pub fn present_cast_bubbles(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut bubbles: ResMut<CastBubbles>,
    tuning: Res<CastFramingTuning>,
    framed: Option<Res<FramedCast>>,
    poses: Query<&BodyPoseView>,
    views: Query<&CameraViewState>,
    main: Query<
        (
            &Camera,
            &Transform,
            &Projection,
            Option<&RenderLayers>,
            Option<&ambition_sim_view::PresentsView>,
        ),
        (With<MainCamera>, Without<CastBubbleCamera>),
    >,
    hud: Query<&Camera, (With<FrontHudCamera>, Without<MainCamera>, Without<CastBubbleCamera>)>,
    mut cameras: Query<
        (&mut Camera, &mut Transform, &mut RenderLayers, &mut Projection),
        (With<CastBubbleCamera>, Without<MainCamera>, Without<FrontHudCamera>),
    >,
    mut discs: Query<
        (&mut Transform, &mut Visibility),
        (With<CastBubbleDisc>, Without<Camera>),
    >,
) {
    let shown = tuning
        .bubble
        .zip(framed.as_deref().filter(|framed| !framed.0.is_empty()))
        .zip(placement(&main, &hud, &views));
    let mut used = vec![false; bubbles.slots.len()];
    if let Some(((bubble, framed), frame)) = shown {
        let radius = bubble.diameter_px * 0.5;
        for (index, member) in framed.0.iter().enumerate() {
            let Ok(pose) = poses.get(*member) else {
                continue;
            };
            if pose.out_of_the_world {
                continue;
            }
            // World +y is down; the view's +y is up.
            let offset = (pose.pos - frame.center_world) * Vec2::new(1.0, -1.0);
            let Some(centre) = bubble_centre(
                frame.half_view_px,
                offset * frame.px_per_unit,
                pose.size * 0.5 * frame.px_per_unit,
                radius,
            ) else {
                continue;
            };
            while bubbles.slots.len() <= index {
                let slot = spawn_slot(&mut commands, &mut images, &mut meshes, &mut materials, bubbles.slots.len());
                bubbles.slots.push(slot);
                used.push(false);
            }
            used[index] = true;
            let slot = &bubbles.slots[index];
            if let Ok((mut camera, mut transform, mut layers, mut projection)) = cameras.get_mut(slot.camera) {
                camera.is_active = true;
                camera.clear_color = ClearColorConfig::Custom(Color::srgba(
                    bubble.backdrop[0],
                    bubble.backdrop[1],
                    bubble.backdrop[2],
                    bubble.backdrop[3],
                ));
                transform.translation = frame.camera_at + offset.extend(0.0);
                transform.rotation = frame.camera_rotation;
                let wanted = frame.layers.clone();
                if *layers != wanted {
                    *layers = wanted;
                }
                if let Projection::Orthographic(orthographic) = &mut *projection {
                    orthographic.scaling_mode = ScalingMode::Fixed {
                        width: bubble.span,
                        height: bubble.span,
                    };
                }
            }
            if let Ok((mut transform, mut visibility)) = discs.get_mut(slot.disc) {
                transform.translation = (frame.view_centre_hud + centre).extend(0.0);
                transform.scale = Vec3::splat(bubble.diameter_px);
                *visibility = Visibility::Visible;
            }
            let [r, g, b, a] = bubble.rings[index % bubble.rings.len()];
            let ring = Color::srgba(r, g, b, a);
            if materials.get(&slot.ring_material).is_some_and(|material| material.color != ring) {
                if let Some(mut material) = materials.get_mut(&slot.ring_material) {
                    material.color = ring;
                }
            }
        }
    }
    for (slot, taken) in bubbles.slots.iter().zip(used) {
        if taken {
            continue;
        }
        if let Ok((mut camera, ..)) = cameras.get_mut(slot.camera) {
            if camera.is_active {
                camera.is_active = false;
            }
        }
        if let Ok((_, mut visibility)) = discs.get_mut(slot.disc) {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// What a bubble needs from the presented view this frame.
struct ViewPlacement {
    /// The world point at the view's centre.
    center_world: Vec2,
    /// The main camera's render-space position and rotation.
    camera_at: Vec3,
    camera_rotation: Quat,
    /// The layers the main camera draws, less the parallax (a parallax
    /// layer is placed for the main camera and would be wrong in a bubble).
    layers: RenderLayers,
    /// Half the view's size on screen, in logical pixels.
    half_view_px: Vec2,
    /// Logical pixels per world unit in the view.
    px_per_unit: f32,
    /// The view's centre in the front HUD camera's space (+y up).
    view_centre_hud: Vec2,
}

/// The single presented view, or `None` when there is no one view to place a
/// bubble in (no camera, or a split screen).
fn placement(
    main: &Query<
        (
            &Camera,
            &Transform,
            &Projection,
            Option<&RenderLayers>,
            Option<&ambition_sim_view::PresentsView>,
        ),
        (With<MainCamera>, Without<CastBubbleCamera>),
    >,
    hud: &Query<&Camera, (With<FrontHudCamera>, Without<MainCamera>, Without<CastBubbleCamera>)>,
    views: &Query<&CameraViewState>,
) -> Option<ViewPlacement> {
    let (camera, transform, projection, layers, presents) = main.single().ok()?;
    let Projection::Orthographic(orthographic) = projection else {
        return None;
    };
    let view = views.get(presents?.0).ok()?;
    let viewport = camera.logical_viewport_rect()?;
    let target = hud.single().ok()?.logical_target_size()?;
    let area = orthographic.area.size();
    if area.x <= 0.0 || viewport.width() <= 0.0 {
        return None;
    }
    let centre_screen = viewport.center();
    Some(ViewPlacement {
        center_world: view.center_world,
        camera_at: transform.translation,
        camera_rotation: transform.rotation,
        layers: layers
            .cloned()
            .unwrap_or_default()
            .without(PARALLAX_BACKGROUND_LAYER),
        half_view_px: viewport.size() * 0.5,
        px_per_unit: viewport.width() / area.x,
        view_centre_hud: Vec2::new(centre_screen.x - target.x * 0.5, target.y * 0.5 - centre_screen.y),
    })
}

/// One bubble: a texture, the camera that draws into it, and the disc and
/// ring that show it on the front HUD layer.
fn spawn_slot(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    slot: usize,
) -> BubbleSlot {
    let image = images.add(Image::new_target_texture(
        BUBBLE_TEXELS,
        BUBBLE_TEXELS,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: BUBBLE_CAMERA_ORDER - slot as isize,
                is_active: false,
                ..default()
            },
            // A 4x-MSAA camera draws nothing into a single-sampled target.
            bevy::render::view::Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image.clone())),
            RenderLayers::layer(0),
            Projection::Orthographic(OrthographicProjection::default_2d()),
            CastBubbleCamera,
            Name::new("Cast bubble camera"),
        ))
        .id();
    let disc_material = materials.add(ColorMaterial {
        texture: Some(image),
        ..default()
    });
    let ring_material = materials.add(ColorMaterial::from_color(Color::WHITE));
    let disc = commands
        .spawn((
            Mesh2d(meshes.add(Circle::new(0.5))),
            MeshMaterial2d(disc_material),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(FRONT_HUD_LAYER),
            CastBubbleDisc,
            Name::new("Cast bubble"),
        ))
        .with_child((
            Mesh2d(meshes.add(Annulus::new(0.5, 0.5 * (1.0 + RING_FRACTION)))),
            MeshMaterial2d(ring_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.1),
            RenderLayers::layer(FRONT_HUD_LAYER),
        ))
        .id();
    BubbleSlot {
        camera,
        disc,
        ring_material,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF_VIEW: Vec2 = Vec2::new(400.0, 225.0);
    const HALF_BODY: Vec2 = Vec2::new(12.0, 24.0);

    #[test]
    fn a_member_with_any_part_in_view_has_no_bubble() {
        assert_eq!(bubble_centre(HALF_VIEW, Vec2::new(410.0, 0.0), HALF_BODY, 40.0), None);
        assert_eq!(bubble_centre(HALF_VIEW, Vec2::new(0.0, -240.0), HALF_BODY, 40.0), None);
    }

    #[test]
    fn a_member_past_the_side_is_shown_at_that_side_on_its_line() {
        let centre = bubble_centre(HALF_VIEW, Vec2::new(800.0, 100.0), HALF_BODY, 40.0).expect("off the view");
        assert!((centre.x - (400.0 - 40.0 - EDGE_GAP_PX)).abs() < 1e-3, "{centre:?}");
        assert!((centre.y / centre.x - 100.0 / 800.0).abs() < 1e-5, "on the line to the member: {centre:?}");
    }

    #[test]
    fn a_member_past_a_corner_is_held_inside_both_edges() {
        let centre = bubble_centre(HALF_VIEW, Vec2::new(900.0, 900.0), HALF_BODY, 40.0).expect("off the view");
        assert!(centre.x <= 400.0 - 40.0 && centre.y <= 225.0 - 40.0, "{centre:?}");
    }
}
