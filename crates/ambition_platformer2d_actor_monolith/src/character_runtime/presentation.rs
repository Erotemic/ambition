//! Authorize presentation sources for a staged cast.
//!
//! Each provider source is bound to its cue registry and bank allowlist so the
//! same logical cue id can resolve differently for different characters in one
//! session. Seating the cast establishes those authorizations.

use ambition_characters::prepared::PreparedCharacterRegistry;
use bevy::prelude::*;
use std::collections::BTreeSet;

use ambition_characters::actor::character_catalog::CharacterCatalogOwners;

use super::CharacterLoadStates;
use ambition_platformer2d_actor_spawn::character_body::{
    grant_prepared_character_body, KitOwnership, ProjectedCharacterKit,
};

/// The provider that authored a character, from either declaration source.
///
/// The prepared registry is consulted FIRST and the assembled catalog second, for
/// the same reason `sheet_for_declared_character` prefers the registered sheet: a
/// character may exist only on the registration seam, and then the registry is the
/// only place its provider is written down.
pub fn provider_of_character<'a>(
    registry: Option<&'a PreparedCharacterRegistry>,
    owners: Option<&'a CharacterCatalogOwners>,
    character_id: &str,
) -> Option<&'a str> {
    registry
        .and_then(|registry| registry.get(character_id))
        .map(|prepared| prepared.provider.as_str())
        .or_else(|| owners.and_then(|owners| owners.provider_for(character_id)))
}

/// Authorize a presentation source for every provider in the staged cast.
///
/// Runs off [`CharacterLoadStates::cast`] — the characters THIS session staged, by
/// canonical id — rather than off everything registered, because authorization is a
/// property of one session's cast. A game with fifty registered fighters and two on
/// stage authorizes two providers.
///
/// It deliberately does not read the load ledger's token history. That map is
/// append-only and keyed by demand spelling, which authorized every character the
/// process had ever loaded and failed to authorize any room that staged a display
/// name. [`StagedCast`](super::StagedCast) exists because those are two different
/// facts.
///
/// The source id is the provider id, which is exactly what `advance_move_playback`
/// stamps onto a `MoveEventMessage` when it resolves a body's character to its
/// author. Those two must agree or the tag is unauthorized; deriving both from
/// `provider_of_character` is what keeps them agreeing.
///
/// Idempotent by construction: for the SAME provider, `authorize_sfx_source` merges by union, so
/// running every tick can only add cues as banks arrive. Two DIFFERENT providers claiming one
/// source is a content conflict and is recorded on the selection (`sfx_source_conflicts`) rather
/// than merged or fatal.
pub fn authorize_staged_character_presentation_sources(
    states: Option<Res<CharacterLoadStates>>,
    registry: Option<Res<PreparedCharacterRegistry>>,
    owners: Option<Res<CharacterCatalogOwners>>,
    audio_catalog: Option<Res<ambition_audio::catalog::AudioCatalogRegistry>>,
    bank_ids: Option<Res<ambition_audio::catalog::SfxBankRegistry>>,
    selection: Option<ResMut<ambition_audio::selection::ActiveAudioSelection>>,
) {
    let (Some(states), Some(mut selection)) = (states, selection) else {
        return;
    };
    // No session owns the speakers yet (a frontend route, or startup before the
    // gameplay session commits). Authorizing into nothing is a silent no-op inside
    // `authorize_sfx_source` anyway; returning here says so out loud.
    if selection.current().is_none() {
        return;
    }
    let mut authorized: BTreeSet<String> = BTreeSet::new();
    for character_id in states.cast().ids() {
        let Some(provider) =
            provider_of_character(registry.as_deref(), owners.as_deref(), character_id)
        else {
            // No declaration names an author. The load ledger already reports
            // unknown characters; this is not a second place to complain about it.
            continue;
        };
        if !authorized.insert(provider.to_string()) {
            continue;
        }
        // Re-authorizing the session owner's own source is FINE now, and used to
        // be a crash.
        //
        // `select_gameplay` registers that provider as a presentation source with the registry and
        // bank allowlist it had at selection time; this system's view is whatever has arrived
        // since, because bank ids load asynchronously. It now merges by union for the same
        // provider, so re-authorizing can only add cues, never remove them, and the outcome does
        // not depend on which view came first (A15).
        let sfx = audio_catalog
            .as_deref()
            .and_then(|catalog| catalog.sfx_for(provider))
            .cloned();
        let ids = bank_ids
            .as_deref()
            .map(|banks| banks.ids_for(provider))
            .unwrap_or_default();
        selection.authorize_sfx_source(provider.to_string(), provider.to_string(), sfx, ids);
    }
}

/// Publish each body's presentation source, once per SIM tick.
///
/// Published once per FRAME it would be a tick stale for every resimulated tick, and absent
/// entirely for a body that is spawned and strikes before the next frame boundary — which is
/// precisely the case a versus match makes ordinary.
///
/// A body's source is its WORN character's author, falling back to the sprite
/// character its combat tuning names. A body wearing nothing gets no component at
/// all rather than an empty source: absent means "ask the session", which is the
/// honest answer for a hazard or an unworn body, and is materially different from
/// "this body belongs to nobody".
pub fn publish_body_presentation_sources(
    mut commands: Commands,
    registry: Option<Res<PreparedCharacterRegistry>>,
    owners: Option<Res<CharacterCatalogOwners>>,
    bodies: Query<
        (
            Entity,
            Option<&ambition_characters::actor::WornCharacter>,
            Option<&ambition_combat::CombatTuning>,
            Option<&ambition_sfx::BodyPresentationSource>,
        ),
        // Filtered, because this runs on the SIM clock and an all-`Option` tuple
        // matches EVERY entity in the world — every resimulated tick. The three
        // filters are exactly the components the body arms below read: the first
        // two are the identity sources, and the third keeps an entity matched
        // long enough for the removal arm to see it lose its identity.
        //
        // The third is the DERIVED marker rather than the source itself, because
        // this system is not the only thing that stamps a source: a projectile
        // inherits its firer's, and matching on the source alone made the removal
        // arm delete exactly those inherited ones on the very next tick.
        Or<(
            With<ambition_characters::actor::WornCharacter>,
            With<ambition_combat::CombatTuning>,
            With<ambition_sfx::DerivedPresentationSource>,
        )>,
    >,
) {
    for (entity, worn, tuning, current) in &bodies {
        let character_id = worn
            .map(ambition_characters::actor::WornCharacter::id)
            .or_else(|| tuning.and_then(|t| t.sprite_character_id.as_deref()));
        let provider = character_id
            .and_then(|id| provider_of_character(registry.as_deref(), owners.as_deref(), id));
        match provider {
            Some(provider) => {
                let next = ambition_sfx::PresentationSourceId::new(provider);
                // Change detection: a body's author is stable for the whole session
                // in every ordinary case, and this runs over every body every tick.
                if current.map(|c| c.id()) != Some(&next) {
                    commands.entity(entity).insert((
                        ambition_sfx::BodyPresentationSource(next),
                        ambition_sfx::DerivedPresentationSource,
                    ));
                }
            }
            None if current.is_some() => {
                commands
                    .entity(entity)
                    .remove::<ambition_sfx::BodyPresentationSource>()
                    .remove::<ambition_sfx::DerivedPresentationSource>();
            }
            None => {}
        }
    }
}

/// The BACKSTOP for a projectile that reached the world without a source.
///
/// The bolt is the emitter: it is the entity that owns the impact and the
/// detonation, and it routinely outlives the body that fired it. So the source is
/// STAMPED at spawn rather than looked up at impact — a shot whose firer has since
/// died still lands in that character's voice, which is the whole reason
/// `ProjectileOwner` being `Option` is not an accident.
///
/// Attribution belongs where the entity is born.
///
/// This remains as the backstop for any other path that stamps `ProjectileOwner`
/// without a source — the reflect re-own does exactly that — and for a firer whose
/// own source is published after its first shot. `Without<BodyPresentationSource>`
/// means it can only ever fill a gap, never overwrite an answer.
///
/// `Without<BodyPresentationSource>` rather than `Added<ProjectileOwner>`: bevy_ggrs
/// destroys and recreates rollback entities, so an `Added` filter fires again on
/// every restored frame while the change-detection tick the filter reads is not the
/// sim's. Filtering on the absence of the component is idempotent under any number
/// of loads, and the snapshot restores the stamp for a projectile whose firer is
/// gone by the time it comes back.
///
/// A firer with no source of its own leaves the bolt unstamped, which falls back to
/// the session context — correct for an environmental hazard's shot, and identical
/// to what every projectile did before this existed.
pub fn inherit_projectile_presentation_sources(
    mut commands: Commands,
    unstamped: Query<
        (Entity, &ambition_projectiles::ProjectileOwner),
        Without<ambition_sfx::BodyPresentationSource>,
    >,
    sources: Query<&ambition_sfx::BodyPresentationSource>,
) {
    for (projectile, owner) in &unstamped {
        if let Ok(source) = sources.get(owner.0) {
            commands.entity(projectile).insert(source.clone());
        }
    }
}


/// Project prepared character-authored combat/presentation facts onto bodies.
///
/// Identity follows worn character first, then combat tuning. Prepared registry
/// values override older catalog-derived facts where authored. Change detection
/// makes the projection replay safely after rollback recreation and character
/// replacement. Live vitals are intentionally not projected here.
pub fn project_prepared_character_definitions(
    mut commands: Commands,
    registry: Option<Res<PreparedCharacterRegistry>>,
    changed_bodies: Query<
        (
            Entity,
            Option<&ambition_characters::actor::WornCharacter>,
            Option<&ambition_combat::CombatTuning>,
            Option<&ProjectedCharacterKit>,
        ),
        Or<(
            Changed<ambition_characters::actor::WornCharacter>,
            Added<ambition_combat::CombatTuning>,
        )>,
    >,
    // The bodies a CAST REPLACEMENT invalidates. (H6)
    //
    // A new cast changes nothing on a body, so the change-detection query above
    // cannot see one — the worn id is still the worn id. That is exactly how a
    // body kept a retired cast's moves with every check green. This query is
    // The bodies `apply_worn_character_gameplay` can actually see.
    //
    // Its required columns, spelled out, because "does the derive match this
    // entity" has no shorter honest form. `WornCharacter` looks like the answer
    // and is not — it REQUIRES `IdentityKit`, so gating on either one silently
    // means "does it wear a character", which is a different and wrong question:
    // a hand-assembled body wearing a character without a moveset column matches
    // neither writer and would get no kit at all while reading as covered.
    //
    // this list is coupled to that system's query.
    persona_bodies: Query<
        Entity,
        (
            With<ambition_characters::actor::WornCharacter>,
            With<Name>,
            With<ambition_characters::brain::ActionSet>,
            With<ambition_combat::moveset::ActorMoveset>,
            With<ambition_characters::brain::action_set::IdentityKit>,
            With<ambition_platformer2d_core::BodyAbilities>,
            With<ambition_platformer2d_core::movement::MotionModel>,
        ),
    >,
    // walked ONLY on the tick the registry changes, which is startup and hot
    // reload; the ordinary tick still pays only for identities that moved.
    all_bodies: Query<(
        Entity,
        Option<&ambition_characters::actor::WornCharacter>,
        Option<&ambition_combat::CombatTuning>,
        Option<&ProjectedCharacterKit>,
    )>,
) {
    let Some(registry) = registry else {
        return;
    };
    let candidates: Vec<_> = if registry.is_changed() {
        all_bodies.iter().collect()
    } else {
        changed_bodies.iter().collect()
    };
    for (entity, worn, tuning, projected) in candidates {
        let character_id = worn
            .map(ambition_characters::actor::WornCharacter::id)
            .or_else(|| tuning.and_then(|t| t.sprite_character_id.as_deref()));
        let resolved = character_id.filter(|id| registry.get(id).is_some());
        let unchanged = projected.is_some_and(|projected| {
            Some(projected.id.as_str()) == resolved && projected.generation == registry.generation()
        });
        if unchanged || (projected.is_none() && resolved.is_none()) {
            continue;
        }
        // Looked up by the recorded id, so the removal is exactly what this system granted and
        // never something the spawn seeded.
        //
        // Do not retract `ActorMoveset`: `apply_worn_character_gameplay` requires
        // the component as a query column and replaces its value wholesale for worn bodies.
        // the same tick this runs.
        if let Some(previous) = projected {
            previous.granted.retract(entity, &mut commands);
        }
        let Some(prepared) = resolved.and_then(|id| registry.get(id)) else {
            if projected.is_some() {
                commands.entity(entity).remove::<ProjectedCharacterKit>();
            }
            continue;
        };
        // this marker records what THIS system granted, and nothing else.
        //
        // The body recorded that it was up to date and no later pass revisited it, which is worse
        // than a missed update.
        //
        // That writer keeps its own record now, `avatar::PersonaBaseline`, stamped
        // after IT applies the baseline. One writer, one record. This one covers
        // the authored silhouette, the movement feel and the motion model below.
        grant_prepared_character_body(
            &mut commands,
            entity,
            prepared,
            registry.generation(),
            if persona_bodies.contains(entity) {
                KitOwnership::PersonaDerive
            } else {
                KitOwnership::Grant
            },
            // This body answers to no match, so the character's own feel is
            // the whole answer. A SEAT resolves the same question against its
            // match's rules and hands the result in — see
            // `MatchRules::body_over`, which is the only place the two are
            // weighed against each other.
            prepared.movement_tuning,
        );
    }
}


#[cfg(test)]
mod tests;
