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
//! - a body's parts and attachments have the body's plane;
//! - bodies and parts keep their order in the transparent render pass.
//!
//! A moving platform uses the same sprite frame in both views. A vessel's
//! near and far layers enclose its riders. A platform keeps its layer when
//! its sheet draws from parts.
//!
//! # One picture, when the 2D view has one
//!
//! A body the 2D view composites (a fading frame, a flash, a warp, any reader
//! of it as one image) is drawn here as one card of its composited cell, as
//! the 2D root draws it: a fading frame fades as one picture, not part by
//! part. A whole frame uses the shared flash shader. A direct rig uses a
//! silhouette over each part when a whole frame is not available.
//!
//! # Bodies drawn from a sheet
//!
//! A body with no rigged presentation (an actor or a player the 2D view
//! draws from a sheet cell) is one card of the cell it draws, ranked with the
//! rigged bodies on its depth by the same logical order. It uses the shared
//! flash material for combat and gate transit cues.
//!
//! # The room's terrain
//!
//! The card view uses the room's block skins and earth meshes. The block
//! skins draw at [`TERRAIN_DEPTH`] in source layer order. Filled ground
//! draws behind them. Track earth draws in the order of its lane.
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
//! A room's props and the trim pieces on its lines
//! are cards of the cells the 2D view draws, at their
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
//! at its 2D offset from the view's centre scaled to its viewport.
//! The card view frames the backdrop as the 2D view
//! does, however its camera turns.
//!
//! # Portals
//!
//! Both views use the portal glow shader and the portal's channel label.
//! The card view also uses the 2D view's portal capture texture. A body
//! split across an opening still needs its own presentation.

mod asset_cache;
mod drawings;
mod gameplay;
mod labels;
mod paint_order;
mod world_mesh;

pub use world_mesh::{install_card_mesh_material, CardMaterialSource};
pub(crate) use gameplay::PlaneCameraForCard;

use asset_cache::RecentAsset;
use gameplay::CardScope;
use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::entity::EntityHashSet;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::sprite::{SpriteImageMode, SpriteScalingMode, TextureSlice};

use ambition_platformer2d_shared_tangle::camera_layers::{MainCamera, CARD_VIEW_LAYER};
use ambition_platformer2d_shared_tangle::feature_kind::{BoundFeatureKind, FeatureVisualKind};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, PlayerVisual, PresentationOf};
use ambition_sprite_sheet::character::rigged::PartDraw;

use super::retained_entities::RetainedEntities;
use super::supported_carry::{SlingSpan, SlingString};
use super::actors::rigged::{
    part_color, root_shows, BakedFrame, LimbTwin, RiggedLocalSample, RiggedPresentation,
    RiggedPartImage, RiggedPresentationOf, RiggedPresentedBy,
};
use super::dash_afterimage::DashAfterimage;
use super::hit_flash::PresentedFlash;
use super::posed_hand::HeldArtOf;
use super::camera::coverage::{GameplayOutputs, OutputFootprint};
use super::camera::{PresentedViewFrame, PresentedViewFrames, ViewCameraSet};
use super::moving_platforms::{MovingPlatformVisual, PlatformLooks, SeatedInVessel};
use super::parallax::{ParallaxLayerVisual, FOREGROUND_PARALLAX_Z};
use super::primitives::{AnimatedWorldSpriteTint, PropVisual, WorldSprite};
use super::worn_gear_visuals::{GearLayer, WornGearOrder, WornGearVisual};
pub(super) use paint_order::CardPaintDepth;

/// The distance from the play plane to a vessel's far sprite layer.
pub const VESSEL_DEPTH: f32 = 40.0;
/// The paint key gap between two bodies.
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
/// The depth of the room chart that the camera frames.
const PLAY_PLANE_Z: f32 = 0.0;
/// The depth of a platform without a near panel.
const OPEN_PLATFORM_Z: f32 = -0.125;
/// A line's turf: in front of the block skin's layers, behind the scenery.
pub const TURF_DEPTH: f32 = TERRAIN_DEPTH + 0.1;
/// The back of the scenery props' band: in front of the turf, behind the
/// portal line. A prop's place in the band is its 2D order.
pub const SCENERY_DEPTH: f32 = TERRAIN_DEPTH + 0.15;
/// Backdrop props: behind a vessel's back panel.
pub const BACKDROP_DEPTH: f32 = -(VESSEL_DEPTH + 5.0);
/// How much card depth one unit of 2D depth inside a band is.
const BAND_ORDER_SCALE: f32 = 0.1;
/// Leave space between room scenery and the play-plane body order.
const SCENERY_FRONT_PAINT: f32 = -4.0 * FLASH_STEP;
/// How far behind the play plane, along a card camera's view, the parallax
/// backdrop stands; a layer further back in 2D is further back here.
const BACKDROP_DISTANCE: f32 = 1500.0;
/// How far in front of the play plane a foreground parallax layer stands.
const FOREGROUND_DISTANCE: f32 = 1000.0;
/// The near sprite layer's distance behind the play plane.
const NEAR_PANEL_GAP: f32 = 1.0;
/// The depth available to a platform layer's parts.
const PLATFORM_PART_DEPTH: f32 = 0.05;
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
    /// How much nearer the inset looks: 1 shows the 2D view's
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

    pub(super) fn replaces_plane(self) -> bool {
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

    pub(super) fn inset_size(&self, size: UVec2) -> Option<UVec2> {
        if size.x < 64 || size.y < 64 {
            return None;
        }
        let w = ((size.x as f32 * self.viewport_fraction) as u32).clamp(32, size.x);
        let h = ((w as f32 * size.y as f32 / size.x as f32) as u32).clamp(32, size.y);
        Some(UVec2::new(w, h))
    }

    pub(super) fn projection_scale(&self, plane_scale: f32) -> f32 {
        if self.mode == CardViewMode::Inset {
            plane_scale / (self.viewport_fraction * self.zoom).max(0.05)
        } else {
            plane_scale
        }
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
#[derive(Component, Debug, Default)]
pub struct CardPart {
    tinted_material: Option<Handle<StandardMaterial>>,
}

/// A card drawing one held or wielded item.
#[derive(Component)]
pub struct CardHeldArt;

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

/// The depth of each drawn body and its parts for this frame.
#[derive(Resource, Default, Debug)]
pub struct CardRootDepths {
    roots: HashMap<Entity, CardBodyDepth>,
    fronts: HashMap<Option<LiveRoomInstance>, f32>,
    paint_slots: Vec<BodyPaintSlot>,
}

#[derive(Debug)]
struct BodyPaintSlot {
    room: Option<LiveRoomInstance>,
    source_z: f32,
    paint_z: f32,
    order_scale: f32,
}

#[derive(Clone, Copy, Debug)]
struct CardDepth {
    plane: f32,
    paint: CardPaintDepth,
}

impl CardDepth {
    fn new(plane: f32, paint: f32) -> Self {
        Self { plane, paint: CardPaintDepth(paint) }
    }

    fn on_plane(plane: f32) -> Self {
        Self::new(plane, plane)
    }

    #[cfg(feature = "portal_render")]
    fn on_play_plane(self) -> Self {
        Self { plane: PLAY_PLANE_Z, ..self }
    }

    fn place(self, transform: &Transform) -> Transform {
        sprite_card_place(transform, self.plane)
    }
}

impl CardRootDepths {
    fn scenery(&self, z: f32, room: Option<LiveRoomInstance>) -> CardDepth {
        let front = self.fronts.get(&room).copied().unwrap_or(CARD_BODY_STEP);
        CardDepth::new(scenery_depth(z, CARD_BODY_STEP), scenery_paint_depth(z, front))
    }

    pub(crate) fn field_depth(&self, source: Entity, z: f32, room: Option<LiveRoomInstance>) -> f32 {
        self.roots
            .get(&source)
            .map_or_else(|| self.scenery(z, room).plane, |body| body.depth.plane)
    }

    fn representation_of(&self, subject: Option<&PresentationOf>) -> Option<CardBodyRepresentation> {
        self.roots.get(&subject?.0).map(|depth| depth.representation)
    }

    fn held_depth(&self, root: Entity, source_z: f32) -> Option<CardDepth> {
        let body = self.roots.get(&root)?;
        Some(body.at_offset(held_art_card_offset(source_z, body.source_z)))
    }

    fn effect_depth(&self, subject: Option<&PresentationOf>, source_z: f32) -> Option<CardDepth> {
        let depth = self.roots.get(&subject?.0)?;
        let bias = source_z - depth.source_z;
        let offset = if bias < 0.0 {
            bias.max(-CARD_BODY_STEP * 0.5)
        } else {
            // An effect above the body has the gap between its front part
            // and the flash layer.
            let fraction = (bias / super::hit_flash::FLASH_OVERLAY_Z_BIAS).min(1.0);
            depth.front_part + fraction * FLASH_STEP
        };
        Some(depth.at_offset(offset))
    }

    fn scene_paint_depth(&self, source_z: f32, room: Option<LiveRoomInstance>) -> Option<CardPaintDepth> {
        let index = self.paint_slots.partition_point(|body| {
            body.room < room || (body.room == room && body.source_z.total_cmp(&source_z).is_lt())
        });
        let next = self.paint_slots.get(index).filter(|body| body.room == room)?;
        let before = index.checked_sub(1)
            .and_then(|index| self.paint_slots.get(index))
            .filter(|body| body.room == room)
            .map_or(next.source_z - 1.0, |body| body.source_z);
        let fraction = ((next.source_z - source_z) / (next.source_z - before).max(f32::EPSILON)).clamp(0.0, 1.0);
        // Keep the gap behind the body's back gear. Order lights in the gap
        // by the depth of their source draws.
        Some(CardPaintDepth(next.paint_z - (2.0 + fraction) * FLASH_STEP * next.order_scale))
    }

    fn gear_depth(&self, order: WornGearOrder, layer: GearLayer) -> Option<CardDepth> {
        let depth = self.roots.get(&order.root)?;
        let offset = match layer {
            GearLayer::Back => -FLASH_STEP,
            GearLayer::Front => depth.part_offset(order.anchor_part).unwrap_or(depth.front_part) + FLASH_STEP * 0.5,
        };
        Some(depth.at_offset(offset))
    }

    fn sling_depths(&self, span: SlingSpan) -> Option<[CardDepth; 2]> {
        let depth = self.roots.get(&span.root)?;
        let foot = depth.part_offset(span.foot_part).unwrap_or(0.0) - FLASH_STEP;
        let foot_z = depth.at_offset(foot);
        let load_z = span.load.and_then(|(root, z)| self.held_depth(root, z)).unwrap_or(foot_z);
        Some([foot_z, load_z])
    }
}

/// How the card renderer draws an actor's body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardBodyRepresentation {
    Sprite,
    RiggedFrame,
    RiggedParts,
}

#[derive(Clone, Copy, Debug)]
struct CardBodyDepth {
    depth: CardDepth,
    source_z: f32,
    order_scale: f32,
    front_part: f32,
    representation: CardBodyRepresentation,
}

impl CardBodyDepth {
    fn at_offset(&self, offset: f32) -> CardDepth {
        CardDepth::new(self.depth.plane, self.depth.paint.0 + offset * self.order_scale)
    }

    fn part_offset(&self, part: Option<usize>) -> Option<f32> {
        match (self.representation, part) {
            (CardBodyRepresentation::RiggedParts, Some(index)) => Some(index as f32 * CARD_PART_STEP),
            _ => None,
        }
    }
}

/// The card view: install it beside the 2D presentation.
pub struct CardViewPlugin;

impl Plugin for CardViewPlugin {
    fn build(&self, app: &mut App) {
        ambition_sprite_fx::install_mesh_fragment_shader(app);
        app.init_resource::<CardViewSettings>();
        app.init_resource::<CardMeshes>();
        app.init_resource::<CardRootDepths>();
        paint_order::install(app);
        world_mesh::install(app);
        app.add_systems(
            Update,
            (
                present_card_view_rigs,
                place_card_view_cameras,
                realize_card_bodies,
                realize_card_sprite_bodies,
                realize_card_held_art,
            )
                .chain()
                .run_if(card_outputs_enabled)
                .after(ViewCameraSet)
                .after(super::actors::sync_visuals)
                .after(super::actors::rigged::RiggedDriveSet)
                .after(super::hit_flash::sync_hit_flash_overlays)
                .after(super::item_visuals::sync_held_item_visual)
                .after(super::wielded_item_visuals::sync_wielded_item_visuals)
                .after(super::moving_platforms::sync_moving_platform_visuals)
                .after(super::moving_platforms::seat_bodies_in_vessels),
        );
        app.add_systems(
            PostUpdate,
            (retire_card_outputs, gameplay::route_card_outputs)
                .chain()
                .before(bevy::camera::CameraUpdateSystems),
        );
        app.add_systems(
            PostUpdate,
            realize_card_backdrop
                .run_if(card_outputs_enabled)
                .after(gameplay::route_card_outputs)
                .before(bevy::transform::TransformSystems::Propagate)
                .before(bevy::asset::AssetEventSystems)
                .before(gameplay::isolate_cards),
        );
        app.add_systems(Last, prune_card_cache.run_if(card_outputs_enabled));
        app.add_systems(
            PostUpdate,
            labels::prepare_label_layout
                .after(gameplay::route_card_outputs)
                .after(super::view_isolation::isolate_live_rooms)
                .after(bevy::camera::CameraUpdateSystems)
                .before(bevy::sprite::update_text2d_layout)
                .run_if(card_outputs_enabled),
        );
        app.add_systems(
            PostUpdate,
            (realize_card_scenery, drawings::realize_card_drawings)
                .in_set(world_mesh::CardWorldMeshSet)
                .run_if(card_outputs_enabled),
        );
        app.add_systems(
            PostUpdate,
            labels::realize_card_labels
                .after(bevy::sprite::update_text2d_layout)
                .in_set(world_mesh::CardWorldMeshSet)
                .run_if(card_outputs_enabled),
        );
        app.add_systems(
            PostUpdate,
            gameplay::isolate_cards
                .after(super::view_isolation::isolate_live_rooms)
                .after(gameplay::route_card_outputs)
                .after(world_mesh::CardWorldMeshSet)
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                .run_if(card_outputs_enabled),
        );
    }
}

fn wants_card_outputs(settings: Option<&CardViewSettings>, has_views: bool) -> bool {
    has_views && settings.is_some_and(|settings| settings.mode != CardViewMode::Plane)
}

fn card_outputs_enabled(
    settings: Res<CardViewSettings>,
    views: Query<(), With<ambition_sim_view::LocalView>>,
) -> bool {
    wants_card_outputs(Some(&settings), !views.is_empty())
}

fn retire_card_outputs(
    settings: Res<CardViewSettings>,
    views: Query<(), With<ambition_sim_view::LocalView>>,
    mut commands: Commands,
    outputs: Query<Entity, Or<(With<CardScope>, With<CardBody>, With<CardViewCamera>)>>,
    mut meshes: ResMut<CardMeshes>,
) {
    if wants_card_outputs(Some(&settings), !views.is_empty()) {
        return;
    }
    for output in &outputs {
        commands.entity(output).try_despawn();
    }
    if !meshes.is_empty() {
        *meshes = CardMeshes::default();
    }
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
            // Indirect instance compaction can change the order of a
            // batch. Alpha-blended cards need their sorted order.
            bevy::render::view::NoIndirectDrawing,
            // As the TwinTrack spacetime view and the pause cube: a
            // higher-order Camera3d composited over the gameplay Camera2d,
            // its clear held to its own viewport.
            Tonemapping::None,
            Msaa::Sample4,
            Camera {
                // Below the pause scrim (7) and the front HUD (9).
                order: 6,
                // The output router activates this camera.
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.07, 0.08, 0.11)),
                ..default()
            },
            Projection::Orthographic(card_camera_projection()),
            RenderLayers::layer(CARD_VIEW_LAYER),
            ambition_sim_view::PresentsView(view),
            Name::new("Card view camera"),
        ));
    }
}

pub(super) fn card_camera_projection() -> OrthographicProjection {
    OrthographicProjection {
        scaling_mode: ScalingMode::WindowSize,
        near: 0.0,
        far: CAMERA_DISTANCE * 2.0,
        ..OrthographicProjection::default_3d()
    }
}

/// Set the card camera's pose and scale from the view's presented frame.
pub fn place_card_view_cameras(
    frames: Res<PresentedViewFrames>,
    settings: Res<CardViewSettings>,
    mut cameras: Query<
        (&mut Transform, &mut Projection, &ambition_sim_view::PresentsView),
        With<CardViewCamera>,
    >,
) {
    for (mut transform, mut projection, presents) in &mut cameras {
        let Some(frame) = frames.of(presents.0) else {
            continue;
        };
        *transform = card_camera_transform(frame, &settings);
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = settings.projection_scale(frame.orthographic_scale);
        }
    }
}

/// The card camera's place for a frame: turned and tilted about the frame's
/// centre on the play plane, rolled as the frame is.
pub fn card_camera_transform(frame: &PresentedViewFrame, settings: &CardViewSettings) -> Transform {
    let target = frame.plane_center.extend(PLAY_PLANE_Z);
    let (yaw, pitch) = if settings.mode == CardViewMode::SideOn {
        (0.0, 0.0)
    } else {
        (settings.yaw_radians, settings.pitch_radians)
    };
    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, -pitch, frame.roll_radians);
    Transform::from_translation(target + rotation * Vec3::Z * CAMERA_DISTANCE).with_rotation(rotation)
}

/// The paint key of a body on the play plane or in a vessel.
fn body_paint_z(seated: bool, rank: usize) -> f32 {
    if seated {
        -VESSEL_DEPTH * 0.5 - (rank as f32 * CARD_BODY_STEP).min(VESSEL_DEPTH * 0.25)
    } else {
        rank as f32 * CARD_BODY_STEP
    }
}

/// Part quads and their materials, made once per part and per colour.
#[derive(Resource, Default)]
pub struct CardMeshes {
    meshes: HashMap<QuadKey, RecentAsset<Mesh>>,
    subdivided: HashMap<QuadKey, Vec<(SpriteImageMode, RecentAsset<Mesh>)>>,
    materials: HashMap<(AssetId<Image>, [u8; 4]), RecentAsset<StandardMaterial>>,
    /// Silhouettes, by page and the cue's colour and strength in bytes.
    flashes: HashMap<(AssetId<Image>, [u8; 4]), RecentAsset<StandardMaterial>>,
    frames: u8,
}

/// Keep recent animation frames and atlas pages for one to two passes.
const CARD_CACHE_PASS_FRAMES: u8 = 60;

fn prune_card_cache(mut cache: ResMut<CardMeshes>) {
    cache.frames += 1;
    if cache.frames < CARD_CACHE_PASS_FRAMES {
        return;
    }
    cache.frames = 0;
    cache.meshes.retain(|_, asset| asset.retain());
    cache.subdivided.retain(|_, variants| {
        variants.retain_mut(|(_, asset)| asset.retain());
        !variants.is_empty()
    });
    cache.materials.retain(|_, asset| asset.retain());
    cache.flashes.retain(|_, asset| asset.retain());
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

    fn corners(self) -> (Vec2, Vec2) {
        ambition_sprite_fx::anchored_quad_corners(self.size, self.anchor)
    }

    fn vertices(self) -> ([[f32; 3]; 4], [[f32; 2]; 4]) {
        let (a, b) = self.corners();
        let [x0, y0] = a.to_array();
        let [x1, y1] = b.to_array();
        let [mut u0, mut v0, mut u1, mut v1] = ambition_sprite_fx::TextureRegion {
            rect: self.rect,
            page_size: self.page,
        }
        .uv_rect()
        .to_array();
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
        mesh_for_quads([(self, Vec2::ZERO)])
    }

    fn scaled(mut self, mode: SpriteScalingMode) -> Self {
        use SpriteScalingMode::{FillCenter, FillEnd, FillStart, FitCenter, FitEnd, FitStart};
        let pixels = self.rect.size();
        if !pixels.cmpgt(Vec2::ZERO).all() || !self.size.cmpgt(Vec2::ZERO).all() {
            return self;
        }
        let scale = self.size / pixels;
        match mode {
            FillStart | FillCenter | FillEnd => {
                let crop = self.size / scale.max_element();
                let alignment = match mode {
                    FillStart => 0.0,
                    FillEnd => 1.0,
                    _ => 0.5,
                };
                // The fill crops the image after the flip.
                let alignment = Vec2::from_array(self.flip.map(|flip| if flip {
                    1.0 - alignment
                } else {
                    alignment
                }));
                let start = self.rect.min + (pixels - crop) * alignment;
                self.rect = Rect { min: start, max: start + crop };
            }
            FitStart | FitCenter | FitEnd => {
                let fitted = pixels * scale.min_element();
                let remaining = self.size - fitted;
                let align = match mode {
                    FitStart => Vec2::new(1.0, -1.0),
                    FitEnd => Vec2::new(-1.0, 1.0),
                    _ => Vec2::ZERO,
                };
                // Apply the 2D sprite position before storing the quad anchor.
                let offset = (-Vec2::splat(0.5) - self.anchor) * remaining * align;
                self.anchor -= offset / fitted;
                self.size = fitted;
            }
        }
        self
    }

    fn subdivided_mesh(self, mode: &SpriteImageMode) -> Mesh {
        let slices = match mode {
            SpriteImageMode::Sliced(slicer) => slicer.compute_slices(self.rect, Some(self.size)),
            SpriteImageMode::Tiled { tile_x, tile_y, stretch_value } => TextureSlice {
                texture_rect: self.rect,
                draw_size: self.size,
                offset: Vec2::ZERO,
            }
            .tiled(*stretch_value, (*tile_x, *tile_y)),
            SpriteImageMode::Auto | SpriteImageMode::Scale(_) => unreachable!("mode has one quad"),
        };
        let flip = Vec2::from_array(self.flip.map(|flip| if flip { -1.0 } else { 1.0 }));
        mesh_for_quads(
            slices
                .into_iter()
                .filter(|slice| slice.draw_size.cmpgt(Vec2::ZERO).all())
                .map(|slice| {
                    let tile = Self {
                        rect: slice.texture_rect,
                        size: slice.draw_size,
                        anchor: Vec2::ZERO,
                        ..self
                    };
                    (tile, slice.offset * flip - self.anchor * self.size)
                }),
        )
    }
}

fn mesh_for_quads(quads: impl IntoIterator<Item = (TextureQuad, Vec2)>) -> Mesh {
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for (quad, offset) in quads {
        let first = positions.len() as u32;
        let (corners, quad_uvs) = quad.vertices();
        positions.extend(corners.map(|[x, y, z]| [x + offset.x, y + offset.y, z]));
        uvs.extend(quad_uvs);
        indices.extend([0, 1, 2, 0, 2, 3].map(|i| first + i));
    }
    let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
#[test]
fn a_tiled_sprite_card_repeats_and_crops_its_last_tile() {
    use bevy::mesh::VertexAttributeValues::{Float32x2, Float32x3};

    let tile_size = Vec2::splat(8.0);
    let quad = TextureQuad {
        rect: Rect::from_corners(Vec2::ZERO, tile_size),
        page: tile_size,
        size: Vec2::new(20.0, 8.0),
        anchor: Vec2::ZERO,
        flip: [false, false],
    };
    let mesh = quad.subdivided_mesh(&SpriteImageMode::Tiled {
        tile_x: true,
        tile_y: false,
        stretch_value: 1.0,
    });
    let Some(Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!("missing positions") };
    let Some(Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else { panic!("missing UVs") };
    assert_eq!(positions.len(), 12);
    assert_eq!((positions[0][0], positions[9][0]), (-10.0, 10.0));
    assert_eq!((uvs[1][0], uvs[5][0], uvs[9][0]), (1.0, 1.0, 0.5));
}

#[cfg(test)]
#[test]
fn a_sliced_column_keeps_its_caps_and_repeats_its_middle() {
    use bevy::mesh::VertexAttributeValues::{Float32x2, Float32x3};
    use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};

    let page = Vec2::new(16.0, 32.0);
    let quad = TextureQuad {
        rect: Rect::from_corners(Vec2::ZERO, page),
        page,
        size: Vec2::new(16.0, 64.0),
        anchor: Vec2::ZERO,
        flip: [false, false],
    };
    let mesh = quad.subdivided_mesh(&SpriteImageMode::Sliced(TextureSlicer {
        border: BorderRect {
            min_inset: Vec2::new(0.0, 8.0),
            max_inset: Vec2::new(0.0, 8.0),
        },
        center_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    }));
    let Some(Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
        panic!("missing positions");
    };
    let Some(Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
        panic!("missing UVs");
    };
    let mut rows: Vec<_> = positions.chunks_exact(4).zip(uvs.chunks_exact(4))
        .map(|(points, uv)| (points[0][1], points[2][1], uv[2][1], uv[0][1]))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(rows, [
        (-32.0, -24.0, 0.75, 1.0),
        (-24.0, -8.0, 0.25, 0.75),
        (-8.0, 8.0, 0.25, 0.75),
        (8.0, 24.0, 0.25, 0.75),
        (24.0, 32.0, 0.0, 0.25),
    ]);
}

#[cfg(test)]
#[test]
fn changing_sprite_scale_mode_changes_the_card_crop_or_bounds() {
    use bevy::mesh::VertexAttributeValues::{Float32x2, Float32x3};

    let page = Vec2::new(64.0, 32.0);
    let quad = TextureQuad {
        rect: Rect::from_corners(Vec2::ZERO, page),
        page,
        size: Vec2::new(16.0, 32.0),
        anchor: Vec2::ZERO,
        flip: [true, false],
    };
    let mut meshes = Assets::<Mesh>::default();
    let mut cache = CardMeshes::default();
    let fill = SpriteImageMode::Scale(SpriteScalingMode::FillStart);
    let fit = SpriteImageMode::Scale(SpriteScalingMode::FitStart);
    let filled = cache.sprite_mesh(quad, &fill, &mut meshes);
    let fitted = cache.sprite_mesh(quad, &fit, &mut meshes);
    assert_eq!(cache.sprite_mesh(quad, &fill, &mut meshes), filled);
    assert_ne!(fitted, filled);

    let filled = meshes.get(&filled).unwrap();
    let fitted = meshes.get(&fitted).unwrap();
    assert_eq!(
        filled.attribute(Mesh::ATTRIBUTE_UV_0),
        Some(&Float32x2(vec![[1.0, 1.0], [0.75, 1.0], [0.75, 0.0], [1.0, 0.0]])),
    );
    assert_eq!(
        fitted.attribute(Mesh::ATTRIBUTE_POSITION),
        Some(&Float32x3(vec![[-8.0, 8.0, 0.0], [8.0, 8.0, 0.0], [8.0, 16.0, 0.0], [-8.0, 16.0, 0.0]])),
    );
}

impl CardMeshes {
    fn is_empty(&self) -> bool {
        self.meshes.is_empty() && self.subdivided.is_empty() && self.materials.is_empty() && self.flashes.is_empty()
    }

    fn quad(&mut self, quad: TextureQuad, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.meshes.entry(quad.key()).or_insert_with(|| RecentAsset::new(quad.mesh(), meshes)).get()
    }

    fn sprite_mesh(
        &mut self,
        quad: TextureQuad,
        mode: &SpriteImageMode,
        meshes: &mut Assets<Mesh>,
    ) -> Handle<Mesh> {
        if let Some(mode) = mode.scale() {
            // The image region and the fitted anchor are part of the quad key.
            return self.quad(quad.scaled(mode), meshes);
        }
        if !mode.uses_slices() {
            return self.quad(quad, meshes);
        }
        let variants = self.subdivided.entry(quad.key()).or_default();
        if let Some((_, asset)) = variants.iter_mut().find(|(recipe, _)| recipe == mode) {
            return asset.get();
        }
        let mut asset = RecentAsset::new(quad.subdivided_mesh(mode), meshes);
        let handle = asset.get();
        variants.push((mode.clone(), asset));
        handle
    }

    fn textured_material(
        &mut self,
        texture: &Handle<Image>,
        color: [u8; 4],
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.materials.entry((texture.id(), color)).or_insert_with(|| {
            let color = Color::srgba_u8(color[0], color[1], color[2], color[3]);
            let texture = (*texture != Handle::default()).then(|| texture.clone());
            RecentAsset::new(sprite_material(texture, color), materials)
        }).get()
    }

    fn flash(
        &mut self,
        page: &Handle<Image>,
        flash: PresentedFlash,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        let key = flash_key(flash);
        self.flashes.entry((page.id(), key)).or_insert_with(|| {
            let [r, g, b, intensity] = key.map(|component| f32::from(component) / 255.0);
            let flash = PresentedFlash { tint: Vec3::new(r, g, b), intensity };
            RecentAsset::new(flash_material(page, flash), materials)
        }).get()
    }
}

fn sprite_material(texture: Option<Handle<Image>>, color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        base_color_texture: texture,
        unlit: true,
        cull_mode: None,
        double_sided: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

/// Update a material owned by one card.
fn retain_sprite_material(
    handle: Option<&Handle<StandardMaterial>>,
    image: &Handle<Image>,
    color: Color,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    let texture = (*image != Handle::default()).then_some(image);
    if let Some(handle) = handle {
        if let Some(material) = materials.get(handle) {
            if material.base_color != color || material.base_color_texture.as_ref() != texture {
                let mut material = materials.get_mut(handle).unwrap();
                material.base_color = color;
                material.base_color_texture = texture.cloned();
            }
            return handle.clone();
        }
    }
    materials.add(sprite_material(texture.cloned(), color))
}

enum PartMaterial {
    Shared(Handle<StandardMaterial>),
    Tinted { image: Handle<Image>, color: Color },
}

impl CardPart {
    fn material(
        &mut self,
        draw: &PartMaterial,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        match draw {
            PartMaterial::Shared(material) => {
                self.tinted_material = None;
                material.clone()
            }
            PartMaterial::Tinted { image, color } => {
                let handle = retain_sprite_material(self.tinted_material.as_ref(), image, *color, materials);
                if self.tinted_material.as_ref() != Some(&handle) {
                    self.tinted_material = Some(handle.clone());
                }
                handle
            }
        }
    }
}

type CardQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut CardPart, &'static mut Mesh3d, &'static mut MeshMaterial3d<StandardMaterial>,
        &'static mut Transform, &'static mut CardPaintDepth, &'static mut Visibility),
    Without<CardBody>,
>;

struct PartCardDraw {
    mesh: Handle<Mesh>,
    material: PartMaterial,
    place: Transform,
    paint: CardPaintDepth,
}

fn hide_card(
    mut mesh: Mut<Mesh3d>,
    mut material: Mut<MeshMaterial3d<StandardMaterial>>,
    mut visibility: Mut<Visibility>,
) {
    visibility.set_if_neq(Visibility::Hidden);
    if mesh.0 != Handle::default() {
        mesh.0 = Handle::default();
    }
    if material.0 != Handle::default() {
        material.0 = Handle::default();
    }
}

fn hide_part_cards(cards: &mut CardQuery, parts: &[Entity]) {
    for &card in parts {
        if let Ok((mut part, mesh, material, .., visibility)) = cards.get_mut(card) {
            part.bypass_change_detection().tinted_material = None;
            hide_card(mesh, material, visibility);
        }
    }
}

/// Show `shown` as a body's cards, in order, spawning what it lacks and
/// hiding the rest.
fn show_cards(
    commands: &mut Commands,
    body: &mut CardBody,
    cards: &mut CardQuery,
    owner: Entity,
    source: Entity,
    layer: &RenderLayers,
    materials: &mut Assets<StandardMaterial>,
    shown: &[PartCardDraw],
) {
    for (index, PartCardDraw { mesh, material, place, paint }) in shown.iter().enumerate() {
        if body.parts.get(index).is_none_or(|entity| commands.get_entity(*entity).is_err()) {
            let mut part = CardPart::default();
            let material = part.material(material, materials);
            let card = commands
                .spawn((
                    part,
                    CardScope::Source(source),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    *place,
                    *paint,
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
        let Ok((mut part, mut card_mesh, mut card_material, mut transform, mut order, mut visibility)) = cards.get_mut(body.parts[index]) else {
            continue;
        };
        let material = part.bypass_change_detection().material(material, materials);
        if card_mesh.0 != *mesh {
            card_mesh.0 = mesh.clone();
        }
        if card_material.0 != material {
            card_material.0 = material;
        }
        transform.set_if_neq(*place);
        order.set_if_neq(*paint);
        visibility.set_if_neq(Visibility::Inherited);
    }
    hide_part_cards(cards, &body.parts[shown.len()..]);
}

/// Place a part in the plane of its body.
pub(super) fn card_part_local(draw: &PartDraw) -> Transform {
    super::actors::rigged::part_local_transform(draw, 0.0)
}

#[derive(Default)]
pub(super) struct CardBodyScratch {
    by_owner: RetainedEntities<Entity>,
    rigged_roots: EntityHashSet,
    ranked: Vec<RankedBody>,
    placements: HashMap<Entity, CardBodyDepth>,
    seated_fragments: Vec<SeatedFragment>,
    shown: Vec<PartCardDraw>,
}

struct RankedBody {
    room: Option<LiveRoomInstance>,
    source_z: f32,
    source: Entity,
    owner: Entity,
    seat: Option<usize>,
}

struct SeatedFragment {
    host: Entity,
    seat: usize,
    over: bool,
    owner: Entity,
    source: Entity,
    source_z: f32,
    span: f32,
}

fn seated_body_depth(seat: usize) -> CardDepth {
    CardDepth::new(-VESSEL_DEPTH * 0.5, body_paint_z(true, seat))
}

fn rig_paint_span(sample: &RiggedLocalSample) -> f32 {
    let parts = sample.parts().map_or(0, <[PartDraw]>::len) as f32 * CARD_PART_STEP;
    CARD_BODY_STEP.max(parts + HELD_ART_REACH)
}

/// Realize every sampled rigged body as cards.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_bodies(
    mut commands: Commands,
    owners: Query<(Entity, &RiggedPresentationOf, &RiggedPresentation, &RiggedLocalSample)>,
    roots: Query<
        (
            &Transform, Option<&SeatedInVessel>,
            Option<&PresentedFlash>, Option<&MovingPlatformVisual>, Has<PropVisual>,
            (Option<&Visibility>, Option<&InheritedVisibility>, Has<ChildOf>),
        ),
        (Without<CardBody>, Without<CardPart>),
    >,
    twins: Query<&LimbTwin>,
    seats: Query<(&SeatedInVessel, &RiggedPresentedBy)>,
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
        (
            Entity, &Transform, Option<&SeatedInVessel>, Option<&BoundFeatureKind>,
            (Option<&Visibility>, Option<&InheritedVisibility>, Has<ChildOf>),
        ),
        SpriteBodyFilter,
    >,
    rooms: LiveRooms,
    platform_looks: Option<Res<PlatformLooks>>,
    mut scratch: Local<CardBodyScratch>,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    let layer = RenderLayers::layer(CARD_VIEW_LAYER);

    let CardBodyScratch { by_owner, rigged_roots, ranked, placements, seated_fragments, shown } = &mut *scratch;
    by_owner.reconcile(&mut commands, |owner| owners.contains(*owner));

    // Rank the drawn bodies on each depth by their logical order.
    rigged_roots.clear();
    ranked.clear();
    placements.clear();
    seated_fragments.clear();
    for (owner, of, _, sample) in &owners {
        rigged_roots.insert(of.0);
        let drawable = (sample.parts().is_some() && sample.feet.is_some()) || sample.baked.is_some();
        if !drawable {
            continue;
        }
        let Ok((root, seated, .., platform, prop, visibility)) = roots.get(of.0) else {
            continue;
        };
        if !root_shows(visibility) {
            continue;
        }
        if let Ok(twin) = twins.get(of.0) {
            if let Ok((seat, host)) = seats.get(twin.of) {
                seated_fragments.push(SeatedFragment {
                    host: host.owner(),
                    seat: seat.slot,
                    over: twin.is_over(),
                    owner,
                    source: of.0,
                    source_z: root.translation.z,
                    span: rig_paint_span(sample),
                });
                continue;
            }
        }
        if let Some(platform) = platform {
            let part_count = sample.parts().map_or(1, <[PartDraw]>::len).max(1);
            placements.insert(owner, CardBodyDepth {
                depth: platform_card_depth(platform, platform_looks.as_deref(), root.translation.z),
                source_z: root.translation.z,
                order_scale: (PLATFORM_PART_DEPTH / (part_count as f32 * CARD_PART_STEP)).min(1.0),
                front_part: 0.0,
                representation: CardBodyRepresentation::Sprite,
            });
        } else if !prop {
            ranked.push(RankedBody {
                room: rooms.of(of.0),
                source_z: root.translation.z,
                source: of.0,
                owner,
                seat: seated.map(|seat| seat.slot),
            });
        }
    }
    for (entity, place, seated, kind, visibility) in &sprite_bodies {
        let body = kind.is_none_or(|kind| kind.kind == FeatureVisualKind::Actor);
        if body && root_shows(visibility) && !rigged_roots.contains(&entity) {
            ranked.push(RankedBody {
                room: rooms.of(entity),
                source_z: place.translation.z,
                source: entity,
                owner: entity,
                seat: seated.map(|seat| seat.slot),
            });
        }
    }
    ranked.sort_unstable_by(|a, b| {
        a.room.cmp(&b.room)
            .then(a.source_z.total_cmp(&b.source_z))
            .then(a.source.cmp(&b.source))
    });
    if let Some(depths) = root_depths.as_deref_mut() {
        depths.roots.clear();
        depths.fronts.clear();
        depths.paint_slots.clear();
    }
    for row in ranked.chunk_by(|a, b| a.room == b.room) {
        let count = row.iter().filter(|body| body.seat.is_none()).count();
        let scale = (BODY_ORDER_RANGE / body_paint_z(false, count.max(1))).min(1.0);
        let mut rank = 0;
        for &RankedBody { room, source_z, owner, seat, .. } in row {
            let (depth, order_scale) = if let Some(seat) = seat {
                (seated_body_depth(seat), 1.0)
            } else {
                let paint = body_paint_z(false, rank) * scale;
                rank += 1;
                (CardDepth::new(PLAY_PLANE_Z, paint), scale)
            };
            if let Some(depths) = root_depths.as_deref_mut() {
                if seat.is_none() {
                    depths.fronts.insert(room, depth.paint.0 + CARD_BODY_STEP * scale);
                }
                depths.paint_slots.push(BodyPaintSlot { room, source_z, paint_z: depth.paint.0, order_scale });
            }
            placements.insert(owner, CardBodyDepth {
                depth,
                source_z,
                order_scale,
                front_part: 0.0,
                representation: CardBodyRepresentation::Sprite,
            });
        }
    }
    // Put the host, the limb pictures, and the parts above them in one seat.
    // Give each fragment part of the seat's paint range.
    seated_fragments.sort_unstable_by_key(|fragment| (fragment.host, fragment.over, fragment.source));
    for row in seated_fragments.chunk_by(|a, b| a.host == b.host) {
        let step = CARD_BODY_STEP / (row.len() + 1) as f32;
        let base = seated_body_depth(row[0].seat);
        if let Some(host_depth) = placements.get_mut(&row[0].host) {
            if let Ok((_, _, _, sample)) = owners.get(row[0].host) {
                host_depth.order_scale = step / rig_paint_span(sample);
            }
        }
        for (index, fragment) in row.iter().enumerate() {
            placements.insert(fragment.owner, CardBodyDepth {
                depth: CardDepth::new(base.plane, base.paint.0 + (index + 1) as f32 * step),
                source_z: fragment.source_z,
                order_scale: step / fragment.span,
                front_part: 0.0,
                representation: CardBodyRepresentation::Sprite,
            });
        }
    }
    if let Some(depths) = root_depths.as_deref_mut() {
        for (entity, ..) in &sprite_bodies {
            if let Some(depth) = placements.get(&entity) {
                depths.roots.insert(entity, *depth);
            }
        }
    }

    for (owner, of, presentation, sample) in &owners {
        shown.clear();
        let placement = (|| {
            let (root, _, flash, _, prop, visibility) = roots.get(of.0).ok()?;
            if !root_shows(visibility) {
                return None;
            }
            let depth = placements.get(&owner).copied().or_else(|| {
                if !prop || (sample.baked.is_none() && sample.feet.is_none()) {
                    return None;
                }
                let source_z = root.translation.z;
                let room = rooms.of(of.0);
                let projected = |z| root_depths.as_deref().map_or_else(
                    || CardDepth::on_plane(scenery_depth(z, CARD_BODY_STEP)),
                    |depths| depths.scenery(z, room),
                );
                let depth = projected(source_z);
                let span = projected(source_z + super::world::SCENERY_Z_STEP).paint.0 - depth.paint.0;
                let count = sample.parts().map_or(1, |parts| parts.len()).max(1);
                Some(CardBodyDepth {
                    depth,
                    source_z,
                    order_scale: span / (count as f32 * CARD_PART_STEP),
                    front_part: 0.0,
                    representation: CardBodyRepresentation::Sprite,
                })
            })?;
            let flash = flash.copied().filter(|flash| flash.intensity > 0.0);
            let baked = sample.baked.as_ref().or_else(|| presentation.composed_frame());
            if let Some(depths) = root_depths.as_deref_mut() {
                let front_part = sample.parts()
                    .filter(|_| baked.is_none())
                    .map_or(0.0, |parts| parts.len().saturating_sub(1) as f32 * CARD_PART_STEP);
                depths.roots.insert(of.0, CardBodyDepth {
                    front_part,
                    representation: if baked.is_some() {
                        CardBodyRepresentation::RiggedFrame
                    } else {
                        CardBodyRepresentation::RiggedParts
                    },
                    ..depth
                });
            }
            if let Some(baked) = baked {
                let page_size = images.get(&baked.image)?.size().as_vec2();
                let mesh = cache.quad(TextureQuad::baked(baked, page_size), &mut meshes);
                let material = if sample.body_color == LinearRgba::WHITE {
                    PartMaterial::Shared(cache.textured_material(&baked.image, [255; 4], &mut materials))
                } else {
                    PartMaterial::Tinted { image: baked.image.clone(), color: Color::LinearRgba(sample.body_color) }
                };
                shown.push(PartCardDraw { mesh, material, place: Transform::IDENTITY, paint: depth.depth.paint });
                return Some(depth.depth.place(root));
            }
            let draws = sample.parts()?;
            let feet = sample.feet?;
            let body_color = sample.direct_body_color();
            for (index, draw) in draws.iter().enumerate() {
                let Some(RiggedPartImage { page, rect, size, anchor }) = presentation.part_image(draw) else {
                    continue;
                };
                let Some(page_size) = images.get(page).map(|image| image.size().as_vec2()) else {
                    continue;
                };
                let mesh = cache.quad(TextureQuad {
                    rect,
                    page: page_size,
                    size,
                    anchor,
                    flip: [false; 2],
                }, &mut meshes);
                let material = if body_color == LinearRgba::WHITE {
                    PartMaterial::Shared(cache.textured_material(page, draw.color, &mut materials))
                } else {
                    PartMaterial::Tinted { image: page.clone(), color: part_color(draw, body_color) }
                };
                let place = card_part_local(draw);
                let offset = index as f32 * CARD_PART_STEP;
                let paint = depth.at_offset(offset).paint;
                if let Some(flash) = flash {
                    let silhouette = cache.flash(page, flash, &mut materials);
                    shown.push(PartCardDraw { mesh: mesh.clone(), material, place, paint });
                    shown.push(PartCardDraw {
                        mesh,
                        material: PartMaterial::Shared(silhouette),
                        place,
                        paint: depth.at_offset(offset + FLASH_STEP).paint,
                    });
                } else {
                    shown.push(PartCardDraw { mesh, material, place, paint });
                }
            }
            // Place the body root, then its feet.
            Some(depth.depth.place(&root.mul_transform(feet)))
        })();
        let existing = by_owner.get(&owner).and_then(|body| bodies.get_mut(*body).ok());
        match (placement.filter(|_| !shown.is_empty()), existing) {
            (Some(place), Some((body, mut card, mut transform, mut visibility))) => {
                transform.set_if_neq(place);
                visibility.set_if_neq(Visibility::Inherited);
                show_cards(&mut commands, &mut card, &mut cards, body, of.0, &layer, &mut materials, shown);
            }
            (Some(place), None) => {
                let body = commands.spawn((place, Visibility::Inherited, layer.clone(), Name::new("Card body"))).id();
                let mut card = CardBody { owner, parts: Vec::new() };
                show_cards(&mut commands, &mut card, &mut cards, body, of.0, &layer, &mut materials, shown);
                commands.entity(body).insert(card);
                by_owner.insert(owner, body);
            }
            (None, Some((_, card, _, mut visibility))) => {
                visibility.set_if_neq(Visibility::Hidden);
                hide_part_cards(&mut cards, &card.parts);
            }
            (None, None) => {}
        }
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
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut CardPaintDepth, &mut Visibility),
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
            .then(|| depths.held_depth(*root, place.translation.z))
            .flatten()
            .and_then(|depth| Some((depth, sprite_frame(sprite, anchor, &images, layouts.as_deref())?)));
        let Some((depth, (frame, page_size))) = frame else {
            if let Some(card) = by_art.get(&entity) {
                if let Ok((mesh, material, .., visibility)) = cards.get_mut(*card) {
                    hide_card(mesh, material, visibility);
                }
            }
            continue;
        };
        let (mesh, material) = sprite_card(sprite, &frame, page_size, &mut meshes, &mut materials, &mut cache);
        let card_place = depth.place(place);
        match by_art.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
            Some((mut card_mesh, mut card_material, mut transform, mut paint, mut card_visibility)) => {
                if card_mesh.0 != mesh {
                    card_mesh.0 = mesh;
                }
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(card_place);
                paint.set_if_neq(depth.paint);
                card_visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                let card = commands
                    .spawn((
                        CardHeldArt,
                        CardScope::Source(entity),
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        card_place,
                        depth.paint,
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
    let page_size = if sprite.image == Handle::default() {
        Vec2::ONE
    } else {
        images.get(&sprite.image)?.size().as_vec2()
    };
    let region = ambition_sprite_fx::sprite_texture_region(sprite, layouts, Some(page_size))?;
    let frame = BakedFrame {
        image: sprite.image.clone(),
        rect: region.rect,
        size: sprite.custom_size.unwrap_or_else(|| region.rect.size()),
        anchor: anchor.map_or(Vec2::ZERO, |anchor| anchor.0),
        flip_x: sprite.flip_x,
        flip_y: sprite.flip_y,
    };
    Some((frame, region.page_size))
}

/// A sprite's card quad and material, made once per cell and per colour.
fn sprite_card(
    sprite: &Sprite,
    frame: &BakedFrame,
    page_size: Vec2,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut CardMeshes,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let quad = TextureQuad::baked(frame, page_size);
    let mesh = cache.sprite_mesh(quad, &sprite.image_mode, meshes);
    let color = sprite.color.to_srgba().to_u8_array();
    let material = cache.textured_material(&frame.image, color, materials);
    (mesh, material)
}

fn sprite_card_place(place: &Transform, z: f32) -> Transform {
    place.with_translation(place.translation.truncate().extend(z))
}

fn sprite_card_seen(
    outputs: Option<&[OutputFootprint]>,
    place: &Transform,
    frame: &BakedFrame,
    mode: &SpriteImageMode,
) -> bool {
    let quad = TextureQuad::baked(frame, Vec2::ONE);
    let quad = match mode {
        SpriteImageMode::Auto => quad,
        SpriteImageMode::Scale(scale) => quad.scaled(*scale),
        SpriteImageMode::Sliced(_) | SpriteImageMode::Tiled { .. } => return true,
    };
    let (a, b) = quad.corners();
    let bounds = Rect::from_corners(a, b);
    outputs.is_none_or(|outputs| outputs.iter().any(|output| output.sees_quad(place, bounds)))
}

fn sprite_span_place(place: &Transform, width: f32, [start, end]: [f32; 2]) -> Transform {
    let mut span = sprite_card_place(place, (start + end) * 0.5);
    let planar_length = width * place.scale.x;
    if planar_length > 0.0 {
        let rise = end - start;
        span.rotation *= Quat::from_rotation_y(-rise.atan2(planar_length));
        span.scale.x *= planar_length.hypot(rise) / planar_length;
    }
    span
}

/// Realize each body drawn from a sheet (no rigged presentation) as one card
/// of the cell it draws, with the placement and paint key of its body.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_sprite_bodies(
    mut commands: Commands,
    bodies: Query<(Entity, &Sprite, Option<&bevy::sprite::Anchor>, &Transform), SpriteBodyFilter>,
    depths: Res<CardRootDepths>,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform, &mut CardPaintDepth, &mut Visibility),
        (With<CardSpriteBody>, Without<Sprite>, Without<MainCamera>),
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_body: Local<RetainedEntities<Entity>>,
    outputs: GameplayOutputs,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_body.reconcile(&mut commands, |body| bodies.contains(*body));
    let footprints = outputs.card_footprints();
    for (entity, sprite, anchor, place) in &bodies {
        // Ranked only when it is a shown body with no rigged presentation.
        let drawn = depths.roots.get(&entity)
            .filter(|depth| depth.representation == CardBodyRepresentation::Sprite)
            .and_then(|body| {
                let (frame, page_size) = sprite_frame(sprite, anchor, &images, layouts.as_deref())?;
                let card_place = body.depth.place(place);
                sprite_card_seen(footprints.as_deref(), &card_place, &frame, &sprite.image_mode)
                    .then_some((body.depth, card_place, frame, page_size))
            });
        let Some((depth, card_place, frame, page_size)) = drawn else {
            if let Some(card) = by_body.get(&entity) {
                if let Ok((mesh, material, .., visibility)) = cards.get_mut(*card) {
                    hide_card(mesh, material, visibility);
                }
            }
            continue;
        };
        let (mesh, material) = sprite_card(sprite, &frame, page_size, &mut meshes, &mut materials, &mut cache);
        match by_body.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
            Some((mut card_mesh, mut card_material, mut transform, mut paint, mut card_visibility)) => {
                if card_mesh.0 != mesh {
                    card_mesh.0 = mesh;
                }
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(card_place);
                paint.set_if_neq(depth.paint);
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
                        depth.paint,
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

/// Map the source z bands to card placement or paint keys.
fn scenery_depth(z: f32, front: f32) -> f32 {
    use ambition_platformer2d_core::config::{WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER};
    if z >= WORLD_Z_PLAYER {
        front + (z - WORLD_Z_PLAYER) * BAND_ORDER_SCALE
    } else if z >= WORLD_Z_DUMMY {
        SCENERY_DEPTH + (z - WORLD_Z_DUMMY) * BAND_ORDER_SCALE
    } else if z >= WORLD_Z_BLOCK {
        let fraction = (z - WORLD_Z_BLOCK) / (WORLD_Z_DUMMY - WORLD_Z_BLOCK);
        TURF_DEPTH.lerp(SCENERY_DEPTH, fraction)
    } else {
        BACKDROP_DEPTH + (z - WORLD_Z_BLOCK) * BAND_ORDER_SCALE
    }
}

fn scenery_paint_depth(z: f32, front: f32) -> f32 {
    use ambition_platformer2d_core::config::{WORLD_Z_DUMMY, WORLD_Z_PLAYER};
    if (WORLD_Z_DUMMY..WORLD_Z_PLAYER).contains(&z) {
        let fraction = (z - WORLD_Z_DUMMY) / (WORLD_Z_PLAYER - WORLD_Z_DUMMY);
        SCENERY_DEPTH + (SCENERY_FRONT_PAINT - SCENERY_DEPTH) * fraction
    } else {
        scenery_depth(z, front)
    }
}

fn source_room(source: Entity, rooms: &LiveRooms, parents: &Query<&ChildOf>) -> Option<LiveRoomInstance> {
    rooms.stamped(source)
        .or_else(|| parents.iter_ancestors(source).find_map(|parent| rooms.stamped(parent)))
        .or_else(|| rooms.of(source))
}

fn platform_card_depth(platform: &MovingPlatformVisual, looks: Option<&PlatformLooks>, source_z: f32) -> CardDepth {
    if platform.front {
        CardDepth::on_plane(-NEAR_PANEL_GAP)
    } else if platform.sheet.as_ref().and_then(|key| looks?.0.get(key)).is_some_and(|look| look.front_row.is_some()) {
        CardDepth::on_plane(-VESSEL_DEPTH)
    } else {
        CardDepth::new(OPEN_PLATFORM_Z, scenery_paint_depth(source_z, CARD_BODY_STEP))
    }
}

#[cfg(feature = "portal_render")]
type ScenerySource = Or<(With<WorldSprite>, With<ambition_portal2d_presentation::PortalVisual>)>;
#[cfg(not(feature = "portal_render"))]
type ScenerySource = With<WorldSprite>;

/// Draw world sprites at their propagated positions. A prop ranked as a
/// body uses the body renderer.
#[allow(clippy::too_many_arguments)]
pub(super) fn realize_card_scenery(
    mut commands: Commands,
    scenery: Query<
        (
            Entity, &Sprite, Option<&bevy::sprite::Anchor>, &Transform, &GlobalTransform,
            &InheritedVisibility, Option<&MovingPlatformVisual>, Has<AnimatedWorldSpriteTint>,
            Option<&PresentationOf>, Option<&WornGearVisual>, Option<&SlingString>, Has<DashAfterimage>,
        ),
        (ScenerySource, Without<CardScenery>),
    >,
    depths: Res<CardRootDepths>,
    platform_looks: Option<Res<PlatformLooks>>,
    rooms: LiveRooms,
    parents: Query<&ChildOf>,
    mut cards: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Transform,
            &mut GlobalTransform, &mut CardPaintDepth, &mut Visibility, &mut InheritedVisibility),
        (With<CardScenery>, Without<Sprite>, Without<MainCamera>),
    >,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut cache: ResMut<CardMeshes>,
    mut by_source: Local<RetainedEntities<Entity>>,
    outputs: GameplayOutputs,
) {
    let (Some(images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials) else {
        return;
    };
    by_source.reconcile(&mut commands, |source| scenery.contains(*source));
    let footprints = outputs.card_footprints();
    for (entity, sprite, anchor, local, global, visibility, platform, animated_tint, subject, gear, string, afterimage) in &scenery {
        let gear_depth = gear.and_then(|gear| depths.gear_depth(gear.order, gear.layer));
        let shown = visibility.get()
            && sprite.color.alpha() > 0.0
            && (gear.is_none() || gear_depth.is_some())
            && !depths.roots.contains_key(&entity);
        let Some((mut frame, page_size)) =
            sprite_frame(sprite, anchor, &images, layouts.as_deref()).filter(|_| shown)
        else {
            if let Some((mesh, material, .., visibility, mut inherited)) = by_source.get(&entity).and_then(|card| cards.get_mut(*card).ok()) {
                hide_card(mesh, material, visibility);
                inherited.set_if_neq(InheritedVisibility::HIDDEN);
            }
            continue;
        };
        let (mut card_place, paint) = if let Some(platform) = platform {
            let depth = platform_card_depth(platform, platform_looks.as_deref(), local.translation.z);
            (depth.place(local), depth.paint)
        } else if let Some([start, end]) = string.and_then(|string| depths.sling_depths(string.span)) {
            (
                sprite_span_place(&global.compute_transform(), frame.size.x, [start.plane, end.plane]),
                CardPaintDepth((start.paint.0 + end.paint.0) * 0.5),
            )
        } else if afterimage {
            let place = global.compute_transform();
            let z = place.translation.z;
            let room = source_room(entity, &rooms, &parents);
            let paint = depths.scene_paint_depth(z, room).unwrap_or_else(|| depths.scenery(z, room).paint);
            (CardDepth::on_plane(PLAY_PLANE_Z).place(&place), paint)
        } else {
            let place = global.compute_transform();
            let z = place.translation.z;
            let depth = gear_depth.or_else(|| depths.effect_depth(subject, z)).unwrap_or_else(|| {
                let room = if z >= ambition_platformer2d_core::config::WORLD_Z_PLAYER {
                    source_room(entity, &rooms, &parents)
                } else {
                    None
                };
                depths.scenery(z, room)
            });
            (depth.place(&place), depth.paint)
        };
        let card = by_source.get(&entity).and_then(|card| cards.get_mut(*card).ok());
        if !sprite_card_seen(footprints.as_deref(), &card_place, &frame, &sprite.image_mode) {
            if let Some((mesh, material, .., visibility, mut inherited)) = card {
                hide_card(mesh, material, visibility);
                inherited.set_if_neq(InheritedVisibility::HIDDEN);
            }
            continue;
        }
        // Keep the quad when the size of an unsliced sprite changes.
        if matches!(sprite.image_mode, SpriteImageMode::Auto) {
            card_place.scale *= frame.size.extend(1.0);
            frame.size = Vec2::ONE;
        }
        let mesh = cache.sprite_mesh(TextureQuad::baked(&frame, page_size), &sprite.image_mode, &mut meshes);
        let material = if animated_tint {
            let handle = card.as_ref().map(|(_, material, ..)| &material.0);
            retain_sprite_material(handle, &sprite.image, sprite.color, &mut materials)
        } else {
            let color = sprite.color.to_srgba().to_u8_array();
            cache.textured_material(&frame.image, color, &mut materials)
        };
        match card {
            Some((mut card_mesh, mut card_material, mut transform, mut global, mut order, mut card_visibility, mut inherited)) => {
                if card_mesh.0 != mesh {
                    card_mesh.0 = mesh;
                }
                if card_material.0 != material {
                    card_material.0 = material;
                }
                transform.set_if_neq(card_place);
                global.set_if_neq(GlobalTransform::from(card_place));
                order.set_if_neq(paint);
                card_visibility.set_if_neq(Visibility::Inherited);
                inherited.set_if_neq(InheritedVisibility::VISIBLE);
            }
            None => {
                let card = commands
                    .spawn((
                        CardScenery,
                        CardScope::Source(entity),
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        card_place,
                        GlobalTransform::from(card_place),
                        paint,
                        Visibility::Inherited,
                        InheritedVisibility::VISIBLE,
                        RenderLayers::layer(CARD_VIEW_LAYER),
                        Name::new("Card scenery"),
                    ))
                    .id();
                by_source.insert(entity, card);
            }
        }
    }
}

/// Face the panel toward the camera. Keep its 2D orientation when the view rolls.
fn backdrop_card_place(
    camera: &Transform,
    frame: &PresentedViewFrame,
    panel: Vec2,
    layer_z: f32,
    scale: f32,
) -> Transform {
    let along = if layer_z < FOREGROUND_PARALLAX_Z {
        -(BACKDROP_DISTANCE - layer_z)
    } else {
        FOREGROUND_DISTANCE + (layer_z - FOREGROUND_PARALLAX_Z) * BAND_ORDER_SCALE
    };
    let offset = (panel - frame.plane_center) * scale;
    let rotation = camera.rotation * Quat::from_rotation_z(-frame.roll_radians);
    Transform {
        translation: frame.plane_center.extend(0.0) + rotation * Vec3::new(offset.x, offset.y, along),
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
    cameras: Query<(
        Entity, &Camera, &Transform, &ambition_sim_view::PresentsView, &PlaneCameraForCard,
    ), With<CardViewCamera>>,
    planes: Query<&Camera, With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>>,
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
    by_panel.reconcile(&mut commands, |(panel, camera)| {
        panels.contains(*panel) && cameras.get(*camera).is_ok_and(|(_, camera, ..)| camera.is_active)
    });
    for (camera, output, camera_place, presents, plane) in &cameras {
        if !output.is_active {
            continue;
        }
        let scale = if settings.mode == CardViewMode::Inset {
            let Some((inset, plane_size)) = output.viewport.as_ref().zip(
                planes.get(plane.0).ok().and_then(Camera::physical_viewport_size),
            ) else {
                continue;
            };
            // Match the plane view's framing along the width of the inset.
            inset.physical_size.x as f32 / plane_size.x as f32 * settings.projection_scale(1.0)
        } else {
            1.0
        };
        let frame = frames.of(presents.0);
        for (panel, sprite, anchor, place, layer, visibility, key) in &panels {
            // The panel the 2D view draws for this camera's view: shown, and
            // kept for that view (or for no view in particular).
            let drawn = *visibility != Visibility::Hidden && key.is_none_or(|key| key.0 == presents.0);
            let made = frame.filter(|_| drawn).and_then(|frame| {
                Some((frame, sprite_frame(sprite, anchor, &images, layouts.as_deref())?))
            });
            let Some((frame, (cell, page_size))) = made else {
                if let Some((mesh, material, .., visibility)) =
                    by_panel.get(&(panel, camera)).and_then(|card| cards.get_mut(*card).ok())
                {
                    hide_card(mesh, material, visibility);
                }
                continue;
            };
            let (mesh, material) = sprite_card(sprite, &cell, page_size, &mut meshes, &mut materials, &mut cache);
            let card_place =
                backdrop_card_place(camera_place, frame, place.translation.truncate(), layer.z, scale);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::chain_skin::ChainTurfPiece;
    use crate::rendering::PropVisual;

    #[test]
    fn unused_card_assets_retire_while_other_views_keep_drawing() {
        use ambition_sim_view::LocalView;
        use bevy::asset::{AssetApp, AssetPlugin};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .insert_resource(CardViewSettings::for_game(CardViewMode::SideOn))
            .add_systems(Update, realize_card_scenery.run_if(card_outputs_enabled))
            .add_systems(PostUpdate, retire_card_outputs)
            .add_systems(Last, prune_card_cache.run_if(card_outputs_enabled));
        let world = app.world_mut();
        let first = world.spawn(LocalView).id();
        let second = world.spawn(LocalView).id();
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let image_id = image.id();
        let old = world.spawn((
            WorldSprite,
            Sprite {
                image,
                color: Color::srgb(0.3, 0.4, 0.5),
                custom_size: Some(Vec2::splat(20.0)),
                image_mode: SpriteImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 },
                ..default()
            },
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
        )).id();
        let live = world.spawn((
            WorldSprite,
            Sprite::from_color(Color::WHITE, Vec2::splat(25.0)),
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
        )).id();
        let assets_of = |world: &mut World, source| {
            world.query_filtered::<(
                Entity, &CardScope, &Mesh3d, &MeshMaterial3d<StandardMaterial>,
            ), With<CardScenery>>().iter(world)
                .find_map(|(card, scope, mesh, material)| {
                    (*scope == CardScope::Source(source))
                        .then_some((card, mesh.0.id(), material.0.id()))
                }).unwrap()
        };
        app.update();
        let (old_card, old_mesh, old_material) = assets_of(app.world_mut(), old);
        let active = assets_of(app.world_mut(), live);

        let world = app.world_mut();
        world.get_mut::<Sprite>(old).unwrap().image = Handle::default();
        world.entity_mut(old).insert(InheritedVisibility::HIDDEN);
        world.despawn(first);
        for _ in 0..(u16::from(CARD_CACHE_PASS_FRAMES) * 2 + 3) {
            app.update();
        }
        let world = app.world_mut();
        assert_eq!(world.get::<Visibility>(old_card), Some(&Visibility::Hidden));
        assert!(!world.resource::<Assets<Mesh>>().contains(old_mesh));
        assert!(!world.resource::<Assets<StandardMaterial>>().contains(old_material));
        assert!(!world.resource::<Assets<Image>>().contains(image_id));
        assert_eq!(assets_of(world, live), active);

        world.despawn(second);
        app.update();
        app.update();
        let world = app.world();
        assert!(world.get::<CardScenery>(active.0).is_none());
        assert!(!world.resource::<Assets<Mesh>>().contains(active.1));
        assert!(!world.resource::<Assets<StandardMaterial>>().contains(active.2));
        assert!(world.get::<Sprite>(live).is_some());
    }

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

    #[test]
    fn a_tilted_sling_card_keeps_its_endpoints_in_the_side_view() {
        let place = Transform::from_xyz(-20.0, 100.0, 2.0)
            .with_rotation(Quat::from_rotation_z(0.7))
            .with_scale(Vec3::new(1.5, 1.0, 1.0));
        let width = 18.0;
        let depths = [-7.0, 36.0];
        let sling = sprite_span_place(&place, width, depths);
        for (local_x, expected_depth) in [(-width * 0.5, depths[0]), (width * 0.5, depths[1])] {
            let point = Vec3::new(local_x, 0.0, 0.0);
            let expected = place.transform_point(point).truncate().extend(expected_depth);
            assert!(sling.transform_point(point).distance(expected) < 0.0001);
        }
    }

    /// RV3: a held item is a card on its holder's card depth, its 2D offset
    /// from the root kept in part steps (between the same two parts); one at
    /// an absolute 2D depth is held to the body's band; a hidden one, or one
    /// whose holder draws no card, shows none.
    #[test]
    fn held_art_is_a_card_on_its_holders_depth() {
        use ambition_platformer2d_shared_tangle::camera_layers::{MainCamera, PrivateCameraLayer};
        use ambition_sim_view::{LocalView, LocalViewId, PresentsView};

        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .insert_resource(CardViewSettings::for_game(CardViewMode::SideOn))
            .add_systems(Update, realize_card_held_art)
            .add_systems(PostUpdate, (gameplay::route_card_outputs, gameplay::isolate_cards).chain());
        let view = app.world_mut().spawn((LocalView, LocalViewId::FIRST)).id();
        app.insert_resource(PresentedViewFrames::of_views(vec![(view, PresentedViewFrame {
            room: LiveRoomInstance::ACTIVATION,
            plane_center: Vec2::ZERO,
            roll_radians: 0.0,
            orthographic_scale: 1.0,
        })]));
        app.world_mut().spawn((Camera2d, MainCamera, PresentsView(view)));
        app.world_mut().spawn((Camera3d::default(), CardViewCamera, PresentsView(view), RenderLayers::none()));
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        let root = app.world_mut().spawn_empty().id();
        let no_card = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<CardRootDepths>().roots.insert(root, CardBodyDepth {
            depth: CardDepth::new(-20.0, -21.0), source_z: 5.0, order_scale: 1.0, front_part: 0.0,
            representation: CardBodyRepresentation::Sprite,
        });
        let item = |at: Vec3, holder| {
            (
                Sprite { image: image.clone(), custom_size: Some(Vec2::new(10.0, 4.0)), ..default() },
                Transform::from_translation(at),
                HeldArtOf(holder),
            )
        };
        // Three slots in front of the root, as a hand's item is.
        let in_hand = app.world_mut().spawn(item(Vec3::new(30.0, 40.0, 5.0 + 3.0 * SLOT_STEP_2D), root)).id();
        app.world_mut().spawn(item(Vec3::new(0.0, 0.0, 12.0), root));
        app.world_mut().spawn((item(Vec3::ZERO, root), Visibility::Hidden));
        app.world_mut().spawn(item(Vec3::ZERO, no_card));
        app.update();
        let world = app.world_mut();
        let mut cards = world.query_filtered::<(&Transform, &CardPaintDepth, &Visibility), With<CardHeldArt>>();
        let mut places: Vec<_> = cards
            .iter(world)
            .filter(|(_, _, visibility)| **visibility != Visibility::Hidden)
            .map(|(place, paint, _)| (place.translation, paint.0))
            .collect();
        places.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
        assert_eq!(places.len(), 2, "the hidden item and the one whose holder has no card show none");
        assert_eq!(places[0], (Vec3::new(0.0, 0.0, -20.0), -21.0 + HELD_ART_REACH));
        assert_eq!(places[1].0, Vec3::new(30.0, 40.0, -20.0));
        assert!((places[1].1 - (-21.0 + 3.0 * CARD_PART_STEP)).abs() < 1e-4, "three part steps in front");

        let item_card = world.query_filtered::<(Entity, &Transform), With<CardHeldArt>>()
            .iter(world)
            .find(|(_, place)| place.translation.x == 30.0)
            .unwrap().0;
        let private = RenderLayers::none().with(PrivateCameraLayer::PortalCarry.at(0));
        let output = RenderLayers::none().with(PrivateCameraLayer::CardOutput.at(0));
        world.entity_mut(root).insert(private.clone());
        app.update();
        assert!(app.world().get::<RenderLayers>(item_card).unwrap().intersects(&output));

        app.world_mut().entity_mut(in_hand).insert(private);
        app.update();
        assert!(!app.world().get::<RenderLayers>(item_card).unwrap().intersects(&output));

        app.world_mut().entity_mut(in_hand).remove::<RenderLayers>();
        app.update();
        assert!(app.world().get::<RenderLayers>(item_card).unwrap().intersects(&output));
    }

    #[test]
    fn worn_gear_uses_the_wearers_back_and_anchor_slots() {
        let mut world = World::new();
        let wearer = world.spawn_empty().id();
        let order = WornGearOrder { root: wearer, anchor_part: Some(2) };
        let mut depths = CardRootDepths::default();
        let mut body = CardBodyDepth {
            depth: CardDepth::new(-2.0, 4.0),
            order_scale: 0.5,
            source_z: 20.0,
            front_part: 5.0 * CARD_PART_STEP,
            representation: CardBodyRepresentation::RiggedParts,
        };
        depths.roots.insert(wearer, body);
        let part = |index| body.at_offset(index as f32 * CARD_PART_STEP).paint.0;
        let back = depths.gear_depth(order, GearLayer::Back).unwrap();
        let front = depths.gear_depth(order, GearLayer::Front).unwrap();
        assert_eq!((back.plane, front.plane), (-2.0, -2.0));
        assert!(back.paint.0 < part(0) && part(2) < front.paint.0 && front.paint.0 < part(3));

        body.front_part = 0.0;
        body.representation = CardBodyRepresentation::RiggedFrame;
        depths.roots.insert(wearer, body);
        let front = depths.gear_depth(order, GearLayer::Front).unwrap();
        assert!(back.paint.0 < body.depth.paint.0 && body.depth.paint.0 < front.paint.0);
        assert!(front.paint.0 < body.at_offset(CARD_PART_STEP).paint.0);
        depths.roots.remove(&wearer);
        assert!(depths.gear_depth(order, GearLayer::Front).is_none());
    }

    #[test]
    fn a_body_keeps_its_plane_when_the_paint_order_changes() {
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
        let enters = app.world_mut().spawn((
            sprite(Vec3::new(90.0, 0.0, 1.0)), actor(FeatureVisualKind::Actor), Visibility::Hidden,
        )).id();
        app.update();
        let draw = |app: &mut App| {
            let world = app.world_mut();
            let mut rows: Vec<_> = world.query_filtered::<
                (Entity, &Transform, &CardPaintDepth, &Visibility), With<CardSpriteBody>,
            >().iter(world).filter(|(_, _, _, visibility)| **visibility != Visibility::Hidden)
                .map(|(entity, transform, paint, _)| (entity, transform.translation, paint.0))
                .collect();
            rows.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
            rows
        };
        let before = draw(&mut app);
        assert_eq!(before.len(), 2);
        assert_eq!(before[0].1, Vec3::new(-10.0, 0.0, 0.0));
        assert_eq!(before[1].1, Vec3::new(10.0, 0.0, 0.0));
        assert!(before[0].2 < before[1].2);
        let depths = app.world().resource::<CardRootDepths>();
        let under_both = depths.scene_paint_depth(2.95, None).unwrap().0;
        let between = depths.scene_paint_depth(6.94, None).unwrap().0;
        let nearer_light = depths.scene_paint_depth(6.96, None).unwrap().0;
        assert!(under_both < before[0].2);
        assert!(before[0].2 < between && between < nearer_light && nearer_light < before[1].2);
        assert!(depths.scene_paint_depth(7.5, None).is_none());
        assert!(depths.scene_paint_depth(2.95, Some(LiveRoomInstance::ACTIVATION)).is_none());
        app.world_mut().entity_mut(enters).insert(Visibility::Inherited);
        app.update();
        let after = draw(&mut app);
        assert_eq!(after.len(), 3);
        for (before, after) in before.iter().zip(&after) {
            assert_eq!((before.0, before.1), (after.0, after.1));
            assert!(before.2 < after.2);
        }
        assert!(after[2].2 < after[0].2 && after[0].2 < after[1].2);
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
        let scenery = app.world_mut().spawn((sprite(Vec3::new(10.0, 0.0, 10.6)), prop(), AnimatedWorldSpriteTint)).id();
        app.world_mut().spawn((sprite(Vec3::new(20.0, 0.0, 21.75)), prop()));
        app.world_mut().spawn((sprite(Vec3::new(30.0, 0.0, -11.0)), prop()));
        app.world_mut().spawn((sprite(Vec3::new(40.0, 0.0, 2.01)), ChainTurfPiece));
        app.world_mut().spawn((sprite(Vec3::new(50.0, 0.0, 10.7)), prop())).insert(InheritedVisibility::HIDDEN);
        let actor = app.world_mut().spawn((
            sprite(Vec3::new(60.0, 0.0, 19.0)),
            prop(),
            BoundFeatureKind::new(FeatureVisualKind::Actor, Vec2::new(16.0, 24.0)),
        )).id();
        app.world_mut().spawn((sprite(Vec3::new(70.0, 0.0, 18.94)), WorldSprite, PresentationOf(actor)));
        app.world_mut().spawn((sprite(Vec3::new(80.0, 0.0, 19.8)), WorldSprite, PresentationOf(actor)));
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
        assert_eq!(xs, vec![10.0, 20.0, 30.0, 40.0, 70.0, 80.0], "no hidden prop, and the actor prop is a body");
        let depth = |x: f32| found.iter().find(|at| at.x == x).unwrap().z;
        let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(near(depth(10.0), SCENERY_DEPTH + 0.06), "scenery in its band: {}", depth(10.0));
        assert!(near(depth(20.0), CARD_BODY_STEP + 0.175), "foreground plane: {}", depth(20.0));
        assert!(near(depth(30.0), BACKDROP_DEPTH - 1.1), "backdrop: {}", depth(30.0));
        assert!(depth(40.0) >= TURF_DEPTH, "turf: {}", depth(40.0));
        assert!(depth(30.0) < -VESSEL_DEPTH, "a backdrop is behind the cart's back panel");
        assert!(TERRAIN_DEPTH + 0.04 < depth(40.0) && depth(40.0) < depth(10.0) && depth(10.0) < PLAY_PLANE_Z,
            "skin, turf, scenery, then the portal line");
        assert_eq!(depth(70.0), 0.0);
        assert_eq!(depth(80.0), 0.0);
        let world = app.world_mut();
        let paint_at = |world: &mut World, x| world.query_filtered::<(&Transform, &CardPaintDepth), With<CardScenery>>()
            .iter(world).find_map(|(place, paint)| (place.translation.x == x).then_some(paint.0)).unwrap();
        assert!(paint_at(world, 20.0) > body_paint_z(false, 1));
        assert!(paint_at(world, 70.0) < body_paint_z(false, 0));
        let front_effect = paint_at(world, 80.0);
        assert!(body_paint_z(false, 0) < front_effect && front_effect < body_paint_z(false, 1));
        let mut bodies = app.world_mut().query_filtered::<(), With<CardSpriteBody>>();
        assert_eq!(bodies.iter(app.world()).count(), 2, "the player and the actor prop");
        let card_of_scenery = {
            let world = app.world_mut();
            let mut cards = world.query_filtered::<(Entity, &CardScope), With<CardScenery>>();
            cards.iter(world).find_map(|(card, scope)| {
                matches!(scope, CardScope::Source(source) if *source == scenery).then_some(card)
            }).unwrap()
        };
        let mesh = app.world().get::<Mesh3d>(card_of_scenery).unwrap().0.clone();
        let material = app.world().get::<MeshMaterial3d<StandardMaterial>>(card_of_scenery).unwrap().0.clone();
        let size = Vec2::new(32.0, 40.0);
        let color = Color::srgba(0.5, 0.8, 1.0, 0.6);
        let mut prop_sprite = app.world_mut().get_mut::<Sprite>(scenery).unwrap();
        prop_sprite.custom_size = Some(size);
        prop_sprite.color = color;
        app.update();
        assert_eq!(app.world().get::<Mesh3d>(card_of_scenery).unwrap().0, mesh);
        assert_eq!(app.world().get::<Transform>(card_of_scenery).unwrap().scale.truncate(), size);
        assert_eq!(app.world().get::<MeshMaterial3d<StandardMaterial>>(card_of_scenery).unwrap().0, material);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().get(&material).unwrap().base_color, color);
        app.world_mut().entity_mut(card_of_scenery).despawn();
        app.update();
        assert_eq!(shown(&mut app), found, "the card returns while its source exists");
        app.world_mut().entity_mut(scenery).despawn();
        app.update();
        let remaining: Vec<_> = found.into_iter().filter(|at| at.x != 10.0).collect();
        assert_eq!(shown(&mut app), remaining, "removing a prop removes its card");
    }

    #[test]
    fn a_ladder_keeps_its_place_when_room_art_replaces_its_rungs() {
        use ambition_platformer2d_core as core;
        use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
        use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxTheme, RoomDressingPart};
        use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        use crate::rendering::terrain_skin::{FixtureArtPiece, ThemedFixture};

        let mut app = App::new();
        app.add_plugins((bevy::transform::TransformPlugin, bevy::camera::visibility::VisibilityPlugin))
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .add_systems(Startup, |mut commands: Commands| {
                let mut world = core::World::new(
                    "ladder", Vec2::splat(256.0), Vec2::ZERO, Vec::new(),
                );
                world.climbable_regions.push(core::ClimbableRegion::ladder(
                    core::aabb_from_min_size(Vec2::new(144.0, 32.0), Vec2::new(16.0, 64.0)),
                ));
                let mut room = ambition_platformer2d_world::rooms::RoomSpec::new("ladder", world);
                room.metadata.visual_profile.parallax_theme = Some("cave".into());
                super::super::world::spawn_room_visuals(
                    &mut commands, SessionSpawnScope::UNSCOPED, &room,
                    &|_| ambition_platformer2d_world::rooms::ZoneWay::BothWays, None, None,
                );
            })
            .add_systems(Update, super::super::terrain_skin::dress_themed_fixtures)
            .configure_sets(
                PostUpdate,
                world_mesh::CardWorldMeshSet
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            )
            .add_systems(PostUpdate, realize_card_scenery.in_set(world_mesh::CardWorldMeshSet));

        let cards = |app: &mut App| {
            let world = app.world_mut();
            let mut query = world.query_filtered::<(&CardScope, &GlobalTransform, &Visibility), With<CardScenery>>();
            query.iter(world).filter_map(|(scope, transform, visibility)| {
                let CardScope::Source(source) = *scope else { return None };
                (*visibility != Visibility::Hidden).then_some((source, transform.translation()))
            }).collect::<Vec<_>>()
        };
        app.update();
        let plain = cards(&mut app);
        assert_eq!(plain.len(), 5, "the ladder body and four rungs are visible");
        let (root, body) = *plain.iter().find(|(source, _)| app.world().get::<ThemedFixture>(*source).is_some()).unwrap();
        assert_eq!(body.truncate(), Vec2::new(24.0, 64.0));
        assert!(plain.iter().all(|(source, at)| *source == root || at.x == body.x && at.z > body.z));

        let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::new_fill(
            Extent3d { width: 16, height: 32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[255; 4],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let mut assets = GameAssets::default();
        assets.room_dressing.insert(ParallaxTheme::new("cave"), RoomDressingPart::Ladder, image);
        app.insert_resource(assets);
        app.update();
        let themed = cards(&mut app);
        assert_eq!(themed.len(), 2, "two tiles replace the body and rungs");
        assert!(themed.iter().all(|(source, at)| {
            app.world().get::<FixtureArtPiece>(*source).is_some()
                && at.x == body.x && (at.y - body.y).abs() == 16.0 && at.z > body.z
        }));
    }

    #[test]
    fn a_rolled_backdrop_keeps_its_framing_on_a_switch_from_inset_to_side_on() {
        use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
        use ambition_sim_view::{LocalView, PresentsView};
        use bevy::camera::RenderTargetInfo;

        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardViewSettings>()
            .add_systems(Update, place_card_view_cameras)
            .add_systems(PostUpdate, (gameplay::route_card_outputs, realize_card_backdrop).chain());
        let view = app.world_mut().spawn(LocalView).id();
        let other = app.world_mut().spawn_empty().id();
        let frame = super::super::camera::PresentedViewFrame {
            room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
            plane_center: Vec2::new(100.0, 50.0),
            roll_radians: std::f32::consts::FRAC_PI_2,
            orthographic_scale: 1.0,
        };
        app.insert_resource(PresentedViewFrames::of_views(vec![(view, frame)]));
        let settings = CardViewSettings::default();
        let plane_size = UVec2::new(623, 971);
        let mut plane = Camera::default();
        plane.computed.target_info = Some(RenderTargetInfo { physical_size: plane_size, scale_factor: 1.5 });
        app.world_mut().spawn((MainCamera, Camera2d, plane, PresentsView(view)));
        let camera = app.world_mut().spawn((
            CardViewCamera, Camera::default(), Projection::Orthographic(card_camera_projection()), PresentsView(view),
        )).id();
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
        let mut cards = world.query_filtered::<(Entity, &Transform, &Visibility), With<CardBackdrop>>();
        let shown: Vec<_> = cards.iter(world).filter(|(_, _, v)| **v != Visibility::Hidden)
            .map(|(id, place, _)| (id, *place)).collect();
        assert_eq!(shown.len(), 2, "the two panels drawn for this view: {shown:?}");
        let target = frame.plane_center.extend(0.0);
        let rotation = world.get::<Transform>(camera).unwrap().rotation;
        let local = |place: &Transform| rotation.inverse() * (place.translation - target);
        let (back_id, back) = shown.iter().find(|(_, place)| local(place).z < 0.0).expect("a background card");
        let (_, front) = shown.iter().find(|(_, place)| local(place).z > 0.0).expect("a foreground card");
        let viewport = world.get::<Camera>(camera).unwrap().viewport.as_ref().unwrap();
        let scale = viewport.physical_size.x as f32 / plane_size.x as f32 * settings.projection_scale(1.0);
        assert!(local(back).distance(Vec3::new(0.0, -200.0 * scale, -(BACKDROP_DISTANCE + 16.0))) < 1e-2, "{:?}", local(back));
        assert!(local(front).distance(Vec3::new(100.0 * scale, 0.0, FOREGROUND_DISTANCE + 0.5)) < 1e-2, "{:?}", local(front));
        let panel_in_view = rotation.inverse() * back.rotation;
        assert!((panel_in_view * Vec3::X).distance(-Vec3::Y) < 1e-5);
        assert!((panel_in_view * Vec3::Z).distance(Vec3::Z) < 1e-5);
        assert_eq!(back.scale, Vec3::new(scale, scale, 1.0));
        assert!(local(back).z.abs() < CAMERA_DISTANCE, "inside the camera's far plane");

        app.world_mut().resource_mut::<CardViewSettings>().mode = CardViewMode::SideOn;
        app.update();
        let world = app.world();
        let rotation = world.get::<Transform>(camera).unwrap().rotation;
        let back = world.get::<Transform>(*back_id).expect("keep the same backdrop card");
        let moved = rotation.inverse() * (back.translation - target);
        assert!(moved.distance(Vec3::new(0.0, -200.0, -(BACKDROP_DISTANCE + 16.0))) < 1e-2, "{moved:?}");
        assert_eq!(back.scale, Vec3::ONE);
    }

    /// RV3: a portal is a thin card along its opening at its glow's place
    /// and turn, in its look's colour, as strong as the glow is open; it
    /// leaves with its glow.
    #[test]
    fn a_moving_cart_uses_its_two_sprite_layers_around_the_rider() {
        use super::super::moving_platforms::PlatformLook;

        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CardMeshes>()
            .init_resource::<CardRootDepths>()
            .insert_resource(PlatformLooks(HashMap::from([("cart".into(), PlatformLook {
                sheet: None,
                row: "body".into(),
                frame_px: [80.0, 60.0],
                span_px: [4.0, 76.0],
                top_px: 32.0,
                faces_right: true,
                front_row: Some("rail".into()),
                rim_px: Some(12.0),
            })])))
            .add_systems(Update, realize_card_scenery);
        let origin = Vec3::new(25.0, 50.0, 5.0);
        let mut spawn_layer = |front| {
            let world = app.world_mut();
            let image = world.resource_mut::<Assets<Image>>().add(Image::default());
            let source = world.spawn((
                Sprite::from_image(image.clone()),
                Transform::from_translation(origin),
                InheritedVisibility::VISIBLE,
                MovingPlatformVisual { room: LiveRoomInstance::ACTIVATION, index: 0, sheet: Some("cart".into()), front },
            )).id();
            (source, image)
        };
        let (back_source, back_image) = spawn_layer(false);
        let (near_source, near_image) = spawn_layer(true);
        app.update();

        let draw_of = |app: &mut App, source| {
            let world = app.world_mut();
            let mut cards = world.query_filtered::<(Entity, &CardScope, &Transform, &MeshMaterial3d<StandardMaterial>), With<CardScenery>>();
            let (card, _, place, material) = cards.iter(world)
                .find(|(_, scope, ..)| **scope == CardScope::Source(source)).unwrap();
            let material = world.resource::<Assets<StandardMaterial>>().get(&material.0).unwrap();
            assert_eq!(material.alpha_mode, AlphaMode::Blend);
            (card, place.translation, material.base_color_texture.clone().unwrap())
        };
        let (_, back, back_texture) = draw_of(&mut app, back_source);
        let (near_card, near, near_texture) = draw_of(&mut app, near_source);
        assert_eq!((back_texture, near_texture), (back_image, near_image));
        assert_eq!(back.truncate(), origin.truncate());
        assert_eq!(near.truncate(), origin.truncate());
        let seated = -VESSEL_DEPTH * 0.5;
        let free = 0.0;
        assert!(back.z < seated && seated < near.z && near.z < free);
        assert!(body_paint_z(true, 100) > back.z);

        app.world_mut().get_mut::<Transform>(near_source).unwrap().translation.x += 10.0;
        app.update();
        let (retained_card, moved, _) = draw_of(&mut app, near_source);
        assert_eq!(retained_card, near_card);
        assert_eq!(moved, near + Vec3::X * 10.0);
    }
}
