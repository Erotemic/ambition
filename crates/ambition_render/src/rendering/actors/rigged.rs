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
//! The root keeps its baked sprite with zero alpha. That keeps the baked sheet
//! as the parity oracle of this trial, and keeps the root the body's ONE portal
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
//! ⛔ Nothing here runs unless [`RiggedSpriteAdmission`] admits the trial.
//! Known gaps of the trial: the crouch squash of a sheet without a crouch row
//! is not applied to parts, and the hit flash copies the invisible root sprite.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_persistence::settings::TextureResolutionScale;
use ambition_sprite_sheet::character::rigged::{RiggedSpriteAdmission, RiggedSpritePages};
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
    pub pages: RiggedSpritePages,
    /// Reusable part sprites, children of the owner, in draw order.
    pub slots: Vec<Entity>,
    /// The root's last visible tint. The root is drawn with zero alpha, so the
    /// tint is kept here for the parts.
    pub tint: Color,
    /// The tint last stated on the root as its portal piece tint.
    pub stated_tint: Option<Color>,
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
pub fn bind_rigged_presentations(
    mut commands: Commands,
    admission: Option<Res<RiggedSpriteAdmission>>,
    assets: Option<Res<GameAssets>>,
    mut owners: ResMut<RiggedPresentations>,
    mut by_sheet: Local<HashMap<(String, TextureResolutionScale), RiggedSpritePages>>,
    roots: Query<(Entity, &CharacterAnimator, Option<&BoundSpriteQuality>)>,
    presentations: Query<&RiggedPresentation>,
    mut sprites: Query<&mut Sprite>,
) {
    if !admission.is_some_and(|admission| admission.admit) {
        return;
    }
    let Some(assets) = assets else {
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
        let tint = current.map(|current| current.tint);
        if let Some(owner) = owners.0.remove(&root) {
            commands.entity(owner).try_despawn();
        }
        match wanted {
            Some(pages) => {
                let owner = spawn_presentation(&mut commands, root, pages.clone(), tint.unwrap_or(Color::WHITE));
                owners.0.insert(root, owner);
            }
            // Back to the baked sheet: the root draws itself again.
            None => {
                if let (Ok(mut sprite), Some(tint)) = (sprites.get_mut(root), tint) {
                    sprite.color = tint;
                }
                #[cfg(feature = "portal_render")]
                commands
                    .entity(root)
                    .try_remove::<ambition_portal2d_presentation::PortalPieceTint>();
            }
        }
    }
}

fn spawn_presentation(commands: &mut Commands, root: Entity, pages: RiggedSpritePages, tint: Color) -> Entity {
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
        pages,
        slots,
        tint,
        stated_tint: None,
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
        let Ok((animator, mut root_sprite, root_transform, root_visibility, root_layers)) =
            roots.get_mut(presentation.root)
        else {
            continue;
        };
        *owner_transform = *root_transform;
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
        let flipbook = &presentation.pages.flipbook;
        // `None` for a baked clip of a hybrid: `check_rows` at attach makes
        // sure that every other row has draws.
        let draws = animator
            .drawn_row()
            .and_then(|row| animator.spec.row_name(row))
            .and_then(|row| flipbook.frame(row, animator.frame));
        let (Some(draws), Some(basis)) = (draws, animator.render_basis) else {
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
    }
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
