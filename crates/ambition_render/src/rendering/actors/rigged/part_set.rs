//! Where a named set of a body's parts is drawn, for a reader that draws
//! about a part of a body and not about the whole of it.
//!
//! A body read as one image is composited (`ComposedBodyDemand`), and that
//! image has no parts: a reader of it cannot tell a wing from an arm. A
//! reader that needs the difference (a barrier about a body and not about
//! its wings) puts a [`PartSetRequest`] on the root. Each frame the body is
//! drawn from parts, the driver writes [`PartSetDraws`] on the same root:
//! the draws of those tracks, and the place of the body's feet in the world.
//! The reader draws what it wants with them. Nothing here draws.

use bevy::prelude::*;

use ambition_sprite_sheet::character::rigged::PartDraw;

use super::{RiggedPresentation, SLOT_DEPTH_STEP};

/// A reader's ask of a rigged root: the draws of these tracks, each frame.
///
/// A track is named as the body's flipbook names it
/// (`RiggedSpriteAsset::tracks`): `torso`, `overlay:hair`. A name the
/// flipbook does not have matches nothing.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
#[require(PartSetDraws)]
pub struct PartSetRequest {
    pub tracks: Vec<String>,
}

/// The answer to a [`PartSetRequest`], written by the rigged-sprite driver.
///
/// Empty on a frame the body is not drawn from parts (a baked row of a
/// hybrid, a sheet with no flipbook, a tier that is still loading).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PartSetDraws {
    /// The body's feet in the world. Under it one unit is one sheet pixel and
    /// +y is up; it carries the body's size, its mirror and its turn. It is
    /// the same on both roads (parts in the world, and a composited cell).
    pub place: Transform,
    /// The draws of the asked tracks, in paint order.
    pub parts: Vec<PlacedPart>,
}

/// One part of a [`PartSetDraws`].
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedPart {
    /// The page the part is on.
    pub image: Handle<Image>,
    /// The part on its page, in texels of the page.
    pub rect: Rect,
    /// The part's size, in sheet pixels (a quality tier keeps it).
    pub size: Vec2,
    /// The part's pivot as a sprite anchor: `(-0.5, -0.5)` is the bottom left.
    pub anchor: Vec2,
    /// The pivot under [`PartSetDraws::place`]: where, turned and scaled as
    /// the draw is. Its z is the depth of the draw among the body's draws.
    pub local: Transform,
}

/// Write the answer for one root: the draws of `draws` on the asked tracks.
pub(super) fn answer(
    request: &PartSetRequest,
    answer: &mut Mut<PartSetDraws>,
    presentation: &RiggedPresentation,
    draws: &[PartDraw],
    place: Transform,
) {
    let flipbook = &presentation.pages.flipbook;
    let mut parts = Vec::new();
    for (index, draw) in draws.iter().enumerate() {
        let asked = draw
            .track
            .and_then(|track| flipbook.tracks.get(usize::from(track)))
            .is_some_and(|name| request.tracks.contains(name));
        if !asked {
            continue;
        }
        let part = flipbook.parts[usize::from(draw.part)];
        let Some(image) = presentation.pages.pages.get(usize::from(part.page)) else {
            continue;
        };
        parts.push(PlacedPart {
            image: image.clone(),
            rect: Rect::new(
                part.rect.min.x as f32,
                part.rect.min.y as f32,
                part.rect.max.x as f32,
                part.rect.max.y as f32,
            ),
            size: part.size,
            anchor: part.anchor(),
            // As a slot is placed (`write_slots`).
            local: Transform::from_translation(Vec3::new(draw.at.x, -draw.at.y, index as f32 * SLOT_DEPTH_STEP))
                .with_rotation(Quat::from_rotation_z(-draw.rotation))
                .with_scale(draw.scale.extend(1.0)),
        });
    }
    let next = PartSetDraws { place, parts };
    if **answer != next {
        **answer = next;
    }
}

/// Nothing of the body is drawn from parts this frame.
pub(super) fn clear(answer: &mut Mut<PartSetDraws>) {
    if !answer.parts.is_empty() {
        answer.parts.clear();
    }
}
