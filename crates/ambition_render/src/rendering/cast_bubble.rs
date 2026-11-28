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
//! member, into a texture. The front HUD draws the texture as a disc inside
//! a ring in the member's color. The world is drawn by its own
//! cameras, so a portal, a parallax-free stage and every body show in the
//! bubble as they show in the view; nothing is copied.
//!
//! Presentation only: it reads [`BodyPoseView`], never a simulation component.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use ambition_platformer2d_core::{config::world_to_bevy, RoomGeometry};
use ambition_platformer2d_shared_tangle::camera_layers::{
    FrontHudCamera, MainCamera, PARALLAX_BACKGROUND_LAYER,
};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRoomOf};
use ambition_platformer2d_shared_tangle::markers::{CastFramingTuning, FramedCast};
use ambition_sim_view::{presented_pose::draw_pos, BodyPoseView, PresentedPose};

use super::camera::coverage::{GameplayOutputs, OutputFootprint};

/// The texels across a bubble's texture. Twice a typical diameter, so the
/// bubble stays sharp on a high-density display.
const BUBBLE_TEXELS: u32 = 192;

/// The bubble cameras draw before every camera that shows their texture.
const BUBBLE_CAMERA_ORDER: isize = -500;

/// The ring's width, as a fraction of the bubble's radius.
const RING_FRACTION: f32 = 0.12;

/// The space between a bubble and the view's edge, in logical pixels.
const EDGE_GAP_PX: f32 = 6.0;

/// Draw behind the HUD controls and above the display surround.
const BUBBLE_UI_ORDER: i32 = -1;

/// The camera and HUD node that hold a bubble's texture.
struct BubbleSlot {
    camera: Entity,
    disc: Entity,
}

/// One bubble per cast position that needs it. Hide unused slots and keep
/// them while the game has a bubble declaration.
#[derive(Resource, Default)]
pub struct CastBubbles {
    slots: Vec<Option<BubbleSlot>>,
}

/// Marks a bubble's camera.
#[derive(Component)]
pub struct CastBubbleCamera;

/// Marks the bubble's root HUD node.
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

/// Place the bubble along the line from the viewport center to the fighter.
fn bubble_on_edge(half_view: Vec2, at: Vec2, radius: f32) -> Vec2 {
    let room = (half_view - Vec2::splat(radius + EDGE_GAP_PX)).max(Vec2::ZERO);
    let reach = |room: f32, along: f32| if along.abs() > f32::EPSILON { room / along.abs() } else { f32::INFINITY };
    let t = reach(room.x, at.x).min(reach(room.y, at.y));
    at * t
}

/// Show each cast member that is wholly outside the view in its bubble.
#[allow(clippy::too_many_arguments)]
pub fn present_cast_bubbles(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut bubbles: ResMut<CastBubbles>,
    tuning: Option<Res<CastFramingTuning>>,
    ui_scale: Option<Res<UiScale>>,
    framed: Option<Res<FramedCast>>,
    poses: Query<(&BodyPoseView, Option<&PresentedPose>)>,
    outputs: GameplayOutputs,
    rooms: LiveRoomOf<RoomGeometry>,
    main: Query<(&Camera, &Transform), (With<MainCamera>, Without<CastBubbleCamera>)>,
    hud: Query<(Entity, &Camera), (With<FrontHudCamera>, Without<MainCamera>, Without<CastBubbleCamera>)>,
    mut cameras: Query<
        (&mut Camera, &mut Transform, &mut RenderLayers, &mut Projection),
        (With<CastBubbleCamera>, Without<MainCamera>, Without<FrontHudCamera>),
    >,
    mut discs: Query<
        (&mut Node, &mut BorderColor, &mut UiTargetCamera, &mut Visibility),
        (With<CastBubbleDisc>, Without<Camera>),
    >,
) {
    let Some(bubble) = tuning.and_then(|tuning| tuning.bubble) else {
        for slot in bubbles.slots.drain(..).flatten() {
            commands.entity(slot.camera).try_despawn();
            commands.entity(slot.disc).try_despawn();
        }
        return;
    };
    let shown = framed.as_deref().filter(|framed| !framed.0.is_empty()).and_then(|framed| {
        let frame = placement(&outputs, &main, &hud)?;
        let room = rooms.in_room(frame.room)?;
        Some((framed, frame, &room.0))
    });
    let mut used = vec![false; bubbles.slots.len()];
    if let Some((framed, frame, world)) = shown {
        let radius = bubble.diameter_px * 0.5;
        let ui_scale = ui_scale.as_ref().map_or(1.0, |scale| scale.0);
        for (index, member) in framed.0.iter().enumerate() {
            let Ok((pose, presented)) = poses.get(*member) else {
                continue;
            };
            if pose.out_of_the_world || rooms.room_of(*member) != Some(frame.room) {
                continue;
            }
            let at = world_to_bevy(world, draw_pos(pose, presented), frame.capture_z);
            let bounds = Rect::from_center_size(at.truncate(), pose.size);
            let Some(offscreen) = frame.output.offscreen_center(bounds) else {
                continue;
            };
            let centre = bubble_on_edge(frame.output.logical_size * 0.5, offscreen, radius);
            let [r, g, b, a] = bubble.rings[index % bubble.rings.len()];
            let [red, green, blue, alpha] = bubble.backdrop;
            let screen_center = frame.view_centre_hud + Vec2::new(centre.x, -centre.y);
            let draw = BubbleDraw {
                camera: Transform::from_translation(at).with_rotation(frame.capture_rotation),
                hud: UiTargetCamera(frame.hud),
                disc: bubble_node(screen_center / ui_scale, radius / ui_scale),
                layers: &frame.layers,
                scaling: ScalingMode::Fixed { width: bubble.span, height: bubble.span },
                backdrop: ClearColorConfig::Custom(Color::srgba(red, green, blue, alpha)),
                ring: BorderColor::all(Color::srgba(r, g, b, a)),
            };
            if bubbles.slots.len() <= index {
                bubbles.slots.resize_with(index + 1, || None);
                used.resize(index + 1, false);
            }
            used[index] = true;
            let Some(slot) = &bubbles.slots[index] else {
                bubbles.slots[index] = Some(spawn_slot(&mut commands, &mut images, index, draw));
                continue;
            };
            if let Ok((mut camera, mut transform, mut layers, mut projection)) = cameras.get_mut(slot.camera) {
                camera.is_active = true;
                camera.clear_color = draw.backdrop;
                *transform = draw.camera;
                if *layers != *draw.layers {
                    *layers = draw.layers.clone();
                }
                if let Projection::Orthographic(orthographic) = &mut *projection {
                    orthographic.scaling_mode = draw.scaling;
                }
            }
            if let Ok((mut node, mut ring, mut hud, mut visibility)) = discs.get_mut(slot.disc) {
                node.set_if_neq(draw.disc);
                ring.set_if_neq(draw.ring);
                hud.set_if_neq(draw.hud);
                visibility.set_if_neq(Visibility::Visible);
            }
        }
    }
    for (slot, taken) in bubbles.slots.iter().zip(used) {
        if taken {
            continue;
        }
        let Some(slot) = slot else { continue };
        if let Ok((mut camera, ..)) = cameras.get_mut(slot.camera) {
            if camera.is_active {
                camera.is_active = false;
            }
        }
        if let Ok((.., mut visibility)) = discs.get_mut(slot.disc) {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// What a bubble needs from the presented view this frame.
struct ViewPlacement {
    room: LiveRoomInstance,
    output: OutputFootprint,
    /// The close-up looks along the plane camera's axis.
    capture_z: f32,
    capture_rotation: Quat,
    /// The layers the main camera draws, less the parallax (a parallax
    /// layer is placed for the main camera and would be wrong in a bubble).
    layers: RenderLayers,
    hud: Entity,
    /// The view center in logical pixels from the top left of the HUD.
    view_centre_hud: Vec2,
}

/// The single presented view, or `None` when there is no one view to place a
/// bubble in (no camera, or a split screen).
fn placement(
    outputs: &GameplayOutputs,
    main: &Query<(&Camera, &Transform), (With<MainCamera>, Without<CastBubbleCamera>)>,
    hud: &Query<(Entity, &Camera), (With<FrontHudCamera>, Without<MainCamera>, Without<CastBubbleCamera>)>,
) -> Option<ViewPlacement> {
    let mut outputs = outputs.coverage();
    let primary = outputs.next()?;
    if outputs.next().is_some() {
        return None;
    }
    // The comparison inset shares the HUD of its primary output.
    let output = primary.plane.or(primary.cards)?;
    let (camera, transform) = main.get(primary.source).ok()?;
    let viewport = camera.logical_viewport_rect()?;
    let (hud, hud_camera) = hud.single().ok()?;
    let hud_viewport = hud_camera.logical_viewport_rect()?;
    Some(ViewPlacement {
        room: primary.frame.room,
        output,
        capture_z: transform.translation.z,
        capture_rotation: transform.rotation,
        layers: primary.layers
            .cloned()
            .unwrap_or_default()
            .without(PARALLAX_BACKGROUND_LAYER),
        hud,
        view_centre_hud: viewport.center() - hud_viewport.min,
    })
}

struct BubbleDraw<'a> {
    camera: Transform,
    hud: UiTargetCamera,
    disc: Node,
    layers: &'a RenderLayers,
    scaling: ScalingMode,
    backdrop: ClearColorConfig,
    ring: BorderColor,
}

fn bubble_node(center: Vec2, radius: f32) -> Node {
    let ring = radius * RING_FRACTION;
    let outer = radius + ring;
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(center.x - outer),
        top: Val::Px(center.y - outer),
        width: Val::Px(outer * 2.0),
        height: Val::Px(outer * 2.0),
        border: UiRect::all(Val::Px(ring)),
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// Give the new camera and disc their draw state in the first frame.
fn spawn_slot(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    slot: usize,
    draw: BubbleDraw<'_>,
) -> BubbleSlot {
    let mut target = Image::new_uninit(
        Extent3d { width: BUBBLE_TEXELS, height: BUBBLE_TEXELS, depth_or_array_layers: 1 },
        TextureDimension::D2, TextureFormat::Rgba8UnormSrgb, default(),
    );
    target.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
    let image = images.add(target);
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: BUBBLE_CAMERA_ORDER - slot as isize,
                clear_color: draw.backdrop,
                ..default()
            },
            // A 4x-MSAA camera draws nothing into a single-sampled target.
            bevy::render::view::Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image.clone())),
            draw.layers.clone(),
            draw.camera,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: draw.scaling,
                ..OrthographicProjection::default_2d()
            }),
            CastBubbleCamera,
            Name::new("Cast bubble camera"),
        ))
        .id();
    let disc = commands
        .spawn((
            draw.disc,
            draw.ring,
            draw.hud,
            GlobalZIndex(BUBBLE_UI_ORDER),
            Pickable::IGNORE,
            Visibility::Visible,
            CastBubbleDisc,
            Name::new("Cast bubble"),
        ))
        .with_child((
            ImageNode::new(image),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    BubbleSlot { camera, disc }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF_VIEW: Vec2 = Vec2::new(400.0, 225.0);

    #[test]
    fn a_member_past_the_side_is_shown_at_that_side_on_its_line() {
        let centre = bubble_on_edge(HALF_VIEW, Vec2::new(800.0, 100.0), 40.0);
        assert!((centre.x - (400.0 - 40.0 - EDGE_GAP_PX)).abs() < 1e-3, "{centre:?}");
        assert!((centre.y / centre.x - 100.0 / 800.0).abs() < 1e-5, "on the line to the member: {centre:?}");
    }

    #[test]
    fn a_member_past_a_corner_is_held_inside_both_edges() {
        let centre = bubble_on_edge(HALF_VIEW, Vec2::new(900.0, 900.0), 40.0);
        assert!(centre.x <= 400.0 - 40.0 && centre.y <= 225.0 - 40.0, "{centre:?}");
    }

    #[cfg(feature = "card_view")]
    #[test]
    fn renderer_switch_uses_the_primary_output_to_place_the_bubble() {
        use ambition_platformer2d_core::{self as ae, RoomGeometry};
        use ambition_platformer2d_shared_tangle::lifecycle::{insert_live_room_component, LiveRoomInstance};
        use ambition_platformer2d_shared_tangle::markers::CastBubble;
        use ambition_sim_view::{LocalView, LocalViewId, PresentsView};
        use bevy::camera::{CameraProjection, RenderTargetInfo};
        use bevy::ecs::system::RunSystemOnce as _;

        use crate::rendering::camera::{PresentedViewFrame, PresentedViewFrames};
        use crate::rendering::card_view::{CardViewMode, CardViewSettings};

        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<CastBubbles>();
        let ui_scale = 1.25;
        world.insert_resource(UiScale(ui_scale));
        let bubble = CastBubble { diameter_px: 96.0, span: 100.0, rings: [[1.0; 4]; 4], backdrop: [0.1; 4] };
        world.insert_resource(CastFramingTuning { bubble: Some(bubble), ..Default::default() });
        let size = Vec2::new(1200.0, 800.0);
        let center = size * 0.5;
        insert_live_room_component(&mut world, RoomGeometry(ae::World::new("stage", size, center, Vec::new())));
        let observer = world.spawn((LocalView, LocalViewId::FIRST)).id();
        world.insert_resource(PresentedViewFrames::of_views(vec![(observer, PresentedViewFrame {
            room: LiveRoomInstance::ACTIVATION,
            plane_center: Vec2::ZERO,
            roll_radians: 0.0,
            orthographic_scale: 1.0,
        })]));
        let mut camera = Camera::default();
        camera.computed.target_info = Some(RenderTargetInfo { physical_size: (size * 2.0).as_uvec2(), scale_factor: 2.0 });
        let mut projection = OrthographicProjection::default_2d();
        projection.update(size.x, size.y);
        world.spawn((camera.clone(), Camera2d, MainCamera, PresentsView(observer),
            Transform::from_xyz(0.0, 0.0, 1000.0), Projection::Orthographic(projection)));
        world.spawn((camera, Camera2d, FrontHudCamera));
        let onstage = world.spawn(BodyPoseView { pos: center, ..Default::default() }).id();
        let fighter = world.spawn(BodyPoseView {
            pos: center + Vec2::X * 635.0,
            size: Vec2::splat(24.0),
            ..Default::default()
        }).id();
        world.insert_resource(FramedCast(vec![onstage, fighter]));
        let discs = |world: &mut World| {
            world.query_filtered::<(&Visibility, &Node), With<CastBubbleDisc>>()
                .iter(world)
                .filter(|(visibility, _)| **visibility == Visibility::Visible)
                .map(|(_, node)| (node.left, node.top))
                .collect::<Vec<_>>()
        };

        world.insert_resource(CardViewSettings::gameplay());
        world.run_system_once(present_cast_bubbles).unwrap();
        assert!(discs(&mut world).is_empty(), "the tilted output still shows part of the fighter");

        let mut cameras = world.query_filtered::<&Camera, With<CastBubbleCamera>>();
        for mode in [CardViewMode::Plane, CardViewMode::SideOn, CardViewMode::Inset] {
            world.insert_resource(CardViewSettings::for_game(mode));
            world.run_system_once(present_cast_bubbles).unwrap();
            let [at] = discs(&mut world)[..] else { panic!("one bubble in {mode:?}") };
            let (Val::Px(x), Val::Px(y)) = at else { panic!("pixel placement") };
            let expected = Vec2::new(1146.0, 400.0) - Vec2::splat(48.0 * (1.0 + RING_FRACTION));
            assert!(Vec2::new(x, y).abs_diff_eq(expected / ui_scale, 0.01), "{mode:?}: {at:?}");
            assert!(cameras.single(&world).expect("one close-up").is_active);
        }

        world.insert_resource(CardViewSettings::gameplay());
        world.get_mut::<BodyPoseView>(fighter).unwrap().pos.x += 100.0;
        world.run_system_once(present_cast_bubbles).unwrap();
        assert_eq!(discs(&mut world).len(), 1);

        world.get_mut::<BodyPoseView>(fighter).unwrap().pos.x -= 100.0;
        world.run_system_once(present_cast_bubbles).unwrap();
        assert!(discs(&mut world).is_empty());
        assert!(cameras.iter(&world).all(|camera| !camera.is_active));

        world.remove_resource::<CastFramingTuning>();
        world.run_system_once(present_cast_bubbles).unwrap();
        assert_eq!(cameras.iter(&world).count(), 0);
        assert!(world.query_filtered::<Entity, With<CastBubbleDisc>>().iter(&world).next().is_none());
    }
}
