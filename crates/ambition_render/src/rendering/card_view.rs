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
//! # One picture, when the 2D view has one
//!
//! A body the 2D view composites (a fading frame, a flash, a warp, any reader
//! of it as one image) is drawn here as one card of its composited cell, as
//! the 2D root draws it: a fading frame fades as one picture, not part by
//! part. A body's flash ([`PresentedFlash`]) is a silhouette card over each of
//! its cards, in the cue's colour, as strong as the cue.
//!
//! # Not yet realized (RV3)
//!
//! # Held and wielded art
//!
//! An item in a body's hand ([`HeldArtOf`]) is a card at its 2D sprite's
//! place, on its holder's depth. Its 2D depth offset from the holder's root,
//! counted in part slots, is the same count of card part steps, so it is
//! between the same two parts here as there; an item placed at no hand (an
//! absolute 2D depth) is held to its body's own band.
//!
//! # Not yet realized (RV3)
//!
//! The gate cue's scanlines (drawn as its flat tint), portals and room tiles
//! are not drawn in this view.

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

use super::actors::rigged::{
    BakedFrame, RiggedImpostorAtlas, RiggedLocalSample, RiggedPresentation, RiggedPresentationOf,
};
use super::hit_flash::PresentedFlash;
use super::posed_hand::HeldArtOf;
use super::camera::{PresentedViewFrame, PresentedViewFrames, ViewCameraSet};
use super::moving_platforms::{PresentedVessels, SeatedInVessel, VesselBox};

/// How deep a vessel's bed is, in world units: the pilot cart's authored
/// thickness.
pub const VESSEL_DEPTH: f32 = 40.0;
/// The gap between two bodies' card blocks on one depth.
pub const CARD_BODY_STEP: f32 = 2.5;
/// The gap between two parts of one body, in paint order.
pub const CARD_PART_STEP: f32 = 0.05;
/// A flash card's distance in front of the card it covers: inside the gap to
/// the next part.
const FLASH_STEP: f32 = CARD_PART_STEP * 0.5;
/// A 2D part slot's depth step (`rigged::SLOT_DEPTH_STEP`): one part.
const SLOT_STEP_2D: f32 = 1.0e-4;
/// How far in front of or behind its body's card place a held item may be:
/// inside the body's own band ([`CARD_BODY_STEP`]).
const HELD_ART_REACH: f32 = 1.0;
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

/// A card drawing one held or wielded item.
#[derive(Component)]
pub struct CardHeldArt;

/// Each drawn rigged root's card depth and its 2D depth, this frame: where
/// its held art goes.
#[derive(Resource, Default, Debug)]
pub struct CardRootDepths(pub HashMap<Entity, (f32, f32)>);

/// The card view: install it beside the 2D presentation.
pub struct CardViewPlugin;

impl Plugin for CardViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardViewSettings>();
        app.init_resource::<CardMeshes>();
        app.init_resource::<CardRootDepths>();
        // Seating publishes its vessels only while this resource exists.
        app.init_resource::<PresentedVessels>();
        app.add_systems(
            Update,
            (
                present_card_view_rigs,
                place_card_view_cameras,
                realize_card_bodies,
                realize_card_held_art,
                realize_card_vessels,
            )
                .chain()
                .after(ViewCameraSet)
                .after(super::actors::rigged::RiggedDriveSet)
                .after(super::hit_flash::sync_hit_flash_overlays)
                .after(super::item_visuals::sync_held_item_visual)
                .after(super::wielded_item_visuals::sync_wielded_item_visuals)
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
    baked_meshes: HashMap<(AssetId<Image>, [u32; 4], [u32; 4]), Handle<Mesh>>,
    materials: HashMap<(AssetId<Image>, [u8; 4], bool), Handle<StandardMaterial>>,
    /// Silhouettes, by page and the cue's colour and strength in bytes.
    flashes: HashMap<(AssetId<Image>, [u8; 4]), Handle<StandardMaterial>>,
}

/// A flash's silhouette over a card of `page`: the page's alpha, its colour
/// replaced by the cue's (the base is black and unlit by nothing, so only the
/// emission shows), as opaque as the cue is strong.
pub fn flash_material(page: &Handle<Image>, flash: PresentedFlash) -> StandardMaterial {
    let tint = Color::srgb(flash.tint.x, flash.tint.y, flash.tint.z);
    StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, flash.intensity.clamp(0.0, 1.0)),
        base_color_texture: Some(page.clone()),
        emissive: tint.to_linear(),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        cull_mode: None,
        double_sided: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

fn flash_key(flash: PresentedFlash) -> [u8; 4] {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(flash.tint.x), byte(flash.tint.y), byte(flash.tint.z), byte(flash.intensity)]
}

/// The frame a composited root draws, as one picture: its cell of the atlas
/// page, at the root's quad. `None` while the root does not draw a cell of
/// its body's page (its page is fresh, or it is drawn part by part).
fn composited_frame(
    presentation: &RiggedPresentation,
    atlas: &RiggedImpostorAtlas,
    sprite: Option<&Sprite>,
    anchor: Option<&bevy::sprite::Anchor>,
    layouts: Option<&Assets<TextureAtlasLayout>>,
) -> Option<BakedFrame> {
    let impostor = presentation.impostor.as_ref()?;
    let page = atlas.page(impostor)?;
    let sprite = sprite.filter(|sprite| sprite.image == page.image)?;
    let cell = sprite.texture_atlas.as_ref().filter(|cell| cell.index == impostor.cell as usize)?;
    Some(BakedFrame {
        image: page.image.clone(),
        rect: *layouts?.get(&cell.layout)?.textures.get(cell.index)?,
        size: sprite.custom_size?,
        anchor: anchor.map_or(Vec2::ZERO, |anchor| anchor.0),
        flip_x: sprite.flip_x,
    })
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

fn baked_key(baked: &BakedFrame) -> (AssetId<Image>, [u32; 4], [u32; 4]) {
    (
        baked.image.id(),
        [baked.rect.min.x, baked.rect.min.y, baked.rect.max.x, baked.rect.max.y],
        [baked.size.x.to_bits(), baked.size.y.to_bits(), baked.anchor.x.to_bits(), baked.anchor.y.to_bits()],
    )
}

/// A baked frame's quad: `size` world units about its anchor, with its page
/// region as texture coordinates.
fn baked_quad(baked: &BakedFrame, page_size: Vec2) -> Mesh {
    let (w, h) = (baked.size.x, baked.size.y);
    let anchor = baked.anchor;
    let (x0, x1) = (-(0.5 + anchor.x) * w, (0.5 - anchor.x) * w);
    let (y0, y1) = (-(0.5 + anchor.y) * h, (0.5 - anchor.y) * h);
    let (u0, u1) = (baked.rect.min.x as f32 / page_size.x, baked.rect.max.x as f32 / page_size.x);
    let (v0, v1) = (baked.rect.min.y as f32 / page_size.y, baked.rect.max.y as f32 / page_size.y);
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[x0, y0, 0.0], [x1, y0, 0.0], [x1, y1, 0.0], [x0, y1, 0.0]],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]])
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

type CardQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Mesh3d, &'static mut MeshMaterial3d<StandardMaterial>, &'static mut Transform, &'static mut Visibility),
    (With<CardPart>, Without<CardBody>),
>;

/// Show `shown` as a body's cards, in order, spawning what it lacks and
/// hiding the rest.
fn show_cards(
    commands: &mut Commands,
    body: &mut CardBody,
    cards: &mut CardQuery,
    owner: Entity,
    layer: &RenderLayers,
    shown: &[(Handle<Mesh>, Handle<StandardMaterial>, Transform)],
) {
    for (index, (mesh, material, local)) in shown.iter().enumerate() {
        if body.parts.len() <= index {
            let card = commands
                .spawn((
                    CardPart,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    *local,
                    Visibility::Inherited,
                    layer.clone(),
                    ChildOf(owner),
                ))
                .id();
            body.parts.push(card);
            continue;
        }
        let Ok((mut card_mesh, mut card_material, mut transform, mut visibility)) = cards.get_mut(body.parts[index]) else {
            continue;
        };
        if card_mesh.0 != *mesh {
            card_mesh.0 = mesh.clone();
        }
        if card_material.0 != *material {
            card_material.0 = material.clone();
        }
        transform.set_if_neq(*local);
        visibility.set_if_neq(Visibility::Inherited);
    }
    for &card in body.parts.iter().skip(shown.len()) {
        if let Ok((_, _, _, mut visibility)) = cards.get_mut(card) {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
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
    roots: Query<
        (&Transform, Has<SeatedInVessel>, Option<&Sprite>, Option<&bevy::sprite::Anchor>, Option<&PresentedFlash>),
        (Without<CardBody>, Without<CardPart>),
    >,
    atlas: Option<Res<RiggedImpostorAtlas>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut bodies: Query<(Entity, &mut CardBody, &mut Transform, &mut Visibility), Without<CardPart>>,
    mut cards: CardQuery,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut cache: ResMut<CardMeshes>,
    // Where held art goes. Optional: an app with bodies alone runs without it.
    mut root_depths: Option<ResMut<CardRootDepths>>,
    mut by_owner: Local<HashMap<Entity, Entity>>,
    mut ranked: Local<Vec<(bool, f32, Entity)>>,
    mut shown: Local<Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)>>,
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
        let drawable = (sample.parts().is_some() && sample.feet.is_some()) || sample.baked.is_some();
        if !drawable {
            continue;
        }
        if let Ok((root, seated, ..)) = roots.get(of.0) {
            ranked.push((seated, root.translation.z, owner));
        }
    }
    ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut rank_of: HashMap<Entity, f32> = HashMap::with_capacity(ranked.len());
    if let Some(depths) = root_depths.as_deref_mut() {
        depths.0.clear();
    }
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
        let (Some(depth), Ok((root, _, root_sprite, root_anchor, flash))) = (rank_of.get(&owner), roots.get(of.0)) else {
            body_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        if let Some(depths) = root_depths.as_deref_mut() {
            depths.0.insert(of.0, (*depth, root.translation.z));
        }
        let flash = flash.copied().filter(|flash| flash.intensity > 0.0);
        // A baked frame is one card at the root, drawn whole; so is a body
        // the 2D view composites, as its cell (its fade is in the cell).
        let composited = sample.parts().and_then(|_| {
            composited_frame(presentation, atlas.as_deref()?, root_sprite, root_anchor, layouts.as_deref())
        });
        if let Some(baked) = sample.baked.as_ref().or(composited.as_ref()) {
            let opacity = if composited.is_some() { 1.0 } else { sample.frame_opacity };
            let mut place = *root;
            place.translation.z = *depth;
            body_transform.set_if_neq(place);
            body_visibility.set_if_neq(Visibility::Inherited);
            let Some(page_size) = images.get(&baked.image).map(|image| image.size().as_vec2()) else {
                // Not loaded yet: draw nothing rather than the last clip's parts.
                show_cards(&mut commands, &mut card_body, &mut cards, body, &layer, &[]);
                continue;
            };
            let mesh = cache
                .baked_meshes
                .entry(baked_key(baked))
                .or_insert_with(|| meshes.add(baked_quad(baked, page_size)))
                .clone();
            let opacity = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
            let material = cache
                .materials
                .entry((baked.image.id(), [255, 255, 255, opacity], opacity < 255))
                .or_insert_with(|| {
                    materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 1.0, f32::from(opacity) / 255.0),
                        base_color_texture: Some(baked.image.clone()),
                        unlit: true,
                        cull_mode: None,
                        double_sided: true,
                        alpha_mode: if opacity < 255 { AlphaMode::Blend } else { AlphaMode::Mask(0.5) },
                        ..default()
                    })
                })
                .clone();
            let local = Transform::from_scale(Vec3::new(if baked.flip_x { -1.0 } else { 1.0 }, 1.0, 1.0));
            shown.clear();
            shown.push((mesh.clone(), material, local));
            if let Some(flash) = flash {
                let silhouette = cache
                    .flashes
                    .entry((baked.image.id(), flash_key(flash)))
                    .or_insert_with(|| materials.add(flash_material(&baked.image, flash)))
                    .clone();
                shown.push((mesh, silhouette, local.with_translation(Vec3::Z * FLASH_STEP)));
            }
            show_cards(&mut commands, &mut card_body, &mut cards, body, &layer, &shown);
            continue;
        }
        let (Some(draws), Some(feet)) = (sample.parts(), sample.feet) else {
            body_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        // The body's scene placement (its root), then its own feet.
        let mut place = root.mul_transform(feet);
        place.translation.z = *depth;
        body_transform.set_if_neq(place);
        body_visibility.set_if_neq(Visibility::Inherited);

        shown.clear();
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
            let local = card_part_local(draw, index);
            if let Some(flash) = flash {
                let silhouette = cache
                    .flashes
                    .entry((page.id(), flash_key(flash)))
                    .or_insert_with(|| materials.add(flash_material(page, flash)))
                    .clone();
                shown.push((mesh.clone(), material, local));
                shown.push((mesh, silhouette, local * Transform::from_translation(Vec3::Z * FLASH_STEP)));
            } else {
                shown.push((mesh, material, local));
            }
        }
        show_cards(&mut commands, &mut card_body, &mut cards, body, &layer, &shown);
    }
}

/// A held item's card depth relative to its body's card place: its 2D depth
/// offset from the holder's root, one card part step per 2D part slot, held
/// to the body's band.
pub fn held_art_card_offset(art_z: f32, root_z: f32) -> f32 {
    ((art_z - root_z) / SLOT_STEP_2D * CARD_PART_STEP).clamp(-HELD_ART_REACH, HELD_ART_REACH)
}

/// Realize each held and wielded item sprite as a card on its holder's depth.
#[allow(clippy::too_many_arguments)]
pub fn realize_card_held_art(
    mut commands: Commands,
    art: Query<
        (Entity, &Sprite, Option<&bevy::sprite::Anchor>, &Transform, &HeldArtOf, Option<&Visibility>),
        (Without<CardBody>, Without<CardPart>, Without<CardHeldArt>),
    >,
    depths: Res<CardRootDepths>,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut Visibility),
        With<CardHeldArt>,
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_art: Local<HashMap<Entity, Entity>>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_art.retain(|art_entity, card| {
        let keep = art.contains(*art_entity);
        if !keep {
            commands.entity(*card).try_despawn();
        }
        keep
    });
    for (entity, sprite, anchor, place, HeldArtOf(root), visibility) in &art {
        let shown = visibility.is_none_or(|visibility| *visibility != Visibility::Hidden);
        let frame = shown
            .then(|| depths.0.get(root).copied())
            .flatten()
            .and_then(|depth| {
                let page_size = images.get(&sprite.image)?.size().as_vec2();
                let rect = match (&sprite.texture_atlas, sprite.rect) {
                    (Some(cell), _) => *layouts.as_deref()?.get(&cell.layout)?.textures.get(cell.index)?,
                    (None, Some(rect)) => rect.as_urect(),
                    (None, None) => URect::from_corners(UVec2::ZERO, page_size.as_uvec2()),
                };
                let frame = BakedFrame {
                    image: sprite.image.clone(),
                    rect,
                    size: sprite.custom_size.unwrap_or_else(|| rect.size().as_vec2()),
                    anchor: anchor.map_or(Vec2::ZERO, |anchor| anchor.0),
                    flip_x: sprite.flip_x,
                };
                Some((depth, frame, page_size))
            });
        let Some(((depth, root_z), frame, page_size)) = frame else {
            if let Some(card) = by_art.get(&entity) {
                if let Ok((.., mut card_visibility)) = cards.get_mut(*card) {
                    card_visibility.set_if_neq(Visibility::Hidden);
                }
            }
            continue;
        };
        let mesh = cache
            .baked_meshes
            .entry(baked_key(&frame))
            .or_insert_with(|| meshes.add(baked_quad(&frame, page_size)))
            .clone();
        let color = sprite.color.to_srgba().to_u8_array();
        let blended = color[3] < 255;
        let material = cache
            .materials
            .entry((frame.image.id(), color, blended))
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: sprite.color,
                    base_color_texture: Some(frame.image.clone()),
                    unlit: true,
                    cull_mode: None,
                    double_sided: true,
                    alpha_mode: if blended { AlphaMode::Blend } else { AlphaMode::Mask(0.5) },
                    ..default()
                })
            })
            .clone();
        let flip = Vec3::new(
            if sprite.flip_x { -1.0 } else { 1.0 },
            if sprite.flip_y { -1.0 } else { 1.0 },
            1.0,
        );
        let mut card_place = *place;
        card_place.translation.z = depth + held_art_card_offset(place.translation.z, root_z);
        card_place.scale *= flip;
        match by_art.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
            Some((mut card_mesh, mut card_material, mut transform, mut card_visibility)) => {
                if card_mesh.0 != mesh {
                    card_mesh.0 = mesh;
                }
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(card_place);
                card_visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                let card = commands
                    .spawn((
                        CardHeldArt,
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        card_place,
                        Visibility::Inherited,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                        Name::new("Card held art"),
                    ))
                    .id();
                by_art.insert(entity, card);
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

    /// RV3: a held item is a card on its holder's card depth, its 2D offset
    /// from the root kept in part steps (between the same two parts); one at
    /// an absolute 2D depth is held to the body's band; a hidden one, or one
    /// whose holder draws no card, shows none.
    #[test]
    fn held_art_is_a_card_on_its_holders_depth() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .add_systems(Update, realize_card_held_art);
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        let root = app.world_mut().spawn_empty().id();
        let no_card = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<CardRootDepths>().0.insert(root, (-20.0, 5.0));
        let item = |at: Vec3, holder| {
            (
                Sprite { image: image.clone(), custom_size: Some(Vec2::new(10.0, 4.0)), ..default() },
                Transform::from_translation(at),
                HeldArtOf(holder),
            )
        };
        // Three slots in front of the root, as a hand's item is.
        let in_hand = app.world_mut().spawn(item(Vec3::new(30.0, 40.0, 5.0 + 3.0 * SLOT_STEP_2D), root)).id();
        let at_no_hand = app.world_mut().spawn(item(Vec3::new(0.0, 0.0, 12.0), root)).id();
        let hidden = app.world_mut().spawn((item(Vec3::ZERO, root), Visibility::Hidden)).id();
        let unheld = app.world_mut().spawn(item(Vec3::ZERO, no_card)).id();
        app.update();
        let world = app.world_mut();
        let mut cards = world.query_filtered::<(&Transform, &Visibility), With<CardHeldArt>>();
        let mut places: Vec<Vec3> = cards
            .iter(world)
            .filter(|(_, visibility)| **visibility != Visibility::Hidden)
            .map(|(place, _)| place.translation)
            .collect();
        places.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert_eq!(places.len(), 2, "the hidden item and the one whose holder has no card show none");
        assert_eq!(places[0], Vec3::new(0.0, 0.0, -20.0 + HELD_ART_REACH), "held to the body's band");
        assert!((places[1] - Vec3::new(30.0, 40.0, -20.0 + 3.0 * CARD_PART_STEP)).length() < 1e-4, "three part steps in front");
        let _ = (in_hand, at_no_hand, hidden, unheld);
    }

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
