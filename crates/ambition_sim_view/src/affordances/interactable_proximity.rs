//! Nearest-interactable proximity query.
//!
//! Walks every feature entity that can be interacted with (peaceful
//! NPCs, switches, intact chests) and reports the closest one
//! overlapping the player's AABB. The result feeds
//! [`super::resolvers::resolve_interact`] via the [`super::WorldView`]
//! the affordance compute system builds each frame.
//!
//! It asks the rule the press asks
//! ([`ambition_platformer2d_actor_monolith::features::InteractReach`]): the
//! same reach, live room, facing gate and door rule as
//! [`ambition_platformer2d_actor_monolith::features::interact_ecs_actors_and_switches`]
//! and [`ambition_platformer2d_actor_monolith::features::open_ecs_chests`]. So
//! the label says "Talk" only where the press talks.

use bevy::prelude::*;

use super::variants::InteractVariant;
use ambition_combat::{ActorDisposition, ActorInteraction, ChestFeature, Opened};
use ambition_encounter::switches::SwitchFeature;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_actor_monolith::features::InteractReach;

/// Resource: the nearest live interactable overlapping the controlled
/// subject's AABB, classified into an [`InteractVariant`]. Default is
/// [`InteractVariant::None`] (no interactable nearby).
///
/// ⛔⛔ THE TUPLE FIELD IS SEAT ZERO'S ANSWER AND ONLY SEAT ZERO'S. It is what
/// the HUD label needs — there is one prompt on the screen — and it was ALSO
/// being read as a gameplay decision for every seat (`portal/input_adapter`
/// asked it whether an ordinary interaction had claimed THIS body's press). With
/// two people playing, seat zero standing near a chest suppressed seat one's
/// portal toggle, and seat zero standing clear let seat one both toggle and
/// interact. Per-body answers live in [`Self::by_body`].
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct NearestInteractable(
    pub InteractVariant,
    /// One answer per DRIVEN body, for consumers deciding something about a
    /// particular seat rather than drawing one label.
    pub std::collections::HashMap<Entity, InteractVariant>,
);

impl NearestInteractable {
    /// What is in reach of ONE body.
    ///
    /// ⭐ ASK THIS, NOT `.0`, from anything keyed to a body. `.0` is the screen's
    /// single prompt; a body nobody drives, or one that arrived after the last
    /// rebuild, is `None` here rather than borrowing seat zero's answer.
    pub fn for_body(&self, body: Entity) -> InteractVariant {
        self.1.get(&body).cloned().unwrap_or(InteractVariant::None)
    }
}

/// The peaceful bodies a prompt can name Talk for.
type Talkers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CenteredAabb,
        &'static ActorDisposition,
        &'static ActorInteraction,
        Option<&'static ambition_characters::actor::BodyHealth>,
        // The world's hands are off this body: no prompt from it either.
        bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        Option<&'static ambition_platformer2d_core::DepthPlane>,
        bevy::prelude::Has<ambition_combat::components::RequiresFacing>,
    ),
    With<FeatureSimEntity>,
>;

/// The chests a prompt can name Open for.
type Chests<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CenteredAabb,
        Option<&'static Opened>,
        bevy::prelude::Has<ambition_combat::components::FallingChest>,
    ),
    (With<FeatureSimEntity>, With<ChestFeature>),
>;

/// The switches a prompt can name Activate for.
type Switches<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CenteredAabb,
        bevy::prelude::Has<ambition_combat::components::RequiresFacing>,
    ),
    (With<FeatureSimEntity>, With<SwitchFeature>),
>;

/// A body's kinematics, last step and frame: what its reach is built from.
type Reaching = (
    &'static ambition_platformer2d_core::BodyKinematics,
    Option<&'static ambition_platformer2d_core::SweepSample>,
    Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
);

/// Rebuild [`NearestInteractable`] each frame from what each driven body's
/// press reaches.
///
/// The prompt follows the body the player is DRIVING (the home avatar, or a
/// possessed actor), matching [`ambition_platformer2d_actor_monolith::features::interact_ecs_actors_and_switches`],
/// which resolves the interaction against the same controlled subject — so the
/// "Talk / Open / Activate" label appears exactly where the interact would fire.
pub fn update_nearest_interactable(
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<Reaching>,
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    actors: Talkers,
    chests: Chests,
    switches: Switches,
    driven: Query<Entity, With<ambition_characters::control::DrivingParticipant>>,
    // The live rooms, and the doors in them: the press's room and door rules.
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut out: ResMut<NearestInteractable>,
) {
    let variant_of = |body: Entity| {
        bodies.get(body).ok().map(|(kin, step, frame)| {
            let reach = InteractReach::of(kin, step, frame, rooms.live().of(body));
            let door = rooms.nearest_door_under(body, reach.collision_box(), reach.pos());
            variant_in_reach(&reach, door, rooms.live(), &actors, &chests, &switches)
        })
    };
    // ⭐⭐ EVERY DRIVEN BODY, not just the one holding the primary seat. The
    // label on screen is still seat zero's, but a consumer deciding something
    // about a PARTICULAR body needs that body's answer — see the type's doc.
    let mut by_body: std::collections::HashMap<Entity, InteractVariant> =
        std::collections::HashMap::new();
    for body in &driven {
        if let Some(variant) = variant_of(body) {
            by_body.insert(body, variant);
        }
    }

    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    // The primary body may not be a driving participant in a bare fixture, so
    // its own answer is computed here rather than assumed to be in the map.
    let chosen = match subject.and_then(|subject| variant_of(subject).map(|variant| (subject, variant))) {
        Some((subject, variant)) => {
            by_body.insert(subject, variant.clone());
            variant
        }
        None => InteractVariant::None,
    };
    if out.0 != chosen || out.1 != by_body {
        *out = NearestInteractable(chosen, by_body);
    }
}

/// What ONE body's press reaches, in the priority order the buffered
/// interact systems fire in. `door` is the distance to the door the body
/// stands in, if any: a door the press is for takes it from a body to talk
/// to ([`InteractReach::a_door_keeps_the_press`]), and the prompt then names
/// what else the press reaches.
///
/// ⭐ EXTRACTED SO EVERY BODY GETS THE SAME ANSWER. Inlining it per caller is how
/// a second seat ends up asking a slightly different question from the first.
fn variant_in_reach(
    reach: &InteractReach,
    door: Option<f32>,
    rooms: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    actors: &Talkers,
    chests: &Chests,
    switches: &Switches,
) -> InteractVariant {
    // Talkable actors first — `Talk` is the most common contextual swap and the
    // one players need feedback on while approaching dialog. A talkable actor
    // carries `ActorInteraction`; a provoked one keeps it but flips to
    // `Hostile`, so the disposition gate drops it out of the prompt.
    for (entity, aabb, disposition, _interaction, health, out_of_play, plane, requires_facing) in actors {
        // A hostile actor drops out of the Talk prompt; a dead one is an
        // intangible corpse and offers no prompt.
        if disposition.is_hostile()
            || ambition_combat::util::body_is_untouchable(health, out_of_play, plane)
        {
            continue;
        }
        if reach.can_talk_to(rooms.of(entity), aabb, requires_facing)
            && !InteractReach::a_door_keeps_the_press(door, aabb.center.distance(reach.pos()))
        {
            return InteractVariant::Talk;
        }
    }
    for (entity, aabb, opened, falling) in chests {
        // A chest has no facing gate: its spec has no `requires_facing`.
        if opened.is_none() && !falling && reach.touches(rooms.of(entity), aabb, false) {
            return InteractVariant::Open;
        }
    }
    for (entity, aabb, requires_facing) in switches {
        if reach.touches(rooms.of(entity), aabb, requires_facing) {
            return InteractVariant::Activate;
        }
    }
    InteractVariant::None
}

#[cfg(test)]
mod tests;
