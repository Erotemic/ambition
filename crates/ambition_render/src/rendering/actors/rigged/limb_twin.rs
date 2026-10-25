//! A limb of a body, drawn as a picture of its own while it is in a portal
//! pair.
//!
//! A giant's arm is drawn by the giant, and the limb that goes into a pair is
//! its fist (`docs/planning/engine/portals-a-body-in-two-places.md`). The pair
//! must carry the arm, and only the arm. The picture of the whole body was cut
//! by a band past the pair's near line, and the band also held what is not the
//! arm: the giant's back came out of the far line with its fist (2026-10-24).
//!
//! So a body whose flipbook names the parts of a limb (`limb:<slot>` tracks,
//! `is_limb_track`) is drawn as three pictures while the limb is in a pair,
//! which together draw each part once, in the frame's order:
//!
//! * the body, which draws the tracks drawn before the limb's
//!   ([`RiggedTrackMask::under`]);
//! * the limb's twin ([`LimbTwin`]), a root of its own that follows the body
//!   (its animator, its size, its place, its room) and draws the limb's
//!   tracks only. The portal cuts the twin as it cuts any body in a pair;
//! * the body's over twin ([`LimbTwin::spawn_over`]), which draws the tracks drawn
//!   after the limb's first part, uncut.
//!
//! The host adapter decides when (`publish_threaded_host_pictures`): this
//! module owns the components and keeps each twin on its body.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_sprite_sheet::character::rigged::{is_limb_track, PartDraw, RiggedSpriteAsset};
use ambition_sprite_sheet::character::{CharacterAnimator, CharacterColorShift};

use super::RiggedPresentation;
use crate::rendering::actors::BoundSpriteQuality;

/// The tracks a rigged root draws, when not all of them: the parts of the
/// limbs `limbs` only (`only`), or the parts that are not theirs, all of them
/// or those drawn under or over the limbs ([`TrackLayer`]).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct RiggedTrackMask {
    pub only: bool,
    pub limbs: Vec<String>,
    pub layer: TrackLayer,
}

/// Which of the parts that are not the limbs' a root draws, by the frame's
/// draw order: all of them, those drawn before the first part of the limbs,
/// or those drawn after it.
///
/// ⛔ A LIMB DRAWN APART KEEPS ITS PLACE IN THE DRAW ORDER. The limb's twin
/// is one picture over the body; the body's parts drawn over the limb (an
/// epaulette on a shoulder) are drawn over the twin by a second twin
/// ([`LimbTwin::spawn_over`]). Drawn at one depth, the twin and the body took
/// turns on top, and the arm flickered behind the giant (2026-10-25).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackLayer {
    #[default]
    Whole,
    UnderLimbs,
    OverLimbs,
}

impl RiggedTrackMask {
    /// Draw every track but the parts of `limbs`.
    pub fn without(limbs: Vec<String>) -> Self {
        Self { only: false, limbs, layer: TrackLayer::Whole }
    }

    /// Draw the tracks drawn before the first part of `limbs`.
    pub fn under(limbs: Vec<String>) -> Self {
        Self { only: false, limbs, layer: TrackLayer::UnderLimbs }
    }

    /// Draw the tracks drawn after the first part of `limbs`, but theirs.
    pub fn over(limbs: Vec<String>) -> Self {
        Self { only: false, limbs, layer: TrackLayer::OverLimbs }
    }

    /// Draw the parts of `limb` only.
    pub fn only(limb: &str) -> Self {
        Self { only: true, limbs: vec![limb.to_owned()], layer: TrackLayer::Whole }
    }

    /// Draw nothing.
    pub fn nothing() -> Self {
        Self { only: true, limbs: Vec::new(), layer: TrackLayer::Whole }
    }

    /// The draws of `draws` the root draws. A draw of no track is a part of
    /// no limb.
    pub(super) fn keep(&self, flipbook: &RiggedSpriteAsset, draws: &[PartDraw]) -> Vec<PartDraw> {
        let of_limbs = |draw: &PartDraw| {
            draw.track
                .and_then(|track| flipbook.tracks.get(usize::from(track)))
                .is_some_and(|name| self.limbs.iter().any(|slot| is_limb_track(name, slot)))
        };
        let first = draws.iter().position(of_limbs);
        draws
            .iter()
            .enumerate()
            .filter(|(at, draw)| match (self.only, of_limbs(draw)) {
                (true, of) => of,
                (false, true) => false,
                (false, false) => match self.layer {
                    TrackLayer::Whole => true,
                    TrackLayer::UnderLimbs => first.is_none_or(|first| *at < first),
                    TrackLayer::OverLimbs => first.is_some_and(|first| *at > first),
                },
            })
            .map(|(_, draw)| *draw)
            .collect()
    }
}

/// Does the flipbook of this presentation name the parts of the limb `slot`?
/// A body that does not is cut as one picture.
pub fn limb_tracks_drawn(presentation: &RiggedPresentation, slot: &str) -> bool {
    presentation.pages.flipbook.tracks.iter().any(|name| is_limb_track(name, slot))
}

/// On the picture of one limb of a body: the body's root, and the limb's slot.
/// It is despawned with the root. A twin of no slot ([`Self::over`]) draws the
/// body's parts that are drawn over its limbs, while its limbs go as twins.
#[derive(Component, Debug)]
#[relationship(relationship_target = LimbTwins)]
pub struct LimbTwin {
    #[relationship]
    pub of: Entity,
    pub slot: String,
}

/// On a root: the pictures of its limbs ([`LimbTwin`]).
#[derive(Component, Debug)]
#[relationship_target(relationship = LimbTwin, linked_spawn)]
pub struct LimbTwins(Vec<Entity>);

impl LimbTwins {
    /// The twins of the root.
    pub fn twins(&self) -> &[Entity] {
        &self.0
    }
}

/// How far in front of its body a limb's twin is drawn, and the twin of the
/// parts over the limbs in front of that: a body drawn directly spreads its
/// parts over less than this (`SLOT_DEPTH_STEP` a part).
const TWIN_DEPTH: f32 = 0.05;

impl LimbTwin {
    /// Is it the twin of the parts drawn over the limbs, not of a limb?
    pub fn is_over(&self) -> bool {
        self.slot.is_empty()
    }

    /// The twin of the parts of `root` drawn over its limbs, hidden. Its mask
    /// is set by who sends the limbs apart.
    pub fn spawn_over(commands: &mut Commands, root: Entity) -> Entity {
        commands
            .spawn((
                LimbTwin { of: root, slot: String::new() },
                RiggedTrackMask::nothing(),
                Sprite::default(),
                Anchor::default(),
                Transform::default(),
                Visibility::Hidden,
                Name::new("Limb twin (over the limbs)"),
            ))
            .id()
    }

    /// A twin of the limb `slot` of `root`, hidden. It draws the limb's tracks
    /// only, and [`follow_limb_twins`] keeps it on the root.
    pub fn spawn(commands: &mut Commands, root: Entity, slot: &str) -> Entity {
        commands
            .spawn((
                LimbTwin { of: root, slot: slot.to_owned() },
                RiggedTrackMask::only(slot),
                Sprite::default(),
                Anchor::default(),
                Transform::default(),
                Visibility::Hidden,
                Name::new(format!("Limb twin ({slot})")),
            ))
            .id()
    }
}

/// Keep each twin on its root: the root's animator (its row and frame), its
/// mirror, tint, size, anchor and place, its layers, room, tier and colour,
/// as the root's animator drew them: after the animators, before the rigged
/// driver. A limb's twin is drawn in front of its body, and the twin of the
/// parts over the limbs in front of that. A twin draws its parts directly: a
/// portal pair carries them as they are drawn (the portal presentation's
/// carry), so no image of them is waited for.
#[allow(clippy::type_complexity)]
pub fn follow_limb_twins(
    mut commands: Commands,
    mut twins: Query<
        (
            Entity,
            &LimbTwin,
            Option<&mut CharacterAnimator>,
            &mut Sprite,
            &mut Anchor,
            &mut Transform,
            (
                Option<&RenderLayers>,
                Option<&InRoomInstance>,
                Option<&BoundSpriteQuality>,
                Option<&CharacterColorShift>,
            ),
        ),
        Without<LimbTwins>,
    >,
    roots: Query<
        (
            &CharacterAnimator,
            &Sprite,
            Option<&Anchor>,
            &Transform,
            (
                Option<&RenderLayers>,
                Option<&InRoomInstance>,
                Option<&BoundSpriteQuality>,
                Option<&CharacterColorShift>,
            ),
        ),
        (With<LimbTwins>, Without<LimbTwin>),
    >,
) {
    for (twin, of, animator, mut sprite, mut anchor, mut transform, (layers, room, tier, shift)) in &mut twins {
        let Ok((root_animator, root_sprite, root_anchor, root_transform, (root_layers, root_room, root_tier, root_shift))) =
            roots.get(of.of)
        else {
            continue;
        };
        match animator {
            Some(mut animator) => animator.clone_from(root_animator),
            None => {
                commands.entity(twin).insert(root_animator.clone());
            }
        }
        sprite.flip_x = root_sprite.flip_x;
        sprite.color = root_sprite.color;
        // ⛔ THE SIZE AS THE ANIMATOR DREW IT. The rigged driver reads a
        // root's size against its animator's frame as a stance squash, and
        // then writes its cell's size over it. No animator writes a twin's
        // size, so a twin that kept its own read its last cell as a squash
        // and grew each frame, to millions of pixels (2026-10-25).
        sprite.custom_size = root_sprite.custom_size;
        if let Some(root_anchor) = root_anchor {
            *anchor = *root_anchor;
        }
        *transform = *root_transform;
        transform.translation.z += if of.is_over() { 2.0 * TWIN_DEPTH } else { TWIN_DEPTH };
        // The facts that are the root's own: put on, changed or taken off
        // with it.
        let mut twin = commands.entity(twin);
        match (root_layers, layers) {
            (Some(root), Some(own)) if root == own => {}
            (Some(root), _) => {
                twin.insert(root.clone());
            }
            (None, Some(_)) => {
                twin.remove::<RenderLayers>();
            }
            (None, None) => {}
        }
        match (root_room, room) {
            (Some(root), Some(own)) if root == own => {}
            (Some(root), _) => {
                twin.insert(*root);
            }
            (None, Some(_)) => {
                twin.remove::<InRoomInstance>();
            }
            (None, None) => {}
        }
        match (root_tier, tier) {
            (Some(root), Some(own)) if root == own => {}
            (Some(root), _) => {
                twin.insert(*root);
            }
            (None, Some(_)) => {
                twin.remove::<BoundSpriteQuality>();
            }
            (None, None) => {}
        }
        match (root_shift, shift) {
            (Some(root), Some(own)) if root == own => {}
            (Some(root), _) => {
                twin.insert(*root);
            }
            (None, Some(_)) => {
                twin.remove::<CharacterColorShift>();
            }
            (None, None) => {}
        }
    }
}
