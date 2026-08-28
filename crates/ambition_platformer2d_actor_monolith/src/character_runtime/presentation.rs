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

/// Character definition currently projected onto a body, including the catalog
/// generation that produced it so stale projections can be detected.
///
/// TODO(character-projection): record displaced moveset values as well as granted
/// values so removing an authored moveset can restore the body's prior one.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ProjectedCharacterKit {
    pub id: String,
    /// The cast this body's projected kit was derived from.
    pub generation: ambition_characters::prepared::CharacterCatalogGeneration,
    /// Facts granted by the projected character, recorded so they can be
    /// retracted even if the character changes or leaves the catalog.
    pub granted: GrantedBodyFacts,
}

/// What the projection put on a body, so it can take exactly that back.
///
/// `retract` DESTRUCTURES this struct, so adding a fact here is a COMPILE ERROR
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
    ///
    /// `movement_tuning` is the CALLER's resolved answer, not
    /// `prepared.movement_tuning`: a seat in a match may be granted a body its
    /// character never authored, and a record that read the definition would
    /// then fail to retract exactly the fact that WAS granted. See
    /// [`grant_prepared_character_body`].
    fn of(
        prepared: &ambition_characters::prepared::PreparedCharacterDefinition,
        movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    ) -> Self {
        Self {
            hurtboxes: prepared.hurtboxes.is_some(),
            movement_tuning: movement_tuning.is_some(),
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
                .remove::<ambition_platformer2d_core::AuthoredMovementTuning>();
        }
        if posed_body {
            commands
                .entity(entity)
                .remove::<ambition_sprite_sheet::character::SpritePosedBody>();
        }
    }
}

/// The sprite-posed body a definition's authored `body` asks for, if any.
///
/// only `BodySource::SpriteAuthored` resolves here.
///
/// Returns `None` when the character authored no sheet: the posed body reads its
/// rectangles off a sheet manifest, so one without a target has nothing to pose
/// against. Preparation already RESOLVES that target, so a typo is named at load
/// rather than producing a body that silently never poses.
fn posed_body_for(
    prepared: &ambition_characters::prepared::PreparedCharacterDefinition,
) -> Option<ambition_sprite_sheet::character::SpritePosedBody> {
    match prepared.body.as_ref()? {
        ambition_characters::actor::definition::BodySource::SpriteAuthored { world_per_pixel } => {
            Some(ambition_sprite_sheet::character::SpritePosedBody::new(
                prepared.sheet.as_deref()?,
                *world_per_pixel,
            ))
        }
        ambition_characters::actor::definition::BodySource::Explicit { .. } => None,
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

/// Who writes this body's action set and moves — the one axis on which
/// projecting a definition differs between its two callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitOwnership {
    /// Put the definition's kit on the body. Construction always does; the
    /// re-template pass does for a body the persona derive cannot see.
    Grant,
    /// `apply_worn_character_gameplay` owns it.
    PersonaDerive,
    /// The CALLER already resolved this body's kit and inserted it — a match
    /// seat, whose repertoire is the character's overlaid with the match's own
    /// override and so cannot be read off the definition alone.
    ///
    /// everything else is granted, and BOTH applied-template records are stamped. That is
    /// the difference from [`Self:PersonaDerive`], which leaves the gameplay baseline to the
    /// derive because the derive is coming.
    CallerResolved,
}

/// Put every fact a prepared character owns onto a body, as ONE batch.
///
/// the ONE place a prepared definition becomes a body, and that is the
/// point of extracting it. Two callers:
///
/// * CONSTRUCTION, so a normal character actor is COMPLETE on the frame it
/// is built. This is the fix and it is the architecture the rule asked for (§3): *"There should be no next-tick persona grant required for correctness."* A body built this way carries the memo already, so the re-template pass below sees it as current and never touches it.
/// * RE-TEMPLATING — a cast hot reload, a deliberate runtime re-wear — which is what [`project_prepared_character_definitions`] is FOR once ordinary spawning stops depending on it.
///
/// the memo goes in the same batch as the grants, deliberately. Splitting them is the shape
/// investigation kept circling: a save taken between the two restores a world claiming to be
/// projected and missing what the projection grants. One batch, one archetype move, one tick.
pub fn grant_prepared_character_body(
    commands: &mut Commands,
    entity: Entity,
    prepared: &ambition_characters::prepared::PreparedCharacterDefinition,
    generation: ambition_characters::prepared::CharacterCatalogGeneration,
    kit: KitOwnership,
    // THE BODY THIS ENTITY PLAYS WITH, already resolved by the caller.
    //
    // NOT read off `prepared` here, and that is the whole point. A seat
    // in a match answers to the MATCH's body as well as its character's, and
    // that weighing belongs in one place (`MatchRules::body_over`) rather than
    // in the materializer, which would then be a second authority on it — the
    // shape the kit already paid for once (`KitOwnership::CallerResolved`).
    // A caller with no match to answer to passes `prepared.movement_tuning`.
    movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
) {
    {
        // Construction writes the gameplay baseline unless persona derivation will
        // do so. A newly constructed body displaced nothing, so its baseline has
        // an empty `displaced` set even when the caller resolved its kit.
        if kit != KitOwnership::PersonaDerive {
            commands
                .entity(entity)
                .insert(crate::avatar::PersonaBaseline {
                    id: prepared.id.as_str().to_string(),
                    generation,
                    displaced: Default::default(),
                });
        }
        commands.entity(entity).insert(ProjectedCharacterKit {
            id: prepared.id.as_str().to_string(),
            generation,
            granted: GrantedBodyFacts::of(prepared, movement_tuning),
        });
        // Only bodies without `WornCharacter` use this grant path; worn personas
        // are projected by `apply_worn_character_gameplay`.
        // TODO(compat-remove): give tuning-identified staged bodies a real worn
        // identity, then remove this secondary kit writer.
        if kit == KitOwnership::Grant {
            // That made two writers for one question, on two paths, and they answered
            // it differently: this one wrote what the definition AUTHORED, while the
            // worn path resolved authored-vs-catalog first. A character that authored
            // no action set therefore fought as the worn player and stood empty-handed
            // as player two. Phase A made the answer identical; giving
            // seated bodies `IdentityKit` and `BodyAbilities` makes the WRITER
            // identical, which is the half that stops it happening again.
            if let Some(moveset) = prepared.kit.projectable_moveset().cloned() {
                // The routing markers are NOT set here. They are derived from the
                // live `ActorMoveset` by `reconcile_moveset_routing_markers` —
                // deriving them is what makes them right for the persona path too,
                // which replaces the moveset and never knew the markers existed.
                commands
                    .entity(entity)
                    .insert(ambition_combat::moveset::ActorMoveset(moveset));
            }
            if let Some(action_set) = prepared.kit.action_set().cloned() {
                let combat_kit =
                    ambition_combat::components::CombatKit::from_action_set(&action_set);
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
                ambition_combat::components::DamageableVolumes::default(),
            ));
        }
        // THE AUTHORED BODY, which had no consumer at all.
        //
        // `CharacterDefinition.body` has existed since §4.11 and nothing read it:
        // a provider could author `SpriteAuthored { world_per_pixel }` and receive
        // a body of some other size entirely.
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
        // The MOTION MODEL, on the same path and for the X9 reason.
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
        // two earlier shapes were both wrong, and the reasons are worth
        // keeping. Removing on `None` HERE made this fight the worn path: for a
        // character with CATALOG tuning and no authored tuning, that path
        // inserts the marker and this removed it on the same tick. Passing the
        // catalog in to resolve both the same way then violated the workspace
        // policy against `Option<Res<CharacterCatalog>>` — and requiring it
        // broke three fixtures that deliberately run character demand with NO
        // catalog, which is a state another test exists to name.
        if let Some(tuning) = movement_tuning {
            commands
                .entity(entity)
                .insert(ambition_platformer2d_core::AuthoredMovementTuning(tuning));
        }
        {
            let spec = prepared.motion_model;
            commands.queue(move |world: &mut World| {
                let Some(mut model) =
                    world.get_mut::<ambition_platformer2d_core::movement::MotionModel>(entity)
                else {
                    return;
                };
                ambition_platformer2d_core::switch_motion_model(&mut model, spec);
            });
        }
    }
}

#[cfg(test)]
mod tests;
