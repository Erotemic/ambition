//! Portal shots in flight. A shot is the portal crate's, whatever fired it.

use bevy::prelude::*;

use ambition_portal2d::{PortalChannel, PortalShot};

use crate::{PortalFrames, PortalVisual};

/// What a portal visual draws: whose visual it is, in which live room, and
/// what it looks like. A visual is kept, and moved, while its source wants the
/// same look in the same room; any other look is a new visual.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PortalVisualDrawn {
    pub source: Entity,
    pub room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    pub look: PortalVisualLook,
}

/// The look of one portal visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PortalVisualLook {
    /// A shot in flight, in its pair's colour.
    Shot(PortalChannel),
    /// A portal's colour-name label.
    Label(PortalChannel),
}

/// In-flight portal shots, each with where it is drawn.
pub(crate) fn want_portal_shot_visuals(
    frames: &PortalFrames,
    projectiles: &Query<(Entity, &PortalShot)>,
    wanted: &mut Vec<(PortalVisualDrawn, Transform)>,
) {
    for (shot, proj) in projectiles.iter() {
        // In the shot's own live room (it carries the room it was fired in).
        let Some(placement) = frames.of(shot) else {
            continue;
        };
        let translation = placement.frame.to_render(proj.pos, 9.5);
        wanted.push((
            PortalVisualDrawn { source: shot, room: placement.room, look: PortalVisualLook::Shot(proj.channel) },
            Transform::from_translation(translation),
        ));
    }
}

/// Spawn the visual of a shot. A label is spawned by `sync_portal_visuals`.
pub(crate) fn spawn_shot_visual(commands: &mut Commands, drawn: PortalVisualDrawn, transform: Transform) {
    let stamp = ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(drawn.room);
    if let PortalVisualLook::Shot(channel) = drawn.look {
        commands.spawn((
            PortalVisual,
            drawn,
            Sprite::from_color(channel.display().1, Vec2::new(16.0, 8.0)),
            transform,
            stamp,
            Name::new("Portal shot visual"),
        ));
    }
}
