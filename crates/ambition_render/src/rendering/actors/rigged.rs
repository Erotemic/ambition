//! The rigged-sprite realization: a character drawn from its sheet's transform
//! flipbook with a fixed set of reusable part sprites
//! (`docs/planning/engine/semantic-part-rendering-and-ragdolls.md`).
//!
//! The baked sheet path stays in charge of WHAT is drawn. The root's
//! [`CharacterAnimator`] still picks the row, the frame and the facing from the
//! same simulation facts, and this realization only draws that frame from
//! parts. So the rigged path cannot pick another clip or another timing.
//!
//! ⭐ THE PARTS DRAW IN THE WORLD. Each part slot is a sprite in the root's own
//! render layers, under an owner standing at the body's feet and scaled from
//! sheet pixels to world units through the same quad the animator gave the
//! root (`body_quad`: size, anchor, facing, squash, mirror row). The root draws
//! nothing. No offscreen pass, no camera: 100 bodies cost 2.5 ms over their
//! baked sheets on llvmpipe, where compositing every body cost 49 ms
//! (`examples/rigged_sprite_bench.rs`, 2026-10-05).
//!
//! ⭐ A BODY READ AS ONE IMAGE IS COMPOSITED, WHILE IT IS READ. Some readers
//! take a body as one picture: the hit flash's silhouette, a portal's clipped
//! pieces, a content overlay such as Mary-O's star power or the puppy slug's
//! dream, and a frame that fades as one picture (`opacity(composite(parts))`).
//! A reader declares the root in [`ComposedBodyDemand`] each frame it reads
//! it; the body then takes a cell of a shared offscreen atlas, a private camera
//! draws its parts there, and the ROOT draws that cell at its own size and
//! feet, with its own tint and flip. It goes back to the world
//! [`COMPOSED_HOLD_FRAMES`] after the last read and gives its cell back.
//!
//! ⛔ ONE COMPOSITING LAW FOR BOTH ROADS. The world cameras blend in linear
//! light (`WORLD_COMPOSITING`; gamma there blacked out every window), and so
//! does the atlas (`impostor_compositing`), so a body that switches road on a
//! hit flash does not change its outlines. The art was composited in gamma
//! space (`ART_COMPOSITING`): against it the runtime drifts a shade on
//! anti-aliased outlines, measured and accepted, not chased.
//!
//! ⭐ ONE ATLAS FOR EVERY BODY OF A SIZE ([`RiggedImpostorAtlas`]). Each
//! composited body is a cell of a shared target, and one camera draws every
//! cell of a page in one pass. There is one atlas per cell size
//! ([`IMPOSTOR_CELL_CLASSES`]): a body takes the smallest cell its frame fits.
//! When every cell of a class is taken, the class opens one more PAGE, one
//! growth step larger than the last; a page never regrows, so a body already
//! composited keeps its target, and the last page of a class is retired once
//! its last body leaves. The private camera draws
//! [`RIGGED_IMPOSTOR_LAYER`], and the cells stand far below any world
//! ([`impostor_cell_feet`]).
//!
//! ⛔ TWO CAMERAS, BECAUSE A SPRITE OVER A TRANSPARENT CLEAR STORES
//! PREMULTIPLIED COLOUR. A sprite blends `src * a + dst * (1 - a)`, so the
//! first camera's target holds half the colour of a half-covered pixel — every
//! anti-aliased edge and every translucent effect layer. Drawn by the root as
//! the straight-alpha texture it is taken for, those pixels would darken a
//! second time. The second camera draws one quad that divides the colour back
//! out ([`ImpostorUnpremultiply`]), and the root draws ITS target.
//!
//! Each rigged root gets one presentation OWNER whose children are the part
//! slots. The owner is not a child of the root, because a player's root is its
//! simulation body, and the body must not grow presentation children. It is
//! related to its root ([`RiggedPresentationOf`], [`RiggedPresentedBy`]), and
//! the relationship despawns the owner and its slots with the root. So the
//! parts of a body that is gone are gone also where no presentation system
//! runs: a quit to the title despawns the session's bodies, and the systems
//! that draw parts run only while a session is presented.
//!
//! The slots are allocated once per flipbook (its most draws in one frame) and
//! reused: a frame change writes their rect, transform and visibility, and
//! never spawns or despawns one.
//!
//! A sheet carries its part pages from the frame they are requested, still
//! loading, as it carries its own pages. So a body changes to its parts only
//! when every part page is ready ([`super::texture_is_ready`], the rule of the
//! baked binder). Until then it keeps what it draws now: its baked sprite, or
//! the parts of the tier it had. Then it changes in one frame. A page that
//! fails to load is never ready, and the body stays as it was. A re-wear to
//! another character does not keep the old character's parts: they are
//! dropped at once, and the root draws the new character's baked sheet until
//! its pages are ready.
//!
//! A hybrid flipbook leaves some rows to the baked sheet. For such a row,
//! and for a frame with no row or no render basis, the root draws its baked
//! frame (its page and atlas index, from its animator). The root, its feet and
//! its animator are the same in both cases, so a body moves between a baked
//! clip and a part clip with no jump in place or in timing.
//!
//! A cell is the frame plus [`impostor_margin`] sheet pixels each side, so art
//! that runs past the baked frame (Mary-O's feet, up to 8 px) is drawn, not
//! cut. A body whose frame fits no cell draws directly even when read as one
//! image (and says so): its readers see no image.
//!
//! ⛔ EVERY PIXEL A COMPOSITED BODY DRAWS LIES IN ITS CELL. One camera draws a
//! whole page with no scissor, so a part past its cell is cut from its own
//! body and drawn into its neighbour's. Three kinds of draw, three proofs:
//! a published frame and a tween's in-between are inside the margin by the
//! flipbook's stated overhang (`RiggedSpriteAsset::art_overhang`, a
//! conservative envelope taken when the flipbook is read); parts placed by a
//! [`PartPose`] are in no file, so they are measured each frame
//! (measured on its [`RiggedLocalSample`]), and a body whose pose reaches past its cell draws
//! directly while it does, as a body that fits no cell.
//!
//! ⛔ Nothing here runs unless [`RiggedSpriteAdmission`] admits the flipbooks
//! (on by default since 2026-10-01). The crouch squash of a sheet without a
//! crouch row squashes the root's quad about the line it holds still, as it
//! squashes a baked quad (`stance_squash`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
mod readiness;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::shader::ShaderRef;
use bevy::sprite::Anchor;
use bevy::sprite_render::{AlphaMode2d, Material2d, MeshMaterial2d};

use ambition_persistence::settings::TextureResolutionScale;
use ambition_platformer2d_shared_tangle::camera_layers::RIGGED_IMPOSTOR_LAYER;

use crate::rendering::camera::coverage::{GameplayCoverage, GameplayOutputs, PlayPlane};
use crate::rendering::{impostor_compositing, ART_COMPOSITING};
use ambition_sprite_sheet::character::rigged::{
    BodyWarp, ComposedBodyDemand, FrameInSprite, PartDraw, PartPose, PartPresentation, PerformedBodyWarp, PosedParts, Swing,
    RiggedSpriteAdmission, RiggedSpriteAsset, RiggedSpritePages, BLINK_WARP_S,
};
use ambition_sprite_sheet::character::{CharacterAnimator, CharacterColorShift};
use ambition_sprite_sheet::game_assets::GameAssets;

use super::{sprite_mirror, BoundSpriteQuality};

/// Depth between two part slots of one body: the draw order of the flipbook.
const SLOT_DEPTH_STEP: f32 = 1.0e-4;

/// A part draw's place in its body: offset, turn, scale, and its rank in the
/// body's draw order. Under its owner one unit is one sheet pixel from the
/// feet, +y up; clockwise in the sheet's +y-down frame is a negative angle.
/// The scale is in the transform, not the quad, so a mirrored draw
/// (`scale.x < 0`) mirrors about its pivot, then turns. The body's own slots
/// (`write_slots`) and a reader's placed parts (`part_set::answer`) both
/// take their placement from here.
pub(super) fn slot_local(draw: &PartDraw, index: usize) -> Transform {
    part_local_transform(draw, index as f32 * SLOT_DEPTH_STEP)
}

pub(in crate::rendering) fn part_local_transform(draw: &PartDraw, depth: f32) -> Transform {
    Transform::from_translation(Vec3::new(draw.at.x, -draw.at.y, depth))
        .with_rotation(Quat::from_rotation_z(-draw.rotation))
        .with_scale(draw.scale.extend(1.0))
}

/// Frames a body stays composited after the last frame something read it as
/// one image: half a second at 60 Hz, so a demand that flickers (a blink, a
/// flash cue, a body edging along a portal) does not move it between the
/// atlas and the world every frame.
pub const COMPOSED_HOLD_FRAMES: u16 = 30;

/// The least transparent sheet pixels on each side of the frame in a body's
/// cell ([`impostor_margin`] widens it for art that reaches farther).
pub const IMPOSTOR_MARGIN: f32 = 16.0;

/// The margin around `flipbook`'s frame in its cell: [`IMPOSTOR_MARGIN`], or
/// the farthest its art reaches past the frame plus the two-pixel fringe a
/// resampled part's border adds, whichever is more.
///
/// ⛔ A FIXED MARGIN CLIPPED A COMPOSITED BODY. The oni leader's banner and ears
/// reach past the frame by more than 16 px in its jumps: drawn directly they
/// showed, composited (on every hit flash) they were cut off — the largest road
/// switch measured, a blob of 522 px (`measure_composition_switch.py`,
/// 2026-10-05).
pub fn impostor_margin(flipbook: &ambition_sprite_sheet::character::rigged::RiggedSpriteAsset) -> f32 {
    IMPOSTOR_MARGIN.max(flipbook.art_overhang.ceil() + IMPOSTOR_FRINGE)
}

/// The fringe, in sheet pixels, a resampled part's border adds past its quad.
pub const IMPOSTOR_FRINGE: f32 = 2.0;

/// The cell sizes of the impostor atlases, in sheet pixels (one texel each),
/// smallest first, with the most cells per side each atlas grows to. A frame
/// with its margins: the player robot's 288 x 288, Mary-O's 192 x 224 and a
/// pirate's about 135 x 146 take the first; Noether's 528 x 560 the second;
/// the Perfect Cellular Automaton's 686 x 878 the third; Wolfenstein's
/// 956 x 956 the fourth (a 924 px frame at the game's texel density: in the
/// third, his hit flash had no image and showed nothing, 2026-10-19).
///
/// A page of the first grows to 6 x 6 = 36 bodies (two 1728-texel targets,
/// about 24 MB); a page of a larger one to 4 x 4 (2304 and 3584 texels a
/// side), or 3 x 3 for the largest (3456), for the few large bodies a room
/// has. The first page is one cell,
/// made for the first body that needs it; each page after is one step larger
/// than the last (1, 2, 4, then its most), opened when every cell is taken.
///
/// ⛔ GROWN, NOT SIZED FOR THE MOST. Every frame the cameras clear and the
/// un-premultiplying quad shades the whole target: at 6 x 6 that cost a single
/// rigged body 13 ms on llvmpipe (measured 2026-10-02), against a quarter of
/// that for the one cell it needs.
pub const IMPOSTOR_CELL_CLASSES: [(f32, u32); 4] = [(288.0, 6), (576.0, 4), (896.0, 4), (1152.0, 3)];

/// The smallest cell, the one most bodies take.
pub const IMPOSTOR_CELL: f32 = IMPOSTOR_CELL_CLASSES[0].0;

/// The most cells per side any atlas grows to.
pub const IMPOSTOR_MAX_CELLS_PER_SIDE: u32 = 6;

/// The class of the smallest cell a frame of `frame_size` fits with `margin`
/// on each side, or `None` when it fits none.
pub fn impostor_cell_class(frame_size: Vec2, margin: f32) -> Option<usize> {
    let needed = (frame_size + Vec2::splat(2.0 * margin)).max_element();
    IMPOSTOR_CELL_CLASSES.iter().position(|(cell, _)| needed <= *cell)
}

/// The next page size after `side` cells per side, for a page below its
/// `most`.
fn grown(side: u32, most: u32) -> u32 {
    match side {
        0 => 1,
        1 => 2,
        2 => 4.min(most),
        _ => most,
    }
}

/// The top left of the first page's cell grid, far below any world. Small
/// enough that `f32` keeps sub-pixel positions there (its step at 65,536 is
/// 1/256 px), so a tweened part moves smoothly. Each larger class's grid
/// stands [`IMPOSTOR_CLASS_STEP`] higher, and each next page of a class
/// [`IMPOSTOR_PAGE_STEP`] to the right.
const IMPOSTOR_ORIGIN: Vec2 = Vec2::new(0.0, -65_536.0);

/// Between two classes' grids: more than the largest atlas (3584 texels), so
/// no camera sees another class's cells.
const IMPOSTOR_CLASS_STEP: f32 = 8192.0;

/// Between two pages of a class: more than a page and its un-premultiplying
/// quad ([`IMPOSTOR_QUAD_OFFSET`] plus 3584 texels), so no camera sees another
/// page's cells. The sixteenth page stands at x = 245,760, where the `f32`
/// step is 1/64 px.
const IMPOSTOR_PAGE_STEP: f32 = 16_384.0;

/// Where the un-premultiplying quad stands: beside the grid, out of its
/// camera's view.
const IMPOSTOR_QUAD_OFFSET: Vec2 = Vec2::new(8192.0, 0.0);

/// Camera order of the impostor cameras. Far below every view's, so the
/// impostors are drawn before any view samples them in the same frame.
const IMPOSTOR_CAMERA_ORDER: isize = -100_000;

/// On a presentation owner: the rigged root it draws.
#[derive(Component, Debug)]
#[relationship(relationship_target = RiggedPresentedBy)]
pub struct RiggedPresentationOf(pub Entity);

/// On a rigged root: its presentation owner. `linked_spawn`: the owner, and
/// with it its part slots, is despawned with the root.
#[derive(Component, Debug)]
#[relationship_target(relationship = RiggedPresentationOf, linked_spawn)]
pub struct RiggedPresentedBy(Entity);

impl RiggedPresentedBy {
    /// The presentation owner.
    pub fn owner(&self) -> Entity {
        self.0
    }
}

/// A presentation that goes, by any road, gives back its impostor cell.
pub fn give_back_the_cell_of_a_removed_presentation(
    removed: On<Remove, RiggedPresentation>,
    presentations: Query<&RiggedPresentation>,
    atlas: Option<ResMut<RiggedImpostorAtlas>>,
) {
    if let (Ok(presentation), Some(mut atlas)) = (presentations.get(removed.entity), atlas) {
        if let Some(impostor) = presentation.impostor.as_ref() {
            atlas.give(impostor);
        }
    }
}

/// The shared impostor atlases, the pages of each cell class
/// ([`IMPOSTOR_CELL_CLASSES`]): two targets each, their cameras, and which
/// cells are taken. A class has no page until the first rigged body of its
/// class binds.
#[derive(Resource, Default, Debug)]
pub struct RiggedImpostorAtlas(pub [Vec<ImpostorAtlas>; IMPOSTOR_CELL_CLASSES.len()]);

impl RiggedImpostorAtlas {
    /// The page `impostor` names.
    pub fn page(&self, impostor: &Impostor) -> Option<&ImpostorAtlas> {
        self.0[impostor.class].get(impostor.page)
    }

    fn give(&mut self, impostor: &Impostor) {
        if let Some(page) = self.0[impostor.class].get_mut(impostor.page) {
            page.give(impostor.cell);
        }
    }
}

/// One page of a class: one atlas.
#[derive(Debug)]
pub struct ImpostorAtlas {
    /// Its index in [`IMPOSTOR_CELL_CLASSES`].
    pub class: usize,
    /// Its index in its class's pages.
    pub page: usize,
    /// Cells per side.
    pub side: u32,
    /// What roots draw: straight alpha.
    pub image: Handle<Image>,
    /// What the parts are drawn into: premultiplied. Roots draw this where the
    /// app cannot un-premultiply (no renderer: nothing is drawn anyway).
    pub premultiplied: Handle<Image>,
    /// One frame per cell, cell `n` at index `n`.
    pub layout: Option<Handle<TextureAtlasLayout>>,
    pub cameras: Vec<Entity>,
    /// The un-premultiplying material, where the app renders.
    material: Option<Handle<ImpostorUnpremultiply>>,
    /// The cell opacities the material was last given.
    cells: ImpostorCellOpacity,
    /// Its cameras and its quad: what retiring the page despawns.
    entities: Vec<Entity>,
    taken: Vec<bool>,
    /// How many times this page has been rendered. A render clears the WHOLE
    /// page, so a cell holds its body's pixels only if that body was drawn
    /// into the latest render (`RiggedPresentation::shown`).
    pub generation: u64,
    pass_ready: readiness::PassReady,
}

impl ImpostorAtlas {
    fn take(&mut self) -> Option<u32> {
        let free = self.taken.iter().position(|taken| !taken)?;
        self.taken[free] = true;
        Some(free as u32)
    }

    /// Where a body in cell `cell` stands its feet (see [`impostor_cell_feet`]).
    pub fn cell_feet(&self, cell: u32, feet: Vec2) -> Vec2 {
        impostor_cell_feet(self.class, self.page, self.side, cell, feet)
    }

    /// Its cells' size in sheet pixels.
    pub fn cell_size(&self) -> f32 {
        IMPOSTOR_CELL_CLASSES[self.class].0
    }

    fn give(&mut self, cell: u32) {
        if let Some(taken) = self.taken.get_mut(cell as usize) {
            *taken = false;
        }
    }
}

/// Where a body in cell `cell` of page `page` of the class-`class` atlas,
/// `side` cells wide, stands its feet, so that its frame (with margins) fills
/// the cell from its top left: `feet` is the feet pixel in the cell, +y down.
pub fn impostor_cell_feet(class: usize, page: usize, side: u32, cell: u32, feet: Vec2) -> Vec2 {
    let side = side.max(1);
    let (column, row) = (cell % side, cell / side);
    let size = IMPOSTOR_CELL_CLASSES[class].0;
    let top_left = impostor_grid_origin(class, page) + Vec2::new(column as f32, -(row as f32)) * size;
    top_left + Vec2::new(feet.x, -feet.y)
}

/// The top left of the cell grid of page `page` of the class-`class` atlas.
fn impostor_grid_origin(class: usize, page: usize) -> Vec2 {
    IMPOSTOR_ORIGIN + Vec2::new(page as f32 * IMPOSTOR_PAGE_STEP, class as f32 * IMPOSTOR_CLASS_STEP)
}

/// Divides a premultiplied impostor's colour back out (see the module docs),
/// and fades each cell by its body's frame opacity.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct ImpostorUnpremultiply {
    #[texture(0)]
    pub premultiplied: Handle<Image>,
    #[uniform(1)]
    pub cells: ImpostorCellOpacity,
}

/// The most cells any atlas has.
const IMPOSTOR_MAX_CELLS: usize = (IMPOSTOR_MAX_CELLS_PER_SIDE * IMPOSTOR_MAX_CELLS_PER_SIDE) as usize;

/// Each cell's frame opacity, four to a vector (a uniform array's stride),
/// and its body's colour shift.
///
/// ⭐ A FRAME THAT FADES AS ONE PICTURE FADES HERE, after its parts are
/// composited (`RiggedSpriteAsset::frame_opacity`). Faded part by part, the
/// parts would show through each other where they overlap — the robot's death
/// fade drawn with the torso through its arm.
///
/// ⭐ A VARIANT'S COLOURS ARE TURNED HERE TOO (`CharacterColorShift`): per
/// body, after its parts are composited, at the cost of a few shader ops on a
/// pixel already being read. One sheet serves every coloured variant.
#[derive(bevy::render::render_resource::ShaderType, Debug, Clone, PartialEq)]
pub struct ImpostorCellOpacity {
    pub opacity: [Vec4; IMPOSTOR_MAX_CELLS / 4],
    /// Per cell: (hue in turns, saturation, value, 0)
    /// (`CharacterColorShift::as_uniform`).
    pub shift: [Vec4; IMPOSTOR_MAX_CELLS],
    /// Per cell: the teleport warp of its body's row ([`BodyWarp`]), as
    /// (kind, progress, left, right). Kind is 0 for none, 1 for a departure, 2
    /// for an arrival; progress is 0..1 through the row; left and right are
    /// the span of the body across the cell, in sheet pixels from the cell's
    /// left edge.
    ///
    /// ⭐ THE BODY IS TAKEN APART HERE, as one picture, after its parts are
    /// composited. Each reader of the body (its own quad, the hit flash, a
    /// portal piece, an overlay) then sees the same slivers. Taken apart part
    /// by part in the art, a teleport was a fade: the parts are few and they
    /// overlap.
    pub warp: [Vec4; IMPOSTOR_MAX_CELLS],
    /// Cells per side.
    pub side: u32,
    /// 1 when the parts were blended in gamma space (`ART_COMPOSITING`): the
    /// stored colour is encoded back to the values blended, divided there.
    pub gamma: u32,
    /// Sheet pixels on a side of one cell.
    pub cell_px: f32,
}

/// A developer preview of a [`BodyWarp`]: each part-drawn body is taken apart
/// by it on the row it draws, again and again, one time each `period_s`. It
/// shows the warp without a blink (`capture_scene --body-warp`). No game
/// inserts it.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct BodyWarpPreview {
    pub warp: BodyWarp,
    pub period_s: f32,
}

/// The half-widths of the span a teleport warp cuts into slivers, behind and
/// ahead of the feet, as fractions of the frame width. They are the player
/// robot's own (28 and 32 px of a 256 px frame), which the baked effect used.
const WARP_SPAN_BEHIND: f32 = 28.0 / 256.0;
const WARP_SPAN_AHEAD: f32 = 32.0 / 256.0;

impl ImpostorCellOpacity {
    fn opaque(side: u32, cell_px: f32) -> Self {
        Self {
            opacity: [Vec4::ONE; IMPOSTOR_MAX_CELLS / 4],
            shift: [CharacterColorShift::NONE.as_uniform(); IMPOSTOR_MAX_CELLS],
            warp: [Vec4::ZERO; IMPOSTOR_MAX_CELLS],
            side,
            gamma: u32::from(impostor_compositing() == ART_COMPOSITING),
            cell_px,
        }
    }

    /// Take the body in `cell` apart by `warp`, `progress` (0..1) of the way
    /// through its row. `feet_x` is the feet in the cell and `frame_width`
    /// the width of the body's frame, both in sheet pixels.
    fn set_warp(&mut self, cell: u32, warp: BodyWarp, progress: f32, feet_x: f32, frame_width: f32) {
        if let Some(slot) = self.warp.get_mut(cell as usize) {
            let kind = match warp {
                BodyWarp::TeleportOut => 1.0,
                BodyWarp::TeleportIn => 2.0,
            };
            *slot = Vec4::new(
                kind,
                progress.clamp(0.0, 1.0),
                feet_x - WARP_SPAN_BEHIND * frame_width,
                feet_x + WARP_SPAN_AHEAD * frame_width,
            );
        }
    }

    fn set_shift(&mut self, cell: u32, shift: &CharacterColorShift) {
        if let Some(slot) = self.shift.get_mut(cell as usize) {
            *slot = shift.as_uniform();
        }
    }

    fn set(&mut self, cell: u32, opacity: f32) {
        let cell = cell as usize;
        if cell < IMPOSTOR_MAX_CELLS {
            self.opacity[cell / 4][cell % 4] = opacity;
        }
    }
}

impl Material2d for ImpostorUnpremultiply {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/rendering/actors/rigged/impostor_unpremultiply.wgsl".into()
    }

    /// Replace, not blend: the quad IS the texture.
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}

/// Install the impostor's material, where this app renders.
pub fn add_rigged_impostor_material_plugin(app: &mut App) {
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_some()
    {
        bevy::asset::embedded_asset!(app, "rigged/impostor_unpremultiply.wgsl");
        // `ambition_render::frame_in_sprite`: where a body's frame lies on its
        // root's quad, for an overlay shader that patterns over the body. A
        // composition with no shader assets draws no overlay to import it.
        if app.world().contains_resource::<Assets<bevy::shader::Shader>>() {
            bevy::shader::load_shader_library!(app, "rigged/frame_in_sprite.wgsl");
        }
    }
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(bevy::sprite_render::Material2dPlugin::<ImpostorUnpremultiply>::default());
        readiness::install(app);
    }
}

/// A body's place in the impostor atlases.
#[derive(Debug, Clone, Copy)]
pub struct Impostor {
    /// The atlas: its index in [`IMPOSTOR_CELL_CLASSES`].
    pub class: usize,
    /// The page of that class's atlas.
    pub page: usize,
    pub cell: u32,
    /// The feet in the cell: sheet pixels from its top left, +y down.
    pub feet: Vec2,
}

/// What one rigged body's local sample is this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalSampleState {
    /// Nothing is sampled: the body is not drawn, or it was never sampled.
    #[default]
    Unavailable,
    /// The row is the baked sheet's (a baked clip of a hybrid): the body is
    /// its baked frame, and there are no parts to draw.
    Baked,
    /// `draws` holds the body's parts, in draw order.
    Parts,
}

/// One rigged body's local sample at this frame's presentation cut, made once
/// before any renderer reads it (RV1, PVS-2).
///
/// It is the whole local interpretation of the animation: the clip's frame,
/// its tween, legs, head turn, expression, layer, wounds, the blend from the
/// last row, the follow-through, a pose, and the track mask, in that order.
/// Each draw keeps its pivot place, scale, rotation, part (its texture region
/// in the flipbook) and full RGBA. Where the body is in the world is not
/// here: that is placement, which each renderer applies at its own stage.
/// A renderer reads this; it never interprets the animation itself.
#[derive(Component, Debug, Default)]
pub struct RiggedLocalSample {
    pub state: LocalSampleState,
    /// The drawn row's index in the animator's spec, and its frame.
    pub row: Option<usize>,
    pub frame: usize,
    /// The frame's own opacity (a fading frame is one picture fading).
    pub frame_opacity: f32,
    /// Tint and opacity applied to the body in the scene.
    pub body_color: LinearRgba,
    pub draws: Vec<PartDraw>,
    /// Where the frame's parts stand in the body's own frame: the feet of
    /// the frame's quad (its size, anchor, crouch squash and mirror row),
    /// one unit a sheet pixel, mirrored with the root. A renderer carries it
    /// by the body's scene placement (the root's transform); `None` with no
    /// parts.
    pub feet: Option<Transform>,
    /// The frame when it is baked (`LocalSampleState::Baked`): one picture
    /// a renderer draws whole. `None` otherwise, or while its page is not
    /// laid out.
    pub baked: Option<BakedFrame>,
    check_composite_extent: bool,
}

/// One complete frame from a baked sheet or the compositor. Draw its page
/// region at `size` world units with this sprite anchor and these flips.
/// Use the root transform to place it in the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct BakedFrame {
    pub image: Handle<Image>,
    pub rect: Rect,
    pub size: Vec2,
    pub anchor: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl RiggedLocalSample {
    /// The parts to draw, when the sample has parts.
    pub fn parts(&self) -> Option<&[PartDraw]> {
        (self.state == LocalSampleState::Parts).then_some(self.draws.as_slice())
    }

    /// The body color for loose parts when a composed frame is unavailable.
    pub(in crate::rendering) fn direct_body_color(&self) -> LinearRgba {
        let mut color = self.body_color;
        color.alpha *= self.frame_opacity;
        color
    }
}

/// The rigged presentation stage: demand, local sample, and sprite draws.
/// Order a system against the whole stage, not one of its systems.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RiggedDriveSet;

/// The stage's systems, in order, in [`RiggedDriveSet`].
pub fn rigged_presentation_drive() -> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::ScheduleSystem> {
    (prepare_rigged_demand, sample_rigged_presentations, drive_rigged_presentations)
        .chain()
        .in_set(RiggedDriveSet)
}

/// The presentation owner of one rigged root.
#[derive(Component)]
#[require(RiggedLocalSample)]
pub struct RiggedPresentation {
    /// The sheet target whose parts these are.
    pub target: String,
    pub pages: RiggedSpritePages,
    /// Reusable part sprites, children of the owner, in draw order.
    pub slots: Vec<Entity>,
    /// Whether a view or a reader needs this owner's current pose.
    sampling: bool,
    /// A world sprite pass needs the parts directly.
    direct: bool,
    /// Frames to keep allocated slots after their last use.
    slot_hold: u8,
    /// Its cell of the impostor atlas while the body is composited (something
    /// reads it as one image: [`ComposedBodyDemand`], a fading frame), else
    /// `None`: its parts are drawn directly in world space.
    pub impostor: Option<Impostor>,
    composed_frame: Option<BakedFrame>,
    /// Frames the body stays composited with no new demand
    /// ([`COMPOSED_HOLD_FRAMES`] after the last).
    pub composed_hold: u16,
    /// Frames the body stays OFF the composited road after the last frame its
    /// [`PartPose`] put a part past its cell (measured on its [`RiggedLocalSample`]): the same hold,
    /// so a pose at the edge of the cell does not move the body between the
    /// atlas and the world every frame.
    pub overflow_hold: u16,
    /// The render layers its slots carry now: the root's own while drawn
    /// directly, the impostor layer while composited.
    pub layers: RenderLayers,
    /// This frame's draws, tweened toward the next frame when the clip is
    /// (reused so a frame allocates nothing).
    pub drawn: Vec<PartDraw>,
    /// Its tracks bound to the joints of its body rig, when the sheet
    /// publishes one: what places its parts from a [`PartPose`].
    pub posed: Option<Arc<PosedParts>>,
    /// This frame's draws placed by a [`PartPose`] (reused).
    pub posed_draws: Vec<PartDraw>,
    shown: Option<ShownParts>,
    /// POSE-CONTINUITY's clip blend: the pose drawn before the last change of
    /// row, the seconds of the blend left, and the authored row (before
    /// facing) and draws of the last frame drawn. See [`CLIP_BLEND_SECONDS`].
    pub blend_from: Vec<PartDraw>,
    pub blend_left: f32,
    pub last_row: Option<usize>,
    pub last_draws: Vec<PartDraw>,
    /// POSE-CONTINUITY's follow-through: what the pose the body came into
    /// set swinging, and the seconds since. See [`FOLLOW_THROUGH_SECONDS`].
    pub swing: Option<(ambition_sprite_sheet::character::FollowThrough, f32)>,
}

struct ShownParts {
    draws: Vec<PartDraw>,
    target: PartDrawTarget,
}

#[derive(Clone, Copy, PartialEq)]
enum PartDrawTarget {
    World(LinearRgba),
    // A page render clears all cells. The generation identifies its result.
    Atlas { place: Vec3, generation: u64 },
}

impl RiggedPresentation {
    /// Get the frame when the compositor can draw the current atlas cell.
    #[cfg(feature = "card_view")]
    pub(in crate::rendering) fn composed_frame(&self) -> Option<&BakedFrame> {
        self.composed_frame.as_ref()
    }

    /// Get the position and generation of the last draw to the atlas.
    pub fn atlas_draw_stamp(&self) -> Option<(Vec3, u64)> {
        match self.shown.as_ref()?.target {
            PartDrawTarget::Atlas { place, generation } => Some((place, generation)),
            PartDrawTarget::World(_) => None,
        }
    }

    fn update_draws(&mut self, draws: &[PartDraw], target: PartDrawTarget) -> bool {
        match &mut self.shown {
            Some(shown) if shown.target == target && shown.draws == draws => false,
            Some(shown) => {
                shown.draws.clear();
                shown.draws.extend_from_slice(draws);
                shown.target = target;
                true
            }
            None => {
                self.shown = Some(ShownParts { draws: draws.to_vec(), target });
                true
            }
        }
    }
}

/// How long a change between two of the body's carriage poses
/// ([`CharacterAnim::blends_in`]) slides from the old pose to the new: a few
/// frames, so a fall into a ledge catch reads as one motion and a strike or a
/// landing, which do not blend, stay sharp. Measured on the reference fighter
/// (`scripts/measure/pose_transitions.py`): the pops were 14 to 44 px.
///
/// [`CharacterAnim::blends_in`]: ambition_sprite_sheet::character::CharacterAnim::blends_in
pub const CLIP_BLEND_SECONDS: f32 = 0.07;

/// How long a follow-through ([`CharacterAnim::follow_through`]) swings: one
/// arc out and back, `sin(pi t / T)`, from and to the pose as authored.
///
/// [`CharacterAnim::follow_through`]: ambition_sprite_sheet::character::CharacterAnim::follow_through
pub const FOLLOW_THROUGH_SECONDS: f32 = 0.18;

/// This frame of a follow-through `seconds` in, for a body drawn facing the
/// sheet's `forward` (+1 or -1 along x); `None` once it is over.
fn swing_at(follow: ambition_sprite_sheet::character::FollowThrough, seconds: f32, forward: f32) -> Option<Swing> {
    use ambition_sprite_sheet::character::SwingToward;
    let u = seconds / FOLLOW_THROUGH_SECONDS;
    if !(0.0..1.0).contains(&u) {
        return None;
    }
    let arc = (std::f32::consts::PI * u).sin();
    Some(Swing {
        hands_toward: match follow.toward {
            SwingToward::DownAndForward => Vec2::new(forward, 1.0).normalize(),
            SwingToward::Forward => Vec2::new(forward, 0.0),
        },
        arms: follow.arms_deg.to_radians() * arc,
        // Clockwise (+) turns the face of a body facing +x down: a nod.
        head: follow.head_deg.to_radians() * arc * forward,
    })
}

/// One reusable part sprite of a rigged presentation.
#[derive(Component)]
pub struct RiggedPartSlot;

/// One of the impostor atlas's cameras.
#[derive(Component)]
pub struct RiggedImpostorCamera;

/// The roots a presentation follows: their animator, sprite and anchor.
type Roots<'w, 's> = Query<
    'w,
    's,
    (
        &'static CharacterAnimator,
        &'static mut Sprite,
        Option<&'static mut Anchor>,
        Option<&'static CharacterColorShift>,
        &'static Transform,
        RootShows,
        Option<&'static RenderLayers>,
        Option<&'static FrameInSprite>,
        Option<&'static PerformedBodyWarp>,
        // A reader's ask for the draws of some tracks, and its answer.
        Option<(&'static PartSetRequest, &'static mut PartSetDraws)>,
        // The tracks the root draws, when not all of them (`limb_twin`).
        Option<&'static RiggedTrackMask>,
    ),
    (Without<RiggedPresentation>, Without<RiggedPartSlot>),
>;

/// The teleport warp a body is under this frame, and how far through it
/// (0 to 1). `row` is the row the body draws.
///
/// One rule for each reader ([`BodyWarp`] states it): a blink performed on
/// the body, then a blink row the body draws (its progress is the row's),
/// then a blink the body asks for with no row for it (its progress is the
/// time since it asked). A row that does not draw the body whole has no
/// warp: the sheet takes its own body apart there, or the body fades.
fn body_warp(
    animator: &CharacterAnimator,
    flipbook: &RiggedSpriteAsset,
    row: Option<&str>,
    performed: Option<&PerformedBodyWarp>,
) -> Option<(BodyWarp, f32)> {
    let row = row?;
    if !flipbook.row_draws_the_body_whole(row) {
        return None;
    }
    if let Some(performed) = performed {
        return Some((performed.warp, performed.progress()));
    }
    if let (Some(warp), Some(clip)) = (BodyWarp::of_row(row), flipbook.clip(row)) {
        let last = clip.frame_count().saturating_sub(1).max(1) as f32;
        let within = if clip.frame_duration_s > 0.0 {
            (animator.elapsed / clip.frame_duration_s).clamp(0.0, 1.0)
        } else {
            0.0
        };
        return Some((warp, (animator.frame as f32 + within) / last));
    }
    let (asked, asked_for) = animator.asked();
    Some((BodyWarp::of_anim(asked)?, (asked_for / BLINK_WARP_S).clamp(0.0, 1.0)))
}

/// Run the clock of each blink performed on a body, and take it off the body
/// at its end. Before [`drive_rigged_presentations`], which reads it.
pub fn advance_performed_body_warps(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mut bodies: Query<(Entity, &mut PerformedBodyWarp)>,
) {
    let dt = time.map_or(0.0, |time| time.delta_secs());
    for (body, mut performed) in &mut bodies {
        if performed.is_over() {
            commands.entity(body).try_remove::<PerformedBodyWarp>();
        } else {
            performed.elapsed_s += dt;
        }
    }
}

/// What says whether a root is shown this frame: its own visibility, and, for
/// a root under a parent, what propagation last made of it.
type RootShows = (
    Option<&'static Visibility>,
    Option<&'static InheritedVisibility>,
    Has<ChildOf>,
);

/// How far past a body's quad its parts can reach, and the camera can move
/// in the frame after its view was measured (world units). A view's area and
/// place are those of the last frame (`camera_system` and the transform
/// propagation run in `PostUpdate`).
const VIEW_MARGIN: f32 = 256.0;
const SLOT_HOLD_FRAMES: u8 = 8;

/// The cameras, for what each one shows. A part slot is not a camera; the
/// filter says so, because the slots' layers are written beside this read.
type Views<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Camera, &'static Projection, &'static GlobalTransform, Option<&'static RenderLayers>),
    (With<Camera2d>, Without<RiggedImpostorCamera>, Without<RiggedPartSlot>),
>;

/// What each active orthographic camera that is not an impostor camera
/// shows: its area in the world, and its layers. A perspective camera does
/// not draw the world's sprites. A camera whose target is not measured yet
/// (its first frame, a host with no window) has no area yet: it shows the
/// whole world.
fn world_views<'a>(
    views: &'a Views,
    gameplay: &[GameplayCoverage<'_>],
) -> Vec<(Rect, Option<&'a RenderLayers>)> {
    views
        .iter()
        .filter_map(|(entity, camera, projection, transform, layers)| {
            if !camera.is_active || gameplay.iter().any(|output| output.source == entity) {
                return None;
            }
            let Projection::Orthographic(ortho) = projection else {
                return None;
            };
            if camera.logical_viewport_size().is_none_or(|size| size.min_element() <= 0.0) {
                return Some((Rect::new(f32::MIN, f32::MIN, f32::MAX, f32::MAX), layers));
            }
            Some((PlayPlane::intersect(transform, ortho.area)?.bounds(Vec2::X), layers))
        })
        .collect()
}

/// Test the body's bounds against outputs that draw its scene layers.
fn seen_by_a_view(views: &[(Rect, Option<&RenderLayers>)], body: Rect, layers: Option<&RenderLayers>) -> bool {
    views.iter().any(|(shown, view_layers)| {
        view_layers.unwrap_or_default().intersects(layers.unwrap_or_default())
            && !shown.intersect(body).is_empty()
    })
}

/// Whether a root is shown. ⛔ A top-level root's own `Visibility` is this
/// frame's answer; its `InheritedVisibility` is last frame's (propagation runs
/// in `PostUpdate`), so parts that followed it appeared a frame after the body
/// and outlived its hiding by one. A parented root has only the propagated
/// answer.
pub(in crate::rendering) fn root_shows(
    (visibility, inherited, parented): (Option<&Visibility>, Option<&InheritedVisibility>, bool),
) -> bool {
    if visibility == Some(&Visibility::Hidden) {
        return false;
    }
    !parented || inherited.is_none_or(|inherited| inherited.get())
}

/// The part slots: what a frame writes on each.
type Slots<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Anchor,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut RenderLayers,
    ),
    (With<RiggedPartSlot>, Without<RiggedPresentation>),
>;

/// What the atlas is built with, the first time a body needs it. Absent pieces
/// leave it partial: no renderer, no un-premultiplying quad.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ImpostorAssets<'w> {
    images: Option<ResMut<'w, Assets<Image>>>,
    layouts: Option<ResMut<'w, Assets<TextureAtlasLayout>>>,
    meshes: Option<ResMut<'w, Assets<Mesh>>>,
    materials: Option<ResMut<'w, Assets<ImpostorUnpremultiply>>>,
    atlas: ResMut<'w, RiggedImpostorAtlas>,
    gpu: Option<Res<'w, readiness::GpuImpostors>>,
    gpu_images: Option<Res<'w, ambition_asset_manager::image_stages::AppGpuPreparedImages>>,
}

/// Bind, rebind and unbind each root's rigged presentation: a root whose sheet
/// carries a flipbook for the tier it is bound at gets an owner with slots; a
/// root that loses it, or goes away, loses its owner.
#[allow(clippy::too_many_arguments)]
pub fn bind_rigged_presentations(sprite_content: bevy::prelude::Res<ambition_sprite_sheet::SpriteContent>,
    mut commands: Commands,
    admission: Option<Res<RiggedSpriteAdmission>>,
    assets: Option<Res<GameAssets>>,
    asset_server: Option<Res<AssetServer>>,
    impostors: ImpostorAssets,
    mut sheets_by_tier: Local<HashMap<TextureResolutionScale, HashMap<String, RiggedSpritePages>>>,
    mut posed_by_target: Local<HashMap<String, BoundParts>>,
    mut roots: Query<(
        Entity,
        &CharacterAnimator,
        Option<&BoundSpriteQuality>,
        &mut Sprite,
        Option<&RiggedPresentedBy>,
    )>,
    presentations: Query<&RiggedPresentation>,
) {
    if !admission.is_some_and(|admission| admission.admit) {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    if impostors.images.is_none() {
        return;
    }
    if assets.is_changed() {
        for sheets in sheets_by_tier.values_mut() { sheets.clear(); }
        // The posed parts are NOT dropped here: a binding is kept with the
        // admitted rig and draw program it read, and is bound again only when
        // either differs (`BoundParts::binds`). Unrelated art loading keeps it.
        for sheet in assets.characters.ready_sheets() {
            if let Some(pages) = &sheet.rigged {
                // By the sheet's key: a generator's sheets share a `target`.
                sheets_by_tier.entry(sheet.resolved_tier).or_default().insert(sheet.spec.base_sheet_key().to_owned(), pages.clone());
            }
        }
    }
    // A root that went away took its owner with it (`RiggedPresentedBy`); a
    // root that lost its sheet loses its owner below.
    for (root, animator, bound, mut sprite, presented) in &mut roots {
        let owner = presented.map(RiggedPresentedBy::owner);
        let tier = bound.map_or(TextureResolutionScale::Full, |bound| bound.scale);
        let wanted = sheets_by_tier.get(&tier).and_then(|sheets| sheets.get(animator.spec.base_sheet_key()));
        let current = owner.and_then(|owner| presentations.get(owner).ok());
        let same = match (wanted, current) {
            (Some(wanted), Some(current)) => Arc::ptr_eq(&wanted.flipbook, &current.pages.flipbook),
            (None, None) => true,
            _ => false,
        };
        if same {
            continue;
        }
        let target = animator.spec.base_sheet_key();
        let ready = wanted.is_some_and(|wanted| {
            pages_ready(asset_server.as_deref(), impostors.images.as_deref().unwrap(), wanted)
        });
        // Not until every part page is ready: parts with no pixels would make
        // the body vanish.
        if wanted.is_some() && !ready {
            // ⛔ ONLY THE SAME CHARACTER KEEPS ITS OLD PARTS MEANWHILE (a tier
            // change). After a re-wear the root's animator is the new
            // character's, so the old parts would be driven with the new rows
            // (both have `idle`, `walk`). Drop them now: the root draws the new
            // character's baked sheet until its pages are ready.
            if current.is_some_and(|current| current.target != target) {
                drop_presentation(&mut commands, root, owner);
                draw_baked_frame(&mut sprite, animator);
            }
            continue;
        }
        drop_presentation(&mut commands, root, owner);
        match wanted {
            Some(pages) => {
                let rig = ambition_sprite_sheet::baked_body_rigs::admitted_body_rig(&sprite_content, target);
                let bound = posed_by_target.get(target).filter(|bound| bound.binds(rig.as_ref(), &pages.flipbook));
                let posed = match bound {
                    Some(bound) => bound.posed.clone(),
                    None => {
                        let posed = rig.as_deref().and_then(|rig| bind_posed_parts(rig, pages));
                        posed_by_target.insert(target.to_owned(), BoundParts { rig, flipbook: pages.flipbook.clone(), posed: posed.clone() });
                        posed
                    }
                };
                spawn_presentation(&mut commands, root, target, pages.clone(), posed);
            }
            // Back to the baked sheet: the root draws itself again.
            None => draw_baked_frame(&mut sprite, animator),
        }
    }
}

/// Despawn `root`'s presentation owner, if it has one. Its impostor cell is
/// given back by [`give_back_the_cell_of_a_removed_presentation`].
fn drop_presentation(commands: &mut Commands, root: Entity, owner: Option<Entity>) {
    if let Some(owner) = owner {
        commands.entity(root).try_remove::<FrameInSprite>();
        commands.entity(owner).try_despawn();
    }
}

/// The root's baked frame: the page its animator draws from and the frame's
/// atlas index.
/// `sprite` is the `Mut` itself, so a frame that is already drawn leaves the
/// sprite unchanged.
fn draw_baked_frame(sprite: &mut Mut<Sprite>, animator: &CharacterAnimator) {
    let page = animator
        .pages
        .get(animator.current_page() as usize)
        .or_else(|| animator.pages.first());
    if let Some(page) = page {
        if sprite.image != page.texture {
            sprite.image = page.texture.clone();
        }
        let texture_atlas = Some(TextureAtlas {
            layout: page.layout.clone(),
            index: animator.atlas_index(),
        });
        if sprite.texture_atlas != texture_atlas {
            sprite.texture_atlas = texture_atlas;
        }
    }
}

/// Every page of `pages` is ready to draw. Without an asset server (a
/// composition with no asset IO) a page is ready when its image is present.
fn pages_ready(asset_server: Option<&AssetServer>, images: &Assets<Image>, pages: &RiggedSpritePages) -> bool {
    pages.pages.iter().all(|page| match asset_server {
        Some(server) => super::texture_is_ready(server, images, page),
        None => images.contains(page),
    })
}

fn impostor_camera(target: &Handle<Image>, order: isize, at: Vec2, space: bevy::camera::CompositingSpace) -> impl Bundle {
    (
        RiggedImpostorCamera,
        space,
        Camera2d,
        Camera {
            order,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            is_active: false,
            ..default()
        },
        bevy::render::view::Msaa::Off,
        bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget::from(target.clone())),
        RenderLayers::layer(RIGGED_IMPOSTOR_LAYER),
        Transform::from_translation(at.extend(100.0)),
    )
}

/// Build page `page` of the class-`class` atlas `cells` per side: its two
/// targets, its layout, its cameras and its un-premultiplying quad.
fn build_atlas(commands: &mut Commands, assets: &mut ImpostorAssets, class: usize, page: usize, cells: u32) -> ImpostorAtlas {
    let cell = IMPOSTOR_CELL_CLASSES[class].0;
    let side = cells as f32 * cell;
    let texels = UVec2::splat(side as u32);
    let images = assets.images.as_deref_mut().expect("checked by the binder");
    let mut target = |format| {
        // The cameras supply the pixels. No blank CPU image needs an upload.
        let mut image = Image::new_uninit(
            Extent3d { width: texels.x, height: texels.y, depth_or_array_layers: 1 },
            TextureDimension::D2, format, default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
        images.add(image)
    };
    // The parts blend as the world blends them drawn directly
    // (`impostor_compositing`: linear light) into an sRGB target; the
    // un-premultiplying pass divides in the space they were blended in (with
    // `ART_COMPOSITING`, Bevy's output blit decodes the stored sRGB values and
    // the sRGB target stores them again exactly).
    let premultiplied = target(TextureFormat::Rgba8UnormSrgb);
    let straight = target(TextureFormat::Rgba8UnormSrgb);
    let layout = assets.layouts.as_deref_mut().map(|layouts| {
        layouts.add(TextureAtlasLayout::from_grid(UVec2::splat(cell as u32), cells, cells, None, None))
    });
    let centre = impostor_grid_origin(class, page) + Vec2::new(side * 0.5, -side * 0.5);
    let mut cameras = vec![commands
        .spawn((Name::new("rigged impostor camera"), impostor_camera(&premultiplied, IMPOSTOR_CAMERA_ORDER, centre, impostor_compositing())))
        .id()];
    let mut entities = cameras.clone();
    let mut material = None;
    let pass_ready = readiness::PassReady::new(cameras[0]);
    // Where the app renders, the second camera divides the colour out; where it
    // does not, nothing is drawn and roots may as well name the first target.
    let image = match (assets.meshes.as_deref_mut(), assets.materials.as_deref_mut()) {
        (Some(meshes), Some(materials)) => {
            let quad = centre + IMPOSTOR_QUAD_OFFSET;
            let handle = materials.add(ImpostorUnpremultiply {
                premultiplied: premultiplied.clone(),
                cells: ImpostorCellOpacity::opaque(cells, cell),
            });
            material = Some(handle.clone());
            entities.push(
                commands
                    .spawn((
                        Name::new("rigged impostor unpremultiply"),
                        pass_ready.clone(),
                        Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(side)))),
                        MeshMaterial2d(handle),
                        Transform::from_translation(quad.extend(0.0)),
                        Visibility::Inherited,
                        RenderLayers::layer(RIGGED_IMPOSTOR_LAYER),
                    ))
                    .id(),
            );
            cameras.push(
                commands
                    .spawn((
                        Name::new("rigged impostor unpremultiply camera"),
                        // One opaque quad, nothing to blend: linear light, so the
                        // material writes plain linear colour.
                        impostor_camera(&straight, IMPOSTOR_CAMERA_ORDER + 1, quad, bevy::camera::CompositingSpace::Linear),
                    ))
                    .id(),
            );
            straight
        }
        _ => premultiplied.clone(),
    };
    entities.extend(cameras.iter().skip(1).copied());
    ImpostorAtlas {
        class,
        page,
        side: cells,
        image,
        premultiplied,
        layout,
        cameras,
        material,
        cells: ImpostorCellOpacity::opaque(cells, cell),
        entities,
        taken: vec![false; (cells * cells) as usize],
        generation: 0,
        pass_ready,
    }
}

/// A cell of class `class` for a body, as `(page, cell, fresh)`: a free cell
/// of any page, else the first cell of a new page one growth step larger than
/// the last (`fresh`: its cameras are spawned with this frame's commands, so
/// it cannot render until the next frame).
///
/// ⛔ A PAGE NEVER REGROWS. Composition is on demand, so a cell is taken in
/// the middle of play; regrowing a page in place made a new blank target for
/// every body already drawn in it, and each showed nothing for a frame.
fn take_cell(commands: &mut Commands, assets: &mut ImpostorAssets, class: usize) -> (usize, u32, bool) {
    let pages = &mut assets.atlas.0[class];
    if let Some(taken) = pages.iter_mut().enumerate().find_map(|(page, atlas)| atlas.take().map(|cell| (page, cell))) {
        return (taken.0, taken.1, false);
    }
    let most = IMPOSTOR_CELL_CLASSES[class].1;
    let page = pages.len();
    let side = grown(pages.last().map_or(0, |last| last.side), most);
    let mut atlas = build_atlas(commands, assets, class, page, side);
    let cell = atlas.take().expect("a new page has a free cell");
    assets.atlas.0[class].push(atlas);
    (page, cell, true)
}

/// Bind the flipbook before a view or a reader asks for its part sprites.
fn spawn_presentation(
    commands: &mut Commands,
    root: Entity,
    target: &str,
    pages: RiggedSpritePages,
    posed: Option<Arc<PosedParts>>,
) -> Entity {
    let owner = commands
        .spawn((
            Name::new("rigged presentation"),
            Transform::default(),
            Visibility::Hidden,
            RiggedPresentationOf(root),
        ))
        .id();
    commands.entity(owner).insert(RiggedPresentation {
        target: target.to_owned(),
        pages,
        slots: Vec::new(),
        sampling: false,
        direct: false,
        slot_hold: 0,
        impostor: None,
        composed_frame: None,
        composed_hold: 0,
        overflow_hold: 0,
        layers: RenderLayers::default(),
        drawn: Vec::new(),
        posed,
        posed_draws: Vec::new(),
        shown: None,
        blend_from: Vec::new(),
        blend_left: 0.0,
        last_row: None,
        last_draws: Vec::new(),
        swing: None,
    });
    owner
}

/// Select the bodies to sample and the bodies that need sprite slots.
fn prepare_rigged_demand(
    mut commands: Commands,
    views: Views,
    outputs: GameplayOutputs,
    mut demand: Option<ResMut<ComposedBodyDemand>>,
    preview: Option<Res<BodyWarpPreview>>,
    roots: Query<(&Transform, &Sprite, Option<&RenderLayers>, Has<ChildOf>, Has<PartSetRequest>,
        &CharacterAnimator, Option<&PerformedBodyWarp>)>,
    mut owners: Query<(Entity, &RiggedPresentationOf, &mut RiggedPresentation)>,
) {
    let gameplay: Vec<_> = outputs.coverage().collect();
    let mut plane_views = world_views(&views, &gameplay);
    let mut card_views = Vec::new();
    for output in &gameplay {
        if let Some(plane) = &output.plane {
            plane_views.push((plane.bounds(Vec2::X), output.layers));
        }
        if let Some(cards) = &output.cards {
            card_views.push((cards.bounds(Vec2::X), output.layers));
        }
    }
    let no_views = plane_views.is_empty() && card_views.is_empty();
    let always_compose = PartPresentation::current() == PartPresentation::Impostor;
    for (owner, &RiggedPresentationOf(root), mut presentation) in &mut owners {
        let Ok((transform, sprite, layers, parented, part_reader, animator, performed)) = roots.get(root) else {
            continue;
        };
        let reach = sprite.custom_size.map_or(0.0, |size| size.max_element()) + VIEW_MARGIN;
        let body = Rect::from_center_half_size(transform.translation.truncate(), Vec2::splat(reach));
        let image_reader = demand.as_ref().is_some_and(|demand| demand.is_declared(root));
        presentation.direct = no_views
            || (parented && !plane_views.is_empty())
            || seen_by_a_view(&plane_views, body, layers);
        presentation.sampling = presentation.direct
            || part_reader
            || image_reader
            || parented
            || seen_by_a_view(&card_views, body, layers);

        if presentation.sampling {
            let flipbook = &presentation.pages.flipbook;
            let row = animator.drawn_row().and_then(|row| animator.spec.row_name(row));
            // Fade or warp all the parts as one image.
            let fades = row.is_some_and(|row| flipbook.frame_opacity(row, animator.frame) < 1.0);
            let warps = preview.is_some() || body_warp(animator, flipbook, row, performed).is_some();
            presentation.composed_hold = if always_compose || fades || warps || image_reader {
                COMPOSED_HOLD_FRAMES
            } else {
                presentation.composed_hold.saturating_sub(1)
            };
        } else {
            presentation.composed_hold = 0;
        }
        if presentation.direct || presentation.composed_hold > 0 {
            presentation.slot_hold = SLOT_HOLD_FRAMES;
            if presentation.slots.is_empty() {
                let count = presentation.pages.flipbook.max_draws();
                presentation.slots.extend((0..count).map(|_| {
                    commands.spawn((
                        RiggedPartSlot,
                        Sprite::default(),
                        Anchor::default(),
                        Transform::default(),
                        Visibility::Hidden,
                        RenderLayers::default(),
                        ChildOf(owner),
                    )).id()
                }));
                presentation.layers = RenderLayers::default();
                presentation.shown = None;
            }
        } else {
            presentation.slot_hold = presentation.slot_hold.saturating_sub(1);
            if presentation.slot_hold == 0 && !presentation.slots.is_empty() {
                for slot in presentation.slots.drain(..) {
                    commands.entity(slot).try_despawn();
                }
                presentation.shown = None;
            }
        }
    }
    if let Some(demand) = &mut demand {
        demand.clear();
    }
}

/// `target`'s tracks bound to the joints of the body rig its sheet publishes,
/// or `None` when it publishes none. A track the rig names rides that joint
/// (`part_bindings`); any other, the joint that holds it stillest. Within a
/// pixel and a degree: a part that rides its joint less tightly than that
/// draws where its frame puts it.
fn bind_posed_parts(rig: &ambition_sprite_sheet::baked_sheet_rons::AdmittedBodyRig, pages: &RiggedSpritePages) -> Option<Arc<PosedParts>> {
    let published = &rig.published;
    let posed = PosedParts::bind_named(&pages.flipbook, &rig.prepared, &published.part_bindings, 1.0, 1.0_f32.to_radians())
        .with_limbs(&rig.prepared, &published.limbs);
    (posed.bound().0 > 0).then(|| Arc::new(posed))
}

/// A target's binding with the admitted rig and flipbook it was bound from.
/// Holding both keeps their identities from being reused while it is kept.
pub struct BoundParts {
    rig: Option<Arc<ambition_sprite_sheet::baked_sheet_rons::AdmittedBodyRig>>,
    flipbook: Arc<ambition_sprite_sheet::character::rigged::RiggedSpriteAsset>,
    posed: Option<Arc<PosedParts>>,
}

impl BoundParts {
    /// Whether this binding is the one `rig` and `flipbook` would bind: the
    /// same admitted rig and the same draw program (any quality tier of it).
    fn binds(&self, rig: Option<&Arc<ambition_sprite_sheet::baked_sheet_rons::AdmittedBodyRig>>, flipbook: &ambition_sprite_sheet::character::rigged::RiggedSpriteAsset) -> bool {
        let same_rig = match (&self.rig, rig) {
            (Some(kept), Some(rig)) => Arc::ptr_eq(kept, rig),
            (None, None) => true,
            _ => false,
        };
        same_rig && self.flipbook.same_program(flipbook)
    }
}

/// Make each drawn rigged body's [`RiggedLocalSample`] (RV1, PVS-2): the
/// whole local interpretation of its animation, once, before any renderer
/// reads it. The blend and follow-through clocks advance here, beside the one
/// interpretation that reads them.
pub fn sample_rigged_presentations(
    mut owners: Query<(&RiggedPresentationOf, &mut RiggedPresentation, &mut RiggedLocalSample), Without<RiggedPartSlot>>,
    roots: Query<
        (
            &CharacterAnimator,
            Option<&PartPose>,
            Option<&RiggedTrackMask>,
            Option<&Sprite>,
            Option<&Anchor>,
        ),
        (Without<RiggedPresentation>, Without<RiggedPartSlot>),
    >,
    time: Option<Res<Time>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
) {
    for (&RiggedPresentationOf(root), mut presentation, mut sample) in &mut owners {
        let sample = &mut *sample;
        sample.state = LocalSampleState::Unavailable;
        sample.draws.clear();
        sample.row = None;
        sample.frame = 0;
        sample.frame_opacity = 1.0;
        sample.body_color = LinearRgba::WHITE;
        sample.feet = None;
        sample.baked = None;
        sample.check_composite_extent = false;
        let Ok((animator, pose, mask, root_sprite, root_anchor)) = roots.get(root) else {
            continue;
        };
        if !presentation.sampling {
            // Unseen, it draws nothing to blend from: the frame it is seen
            // again is drawn as it is.
            presentation.last_row = None;
            presentation.blend_left = 0.0;
            presentation.swing = None;
            continue;
        }
        sample.body_color = root_sprite.map_or(LinearRgba::WHITE, |sprite| sprite.color.to_linear());
        let flipbook = presentation.pages.flipbook.clone();
        // `None` for a baked clip of a hybrid: `check_rows` at attach makes
        // sure that every other row has draws. A tweened clip draws the frame
        // `frame_phase` of the way to the next (the flipbook's published rule).
        let mut drawn = std::mem::take(&mut presentation.drawn);
        let row = animator.drawn_row().and_then(|row| animator.spec.row_name(row));
        let mut posed_draws = std::mem::take(&mut presentation.posed_draws);
        // POSE-CONTINUITY: a change of row between two carriage poses slides
        // from the pose last drawn (blended or not, so an interrupted blend
        // goes on from where it is) over `CLIP_BLEND_SECONDS`. A turn to face
        // the other way is not a change of row here (the authored row, before
        // facing), and a strike or an impact does not blend.
        let authored_row = animator.authored_slot();
        let blends = animator.drawn_clip().is_none() && animator.current.blends_in();
        if !blends {
            presentation.blend_left = 0.0;
        } else if presentation.last_row.is_some_and(|last| last != authored_row) && !presentation.last_draws.is_empty() {
            let RiggedPresentation { blend_from, last_draws, .. } = &mut *presentation;
            blend_from.clone_from(last_draws);
            presentation.blend_left = CLIP_BLEND_SECONDS;
        }
        // The follow-through: set off by coming into a pose that swings (a
        // landing, a brake, a blow), carried on through the carriage poses
        // after it, and dropped at once by a strike or any clip.
        let row_changed = presentation.last_row.is_some_and(|last| last != authored_row);
        let carriage = animator.drawn_clip().is_none()
            && (animator.current.blends_in() || animator.current.follow_through().is_some());
        if !carriage {
            presentation.swing = None;
        } else if let Some(follow) = animator.current.follow_through().filter(|_| row_changed) {
            presentation.swing = Some((follow, 0.0));
        }
        // The sheet faces +x; a body drawing its mirror rows faces -x.
        let forward = if animator.drawn_row() == Some(authored_row) { 1.0 } else { -1.0 };
        let swing = presentation.swing.and_then(|(follow, seconds)| swing_at(follow, seconds, forward));
        let blend_t = 1.0 - presentation.blend_left / CLIP_BLEND_SECONDS;
        let blend_t = blend_t * blend_t * (3.0 - 2.0 * blend_t);
        let blend_from = std::mem::take(&mut presentation.blend_from);
        let draws = frame_draws(
            &flipbook,
            animator,
            pose,
            presentation.posed.as_deref(),
            (presentation.blend_left > 0.0).then_some((blend_from.as_slice(), blend_t)),
            swing,
            &mut drawn,
            &mut posed_draws,
        ).map(|(draws, check_extent)| {
            sample.check_composite_extent = check_extent;
            draws
        });
        presentation.blend_from = blend_from;
        let dt = time.as_ref().map_or(0.0, |time| time.delta_secs());
        presentation.blend_left = (presentation.blend_left - dt).max(0.0);
        presentation.swing = presentation
            .swing
            .map(|(follow, seconds)| (follow, seconds + dt))
            .filter(|(_, seconds)| *seconds < FOLLOW_THROUGH_SECONDS);
        presentation.last_row = Some(authored_row);
        presentation.last_draws.clear();
        if let Some(draws) = draws {
            presentation.last_draws.extend_from_slice(draws);
        }
        // A root that draws some of its tracks draws those only, on either
        // road and into its cell.
        sample.row = animator.drawn_row();
        sample.frame = animator.frame;
        sample.frame_opacity = row.map_or(1.0, |row| flipbook.frame_opacity(row, animator.frame));
        match (draws, mask) {
            (Some(draws), Some(mask)) => {
                sample.draws = mask.keep(&flipbook, draws);
                sample.state = LocalSampleState::Parts;
            }
            (Some(draws), None) => {
                sample.draws.extend_from_slice(draws);
                sample.state = LocalSampleState::Parts;
            }
            (None, _) => sample.state = LocalSampleState::Baked,
        }
        // The frame's quad as the animator drew it on the root, before the
        // driver replaces the root's image.
        if let (LocalSampleState::Parts, Some(basis), Some(root_sprite), Some(root_anchor)) =
            (sample.state, animator.render_basis, root_sprite, root_anchor)
        {
            let frame_size = flipbook.frame_size.as_vec2();
            let margin = impostor_margin(&flipbook);
            let cell = frame_size.max_element() + 2.0 * margin;
            let (size, anchor) = body_quad(animator, basis, root_sprite, root_anchor, frame_size, margin, cell);
            sample.feet = Some(feet_in_root(
                &flipbook,
                size,
                anchor,
                cell,
                margin,
                sprite_mirror(root_sprite),
            ));
        }
        if sample.state == LocalSampleState::Baked {
            sample.baked = baked_frame(animator, root_sprite, root_anchor, layouts.as_deref());
        }
        presentation.drawn = drawn;
        presentation.posed_draws = posed_draws;
    }
}

/// The baked frame the animator chose: its page's region for this frame, at
/// the quad the animator gave the root.
fn baked_frame(
    animator: &CharacterAnimator,
    root_sprite: Option<&Sprite>,
    root_anchor: Option<&Anchor>,
    layouts: Option<&Assets<TextureAtlasLayout>>,
) -> Option<BakedFrame> {
    let page = animator
        .pages
        .get(animator.current_page() as usize)
        .or_else(|| animator.pages.first())?;
    let rect = layouts?.get(&page.layout)?.textures.get(animator.atlas_index())?.as_rect();
    let sprite = root_sprite?;
    Some(BakedFrame {
        image: page.texture.clone(),
        rect,
        size: sprite.custom_size?,
        anchor: root_anchor.map_or(Vec2::ZERO, |anchor| anchor.0),
        flip_x: sprite.flip_x,
        flip_y: sprite.flip_y,
    })
}

/// Draw each rigged root's current frame from parts: the slots take the
/// frame's draws in the body's cell, the atlas cameras composite them, and the
/// root draws its cell at its own size and feet.
///
/// Runs after the animators, so it draws the frame they chose this frame.
#[allow(clippy::too_many_arguments)]
pub fn drive_rigged_presentations(
    mut commands: Commands,
    mut impostors: ImpostorAssets,
    mut owners: Query<
        (Entity, &RiggedPresentationOf, &mut RiggedPresentation, &mut Visibility, &mut Transform, &RiggedLocalSample),
        Without<RiggedPartSlot>,
    >,
    mut cameras: Query<&mut Camera, With<RiggedImpostorCamera>>,
    mut roots: Roots,
    mut slots: Slots,
    mut too_large: Local<HashSet<String>>,
    mut posed_past_cell: Local<HashSet<String>>,
    mut held_warp_tick: Local<u64>,
    preview: Option<Res<BodyWarpPreview>>,
    time: Option<Res<Time>>,
) {
    let tick = time.as_ref().map_or(0, |time| (time.elapsed_secs_f64() * 12.0).floor() as u64);
    let held_warp_changed = tick != *held_warp_tick;
    *held_warp_tick = tick;
    // The preview's warp and how far through it this frame is. The clock is
    // only the preview's: a row's own warp runs on the animator.
    let preview = preview.map(|preview| {
        let elapsed = time.as_ref().map_or(0.0, |time| time.elapsed_secs());
        (preview.warp, (elapsed / preview.period_s.max(0.05)).fract())
    });
    // Which bodies are composited this frame, and where their slots draw.
    let can_composite = impostors.images.is_some();
    // Pages built this frame: their cameras arrive with this frame's commands,
    // so a body given a cell in one draws directly once more.
    let mut fresh_pages: Vec<(usize, usize)> = Vec::new();
    for (_, &RiggedPresentationOf(root), mut presentation, _, _, sample) in &mut owners {
        if !presentation.sampling {
            if let Some(impostor) = presentation.impostor.take() {
                impostors.atlas.give(&impostor);
                presentation.shown = None;
            }
            presentation.overflow_hold = 0;
            continue;
        }
        let Ok((_, _, _, _, _, _, root_layers, _, _, _, _)) = roots.get(root) else {
            continue;
        };
        let flipbook = presentation.pages.flipbook.clone();
        let wanted = presentation.composed_hold > 0;
        let margin = impostor_margin(&flipbook);
        // A runtime pose can extend beyond the authored frame.
        let past_cell = sample.parts()
            .filter(|_| wanted && sample.check_composite_extent)
            .map(|draws| flipbook.reach_past_frame(draws))
            .filter(|reach| *reach + IMPOSTOR_FRINGE > margin);
        presentation.overflow_hold = match past_cell {
            Some(_) => COMPOSED_HOLD_FRAMES,
            None => presentation.overflow_hold.saturating_sub(1),
        };
        if let Some(reach) = past_cell.filter(|_| wanted && !posed_past_cell.contains(presentation.target.as_str()) && posed_past_cell.insert(presentation.target.clone())) {
            warn!(
                "rigged sprites: `{}` is read as one image but its pose puts a part {reach:.0} px past its                  frame, more than the {margin} px its cell gives; while it does, its parts draw directly and                  its readers see no image",
                presentation.target,
            );
        }
        let class = impostor_cell_class(flipbook.frame_size.as_vec2(), margin);
        if wanted && class.is_none() && !too_large.contains(presentation.target.as_str()) && too_large.insert(presentation.target.clone()) {
            warn!(
                "rigged sprites: `{}` is read as one image but cannot be composited — its {} px frame fits \
                 no impostor cell (the largest is {} px with {margin} px margins); its parts draw \
                 directly and its readers see no image",
                presentation.target,
                flipbook.frame_size,
                IMPOSTOR_CELL_CLASSES[IMPOSTOR_CELL_CLASSES.len() - 1].0,
            );
        }
        let composited = match class {
            Some(class) if can_composite && wanted && presentation.overflow_hold == 0 => {
                Some(class)
            }
            _ => None,
        };
        match (composited, presentation.impostor) {
            (Some(class), None) => {
                let (page, cell, fresh) = take_cell(&mut commands, &mut impostors, class);
                if fresh {
                    fresh_pages.push((class, page));
                }
                let feet = flipbook.feet_pixel + Vec2::splat(margin);
                presentation.impostor = Some(Impostor { class, page, cell, feet });
                presentation.shown = None;
            }
            (None, Some(impostor)) => {
                impostors.atlas.give(&impostor);
                presentation.impostor = None;
                presentation.shown = None;
            }
            _ => {}
        }
        // ⛔ A DIRECT BODY'S PARTS DRAW IN THE ROOT'S OWN LAYERS: a room that
        // is not live, a local view's layer. The impostor's root quad carried
        // them by being the root; loose parts must be told.
        let layers = match presentation.impostor {
            Some(impostor) if !fresh_pages.contains(&(impostor.class, impostor.page)) => {
                RenderLayers::layer(RIGGED_IMPOSTOR_LAYER)
            }
            _ => root_layers.cloned().unwrap_or_default(),
        };
        if presentation.layers != layers {
            for slot in &presentation.slots {
                if let Ok((_, _, _, _, mut slot_layers)) = slots.get_mut(*slot) {
                    *slot_layers = layers.clone();
                }
            }
            presentation.layers = layers;
        }
    }
    // ⛔ AN EMPTY PAGE AT THE END OF ITS CLASS IS RETIRED: its cameras and quad
    // despawned, its targets dropped with their handles. Composition is
    // transient; a portal room or a brawl would otherwise hold its pages (two
    // targets up to 1728 or 3584 texels a side each) for the rest of the
    // session. Only the last page goes, so every body keeps its page number.
    for pages in &mut impostors.atlas.0 {
        while pages.last().is_some_and(|page| !page.taken.contains(&true)) {
            let page = pages.pop().expect("checked");
            for entity in &page.entities {
                commands.entity(*entity).try_despawn();
            }
        }
    }
    let ImpostorAssets { atlas: mut atlases, layouts, materials, gpu, gpu_images, .. } = impostors;
    // Per page of each class: whether a body draws from it, whether a cell of
    // it changed, and its cells' opacities.
    //
    // ⛔ A page is rendered only on a frame where a cell of it CHANGES: its
    // target keeps the pixels between renders. Rendered every frame, a hall of
    // a few dozen bodies redrew every atlas — 4 targets up to 2304 x 2304, and
    // their un-premultiplied twins — at 110 ms a frame on a software
    // rasterizer against 9.6 ms baked (2026-10-03).
    let mut drawing: [Vec<bool>; IMPOSTOR_CELL_CLASSES.len()] =
        std::array::from_fn(|class| vec![false; atlases.0[class].len()]);
    let mut changed = drawing.clone();
    let mut pending_gpu = drawing.clone();
    let mut cells: [Vec<ImpostorCellOpacity>; IMPOSTOR_CELL_CLASSES.len()] = std::array::from_fn(|class| {
        atlases.0[class].iter().map(|atlas| ImpostorCellOpacity::opaque(atlas.side, atlas.cell_size())).collect()
    });
    // The bodies drawn into their page this frame: if the page renders, their
    // cells hold this frame's draws under its new generation.
    let mut drawn_into: Vec<(Entity, usize, usize)> = Vec::new();
    for (owner, &RiggedPresentationOf(root), mut presentation, mut owner_visibility, mut owner_transform, sample) in &mut owners {
        presentation.composed_frame = None;
        let Ok((
            animator,
            mut root_sprite,
            root_anchor,
            color_shift,
            root_transform,
            root_visibility,
            _,
            frame_in_sprite,
            performed,
            mut asked,
            _,
        )) = roots.get_mut(root)
        else {
            continue;
        };
        if !presentation.sampling {
            owner_visibility.set_if_neq(Visibility::Hidden);
            // The root draws nothing on the direct road, hidden or not: a
            // writer that gives it a baked frame (`animate_bosses`) does not
            // draw it under no parts.
            if root_sprite.image != ambition_sprite_sheet::character::NO_BAKED_IMAGE {
                root_sprite.image = ambition_sprite_sheet::character::NO_BAKED_IMAGE;
            }
            if frame_in_sprite.is_some() {
                commands.entity(root).try_remove::<FrameInSprite>();
            }
            continue;
        }
        let flipbook = presentation.pages.flipbook.clone();
        let row = animator.drawn_row().and_then(|row| animator.spec.row_name(row));
        // The body's local sample (`sample_rigged_presentations`): the
        // driver places and draws it, and interprets nothing itself.
        let draws = sample.parts();
        let (Some(draws), Some(basis), Some(mut root_anchor)) = (draws, animator.render_basis, root_anchor) else {
            // The baked frame draws the body, and is its whole image.
            draw_baked_frame(&mut root_sprite, animator);
            if let Some((_, answer)) = asked.as_mut() {
                part_set::clear(answer);
            }
            if frame_in_sprite.is_some() {
                commands.entity(root).try_remove::<FrameInSprite>();
            }
            owner_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let impostor = presentation
            .impostor
            .filter(|impostor| !fresh_pages.contains(&(impostor.class, impostor.page)));
        let feet_in_world = || sample.feet.map_or(*root_transform, |feet| root_transform.mul_transform(feet));
        let Some(impostor) = impostor else {
            // Drawn directly, the root shows no image at all.
            if frame_in_sprite.is_some() {
                commands.entity(root).try_remove::<FrameInSprite>();
            }
            let root_visible = root_shows(root_visibility);
            // The sample's feet (the body's own frame) carried by the root's
            // scene placement.
            let place = feet_in_world();
            if let Some((request, answer)) = asked.as_mut() {
                part_set::answer(request, answer, &presentation, draws, place);
            }
            drive_direct_presentation(
                &mut presentation,
                draws,
                sample.direct_body_color(),
                &mut root_sprite,
                place,
                root_visible,
                &mut owner_visibility,
                &mut owner_transform,
                &mut slots,
            );
            continue;
        };
        let (class, page) = (impostor.class, impostor.page);
        let Some(atlas) = atlases.page(&impostor) else {
            continue;
        };
        drawing[class][page] = true;
        pending_gpu[class][page] |= gpu.is_some() && (!atlas.pass_ready.is_ready()
            || gpu_images.as_deref().is_none_or(|ready| presentation.pages.pages.iter()
                .any(|image| !ready.is_prepared(image.id().untyped()))));
        // The body's warp and how far through it this frame is.
        match preview.or_else(|| body_warp(animator, &flipbook, row, performed)) {
            // The warp fades its slivers itself: the row's own fade, which a
            // sheet authored before the warp was the engine's, is not applied
            // a second time.
            Some((warp, progress)) => cells[class][page].set_warp(
                impostor.cell,
                warp,
                progress,
                impostor.feet.x,
                flipbook.frame_size.x as f32,
            ),
            None => {
                if row.is_some() {
                    cells[class][page].set(impostor.cell, sample.frame_opacity);
                }
            }
        }
        if let Some(shift) = color_shift {
            cells[class][page].set_shift(impostor.cell, shift);
        }
        // Its cell's place, at one unit a sheet pixel. ⛔ THE WHOLE TRANSFORM:
        // a body drawn directly the frame before left its owner scaled to world
        // units, and the parts shrank inside the cell (every hall body at a
        // third of its size, 2026-10-05).
        let place = atlas.cell_feet(impostor.cell, impostor.feet).extend(0.0);
        owner_transform.set_if_neq(Transform::from_translation(place));
        let slots_changed = presentation.update_draws(draws, PartDrawTarget::Atlas { place, generation: atlas.generation });
        if slots_changed {
            changed[class][page] = true;
        }
        // Stamped with the page's generation after the render decision.
        drawn_into.push((owner, class, page));
        // ⛔ NOT GATED ON THE ROOT'S VISIBILITY. A root hidden by the portal
        // resolver is still drawn — as pieces cut from its image, the impostor
        // — so the impostor must keep up with its frame while the root is
        // hidden. The owner is on the private layer: no view draws it either way.
        owner_visibility.set_if_neq(Visibility::Inherited);

        // The root's quad: its whole cell, placed so the frame inside it lands
        // exactly where the root's baked frame would (`body_quad`).
        let margin = impostor_margin(&flipbook);
        let (size, anchor) = body_quad(
            animator,
            basis,
            &root_sprite,
            &root_anchor,
            flipbook.frame_size.as_vec2(),
            margin,
            atlas.cell_size(),
        );
        if let Some((request, answer)) = asked.as_mut() {
            part_set::answer(request, answer, &presentation, draws, feet_in_world());
        }
        if root_sprite.image != atlas.image {
            root_sprite.image = atlas.image.clone();
        }
        // Written only when it differs: an unchanged root is not extracted again.
        let texture_atlas = atlas.layout.clone().map(|layout| TextureAtlas {
            layout,
            index: impostor.cell as usize,
        });
        presentation.composed_frame = texture_atlas.as_ref().and_then(|cell| {
            let rect = layouts.as_deref()?.get(&cell.layout)?.textures.get(cell.index)?.as_rect();
            Some(BakedFrame {
                image: atlas.image.clone(),
                rect,
                size,
                anchor,
                flip_x: root_sprite.flip_x,
                flip_y: root_sprite.flip_y,
            })
        });
        if root_sprite.texture_atlas != texture_atlas {
            root_sprite.texture_atlas = texture_atlas;
        }
        if root_sprite.rect.is_some() {
            root_sprite.rect = None;
        }
        if root_sprite.custom_size != Some(size) {
            root_sprite.custom_size = Some(size);
        }
        if root_anchor.0 != anchor {
            root_anchor.0 = anchor;
        }
        let frame = FrameInSprite {
            min: Vec2::splat(margin) / atlas.cell_size(),
            max: (Vec2::splat(margin) + flipbook.frame_size.as_vec2()) / atlas.cell_size(),
        };
        if frame_in_sprite != Some(&frame) {
            commands.entity(root).try_insert(frame);
        }

        // ⛔ A CELL THAT IS CURRENT KEEPS ITS SLOTS UNTOUCHED. Its slots hold
        // these draws already (`shown`: same draws, place and page render), and
        // a write marks every slot's `Transform` changed, so the hall
        // propagated and re-extracted thousands of part slots a frame whose
        // bodies had not changed frame.
        if slots_changed {
            write_slots(&presentation, draws, &mut slots, |draw| part_color(draw, LinearRgba::WHITE));
        }
    }
    let mut materials = materials.map(|materials| materials.into_inner());
    for (class, (pages, cells)) in atlases.0.iter_mut().zip(cells).enumerate() {
        for (page, (atlas, cells)) in pages.iter_mut().zip(cells).enumerate() {
            let drawing = drawing[class][page];
            let mut changed = changed[class][page];
            if atlas.cells != cells {
                changed = true;
                if let Some(mut material) = atlas
                    .material
                    .as_ref()
                    .and_then(|handle| materials.as_deref_mut().and_then(|materials| materials.get_mut(handle)))
                {
                    material.cells = cells.clone();
                }
                atlas.cells = cells;
            }
            // Cache only after the finishing pipeline and source images are ready.
            // CPU pose equality cannot certify a completed GPU image.
            let run = drawing && (changed || pending_gpu[class][page]);
            if run {
                atlas.generation += 1;
            }
            // A held warp changes only the finishing pass. Its source parts stay cached.
            let animate_held = drawing && held_warp_changed
                && atlas.cells.warp.iter().any(|warp| warp.x == 1.0 && warp.y >= 0.98);
            for (pass, entity) in atlas.cameras.iter().enumerate() {
                let active = run || (pass > 0 && animate_held);
                if let Ok(mut camera) = cameras.get_mut(*entity) {
                    if camera.is_active != active {
                        camera.is_active = active;
                    }
                }
            }
        }
    }
    for (owner, class, page) in drawn_into {
        let Some(generation) = atlases.0[class].get(page).map(|atlas| atlas.generation) else {
            continue;
        };
        if let Ok((_, _, mut presentation, _, _, _)) = owners.get_mut(owner) {
            if let Some(ShownParts { target: PartDrawTarget::Atlas { generation: stamped, .. }, .. }) = presentation.shown.as_mut() {
                // A page that did not render keeps its generation, and so does
                // every cell that was current; a page that rendered redrew
                // every body drawn into it this frame.
                *stamped = generation;
            }
        }
    }
}

/// This frame's draws of one body, in draw order: its clip's frame, tweened
/// toward the next when the clip is, and then placed by its pose when its
/// root carries one. `None` for a row its flipbook leaves to the baked sheet.
///
/// ⭐ POSE IS AN INPUT. A root carrying a `PartPose` (a ragdoll, a reach) has
/// each part that rides a joint placed from it; the frame still says which
/// parts draw, in what order and colour.
fn frame_draws<'a>(
    flipbook: &ambition_sprite_sheet::character::rigged::RiggedSpriteAsset,
    animator: &CharacterAnimator,
    pose: Option<&PartPose>,
    posed: Option<&PosedParts>,
    blend: Option<(&[PartDraw], f32)>,
    swing: Option<Swing>,
    drawn: &'a mut Vec<PartDraw>,
    posed_draws: &'a mut Vec<PartDraw>,
) -> Option<(&'a [PartDraw], bool)> {
    let row = animator.drawn_row().and_then(|row| animator.spec.row_name(row))?;
    flipbook.tween_into(row, animator.frame, animator.frame_phase(), drawn)?;
    let mut check_extent = flipbook.has_part_hierarchy();
    // The row is the picture of the upper body: its legs are those of the
    // pose the body is in under the move (it walks, it runs, it falls).
    if let Some((legs_row, frame, t)) = animator.legs_row() {
        if let Some(legs_row) = animator.spec.row_name(legs_row) {
            check_extent |= flipbook.legs_under(legs_row, frame, t, drawn, posed_draws);
        }
    }
    // A body that looks up while it walks: its head turns up on the walk.
    if let Some((look_row, fraction)) = animator.head_turn_row() {
        if let Some(look_row) = animator.spec.row_name(look_row) {
            check_extent |= flipbook.head_turned_as(look_row, fraction, drawn);
        }
    }
    // A hurt face over the pose it has. A body with no face to wince with
    // (no expression track) flinches whole, drawn in the frame of its hurt
    // row as its baked sheet drew it, unless parts posed from its row carry
    // its hits: then a part drawn elsewhere would not be where a strike lands.
    if let Some((face_row, index)) = animator.expression.as_ref() {
        if flipbook.has_expression_tracks() {
            check_extent |= flipbook.face_as(face_row, *index, drawn);
        } else if posed.is_none() && flipbook.tween_into(face_row, *index, 0.0, posed_draws).is_some() {
            std::mem::swap(drawn, posed_draws);
        }
    }
    // A layer over the row, on its own clock (a ward round a head).
    if let Some((layer_row, index, mirrored)) = animator.layer.as_ref() {
        check_extent |= flipbook.layer_over(layer_row, *index, *mirrored, drawn);
    }
    // Its wounds, as much as it is hurt (eyes that go bloodshot).
    flipbook.wounded_by(animator.wounds, drawn);
    // Sliding from the pose before a change of row, when it blends; the
    // limb ends the new pose plants (a ledge catch's hands) stay where it
    // draws them, their limbs solved to them.
    if let Some((from, t)) = blend {
        check_extent = true;
        flipbook.blend_from(from, t, drawn);
        if let Some(posed) = posed {
            posed.hold(row, animator.frame, animator.current.planted_contacts(), drawn);
        }
    }
    // The arms and head swinging on after a landing, a brake or a blow.
    if let (Some(swing), Some(posed)) = (swing, posed) {
        check_extent = true;
        posed.follow_through(row, animator.frame, swing, drawn);
    }
    Some(match (pose, posed) {
        (Some(pose), Some(posed)) => {
            posed.place(drawn, &pose.joints, posed_draws);
            (posed_draws.as_slice(), true)
        }
        _ => (drawn.as_slice(), check_extent),
    })
}

/// Write `draws` onto a presentation's slots, in draw order, each slot's
/// colour from `color_of`. Slots past the last draw are hidden.
fn write_slots(presentation: &RiggedPresentation, draws: &[PartDraw], slots: &mut Slots, color_of: impl Fn(&PartDraw) -> Color) {
    let flipbook = &presentation.pages.flipbook;
    for (index, slot) in presentation.slots.iter().enumerate() {
        let Ok((mut sprite, mut slot_anchor, mut transform, mut visibility, _)) = slots.get_mut(*slot) else {
            continue;
        };
        let Some(draw) = draws.get(index) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let part = flipbook.parts[usize::from(draw.part)];
        let page = &presentation.pages.pages[usize::from(part.page)];
        if sprite.image != *page {
            sprite.image = page.clone();
        }
        // Each field is written only when it differs. A slot that shows the
        // same part at the same place as last frame stays unchanged, so its
        // sprite is not extracted again and its transform is not propagated.
        let rect = Some(Rect::new(
            part.rect.min.x as f32,
            part.rect.min.y as f32,
            part.rect.max.x as f32,
            part.rect.max.y as f32,
        ));
        if sprite.rect != rect {
            sprite.rect = rect;
        }
        if sprite.custom_size != Some(part.size) {
            sprite.custom_size = Some(part.size);
        }
        let color = color_of(draw);
        if sprite.color != color {
            sprite.color = color;
        }
        if slot_anchor.0 != part.anchor() {
            slot_anchor.0 = part.anchor();
        }
        transform.set_if_neq(slot_local(draw, index));
        visibility.set_if_neq(Visibility::Inherited);
    }
}

/// Draw one body's frame directly in world space (`PartPresentation::Direct`).
///
/// A part lands where its pixel of the baked frame would: through the quad
/// the animator gave the root this frame (its size, anchor and facing flip,
/// squash and mirror rows included), so no anchor convention is re-derived
/// here. The owner stands at the feet, scaled from sheet pixels to world units
/// and mirrored when the root is flipped, then turned and scaled as the root
/// is (`place`); the root itself draws nothing, and
/// keeps that quad for every reader of the body's extent. The owner is not
/// the root's child (a player's root is its simulation body), so it follows
/// the root's visibility explicitly.
///
/// Exact group opacity requires a composed frame. When the frame is
/// unavailable, each part uses the body's opacity.
#[allow(clippy::too_many_arguments)]
fn drive_direct_presentation(
    presentation: &mut RiggedPresentation,
    draws: &[PartDraw],
    body_color: LinearRgba,
    // The `Mut` itself: it is only read here, but for the image, which is
    // compared first, so an unchanged root stays unchanged.
    root_sprite: &mut Mut<Sprite>,
    place: Transform,
    root_visible: bool,
    owner_visibility: &mut Mut<Visibility>,
    owner_transform: &mut Mut<Transform>,
    slots: &mut Slots,
) {
    if root_sprite.image != ambition_sprite_sheet::character::NO_BAKED_IMAGE {
        root_sprite.image = ambition_sprite_sheet::character::NO_BAKED_IMAGE;
    }
    if !presentation.direct {
        owner_visibility.set_if_neq(Visibility::Hidden);
        return;
    }
    owner_transform.set_if_neq(place);
    owner_visibility.set_if_neq(if root_visible { Visibility::Inherited } else { Visibility::Hidden });
    if presentation.update_draws(draws, PartDrawTarget::World(body_color)) {
        write_slots(presentation, draws, slots, |draw| part_color(draw, body_color));
    }
}

/// The sprite colour of one draw, multiplied by `root` (a tint or flash on
/// the whole body). The publisher's tint multiplies the part's stored sRGB
/// values, so it is an sRGB colour; the page is sampled decoded and the camera
/// encodes again (a camera in `ART_COMPOSITING`), or blends it in linear
/// light (the world's).
pub(in crate::rendering) fn part_color(draw: &PartDraw, root: LinearRgba) -> Color {
    let (tint, opacity) = (draw.tint(), draw.opacity());
    let part = Color::srgba(tint.x, tint.y, tint.z, opacity).to_linear();
    Color::LinearRgba(LinearRgba::new(
        part.red * root.red,
        part.green * root.green,
        part.blue * root.blue,
        part.alpha * root.alpha,
    ))
}

/// Place the parts in the root's frame. Flip the feet offset and the parts
/// together. The scene applies the root transform after this transform.
fn feet_in_root(
    flipbook: &RiggedSpriteAsset,
    size: Vec2,
    anchor: Vec2,
    cell: f32,
    margin: f32,
    mirror: Vec2,
) -> Transform {
    let feet_in_cell = (Vec2::splat(margin) + flipbook.feet_pixel) / cell;
    let feet = Vec2::new(feet_in_cell.x - 0.5, 0.5 - feet_in_cell.y);
    let offset = (feet * mirror - anchor) * size;
    Transform::from_translation(offset.extend(0.0))
        .with_scale((mirror * size / cell).extend(1.0))
}

/// The quad a body's frame is drawn on this frame, in a cell `cell` sheet
/// pixels square (`cell_quad`), with the root's crouch squash and mirror row
/// applied and mirrored with the root.
fn body_quad(
    animator: &CharacterAnimator,
    mut basis: ambition_sprite_sheet::character::RenderBasis,
    root_sprite: &Sprite,
    root_anchor: &Anchor,
    frame_size: Vec2,
    margin: f32,
    cell: f32,
) -> (Vec2, Vec2) {
    // The squash of a sheet with no compact row is read off the root as the
    // animator drew it, before it is replaced.
    let mirror = sprite_mirror(root_sprite);
    let squash = stance_squash(animator, root_sprite, root_anchor.0.y * mirror.y);
    // ⛔ A MIRROR ROW stands on the MIRRORED feet anchor. A body facing
    // left on a sheet drawn from both sides draws its mirror row unflipped
    // (`CharacterAnimator::face`), and the baked road places that frame at
    // the mirrored anchor (`current_render`). Without this the player
    // robot drew 3 to 5 px off its baked frame in every left-facing frame
    // (2026-10-04).
    if animator.draws_mirror_row() {
        basis.feet_anchor.x = -basis.feet_anchor.x;
    }
    let (mut size, mut anchor) = cell_quad(basis, frame_size, margin, cell);
    if let Some((ratio, held_y)) = squash {
        (size.y, anchor.y) = squashed_about(size.y, anchor.y, ratio, held_y);
    }
    (size, anchor * mirror)
}

/// The size and anchor of a body's cell quad (a cell `cell` sheet pixels
/// square): the cell drawn so that every pixel of the frame inside it (the
/// frame's top left at `margin`, [`impostor_margin`]) lands where the root's baked FULL
/// frame puts that pixel — the frame of `basis.render_size` at
/// `basis.feet_anchor`.
///
/// ⛔⛔ THE ROOT'S ANCHOR IS NOT ALWAYS ITS FEET. `basis.feet_anchor` is the
/// anchor the root was BUILT with: the feet for an NPC
/// (`feet_anchor_for_render_size`), but `Anchor::CENTER` for a player with a
/// sheet-authored quad (`character_render_basis`), whose translation is the
/// quad's centre. The first impostor put the frame's feet on the root's origin
/// whatever the anchor, and every centre-anchored body — Mary-O, the player —
/// drew half a body above its place (2026-10-02). Deriving the quad from the
/// basis, as the baked frame is derived, leaves no convention to assume;
/// `the_impostor_lands_where_the_baked_frame_would_for_either_anchor` holds
/// both.
pub fn cell_quad(
    basis: ambition_sprite_sheet::character::RenderBasis,
    frame_size: Vec2,
    margin: f32,
    cell: f32,
) -> (Vec2, Vec2) {
    let world_per_pixel = basis.render_size / frame_size;
    let size = Vec2::splat(cell) * world_per_pixel;
    // The frame's centre in the cell, normalized (+y up), and the frame's
    // anchor carried from frame units into cell units.
    let centre = (Vec2::splat(margin) + frame_size * 0.5) / cell;
    let anchor = Vec2::new(centre.x - 0.5, 0.5 - centre.y) + basis.feet_anchor * frame_size / cell;
    (size, anchor)
}

/// A quad of height `height` and anchor `anchor_y` squashed by `ratio` about
/// the local y `held_y` (from the anchor point, the root's units): the new
/// height and the anchor that keeps `held_y` still.
fn squashed_about(height: f32, anchor_y: f32, ratio: f32, held_y: f32) -> (f32, f32) {
    let bottom = -(anchor_y + 0.5) * height;
    let squashed_bottom = held_y + (bottom - held_y) * ratio;
    let squashed = height * ratio;
    (squashed, -squashed_bottom / squashed - 0.5)
}

/// The squash the root's quad is drawn with this frame, as `(ratio, held_y)`:
/// its height over the height the animator gives the frame, and the y (in the
/// root's local units) that holds still. `None` when the root is drawn at its
/// full height.
///
/// Read from the root itself, not from the stance, so the two pivots of
/// `StanceSquash` (the anchor, or the quad's foot edge) need no copy here. For
/// a quad of height `h0` and anchor `a0` drawn at `h1` and `a1`, the
/// normalized height `t` that holds still is `(a0 h0 - a1 h1) / (h0 - h1)`.
fn stance_squash(animator: &CharacterAnimator, root: &Sprite, unflipped_anchor_y: f32) -> Option<(f32, f32)> {
    let (unsquashed, unsquashed_anchor) = animator.current_render()?;
    let (h0, a0) = (unsquashed.y, unsquashed_anchor.y);
    let (h1, a1) = (root.custom_size?.y, unflipped_anchor_y);
    if h0 <= f32::EPSILON || (h0 - h1).abs() <= 1.0e-4 * h0 {
        return None;
    }
    let held = (a0 * h0 - a1 * h1) / (h0 - h1);
    Some((h1 / h0, (held - a0) * h0))
}

mod part_set;
pub use part_set::{ask_for_parts, forget_parts, PartSetDraws, PartSetRequest, PlacedPart};
mod limb_twin;
pub use limb_twin::{follow_limb_twins, limb_tracks_drawn, LimbTwin, LimbTwins, RiggedTrackMask, TrackLayer};

#[cfg(test)]
mod tests;
