//! The tether line: a grab you can see coming.
//!
//! A grab with arm's-length reach needs no line. One that crosses a third of
//! the stage is unreadable without one, and neither player can respect a
//! threat they cannot see.
//!
//! Both body roads are required, like `flyline.rs`. `PlayerVisual` is only on
//! the session's single exploration player, so a visual gated on it never
//! appears in a versus match. Every match fighter is a `FeatureVisual` that
//! reads `FeatureViewIndex`.
//!
//! It draws the flyline's rope: same procedural sprite and placement helper.
//! A tether and a flying wire are one shape at two lengths.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};

/// The tether a body is currently reaching with.
#[derive(Component)]
pub struct TetherVisual {
    /// The body doing the reaching. One per body, never a singleton: a match
    /// has four fighters and any of them may have the long grab.
    pub body: Entity,
}

/// Draw a line from each reaching body to where its grab actually reaches.
pub fn sync_tether_visuals(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    sprite: Option<Res<super::flyline::FlylineSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    bodies: Query<
        (
            Entity,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
        ),
        With<PlayerVisual>,
    >,
    actors: Query<(Entity, &super::FeatureVisual), Without<PlayerVisual>>,
    // `Option`: a plain `Res` fails any composition that does not build the
    // index ("Resource does not exist"). Same reason as the flyline.
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    mut lines: Query<(Entity, &TetherVisual, &mut Transform, &mut Sprite)>,
) {
    // Both roads reduced to two facts: where the body is and where it
    // reaches. The code below reads only this.
    let mut reaching: Vec<(Entity, bevy::math::Vec2, bevy::math::Vec2)> = Vec::new();
    for (body, pose, presented) in &bodies {
        // Either fact draws the same line. A live grab reaches to a point; a
        // ledge tether reels toward a point it latched. A new mechanic that
        // publishes `line_anchor` draws itself.
        //
        // The grab wins a tie: a capture window is shorter-lived and more
        // urgent, so a fighter reeling to a ledge with a live grab box is
        // threatening with the grab.
        if let Some(reach) = pose.grab_reach.or(pose.line_anchor) {
            reaching.push((
                body,
                ambition_sim_view::presented_pose::draw_pos(pose, presented),
                bevy::math::Vec2::new(reach.x, reach.y),
            ));
        }
    }
    for (body, visual) in &actors {
        let Some(view) = feature_views.as_ref().and_then(|i| i.get(&visual.id)) else {
            continue;
        };
        // Same rule as the player road above.
        if let Some(reach) = view.grab_reach.or(view.line_anchor) {
            reaching.push((
                body,
                bevy::math::Vec2::new(view.pos.x, view.pos.y),
                bevy::math::Vec2::new(reach.x, reach.y),
            ));
        }
    }

    let mut standing = bevy::platform::collections::HashSet::new();
    for (line, owner, mut transform, mut art) in &mut lines {
        let Some((_, from, to)) = reaching.iter().copied().find(|(b, _, _)| *b == owner.body)
        else {
            commands.entity(line).despawn();
            continue;
        };
        standing.insert(owner.body);
        super::flyline::place_wire(&world.0, &mut transform, &mut art, to, from);
    }

    let Some(sprite) = sprite else {
        return;
    };
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(
        active_session.as_deref(),
    ) else {
        return;
    };
    for (body, from, to) in reaching {
        if standing.contains(&body) {
            continue;
        }
        let mut transform = Transform::default();
        let mut art = Sprite::from_image(sprite.handle.clone());
        super::flyline::place_wire(&world.0, &mut transform, &mut art, to, from);
        commands.spawn_session_scoped(
            session_scope,
            (
                art,
                transform,
                TetherVisual { body },
                // Which body this drawable draws, in the shared spelling that any
                // consumer can query. `TetherVisual` above keeps it for placement.
                ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(body),
                Name::new("Tether line"),
            ),
        );
    }
}

#[cfg(test)]
mod tests;
