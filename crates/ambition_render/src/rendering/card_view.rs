//! Orthographic card outputs for the same local views used by the 2D renderer.
//!
//! The card output reads the shared view frames and rig samples. During the
//! rollout, the world bridge also uses textures, transforms, and masks from
//! the 2D preparation path. Each backend owns its drawing entities.
//!
//! Each card output has its own camera and visibility mask. The gameplay
//! output uses the plane camera's viewport and target. The comparison output
//! draws both backends from one observer frame.
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
//! # Whole-body cards
//!
//! The card view requests one image for each sampled rigged player. It draws
//! the atlas cell at the body's depth and blends the cell's alpha like a
//! sprite. A body composited for a fade, flash, or warp uses the same path.
//! Direct part cards cover frames whose cell is not ready or cannot fit the
//! pose. A flash ([`PresentedFlash`]) adds a silhouette over each visible card.
//!
//! # Bodies drawn from a sheet
//!
//! A body with no rigged presentation (an actor or a player the 2D view
//! draws from a sheet cell) is one card of the cell it draws, ranked with the
//! rigged bodies on its depth by the same logical order.
//!
//! # The room's terrain
//!
//! The skin of a room's blocks ([`TerrainSkinMesh`], what the 2D view draws
//! as its tiles) and the earth under its lines ([`ChainEarth`], a hill's
//! fill) are the same meshes here, pictures on the play plane just behind
//! its bodies ([`TERRAIN_DEPTH`]), the earth behind the block skin and the
//! skin's layers in their 2D order. A
//! body recessed behind the play plane (a seated one) is behind it, as the
//! policy places everything behind that plane.
//!
//! # Held and wielded art
//!
//! An item in a body's hand ([`HeldArtOf`]) is a card at its 2D sprite's
//! place, on its holder's depth. Its 2D depth offset from the holder's root,
//! counted in part slots, is the same count of card part steps, so it is
//! between the same two parts here as there; an item placed at no hand (an
//! absolute 2D depth) is held to its body's own band.
//!
//! # Room scenery
//!
//! A room's props ([`PropVisual`]) and the trim pieces on its lines
//! ([`ChainTurfPiece`]) are cards of the cells the 2D view draws, at their
//! 2D places, in the 2D view's order: backdrop props behind a vessel's back
//! panel ([`BACKDROP_DEPTH`]); the turf just in front of the block skin
//! ([`TURF_DEPTH`]); scenery props in front of the turf and behind the
//! portal line ([`SCENERY_DEPTH`]), in their 2D order; and what the 2D view
//! draws in front of its bodies (a prop that hides a body, a near lane's
//! turf) one [`CARD_BODY_STEP`] in front of the front-most body. A prop
//! drawn as a body is a body here.
//!
//! # The parallax backdrop
//!
//! The room's parallax panels ([`ParallaxLayerVisual`]) are drawn for each
//! view by the 2D view's own rule, relative to the view's centre. Here each
//! panel the 2D view draws for a view is a card facing that view's card
//! camera, far behind the play plane (a foreground layer: in front of it),
//! at its 2D offset from the view's centre scaled as the card view scales
//! the view (`1 / zoom`). The card view frames the backdrop as the 2D view
//! does, however its camera turns.
//!
//! # Portals
//!
//! A portal is its line of light here too: a thin card along its opening in
//! the colour the 2D glow draws its room side in, opening and dissolving as
//! the glow does ([`ambition_portal2d_presentation::PortalGlow`]), on the
//! play plane in front of the terrain and behind the bodies. What a portal
//! shows of its far side is not drawn in this view (P5/P6).
//!
//! # Not yet realized (RV3)
//!
//! The gate cue's scanlines (drawn as its flat tint) and a portal's far side
//! are not drawn in this view.

mod gameplay;
mod labels;
#[cfg(feature = "portal_render")]
mod portal_windows;

use gameplay::CardScope;
use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ScalingMode, Viewport};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::CARD_VIEW_LAYER;
use ambition_platformer2d_shared_tangle::feature_kind::{BoundFeatureKind, FeatureVisualKind};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, PlayerVisual};
use ambition_sprite_sheet::character::rigged::{ComposedBodyDemand, PartDraw};

use super::retained_entities::RetainedEntities;
use super::actors::rigged::{
    drive_rigged_presentations, sample_rigged_presentations, BakedFrame, RiggedImpostorAtlas,
    RiggedLocalSample, RiggedPresentation, RiggedPresentationOf,
};
use super::hit_flash::PresentedFlash;
use super::posed_hand::HeldArtOf;
use super::camera::{PresentedViewFrame, PresentedViewFrames, ViewCameraSet};
use super::moving_platforms::{PresentedVessels, SeatedInVessel, VesselBox, VesselOccurrence};
use super::chain_skin::{ChainEarth, ChainTurfPiece};
use super::parallax::{ParallaxLayerVisual, FOREGROUND_PARALLAX_Z};
use super::primitives::PropVisual;
use super::terrain_skin::TerrainSkinMesh;

/// How deep a vessel's bed is, in world units: the pilot cart's authored
/// thickness.
pub const VESSEL_DEPTH: f32 = 40.0;
/// The gap between two bodies' card blocks on one depth.
pub const CARD_BODY_STEP: f32 = 2.5;
const BODY_ORDER_RANGE: f32 = 12.0;
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
/// The room's terrain picture: just behind the bodies on the play plane, in
/// front of a vessel's near panel.
pub const TERRAIN_DEPTH: f32 = -0.5;
/// How far the earth under a line is behind the block skin (its 2D band is
/// under the blocks').
const EARTH_BEHIND_SKIN: f32 = 0.1;
/// A portal's line of light: in front of the terrain, behind the bodies.
pub const PORTAL_DEPTH: f32 = -0.25;
/// A line's turf: in front of the block skin's layers, behind the scenery.
pub const TURF_DEPTH: f32 = TERRAIN_DEPTH + 0.1;
/// The back of the scenery props' band: in front of the turf, behind the
/// portal line. A prop's place in the band is its 2D order.
pub const SCENERY_DEPTH: f32 = TERRAIN_DEPTH + 0.15;
/// Backdrop props: behind a vessel's back panel.
pub const BACKDROP_DEPTH: f32 = -(VESSEL_DEPTH + 5.0);
/// How much card depth one unit of 2D depth inside a band is.
const BAND_ORDER_SCALE: f32 = 0.1;
/// How far behind the play plane, along a card camera's view, the parallax
/// backdrop stands; a layer further back in 2D is further back here.
const BACKDROP_DISTANCE: f32 = 1500.0;
/// How far in front of the play plane a foreground parallax layer stands.
const FOREGROUND_DISTANCE: f32 = 1000.0;
/// The width of a portal's line of light, in world units.
pub const PORTAL_LINE_WIDTH: f32 = 3.0;
/// The near panel's distance behind the play plane.
const NEAR_PANEL_GAP: f32 = 1.0;
/// Panel and floor thickness.
const BOARD: f32 = 3.0;
/// How far the camera stands from what it looks at.
const CAMERA_DISTANCE: f32 = 2000.0;

/// The card view's camera choices.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CardViewSettings {
    pub mode: CardViewMode,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardViewMode {
    #[default]
    Inset,
    Plane,
    SideOn,
    Gameplay,
}

impl CardViewMode {
    pub fn next(self) -> Self {
        match self {
            Self::Plane => Self::SideOn,
            Self::SideOn => Self::Gameplay,
            Self::Gameplay => Self::Inset,
            Self::Inset => Self::Plane,
        }
    }

    fn replaces_plane(self) -> bool {
        matches!(self, Self::SideOn | Self::Gameplay)
    }
}

impl CardViewSettings {
    pub fn gameplay() -> Self {
        Self {
            mode: CardViewMode::Gameplay,
            yaw_radians: 18f32.to_radians(),
            pitch_radians: 9f32.to_radians(),
            viewport_fraction: 1.0,
            zoom: 1.0,
        }
    }

    pub fn for_game(mode: CardViewMode) -> Self {
        Self { mode, viewport_fraction: 0.48, ..Self::gameplay() }
    }
}

impl Default for CardViewSettings {
    fn default() -> Self {
        Self {
            mode: CardViewMode::Inset,
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

/// The card of one mesh of a room's terrain skin.
#[derive(Component)]
pub struct CardTerrain;

/// The card of one portal's line of light.
#[derive(Component)]
pub struct CardPortal;

/// The card of a body drawn from a sheet (no rigged presentation).
#[derive(Component)]
pub struct CardSpriteBody;

/// The card of a room prop or a piece of a line's turf.
#[derive(Component)]
pub struct CardScenery;

/// The card of one parallax panel for one card camera.
#[derive(Component)]
pub struct CardBackdrop;

/// A 2D sprite that draws a whole body from a sheet: a player or an actor.
type SpriteBodyFilter = (
    With<Sprite>,
    Or<(With<PlayerVisual>, With<BoundFeatureKind>)>,
    Without<CardBody>,
    Without<CardPart>,
    Without<CardSpriteBody>,
);

/// Each drawn rigged root's card depth and its 2D depth, this frame: where
/// its held art goes.
#[derive(Resource, Default, Debug)]
pub struct CardRootDepths {
    roots: HashMap<Entity, CardBodyDepth>,
    fronts: HashMap<Option<LiveRoomInstance>, f32>,
}

#[derive(Clone, Copy, Debug)]
struct CardBodyDepth {
    plane_z: f32,
    source_z: f32,
    order_scale: f32,
}

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
            request_player_cards
                .run_if(card_outputs_enabled)
                .in_set(super::actors::rigged::RiggedDriveSet)
                .after(sample_rigged_presentations)
                .before(drive_rigged_presentations),
        );
        app.add_systems(
            Update,
            (
                present_card_view_rigs,
                place_card_view_cameras,
                realize_card_bodies,
                realize_card_sprite_bodies,
                realize_card_held_art,
                realize_card_vessels,
                realize_card_terrain,
                realize_card_scenery,
            )
                .chain()
                .run_if(card_outputs_enabled)
                .after(ViewCameraSet)
                .after(super::actors::sync_visuals)
                .after(super::actors::rigged::RiggedDriveSet)
                .after(super::hit_flash::sync_hit_flash_overlays)
                .after(super::item_visuals::sync_held_item_visual)
                .after(super::wielded_item_visuals::sync_wielded_item_visuals)
                .after(super::moving_platforms::seat_bodies_in_vessels),
        );
        app.add_systems(
            Update,
            realize_card_backdrop
                .run_if(card_outputs_enabled)
                .after(place_card_view_cameras)
                .after(super::parallax::sync_parallax_layers),
        );
        app.add_systems(
            Update,
            labels::realize_card_labels
                .run_if(card_outputs_enabled)
                .after(realize_card_bodies)
                .after(super::label_layout::WorldLabelLayoutSet),
        );
        app.add_systems(
            PostUpdate,
            (retire_card_outputs, gameplay::select_gameplay_cameras)
                .chain()
                .before(bevy::camera::CameraUpdateSystems),
        );
        app.add_systems(
            PostUpdate,
            labels::prepare_label_layout
                .after(gameplay::select_gameplay_cameras)
                .after(super::view_isolation::isolate_live_rooms)
                .before(bevy::camera::CameraUpdateSystems),
        );
        app.add_systems(
            PostUpdate,
            gameplay::isolate_cards
                .after(super::view_isolation::isolate_live_rooms)
                .after(gameplay::select_gameplay_cameras)
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                .run_if(card_outputs_enabled),
        );
        #[cfg(feature = "portal_render")]
        app.add_systems(
            Update,
            (
                realize_card_portals.after(ambition_portal2d_presentation::sync_portal_glows),
                portal_windows::realize_portal_windows.after(ambition_portal2d_presentation::sync_portal_view_cones),
            )
                .run_if(card_outputs_enabled)
                .after(ViewCameraSet),
        );
    }
}

fn card_outputs_enabled(settings: Res<CardViewSettings>) -> bool {
    settings.mode != CardViewMode::Plane
}

/// Ask for a whole image after sampling so offscreen players do not allocate cells.
pub(super) fn request_player_cards(
    players: Query<(), With<PlayerVisual>>,
    poses: Query<(&RiggedPresentationOf, &RiggedLocalSample)>,
    mut demand: ResMut<ComposedBodyDemand>,
) {
    for (owner, pose) in &poses {
        if players.contains(owner.0) && pose.parts().is_some() {
            demand.declare(owner.0);
        }
    }
}

fn retire_card_outputs(
    settings: Res<CardViewSettings>,
    mut commands: Commands,
    outputs: Query<Entity, Or<(With<CardScope>, With<CardBody>, With<CardViewCamera>)>>,
    mut meshes: ResMut<CardMeshes>,
) {
    if settings.mode != CardViewMode::Plane || outputs.is_empty() {
        return;
    }
    for output in &outputs {
        commands.entity(output).try_despawn();
    }
    *meshes = CardMeshes::default();
}

/// Give each local view one card camera, and retire it with its view.
pub fn present_card_view_rigs(
    mut commands: Commands,
    views: Query<(Entity, &ambition_sim_view::LocalViewId), With<ambition_sim_view::LocalView>>,
    settings: Res<CardViewSettings>,
    rigs: Query<(Entity, &ambition_sim_view::PresentsView), With<CardViewCamera>>,
) {
    let shown = |id: &ambition_sim_view::LocalViewId| {
        settings.mode != CardViewMode::Inset || *id == ambition_sim_view::LocalViewId::FIRST
    };
    for (rig, presents) in &rigs {
        if !views.get(presents.0).is_ok_and(|(_, id)| shown(id)) {
            commands.entity(rig).try_despawn();
        }
    }
    for (view, id) in &views {
        if !shown(id) || rigs.iter().any(|(_, presents)| presents.0 == view) {
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
                // Below the pause scrim (7) and the front HUD (9).
                order: 6,
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
    plane: Query<
        (&Camera, Option<&ambition_sim_view::PresentsView>),
        (With<Camera2d>, With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>, Without<CardViewCamera>),
    >,
    mut cameras: Query<
        (&mut Camera, &mut Transform, &mut Projection, &ambition_sim_view::PresentsView),
        With<CardViewCamera>,
    >,
) {
    for (mut camera, mut transform, mut projection, presents) in &mut cameras {
        let frame = frames.of(presents.0);
        if settings.mode == CardViewMode::Inset {
            let plane_viewport = plane.iter().find_map(|(camera, view)| {
                (view.is_none_or(|view| view.0 == presents.0))
                    .then_some(camera.viewport.as_ref()).flatten()
            });
            let size = plane_viewport.map(|v| v.physical_size).or_else(|| camera.physical_target_size());
            let viewport = size.and_then(|size| card_viewport(size, settings.viewport_fraction))
                .map(|mut viewport| {
                    if let Some(plane) = plane_viewport {
                        viewport.physical_position += plane.physical_position;
                    }
                    viewport
                });
            let active = viewport.is_some() && frame.is_some();
            let rect = |v: &Option<Viewport>| v.as_ref().map(|v| (v.physical_position, v.physical_size));
            if rect(&camera.viewport) != rect(&viewport) {
                camera.viewport = viewport;
            }
            if camera.order != 6 || camera.is_active != active {
                camera.order = 6;
                camera.is_active = active;
                camera.clear_color = ClearColorConfig::None;
            }
        }
        let Some(frame) = frame else {
            continue;
        };
        *transform = card_camera_transform(frame, &settings);
        if let Projection::Orthographic(orthographic) = &mut *projection {
            let inset_scale = if settings.mode == CardViewMode::Inset {
                (settings.viewport_fraction * settings.zoom).max(0.05)
            } else {
                1.0
            };
            orthographic.scale = frame.orthographic_scale / inset_scale;
        }
    }
}

/// The card camera's place for a frame: turned and tilted about the frame's
/// centre on the play plane, rolled as the frame is.
pub fn card_camera_transform(frame: &PresentedViewFrame, settings: &CardViewSettings) -> Transform {
    let target = frame.plane_center.extend(0.0);
    let (yaw, pitch) = if settings.mode == CardViewMode::SideOn {
        (0.0, 0.0)
    } else {
        (settings.yaw_radians, settings.pitch_radians)
    };
    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, -pitch, frame.roll_radians);
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
    if seated {
        -VESSEL_DEPTH * 0.5 - (rank as f32 * CARD_BODY_STEP).min(VESSEL_DEPTH * 0.25)
    } else {
        rank as f32 * CARD_BODY_STEP
    }
}

/// Part quads and their materials, made once per part and per colour.
#[derive(Resource, Default)]
pub struct CardMeshes {
    meshes: HashMap<QuadKey, Handle<Mesh>>,
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
        rect: layouts?.get(&cell.layout)?.textures.get(cell.index)?.as_rect(),
        size: sprite.custom_size?,
        anchor: anchor.map_or(Vec2::ZERO, |anchor| anchor.0),
        flip_x: sprite.flip_x,
        flip_y: sprite.flip_y,
    })
}

#[derive(Clone, Copy)]
struct TextureQuad {
    rect: Rect,
    page: Vec2,
    size: Vec2,
    anchor: Vec2,
    flip: [bool; 2],
}

#[derive(PartialEq, Eq, Hash)]
struct QuadKey {
    geometry: [u32; 10],
    flip: [bool; 2],
}

impl TextureQuad {
    fn baked(frame: &BakedFrame, page: Vec2) -> Self {
        Self {
            rect: frame.rect,
            page,
            size: frame.size,
            anchor: frame.anchor,
            flip: [frame.flip_x, frame.flip_y],
        }
    }

    fn key(self) -> QuadKey {
        let [x0, y0] = self.rect.min.to_array();
        let [x1, y1] = self.rect.max.to_array();
        let [w, h] = self.size.to_array();
        let [ax, ay] = self.anchor.to_array();
        let [pw, ph] = self.page.to_array();
        QuadKey {
            geometry: [x0, y0, x1, y1, w, h, ax, ay, pw, ph].map(f32::to_bits),
            flip: self.flip,
        }
    }

    fn vertices(self) -> ([[f32; 3]; 4], [[f32; 2]; 4]) {
        let [x0, y0] = ((-Vec2::splat(0.5) - self.anchor) * self.size).to_array();
        let [x1, y1] = ((Vec2::splat(0.5) - self.anchor) * self.size).to_array();
        let [mut u0, mut v0] = (self.rect.min / self.page).to_array();
        let [mut u1, mut v1] = (self.rect.max / self.page).to_array();
        if self.flip[0] {
            std::mem::swap(&mut u0, &mut u1);
        }
        if self.flip[1] {
            std::mem::swap(&mut v0, &mut v1);
        }
        (
            [[x0, y0, 0.0], [x1, y0, 0.0], [x1, y1, 0.0], [x0, y1, 0.0]],
            [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        )
    }

    fn mesh(self) -> Mesh {
        let (positions, uvs) = self.vertices();
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec())
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    }
}

impl CardMeshes {
    fn quad(&mut self, quad: TextureQuad, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.meshes.entry(quad.key()).or_insert_with(|| meshes.add(quad.mesh())).clone()
    }

    fn textured_material(
        &mut self,
        texture: &Handle<Image>,
        color: [u8; 4],
        blend: bool,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.materials.entry((texture.id(), color, blend)).or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::srgba_u8(color[0], color[1], color[2], color[3]),
                base_color_texture: Some(texture.clone()),
                unlit: true,
                cull_mode: None,
                double_sided: true,
                alpha_mode: if blend { AlphaMode::Blend } else { AlphaMode::Mask(0.5) },
                ..default()
            })
        }).clone()
    }
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
    source: Entity,
    layer: &RenderLayers,
    shown: &[(Handle<Mesh>, Handle<StandardMaterial>, Transform)],
) {
    for (index, (mesh, material, local)) in shown.iter().enumerate() {
        if body.parts.get(index).is_none_or(|entity| commands.get_entity(*entity).is_err()) {
            let card = commands
                .spawn((
                    CardPart,
                    CardScope::Source(source),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    *local,
                    Visibility::Inherited,
                    layer.clone(),
                    ChildOf(owner),
                ))
                .id();
            if let Some(slot) = body.parts.get_mut(index) {
                *slot = card;
            } else {
                body.parts.push(card);
            }
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
    super::actors::rigged::part_local_transform(draw, index as f32 * CARD_PART_STEP)
}

#[derive(Default)]
pub(super) struct CardBodyScratch {
    by_owner: RetainedEntities<Entity>,
    ranked: Vec<(Option<LiveRoomInstance>, f32, Entity, Option<usize>)>,
    rank_of: HashMap<Entity, CardBodyDepth>,
    shown: Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)>,
}

/// Realize every sampled rigged body as cards.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_bodies(
    mut commands: Commands,
    owners: Query<(Entity, &RiggedPresentationOf, &RiggedPresentation, &RiggedLocalSample)>,
    roots: Query<
        (&Transform, Option<&SeatedInVessel>, Option<&Sprite>, Option<&bevy::sprite::Anchor>, Option<&PresentedFlash>),
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
    // Where held art goes, and sprite bodies. Optional: an app with rigged
    // bodies alone runs without it.
    mut root_depths: Option<ResMut<CardRootDepths>>,
    // Bodies drawn from a sheet, ranked with these on each depth.
    sprite_bodies: Query<
        (Entity, &Transform, Option<&SeatedInVessel>, Option<&BoundFeatureKind>, Option<&Visibility>),
        SpriteBodyFilter,
    >,
    rooms: LiveRooms,
    mut scratch: Local<CardBodyScratch>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    let layer = RenderLayers::layer(CARD_VIEW_LAYER);

    let CardBodyScratch { by_owner, ranked, rank_of, shown } = &mut *scratch;
    by_owner.reconcile(&mut commands, |owner| owners.contains(*owner));

    // Rank the drawn bodies on each depth by their logical order.
    ranked.clear();
    for (owner, of, _, sample) in &owners {
        let drawable = (sample.parts().is_some() && sample.feet.is_some()) || sample.baked.is_some();
        if !drawable {
            continue;
        }
        if let Ok((root, seated, ..)) = roots.get(of.0) {
            ranked.push((rooms.of(of.0), root.translation.z, owner, seated.map(|seat| seat.slot)));
        }
    }
    let rigged_roots: Vec<Entity> = owners.iter().map(|(_, of, ..)| of.0).collect();
    for (entity, place, seated, kind, visibility) in &sprite_bodies {
        let body = kind.is_none_or(|kind| kind.kind == FeatureVisualKind::Actor);
        let shown = visibility.is_none_or(|visibility| *visibility != Visibility::Hidden);
        if body && shown && !rigged_roots.contains(&entity) {
            ranked.push((rooms.of(entity), place.translation.z, entity, seated.map(|seat| seat.slot)));
        }
    }
    ranked.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
    rank_of.clear();
    if let Some(depths) = root_depths.as_deref_mut() {
        depths.roots.clear();
        depths.fronts.clear();
    }
    for row in ranked.chunk_by(|a, b| a.0 == b.0) {
        let count = row.iter().filter(|(_, _, _, seat)| seat.is_none()).count();
        let scale = (BODY_ORDER_RANGE / card_body_depth(false, count.max(1))).min(1.0);
        let mut rank = 0;
        for &(room, source_z, owner, seat) in row {
            let (plane_z, order_scale) = if let Some(seat) = seat {
                (card_body_depth(true, seat), 1.0)
            } else {
                let depth = card_body_depth(false, rank) * scale;
                rank += 1;
                if let Some(depths) = root_depths.as_deref_mut() {
                    depths.fronts.insert(room, depth + CARD_BODY_STEP * scale);
                }
                (depth, scale)
            };
            rank_of.insert(owner, CardBodyDepth { plane_z, source_z, order_scale });
        }
    }
    if let Some(depths) = root_depths.as_deref_mut() {
        for (entity, ..) in &sprite_bodies {
            if let Some(depth) = rank_of.get(&entity) {
                depths.roots.insert(entity, *depth);
            }
        }
    }

    for (owner, of, presentation, sample) in &owners {
        let body = match by_owner.get(&owner) {
            Some(&body) => body,
            None => {
                let body = commands.spawn((
                    CardBody { owner, parts: Vec::new() },
                    Transform::default(),
                    Visibility::Hidden,
                    layer.clone(),
                    Name::new("Card body"),
                )).id();
                by_owner.insert(owner, body);
                body
            }
        };
        let Ok((_, mut card_body, mut body_transform, mut body_visibility)) = bodies.get_mut(body) else {
            // Spawned this frame: filled from the next.
            continue;
        };
        let (Some(depth), Ok((root, _, root_sprite, root_anchor, flash))) = (rank_of.get(&owner), roots.get(of.0)) else {
            body_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        if let Some(depths) = root_depths.as_deref_mut() {
            depths.roots.insert(of.0, *depth);
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
            place.translation.z = depth.plane_z;
            body_transform.set_if_neq(place);
            body_visibility.set_if_neq(Visibility::Inherited);
            let Some(page_size) = images.get(&baked.image).map(|image| image.size().as_vec2()) else {
                // Not loaded yet: draw nothing rather than the last clip's parts.
                show_cards(&mut commands, &mut card_body, &mut cards, body, of.0, &layer, &[]);
                continue;
            };
            let mesh = cache.quad(TextureQuad::baked(baked, page_size), &mut meshes);
            let opacity = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
            let color = if let Some(sprite) = root_sprite.filter(|_| composited.is_some()) {
                sprite.color.to_srgba().to_u8_array()
            } else {
                [255, 255, 255, opacity]
            };
            let material = cache.textured_material(
                &baked.image, color, composited.is_some() || opacity < 255, &mut materials,
            );
            let local = Transform::IDENTITY;
            shown.clear();
            shown.push((mesh.clone(), material, local));
            if let Some(flash) = flash {
                let silhouette = cache
                    .flashes
                    .entry((baked.image.id(), flash_key(flash)))
                    .or_insert_with(|| materials.add(flash_material(&baked.image, flash)))
                    .clone();
                shown.push((mesh, silhouette, local.with_translation(Vec3::Z * FLASH_STEP * depth.order_scale)));
            }
            show_cards(&mut commands, &mut card_body, &mut cards, body, of.0, &layer, &shown);
            continue;
        }
        let (Some(draws), Some(feet)) = (sample.parts(), sample.feet) else {
            body_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        // The body's scene placement (its root), then its own feet.
        let mut place = root.mul_transform(feet);
        place.translation.z = depth.plane_z;
        body_transform.set_if_neq(place);
        body_visibility.set_if_neq(Visibility::Inherited);

        shown.clear();
        let flipbook = &presentation.pages.flipbook;
        let translucent = sample.frame_opacity < 1.0;
        for (index, draw) in draws.iter().enumerate() {
            let part = flipbook.parts[usize::from(draw.part)];
            let Some(page) = presentation.pages.pages.get(usize::from(part.page)) else {
                continue;
            };
            let Some(page_size) = images.get(page).map(|image| image.size().as_vec2()) else {
                continue;
            };
            let mesh = cache.quad(TextureQuad {
                rect: part.rect.as_rect(),
                page: page_size,
                size: part.size,
                anchor: part.anchor(),
                flip: [false; 2],
            }, &mut meshes);
            let mut color = draw.color;
            if translucent {
                color[3] = (f32::from(color[3]) * sample.frame_opacity).round() as u8;
            }
            let blended = color[3] < 255;
            let material = cache.textured_material(page, color, blended, &mut materials);
            let mut local = card_part_local(draw, index);
            local.translation.z *= depth.order_scale;
            if let Some(flash) = flash {
                let silhouette = cache
                    .flashes
                    .entry((page.id(), flash_key(flash)))
                    .or_insert_with(|| materials.add(flash_material(page, flash)))
                    .clone();
                shown.push((mesh.clone(), material, local));
                shown.push((mesh, silhouette, local * Transform::from_translation(Vec3::Z * FLASH_STEP * depth.order_scale)));
            } else {
                shown.push((mesh, material, local));
            }
        }
        show_cards(&mut commands, &mut card_body, &mut cards, body, of.0, &layer, &shown);
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
fn realize_card_held_art(
    mut commands: Commands,
    art: Query<
        (Entity, &Sprite, Option<&bevy::sprite::Anchor>, &Transform, &HeldArtOf, Option<&Visibility>),
        (Without<CardBody>, Without<CardPart>, Without<CardHeldArt>),
    >,
    depths: Res<CardRootDepths>,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut CardScope, &mut Visibility),
        With<CardHeldArt>,
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_art: Local<RetainedEntities<Entity>>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_art.reconcile(&mut commands, |art_entity| art.contains(*art_entity));
    for (entity, sprite, anchor, place, HeldArtOf(root), visibility) in &art {
        let shown = visibility.is_none_or(|visibility| *visibility != Visibility::Hidden);
        let frame = shown
            .then(|| depths.roots.get(root).copied())
            .flatten()
            .and_then(|depth| Some((depth, sprite_frame(sprite, anchor, &images, layouts.as_deref())?)));
        let Some((depth, (frame, page_size))) = frame else {
            if let Some(card) = by_art.get(&entity) {
                if let Ok((.., mut card_visibility)) = cards.get_mut(*card) {
                    card_visibility.set_if_neq(Visibility::Hidden);
                }
            }
            continue;
        };
        let (mesh, material) = sprite_card(sprite, &frame, page_size, false, &mut meshes, &mut materials, &mut cache);
        let card_place = sprite_card_place(place, depth.plane_z + held_art_card_offset(place.translation.z, depth.source_z) * depth.order_scale);
        match by_art.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
            Some((mut card_mesh, mut card_material, mut transform, mut scope, mut card_visibility)) => {
                if card_mesh.0 != mesh {
                    card_mesh.0 = mesh;
                }
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(card_place);
                scope.set_if_neq(CardScope::Source(*root));
                card_visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                let card = commands
                    .spawn((
                        CardHeldArt,
                        CardScope::Source(*root),
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

/// The page cell a 2D sprite draws, as a card frame, and its page's size.
/// `None` until the page is loaded.
fn sprite_frame(
    sprite: &Sprite,
    anchor: Option<&bevy::sprite::Anchor>,
    images: &Assets<Image>,
    layouts: Option<&Assets<TextureAtlasLayout>>,
) -> Option<(BakedFrame, Vec2)> {
    let page_size = images.get(&sprite.image)?.size().as_vec2();
    let frame_rect = if let Some(cell) = &sprite.texture_atlas {
        layouts?.get(&cell.layout)?.textures.get(cell.index)?.as_rect()
    } else {
        Rect::from_corners(Vec2::ZERO, page_size)
    };
    let rect = sprite.rect.map_or(frame_rect, |crop| {
        Rect::from_corners(frame_rect.min + crop.min, frame_rect.min + crop.max)
    });
    let frame = BakedFrame {
        image: sprite.image.clone(),
        rect,
        size: sprite.custom_size.unwrap_or_else(|| rect.size()),
        anchor: anchor.map_or(Vec2::ZERO, |anchor| anchor.0),
        flip_x: sprite.flip_x,
        flip_y: sprite.flip_y,
    };
    Some((frame, page_size))
}

/// A sprite's card quad and material, made once per cell and per colour.
fn sprite_card(
    sprite: &Sprite,
    frame: &BakedFrame,
    page_size: Vec2,
    blend: bool,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut CardMeshes,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let mesh = cache.quad(TextureQuad::baked(frame, page_size), meshes);
    let color = sprite.color.to_srgba().to_u8_array();
    let blended = blend || color[3] < 255;
    let material = cache.textured_material(&frame.image, color, blended, materials);
    (mesh, material)
}

fn sprite_card_place(place: &Transform, z: f32) -> Transform {
    place.with_translation(place.translation.truncate().extend(z))
}

/// Realize each body drawn from a sheet (no rigged presentation) as one card
/// of the cell it draws, at its 2D place on the depth `realize_card_bodies`
/// ranked it to.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_sprite_bodies(
    mut commands: Commands,
    bodies: Query<(Entity, &Sprite, Option<&bevy::sprite::Anchor>, &Transform), SpriteBodyFilter>,
    depths: Res<CardRootDepths>,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut Visibility),
        (With<CardSpriteBody>, Without<Sprite>),
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_body: Local<RetainedEntities<Entity>>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_body.reconcile(&mut commands, |body| bodies.contains(*body));
    for (entity, sprite, anchor, place) in &bodies {
        // Ranked only when it is a shown body with no rigged presentation.
        let drawn = depths.roots.get(&entity)
            .and_then(|depth| Some((depth.plane_z, sprite_frame(sprite, anchor, &images, layouts.as_deref())?)));
        let Some((depth, (frame, page_size))) = drawn else {
            if let Some(card) = by_body.get(&entity) {
                if let Ok((.., mut card_visibility)) = cards.get_mut(*card) {
                    card_visibility.set_if_neq(Visibility::Hidden);
                }
            }
            continue;
        };
        let (mesh, material) = sprite_card(sprite, &frame, page_size, false, &mut meshes, &mut materials, &mut cache);
        let card_place = sprite_card_place(place, depth);
        match by_body.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
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
                        CardSpriteBody,
                        CardScope::Source(entity),
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        card_place,
                        Visibility::Inherited,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                        Name::new("Card sprite body"),
                    ))
                    .id();
                by_body.insert(entity, card);
            }
        }
    }
}

/// Realize each mesh of a room's terrain skin as a card of the same mesh on
/// the play plane ([`TERRAIN_DEPTH`]), its skin layer's 2D depth kept as its
/// order among the terrain.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_terrain(
    mut commands: Commands,
    terrain: Query<
        (
            Entity,
            &Mesh2d,
            &bevy::sprite_render::MeshMaterial2d<bevy::sprite_render::ColorMaterial>,
            &Transform,
            &GlobalTransform,
            Option<&InheritedVisibility>,
            Has<ChainEarth>,
        ),
        (Or<(With<TerrainSkinMesh>, With<ChainEarth>)>, Without<CardTerrain>),
    >,
    mut cards: Query<(&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut Visibility), With<CardTerrain>>,
    color_materials: Option<Res<Assets<bevy::sprite_render::ColorMaterial>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_mesh: Local<RetainedEntities<Entity>>,
) {
    let (Some(color_materials), Some(mut materials)) = (color_materials, materials) else {
        return;
    };
    by_mesh.reconcile(&mut commands, |mesh| terrain.contains(*mesh));
    for (entity, mesh, skin, local, global, visibility, earth) in &terrain {
        let shown = visibility.is_none_or(|visibility| visibility.get());
        let Some((skin, image)) = color_materials
            .get(&skin.0)
            .and_then(|skin| Some((skin, skin.texture.clone()?)))
            .filter(|_| shown)
        else {
            if let Some((.., mut card_visibility)) = by_mesh.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
                card_visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        let color = skin.color.to_srgba().to_u8_array();
        let material = cache
            .materials
            .entry((image.id(), color, true))
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: skin.color,
                    base_color_texture: Some(image.clone()),
                    unlit: true,
                    cull_mode: None,
                    double_sided: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })
            })
            .clone();
        let (scale, rotation, translation) = global.to_scale_rotation_translation();
        let place = Transform {
            translation: translation.truncate().extend(if earth {
                TERRAIN_DEPTH - EARTH_BEHIND_SKIN
            } else {
                TERRAIN_DEPTH + local.translation.z
            }),
            rotation,
            scale,
        };
        match by_mesh.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
            Some((mut card_mesh, mut card_material, mut transform, mut card_visibility)) => {
                if card_mesh.0 != mesh.0 {
                    card_mesh.0 = mesh.0.clone();
                }
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(place);
                card_visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                let card = commands
                    .spawn((
                        CardTerrain,
                        CardScope::Source(entity),
                        Mesh3d(mesh.0.clone()),
                        MeshMaterial3d(material),
                        place,
                        Visibility::Inherited,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                        Name::new("Card terrain"),
                    ))
                    .id();
                by_mesh.insert(entity, card);
            }
        }
    }
}

/// The card depth of scenery the 2D view draws at `z`, with `front` the
/// depth just in front of the front-most body.
fn scenery_depth(z: f32, front: f32) -> f32 {
    use ambition_platformer2d_core::config::{WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER};
    if z >= WORLD_Z_PLAYER {
        front + (z - WORLD_Z_PLAYER) * BAND_ORDER_SCALE
    } else if z >= WORLD_Z_DUMMY {
        SCENERY_DEPTH + (z - WORLD_Z_DUMMY) * BAND_ORDER_SCALE
    } else if z >= WORLD_Z_BLOCK {
        TURF_DEPTH
    } else {
        BACKDROP_DEPTH + (z - WORLD_Z_BLOCK) * BAND_ORDER_SCALE
    }
}

/// Realize each room prop and each piece of a line's turf as a card of the
/// cell it draws, at its 2D place, on the depth its 2D band maps to
/// ([`scenery_depth`]). A prop ranked as a body is left to the bodies.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_scenery(
    mut commands: Commands,
    scenery: Query<
        (Entity, &Sprite, Option<&bevy::sprite::Anchor>, &GlobalTransform, Option<&InheritedVisibility>),
        (
            Or<(
                With<PropVisual>,
                With<ChainTurfPiece>,
                With<super::item_visuals::GroundItemVisual>,
                With<super::item_visuals::WorldItemVisual>,
                With<super::projectile_visuals::ProjectileVisual>,
            )>,
            Without<CardScenery>,
        ),
    >,
    depths: Res<CardRootDepths>,
    rooms: LiveRooms,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut Visibility),
        (With<CardScenery>, Without<Sprite>),
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_source: Local<RetainedEntities<Entity>>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_source.reconcile(&mut commands, |source| scenery.contains(*source));
    for (entity, sprite, anchor, global, visibility) in &scenery {
        let shown = visibility.is_none_or(|visibility| visibility.get())
            && sprite.color.alpha() > 0.0
            && !depths.roots.contains_key(&entity);
        let Some((frame, page_size)) =
            sprite_frame(sprite, anchor, &images, layouts.as_deref()).filter(|_| shown)
        else {
            if let Some((.., mut card_visibility)) = by_source.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
                card_visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        let (mesh, material) =
            sprite_card(sprite, &frame, page_size, true, &mut meshes, &mut materials, &mut cache);
        let (scale, rotation, translation) = global.to_scale_rotation_translation();
        let card_place = sprite_card_place(
            &Transform { translation, rotation, scale },
            scenery_depth(translation.z, depths.fronts.get(&rooms.of(entity)).copied().unwrap_or(CARD_BODY_STEP)),
        );
        match by_source.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
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
                        CardScenery,
                        CardScope::Source(entity),
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        card_place,
                        Visibility::Inherited,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                        Name::new("Card scenery"),
                    ))
                    .id();
                by_source.insert(entity, card);
            }
        }
    }
}

/// Where a parallax panel's card stands for a card camera: facing the
/// camera, along its view from the view's centre `target`, at the panel's 2D
/// offset from that centre scaled by `scale`; behind the play plane for a
/// background layer, in front of it for a foreground one.
pub fn backdrop_card_place(camera: &Transform, target: Vec2, panel: Vec2, layer_z: f32, scale: f32) -> Transform {
    let along = if layer_z < FOREGROUND_PARALLAX_Z {
        -(BACKDROP_DISTANCE - layer_z)
    } else {
        FOREGROUND_DISTANCE + (layer_z - FOREGROUND_PARALLAX_Z) * BAND_ORDER_SCALE
    };
    let offset = (panel - target) * scale;
    let rotation = camera.rotation;
    Transform {
        translation: target.extend(0.0) + rotation * Vec3::new(offset.x, offset.y, along),
        rotation,
        scale: Vec3::new(scale, scale, 1.0),
    }
}

/// Realize each parallax panel the 2D view draws for a view as a card facing
/// that view's card camera ([`backdrop_card_place`]).
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_backdrop(
    mut commands: Commands,
    frames: Res<PresentedViewFrames>,
    settings: Res<CardViewSettings>,
    cameras: Query<(Entity, &Transform, &ambition_sim_view::PresentsView), With<CardViewCamera>>,
    panels: Query<
        (
            Entity,
            &Sprite,
            Option<&bevy::sprite::Anchor>,
            &Transform,
            &ParallaxLayerVisual,
            &Visibility,
            Option<&ambition_sim_view::PresentedForView>,
        ),
        (Without<CardBackdrop>, Without<super::parallax::PortalCaptureParallaxLayerVisual>),
    >,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut Visibility),
        (With<CardBackdrop>, Without<Sprite>, Without<CardViewCamera>),
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_panel: Local<RetainedEntities<(Entity, Entity)>>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_panel.reconcile(&mut commands, |(panel, camera)| panels.contains(*panel) && cameras.contains(*camera));
    let scale = 1.0 / settings.zoom.max(0.05);
    for (camera, camera_place, presents) in &cameras {
        let frame = frames.of(presents.0);
        for (panel, sprite, anchor, place, layer, visibility, key) in &panels {
            // The panel the 2D view draws for this camera's view: shown, and
            // kept for that view (or for no view in particular).
            let drawn = *visibility != Visibility::Hidden && key.is_none_or(|key| key.0 == presents.0);
            let made = frame.filter(|_| drawn).and_then(|frame| {
                Some((frame, sprite_frame(sprite, anchor, &images, layouts.as_deref())?))
            });
            let Some((frame, (cell, page_size))) = made else {
                if let Some((.., mut card_visibility)) =
                    by_panel.get(&(panel, camera)).and_then(|card| cards.get_mut(*card).ok())
                {
                    card_visibility.set_if_neq(Visibility::Hidden);
                }
                continue;
            };
            let (mesh, material) = sprite_card(sprite, &cell, page_size, true, &mut meshes, &mut materials, &mut cache);
            let card_place =
                backdrop_card_place(camera_place, frame.plane_center, place.translation.truncate(), layer.z, scale);
            match by_panel.get(&(panel, camera)).and_then(|card| cards.get_mut(*card).ok()) {
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
                            CardBackdrop,
                            CardScope::Camera(camera),
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            card_place,
                            Visibility::Inherited,
                            RenderLayers::layer(CARD_VIEW_LAYER),
                            Name::new("Card backdrop"),
                        ))
                        .id();
                    by_panel.insert((panel, camera), card);
                }
            }
        }
    }
}

/// Realize each portal's line of light ([`ambition_portal2d_presentation::PortalGlow`])
/// as a thin card along its opening, at the glow's place and turn, in its
/// look's colour, as opaque as the glow is open and not dissolved.
#[cfg(feature = "portal_render")]
pub(super) fn realize_card_portals(
    mut commands: Commands,
    glows: Query<(Entity, &ambition_portal2d_presentation::PortalGlow, &Transform), Without<CardPortal>>,
    mut cards: Query<(&mut MeshMaterial3d<StandardMaterial>, &mut Transform), With<CardPortal>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut line: Local<Option<Handle<Mesh>>>,
    mut by_colour: Local<HashMap<[u8; 4], Handle<StandardMaterial>>>,
    mut by_glow: Local<RetainedEntities<Entity>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    by_glow.reconcile(&mut commands, |glow| glows.contains(*glow));
    let mesh = line.get_or_insert_with(|| meshes.add(Rectangle::new(1.0, 1.0))).clone();
    for (entity, glow, place) in &glows {
        let strength = (glow.appear * (1.0 - glow.dissolve)).clamp(0.0, 1.0);
        let colour = glow.look.display().1.with_alpha(strength);
        let material = by_colour
            .entry(colour.to_srgba().to_u8_array())
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: colour,
                    unlit: true,
                    cull_mode: None,
                    double_sided: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })
            })
            .clone();
        let card_place = Transform {
            translation: place.translation.truncate().extend(PORTAL_DEPTH),
            rotation: place.rotation,
            scale: Vec3::new(glow.length, PORTAL_LINE_WIDTH, 1.0),
        };
        match by_glow.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
            Some((mut card_material, mut transform)) => {
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(card_place);
            }
            None => {
                let card = commands
                    .spawn((
                        CardPortal,
                        CardScope::Source(entity),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material),
                        card_place,
                        Visibility::Inherited,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                        Name::new("Card portal"),
                    ))
                    .id();
                by_glow.insert(entity, card);
            }
        }
    }
}

/// Realize each published vessel as a cart: a floor, a back panel, two
/// sides and a near panel, recessed behind the play plane by the policy.
pub(super) fn realize_card_vessels(
    mut commands: Commands,
    vessels: Option<Res<PresentedVessels>>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    mut boards: Query<&mut Transform, With<CardVessel>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut assets: Local<Option<(Handle<Mesh>, [Handle<StandardMaterial>; 2])>>,
    mut products: Local<RetainedEntities<(VesselOccurrence, usize)>>,
) {
    let (Some(vessels), Some(mut meshes), Some(mut materials)) = (vessels, meshes, materials) else {
        return;
    };
    products.reconcile(&mut commands, |(key, _)| {
        vessels.0.iter().any(|(current, bounds)| current == key && rooms.in_room(bounds.room).is_some())
    });
    if vessels.0.is_empty() {
        return;
    }
    let (mesh, wood) = assets.get_or_insert_with(|| {
        let wood = [Color::srgb(0.36, 0.23, 0.13), Color::srgb(0.55, 0.36, 0.2)];
        (
            meshes.add(Cuboid::from_size(Vec3::ONE)),
            wood.map(|base_color| materials.add(StandardMaterial { base_color, unlit: true, ..default() })),
        )
    });
    for (key, vessel) in &vessels.0 {
        let Some(room) = rooms.in_room(vessel.room) else {
            continue;
        };
        for (slot, (center, size, near)) in cart_boards(vessel, room.0.size).into_iter().enumerate() {
            let key = (key.clone(), slot);
            let place = Transform::from_translation(center).with_scale(size);
            if let Some(mut board) = products.get(&key).and_then(|entity| boards.get_mut(*entity).ok()) {
                board.set_if_neq(place);
                continue;
            }
            let board = commands.spawn((
                CardVessel,
                CardScope::Room(vessel.room),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(wood[usize::from(near)].clone()),
                place,
                RenderLayers::layer(CARD_VIEW_LAYER),
                Name::new("Card cart board"),
            )).id();
            products.insert(key, board);
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

    #[test]
    fn texture_flips_keep_an_asymmetric_anchor_and_reverse_uvs() {
        let quad = TextureQuad {
            rect: Rect::from_corners(Vec2::new(10.0, 20.0), Vec2::new(30.0, 40.0)),
            page: Vec2::splat(100.0),
            size: Vec2::new(20.0, 10.0),
            anchor: Vec2::new(0.25, -0.5),
            flip: [false, false],
        };
        let (positions, uv) = quad.vertices();
        let flipped = TextureQuad { flip: [true, true], ..quad };
        let (flipped_positions, flipped_uv) = flipped.vertices();
        assert_eq!(positions, flipped_positions);
        assert_eq!(positions[0], [-15.0, 0.0, 0.0]);
        assert_eq!(flipped_uv, [uv[2], uv[3], uv[0], uv[1]]);
        let resized_page = TextureQuad { page: Vec2::splat(200.0), ..quad };
        assert!(quad.key() != resized_page.key());
    }

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
        app.world_mut().resource_mut::<CardRootDepths>().roots.insert(root, CardBodyDepth { plane_z: -20.0, source_z: 5.0, order_scale: 1.0 });
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

    /// RV3: a body drawn from a sheet (no rigged presentation) is one card of
    /// its cell at its 2D place, ranked by logical order with the others on
    /// the play plane: the player and an actor each get a card, the later
    /// (higher 2D z) one step nearer. A prop and a hidden actor get none.
    #[test]
    fn a_sheet_drawn_body_is_one_card_ranked_on_its_depth() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .add_systems(Update, (realize_card_bodies, realize_card_sprite_bodies).chain());
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        let sprite = |at: Vec3| {
            (Sprite { image: image.clone(), custom_size: Some(Vec2::new(16.0, 24.0)), ..default() }, Transform::from_translation(at))
        };
        let actor = |kind| BoundFeatureKind::new(kind, Vec2::new(16.0, 24.0));
        app.world_mut().spawn((sprite(Vec3::new(10.0, 0.0, 7.0)), actor(FeatureVisualKind::Actor)));
        app.world_mut().spawn((sprite(Vec3::new(-10.0, 0.0, 3.0)), PlayerVisual));
        app.world_mut().spawn((sprite(Vec3::new(50.0, 0.0, 9.0)), actor(FeatureVisualKind::Chest)));
        app.world_mut().spawn((sprite(Vec3::new(90.0, 0.0, 1.0)), actor(FeatureVisualKind::Actor), Visibility::Hidden));
        app.update();
        let world = app.world_mut();
        let mut cards = world.query_filtered::<(&Transform, &Visibility), With<CardSpriteBody>>();
        let mut places: Vec<Vec3> = cards
            .iter(world)
            .filter(|(_, visibility)| **visibility != Visibility::Hidden)
            .map(|(place, _)| place.translation)
            .collect();
        places.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert_eq!(
            places,
            vec![Vec3::new(-10.0, 0.0, card_body_depth(false, 0)), Vec3::new(10.0, 0.0, card_body_depth(false, 1))],
            "the player and the actor, in logical order; no prop, no hidden body"
        );
    }

    /// RV3: each mesh of a room's terrain skin is a card of the same mesh on
    /// the play plane, its layer order kept; a hidden mesh shows none, and a
    /// mesh that goes takes its card with it.
    #[test]
    fn a_rooms_terrain_skin_is_a_card_on_the_play_plane() {
        use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<CardMeshes>()
            .add_systems(Update, realize_card_terrain);
        let image = Handle::<Image>::default();
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Rectangle::new(32.0, 16.0));
        let skin = app.world_mut().resource_mut::<Assets<ColorMaterial>>().add(ColorMaterial {
            texture: Some(image.clone()),
            ..ColorMaterial::from_color(Color::WHITE)
        });
        let terrain = |z: f32| {
            (
                Mesh2d(mesh.clone()),
                MeshMaterial2d(skin.clone()),
                Transform::from_xyz(0.0, 0.0, z),
                GlobalTransform::from_translation(Vec3::new(40.0, -12.0, 7.0 + z)),
                TerrainSkinMesh { theme: ambition_sprite_sheet::game_assets::ParallaxTheme::new("cave"), centre: Vec2::ZERO },
            )
        };
        let cap = app.world_mut().spawn(terrain(0.03)).id();
        app.world_mut().spawn(terrain(0.005));
        app.world_mut().spawn((terrain(0.01), InheritedVisibility::HIDDEN));
        app.update();
        let shown = |app: &mut App| {
            let world = app.world_mut();
            let mut cards = world.query_filtered::<(&Mesh3d, &Transform, &Visibility), With<CardTerrain>>();
            let mut found: Vec<(bool, f32)> = cards
                .iter(world)
                .filter(|(.., visibility)| **visibility != Visibility::Hidden)
                .map(|(card, place, _)| (card.0 == mesh, place.translation.z))
                .collect();
            found.sort_by(|a, b| a.1.total_cmp(&b.1));
            found
        };
        let found = shown(&mut app);
        assert_eq!(found.len(), 2, "the hidden mesh shows none");
        assert!(found.iter().all(|(same, _)| *same), "the 2D mesh itself");
        assert!((found[0].1 - (TERRAIN_DEPTH + 0.005)).abs() < 1e-6 && (found[1].1 - (TERRAIN_DEPTH + 0.03)).abs() < 1e-6);
        assert!(found[1].1 < card_body_depth(false, 0) && found[1].1 > -NEAR_PANEL_GAP, "behind the bodies, before the near panel");
        app.world_mut().entity_mut(cap).despawn();
        app.update();
        assert_eq!(shown(&mut app).len(), 1, "a mesh that goes takes its card");
    }

    /// RV3: a room's props and its turf are cards at their 2D places, in the
    /// 2D view's order: a backdrop prop behind the cart's back panel, the
    /// turf in front of the block skin, a scenery prop in front of the turf
    /// and behind the portal line, and a prop that hides bodies in front of
    /// the front-most body. A hidden prop shows none, a prop drawn as a body
    /// is a body, and a prop that goes takes its card with it.
    #[test]
    fn a_rooms_props_and_turf_are_cards_in_the_2d_order() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .add_systems(Update, (realize_card_bodies, realize_card_sprite_bodies, realize_card_scenery).chain());
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        let sprite = |at: Vec3| {
            (
                Sprite { image: image.clone(), custom_size: Some(Vec2::new(16.0, 24.0)), ..default() },
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
                InheritedVisibility::VISIBLE,
            )
        };
        let prop = || PropVisual {
            id: "prop".into(),
            kind: "fence".into(),
            name: "Fence".into(),
            size: Vec2::new(16.0, 24.0),
            draw: Default::default(),
            flip_y: false,
        };
        app.world_mut().spawn((sprite(Vec3::new(0.0, 0.0, 20.0)), PlayerVisual));
        let scenery = app.world_mut().spawn((sprite(Vec3::new(10.0, 0.0, 10.6)), prop())).id();
        app.world_mut().spawn((sprite(Vec3::new(20.0, 0.0, 21.75)), prop()));
        app.world_mut().spawn((sprite(Vec3::new(30.0, 0.0, -11.0)), prop()));
        app.world_mut().spawn((sprite(Vec3::new(40.0, 0.0, 2.01)), ChainTurfPiece));
        app.world_mut().spawn((sprite(Vec3::new(50.0, 0.0, 10.7)), prop())).insert(InheritedVisibility::HIDDEN);
        app.world_mut().spawn((
            sprite(Vec3::new(60.0, 0.0, 19.0)),
            prop(),
            BoundFeatureKind::new(FeatureVisualKind::Actor, Vec2::new(16.0, 24.0)),
        ));
        app.update();
        let shown = |app: &mut App| {
            let world = app.world_mut();
            let mut cards = world.query_filtered::<(&Transform, &Visibility), With<CardScenery>>();
            let mut found: Vec<Vec3> = cards
                .iter(world)
                .filter(|(_, visibility)| **visibility != Visibility::Hidden)
                .map(|(place, _)| place.translation)
                .collect();
            found.sort_by(|a, b| a.x.total_cmp(&b.x));
            found
        };
        let found = shown(&mut app);
        let xs: Vec<f32> = found.iter().map(|at| at.x).collect();
        assert_eq!(xs, vec![10.0, 20.0, 30.0, 40.0], "no hidden prop, and the actor prop is a body");
        let depth = |x: f32| found.iter().find(|at| at.x == x).unwrap().z;
        let front = card_body_depth(false, 1) + CARD_BODY_STEP;
        let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(near(depth(10.0), SCENERY_DEPTH + 0.06), "scenery in its band: {}", depth(10.0));
        assert!(near(depth(20.0), front + 0.175), "in front of the front-most body: {}", depth(20.0));
        assert!(near(depth(30.0), BACKDROP_DEPTH - 1.1), "backdrop: {}", depth(30.0));
        assert!(near(depth(40.0), TURF_DEPTH), "turf: {}", depth(40.0));
        assert!(depth(30.0) < -VESSEL_DEPTH, "a backdrop is behind the cart's back panel");
        assert!(TERRAIN_DEPTH + 0.04 < depth(40.0) && depth(40.0) < depth(10.0) && depth(10.0) < PORTAL_DEPTH,
            "skin, turf, scenery, then the portal line");
        assert!(PORTAL_DEPTH < card_body_depth(false, 0), "behind the bodies");
        let mut bodies = app.world_mut().query_filtered::<(), With<CardSpriteBody>>();
        assert_eq!(bodies.iter(app.world()).count(), 2, "the player and the actor prop");
        let card_of_scenery = {
            let world = app.world_mut();
            let mut cards = world.query_filtered::<(Entity, &CardScope), With<CardScenery>>();
            cards.iter(world).find_map(|(card, scope)| {
                matches!(scope, CardScope::Source(source) if *source == scenery).then_some(card)
            }).unwrap()
        };
        app.world_mut().entity_mut(card_of_scenery).despawn();
        app.update();
        assert_eq!(shown(&mut app).len(), 4);
        app.world_mut().entity_mut(scenery).despawn();
        app.update();
        assert_eq!(shown(&mut app).len(), 3, "a prop that goes takes its card");
    }

    /// RV3: a parallax panel the 2D view draws for a view is a card facing
    /// that view's card camera, far behind the play plane at its 2D offset
    /// from the view's centre scaled by `1 / zoom`; a foreground layer is in
    /// front of the play plane. A panel kept for another view, or hidden,
    /// shows none here.
    #[test]
    fn a_parallax_panel_is_a_card_facing_the_card_camera() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardViewSettings>()
            .add_systems(Update, realize_card_backdrop);
        let view = app.world_mut().spawn_empty().id();
        let other = app.world_mut().spawn_empty().id();
        let frame = super::super::camera::PresentedViewFrame {
            room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
            plane_center: Vec2::new(100.0, 50.0),
            roll_radians: 0.0,
            orthographic_scale: 1.0,
        };
        app.insert_resource(PresentedViewFrames::of_views(vec![(view, frame)]));
        let settings = CardViewSettings::default();
        let camera_place = card_camera_transform(&frame, &settings);
        app.world_mut().spawn((CardViewCamera, camera_place, ambition_sim_view::PresentsView(view)));
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        let panel = |at: Vec2, z: f32, key: Entity| {
            (
                Sprite { image: image.clone(), custom_size: Some(Vec2::splat(900.0)), ..default() },
                Transform::from_translation(at.extend(z)),
                ParallaxLayerVisual { factor: Vec2::splat(0.5), z, panel_scale: 1.5, travel: Vec2::ZERO, world_size: Vec2::splat(2000.0) },
                Visibility::Inherited,
                ambition_sim_view::PresentedForView(key),
            )
        };
        app.world_mut().spawn(panel(Vec2::new(300.0, 50.0), -16.0, view));
        app.world_mut().spawn(panel(Vec2::new(100.0, 150.0), 50.0, view));
        app.world_mut().spawn(panel(Vec2::new(100.0, 50.0), -17.0, other));
        app.world_mut().spawn(panel(Vec2::new(100.0, 50.0), -18.0, view)).insert(Visibility::Hidden);
        app.update();
        let world = app.world_mut();
        let mut cards = world.query_filtered::<(&Transform, &Visibility), With<CardBackdrop>>();
        let shown: Vec<Transform> =
            cards.iter(world).filter(|(_, v)| **v != Visibility::Hidden).map(|(place, _)| *place).collect();
        assert_eq!(shown.len(), 2, "the two panels drawn for this view: {shown:?}");
        let target = frame.plane_center.extend(0.0);
        let rotation = camera_place.rotation;
        let local = |place: &Transform| rotation.inverse() * (place.translation - target);
        let back = shown.iter().find(|place| local(place).z < 0.0).expect("a background card");
        let front = shown.iter().find(|place| local(place).z > 0.0).expect("a foreground card");
        let scale = 1.0 / settings.zoom;
        assert!(local(back).distance(Vec3::new(200.0 * scale, 0.0, -(BACKDROP_DISTANCE + 16.0))) < 1e-2, "{:?}", local(back));
        assert!(local(front).distance(Vec3::new(0.0, 100.0 * scale, FOREGROUND_DISTANCE + 0.5)) < 1e-2, "{:?}", local(front));
        assert!(back.rotation.angle_between(rotation) < 1e-5, "facing the card camera");
        assert_eq!(back.scale, Vec3::new(scale, scale, 1.0));
        assert!(local(back).z.abs() < CAMERA_DISTANCE, "inside the camera's far plane");
    }

    /// RV3: a portal is a thin card along its opening at its glow's place
    /// and turn, in its look's colour, as strong as the glow is open; it
    /// leaves with its glow.
    #[cfg(feature = "portal_render")]
    #[test]
    fn a_portal_is_a_line_of_light_on_the_play_plane() {
        use ambition_portal2d_presentation::PortalGlow;
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, realize_card_portals);
        let look = ambition_portal2d_presentation::PortalChannel::Authored(ambition_portal2d_presentation::PortalChannelColor::Teal);
        let glow = PortalGlow {
            portal: None,
            channel: look,
            look,
            room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
            pos: Vec2::new(100.0, 50.0),
            normal: Vec2::Y,
            length: 48.0,
            appear: 0.5,
            dissolve: 0.0,
        };
        let turn = Quat::from_rotation_z(0.3);
        let place = Transform::from_xyz(-20.0, 30.0, 9.0).with_rotation(turn).with_scale(Vec3::new(80.0, 40.0, 1.0));
        let source = app.world_mut().spawn((glow, place)).id();
        app.update();
        let world = app.world_mut();
        let mut cards = world.query_filtered::<(&Transform, &MeshMaterial3d<StandardMaterial>), With<CardPortal>>();
        let (card, material) = cards.single(world).map(|(t, m)| (*t, m.0.clone())).unwrap();
        assert_eq!(card.translation, Vec3::new(-20.0, 30.0, PORTAL_DEPTH));
        assert_eq!(card.rotation, turn);
        assert_eq!(card.scale, Vec3::new(48.0, PORTAL_LINE_WIDTH, 1.0), "the opening, not the glow's halo quad");
        assert!(PORTAL_DEPTH > TERRAIN_DEPTH && PORTAL_DEPTH < card_body_depth(false, 0));
        let colour = world.resource::<Assets<StandardMaterial>>().get(&material).unwrap().base_color;
        assert_eq!(colour, look.display().1.with_alpha(0.5));
        app.world_mut().entity_mut(source).despawn();
        app.update();
        let world = app.world_mut();
        assert_eq!(world.query_filtered::<(), With<CardPortal>>().iter(world).count(), 0, "it leaves with its glow");
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
        assert!(card_body_depth(false, 1) > free && card_body_depth(true, 1) < seated);
        assert!(card_body_depth(true, 100) > back.0.z + back.1.z * 0.5);
        // The panel stands on the bed (y down in the world, up on the plane).
        let bed = 600.0 * 0.5 - vessel.top;
        assert!((near.0.y - near.1.y * 0.5 - bed).abs() < 1e-3);
        assert!((near.1.y - vessel.rim).abs() < 1e-3);
    }
}
