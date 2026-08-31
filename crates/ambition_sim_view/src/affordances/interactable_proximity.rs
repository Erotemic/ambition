//! Nearest-interactable proximity query.
//!
//! Walks every feature entity that can be interacted with (peaceful
//! NPCs, switches, intact chests) and reports the closest one
//! overlapping the player's AABB. The result feeds
//! [`super::resolvers::resolve_interact`] via the [`super::WorldView`]
//! the affordance compute system builds each frame.
//!
//! Uses the same `strict_intersects` test the buffered-interact
//! systems use ([`ambition_platformer2d_actor_monolith::features::interact_ecs_actors_and_switches`],
//! [`ambition_platformer2d_actor_monolith::features::open_ecs_chests`]) so the HUD label switches at
//! exactly the moment the corresponding interaction would actually
//! fire — no off-by-one frame where the prompt says "Talk" but the
//! buffered press silently misses.

use ambition_platformer2d_core::AabbExt;
use bevy::prelude::*;

use super::variants::InteractVariant;
use ambition_combat::{ActorDisposition, ActorInteraction, ChestFeature, Opened};
use ambition_encounter::switches::SwitchFeature;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

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

/// Rebuild [`NearestInteractable`] each frame from the controlled
/// subject's overlap against peaceful actors, switches, and unopened
/// chests.
///
/// The prompt follows the body the player is DRIVING (the home avatar, or a
/// possessed actor), matching [`ambition_platformer2d_actor_monolith::features::interact_ecs_actors_and_switches`],
/// which resolves the interaction against the same controlled subject — so the
/// "Talk / Open / Activate" label appears exactly where the interact would fire.
///
/// The overlap test is binary today (AABB strict-intersects), matching the existing interact
/// path. When the body overlaps multiple interactables simultaneously, the HUD label still
/// reflects what the buffered-interact systems would fire because both follow the same priority
/// order.
pub fn update_nearest_interactable(
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<&ambition_platformer2d_core::BodyKinematics>,
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    actors: Query<
        (
            &CenteredAabb,
            &ActorDisposition,
            &ActorInteraction,
            Option<&ambition_characters::actor::BodyHealth>,
            // The world's hands are off this body — no prompt from it either.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        With<FeatureSimEntity>,
    >,
    chests: Query<(&CenteredAabb, Option<&Opened>), (With<FeatureSimEntity>, With<ChestFeature>)>,
    switches: Query<&CenteredAabb, (With<FeatureSimEntity>, With<SwitchFeature>)>,
    driven: Query<
        (Entity, &ambition_platformer2d_core::BodyKinematics),
        With<ambition_characters::control::DrivingParticipant>,
    >,
    mut out: ResMut<NearestInteractable>,
) {
    // ⭐⭐ EVERY DRIVEN BODY, not just the one holding the primary seat. The
    // label on screen is still seat zero's, but a consumer deciding something
    // about a PARTICULAR body needs that body's answer — see the type's doc.
    let mut by_body: std::collections::HashMap<Entity, InteractVariant> =
        std::collections::HashMap::new();
    for (body, kin) in &driven {
        by_body.insert(
            body,
            variant_in_reach(kin.aabb(), &actors, &chests, &switches),
        );
    }

    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    // The primary body may not be a driving participant in a bare fixture, so
    // its own answer is computed here rather than assumed to be in the map.
    let chosen =
        match subject.and_then(|subject| bodies.get(subject).ok().map(|kin| (subject, kin))) {
            Some((subject, kin)) => {
                let variant = variant_in_reach(kin.aabb(), &actors, &chests, &switches);
                by_body.insert(subject, variant.clone());
                variant
            }
            None => InteractVariant::None,
        };
    if out.0 != chosen || out.1 != by_body {
        *out = NearestInteractable(chosen, by_body);
    }
}

/// What ONE body's reach box overlaps, in the priority order the buffered
/// interact systems fire in.
///
/// ⭐ EXTRACTED SO EVERY BODY GETS THE SAME ANSWER. Inlining it per caller is how
/// a second seat ends up asking a slightly different question from the first.
fn variant_in_reach(
    reach: ambition_platformer2d_core::Aabb,
    actors: &Query<
        (
            &CenteredAabb,
            &ActorDisposition,
            &ActorInteraction,
            Option<&ambition_characters::actor::BodyHealth>,
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        With<FeatureSimEntity>,
    >,
    chests: &Query<(&CenteredAabb, Option<&Opened>), (With<FeatureSimEntity>, With<ChestFeature>)>,
    switches: &Query<&CenteredAabb, (With<FeatureSimEntity>, With<SwitchFeature>)>,
) -> InteractVariant {
    let player_aabb = reach;

    // Talkable actors first — `Talk` is the most common contextual swap and the
    // one players need feedback on while approaching dialog. A talkable actor
    // carries `ActorInteraction`; a provoked one keeps it but flips to
    // `Hostile`, so the disposition gate drops it out of the prompt.
    let mut chosen = InteractVariant::None;
    for (aabb, disposition, _interaction, health, out_of_play) in actors {
        // A hostile actor drops out of the Talk prompt; a dead one is an
        // intangible corpse and offers no prompt.
        if disposition.is_hostile()
            || ambition_combat::util::body_is_untouchable(health, out_of_play)
        {
            continue;
        }
        if aabb.aabb().strict_intersects(player_aabb) {
            chosen = InteractVariant::Talk;
            break;
        }
    }

    if matches!(chosen, InteractVariant::None) {
        for (aabb, opened) in chests {
            if opened.is_some() {
                continue;
            }
            if aabb.aabb().strict_intersects(player_aabb) {
                chosen = InteractVariant::Open;
                break;
            }
        }
    }

    if matches!(chosen, InteractVariant::None) {
        for aabb in switches {
            if aabb.aabb().strict_intersects(player_aabb) {
                chosen = InteractVariant::Activate;
                break;
            }
        }
    }

    chosen
}
