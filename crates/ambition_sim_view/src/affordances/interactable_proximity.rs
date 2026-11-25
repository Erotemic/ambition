//! The Interact prompt's label, per driven body.
//!
//! The selection is gameplay's (`ambition_platformer2d_actor::body::select_interactions`,
//! interaction I1): what each body's press is for, from the domains' own
//! availability, rebuilt with the gameplay that executes the press. This
//! module only names it ([`InteractVariant`]), so a composition without the
//! prompt selects the same operation, and the label says "Talk" only where
//! the press talks.

use bevy::prelude::*;

use super::variants::InteractVariant;
use ambition_combat::{ActorDisposition, ActorInteraction};
use ambition_encounter::switches::SwitchFeature;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_world_items::containers::ChestFeature;
use ambition_world_items::containers::Opened;

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

/// The bodies the reach observer explains (`interaction_reach_observer`):
/// the population the press talks to, with what [`talk_refusal`] reads.
///
/// [`talk_refusal`]: ambition_combat::util::talk_refusal
pub(crate) type Talkers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CenteredAabb,
        &'static ActorDisposition,
        Option<&'static ambition_combat::components::ActorAggression>,
        Option<&'static ambition_characters::actor::BodyHealth>,
        // The world's hands are off this body: no prompt from it either.
        bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        Option<&'static ambition_platformer2d_core::DepthPlane>,
        bevy::prelude::Has<ambition_combat::components::RequiresFacing>,
    ),
    (With<FeatureSimEntity>, With<ActorInteraction>, With<ambition_combat::components::ActorIdentity>),
>;

/// The chests the reach observer explains.
pub(crate) type Chests<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CenteredAabb,
        Option<&'static Opened>,
        bevy::prelude::Has<ambition_world_items::containers::FallingChest>,
    ),
    (With<FeatureSimEntity>, With<ChestFeature>),
>;

/// The switches the reach observer explains.
pub(crate) type Switches<'w, 's> = Query<
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
pub(crate) type Reaching = (
    &'static ambition_platformer2d_core::BodyKinematics,
    Option<&'static ambition_platformer2d_core::SweepSample>,
    Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
);

/// Rebuild [`NearestInteractable`] from gameplay's selection
/// ([`InteractSelection`]): the label of what each driven body's press is
/// for, and seat zero's, the one on the screen. A mount is named by its
/// identity.
///
/// [`InteractSelection`]: ambition_platformer2d_interaction::InteractSelection
pub fn update_nearest_interactable(
    controlled: Option<Res<ControlledSubject>>,
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    driven: Query<Entity, With<ambition_characters::control::DrivingParticipant>>,
    selection: Option<Res<ambition_platformer2d_interaction::InteractSelection>>,
    names: Query<&ambition_combat::components::ActorIdentity>,
    mut out: ResMut<NearestInteractable>,
) {
    use ambition_platformer2d_interaction::InteractOperation;
    let variant_of = |body: Entity| {
        let Some(selected) = selection.as_deref().and_then(|selection| selection.of(body)) else {
            return InteractVariant::None;
        };
        match selected.operation {
            InteractOperation::Talk => InteractVariant::Talk,
            InteractOperation::Activate => InteractVariant::Activate,
            InteractOperation::Open => InteractVariant::Open,
            InteractOperation::Use => InteractVariant::Use,
            InteractOperation::Dismount => InteractVariant::Dismount,
            InteractOperation::Ride => {
                let name = selected.target.and_then(|mount| names.get(mount).ok()).map_or("mount", |identity| identity.name());
                InteractVariant::Ride(name.to_string().into())
            }
        }
    };
    // ⭐⭐ EVERY DRIVEN BODY, not just the one holding the primary seat. The
    // label on screen is still seat zero's, but a consumer deciding something
    // about a PARTICULAR body needs that body's answer — see the type's doc.
    let mut by_body: std::collections::HashMap<Entity, InteractVariant> =
        driven.iter().map(|body| (body, variant_of(body))).collect();
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    let chosen = match subject {
        Some(subject) => {
            let variant = variant_of(subject);
            by_body.insert(subject, variant.clone());
            variant
        }
        None => InteractVariant::None,
    };
    if out.0 != chosen || out.1 != by_body {
        *out = NearestInteractable(chosen, by_body);
    }
}

#[cfg(test)]
mod tests;
