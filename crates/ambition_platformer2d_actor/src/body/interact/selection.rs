//! What each driven body's Interact press is for: the selection the prompt
//! draws and the contextual consumers yield to (interaction I1,
//! `docs/planning/engine/interaction-opportunities-and-activation.md`).
//!
//! It lives with the gameplay that executes the press, not with the prompt:
//! a headless composition with the same gameplay domains selects the same
//! operation without installing any display, and leaving a display out
//! cannot change which operation takes a press. Each domain supplies its own
//! availability: a talker is offered only with a conversation the dialogue
//! domain would open ([`super::TalkableBodies`]), a mount only when the
//! mount domain would let this body board it.

use ambition_combat::components::CenteredAabb;
use ambition_encounter::switches::SwitchFeature;
use ambition_platformer2d_interaction::{
    InteractCandidate, InteractKind, InteractOperation, InteractReach, InteractSelected, InteractSelection, interact_order,
};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_world_items::containers::{ChestFeature, FallingChest, Opened};
use bevy::prelude::*;

/// What one body's press reaches, from every domain that offers a target in
/// reach: bodies to talk to, switches and chests. The one choice the
/// selection, and an observer explaining it, ask.
#[derive(bevy::ecs::system::SystemParam)]
pub struct InteractTargets<'w, 's> {
    talkable: super::TalkableBodies<'w, 's>,
    switches: Query<
        'w,
        's,
        (Entity, &'static CenteredAabb, Has<ambition_combat::components::RequiresFacing>),
        (With<FeatureSimEntity>, With<SwitchFeature>),
    >,
    chests: Query<
        'w,
        's,
        (Entity, &'static CenteredAabb, Option<&'static Opened>, Has<FallingChest>),
        (With<FeatureSimEntity>, With<ChestFeature>),
    >,
    // The stable identities that break a tie, as the press breaks it.
    objects: ambition_platformer2d_shared_tangle::lifecycle::LiveObjects<'w, 's>,
}

impl InteractTargets<'_, '_> {
    /// The target `subject`'s press is for, and what it does to it: the first
    /// of [`interact_order`] over what the press reaches. `door` is the
    /// distance to the door the body stands in, if any, which keeps the press
    /// from a body to talk to no nearer than it.
    pub fn chosen(&self, subject: Entity, reach: &InteractReach, door: Option<f32>) -> Option<(Entity, InteractKind)> {
        let rooms = self.talkable.rooms();
        let candidate = |kind, center, entity| InteractCandidate {
            kind,
            preference: reach.preference(center),
            id: (self.objects.reference(entity), entity),
        };
        let talkers = self
            .talkable
            .talkers(subject, reach, door)
            .into_iter()
            .map(|(entity, center)| candidate(InteractKind::Talk, center, entity));
        // A switch or a chest is ranked where the body meets it: through an
        // opening, that is the target carried back to the body's own chart.
        let touched = |kind, preference, entity| InteractCandidate {
            kind,
            preference,
            id: (self.objects.reference(entity), entity),
        };
        let switches = self
            .switches
            .iter()
            .filter_map(|(entity, aabb, requires_facing)| {
                reach.touch_preference(rooms.of(entity), aabb, requires_facing)
                    .map(|preference| touched(InteractKind::Activate, preference, entity))
            });
        // A chest has no facing gate: its spec has no `requires_facing`.
        let chests = self
            .chests
            .iter()
            .filter(|(_, _, opened, falling)| opened.is_none() && !falling)
            .filter_map(|(entity, aabb, ..)| {
                reach.touch_preference(rooms.of(entity), aabb, false)
                    .map(|preference| touched(InteractKind::Open, preference, entity))
            });
        let first = interact_order(talkers.chain(switches).chain(chests)).into_iter().next()?;
        Some((first.id.1, first.kind))
    }
}

/// Rebuild [`InteractSelection`] for every driven body, and for the
/// controlled subject (the primary player on the startup frame).
///
/// The precedence, stated here and nowhere else:
/// 1. what the press reaches (a body, a switch, a chest: [`InteractTargets::chosen`]);
/// 2. for a body that rides, the door it stands in, before getting off (the
///    one press would otherwise both leave and bail, and the mount stay
///    behind), and then getting off;
/// 3. for a body on foot, boarding a mount before the door, since a mount
///    crosses with its rider (and the door a body has just come in by does
///    not take the press back), and then the door.
#[allow(clippy::too_many_arguments)]
pub fn select_interactions(
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    driven: Query<Entity, With<ambition_characters::control::DrivingParticipant>>,
    bodies: Query<(
        &ambition_platformer2d_core::BodyKinematics,
        Option<&ambition_platformer2d_core::SweepSample>,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
    targets: InteractTargets,
    // A mount is offered when the mount domain's board law admits the pair,
    // which includes one execution scope: a mount at the same coordinates in
    // another live room is not beside the body.
    mounts: Query<(Entity, &ambition_platformer2d_core::BodyKinematics), With<ambition_mount::Mountable>>,
    board: ambition_mount::BoardFacts,
    rides: ambition_mount::ActiveRides,
    // The live rooms and the doors in them: the press's room and door rules.
    // `Option`: a composition with no room set still selects (no rooms, no
    // doors) rather than skipping the whole system.
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    // The portal openings that cut a body: a switch or a chest is touched by
    // the body's admitted material.
    geometry: Option<Res<ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>>,
    mut out: ResMut<InteractSelection>,
) {
    let selected_for = |body: Entity| -> Option<InteractSelected> {
        let (kin, step, frame) = bodies.get(body).ok()?;
        let room = rooms.as_ref().and_then(|rooms| rooms.live().of(body));
        let reach = InteractReach::of(kin, step, frame, room, geometry.as_deref().and_then(|geometry| geometry.motion_for(body, room)));
        let door = rooms.as_ref().and_then(|rooms| rooms.nearest_door_under(body, reach.collision_box(), reach.pos()));
        let target = targets
            .chosen(body, &reach, door)
            .map(|(entity, kind)| InteractSelected { operation: kind.into(), target: Some(entity) });
        let ride = || {
            let mount = ambition_mount::nearest_commandeerable(body, kin, &board, mounts.iter(), &[])?;
            Some(InteractSelected { operation: InteractOperation::Ride, target: Some(mount) })
        };
        let use_door = || door.map(|_| InteractSelected { operation: InteractOperation::Use, target: None });
        if rides.mount_of(body).is_some() {
            target
                .or_else(use_door)
                .or(Some(InteractSelected { operation: InteractOperation::Dismount, target: None }))
        } else {
            target.or_else(ride).or_else(use_door)
        }
    };
    let subject = controlled.and_then(|subject| subject.0).or_else(|| primary.single().ok());
    let mut selection = std::collections::HashMap::new();
    for body in driven.iter().chain(subject) {
        if let Some(selected) = selected_for(body) {
            selection.insert(body, selected);
        }
    }
    if out.0 != selection {
        out.0 = selection;
    }
}

/// The presses the mount owner acts on this tick, and what each does
/// ([`ambition_mount::MountPresses`]): the bodies selected to ride a mount or
/// to get off one, read off the selection.
pub fn project_mount_presses(selection: Res<InteractSelection>, mut presses: ResMut<ambition_mount::MountPresses>) {
    let mut projected: Vec<(Entity, ambition_mount::MountPress)> = selection
        .0
        .iter()
        .filter_map(|(body, selected)| match (selected.operation, selected.target) {
            (InteractOperation::Ride, Some(mount)) => Some((*body, ambition_mount::MountPress::Board(mount))),
            (InteractOperation::Dismount, _) => Some((*body, ambition_mount::MountPress::Dismount)),
            _ => None,
        })
        .collect();
    projected.sort_by_key(|(body, _)| *body);
    if presses.0 != projected {
        presses.0 = projected;
    }
}
