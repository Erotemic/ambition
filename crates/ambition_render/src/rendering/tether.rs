//! A line from a body to its grab or latched anchor.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope;

/// The tether a body is currently reaching with.
pub enum TetherEffect {}

pub type TetherVisual = super::attached_visuals::BodyAttachedVisual<TetherEffect>;

/// Draw a line from each reaching body to where its grab actually reaches.
pub fn sync_tether_visuals(
    mut commands: Commands,
    bodies: super::presented_bodies::PresentedBodies,
    sprite: Option<Res<super::flyline::FlylineSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut lines: Query<(Entity, &mut TetherVisual, &mut Transform, &mut Sprite)>,
) {
    super::attached_visuals::reconcile_presented_visuals(
        &mut commands,
        &bodies,
        active_session.as_deref(),
        sprite.as_ref().map(|s| &s.handle),
        &mut lines,
        "Tether line",
        |body| {
            body.grab_reach
                .or(body.line_anchor)
                .map(|reach| (reach, body.pos))
        },
        |room, transform, art, (reach, at)| {
            super::flyline::place_wire(room, transform, art, *reach, *at)
        },
    );
}

#[cfg(test)]
mod tests;
