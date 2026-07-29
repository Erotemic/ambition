//! **A staged cast authorizes its own presentation sources.** (§4.5, §4.7)
//!
//! §7.7 gave the engine the vocabulary for several providers emitting cues in one
//! session: a request carries a [`PresentationSourceId`](ambition_sfx::PresentationSourceId), and
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
/// Idempotent by construction: for the SAME provider, `authorize_sfx_source`
/// merges by union, so running every tick can only add cues as banks arrive. Two
/// DIFFERENT providers claiming one source is a content conflict and is recorded
/// on the selection (`sfx_source_conflicts`) rather than merged or fatal.
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
        // Re-authorizing the session owner's own source is FINE now, and used to
        // be a crash.
        //
        // `select_gameplay` registers that provider as a presentation source with
        // the registry and bank allowlist it had at selection time; this system's
        // view is whatever has arrived since, because bank ids load
        // asynchronously. Both views are honest about a different instant.
        // `authorize_sfx_source` used to PANIC on any difference, so this had to
        // skip re-authorizing entirely — which meant a source authorized BEFORE
        // its bank landed was never refreshed by this path at all. It now merges
        // by union for the same provider, so re-authorizing can only add cues,
        // never remove them, and the outcome does not depend on which view came
        // first (A15).
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
///
/// # Why the generation is here (H6, 2026-07-29)
///
/// `CharacterCatalogGeneration` existed for a day with no production reader: X4
/// was marked done on the strength of a counter nothing compared against. This
/// component recorded only the id, and the projection early-exits when the id is
/// unchanged — so replacing the CAST underneath a body left it wearing the
/// previous cast's kit, with every check green, because the id it wore was still
/// the id it wore. The most expensive kind of stale: correct-looking.
///
/// Stamping the generation makes "this body was built from a cast that no longer
/// exists" a question the code can ASK, rather than one it answers by comparing
/// values and guessing.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ProjectedCharacterKit {
    pub id: String,
    /// The cast this body's projected kit was derived from.
    pub generation: super::definition::CharacterCatalogGeneration,
    /// **What was actually GRANTED, recorded rather than re-derived.**
    ///
    /// Retraction used to look the previous id up in the CURRENT registry and
    /// remove whatever that definition happened to carry. That cannot work, and
    /// fails in the two cases most worth handling (GPT 5.6, 2026-07-29):
    ///
    /// * the same id becomes UNAUTHORED in a new cast — the lookup returns the
    ///   new definition, whose fields are `None`, so the old components are left
    ///   standing and the body keeps a retired hurtbox document forever;
    /// * the character LEAVES the cast — the lookup returns nothing at all, and
    ///   nothing is retracted.
    ///
    /// Historical ownership is not a property of the new authority. So it is
    /// written down at grant time.
    pub granted: GrantedBodyFacts,
}

/// **What the projection put on a body**, so it can take exactly that back.
///
/// One record with one producer ([`Self::of`]) and one consumer
/// ([`Self::retract`]), because the three facts used to be three loose booleans
/// with a grant expression, a retract branch and a field declaration each —
/// three places to edit per fact, and forgetting one is silent (the body keeps a
/// retired hurtbox document, or loses a live one).
///
/// ⚠ `retract` DESTRUCTURES this struct, so adding a fact here is a COMPILE ERROR
/// until it is retracted too. That is the whole reason it is a struct rather than
/// three fields: the coupling is real, so it should be enforced rather than
/// remembered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrantedBodyFacts {
    pub hurtboxes: bool,
    pub movement_tuning: bool,
    pub posed_body: bool,
}

impl GrantedBodyFacts {
    /// What projecting `prepared` onto a body WILL grant.
    fn of(prepared: &super::PreparedCharacterDefinition) -> Self {
        Self {
            hurtboxes: prepared.hurtboxes.is_some(),
            movement_tuning: prepared.movement_tuning.is_some(),
            posed_body: posed_body_for(prepared).is_some(),
        }
    }

    /// Take back exactly what was granted, and nothing else.
    ///
    /// Removing only what THIS system granted is what keeps it from fighting the
    /// worn path, which owns the movement-feel marker for a body whose feel came
    /// from the CATALOG — a case this system cannot see and must not overwrite.
    fn retract(self, entity: Entity, commands: &mut Commands) {
        // Exhaustive on purpose: a new fact does not compile until it is handled.
        let Self {
            hurtboxes,
            movement_tuning,
            posed_body,
        } = self;
        if hurtboxes {
            commands.entity(entity).remove::<super::AuthoredHurtboxes>();
        }
        if movement_tuning {
            commands
                .entity(entity)
                .remove::<ambition_engine_core::AuthoredMovementTuning>();
        }
        if posed_body {
            commands
                .entity(entity)
                .remove::<crate::character_sprites::SpritePosedBody>();
        }
    }
}

/// The sprite-posed body a definition's authored `body` asks for, if any.
///
/// ⚠ only `BodySource::SpriteAuthored` resolves here. `Explicit { half_extents }`
/// is a SPAWN-TIME size and belongs to whoever constructs the body — writing a
/// live body's box from a per-tick projection would be a second geometry
/// authority beside the transit seam (ADR 0024), which is the exact shape of bug
/// this module keeps finding.
///
/// Returns `None` when the character authored no sheet: the posed body reads its
/// rectangles off a sheet manifest, so one without a target has nothing to pose
/// against. Preparation already RESOLVES that target, so a typo is named at load
/// rather than producing a body that silently never poses.
fn posed_body_for(
    prepared: &super::PreparedCharacterDefinition,
) -> Option<crate::character_sprites::SpritePosedBody> {
    match prepared.body.as_ref()? {
        super::BodySource::SpriteAuthored { world_per_pixel } => {
            Some(crate::character_sprites::SpritePosedBody::new(
                prepared.sheet.as_deref()?,
                *world_per_pixel,
            ))
        }
        super::BodySource::Explicit { .. } => None,
    }
}

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
    changed_bodies: Query<
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
    // **The bodies a CAST REPLACEMENT invalidates.** (H6)
    //
    // A new cast changes nothing on a body, so the change-detection query above
    // cannot see one — the worn id is still the worn id. That is exactly how a
    // body kept a retired cast's moves with every check green. This query is
    // **The bodies `apply_worn_character_gameplay` can actually see.**
    //
    // Its required columns, spelled out, because "does the derive match this
    // entity" has no shorter honest form. `WornCharacter` looks like the answer
    // and is not — it REQUIRES `IdentityKit`, so gating on either one silently
    // means "does it wear a character", which is a different and wrong question:
    // a hand-assembled body wearing a character without a moveset column matches
    // neither writer and would get no kit at all while reading as covered.
    //
    // ⚠ this list is coupled to that system's query. If a column is added there
    // and not here, bodies quietly fall between the two writers — which is the
    // failure this whole phase exists to remove, so the coupling is stated rather
    // than hidden behind a marker component that could drift on its own.
    persona_bodies: Query<
        Entity,
        (
            With<ambition_characters::actor::WornCharacter>,
            With<Name>,
            With<ambition_characters::brain::ActionSet>,
            With<crate::combat::moveset::ActorMoveset>,
            With<ambition_characters::brain::action_set::IdentityKit>,
            With<crate::actor::BodyAbilities>,
            With<crate::features::MotionModel>,
        ),
    >,
    // walked ONLY on the tick the registry changes, which is startup and hot
    // reload; the ordinary tick still pays only for identities that moved.
    all_bodies: Query<(
        Entity,
        Option<&ambition_characters::actor::WornCharacter>,
        Option<&crate::combat::CombatTuning>,
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
        // A mismatch in EITHER field re-projects. Same id, newer cast, is the
        // case that used to slip through — see the component's docs.
        let unchanged = projected.is_some_and(|projected| {
            Some(projected.id.as_str()) == resolved && projected.generation == registry.generation()
        });
        if unchanged || (projected.is_none() && resolved.is_none()) {
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
        if let Some(previous) = projected {
            previous.granted.retract(entity, &mut commands);
        }
        let Some(prepared) = resolved.and_then(|id| registry.get(id)) else {
            if projected.is_some() {
                commands.entity(entity).remove::<ProjectedCharacterKit>();
            }
            continue;
        };
        // ⚠ **this marker records what THIS system granted, and nothing else.**
        //
        // It used to be written here for every body, including the ones whose kit
        // belongs to `apply_worn_character_gameplay` — so a cast replacement
        // stamped a persona body as current while the writer that owns its kit had
        // not run and, filtered on `Changed<WornCharacter>`, never would. The body
        // recorded that it was up to date and no later pass revisited it, which is
        // worse than a missed update (GPT 5.6, 2026-07-29).
        //
        // That writer keeps its own record now, `avatar::PersonaBaseline`, stamped
        // after IT applies the baseline. One writer, one record. This one covers
        // the authored silhouette, the movement feel and the motion model below.
        commands.entity(entity).insert(ProjectedCharacterKit {
            id: prepared.id.clone(),
            generation: registry.generation(),
            granted: GrantedBodyFacts::of(prepared),
        });
        // **THE KIT, for the bodies the persona derive cannot see.**
        // (Phase B, 2026-07-29)
        //
        // `apply_worn_character_gameplay` is the ONE writer that turns a
        // `WornCharacter` into a persona. This system wrote seated bodies' kits
        // too, on the belief that seated bodies did not match it — and they always
        // did. `WornCharacter` is `#[require(IdentityKit)]` and `BodyAbilities`
        // comes with `AncillaryMovementBundle`, so the two columns the old comment
        // named as missing were both present (checked 2026-07-29).
        //
        // So there were two writers for one question, and they answered it
        // differently: this one wrote what the definition AUTHORED, while the
        // derive resolved authored-vs-catalog first. A character that authored no
        // action set fought as the worn player and stood empty-handed as player
        // two (campaign H1). Phase A made the two answers identical; deleting this
        // writer is what stops them drifting apart again.
        //
        // What is left here is the population the derive genuinely cannot see: a
        // body with no `WornCharacter` at all — an archetype-staged actor
        // identified by its `CombatTuning.sprite_character_id`. This is still its
        // only route to an authored moveset.
        //
        // Two writers on DISJOINT populations, named, rather than two writers on
        // overlapping ones. That is the honest state; collapsing the last of it
        // means giving tuning-identified bodies a real worn identity, which is a
        // change to what those bodies ARE and does not belong in this commit.
        //
        if !persona_bodies.contains(entity) {
            //
            // This used to project the moveset and the action set, because seated
            // bodies did not match `apply_worn_character_gameplay` — they were missing
            // two of its required columns — and something had to give them a kit.
            //
            // That made two writers for one question, on two paths, and they answered
            // it differently: this one wrote what the definition AUTHORED, while the
            // worn path resolved authored-vs-catalog first. A character that authored
            // no action set therefore fought as the worn player and stood empty-handed
            // as player two (campaign H1). Phase A made the answer identical; giving
            // seated bodies `IdentityKit` and `BodyAbilities` makes the WRITER
            // identical, which is the half that stops it happening again.
            //
            if let Some(moveset) = prepared.kit.projectable_moveset().cloned() {
                // The routing markers are NOT set here. They are derived from the
                // live `ActorMoveset` by `reconcile_moveset_routing_markers` —
                // deriving them is what makes them right for the persona path too,
                // which replaces the moveset and never knew the markers existed.
                commands
                    .entity(entity)
                    .insert(crate::combat::moveset::ActorMoveset(moveset));
            }
            if let Some(action_set) = prepared.kit.action_set().cloned() {
                let combat_kit = crate::combat::components::CombatKit::from_action_set(&action_set);
                commands.entity(entity).insert((action_set, combat_kit));
            }
        }
        // The rest is what the persona derive does not own on ANY path: the
        // authored silhouette, the movement feel, and the motion model — body
        // facts rather than kit facts, each with a matching retraction above.
        if let Some(hurtboxes) = prepared.hurtboxes.clone() {
            commands.entity(entity).insert((
                super::AuthoredHurtboxes(hurtboxes),
                super::ResolvedHurtboxes::default(),
                crate::combat::components::DamageableVolumes::default(),
            ));
        }
        // **THE AUTHORED BODY, which had no consumer at all.**
        //
        // `CharacterDefinition.body` has existed since §4.11 and nothing read it:
        // a provider could author `SpriteAuthored { world_per_pixel }` and receive
        // a body of some other size entirely (GPT 5.6, 2026-07-29).
        //
        // `SpritePosedBody` is the live authority for a sprite-shaped body — it
        // carries exactly this number, and `sync_sprite_posed_bodies` derives the
        // collision box, the sprite quad and its offset from the art every tick.
        // Until now it was inserted from ONE place in the repository: a bespoke
        // app-side system in the Mary-O snake matching on a display name. So body
        // geometry was still declared through a second seam, which is the problem
        // `register_character` exists to delete.
        if let Some(posed) = posed_body_for(prepared) {
            commands.entity(entity).insert(posed);
        }
        // **The MOTION MODEL, on the same path and for the X9 reason.**
        //
        // The worn-player path resolves this in `apply_worn_character_kit`;
        // wiring only that one is exactly the mistake the action set made — a
        // seated fighter would move by its catalog row while a worn player moved
        // by its definition, for the same character.
        //
        // Applied through `switch_motion_model` rather than by inserting a fresh
        // `MotionModel`: a cross-model change must preserve every shared body
        // fact and initialize only the DESTINATION solver's private state
        // (ADR 0024). Replacing the component wholesale would reset a momentum
        // rider to Airborne mid-stride.
        // The movement FEEL, on the same path and for the same reason as the
        // motion model above.
        //
        // ⚠ the marker's PRESENCE means "this body's feel is authored rather
        // than the shared dev tuning", so a stale one is a body moving like the
        // character it used to be. It is retracted in the block above, keyed on
        // the id this system recorded — GRANT here, RETRACT there.
        //
        // ⚠ two earlier shapes were both wrong, and the reasons are worth
        // keeping. Removing on `None` HERE made this fight the worn path: for a
        // character with CATALOG tuning and no authored tuning, that path
        // inserts the marker and this removed it on the same tick. Passing the
        // catalog in to resolve both the same way then violated the workspace
        // policy against `Option<Res<CharacterCatalog>>` — and requiring it
        // broke three fixtures that deliberately run character demand with NO
        // catalog, which is a state another test exists to name.
        if let Some(tuning) = prepared.movement_tuning {
            commands
                .entity(entity)
                .insert(ambition_engine_core::AuthoredMovementTuning(tuning));
        }
        {
            let spec = prepared.motion_model;
            commands.queue(move |world: &mut World| {
                let Some(mut model) = world.get_mut::<crate::features::MotionModel>(entity) else {
                    return;
                };
                ambition_engine_core::switch_motion_model(&mut model, spec);
            });
        }
    }
}

#[cfg(test)]
mod tests;
