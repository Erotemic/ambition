//! What a body's interact press reaches, for observation (GOBS-3,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! `interaction.reach` asks the rule the press and the prompt ask
//! ([`InteractReach`], and gameplay's choice of target,
//! `ambition_platformer2d_actor::body::InteractTargets`) for one body:
//! - its touch box (`interact_touch`: what a chest or a switch must touch),
//!   its talk box (`interact_talk`) and the point it reaches with
//!   (`interact_front`: the press is for the nearest thing to it);
//! - every person, switch and chest in its live room (`talk`, `switch`,
//!   `chest`), active when the press could take it, else inactive with the
//!   clause of the rule that refuses it; the one the press is for also has
//!   the role `interact_chosen`.

use std::borrow::Cow;

use ambition_geometry_observation::{
    GeometryObservation, GeometryObserverAppExt as _, ObservationQuery, ObservedShape, ObservedState, ObserverInfo,
};
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_interaction::InteractReach;
use bevy::prelude::*;

use crate::affordances::interactable_proximity::{Chests, Reaching, Switches, Talkers};

pub fn install(app: &mut App) {
    app.add_geometry_observer(
        ObserverInfo {
            name: "interaction.reach",
            description: "what one body's interact press reaches: its boxes, and each person, switch and chest in its room, with why one is refused",
            needs_subject: true,
        },
        reach_of_subject,
    );
}

fn piece(key: String, role: &'static str, chosen: bool, shape: ObservedShape, refused: Option<&'static str>) -> GeometryObservation {
    let mut roles = vec![Cow::Borrowed(role)];
    if chosen {
        roles.push(Cow::Borrowed("interact_chosen"));
    }
    GeometryObservation {
        key,
        roles,
        shape,
        state: refused.map_or(ObservedState::Active, |reason| ObservedState::Inactive { reason: Cow::Borrowed(reason) }),
    }
}

fn boxed(aabb: &CenteredAabb) -> ObservedShape {
    let aabb = aabb.aabb();
    ObservedShape::aabb(aabb.min.to_array(), aabb.max.to_array())
}

fn reach_of_subject(
    query: In<ObservationQuery>,
    bodies: Query<Reaching>,
    names: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    actors: Talkers,
    chests: Chests,
    switches: Switches,
    targets: ambition_platformer2d_actor::body::InteractTargets,
    geometry: Option<Res<ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>>,
) -> Vec<GeometryObservation> {
    let Some(subject) = query.subject else {
        return Vec::new();
    };
    let Ok((kin, step, frame)) = bodies.get(subject) else {
        return Vec::new();
    };
    let room = rooms.live().of(subject);
    let reach = InteractReach::of(kin, step, frame, room,
        geometry.as_deref().and_then(|geometry| geometry.motion_for(subject, room)));
    let door = rooms.nearest_door_under(subject, reach.collision_box(), reach.pos());
    let chosen = targets.chosen(subject, &reach, door).map(|(entity, _)| entity);
    let name = |entity: Entity| names.get(entity).map_or_else(|_| format!("{entity}"), |id| id.as_str().to_owned());
    let me = name(subject);
    let (touch, talk) = (reach.collision_box(), reach.talk_box());
    let mut out = vec![
        piece(format!("{me}/touch"), "interact_touch", false, ObservedShape::aabb(touch.min.to_array(), touch.max.to_array()), None),
        piece(format!("{me}/talk"), "interact_talk", false, ObservedShape::aabb(talk.min.to_array(), talk.max.to_array()), None),
        piece(
            format!("{me}/front"),
            "interact_front",
            false,
            ObservedShape::Circle { center: reach.front().to_array(), radius: 3.0 },
            None,
        ),
    ];
    if let Some(spec) = rooms.spec_of(subject) {
        use ambition_platformer2d_core::AabbExt as _;
        for zone in spec.loading_zones.iter().filter(|zone| zone.activation == ambition_platformer2d_world::rooms::LoadingZoneActivation::Door) {
            let distance = zone.aabb.center().distance(reach.pos());
            let refused = if !touch.strict_intersects(zone.aabb) {
                Some("outside the door")
            } else if door.is_some_and(|nearest| distance > nearest) {
                Some("a nearer door takes the press")
            } else if chosen.is_some() {
                Some("an interactable takes the press")
            } else {
                None
            };
            out.push(piece(format!("door {}", zone.id), "door", refused.is_none(),
                ObservedShape::aabb(zone.aabb.min.to_array(), zone.aabb.max.to_array()), refused));
        }
    }
    for (entity, aabb, disposition, aggression, health, out_of_play, plane, requires_facing) in &actors {
        if entity == subject || rooms.live().of(entity) != room {
            continue;
        }
        let refused = if let Some(reason) = ambition_combat::util::talk_refusal(*disposition, aggression, health, out_of_play, plane) {
            Some(reason)
        } else if let Some(reason) = reach.refuses_talk(room, aabb, requires_facing) {
            Some(reason)
        } else if InteractReach::a_door_keeps_the_press(door, aabb.center.distance(reach.pos())) {
            Some("a door nearer takes the press")
        } else {
            None
        };
        out.push(piece(name(entity), "talk", chosen == Some(entity), boxed(aabb), refused));
    }
    for (entity, aabb, requires_facing) in &switches {
        if rooms.live().of(entity) != room {
            continue;
        }
        let refused = reach.refuses_touch(room, aabb, requires_facing);
        out.push(piece(name(entity), "switch", chosen == Some(entity), boxed(aabb), refused));
    }
    for (entity, aabb, opened, falling) in &chests {
        if rooms.live().of(entity) != room {
            continue;
        }
        let refused = if opened.is_some() {
            Some("opened")
        } else if falling {
            Some("falling")
        } else {
            reach.refuses_touch(room, aabb, false)
        };
        out.push(piece(name(entity), "chest", chosen == Some(entity), boxed(aabb), refused));
    }
    out
}
