//! What the passages through doors and gates look like this tick
//! (`rooms::passage`): presentation reads these, never the passage.
//!
//! - A body in a passage draws the row its passage names, at the size, opacity
//!   and shade its passage gives ([`PassageLookView`] beside its
//!   `BodyPoseView`).
//! - A door opens and shuts with the passage through it, and a gate ripples
//!   ([`PassageViews`]).

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::rooms::passage::{PassagePose, ZonePassage};
use ambition_sprite_sheet::character::CharacterAnim;

use crate::{BodyPoseView, ClipRequest};

/// How a body in a passage is drawn: about its feet, `scale` of its size,
/// `alpha` of it showing, `shade` of the way to black.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PassageLookView {
    pub scale: f32,
    pub alpha: f32,
    pub shade: f32,
}

/// The doors and gates passages are going through this tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PassageViews {
    /// A door's zone and how open it is (0 shut, 1 open).
    pub doors: Vec<(String, f32)>,
    /// A gate's zone, the ripple row it plays (`swallow`, `emit`) and how far
    /// through it.
    pub gates: Vec<(String, &'static str, f32)>,
}

impl PassageViews {
    /// How open the door of `zone` is, when a passage goes through it.
    pub fn door_open(&self, zone: &str) -> Option<f32> {
        self.doors.iter().filter(|(id, _)| id == zone).map(|(_, open)| *open).reduce(f32::max)
    }

    /// The ripple the gate of `zone` plays, when a passage goes through it.
    pub fn gate_ripple(&self, zone: &str) -> Option<(&'static str, f32)> {
        self.gates.iter().find(|(id, ..)| id == zone).map(|(_, row, k)| (*row, *k))
    }
}

fn row_of(pose: PassagePose) -> (&'static [&'static str], CharacterAnim) {
    match pose {
        PassagePose::TurnAway => (&["turn_away", "climb"], CharacterAnim::LadderClimb),
        PassagePose::WalkAway => (&["walk_away", "climb"], CharacterAnim::LadderClimb),
        PassagePose::BackTo => (&["climb"], CharacterAnim::LadderClimb),
        PassagePose::Walk => (&["walk"], CharacterAnim::Walk),
        PassagePose::Jump => (&["jump"], CharacterAnim::Jump),
        PassagePose::Fall => (&["fall"], CharacterAnim::Fall),
    }
}

/// Publish each passage's look, after the pose views are rebuilt.
pub fn rebuild_passage_views(
    mut commands: Commands,
    mut views: ResMut<PassageViews>,
    mut bodies: Query<(Entity, &ZonePassage, Option<&mut BodyPoseView>, Option<&PassageLookView>)>,
    stale: Query<Entity, (With<PassageLookView>, Without<ZonePassage>)>,
) {
    let mut next = PassageViews::default();
    for (entity, passage, pose, shown) in &mut bodies {
        if let Some((zone, open)) = passage.door_open() {
            next.doors.push((zone.to_string(), open));
        }
        if let Some((zone, row, k)) = passage.gate_ripple() {
            next.gates.push((zone.to_string(), row, k));
        }
        match passage.look() {
            Some(look) => {
                if let Some(mut pose) = pose {
                    let (chain, anim) = row_of(look.pose);
                    pose.anim = anim;
                    pose.clip = ClipRequest::from_chain(chain);
                }
                let view = PassageLookView { scale: look.scale, alpha: look.alpha, shade: look.shade };
                if shown != Some(&view) {
                    commands.entity(entity).try_insert(view);
                }
            }
            None => {
                if shown.is_some() {
                    commands.entity(entity).try_remove::<PassageLookView>();
                }
            }
        }
    }
    for entity in &stale {
        commands.entity(entity).try_remove::<PassageLookView>();
    }
    if *views != next {
        *views = next;
    }
}
