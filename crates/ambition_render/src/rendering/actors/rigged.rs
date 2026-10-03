//! The rigged-sprite realization: a character drawn from its sheet's transform
//! flipbook with a fixed set of reusable part sprites.
//!
//! The baked sheet path stays in charge of WHAT is drawn. The root's
//! [`CharacterAnimator`] still picks the row, the frame and the facing from the
//! same simulation facts, and this realization only draws that frame from
//! parts. So the rigged path cannot pick another clip or another timing.
//!
//! Each rigged root gets one presentation OWNER: a top-level entity that
//! follows the root and whose children are the part slots. The owner is not a
//! child of the root, because a player's root is its simulation body, and the
//! body must not grow presentation children. [`RiggedPresentations`] maps a
//! root to its owner.
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
//! The root keeps its baked sprite with zero alpha. That keeps the baked sheet
//! as the parity oracle of the flipbook, and keeps the root the body's ONE portal
//! candidate, of the body's size: the parts are never candidates.
//!
//! Through a portal the body is drawn from its baked frame. The compositor
//! redraws a candidate as clipped pieces of ONE quad, which a set of parts is
//! not, and the baked frame is the same picture. So:
//!
//! * the root states its visible tint as its `PortalPieceTint`, and the pieces
//!   are drawn with it rather than with the root's zero alpha;
//! * the owner is `PresentationOf(root)` with no sprite of its own, so the
//!   portal's visibility resolver hides it in the same pass that hides the root
//!   and gives it back after. The parts and the pieces never draw together.
//!
//! A hybrid flipbook leaves some rows to the baked sheet. For such a row,
//! and for a frame with no row or no render basis, the root draws its baked
//! frame with its tint and the slots hide. The root, its feet and its animator
//! are the same in both cases, so a body moves between a baked clip and a part
//! clip with no jump in place or in timing.
//!
//! ⛔ Nothing here runs unless [`RiggedSpriteAdmission`] admits the flipbooks
//! (on by default since 2026-10-01).
//! The crouch squash of a sheet without a crouch row reaches the parts through
//! the owner (`stance_squash`). The hit flash needs nothing: its material
//! samples the root's texture and frame with its own tint and never reads the
//! sprite color, so a rigged body flashes with its baked silhouette.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_persistence::settings::TextureResolutionScale;
use ambition_sprite_sheet::character::rigged::{PartDraw, RiggedSpriteAdmission, RiggedSpritePages};
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::game_assets::GameAssets;

use super::BoundSpriteQuality;

/// Depth between two part slots of one body: the draw order of the flipbook,
/// small enough that a body's parts never interleave with another body.
const SLOT_DEPTH_STEP: f32 = 1.0e-4;

/// The owner entity of each rigged root.
#[derive(Resource, Default, Debug)]
pub struct RiggedPresentations(pub HashMap<Entity, Entity>);

/// The presentation owner of one rigged root.
#[derive(Component)]
pub struct RiggedPresentation {
    pub root: Entity,
    /// The sheet target whose parts these are.
    pub target: String,
    pub pages: RiggedSpritePages,
    /// Reusable part sprites, children of the owner, in draw order.
    pub slots: Vec<Entity>,
    /// The root's last visible tint. The root is drawn with zero alpha, so the
    /// tint is kept here for the parts.
    pub tint: Color,
    /// The tint last stated on the root as its portal piece tint.
    pub stated_tint: Option<Color>,
    /// This frame's draws, tweened toward the next frame when the clip is
    /// (reused so a frame allocates nothing).
    pub drawn: Vec<PartDraw>,
}

/// One reusable part sprite of a rigged presentation.
#[derive(Component)]
pub struct RiggedPartSlot;

/// The roots a presentation follows: their animator, sprite and placement.
type Roots<'w, 's> = Query<
    'w,
    's,
    (
        &'static CharacterAnimator,
        &'static mut Sprite,
        &'static Transform,
        Option<&'static Visibility>,
        Option<&'static RenderLayers>,
        Option<&'static Anchor>,
    ),
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
        Option<&'static RenderLayers>,
    ),
    (With<RiggedPartSlot>, Without<RiggedPresentation>),
>;

/// Bind, rebind and unbind each root's rigged presentation: a root whose sheet
/// carries a flipbook for the tier it is bound at gets an owner with slots; a
/// root that loses it, or goes away, loses its owner.
#[allow(clippy::too_many_arguments)]
pub fn bind_rigged_presentations(
    mut commands: Commands,
    admission: Option<Res<RiggedSpriteAdmission>>,
    assets: Option<Res<GameAssets>>,
    asset_server: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    mut owners: ResMut<RiggedPresentations>,
    mut by_sheet: Local<HashMap<(String, TextureResolutionScale), RiggedSpritePages>>,
    roots: Query<(Entity, &CharacterAnimator, Option<&BoundSpriteQuality>)>,
    presentations: Query<&RiggedPresentation>,
    mut sprites: Query<&mut Sprite>,
) {
    if !admission.is_some_and(|admission| admission.admit) {
        return;
    }
    let (Some(assets), Some(images)) = (assets, images) else {
        return;
    };
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
        commands.entity(*owner).try_despawn();
        false
    });
    for (root, animator, bound) in &roots {
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
        let tint = current.map(|current| current.tint);
        // Not until every part page is ready: the root draws nothing while it
        // has parts, so parts with no pixels would make the body vanish.
        if wanted.is_some_and(|wanted| !pages_ready(asset_server.as_deref(), &images, wanted)) {
            // ⛔ ONLY THE SAME CHARACTER KEEPS ITS OLD PARTS MEANWHILE (a tier
            // change). After a re-wear the root's animator is the new
            // character's, so the old parts would be driven with the new rows
            // (both have `idle`, `walk`). Drop them now: the root draws the new
            // character's baked sheet until its pages are ready.
            if current.is_some_and(|current| current.target != target) {
                if let Some(owner) = owners.0.remove(&root) {
                    commands.entity(owner).try_despawn();
                }
                draw_the_root_itself(&mut commands, &mut sprites, root, tint);
            }
            continue;
        }
        if let Some(owner) = owners.0.remove(&root) {
            commands.entity(owner).try_despawn();
        }
        match wanted {
            Some(pages) => {
                let owner = spawn_presentation(&mut commands, root, target, pages.clone(), tint.unwrap_or(Color::WHITE));
                owners.0.insert(root, owner);
            }
            // Back to the baked sheet: the root draws itself again.
            None => draw_the_root_itself(&mut commands, &mut sprites, root, tint),
        }
    }
}

/// A root with no parts draws its baked sheet again, in the tint its parts
/// last had.
fn draw_the_root_itself(commands: &mut Commands, sprites: &mut Query<&mut Sprite>, root: Entity, tint: Option<Color>) {
    if let (Ok(mut sprite), Some(tint)) = (sprites.get_mut(root), tint) {
        sprite.color = tint;
    }
    #[cfg(feature = "portal_render")]
    commands
        .entity(root)
        .try_remove::<ambition_portal2d_presentation::PortalPieceTint>();
    #[cfg(not(feature = "portal_render"))]
    let _ = commands;
}

/// Every page of `pages` is ready to draw. Without an asset server (a
/// composition with no asset IO) a page is ready when its image is present.
fn pages_ready(asset_server: Option<&AssetServer>, images: &Assets<Image>, pages: &RiggedSpritePages) -> bool {
    pages.pages.iter().all(|page| match asset_server {
        Some(server) => super::texture_is_ready(server, images, page),
        None => images.contains(page),
    })
}

fn spawn_presentation(commands: &mut Commands, root: Entity, target: &str, pages: RiggedSpritePages, tint: Color) -> Entity {
    let owner = commands
        .spawn((
            Name::new("rigged presentation"),
            Transform::default(),
            Visibility::Hidden,
            // Whose body these parts draw: the portal resolver hides the owner
            // with its root. Not a candidate itself (no sprite, no frame).
            ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(root),
        ))
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
        tint,
        stated_tint: None,
        drawn: Vec::new(),
    });
    owner
}

/// Draw each rigged root's current frame from parts: the owner follows the
/// root, the slots take the frame's draws, and the root's own pixels are made
/// transparent.
///
/// The slots are on the root's render layers, so each camera (each local
/// view's pane) that draws the root draws its parts. The parts are made once
/// for the body, not once for each view.
///
/// Runs after the animators, so it draws the frame they chose this frame.
pub fn drive_rigged_presentations(
    mut commands: Commands,
    mut owners: Query<(&mut RiggedPresentation, &mut Transform, &mut Visibility), Without<RiggedPartSlot>>,
    mut roots: Roots,
    mut slots: Slots,
) {
    for (mut presentation, mut owner_transform, mut owner_visibility) in &mut owners {
        let Ok((animator, mut root_sprite, root_transform, root_visibility, root_layers, root_anchor)) =
            roots.get_mut(presentation.root)
        else {
            continue;
        };
        *owner_transform = *root_transform;
        // The stance squash of a sheet without a row for the compact pose
        // (`StanceSquash`): the root's quad is drawn shorter about a line that
        // holds still. The owner takes the same squash, so the parts do too.
        if let Some((ratio, held_y)) = stance_squash(animator, &root_sprite, root_anchor) {
            owner_transform.scale.y *= ratio;
            owner_transform.translation +=
                root_transform.rotation * (root_transform.scale * Vec3::new(0.0, held_y * (1.0 - ratio), 0.0));
        }
        owner_visibility.set_if_neq(root_visibility.copied().unwrap_or(Visibility::Inherited));
        if root_sprite.color.alpha() > 0.0 {
            presentation.tint = root_sprite.color;
            root_sprite.color.set_alpha(0.0);
        }
        // Stated on the root only when it changes: the animator rewrites the
        // color every frame, mostly with the same value.
        #[cfg(feature = "portal_render")]
        if presentation.stated_tint != Some(presentation.tint) {
            presentation.stated_tint = Some(presentation.tint);
            commands
                .entity(presentation.root)
                .try_insert(ambition_portal2d_presentation::PortalPieceTint(presentation.tint));
        }
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
        let (Some(draws), Some(basis)) = (draws, animator.render_basis) else {
            presentation.drawn = drawn;
            // The baked frame draws the body.
            root_sprite.color = presentation.tint;
            hide(&presentation.slots, &mut slots);
            continue;
        };
        let flip = root_sprite.flip_x;
        let frame_size = flipbook.frame_size.as_vec2();
        let world_per_pixel = basis.render_size / frame_size;
        for (index, slot) in presentation.slots.iter().enumerate() {
            let Ok((mut sprite, mut anchor, mut transform, mut visibility, layers)) = slots.get_mut(*slot) else {
                continue;
            };
            if layers != root_layers {
                match root_layers {
                    Some(root_layers) => commands.entity(*slot).try_insert(root_layers.clone()),
                    None => commands.entity(*slot).try_remove::<RenderLayers>(),
                };
            }
            let Some(draw) = draws.get(index) else {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            };
            let part = flipbook.parts[usize::from(draw.part)];
            // The part pivot in the frame, as the baked quad maps a frame
            // pixel: anchor-normalized (y up), then from the root's anchor.
            let pixel = flipbook.feet_pixel + draw.at;
            let normalized = Vec2::new(pixel.x / frame_size.x - 0.5, 0.5 - pixel.y / frame_size.y);
            let mut local = (normalized - basis.feet_anchor) * basis.render_size;
            let mut part_anchor = part.anchor();
            // Clockwise in the sheet's +y-down frame is a negative angle in
            // Bevy's +y-up frame; a mirrored body turns the other way.
            let mut rotation = -draw.rotation;
            if flip {
                local.x = -local.x;
                part_anchor.x = -part_anchor.x;
                rotation = -rotation;
            }
            let page = &presentation.pages.pages[usize::from(part.page)];
            if sprite.image != *page {
                sprite.image = page.clone();
            }
            let rect = Rect::new(
                part.rect.min.x as f32,
                part.rect.min.y as f32,
                part.rect.max.x as f32,
                part.rect.max.y as f32,
            );
            sprite.rect = Some(rect);
            sprite.custom_size = Some(part.size * draw.scale * world_per_pixel);
            sprite.flip_x = flip;
            sprite.color = presentation.tint;
            anchor.0 = part_anchor;
            *transform = Transform::from_translation(local.extend(index as f32 * SLOT_DEPTH_STEP))
                .with_rotation(Quat::from_rotation_z(rotation));
            visibility.set_if_neq(Visibility::Inherited);
        }
        presentation.drawn = drawn;
    }
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

fn hide(slots: &[Entity], query: &mut Slots) {
    for slot in slots {
        if let Ok((_, _, _, mut visibility, _)) = query.get_mut(*slot) {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

#[cfg(test)]
mod tests;
