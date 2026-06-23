//! Nearest-interactable proximity query.
//!
//! Walks every feature entity that can be interacted with (peaceful
//! NPCs, switches, intact chests) and reports the closest one
//! overlapping the player's AABB. The result feeds
//! [`super::resolvers::resolve_interact`] via the [`super::WorldView`]
//! the affordance compute system builds each frame.
//!
//! Uses the same `strict_intersects` test the buffered-interact
//! systems use ([`crate::features::interact_ecs_actors_and_switches`],
//! [`crate::features::open_ecs_chests`]) so the HUD label switches at
//! exactly the moment the corresponding interaction would actually
//! fire — no off-by-one frame where the prompt says "Talk" but the
//! buffered press silently misses.

use crate::engine_core::AabbExt;
use bevy::prelude::*;

use super::variants::InteractVariant;
use crate::features::{
    ActorDisposition, ActorInteraction, CenteredAabb, ChestFeature, FeatureSimEntity, Opened,
    SwitchFeature,
};

/// Resource: the nearest live interactable overlapping the primary
/// player's AABB, classified into an [`InteractVariant`]. Default is
/// [`InteractVariant::None`] (no interactable nearby).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct NearestInteractable(pub InteractVariant);

/// Rebuild [`NearestInteractable`] each frame from the primary
/// player's overlap against peaceful actors, switches, and unopened
/// chests.
///
/// Selection policy: first overlap wins, in a fixed priority order
/// (NPCs → chests → switches). The overlap test is binary today
/// (AABB strict-intersects), matching the existing interact path.
/// When the player overlaps multiple interactables simultaneously, the
/// HUD label still reflects what the buffered-interact systems would
/// fire because both follow the same priority order.
pub fn update_nearest_interactable(
    player: Query<
        &crate::player::BodyKinematics,
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
        ),
    >,
    actors: Query<(&CenteredAabb, &ActorDisposition, &ActorInteraction), With<FeatureSimEntity>>,
    chests: Query<(&CenteredAabb, Option<&Opened>), (With<FeatureSimEntity>, With<ChestFeature>)>,
    switches: Query<&CenteredAabb, (With<FeatureSimEntity>, With<SwitchFeature>)>,
    mut out: ResMut<NearestInteractable>,
) {
    let Ok(kin) = player.single() else {
        if out.0 != InteractVariant::None {
            *out = NearestInteractable(InteractVariant::None);
        }
        return;
    };
    let player_aabb = kin.aabb();

    // Talkable actors first — `Talk` is the most common contextual swap and the
    // one players need feedback on while approaching dialog. A talkable actor
    // carries `ActorInteraction`; a provoked one keeps it but flips to
    // `Hostile`, so the disposition gate drops it out of the prompt.
    let mut chosen = InteractVariant::None;
    for (aabb, disposition, _interaction) in &actors {
        if disposition.is_hostile() {
            continue;
        }
        if aabb.aabb().strict_intersects(player_aabb) {
            chosen = InteractVariant::Talk;
            break;
        }
    }

    if matches!(chosen, InteractVariant::None) {
        for (aabb, opened) in &chests {
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
        for aabb in &switches {
            if aabb.aabb().strict_intersects(player_aabb) {
                chosen = InteractVariant::Activate;
                break;
            }
        }
    }

    if out.0 != chosen {
        *out = NearestInteractable(chosen);
    }
}
