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
//! ⛔ BOTH BLEND IN GAMMA SPACE, as the art was composited (PIL over stored
//! sRGB values). The world cameras blend in `WORLD_COMPOSITING`; so do the
//! atlas cameras, from the same sRGB part pages. Blended in linear light, every
//! anti-aliased outline over another part came out lighter (the robot's dark
//! outline drew 102 where the frame has 1).
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
//! simulation body, and the body must not grow presentation children.
//! [`RiggedPresentations`] maps a root to its owner.
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
//! A cell is the frame plus [`IMPOSTOR_MARGIN`] sheet pixels each side, so art
//! that runs past the baked frame (Mary-O's feet, up to 8 px) is drawn, not
//! cut. A body whose frame fits no cell draws directly even when read as one
//! image (and says so): its readers see no image.
//!
//! ⛔ Nothing here runs unless [`RiggedSpriteAdmission`] admits the flipbooks
//! (on by default since 2026-10-01). The crouch squash of a sheet without a
//! crouch row squashes the root's quad about the line it holds still, as it
//! squashes a baked quad (`stance_squash`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite::Anchor;
use bevy::sprite_render::{AlphaMode2d, Material2d, MeshMaterial2d};

use ambition_persistence::settings::TextureResolutionScale;
use ambition_platformer2d_shared_tangle::camera_layers::RIGGED_IMPOSTOR_LAYER;

use crate::rendering::WORLD_COMPOSITING;
use ambition_sprite_sheet::character::rigged::{
    ComposedBodyDemand, PartDraw, PartPose, PartPresentation, PosedParts, RiggedSpriteAdmission, RiggedSpritePages,
};
use ambition_sprite_sheet::character::{CharacterAnimator, CharacterColorShift};
use ambition_sprite_sheet::game_assets::GameAssets;

use super::BoundSpriteQuality;

/// Depth between two part slots of one body: the draw order of the flipbook.
const SLOT_DEPTH_STEP: f32 = 1.0e-4;

/// Frames a body stays composited after the last frame something read it as
/// one image: half a second at 60 Hz, so a demand that flickers (a blink, a
/// flash cue, a body edging along a portal) does not move it between the
/// atlas and the world every frame.
pub const COMPOSED_HOLD_FRAMES: u16 = 30;

/// Transparent sheet pixels on each side of the frame in a body's cell.
pub const IMPOSTOR_MARGIN: f32 = 16.0;

/// The cell sizes of the impostor atlases, in sheet pixels (one texel each),
/// smallest first, with the most cells per side each atlas grows to. A frame
/// with its margins: the player robot's 288 x 288, Mary-O's 192 x 224 and a
/// pirate's about 135 x 146 take the first; Noether's 528 x 560 the second;
/// the Perfect Cellular Automaton's 686 x 878 the third.
///
/// A page of the first grows to 6 x 6 = 36 bodies (two 1728-texel targets,
/// about 24 MB); a page of a larger one to 4 x 4 (2304 and 3584 texels a
/// side), for the few large bodies a room has. The first page is one cell,
/// made for the first body that needs it; each page after is one step larger
/// than the last (1, 2, 4, then its most), opened when every cell is taken.
///
/// ⛔ GROWN, NOT SIZED FOR THE MOST. Every frame the cameras clear and the
/// un-premultiplying quad shades the whole target: at 6 x 6 that cost a single
/// rigged body 13 ms on llvmpipe (measured 2026-10-02), against a quarter of
/// that for the one cell it needs.
pub const IMPOSTOR_CELL_CLASSES: [(f32, u32); 3] = [(288.0, 6), (576.0, 4), (896.0, 4)];

/// The smallest cell, the one most bodies take.
pub const IMPOSTOR_CELL: f32 = IMPOSTOR_CELL_CLASSES[0].0;

/// The most cells per side any atlas grows to.
pub const IMPOSTOR_MAX_CELLS_PER_SIDE: u32 = 6;

/// The class of the smallest cell a frame of `frame_size` fits with its
/// margins, or `None` when it fits none.
pub fn impostor_cell_class(frame_size: Vec2) -> Option<usize> {
    let needed = (frame_size + Vec2::splat(2.0 * IMPOSTOR_MARGIN)).max_element();
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

/// The owner entity of each rigged root.
#[derive(Resource, Default, Debug)]
pub struct RiggedPresentations(pub HashMap<Entity, Entity>);

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
    /// Cells per side.
    pub side: u32,
}

impl ImpostorCellOpacity {
    fn opaque(side: u32) -> Self {
        Self {
            opacity: [Vec4::ONE; IMPOSTOR_MAX_CELLS / 4],
            shift: [CharacterColorShift::NONE.as_uniform(); IMPOSTOR_MAX_CELLS],
            side,
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
    }
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(bevy::sprite_render::Material2dPlugin::<ImpostorUnpremultiply>::default());
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

/// The presentation owner of one rigged root.
#[derive(Component)]
pub struct RiggedPresentation {
    pub root: Entity,
    /// The sheet target whose parts these are.
    pub target: String,
    pub pages: RiggedSpritePages,
    /// Reusable part sprites, children of the owner, in draw order.
    pub slots: Vec<Entity>,
    /// Its cell of the impostor atlas while the body is composited (something
    /// reads it as one image: [`ComposedBodyDemand`], a fading frame), else
    /// `None`: its parts are drawn directly in world space.
    pub impostor: Option<Impostor>,
    /// Frames the body stays composited with no new demand
    /// ([`COMPOSED_HOLD_FRAMES`] after the last).
    pub composed_hold: u16,
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
    /// What this body's cell of the atlas holds now: the draws its page last
    /// rendered there, where, and that render's [`ImpostorAtlas::generation`].
    /// The cell holds them only while the page's generation is still that
    /// one: another body's change re-renders (and clears) the whole page, and
    /// a body not drawn into that render (on a baked clip of a hybrid) lost
    /// its pixels. `None` until first rendered.
    pub shown: Option<(Vec<PartDraw>, Vec3, u64)>,
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
        Option<&'static PartPose>,
    ),
    (Without<RiggedPresentation>, Without<RiggedPartSlot>),
>;

/// What says whether a root is shown this frame: its own visibility, and, for
/// a root under a parent, what propagation last made of it.
type RootShows = (
    Option<&'static Visibility>,
    Option<&'static InheritedVisibility>,
    Has<ChildOf>,
);

/// Whether a root is shown. ⛔ A top-level root's own `Visibility` is this
/// frame's answer; its `InheritedVisibility` is last frame's (propagation runs
/// in `PostUpdate`), so parts that followed it appeared a frame after the body
/// and outlived its hiding by one. A parented root has only the propagated
/// answer.
fn root_shows((visibility, inherited, parented): (Option<&Visibility>, Option<&InheritedVisibility>, bool)) -> bool {
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
}

/// Bind, rebind and unbind each root's rigged presentation: a root whose sheet
/// carries a flipbook for the tier it is bound at gets an owner with slots; a
/// root that loses it, or goes away, loses its owner.
#[allow(clippy::too_many_arguments)]
pub fn bind_rigged_presentations(
    mut commands: Commands,
    admission: Option<Res<RiggedSpriteAdmission>>,
    assets: Option<Res<GameAssets>>,
    asset_server: Option<Res<AssetServer>>,
    mut impostors: ImpostorAssets,
    mut owners: ResMut<RiggedPresentations>,
    mut by_sheet: Local<HashMap<(String, TextureResolutionScale), RiggedSpritePages>>,
    mut posed_by_target: Local<HashMap<String, Option<Arc<PosedParts>>>>,
    mut roots: Query<(Entity, &CharacterAnimator, Option<&BoundSpriteQuality>, &mut Sprite)>,
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
        by_sheet.clear();
        for sheet in assets.characters.ready_sheets() {
            if let Some(pages) = &sheet.rigged {
                // By the sheet's key: a generator's sheets share a `target`.
                by_sheet.insert((sheet.spec.base_sheet_key().to_owned(), sheet.resolved_tier), pages.clone());
            }
        }
    }
    // A root that went away, or lost its sheet, loses its owner.
    owners.0.retain(|root, owner| {
        if roots.contains(*root) {
            return true;
        }
        if let Some(impostor) = presentations.get(*owner).ok().and_then(|presentation| presentation.impostor.as_ref()) {
            impostors.atlas.give(impostor);
        }
        commands.entity(*owner).try_despawn();
        false
    });
    for (root, animator, bound, mut sprite) in &mut roots {
        let tier = bound.map_or(TextureResolutionScale::Full, |bound| bound.scale);
        let wanted = by_sheet.get(&(animator.spec.base_sheet_key().to_owned(), tier));
        let current = owners
            .0
            .get(&root)
            .and_then(|owner| presentations.get(*owner).ok());
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
                drop_presentation(&mut commands, &mut owners, &mut impostors.atlas, &presentations, root);
                draw_baked_frame(&mut sprite, animator);
            }
            continue;
        }
        drop_presentation(&mut commands, &mut owners, &mut impostors.atlas, &presentations, root);
        match wanted {
            Some(pages) => {
                let posed = posed_by_target
                    .entry(target.to_owned())
                    .or_insert_with(|| bind_posed_parts(pages, target))
                    .clone();
                owners.0.insert(root, spawn_presentation(&mut commands, root, target, pages.clone(), posed));
            }
            // Back to the baked sheet: the root draws itself again.
            None => draw_baked_frame(&mut sprite, animator),
        }
    }
}

fn drop_presentation(
    commands: &mut Commands,
    owners: &mut RiggedPresentations,
    atlas: &mut RiggedImpostorAtlas,
    presentations: &Query<&RiggedPresentation>,
    root: Entity,
) {
    if let Some(owner) = owners.0.remove(&root) {
        if let Some(impostor) = presentations.get(owner).ok().and_then(|presentation| presentation.impostor.as_ref()) {
            atlas.give(impostor);
        }
        commands.entity(owner).try_despawn();
    }
}

/// The root's baked frame: the page its animator draws from and the frame's
/// atlas index.
fn draw_baked_frame(sprite: &mut Sprite, animator: &CharacterAnimator) {
    let page = animator
        .pages
        .get(animator.current_page() as usize)
        .or_else(|| animator.pages.first());
    if let Some(page) = page {
        if sprite.image != page.texture {
            sprite.image = page.texture.clone();
        }
        sprite.texture_atlas = Some(TextureAtlas {
            layout: page.layout.clone(),
            index: animator.atlas_index(),
        });
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
    let mut target = |format| images.add(Image::new_target_texture(texels.x, texels.y, format, None));
    // The parts blend in GAMMA space, as the baked frame was composited and as
    // the world camera blends them when they are drawn directly
    // (`WORLD_COMPOSITING`). That camera's main texture holds the sRGB values
    // and Bevy's output blit decodes them, so an sRGB target stores them again
    // exactly; the un-premultiplying pass divides in that same space.
    let premultiplied = target(TextureFormat::Rgba8UnormSrgb);
    let straight = target(TextureFormat::Rgba8UnormSrgb);
    let layout = assets.layouts.as_deref_mut().map(|layouts| {
        layouts.add(TextureAtlasLayout::from_grid(UVec2::splat(cell as u32), cells, cells, None, None))
    });
    let centre = impostor_grid_origin(class, page) + Vec2::new(side * 0.5, -side * 0.5);
    let mut cameras = vec![commands
        .spawn((Name::new("rigged impostor camera"), impostor_camera(&premultiplied, IMPOSTOR_CAMERA_ORDER, centre, WORLD_COMPOSITING)))
        .id()];
    let mut entities = cameras.clone();
    let mut material = None;
    // Where the app renders, the second camera divides the colour out; where it
    // does not, nothing is drawn and roots may as well name the first target.
    let image = match (assets.meshes.as_deref_mut(), assets.materials.as_deref_mut()) {
        (Some(meshes), Some(materials)) => {
            let quad = centre + IMPOSTOR_QUAD_OFFSET;
            let handle = materials.add(ImpostorUnpremultiply {
                premultiplied: premultiplied.clone(),
                cells: ImpostorCellOpacity::opaque(cells),
            });
            material = Some(handle.clone());
            entities.push(
                commands
                    .spawn((
                        Name::new("rigged impostor unpremultiply"),
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
        cells: ImpostorCellOpacity::opaque(cells),
        entities,
        taken: vec![false; (cells * cells) as usize],
        generation: 0,
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

/// A presentation for `root`: an owner with one reusable sprite slot per draw
/// of its flipbook's busiest frame. It starts drawn directly; the driver
/// composites it into an impostor cell while something reads it as one image.
fn spawn_presentation(
    commands: &mut Commands,
    root: Entity,
    target: &str,
    pages: RiggedSpritePages,
    posed: Option<Arc<PosedParts>>,
) -> Entity {
    let owner = commands
        .spawn((Name::new("rigged presentation"), Transform::default(), Visibility::Hidden))
        .id();
    let slots = (0..pages.flipbook.max_draws())
        .map(|_| {
            commands
                .spawn((
                    RiggedPartSlot,
                    Sprite::default(),
                    Anchor::default(),
                    Transform::default(),
                    Visibility::Hidden,
                    RenderLayers::default(),
                    ChildOf(owner),
                ))
                .id()
        })
        .collect();
    commands.entity(owner).insert(RiggedPresentation {
        root,
        target: target.to_owned(),
        pages,
        slots,
        impostor: None,
        composed_hold: 0,
        layers: RenderLayers::default(),
        drawn: Vec::new(),
        posed,
        posed_draws: Vec::new(),
        shown: None,
    });
    owner
}

/// `target`'s tracks bound to the joints of the body rig its sheet publishes,
/// or `None` when it publishes none. Within a pixel and a degree: a part that
/// rides its joint less tightly than that draws where its frame puts it.
fn bind_posed_parts(pages: &RiggedSpritePages, target: &str) -> Option<Arc<PosedParts>> {
    let text = ambition_sprite_sheet::baked_body_rigs::baked_body_rig(target)?;
    let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(text).ok()?.prepare().ok()?;
    let posed = PosedParts::bind(&pages.flipbook, &rig, 1.0, 1.0_f32.to_radians());
    (posed.bound().0 > 0).then(|| Arc::new(posed))
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
    demand: Option<ResMut<ComposedBodyDemand>>,
    mut owners: Query<(Entity, &mut RiggedPresentation, &mut Visibility, &mut Transform), Without<RiggedPartSlot>>,
    mut cameras: Query<&mut Camera, With<RiggedImpostorCamera>>,
    mut roots: Roots,
    mut slots: Slots,
    mut too_large: Local<HashSet<String>>,
) {
    // Which bodies are composited this frame, and where their slots draw.
    let always = PartPresentation::current() == PartPresentation::Impostor;
    let can_composite = impostors.images.is_some();
    // Pages built this frame: their cameras arrive with this frame's commands,
    // so a body given a cell in one draws directly once more.
    let mut fresh_pages: Vec<(usize, usize)> = Vec::new();
    for (_, mut presentation, _, _) in &mut owners {
        let Ok((animator, _, _, _, _, _, root_layers, _)) = roots.get(presentation.root) else {
            continue;
        };
        let flipbook = presentation.pages.flipbook.clone();
        // ⛔ A FADING FRAME IS ONE PICTURE FADING: `opacity(composite(parts))`.
        // Spread over loose parts, each would show the one under it through
        // it (alice's blink drew her arm through her coat).
        let fades = animator
            .drawn_row()
            .and_then(|row| animator.spec.row_name(row))
            .is_some_and(|row| flipbook.frame_opacity(row, animator.frame) < 1.0);
        let wanted = always || fades || demand.as_ref().is_some_and(|demand| demand.is_declared(presentation.root));
        presentation.composed_hold = if wanted {
            COMPOSED_HOLD_FRAMES
        } else {
            presentation.composed_hold.saturating_sub(1)
        };
        let class = impostor_cell_class(flipbook.frame_size.as_vec2());
        if wanted && class.is_none() && too_large.insert(presentation.target.clone()) {
            warn!(
                "rigged sprites: `{}` is read as one image but cannot be composited — its {} px frame fits \
                 no impostor cell (the largest is {} px with {IMPOSTOR_MARGIN} px margins); its parts draw \
                 directly and its readers see no image",
                presentation.target,
                flipbook.frame_size,
                IMPOSTOR_CELL_CLASSES[IMPOSTOR_CELL_CLASSES.len() - 1].0,
            );
        }
        let composited = match class {
            Some(class) if can_composite && presentation.composed_hold > 0 => Some(class),
            _ => None,
        };
        match (composited, presentation.impostor) {
            (Some(class), None) => {
                let (page, cell, fresh) = take_cell(&mut commands, &mut impostors, class);
                if fresh {
                    fresh_pages.push((class, page));
                }
                let feet = flipbook.feet_pixel + Vec2::splat(IMPOSTOR_MARGIN);
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
    if let Some(mut demand) = demand {
        demand.clear();
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
    let ImpostorAssets { atlas: mut atlases, materials, .. } = impostors;
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
    let mut cells: [Vec<ImpostorCellOpacity>; IMPOSTOR_CELL_CLASSES.len()] = std::array::from_fn(|class| {
        atlases.0[class].iter().map(|atlas| ImpostorCellOpacity::opaque(atlas.side)).collect()
    });
    // The bodies drawn into their page this frame: if the page renders, their
    // cells hold this frame's draws under its new generation.
    let mut drawn_into: Vec<(Entity, usize, usize)> = Vec::new();
    for (owner, mut presentation, mut owner_visibility, mut owner_transform) in &mut owners {
        let Ok((animator, mut root_sprite, root_anchor, color_shift, root_transform, root_visibility, _, pose)) =
            roots.get_mut(presentation.root)
        else {
            continue;
        };
        let flipbook = presentation.pages.flipbook.clone();
        // `None` for a baked clip of a hybrid: `check_rows` at attach makes
        // sure that every other row has draws. A tweened clip draws the frame
        // `frame_phase` of the way to the next (the flipbook's published rule).
        let mut drawn = std::mem::take(&mut presentation.drawn);
        let row = animator.drawn_row().and_then(|row| animator.spec.row_name(row));
        let tweened = row.and_then(|row| flipbook.tween_into(row, animator.frame, animator.frame_phase(), &mut drawn));
        // ⭐ POSE IS AN INPUT. A root carrying a `PartPose` (a ragdoll, a
        // reach) has each part that rides a joint placed from it; the frame
        // still says which parts draw, in what order and colour.
        let mut posed_draws = std::mem::take(&mut presentation.posed_draws);
        let draws = match (tweened, pose, presentation.posed.as_deref()) {
            (Some(()), Some(pose), Some(posed)) => {
                posed.place(&drawn, &pose.joints, &mut posed_draws);
                Some(posed_draws.as_slice())
            }
            (Some(()), _, _) => Some(drawn.as_slice()),
            (None, _, _) => None,
        };
        let (Some(draws), Some(basis), Some(mut root_anchor)) = (draws, animator.render_basis, root_anchor) else {
            presentation.drawn = drawn;
            presentation.posed_draws = posed_draws;
            // The baked frame draws the body.
            draw_baked_frame(&mut root_sprite, animator);
            owner_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let impostor = presentation
            .impostor
            .filter(|impostor| !fresh_pages.contains(&(impostor.class, impostor.page)));
        let Some(impostor) = impostor else {
            let root_visible = root_shows(root_visibility);
            let frame_size = flipbook.frame_size.as_vec2();
            let cell = frame_size.max_element() + 2.0 * IMPOSTOR_MARGIN;
            let (size, anchor) = body_quad(animator, basis, &root_sprite, &root_anchor, frame_size, cell);
            // The frame's feet pixel in that cell, in the quad's normalized
            // space (+y up, mirrored with the image), carried to the world.
            let feet_in_cell = (Vec2::splat(IMPOSTOR_MARGIN) + flipbook.feet_pixel) / cell;
            let mut feet = Vec2::new(feet_in_cell.x - 0.5, 0.5 - feet_in_cell.y);
            if root_sprite.flip_x {
                feet.x = -feet.x;
            }
            let feet = root_transform.translation.truncate() + (feet - anchor) * size;
            let world_per_pixel = size / cell;
            let flip = if root_sprite.flip_x { -1.0 } else { 1.0 };
            let place = Transform::from_translation(feet.extend(root_transform.translation.z))
                .with_scale(Vec3::new(flip * world_per_pixel.x, world_per_pixel.y, 1.0));
            let frame_opacity = row.map_or(1.0, |row| flipbook.frame_opacity(row, animator.frame));
            drive_direct_presentation(
                &mut presentation,
                draws,
                frame_opacity,
                &mut root_sprite,
                place,
                root_visible,
                &mut owner_visibility,
                &mut owner_transform,
                &mut slots,
            );
            presentation.drawn = drawn;
            presentation.posed_draws = posed_draws;
            continue;
        };
        let (class, page) = (impostor.class, impostor.page);
        let Some(atlas) = atlases.page(&impostor) else {
            presentation.drawn = drawn;
            presentation.posed_draws = posed_draws;
            continue;
        };
        drawing[class][page] = true;
        if let Some(row) = row {
            cells[class][page].set(impostor.cell, flipbook.frame_opacity(row, animator.frame));
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
        let current = presentation
            .shown
            .as_ref()
            .is_some_and(|(shown, at, generation)| *generation == atlas.generation && *at == place && shown.as_slice() == draws);
        if !current {
            changed[class][page] = true;
        }
        match presentation.shown.as_mut() {
            Some((shown, at, _)) if !current => {
                shown.clear();
                shown.extend_from_slice(draws);
                *at = place;
            }
            Some(_) => {}
            None => presentation.shown = Some((draws.to_vec(), place, u64::MAX)),
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
        let (size, anchor) = body_quad(animator, basis, &root_sprite, &root_anchor, flipbook.frame_size.as_vec2(), atlas.cell_size());
        if root_sprite.image != atlas.image {
            root_sprite.image = atlas.image.clone();
        }
        root_sprite.texture_atlas = atlas.layout.clone().map(|layout| TextureAtlas {
            layout,
            index: impostor.cell as usize,
        });
        root_sprite.rect = None;
        root_sprite.custom_size = Some(size);
        root_anchor.0 = anchor;

        // ⛔ A CELL THAT IS CURRENT KEEPS ITS SLOTS UNTOUCHED. Its slots hold
        // these draws already (`shown`: same draws, place and page render), and
        // a write marks every slot's `Transform` changed, so the hall
        // propagated and re-extracted thousands of part slots a frame whose
        // bodies had not changed frame.
        if !current {
            write_slots(&presentation, draws, &mut slots, |draw| part_color(draw, LinearRgba::WHITE));
        }
        presentation.drawn = drawn;
        presentation.posed_draws = posed_draws;
    }
    let mut materials = materials.map(|materials| materials.into_inner());
    let pages = atlases.0.iter_mut().zip(cells).zip(drawing.iter().zip(&changed)).flat_map(
        |((pages, cells), (drawing, changed))| pages.iter_mut().zip(cells).zip(drawing.iter().zip(changed)),
    );
    for ((atlas, cells), (drawing, changed)) in pages {
        let mut changed = *changed;
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
        // A page's cameras run on a frame where a body of the page drawn from
        // parts changed its cell, and rest otherwise: the target keeps what
        // they last rendered.
        let run = *drawing && changed;
        if run {
            atlas.generation += 1;
        }
        for entity in &atlas.cameras {
            if let Ok(mut camera) = cameras.get_mut(*entity) {
                if camera.is_active != run {
                    camera.is_active = run;
                }
            }
        }
    }
    for (owner, class, page) in drawn_into {
        let Some(generation) = atlases.0[class].get(page).map(|atlas| atlas.generation) else {
            continue;
        };
        if let Ok((_, mut presentation, _, _)) = owners.get_mut(owner) {
            if let Some((_, _, stamped)) = presentation.shown.as_mut() {
                // A page that did not render keeps its generation, and so does
                // every cell that was current; a page that rendered redrew
                // every body drawn into it this frame.
                *stamped = generation;
            }
        }
    }
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
        // Under its owner, one unit is one sheet pixel, from the feet, +y up.
        // Clockwise in the sheet's +y-down frame is a negative angle.
        let local = Vec2::new(draw.at.x, -draw.at.y);
        let page = &presentation.pages.pages[usize::from(part.page)];
        if sprite.image != *page {
            sprite.image = page.clone();
        }
        sprite.rect = Some(Rect::new(
            part.rect.min.x as f32,
            part.rect.min.y as f32,
            part.rect.max.x as f32,
            part.rect.max.y as f32,
        ));
        sprite.custom_size = Some(part.size);
        let color = color_of(draw);
        if sprite.color != color {
            sprite.color = color;
        }
        slot_anchor.0 = part.anchor();
        // The scale in the transform, not the quad: a mirrored draw
        // (`scale.x < 0`) mirrors about its pivot, then turns.
        *transform = Transform::from_translation(local.extend(index as f32 * SLOT_DEPTH_STEP))
            .with_rotation(Quat::from_rotation_z(-draw.rotation))
            .with_scale(draw.scale.extend(1.0));
        visibility.set_if_neq(Visibility::Inherited);
    }
}

/// Draw one body's frame directly in world space (`PartPresentation::Direct`).
///
/// A part lands where its pixel of the baked frame would: through the quad
/// the animator gave the root this frame (its size, anchor and facing flip,
/// squash and mirror rows included), so no anchor convention is re-derived
/// here. The owner stands at the feet, scaled from sheet pixels to world units
/// and mirrored when the root is flipped; the root itself draws nothing, and
/// keeps that quad for every reader of the body's extent. The owner is not
/// the root's child (a player's root is its simulation body), so it follows
/// the root's visibility explicitly.
///
/// ⚠ A FADE FRAME IS APPROXIMATED. `frame_opacity` is the opacity of the
/// composited frame (`opacity(composite(parts))`); spread over the parts, a
/// part overlapping another shows the one under it through it. Exact group
/// opacity needs the frame composited first (the semantic part plan's optional
/// group composition); five characters fade a death or blink row this way.
#[allow(clippy::too_many_arguments)]
fn drive_direct_presentation(
    presentation: &mut RiggedPresentation,
    draws: &[PartDraw],
    frame_opacity: f32,
    root_sprite: &mut Sprite,
    place: Transform,
    root_visible: bool,
    owner_visibility: &mut Mut<Visibility>,
    owner_transform: &mut Mut<Transform>,
    slots: &mut Slots,
) {
    owner_transform.set_if_neq(place);
    owner_visibility.set_if_neq(if root_visible { Visibility::Inherited } else { Visibility::Hidden });
    if root_sprite.image != ambition_sprite_sheet::character::NO_BAKED_IMAGE {
        root_sprite.image = ambition_sprite_sheet::character::NO_BAKED_IMAGE;
    }
    // The root's colour (a tint, a flash) multiplies every part, as it
    // multiplied the composited body. Part pages are sampled as sRGB here, so
    // a draw's tint (a multiply on stored sRGB values) is an sRGB colour.
    let mut root_color = root_sprite.color.to_linear();
    root_color.alpha *= frame_opacity;
    let current = presentation
        .shown
        .as_ref()
        .is_some_and(|(shown, color, _)| shown.as_slice() == draws && *color == root_color.to_vec3());
    if current {
        return;
    }
    presentation.shown = Some((draws.to_vec(), root_color.to_vec3(), 0));
    write_slots(presentation, draws, slots, |draw| part_color(draw, root_color));
}

/// The sprite colour of one draw, multiplied by `root` (a tint or flash on
/// the whole body). The publisher's tint multiplies the part's stored sRGB
/// values, so it is an sRGB colour; the page is sampled decoded and the camera
/// encodes again (`WORLD_COMPOSITING`).
fn part_color(draw: &PartDraw, root: LinearRgba) -> Color {
    let (tint, opacity) = (draw.tint(), draw.opacity());
    let part = Color::srgba(tint.x, tint.y, tint.z, opacity).to_linear();
    Color::LinearRgba(LinearRgba::new(
        part.red * root.red,
        part.green * root.green,
        part.blue * root.blue,
        part.alpha * root.alpha,
    ))
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
    cell: f32,
) -> (Vec2, Vec2) {
    // The squash of a sheet with no compact row is read off the root as the
    // animator drew it, before it is replaced.
    let squash = stance_squash(animator, root_sprite, Some(root_anchor));
    // ⛔ A MIRROR ROW stands on the MIRRORED feet anchor. A body facing
    // left on a sheet drawn from both sides draws its mirror row unflipped
    // (`CharacterAnimator::face`), and the baked road places that frame at
    // the mirrored anchor (`current_render`). Without this the player
    // robot drew 3 to 5 px off its baked frame in every left-facing frame
    // (2026-10-04).
    if animator.draws_mirror_row() {
        basis.feet_anchor.x = -basis.feet_anchor.x;
    }
    let (mut size, mut anchor) = cell_quad(basis, frame_size, cell);
    if let Some((ratio, held_y)) = squash {
        (size.y, anchor.y) = squashed_about(size.y, anchor.y, ratio, held_y);
    }
    if root_sprite.flip_x {
        anchor.x = -anchor.x;
    }
    (size, anchor)
}

/// The size and anchor of a body's cell quad (a cell `cell` sheet pixels
/// square): the cell drawn so that every pixel of the frame inside it (the
/// frame's top left at `IMPOSTOR_MARGIN`) lands where the root's baked FULL
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
pub fn cell_quad(basis: ambition_sprite_sheet::character::RenderBasis, frame_size: Vec2, cell: f32) -> (Vec2, Vec2) {
    let world_per_pixel = basis.render_size / frame_size;
    let size = Vec2::splat(cell) * world_per_pixel;
    // The frame's centre in the cell, normalized (+y up), and the frame's
    // anchor carried from frame units into cell units.
    let centre = (Vec2::splat(IMPOSTOR_MARGIN) + frame_size * 0.5) / cell;
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
fn stance_squash(animator: &CharacterAnimator, root: &Sprite, anchor: Option<&Anchor>) -> Option<(f32, f32)> {
    let (unsquashed, unsquashed_anchor) = animator.current_render()?;
    let (h0, a0) = (unsquashed.y, unsquashed_anchor.y);
    let (h1, a1) = (root.custom_size?.y, anchor?.0.y);
    if h0 <= f32::EPSILON || (h0 - h1).abs() <= 1.0e-4 * h0 {
        return None;
    }
    let held = (a0 * h0 - a1 * h1) / (h0 - h1);
    Some((h1 / h0, (held - a0) * h0))
}

#[cfg(test)]
mod tests;
