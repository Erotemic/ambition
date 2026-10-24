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
//! `is_limb_track`) is drawn as two pictures while the limb is in a pair:
//!
//! * the body, which draws every track but the limb's ([`RiggedTrackMask`]);
//! * the limb's twin ([`LimbTwin`]), a root of its own that follows the body
//!   (its animator, its place, its room) and draws the limb's tracks only.
//!   The portal cuts the twin as it cuts any body in a pair.
//!
//! The host adapter decides when (`publish_threaded_host_pictures`): this
//! module owns the two components and keeps each twin on its body.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_sprite_sheet::character::rigged::{is_limb_track, ComposedBodyDemand, PartDraw, RiggedSpriteAsset};
use ambition_sprite_sheet::character::{CharacterAnimator, CharacterColorShift};

use super::RiggedPresentation;
use crate::rendering::actors::BoundSpriteQuality;

/// The tracks a rigged root draws, when not all of them: the parts of the
/// limbs `limbs` only (`only`), or every part but theirs.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct RiggedTrackMask {
    pub only: bool,
    pub limbs: Vec<String>,
}

impl RiggedTrackMask {
    /// Draw every track but the parts of `limbs`.
    pub fn without(limbs: Vec<String>) -> Self {
        Self { only: false, limbs }
    }

    /// Draw the parts of `limb` only.
    pub fn only(limb: &str) -> Self {
        Self { only: true, limbs: vec![limb.to_owned()] }
    }

    /// Does the root draw the track `name`?
    pub fn draws_track(&self, name: &str) -> bool {
        self.limbs.iter().any(|slot| is_limb_track(name, slot)) == self.only
    }

    /// The draws of `draws` the root draws. A draw of no track is a part of
    /// no limb.
    pub(super) fn keep(&self, flipbook: &RiggedSpriteAsset, draws: &[PartDraw]) -> Vec<PartDraw> {
        draws
            .iter()
            .filter(|draw| {
                draw.track
                    .and_then(|track| flipbook.tracks.get(usize::from(track)))
                    .map_or(!self.only, |name| self.draws_track(name))
            })
            .copied()
            .collect()
    }
}

/// Does the flipbook of this presentation name the parts of the limb `slot`?
/// A body that does not is cut as one picture.
pub fn limb_tracks_drawn(presentation: &RiggedPresentation, slot: &str) -> bool {
    presentation.pages.flipbook.tracks.iter().any(|name| is_limb_track(name, slot))
}

/// On the picture of one limb of a body: the body's root, and the limb's slot.
/// It is despawned with the root.
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

impl LimbTwin {
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
/// mirror, tint, anchor and place, its layers, room, tier and colour. A twin
/// that shows is read as one image (it is cut by a portal pair), so it is
/// composited.
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
            &Visibility,
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
    demand: Option<ResMut<ComposedBodyDemand>>,
) {
    let mut demand = demand;
    for (twin, of, animator, mut sprite, mut anchor, mut transform, visibility, (layers, room, tier, shift)) in &mut twins {
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
        if let Some(root_anchor) = root_anchor {
            *anchor = *root_anchor;
        }
        *transform = *root_transform;
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
        if !matches!(visibility, Visibility::Hidden) {
            if let Some(demand) = demand.as_mut() {
                demand.declare(twin.id());
            }
        }
    }
}
