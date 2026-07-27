//! **A staged cast authorizes its own presentation sources.** (§4.5, §4.7)
//!
//! §7.7 gave the engine the vocabulary for several providers emitting cues in one
//! session: a request carries a [`PresentationSourceId`], and
//! `ActiveAudioSelection::authorize_sfx_source` binds that source to a provider's
//! cue registry and bank allowlist. So the same logical cue id — `Dash` — can
//! resolve to two genuinely different sounds for two characters in one fight.
//!
//! Nothing in production called it. The only non-unit-test caller was a rendered
//! test, which means the vocabulary existed and the sentence was never spoken: a
//! secondary provider's correctly-tagged cue reached the audio authority, found no
//! authorization for its source, and was **denied**. A cue that is silently
//! dropped is worse than one that plays the wrong sound, because nothing reports
//! it — the request was well-formed and the refusal was invisible.
//!
//! This module is that sentence. Seating a cast is what authorizes its sources.

use bevy::prelude::*;
use std::collections::BTreeSet;

use ambition_characters::actor::character_catalog::CharacterCatalogOwners;
use ambition_sfx::PresentationSourceId;

use super::{CharacterLoadStates, PreparedCharacterRegistry};

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
/// Idempotent by construction: `authorize_sfx_source` treats a repeat with an
/// identical definition as a no-op (and panics on a repeat with a DIFFERENT one,
/// which is the right behaviour — two disagreeing definitions of one source is a
/// composition bug, not a runtime condition to paper over).
/// Not gated on the `audio` feature: `ambition_audio` is an unconditional
/// dependency (only the Kira playback backend is optional), and gating the
/// AUTHORIZATION would mean a headless or art-free build silently denies cues it
/// would have allowed — a composition difference that changes behaviour, which is
/// the defect class this whole module exists to close.
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
        // NEVER redefine a source the session already established.
        //
        // `select_gameplay` registers the session owner's own provider as a
        // presentation source, with the registry and bank allowlist it had at
        // selection time. A cast member from that same provider must not
        // re-authorize it: `authorize_sfx_source` PANICS on a repeat with a
        // different definition, and the definitions legitimately differ — bank ids
        // load asynchronously, so this system's view is whatever has arrived.
        // Late-loading banks are refreshed per provider by the audio layer, which
        // is the seam that owns that problem.
        if selection.is_sfx_source_authorized(&PresentationSourceId::new(provider)) {
            continue;
        }
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

/// **Publish each body's presentation source, once per SIM tick.**
///
/// The derivation `advance_move_playback` used to do inline, hoisted onto the body
/// so every emitter can attribute a cue without repeating it — and so a cue
/// attributed to the wrong provider is one bug in one place.
///
/// On the sim schedule, immediately before the move clock advances, for the same
/// reason the hurtbox systems are: the move timeline reads this component on the
/// tick it fires a cue. Published once per FRAME it would be a tick stale for
/// every resimulated tick, and absent entirely for a body that is spawned and
/// strikes before the next frame boundary — which is precisely the case a versus
/// match makes ordinary.
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
            Option<&crate::combat::CombatTuning>,
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
            With<crate::combat::CombatTuning>,
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

/// **The BACKSTOP for a projectile that reached the world without a source.**
///
/// The bolt is the emitter: it is the entity that owns the impact and the
/// detonation, and it routinely outlives the body that fired it. So the source is
/// STAMPED at spawn rather than looked up at impact — a shot whose firer has since
/// died still lands in that character's voice, which is the whole reason
/// `ProjectileOwner` being `Option` is not an accident.
///
/// Both materializers now stamp it themselves
/// (`ambition_projectiles::{spawn_systems, enemy::effect_spawn_systems}`), because
/// this system alone was not enough and could not be: it runs before the move clock,
/// while `apply_enemy_projectile_effects` spawns LATER in the same `Combat` set and
/// `step_projectiles` runs immediately after it. An enemy bolt that spawned and hit
/// a wall inside one tick emitted its impact before this could ever see it (GPT 5.6,
/// 2026-07-26). Attribution belongs where the entity is born.
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

/// Which registered character's definition is currently projected onto a body.
///
/// The projection needs REPLACEMENT semantics, not insert-only: a body that wears
/// a new form must lose the previous character's moves, silhouette, and routing
/// markers, and "insert whatever the new definition has" leaves all three behind
/// when the new definition is quieter than the old one (GPT 5.6, 2026-07-27).
/// Knowing which definition is on the body is what makes the removal computable —
/// the same reason [`IdentityKit`](ambition_characters::brain::action_set::IdentityKit)
/// records what identity alone derived, so an equipment grant stays revocable.
///
/// ⚠ It records the id, not the DISPLACED value. So a body whose spawn seeded an
/// archetype moveset, then wore a registered character that authored one, then wore
/// one that authors none, ends with no moveset rather than its archetype's. Fixing
/// that means storing the displaced value here — `IdentityKit`'s exact pattern —
/// and no character needs it yet. Named rather than discovered.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ProjectedCharacterKit(pub String);

/// **C3: a registered character's authored fight reaches a spawned BODY.**
///
/// §4.1's end state is that the prepared registry replaces the old seams; A1 built
/// the DECLARATION half (sheets, cue authorization, presentation source) and left
/// construction reading `CharacterCatalog`. So registering a character with a
/// moveset and a silhouette produced a definition nothing spawned ever consulted —
/// the §7.10 fight test had to project it onto its own bodies by hand, which proved
/// the projection works and not that registration reaches production.
///
/// This is that projection, once, in the engine. It runs on the same identity chain
/// as [`publish_body_presentation_sources`] — the worn character, falling back to
/// the sprite character its combat tuning names — because a body's identity must
/// mean the same thing to every derivation that reads it. In practice that reaches
/// both the player (which carries `WornCharacter`) and every spawned actor (which
/// carries only `CombatTuning`).
///
/// **The registry wins where it authored something**, matching `provider_of_character`
/// and `sheet_for_declared_character`: a character that exists in both authorities
/// is the registry's. Where the definition authored `None` the catalog's value
/// stands untouched, which is the ordinary migration state and not a conflict —
/// `audit_character_authority_parity` reports the case where the two disagree.
///
/// Triggered by change detection rather than an already-projected marker,
/// deliberately:
/// a marker would be one more component to register for rollback, and bevy_ggrs
/// recreating an entity re-inserts its components, so the same edge that fires at
/// spawn fires again after a load. Wearing a different character (a power-up tier,
/// a super form) re-projects for free.
///
/// Vitals are NOT projected. Health is live state, and writing a definition's max
/// HP onto a body mid-fight would heal it on every transformation — a spawn-time
/// concern that belongs to whatever constructs the body, not to a per-tick
/// derivation.
pub fn project_prepared_character_definitions(
    mut commands: Commands,
    registry: Option<Res<PreparedCharacterRegistry>>,
    bodies: Query<
        (
            Entity,
            Option<&ambition_characters::actor::WornCharacter>,
            Option<&crate::combat::CombatTuning>,
            Option<&ProjectedCharacterKit>,
        ),
        Or<(
            Changed<ambition_characters::actor::WornCharacter>,
            Added<crate::combat::CombatTuning>,
        )>,
    >,
) {
    let Some(registry) = registry else {
        return;
    };
    for (entity, worn, tuning, projected) in &bodies {
        let character_id = worn
            .map(ambition_characters::actor::WornCharacter::id)
            .or_else(|| tuning.and_then(|t| t.sprite_character_id.as_deref()));
        let resolved = character_id.filter(|id| registry.get(id).is_some());
        if projected.map(|p| p.0.as_str()) == resolved {
            continue;
        }
        // RETRACT what the previous definition put here, before projecting the new
        // one. Looked up by the recorded id, so the removal is exactly what this
        // system granted and never something the spawn seeded.
        //
        // ⚠ `ActorMoveset` is deliberately NOT retracted, and the first version of
        // this did retract it (GPT 5.6, 2026-07-27). Removing it is worse than
        // leaving a stale value: `apply_worn_character_gameplay` takes
        // `&mut ActorMoveset` as a required query column, so a body without the
        // component stops matching the PERSONA DERIVE ENTIRELY — losing its name,
        // action set and identity kit too, permanently, not just its moves. And
        // the retraction was unnecessary anyway: for a worn body the persona
        // derive is the single writer and replaces the moveset wholesale on the
        // same tick this runs. The routing markers that used to be removed here
        // are derived from the live moveset by
        // `reconcile_moveset_routing_markers`, so they follow whichever writer won.
        if let Some(previous) = projected.and_then(|p| registry.get(&p.0)) {
            if previous.hurtboxes.is_some() {
                commands.entity(entity).remove::<super::AuthoredHurtboxes>();
            }
        }
        let Some(prepared) = resolved.and_then(|id| registry.get(id)) else {
            if projected.is_some() {
                commands.entity(entity).remove::<ProjectedCharacterKit>();
            }
            continue;
        };
        commands
            .entity(entity)
            .insert(ProjectedCharacterKit(prepared.id.clone()));
        if let Some(moveset) = prepared.moveset.clone() {
            // The routing markers are NOT set here. They are derived from the live
            // `ActorMoveset` by `reconcile_moveset_routing_markers` — deriving them
            // is what makes them right for the catalog persona path too, which
            // replaces the moveset and never knew the markers existed.
            commands
                .entity(entity)
                .insert(crate::combat::moveset::ActorMoveset(moveset));
        }
        if let Some(hurtboxes) = prepared.hurtboxes.clone() {
            commands.entity(entity).insert((
                super::AuthoredHurtboxes(hurtboxes),
                super::ResolvedHurtboxes::default(),
                crate::combat::components::DamageableVolumes::default(),
            ));
        }
    }
}

#[cfg(test)]
mod tests;
