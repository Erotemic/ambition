//! The rigged-sprite realization: a character drawn from its sheet's transform
//! flipbook with a fixed set of reusable part sprites, composited into one
//! texture (its IMPOSTOR) that the body's own quad draws.
//!
//! The baked sheet path stays in charge of WHAT is drawn. The root's
//! [`CharacterAnimator`] still picks the row, the frame and the facing from the
//! same simulation facts, and this realization only draws that frame from
//! parts. So the rigged path cannot pick another clip or another timing.
//!
//! ⭐ ONE QUAD PER BODY, DRAWN FROM PARTS (decision D4 of
//! `docs/planning/engine/mary-o-part-realization.md`). Every reader of a body
//! — the portal compositor, the hit flash, a content overlay such as Mary-O's
//! star power — reads the root's `Sprite`: its image and frame. So the parts
//! are not drawn into the world. A private camera draws them into the body's
//! impostor, and the ROOT draws that, at its own size and feet, with its own
//! tint and flip. Every reader then sees the frame the parts drew — a tweened
//! in-between included — and none of them needs the baked sheet. The parts are
//! never portal candidates; the root is the one.
//!
//! ⭐ ONE ATLAS FOR EVERY BODY ([`RiggedImpostorAtlas`]). Each body is a cell
//! of one shared target, and one camera draws every cell in one pass. A camera
//! per body cost about 1.4 ms per actor on llvmpipe (measured 2026-10-02), and
//! the shipped game rigs every pirate. The root names its cell as an atlas
//! frame (the atlas layout's index), so every reader that understands an atlas
//! frame understands the impostor.
//!
//! The private camera draws [`RIGGED_IMPOSTOR_LAYER`], and the cells stand far
//! below any world ([`impostor_cell_feet`]), so no view sees loose parts.
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
//! cut. A body whose frame does not fit a cell, or that finds the atlas full,
//! keeps its baked sheet (and says so once).
//!
//! ⛔ Nothing here runs unless [`RiggedSpriteAdmission`] admits the flipbooks
//! (on by default since 2026-10-01). The crouch squash of a sheet without a
//! crouch row squashes the root's quad about the line it holds still, as it
//! squashes a baked quad (`stance_squash`).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite::Anchor;
use bevy::sprite_render::{AlphaMode2d, Material2d, MeshMaterial2d};

use ambition_persistence::settings::TextureResolutionScale;
use ambition_platformer2d_shared_tangle::camera_layers::RIGGED_IMPOSTOR_LAYER;
use ambition_sprite_sheet::character::rigged::{PartDraw, RiggedSpriteAdmission, RiggedSpritePages};
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::game_assets::GameAssets;

use super::BoundSpriteQuality;

/// Depth between two part slots of one body: the draw order of the flipbook.
const SLOT_DEPTH_STEP: f32 = 1.0e-4;

/// Transparent sheet pixels on each side of the frame in a body's cell.
pub const IMPOSTOR_MARGIN: f32 = 16.0;

/// One cell of the impostor atlas, in sheet pixels (one texel each). Mary-O's
/// frame with its margins is 192 x 224; a pirate's is about 135 x 146.
pub const IMPOSTOR_CELL: f32 = 256.0;

/// The most cells per side the atlas grows to: 6 x 6 = 36 bodies, two
/// 1536-texel targets (about 19 MB). It starts at one cell, made for the first
/// body that needs it, and grows a step when a body finds it full (1, 2, 4,
/// 6 per side).
///
/// ⛔ GROWN, NOT SIZED FOR THE MOST. Every frame the cameras clear and the
/// un-premultiplying quad shades the whole target: at 6 x 6 that cost a single
/// rigged body 13 ms on llvmpipe (measured 2026-10-02), against a quarter of
/// that for the one cell it needs.
pub const IMPOSTOR_MAX_CELLS_PER_SIDE: u32 = 6;

/// The next atlas size after `side` cells per side, or `None` at the most.
fn grown(side: u32) -> Option<u32> {
    match side {
        0 => Some(1),
        1 => Some(2),
        2 => Some(4),
        side if side < IMPOSTOR_MAX_CELLS_PER_SIDE => Some(IMPOSTOR_MAX_CELLS_PER_SIDE),
        _ => None,
    }
}

/// The top left of the atlas's cell grid, far below any world. Small enough
/// that `f32` keeps sub-pixel positions there (its step at 65,536 is 1/256 px),
/// so a tweened part moves smoothly.
const IMPOSTOR_ORIGIN: Vec2 = Vec2::new(0.0, -65_536.0);

/// Where the un-premultiplying quad stands: beside the grid, out of its
/// camera's view.
const IMPOSTOR_QUAD_OFFSET: Vec2 = Vec2::new(8192.0, 0.0);

/// Camera order of the impostor cameras. Far below every view's, so the
/// impostors are drawn before any view samples them in the same frame.
const IMPOSTOR_CAMERA_ORDER: isize = -100_000;

/// The owner entity of each rigged root.
#[derive(Resource, Default, Debug)]
pub struct RiggedPresentations(pub HashMap<Entity, Entity>);

/// The shared impostor atlas: two targets, their cameras, and which cells are
/// taken. `None` until the first rigged body binds.
#[derive(Resource, Default, Debug)]
pub struct RiggedImpostorAtlas(pub Option<ImpostorAtlas>);

#[derive(Debug)]
pub struct ImpostorAtlas {
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
    /// The cameras, the quad: what a regrowth replaces.
    entities: Vec<Entity>,
    taken: Vec<bool>,
    warned: bool,
}

impl ImpostorAtlas {
    fn take(&mut self) -> Option<u32> {
        let free = self.taken.iter().position(|taken| !taken)?;
        self.taken[free] = true;
        Some(free as u32)
    }

    /// Where a body in cell `cell` stands its feet (see [`impostor_cell_feet`]).
    pub fn cell_feet(&self, cell: u32, feet: Vec2) -> Vec2 {
        impostor_cell_feet(self.side, cell, feet)
    }

    fn give(&mut self, cell: u32) {
        if let Some(taken) = self.taken.get_mut(cell as usize) {
            *taken = false;
        }
    }
}

/// Where a body in cell `cell` of an atlas `side` cells wide stands its feet,
/// so that its frame (with margins) fills the cell from its top left: `feet`
/// is the feet pixel in the cell, +y down.
pub fn impostor_cell_feet(side: u32, cell: u32, feet: Vec2) -> Vec2 {
    let side = side.max(1);
    let (column, row) = (cell % side, cell / side);
    let top_left = IMPOSTOR_ORIGIN + Vec2::new(column as f32, -(row as f32)) * IMPOSTOR_CELL;
    top_left + Vec2::new(feet.x, -feet.y)
}

/// Divides a premultiplied impostor's colour back out (see the module docs).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct ImpostorUnpremultiply {
    #[texture(0)]
    pub premultiplied: Handle<Image>,
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

/// A body's place in the impostor atlas.
#[derive(Debug, Clone, Copy)]
pub struct Impostor {
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
    pub impostor: Impostor,
    /// This frame's draws, tweened toward the next frame when the clip is
    /// (reused so a frame allocates nothing).
    pub drawn: Vec<PartDraw>,
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
    (&'static CharacterAnimator, &'static mut Sprite, Option<&'static mut Anchor>),
    (Without<RiggedPresentation>, Without<RiggedPartSlot>),
>;

/// The part slots: what a frame writes on each.
type Slots<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Anchor,
        &'static mut Transform,
        &'static mut Visibility,
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
                by_sheet.insert((sheet.spec.target().to_owned(), sheet.resolved_tier), pages.clone());
            }
        }
    }
    // A root that went away, or lost its sheet, loses its owner.
    owners.0.retain(|root, owner| {
        if roots.contains(*root) {
            return true;
        }
        if let (Ok(presentation), Some(atlas)) = (presentations.get(*owner), impostors.atlas.0.as_mut()) {
            atlas.give(presentation.impostor.cell);
        }
        commands.entity(*owner).try_despawn();
        false
    });
    for (root, animator, bound, mut sprite) in &mut roots {
        let tier = bound.map_or(TextureResolutionScale::Full, |bound| bound.scale);
        let wanted = by_sheet.get(&(animator.spec.target().to_owned(), tier));
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
        let target = animator.spec.target();
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
        let owner = wanted.and_then(|pages| spawn_presentation(&mut commands, &mut impostors, root, target, pages.clone()));
        match owner {
            Some(owner) => {
                owners.0.insert(root, owner);
            }
            // Back to the baked sheet (or no room for parts): the root draws
            // itself again.
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
        if let (Ok(presentation), Some(atlas)) = (presentations.get(owner), atlas.0.as_mut()) {
            atlas.give(presentation.impostor.cell);
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

fn impostor_camera(target: &Handle<Image>, order: isize, at: Vec2) -> impl Bundle {
    (
        RiggedImpostorCamera,
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

/// Build the shared atlas `cells` per side: its two targets, its layout, its
/// cameras and its un-premultiplying quad.
fn build_atlas(commands: &mut Commands, assets: &mut ImpostorAssets, cells: u32) -> ImpostorAtlas {
    let side = cells as f32 * IMPOSTOR_CELL;
    let texels = UVec2::splat(side as u32);
    let images = assets.images.as_deref_mut().expect("checked by the binder");
    let mut target = || images.add(Image::new_target_texture(texels.x, texels.y, TextureFormat::Rgba8UnormSrgb, None));
    let premultiplied = target();
    let straight = target();
    let layout = assets.layouts.as_deref_mut().map(|layouts| {
        layouts.add(TextureAtlasLayout::from_grid(UVec2::splat(IMPOSTOR_CELL as u32), cells, cells, None, None))
    });
    let centre = IMPOSTOR_ORIGIN + Vec2::new(side * 0.5, -side * 0.5);
    let mut cameras = vec![commands
        .spawn((Name::new("rigged impostor camera"), impostor_camera(&premultiplied, IMPOSTOR_CAMERA_ORDER, centre)))
        .id()];
    let mut entities = cameras.clone();
    // Where the app renders, the second camera divides the colour out; where it
    // does not, nothing is drawn and roots may as well name the first target.
    let image = match (assets.meshes.as_deref_mut(), assets.materials.as_deref_mut()) {
        (Some(meshes), Some(materials)) => {
            let quad = centre + IMPOSTOR_QUAD_OFFSET;
            let quad_entity = commands.spawn((
                Name::new("rigged impostor unpremultiply"),
                Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(side)))),
                MeshMaterial2d(materials.add(ImpostorUnpremultiply {
                    premultiplied: premultiplied.clone(),
                })),
                Transform::from_translation(quad.extend(0.0)),
                Visibility::Inherited,
                RenderLayers::layer(RIGGED_IMPOSTOR_LAYER),
            )).id();
            entities.push(quad_entity);
            cameras.push(
                commands
                    .spawn((
                        Name::new("rigged impostor unpremultiply camera"),
                        impostor_camera(&straight, IMPOSTOR_CAMERA_ORDER + 1, quad),
                    ))
                    .id(),
            );
            straight
        }
        _ => premultiplied.clone(),
    };
    entities.extend(cameras.iter().skip(1).copied());
    ImpostorAtlas {
        side: cells,
        image,
        premultiplied,
        layout,
        cameras,
        entities,
        taken: vec![false; (cells * cells) as usize],
        warned: false,
    }
}

/// A cell for a new body, growing the atlas a step when it is full. A body
/// already drawn keeps its cell number; the next frame stands its parts where
/// that number now is (`drive_rigged_presentations` places them every frame).
fn take_cell(commands: &mut Commands, assets: &mut ImpostorAssets) -> Option<u32> {
    if let Some(cell) = assets.atlas.0.as_mut().and_then(ImpostorAtlas::take) {
        return Some(cell);
    }
    let side = assets.atlas.0.as_ref().map_or(0, |atlas| atlas.side);
    let next = grown(side)?;
    let mut atlas = build_atlas(commands, assets, next);
    if let Some(old) = assets.atlas.0.take() {
        for entity in old.entities {
            commands.entity(entity).try_despawn();
        }
        atlas.taken[..old.taken.len()].copy_from_slice(&old.taken);
        atlas.warned = old.warned;
    }
    let cell = atlas.take();
    assets.atlas.0 = Some(atlas);
    cell
}

/// A presentation for `root`, or `None` when its frame does not fit a cell or
/// the atlas is full: the root then keeps its baked sheet.
fn spawn_presentation(
    commands: &mut Commands,
    assets: &mut ImpostorAssets,
    root: Entity,
    target: &str,
    pages: RiggedSpritePages,
) -> Option<Entity> {
    let flipbook = pages.flipbook.clone();
    let needed = flipbook.frame_size.as_vec2() + Vec2::splat(2.0 * IMPOSTOR_MARGIN);
    let cell = if needed.max_element() <= IMPOSTOR_CELL { take_cell(commands, assets) } else { None };
    let Some(cell) = cell else {
        let warned = assets.atlas.0.as_mut().map(|atlas| std::mem::replace(&mut atlas.warned, true));
        if warned != Some(true) {
            warn!(
                "rigged sprites: `{target}` keeps its baked sheet — its {needed} px frame does not fit a \
                 {IMPOSTOR_CELL} px impostor cell, or all {} cells are taken",
                IMPOSTOR_MAX_CELLS_PER_SIDE * IMPOSTOR_MAX_CELLS_PER_SIDE
            );
        }
        return None;
    };
    let side = assets.atlas.0.as_ref().map_or(1, |atlas| atlas.side);
    let feet = flipbook.feet_pixel + Vec2::splat(IMPOSTOR_MARGIN);
    let owner = commands
        .spawn((
            Name::new("rigged presentation"),
            Transform::from_translation(impostor_cell_feet(side, cell, feet).extend(0.0)),
            Visibility::Hidden,
        ))
        .id();
    let slots = (0..flipbook.max_draws())
        .map(|_| {
            commands
                .spawn((
                    RiggedPartSlot,
                    Sprite::default(),
                    Anchor::default(),
                    Transform::default(),
                    Visibility::Hidden,
                    RenderLayers::layer(RIGGED_IMPOSTOR_LAYER),
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
        impostor: Impostor { cell, feet },
        drawn: Vec::new(),
    });
    Some(owner)
}

/// Draw each rigged root's current frame from parts: the slots take the
/// frame's draws in the body's cell, the atlas cameras composite them, and the
/// root draws its cell at its own size and feet.
///
/// Runs after the animators, so it draws the frame they chose this frame.
pub fn drive_rigged_presentations(
    atlas: Option<Res<RiggedImpostorAtlas>>,
    mut owners: Query<(&mut RiggedPresentation, &mut Visibility, &mut Transform), Without<RiggedPartSlot>>,
    mut cameras: Query<&mut Camera, With<RiggedImpostorCamera>>,
    mut roots: Roots,
    mut slots: Slots,
) {
    let Some(atlas) = atlas.as_deref().and_then(|atlas| atlas.0.as_ref()) else {
        return;
    };
    let mut drawing = false;
    for (mut presentation, mut owner_visibility, mut owner_transform) in &mut owners {
        let Ok((animator, mut root_sprite, root_anchor)) = roots.get_mut(presentation.root) else {
            continue;
        };
        let flipbook = presentation.pages.flipbook.clone();
        // `None` for a baked clip of a hybrid: `check_rows` at attach makes
        // sure that every other row has draws. A tweened clip draws the frame
        // `frame_phase` of the way to the next (the flipbook's published rule).
        let mut drawn = std::mem::take(&mut presentation.drawn);
        let tweened = animator
            .drawn_row()
            .and_then(|row| animator.spec.row_name(row))
            .and_then(|row| flipbook.tween_into(row, animator.frame, animator.frame_phase(), &mut drawn));
        let draws = tweened.map(|()| drawn.as_slice());
        let (Some(draws), Some(basis), Some(mut root_anchor)) = (draws, animator.render_basis, root_anchor) else {
            presentation.drawn = drawn;
            // The baked frame draws the body.
            draw_baked_frame(&mut root_sprite, animator);
            owner_visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        drawing = true;
        // Its cell's place, which a regrowth of the atlas moves.
        let place = atlas.cell_feet(presentation.impostor.cell, presentation.impostor.feet).extend(0.0);
        if owner_transform.translation != place {
            owner_transform.translation = place;
        }
        // ⛔ NOT GATED ON THE ROOT'S VISIBILITY. A root hidden by the portal
        // resolver is still drawn — as pieces cut from its image, the impostor
        // — so the impostor must keep up with its frame while the root is
        // hidden. The owner is on the private layer: no view draws it either way.
        owner_visibility.set_if_neq(Visibility::Inherited);

        // The root's quad: its whole cell, its feet on the root's feet, at the
        // size the baked frame would have per sheet pixel. The squash of a
        // sheet with no compact row is read off the root as the animator drew
        // it, before it is replaced.
        let squash = stance_squash(animator, &root_sprite, Some(&root_anchor));
        let impostor = presentation.impostor;
        let world_per_pixel = basis.render_size / flipbook.frame_size.as_vec2();
        let mut size = Vec2::splat(IMPOSTOR_CELL) * world_per_pixel;
        let mut anchor = Vec2::new(impostor.feet.x / IMPOSTOR_CELL - 0.5, 0.5 - impostor.feet.y / IMPOSTOR_CELL);
        if let Some((ratio, held_y)) = squash {
            (size.y, anchor.y) = squashed_about(size.y, anchor.y, ratio, held_y);
        }
        if root_sprite.flip_x {
            anchor.x = -anchor.x;
        }
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

        for (index, slot) in presentation.slots.iter().enumerate() {
            let Ok((mut sprite, mut slot_anchor, mut transform, mut visibility)) = slots.get_mut(*slot) else {
                continue;
            };
            let Some(draw) = draws.get(index) else {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            };
            let part = flipbook.parts[usize::from(draw.part)];
            // In the cell, one world unit is one sheet pixel, from the feet,
            // +y up. Clockwise in the sheet's +y-down frame is a negative angle.
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
            sprite.custom_size = Some(part.size * draw.scale);
            slot_anchor.0 = part.anchor();
            *transform = Transform::from_translation(local.extend(index as f32 * SLOT_DEPTH_STEP))
                .with_rotation(Quat::from_rotation_z(-draw.rotation));
            visibility.set_if_neq(Visibility::Inherited);
        }
        presentation.drawn = drawn;
    }
    // The atlas cameras run while any body draws from parts, and rest when
    // none does.
    for entity in &atlas.cameras {
        if let Ok(mut camera) = cameras.get_mut(*entity) {
            if camera.is_active != drawing {
                camera.is_active = drawing;
            }
        }
    }
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
