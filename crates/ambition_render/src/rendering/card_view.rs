//! RV2: an orthographic 3D view of the same scene, drawn beside the 2D view
//! in one App.
//!
//! A second renderer over the same simulation and the same presentation
//! sample. It reads:
//! - each view's [`PresentedViewFrame`] (the camera stage's renderer-free
//!   framing), through its own camera adapter;
//! - each rigged body's [`RiggedLocalSample`] (the parts, their pivots, scale,
//!   turn and colour, and the feet in the body's own frame) and the body's
//!   scene placement (its root transform);
//! - the vessels seating publishes ([`PresentedVessels`]) and which bodies sit
//!   in one ([`SeatedInVessel`]).
//!
//! It writes only its own entities: one `Camera3d` per view, a card body per
//! rigged body, a card per drawn part, and a cart mesh per vessel. It never
//! reads the 2D slots or the 2D camera.
//!
//! # The visual-depth policy
//!
//! Logical order, physical XY scale and visual thickness are different units,
//! so the 2D z bands are not reused as depths. One declared policy places
//! everything:
//! - the play plane is z 0, and a body not in a vessel stands on it;
//! - a vessel's bed is recessed [`VESSEL_DEPTH`] behind the play plane, its
//!   near panel just behind the plane and its back panel at the far end, and
//!   a seated body stands in the middle of the bed;
//! - bodies on one depth are ordered by their logical order (the root's 2D
//!   z), one [`CARD_BODY_STEP`] apart, nearer for a later one;
//! - a body's parts are [`CARD_PART_STEP`] apart in paint order.
//!
//! # Not yet realized (RV3)
//!
//! Baked-frame bodies, held and wielded art, the hit flash, group fades
//! (a fading frame's parts are each made translucent), portals and room
//! tiles are not drawn in this view.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ScalingMode, Viewport};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::CARD_VIEW_LAYER;
use ambition_sprite_sheet::character::rigged::{PartDraw, RiggedSpriteAsset};

use super::actors::rigged::{RiggedLocalSample, RiggedPresentation, RiggedPresentationOf};
use super::camera::{PresentedViewFrame, PresentedViewFrames, ViewCameraSet};
use super::moving_platforms::{PresentedVessels, SeatedInVessel, VesselBox};

/// How deep a vessel's bed is, in world units: the pilot cart's authored
/// thickness.
pub const VESSEL_DEPTH: f32 = 40.0;
/// The gap between two bodies' card blocks on one depth.
pub const CARD_BODY_STEP: f32 = 2.5;
/// The gap between two parts of one body, in paint order.
pub const CARD_PART_STEP: f32 = 0.05;
/// The near panel's distance behind the play plane.
const NEAR_PANEL_GAP: f32 = 1.0;
/// Panel and floor thickness.
const BOARD: f32 = 3.0;
/// How far the camera stands from what it looks at.
const CAMERA_DISTANCE: f32 = 2000.0;

/// The card view's camera choices.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CardViewSettings {
    /// The camera's turn about the vertical, so depth shows.
    pub yaw_radians: f32,
    /// The camera's tilt down.
    pub pitch_radians: f32,
    /// The viewport's width as a fraction of its target's.
    pub viewport_fraction: f32,
    /// How much nearer than the 2D view it looks: 1 shows the 2D view's
    /// whole width in the smaller viewport.
    pub zoom: f32,
}

impl Default for CardViewSettings {
    fn default() -> Self {
        Self {
            yaw_radians: 35f32.to_radians(),
            pitch_radians: 12f32.to_radians(),
            viewport_fraction: 0.42,
            zoom: 2.5,
        }
    }
}

/// A view's card camera.
#[derive(Component, Debug)]
pub struct CardViewCamera;

/// The cards of one rigged body: placed where the body stands, holding one
/// [`CardPart`] per drawn part.
#[derive(Component, Debug)]
pub struct CardBody {
    pub owner: Entity,
    pub parts: Vec<Entity>,
}

/// One drawn part of a [`CardBody`].
#[derive(Component, Debug)]
pub struct CardPart;

/// The cart mesh of one vessel.
#[derive(Component, Debug)]
pub struct CardVessel;

/// The card view: install it beside the 2D presentation.
pub struct CardViewPlugin;

impl Plugin for CardViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardViewSettings>();
        app.init_resource::<CardMeshes>();
        // Seating publishes its vessels only while this resource exists.
        app.init_resource::<PresentedVessels>();
        app.add_systems(
            Update,
            (
                present_card_view_rigs,
                place_card_view_cameras,
                realize_card_bodies,
                realize_card_vessels,
            )
                .chain()
                .after(ViewCameraSet)
                .after(super::actors::rigged::RiggedDriveSet)
                .after(super::moving_platforms::seat_bodies_in_vessels),
        );
    }
}

/// Give each local view one card camera, and retire it with its view.
pub fn present_card_view_rigs(
    mut commands: Commands,
    views: Query<Entity, With<ambition_sim_view::LocalView>>,
    rigs: Query<(Entity, &ambition_sim_view::PresentsView), With<CardViewCamera>>,
) {
    for (rig, presents) in &rigs {
        if !views.contains(presents.0) {
            commands.entity(rig).try_despawn();
        }
    }
    for view in &views {
        if rigs.iter().any(|(_, presents)| presents.0 == view) {
            continue;
        }
        commands.spawn((
            CardViewCamera,
            Camera3d::default(),
            // As the TwinTrack spacetime view and the pause cube: a
            // higher-order Camera3d composited over the gameplay Camera2d,
            // its clear held to its own viewport.
            Tonemapping::None,
            Msaa::Sample4,
            Camera {
                // Above every gameplay rig (0..=8), below the front HUD (9).
                order: 8,
                // Until its target is measured (`place_card_view_cameras`).
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.07, 0.08, 0.11)),
                ..default()
            },
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize,
                near: 0.0,
                far: CAMERA_DISTANCE * 2.0,
                ..OrthographicProjection::default_3d()
            }),
            RenderLayers::layer(CARD_VIEW_LAYER),
            ambition_sim_view::PresentsView(view),
            Name::new("Card view camera"),
        ));
    }
}

/// The card renderer's camera adapter: look at the view's presented frame
/// from the settings' turn and tilt, through a viewport in the window's lower
/// right corner, showing the middle of what the 2D view shows ([`CardViewSettings::zoom`]). A
/// camera stays inactive until its target is measured, so it never covers the
/// 2D view with a whole-target clear.
pub fn place_card_view_cameras(
    frames: Res<PresentedViewFrames>,
    settings: Res<CardViewSettings>,
    mut cameras: Query<
        (&mut Camera, &mut Transform, &mut Projection, &ambition_sim_view::PresentsView),
        With<CardViewCamera>,
    >,
) {
    for (mut camera, mut transform, mut projection, presents) in &mut cameras {
        // Measured on the camera's own target (the window, or an image a
        // capture points it at), so the corner follows the target it draws.
        let viewport = camera
            .physical_target_size()
            .and_then(|size| card_viewport(size, settings.viewport_fraction));
        // Bevy measures an inactive camera's target too, so it can wait here.
        let active = viewport.is_some();
        // `Viewport` has no equality: compare its rectangle, so an unchanged
        // corner leaves the camera unchanged.
        let rect = |v: &Option<Viewport>| v.as_ref().map(|v| (v.physical_position, v.physical_size));
        if rect(&camera.viewport) != rect(&viewport) {
            camera.viewport = viewport;
        }
        if camera.is_active != active {
            camera.is_active = active;
        }
        let Some(frame) = frames.of(presents.0) else {
            continue;
        };
        *transform = card_camera_transform(frame, &settings);
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale =
                frame.orthographic_scale / (settings.viewport_fraction.max(0.05) * settings.zoom.max(0.05));
        }
    }
}

/// The card camera's place for a frame: turned and tilted about the frame's
/// centre on the play plane, rolled as the frame is.
pub fn card_camera_transform(frame: &PresentedViewFrame, settings: &CardViewSettings) -> Transform {
    let target = frame.plane_center.extend(0.0);
    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        settings.yaw_radians,
        -settings.pitch_radians,
        frame.roll_radians,
    );
    Transform::from_translation(target + rotation * Vec3::Z * CAMERA_DISTANCE).with_rotation(rotation)
}

fn card_viewport(target: UVec2, fraction: f32) -> Option<Viewport> {
    let (width, height) = (target.x, target.y);
    if width < 64 || height < 64 {
        return None;
    }
    let w = ((width as f32 * fraction) as u32).clamp(32, width);
    let h = ((w as f32 * height as f32 / width as f32) as u32).clamp(32, height);
    Some(Viewport {
        physical_position: UVec2::new(width - w, height - h),
        physical_size: UVec2::new(w, h),
        ..default()
    })
}

/// The depth of a body in the policy (module docs): the play plane, or the
/// middle of its vessel's bed, plus its rank among the bodies at that depth.
pub fn card_body_depth(seated: bool, rank: usize) -> f32 {
    let base = if seated { -VESSEL_DEPTH * 0.5 } else { 0.0 };
    base + rank as f32 * CARD_BODY_STEP
}

/// Part quads and their materials, made once per part and per colour.
#[derive(Resource, Default)]
pub struct CardMeshes {
    meshes: HashMap<(usize, u16), Handle<Mesh>>,
    materials: HashMap<(AssetId<Image>, [u8; 4], bool), Handle<StandardMaterial>>,
}

/// One part's quad: its drawn size about its pivot, in sheet pixels (+y up),
/// with the page region as its texture coordinates.
fn part_quad(flipbook: &RiggedSpriteAsset, part: u16, page_size: Vec2) -> Mesh {
    let part = flipbook.parts[usize::from(part)];
    let anchor = part.anchor();
    let (w, h) = (part.size.x, part.size.y);
    let (x0, x1) = (-(0.5 + anchor.x) * w, (0.5 - anchor.x) * w);
    let (y0, y1) = (-(0.5 + anchor.y) * h, (0.5 - anchor.y) * h);
    let (u0, u1) = (part.rect.min.x as f32 / page_size.x, part.rect.max.x as f32 / page_size.x);
    let (v0, v1) = (part.rect.min.y as f32 / page_size.y, part.rect.max.y as f32 / page_size.y);
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[x0, y0, 0.0], [x1, y0, 0.0], [x1, y1, 0.0], [x0, y1, 0.0]],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]])
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

/// A part's card place in its body: the same pivot, turn and scale as its
/// 2D slot (`slot_local`), at its paint-order depth.
pub fn card_part_local(draw: &PartDraw, index: usize) -> Transform {
    Transform::from_translation(Vec3::new(draw.at.x, -draw.at.y, index as f32 * CARD_PART_STEP))
        .with_rotation(Quat::from_rotation_z(-draw.rotation))
        .with_scale(draw.scale.extend(1.0))
}

/// Realize every sampled rigged body as cards.
#[allow(clippy::too_many_arguments)]
pub fn realize_card_bodies(
    mut commands: Commands,
    owners: Query<(Entity, &RiggedPresentationOf, &RiggedPresentation, &RiggedLocalSample)>,
    roots: Query<(&Transform, Has<SeatedInVessel>), (Without<CardBody>, Without<CardPart>)>,
    mut bodies: Query<(Entity, &mut CardBody, &mut Transform, &mut Visibility), Without<CardPart>>,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut Visibility),
        (With<CardPart>, Without<CardBody>),
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_owner: Local<HashMap<Entity, Entity>>,
    mut ranked: Local<Vec<(bool, f32, Entity)>>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    let layer = RenderLayers::layer(CARD_VIEW_LAYER);

    // A body whose owner is gone loses its cards.
    by_owner.retain(|owner, body| {
        let keep = owners.contains(*owner);
        if !keep {
            commands.entity(*body).try_despawn();
        }
        keep
    });

    // Rank the drawn bodies on each depth by their logical order.
    ranked.clear();
    for (owner, of, _, sample) in &owners {
        if sample.parts().is_none() || sample.feet.is_none() {
            continue;
        }
        if let Ok((root, seated)) = roots.get(of.0) {
            ranked.push((seated, root.translation.z, owner));
        }
    }
    ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut rank_of: HashMap<Entity, f32> = HashMap::with_capacity(ranked.len());
    let mut last: Option<bool> = None;
    let mut rank = 0usize;
    for &(seated, _, owner) in ranked.iter() {
        if last != Some(seated) {
            rank = 0;
            last = Some(seated);
        }
        rank_of.insert(owner, card_body_depth(seated, rank));
        rank += 1;
    }

    for (owner, of, presentation, sample) in &owners {
        let body = *by_owner.entry(owner).or_insert_with(|| {
            commands
                .spawn((
                    CardBody { owner, parts: Vec::new() },
                    Transform::default(),
                    Visibility::Hidden,
                    layer.clone(),
                    Name::new("Card body"),
                ))
                .id()
        });
        let Ok((_, mut card_body, mut body_transform, mut body_visibility)) = bodies.get_mut(body) else {
            // Spawned this frame: filled from the next.
            continue;
        };
        let (Some(draws), Some(feet), Some(depth), Ok((root, _))) =
            (sample.parts(), sample.feet, rank_of.get(&owner), roots.get(of.0))
        else {
            body_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        // The body's scene placement (its root), then its own feet.
        let mut place = root.mul_transform(feet);
        place.translation.z = *depth;
        body_transform.set_if_neq(place);
        body_visibility.set_if_neq(Visibility::Inherited);

        let flipbook: &Arc<RiggedSpriteAsset> = &presentation.pages.flipbook;
        let flipbook_key = Arc::as_ptr(flipbook) as usize;
        let translucent = sample.frame_opacity < 1.0;
        for (index, draw) in draws.iter().enumerate() {
            let part = flipbook.parts[usize::from(draw.part)];
            let Some(page) = presentation.pages.pages.get(usize::from(part.page)) else {
                continue;
            };
            let Some(page_size) = images.get(page).map(|image| image.size().as_vec2()) else {
                continue;
            };
            let mesh = cache
                .meshes
                .entry((flipbook_key, draw.part))
                .or_insert_with(|| meshes.add(part_quad(flipbook, draw.part, page_size)))
                .clone();
            let mut color = draw.color;
            if translucent {
                color[3] = (f32::from(color[3]) * sample.frame_opacity).round() as u8;
            }
            let blended = color[3] < 255;
            let material = cache
                .materials
                .entry((page.id(), color, blended))
                .or_insert_with(|| {
                    let tint = draw.tint();
                    materials.add(StandardMaterial {
                        base_color: Color::srgba(tint.x, tint.y, tint.z, f32::from(color[3]) / 255.0),
                        base_color_texture: Some(page.clone()),
                        unlit: true,
                        cull_mode: None,
                        double_sided: true,
                        alpha_mode: if blended { AlphaMode::Blend } else { AlphaMode::Mask(0.5) },
                        ..default()
                    })
                })
                .clone();
            if card_body.parts.len() <= index {
                let card = commands
                    .spawn((
                        CardPart,
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        card_part_local(draw, index),
                        Visibility::Inherited,
                        layer.clone(),
                        ChildOf(body),
                    ))
                    .id();
                card_body.parts.push(card);
                continue;
            }
            let Ok((mut card_mesh, mut card_material, mut transform, mut visibility)) =
                cards.get_mut(card_body.parts[index])
            else {
                continue;
            };
            if card_mesh.0 != mesh {
                card_mesh.0 = mesh;
            }
            if card_material.0 != material {
                card_material.0 = material;
            }
            transform.set_if_neq(card_part_local(draw, index));
            visibility.set_if_neq(Visibility::Inherited);
        }
        for &card in card_body.parts.iter().skip(draws.len()) {
            if let Ok((_, _, _, mut visibility)) = cards.get_mut(card) {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

/// Realize each published vessel as a cart: a floor, a back panel, two
/// sides and a near panel, recessed behind the play plane by the policy.
pub fn realize_card_vessels(
    mut commands: Commands,
    vessels: Option<Res<PresentedVessels>>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    existing: Query<Entity, With<CardVessel>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut shown: Local<Vec<VesselBox>>,
) {
    let (Some(vessels), Some(mut meshes), Some(mut materials)) = (vessels, meshes, materials) else {
        return;
    };
    if shown.as_slice() == vessels.0.as_slice() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
    shown.clone_from(&vessels.0);
    let layer = RenderLayers::layer(CARD_VIEW_LAYER);
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.36, 0.2),
        unlit: true,
        ..default()
    });
    let dark_wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.36, 0.23, 0.13),
        unlit: true,
        ..default()
    });
    for vessel in vessels.0.iter() {
        let Some(room) = rooms.in_room(vessel.room) else {
            continue;
        };
        for (center, size, material) in cart_boards(vessel, room.0.size) {
            commands.spawn((
                CardVessel,
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(if material { wood.clone() } else { dark_wood.clone() }),
                Transform::from_translation(center),
                layer.clone(),
                Name::new("Card cart board"),
            ));
        }
    }
}

/// The cart's boards for a vessel, on the Bevy plane of a room `room_size`
/// wide: (centre, size, near-side wood).
pub fn cart_boards(vessel: &VesselBox, room_size: bevy::math::Vec2) -> [(Vec3, Vec3, bool); 5] {
    let to_plane = |x: f32, y: f32| bevy::math::Vec2::new(x - room_size.x * 0.5, room_size.y * 0.5 - y);
    let width = (vessel.right - vessel.left).max(BOARD);
    let rim = vessel.rim.max(BOARD);
    let mid_x = (vessel.left + vessel.right) * 0.5;
    let bed = to_plane(mid_x, vessel.top);
    let near_z = -NEAR_PANEL_GAP - BOARD * 0.5;
    let back_z = -VESSEL_DEPTH;
    let mid_z = (near_z + back_z) * 0.5;
    let depth = near_z - back_z;
    let panel_y = bed.y + rim * 0.5;
    [
        // The floor, under the bed.
        (Vec3::new(bed.x, bed.y - BOARD * 0.5, mid_z), Vec3::new(width, BOARD, depth), false),
        // The back panel.
        (Vec3::new(bed.x, panel_y, back_z), Vec3::new(width, rim, BOARD), false),
        // The two sides.
        (Vec3::new(bed.x - width * 0.5, panel_y, mid_z), Vec3::new(BOARD, rim, depth), false),
        (Vec3::new(bed.x + width * 0.5, panel_y, mid_z), Vec3::new(BOARD, rim, depth), false),
        // The near panel, just behind the play plane.
        (Vec3::new(bed.x, panel_y, near_z), Vec3::new(width, rim, BOARD), true),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declared depth policy: a seated body is behind the cart's near
    /// panel, which is behind every body on the play plane; the back panel is
    /// behind the seated body; and later bodies on one depth are nearer.
    #[test]
    fn a_seated_body_is_between_the_cart_panels_and_others_are_in_front() {
        let vessel = VesselBox {
            room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
            left: 100.0,
            right: 300.0,
            top: 200.0,
            rim: 12.0,
        };
        let boards = cart_boards(&vessel, Vec2::new(800.0, 600.0));
        let near = boards.iter().find(|(_, _, near)| *near).unwrap();
        let back = boards[1];
        let near_front = near.0.z + near.1.z * 0.5;
        let near_back = near.0.z - near.1.z * 0.5;
        let seated = card_body_depth(true, 0);
        let free = card_body_depth(false, 0);
        assert!(back.0.z + back.1.z * 0.5 < seated, "the seated body is in front of the back panel");
        assert!(seated < near_back, "the seated body is behind the near panel");
        assert!(near_front < free, "a body on the play plane is in front of the near panel");
        assert!(card_body_depth(false, 1) > free && card_body_depth(true, 1) > seated);
        // The panel stands on the bed (y down in the world, up on the plane).
        let bed = 600.0 * 0.5 - vessel.top;
        assert!((near.0.y - near.1.y * 0.5 - bed).abs() < 1e-3);
        assert!((near.1.y - vessel.rim).abs() < 1e-3);
    }
}
