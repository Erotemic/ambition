//! THE TETHER LINE — a grab you can see coming.
//!
//! ⭐⭐ A 150px GRAB THAT DRAWS NOTHING IS THE MECHANIC WITHOUT THE READ. A grab
//! whose reach is the length of an arm needs no line; one that crosses a third
//! of the stage is unreadable without one, and in a 1v1 neither player can
//! respect a threat they cannot see.
//!
//! ⛔⛔ BOTH ROADS, AND THAT IS THE WHOLE REASON THIS FILE IS SHAPED LIKE
//! `flyline.rs`. `PlayerVisual` is inserted in exactly one place in the engine —
//! the session's single exploration player — so a visual gated on it alone
//! appears in an Ambition room and never once in a versus match. That is what
//! happened to the trapdoor, and it is what the charge indicator still does.
//! Every match fighter is a `FeatureVisual` reading `FeatureViewIndex`.
//!
//! ⭐ IT DRAWS THE ROPE THE FLYLINE ALREADY BUILT. Same procedural sprite, same
//! placement helper — a tether and a flying wire are one shape at two lengths,
//! and a second rope texture would be a second thing to keep in step.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};

/// The tether a body is currently reaching with.
#[derive(Component)]
pub struct TetherVisual {
    /// The body doing the reaching. ⛔ One per body, never a singleton: a match
    /// has four fighters and any of them may be the one with the long grab.
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
    // ⛔ `Option`, and not defensively: a plain `Res` is a hard stop for any
    // composition that does not build the index, with an undebuggable
    // "Resource does not exist". The flyline states the same reason.
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    mut lines: Query<(Entity, &TetherVisual, &mut Transform, &mut Sprite)>,
) {
    // Both roads reduced to the two facts a line needs: where the body is, and
    // where it is reaching. Nothing below has to learn there are two kinds of
    // body visual.
    let mut reaching: Vec<(Entity, bevy::math::Vec2, bevy::math::Vec2)> = Vec::new();
    for (body, pose, presented) in &bodies {
        if let Some(reach) = pose.grab_reach {
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
        if let Some(reach) = view.grab_reach {
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
                Name::new("Tether line"),
            ),
        );
    }
}

#[cfg(test)]
mod tests;
