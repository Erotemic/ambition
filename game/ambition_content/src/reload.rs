//! Reload every move table from disk into a running host.
//!
//! This is the production caller of the staged cast revision road
//! (`stage_move_section`, admission, publication).
//!
//! It does not use `pack::shipped()`, the boot-scoped read: that function is a
//! `OnceLock` that serves the boot-time pack forever. Reload compiles a fresh
//! pack from disk and compares it with the App's selection
//! ([`crate::pack::selected`]). Every install reads that selection (I3 step 1),
//! and selection is not publication: only this transaction revises a family.
//!
//! A reload publishes the selected pack together with every participating
//! domain: see [`participates`] (the moveset, character catalog, Smash fighter,
//! boss and audio domains, plus [`PACK_DERIVED_FAMILIES`]). [`ReloadRequest`]
//! refuses a candidate that changes any other domain, because the canonical
//! identity would then name generation N+1 while that domain still serves N. A
//! family added later is refused by default.

use ambition_characters::prepared::{stage_move_section, MovesetRevisionError};
// Only tests use the combined admit-and-publish entry point. The production
// commit path holds an already-admitted value and calls
// `publish_admitted_revision`.
#[cfg(test)]
use ambition_characters::prepared::{activate_staged_revision, RevisionOutcome};

/// What a reload attempt did.
///
/// Each variant is a reachable state. Each non-`Activated` variant also says
/// what happened to the live cast.
#[derive(Debug, Clone, PartialEq)]
/// `PackRefused`, `NoMoveSection`, `Activated` and `Unchanged` are
/// `#[cfg(test)]` because only the test-only fixture road (`publish_candidate`,
/// `reload_move_tables*`) produces them. In a shipped build they would let
/// [`ReloadRequest::Refused`] carry states such as `Activated` that are not
/// refusals.
///
/// The other variants come from `request_reload`, some through
/// [`admit_candidate`].
pub enum MoveReload {
    /// The pack on disk does not compile. Nothing was staged and the live cast
    /// is not changed. The string is the content compiler's full diagnostic.
    #[cfg(test)]
    PackRefused(String),
    /// The pack compiled and carries no move section at all.
    ///
    /// Not an error: removing all `moveset` sources is a valid edit, and it
    /// does not ask to clear the live cast's movesets. Reported so a caller can
    /// tell it from a reload with zero changes.
    #[cfg(test)]
    NoMoveSection,
    /// The preparation barrier has not run, so there is no cast to revise.
    ///
    /// Kept separate from `UnknownCharacters`: a host that has not booted its
    /// cast has a lifecycle problem, not a content problem.
    NoCast,
    /// The section names characters this build did not prepare. Nothing was
    /// staged — `stage_move_section` is all-or-nothing.
    UnknownCharacters(Vec<String>),
    /// The cast was revised.
    #[cfg(test)]
    Activated { generation: u64, changed: usize },
    /// Every table on disk is what the live cast was already built from.
    ///
    /// A file watcher fires on a save, not on a change, so this is the common
    /// case.
    #[cfg(test)]
    Unchanged { generation: u64 },
    /// The revision was refused at admission: an authored effect names a
    /// technique this composition did not install. The previous cast is still
    /// the published one.
    Refused(Vec<String>),
    /// The candidate was prepared against a content generation that is no
    /// longer selected. Nothing was published (not the cast, not the
    /// selection).
    ///
    /// This is the only staleness refusal. `admit_candidate` compares the pack
    /// fingerprint before anything is staged, so a separate cast-generation
    /// check could not fire.
    StaleGeneration {
        prepared_against: String,
        active: String,
    },
    /// A rollback timeline is live over this world, so a content generation
    /// may not be published into it.
    ///
    /// This follows from an existing rule: the GGRS per-frame contract check in
    /// `ambition_platformer2d_rollback_ggrs` invalidates a live timeline when
    /// the prepared content identity changes under it. Refusing is better than
    /// publishing and then reporting a desync.
    RefusedDuringLiveTimeline,
    /// The rollback authority for this world is unhealthy.
    ///
    /// Publishing must not heal it. `RollbackTimelineStatus::carried_from`
    /// passes an unhealthy reason to the replacement timeline, and only
    /// `acknowledge_and_clear` may clear it. A content publication that made a
    /// fresh timeline would otherwise hide a desync.
    RefusedWhileRollbackUnhealthy(String),
    /// The candidate changes a mechanical domain that does not participate in
    /// the generation transaction. Publishing it would make the content
    /// identity false.
    ///
    /// [`participates`] is the authority for which domains participate. Do not
    /// keep a list here. A domain that no reload road replaces must not
    /// participate: a candidate that changed it would make
    /// `PreparedContentIdentity` name N+1 while that domain serves N.
    ///
    /// The rule fails safe: every non-participating domain is refused. When a
    /// domain joins the transaction, flip the test that pins its refusal to
    /// pin its publication.
    ///
    /// The check is sound per domain, not per field. A handler can define a
    /// row whose canonical string omits a field it lowered, and no compiler
    /// check sees that.
    RefusedUnsupportedChangedDomain(Vec<String>),
    /// The candidate stops naming characters whose authored moveset the live
    /// cast plays. Nothing was staged or published.
    ///
    /// Staging iterates the candidate's keys, so a dropped character would
    /// keep its old moves under the new generation. See
    /// [`dropped_moveset_entities`].
    RefusedDroppedMovesetEntities(Vec<String>),
    /// This world installs no technique table.
    ///
    /// Absent is not empty. An empty `TechniqueSupport` means "this host
    /// installs nothing" and admission then refuses every authored effect. No
    /// `InstalledTechniques` resource means the combat capability is not
    /// installed, so `unwrap_or_default()` would report a false roster-wide
    /// refusal.
    NoTechniqueSupport,
    /// The candidate's boss roster or encounters do not form one catalog with
    /// the other providers' fragments. Nothing was staged; the live bosses
    /// keep the published catalog.
    BossCatalogRefused(String),
    /// The candidate's character catalog does not assemble, or a definition
    /// built from it does not stage. Nothing was published; the live cast
    /// keeps its catalog. (A character added or removed is NOT refused: an
    /// added one is staged, a removed one retired; see `CandidateCatalog`.)
    CharacterCatalogRefused(String),
    /// The candidate's audio registries do not compose with the other
    /// providers' fragments (a music track id that another provider maps to a
    /// different file). Nothing was staged; the live audio keeps its catalog.
    AudioCatalogRefused(String),
    /// The candidate's quest book is not compatible with the progress the save
    /// records: a quest in progress at a step the candidate no longer has, or
    /// whose completed prefix or current objective changed condition. Nothing
    /// was staged. The quest registry is session state, rebuilt from the
    /// selected pack and the save when the next session starts
    /// (`quest::populate_quest_registry`); that rebuild would clamp a step it
    /// cannot place and would read an index against a different step list, so
    /// the move would silently rewind a player or change what they are chasing.
    /// The rule is on `candidate_quest_book`.
    QuestBookRefused(String),
    /// The candidate gives a cutscene an id that ANOTHER PROVIDER's row holds in
    /// the shared `CutsceneLibrary`. Nothing was staged. The library is one map
    /// with no record of who wrote a row, so a publication that inserted the
    /// candidate's row would erase the other provider's; refusing is the only
    /// answer that loses nothing. See [`candidate_cutscene_ownership`].
    CutsceneOwnershipRefused(String),
    /// The candidate breaks a reference the unchanged world or another family
    /// holds (a room whose `entry_cutscene` names a script the candidate no
    /// longer has, a quest step naming a boss that does not exist, a boss phase
    /// naming a music track the candidate dropped). The same judge as startup
    /// (`content_validation::validate_content_graph`), asked of the candidate
    /// against the world that is running. Nothing was staged.
    ContentGraphRefused(Vec<String>),
}

/// Republish the cast's move tables from an already-compiled pack.
///
/// Split from [`reload_move_tables`] so a test can use an edited pack instead
/// of writing into `game/ambition_content/assets`.
#[cfg(test)]
pub(crate) fn reload_move_tables_from(
    world: &mut bevy::ecs::world::World,
    fresh: &ambition_content_pack::PreparedContentPack,
) -> MoveReload {
    let Some(section) = ambition_characters::moveset_content_schema::lowered_movesets(fresh) else {
        return MoveReload::NoMoveSection;
    };

    let problems = stage_move_section(world, section);
    if problems
        .iter()
        .any(|p| matches!(p, MovesetRevisionError::NoStagedCast))
    {
        return MoveReload::NoCast;
    }
    if !problems.is_empty() {
        return MoveReload::UnknownCharacters(problems.iter().map(ToString::to_string).collect());
    }

    // See `MoveReload::NoTechniqueSupport` for why an absent resource is
    // reported and not defaulted.
    let Some(support) = world
        .get_resource::<ambition_combat::technique::InstalledTechniques>()
        .map(|installed| installed.0.clone())
    else {
        return MoveReload::NoTechniqueSupport;
    };
    match activate_staged_revision(world, &support) {
        RevisionOutcome::Activated {
            generation,
            changed,
        } => MoveReload::Activated {
            generation: generation.get(),
            changed,
        },
        RevisionOutcome::Unchanged { generation } => MoveReload::Unchanged {
            generation: generation.get(),
        },
        RevisionOutcome::Refused { refusals } => {
            MoveReload::Refused(refusals.iter().map(|r| r.detail.clone()).collect())
        }
        // Unreachable: the section lowered and staging reported no problem, so
        // something is staged. Reported, not panicked, so a reload loop cannot
        // crash the game.
        RevisionOutcome::NothingStaged => MoveReload::NoMoveSection,
    }
}

/// Reload from a content directory, not from this build's own sources.
///
/// This is the fast-iteration loop with no compiler: edit a `.ron` under
/// `root`, call this, and the running cast plays the edit (fast-iteration I2).
///
/// The root must supply the whole pack. See [`crate::pack::compile_pack_from`]
/// and [`crate::pack::export_sources_to`].
#[cfg(test)]
pub(crate) fn reload_move_tables_from_dir(
    world: &mut bevy::ecs::world::World,
    root: &std::path::Path,
) -> MoveReload {
    // No base claim: the base is the pack fingerprint, which is not known
    // before this compile. A caller that needs staleness protection builds its
    // own `CandidateGeneration::prepared_against(pack, Some(base))`.
    match crate::pack::compile_pack_from(root) {
        Ok(pack) => reload_move_tables_selecting(world, std::sync::Arc::new(pack)),
        Err(refusal) => MoveReload::PackRefused(refusal),
    }
}

/// Decide if a candidate generation may proceed. Both roads use this one
/// function, so one rule is tested for both.
///
/// The order is part of the contract. The verdict comes first: a mechanically
/// identical candidate publishes nothing and so cannot invalidate a timeline.
/// Asking the boundary first would report a no-op save as
/// `RefusedDuringLiveTimeline`. `Stale` also does not need the boundary. Only
/// `Publish` asks for permission.
enum CandidateAdmission {
    /// It may not, and this is the answer BOTH roads report.
    Refused(MoveReload),
    /// There is nothing to do. Not a refusal — a real mechanical no-op.
    ///
    /// Carries nothing. The direct road reads the cast generation from the
    /// registry; the request road requested nothing and has no generation.
    Unchanged,
    /// It may.
    Proceed,
}

fn admit_candidate(
    world: &bevy::ecs::world::World,
    candidate: &ambition_content_pack::CandidateGeneration,
) -> CandidateAdmission {
    let active = crate::pack::selected(world).map(|pack| pack.fingerprint);
    match candidate.verdict(active) {
        ambition_content_pack::CandidateVerdict::Stale {
            prepared_against,
            active,
        } => {
            return CandidateAdmission::Refused(MoveReload::StaleGeneration {
                prepared_against: prepared_against.hex(),
                active: active.hex(),
            })
        }
        ambition_content_pack::CandidateVerdict::Unchanged { .. } => {
            return CandidateAdmission::Unchanged
        }
        ambition_content_pack::CandidateVerdict::Publish { .. } => {}
    }

    // Diff against the active pack before anything is staged or selected;
    // after selection it would diff the candidate against itself.
    let unsupported = unsupported_changed_domains(world, candidate.pack());
    if !unsupported.is_empty() {
        return CandidateAdmission::Refused(MoveReload::RefusedUnsupportedChangedDomain(
            unsupported,
        ));
    }

    // The participating domain must also be applicable. Check here, because
    // staging iterates the candidate's keys and never sees a dropped entity.
    if let Some(active) = crate::pack::selected(world) {
        // The predicate lives beside the section it reads, in
        // `ambition_characters::moveset_content_schema`.
        let dropped =
            ambition_characters::moveset_content_schema::dropped_moveset_entities(
                active,
                candidate.pack(),
            );
        if !dropped.is_empty() {
            return CandidateAdmission::Refused(MoveReload::RefusedDroppedMovesetEntities(dropped));
        }
    }

    match publication_boundary(world) {
        PublicationBoundary::Legal => CandidateAdmission::Proceed,
        // A live timeline that this host may rebase is not a refusal. This
        // reuses the LDtk reload protocol: `restart_local_ggrs_after_hot_reload`
        // stops the session and releases ownership, and
        // `maintain_local_session` rebuilds it next frame with the same policy
        // and seating. Per `RollbackSessionOwnership`, local sync-test sessions
        // may be recreated around a content reload; external/P2P sessions may
        // not.
        //
        // The question is "may this host rebase it", and it is asked against
        // the live world here and again at the commit.
        PublicationBoundary::RebasableTimeline => CandidateAdmission::Proceed,
        PublicationBoundary::ForeignTimeline => {
            CandidateAdmission::Refused(MoveReload::RefusedDuringLiveTimeline)
        }
        PublicationBoundary::Unhealthy(reason) => {
            CandidateAdmission::Refused(MoveReload::RefusedWhileRollbackUnhealthy(reason))
        }
    }
}

/// Publish a complete candidate generation, or refuse it, as one act.
///
/// The decision uses the whole pack's identity, not the move family's.
/// [`ambition_content_pack::CandidateGeneration::verdict`] answers stale /
/// no-op / publish from the pack's `ContentFingerprint`, which covers every
/// content id, schema, capability, asset and reference. The move outcome
/// follows from that decision. Deciding from the moves alone would report
/// this as unchanged while installing new items:
///
/// ```text
/// generation N   moves = A   items = X
/// candidate      moves = A   items = Y
/// → "Unchanged", and the whole candidate pack became the App's selection.
/// ```
///
/// A changed pack with identical moves publishes the selection and leaves the
/// cast generation alone.
///
/// Not done yet (fast-iteration I3): this sets no `ContentEpoch`, no
/// `PreparedContentIdentity` and no rollback timeline boundary. The pack
/// fingerprint is the only base clock.
#[cfg(test)]
pub(crate) fn publish_candidate(
    world: &mut bevy::ecs::world::World,
    candidate: ambition_content_pack::CandidateGeneration,
) -> MoveReload {
    match admit_candidate(world, &candidate) {
        CandidateAdmission::Refused(answer) => return answer,
        CandidateAdmission::Unchanged => {
            return MoveReload::Unchanged {
                generation: world
                    .get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
                    .map(|registry| registry.generation().get())
                    .unwrap_or_default(),
            }
        }
        CandidateAdmission::Proceed => {}
    }
    let audio = match candidate_audio(world, candidate.pack()) {
        Ok(audio) => audio,
        Err(error) => return MoveReload::AudioCatalogRefused(error),
    };
    let adaptive_cues = match candidate_adaptive_cues(world, candidate.pack()) {
        Ok(cues) => cues,
        Err(error) => return MoveReload::AudioCatalogRefused(error),
    };
    if let Err(error) = candidate_quest_book(world, candidate.pack()) {
        return MoveReload::QuestBookRefused(error);
    }
    if let Err(error) = candidate_cutscene_ownership(world, candidate.pack()) {
        return MoveReload::CutsceneOwnershipRefused(error);
    }
    let pack = candidate.into_pack();
    let outcome = reload_move_tables_from(world, &pack);
    // The selection follows the cast's admission, not the compile. A refused
    // or stale revision leaves the App on the pack its cast was built from.
    if matches!(
        outcome,
        MoveReload::Activated { .. } | MoveReload::Unchanged { .. }
    ) {
        if let Some(audio) = audio {
            publish_audio(world, audio);
        }
        publish_adaptive_cues(world, adaptive_cues);
        crate::pack::install_selection(world, pack);
    }
    outcome
}

/// Does this schema id name a family the generation transaction can carry?
///
/// A `match`, not a table of ids: a forgotten family is refused instead of
/// promoted and never published.
///
/// `fighter_brain_ladder` was the second family and needed no new resource,
/// ordering edge or refusal; see [`publish_participant_families`].
fn participates(domain: &str) -> bool {
    domain == ambition_characters::moveset_content_schema::MOVESET_SCHEMA
        || domain == ambition_characters::actor::character_catalog::CHARACTER_CATALOG_SCHEMA
        || domain == ambition_characters::smash_fighter::SMASH_FIGHTER_SCHEMA
        || BOSS_DOMAINS.contains(&domain)
        || AUDIO_DOMAINS.contains(&domain)
        || ADAPTIVE_MUSIC_DOMAINS.contains(&domain)
        || PACK_DERIVED_FAMILIES
            .iter()
            .any(|family| family.domain == domain)
}

/// The domains of the boss catalog. Like `moveset`, they are admitted at
/// request time against world state (the other providers' fragments in
/// `BossCatalogRegistry`), so [`PendingGeneration`] carries the candidate
/// catalog and the preparation freezes it (`PendingGenerationInputs::bosses`).
/// The seed library and the validator bands are not in the catalog, and stay
/// refused.
const BOSS_DOMAINS: &[&str] = &[
    ambition_boss_encounter::pattern::content_schema::BOSS_PROFILES_SCHEMA,
    ambition_boss_encounter::pattern::content_schema::BOSS_ENCOUNTER_SCHEMA,
];

/// The domains of the audio registries. Admitted at request time against the
/// other providers' fragments in `AudioCatalogRegistry` (a music track id is
/// global across providers), so [`PendingGeneration`] carries the candidate
/// registry. The cue and track domains are the rows inside the two registries.
const AUDIO_DOMAINS: &[&str] = &[
    ambition_audio::content_schema::MUSIC_REGISTRY_SCHEMA,
    ambition_audio::content_schema::MUSIC_TRACK_SCHEMA,
    ambition_audio::content_schema::SFX_REGISTRY_SCHEMA,
    ambition_audio::content_schema::SFX_CUE_SCHEMA,
];

/// The domains of the adaptive music catalog: the cue file, one cue, and one
/// encounter's binding to a cue. They take part in a reload with or without the
/// `audio` feature (they are pack content, and the pack is selected either
/// way); only a build with the director has a registry to publish them into.
const ADAPTIVE_MUSIC_DOMAINS: &[&str] = &[
    ambition_audio::content_schema::MUSIC_CUE_CATALOG_SCHEMA,
    ambition_audio::content_schema::MUSIC_CUE_SCHEMA,
    ambition_audio::content_schema::ENCOUNTER_MUSIC_BINDING_SCHEMA,
];

/// What a generation carries for the adaptive music catalog: the candidate
/// registry, `None` when it changes no cue domain. A build without the music
/// director has no registry, so it carries nothing.
#[cfg(feature = "audio")]
type AdaptiveCues = Option<ambition_audio::music::AdaptiveMusicCatalogRegistry>;
#[cfg(not(feature = "audio"))]
type AdaptiveCues = Option<std::convert::Infallible>;

/// The adaptive music catalog a candidate publishes, or nothing when it
/// changes no cue domain. Assembled from the App's registry with Ambition's
/// catalog replaced (`AdaptiveMusicCatalogRegistry::with_replaced`), so another
/// provider's catalog survives and the App's registry is not touched here.
///
/// Admission, not publication: a catalog the director's registry would refuse
/// refuses the request, before anything is staged.
#[cfg(feature = "audio")]
fn candidate_adaptive_cues(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<AdaptiveCues, String> {
    let changes_cues = crate::pack::selected(world).is_some_and(|active| {
        ambition_content_pack::changed_domains(active, pack)
            .iter()
            .any(|schema| ADAPTIVE_MUSIC_DOMAINS.contains(&schema.0.as_str()))
    });
    if !changes_cues {
        return Ok(None);
    }
    let registry = world
        .get_resource::<ambition_audio::music::AdaptiveMusicCatalogRegistry>()
        .ok_or("this App registers no adaptive music catalog")?;
    registry
        .with_replaced(
            crate::AMBITION_CONTENT_PROVIDER,
            crate::music::music_cue_catalog_from(pack),
        )
        .map(Some)
        .map_err(|error| error.to_string())
}
#[cfg(not(feature = "audio"))]
fn candidate_adaptive_cues(
    _world: &bevy::ecs::world::World,
    _pack: &ambition_content_pack::PreparedContentPack,
) -> Result<AdaptiveCues, String> {
    Ok(None)
}

/// The providers a candidate's adaptive registry carries a catalog for: what
/// the preparation channel needs of it (`adaptive_cues_ready_for`).
fn adaptive_providers_of(cues: &AdaptiveCues) -> Option<std::collections::BTreeSet<String>> {
    #[cfg(feature = "audio")]
    return cues
        .as_ref()
        .map(|registry| registry.providers().map(str::to_owned).collect());
    #[cfg(not(feature = "audio"))]
    {
        let _ = cues;
        None
    }
}

/// Publish a reload's adaptive music catalog.
///
/// One resource, read each frame by the intent system and the director, so the
/// replacement is the whole publication: a cue's authority (`authorize_cues`)
/// is derived from the catalog each frame and holds no copy. What the director
/// caches (`LoadedMusicCueAssets`) is keyed by the asset path, so a cue whose
/// file changed is requested again on its next play. A cue that is playing keeps
/// the layers it started with until the director next shuts it down or changes
/// state.
fn publish_adaptive_cues(world: &mut bevy::ecs::world::World, cues: AdaptiveCues) {
    #[cfg(feature = "audio")]
    if let Some(registry) = cues {
        world.insert_resource(registry);
    }
    #[cfg(not(feature = "audio"))]
    let _ = (world, cues);
}

/// The audio catalog a candidate publishes, or `None` when it changes no audio
/// domain. Built from the App's registry with Ambition's fragment replaced.
fn candidate_audio(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<Option<ambition_audio::catalog::AudioCatalogRegistry>, String> {
    let changes_audio = crate::pack::selected(world).is_some_and(|active| {
        ambition_content_pack::changed_domains(active, pack)
            .iter()
            .any(|schema| AUDIO_DOMAINS.contains(&schema.0.as_str()))
    });
    if !changes_audio {
        return Ok(None);
    }
    let registry = world
        .get_resource::<ambition_audio::catalog::AudioCatalogRegistry>()
        .ok_or("this App registers no audio catalog")?;
    let music = ambition_audio::content_schema::lowered_music_registry(pack).cloned();
    let sfx = ambition_audio::content_schema::lowered_sfx_registry(pack).cloned();
    registry
        .with_replaced(crate::AMBITION_CONTENT_PROVIDER, music, sfx)
        .map(Some)
        .map_err(|error| error.to_string())
}

/// Publish a reload's audio catalog.
///
/// A gameplay session selects its provider's registries when it is activated
/// (`select_shell_audio_context`), and a reload activates a new session, so
/// the new cues and tracks are selected there; that selection can run before
/// this command in the same frame, so the active selection is revised here
/// too (`ActiveAudioSelection::revise_provider`). The other tables outlive a
/// session: the catalog itself, the host's music track table, and the SFX
/// registry copy the host built its library from. A cue
/// synthesized from the old spec is not played again: the handle cache checks
/// the spec's fingerprint (`ProviderSfxHandleCache::handle_for`).
fn publish_audio(
    world: &mut bevy::ecs::world::World,
    audio: ambition_audio::catalog::AudioCatalogRegistry,
) {
    let provider = crate::AMBITION_CONTENT_PROVIDER;
    if let Some(sfx) = audio.sfx_for(provider).cloned() {
        if world.contains_resource::<ambition_audio::spec::SfxRegistry>() {
            world.insert_resource(sfx);
        }
    }
    let bank_ids = world
        .get_resource::<ambition_audio::catalog::SfxBankRegistry>()
        .map(|banks| banks.ids_for(provider))
        .unwrap_or_default();
    if let Some(mut selection) =
        world.get_resource_mut::<ambition_audio::selection::ActiveAudioSelection>()
    {
        selection.revise_provider(provider, audio.music_for(provider), audio.sfx_for(provider), bank_ids);
    }
    #[cfg(feature = "audio")]
    if let Ok(music) = audio.combined_music_registry(crate::AMBITION_CONTENT_PROVIDER) {
        let catalog = world
            .get_resource::<ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog>()
            .cloned();
        if let Some(mut library) = world.get_resource_mut::<ambition_audio::library::AudioLibrary>() {
            let resolve = |id: &str| {
                catalog.as_ref().and_then(|catalog| {
                    catalog.path_for(
                        &ambition_platformer2d::asset_manager::platformer_assets::ids::music_track(id),
                    )
                })
            };
            library.revise_music_tracks(&music, Some(&resolve));
        }
    }
    world.insert_resource(audio);
}

/// The character catalog a candidate publishes with its cast, or `None` when
/// it does not change the catalog. Assembled from the App's registry with
/// Ambition's fragment rebuilt from the candidate pack, like the boss catalog.
/// The cast is staged from it separately ([`stage_cast_from_catalog`]).
fn candidate_character_catalog(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<Option<ambition_characters::prepared::CandidateCatalog>, String> {
    use ambition_characters::actor::character_catalog as cc;
    let changes_catalog = crate::pack::selected(world).is_some_and(|active| {
        ambition_content_pack::changed_domains(active, pack)
            .iter()
            .any(|schema| schema.0 == cc::CHARACTER_CATALOG_SCHEMA)
    });
    if !changes_catalog {
        return Ok(None);
    }
    let data = cc::lowered_catalog(pack).ok_or("the pack carries no lowered character catalog")?;
    let fragment = crate::character_catalog::catalog_fragment(data.clone()).map_err(|e| e.to_string())?;
    let registry = world
        .get_resource::<cc::CharacterCatalogRegistry>()
        .ok_or("this App registers no character catalog")?;
    let (registry, assembled) = registry.with_replaced(fragment).map_err(|e| e.to_string())?;
    let live = world.get_resource::<cc::CharacterCatalog>().ok_or("this App has no character catalog")?;
    // A character the candidate builds and the live catalog did not is staged
    // like every other (`stage_cast_from_catalog`); one the live catalog built
    // and the candidate does not is RETIRED: it leaves the cast and the stored
    // source with this revision. Only this provider knows which rows it builds,
    // so it names them.
    let (before, after) = (
        crate::character_catalog::buildable_ids(live),
        crate::character_catalog::buildable_ids(&assembled.catalog),
    );
    let retired = before
        .difference(&after)
        .map(|id| ambition_entity_catalog::CharacterId::new(id.as_str()))
        .collect();
    Ok(Some(ambition_characters::prepared::CandidateCatalog { registry, assembled, retired }))
}

/// Stage every buildable character again, defined from the candidate catalog
/// and the candidate pack: the definitions are a function of both
/// (`character_catalog::buildable_definitions`), the moves included.
fn stage_cast_from_catalog(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> Result<(), String> {
    let rigs_admitted = ambition_characters::actor::BodyRigAdmission::of(world).admit;
    for definition in crate::character_catalog::buildable_definitions(catalog, pack, rigs_admitted)
    {
        let bindings = ambition_platformer2d_actor_monolith::character_runtime::definition::with_engine_vocabularies(
            ambition_characters::prepared::CharacterBindings::default(),
        );
        ambition_characters::prepared::stage_character_revision_in(world, definition, &bindings)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The boss catalog a candidate publishes, or `None` when it changes no boss
/// domain. Built from the App's registry with Ambition's fragment replaced, so
/// a candidate that does not assemble is refused before anything is staged.
fn candidate_bosses(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<Option<CandidateBosses>, String> {
    let changes_bosses = crate::pack::selected(world).is_some_and(|active| {
        ambition_content_pack::changed_domains(active, pack)
            .iter()
            .any(|schema| BOSS_DOMAINS.contains(&schema.0.as_str()))
    });
    if !changes_bosses {
        return Ok(None);
    }
    let registry = world
        .get_resource::<ambition_boss_encounter::BossCatalogRegistry>()
        .ok_or("this App registers no boss catalog")?;
    let fragment = crate::bosses::boss_catalog_fragment_from(pack)?;
    let (registry, catalog) = registry.with_replaced(fragment).map_err(|error| error.to_string())?;
    Ok(Some(CandidateBosses { registry, catalog }))
}

/// A reload's boss catalog, and the registry it was assembled from.
struct CandidateBosses {
    registry: ambition_boss_encounter::BossCatalogRegistry,
    catalog: ambition_boss_encounter::BossCatalog,
}

/// One mechanical family whose whole publication is a function of the candidate
/// pack.
///
/// The schema id and its publisher are one declaration. A separate list of
/// ids could name a domain that passes [`unsupported_changed_domains`] but is
/// never published, so the family stays at generation N. Here the publisher is
/// the row, and [`participates`] scans this table, so an unlisted domain is
/// refused.
struct PackDerivedFamily {
    domain: &'static str,
    publish: fn(&mut bevy::ecs::world::World, &ambition_content_pack::PreparedContentPack),
}

/// Every family the transaction publishes from the pack alone.
///
/// `moveset` is not here. Its publication is admitted against world state
/// (the installed technique table and live cast generation), so
/// [`PendingGeneration`] carries the admitted value instead of re-deriving it
/// at the boundary.
const PACK_DERIVED_FAMILIES: &[PackDerivedFamily] = &[
    PackDerivedFamily {
        domain: ambition_combat::brain::fighter::content_schema::FIGHTER_BRAIN_LADDER_SCHEMA,
        publish: publish_fighter_ladder,
    },
    PackDerivedFamily {
        domain: ambition_encounter::content_schema::ENCOUNTER_WAVES_SCHEMA,
        publish: publish_encounter_waves,
    },
    PackDerivedFamily {
        domain: ambition_cutscene::content_schema::CUTSCENE_LIBRARY_SCHEMA,
        publish: publish_cutscene_library,
    },
    PackDerivedFamily {
        domain: ambition_persistence::quest::content_schema::QUEST_BOOK_SCHEMA,
        publish: publish_nothing_the_next_session_derives,
    },
    // The boss seed library and the validator bands are calibration for the
    // offline fight validator. MEASURED 2026-10-01: their only readers are
    // `bosses::seed_library` and `bosses::validator_bands`, and only
    // `tests/boss_fight_validator.rs` calls those. A running game reads neither,
    // so a reload takes their change and has nothing to publish.
    PackDerivedFamily {
        domain: ambition_boss_encounter::pattern::content_schema::BOSS_SEEDS_SCHEMA,
        publish: publish_nothing_a_running_game_reads,
    },
    PackDerivedFamily {
        domain: ambition_boss_encounter::pattern::content_schema::BOSS_VALIDATOR_BANDS_SCHEMA,
        publish: publish_nothing_a_running_game_reads,
    },
    PackDerivedFamily {
        domain: ambition_items::content_schema::ITEM_CATALOG_SCHEMA,
        publish: publish_item_catalog,
    },
];

/// A family no system of the running game reads: the new pack is selected,
/// and nothing else changes. If a runtime reader appears, this row must publish
/// what it reads instead.
fn publish_nothing_a_running_game_reads(
    _world: &mut bevy::ecs::world::World,
    _pack: &ambition_content_pack::PreparedContentPack,
) {
}

/// The quest book publishes nothing at the commit, and that is the design.
///
/// `QuestRegistry` is SESSION state, not a published table: a session's
/// teardown resets it, and the first tick of the next session fills it again
/// from the App's selected pack and the save (`quest::populate_quest_registry`).
/// A reload always activates a new session, and the commit installs the
/// selection before that session's first tick, so the new session's registry is
/// the candidate's quest book with the player's recorded progress. Editing the
/// live registry here would write a second copy of a fact the next session
/// derives, and a rollback-registered one. What the transaction owns is
/// ADMISSION: [`candidate_quest_book`] refuses what that derivation cannot place.
fn publish_nothing_the_next_session_derives(
    _world: &mut bevy::ecs::world::World,
    _pack: &ambition_content_pack::PreparedContentPack,
) {
}

/// Does the candidate still form one consistent content graph with the world
/// that is running?
///
/// ⭐ THE ONE JUDGE. This is `content_validation::validate_content_graph`, the
/// function startup runs over the composed game and aborts on, asked of the
/// candidate pack, its music registry and its character catalog against the
/// LDtk project the App is playing (`ActiveLdtkProject`, which a reload does not
/// change: the worlds are not a reloadable family). It replaces what this file
/// used to list as "not judged": a room naming a cutscene the candidate removed,
/// a quest step naming a boss, encounter, flag or room that does not exist, a
/// boss or encounter naming a music track the candidate dropped, an NPC naming
/// a dialogue or character the candidate no longer has. Warnings are not
/// refusals; the live generation is clean, so an error is the candidate's.
///
/// An App with no LDtk project (a headless composition without a world) has
/// nothing to judge against and is not asked.
///
/// Runs after the candidate's character catalog and boss catalog were admitted
/// (a candidate that cannot form them is already refused), so the lowerings it
/// reads are present; the music registry may be absent in a candidate and then
/// judges as an empty one, which names every track the world still asks for.
fn candidate_content_graph(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<(), Vec<String>> {
    let Some(project) = world.get_resource::<ambition_platformer2d_ldtk::ActiveLdtkProject>() else {
        return Ok(());
    };
    let music = ambition_audio::content_schema::lowered_music_registry(pack)
        .cloned()
        .unwrap_or_else(|| ambition_audio::spec::MusicRegistry { default_track: String::new(), tracks: Vec::new() });
    let report = crate::content_validation::validate_content_graph(
        pack,
        &music,
        &project.0,
        &crate::character_catalog::catalog_of(pack),
    );
    if report.is_ok() { Ok(()) } else { Err(report.errors) }
}

/// Is the candidate's quest book COMPATIBLE with the progress the save records?
///
/// ⭐ THE COMPATIBILITY RULE (stated, not inferred from an index). A save records
/// a quest as `(InProgress, k)`: steps `0..k` are DONE and step `k` is the
/// objective the player is pursuing now. That number means something only
/// against the step list it was recorded under, so the candidate is compared with
/// the quest book the App is playing (generation N, still the selection at this
/// point):
///
/// 1. **The step must exist.** `k < candidate.steps.len()`. The rebuild at the
///    next session clamps an out-of-range step to the last one, which would
///    rewind a player or push them past a step they never did, without a word.
/// 2. **The completed prefix and the current objective keep their CONDITIONS.**
///    For `i` in `0..=k`, the candidate's step `i` has the same
///    `QuestStepCondition` as N's. Otherwise the save says the player already
///    did a thing the candidate never asked of them, or silently swaps the
///    objective they are chasing (`BossDefeated(warden)` → `BossDefeated(dragon)`).
/// 3. **Free to edit:** a step's description text, every step AFTER `k`, the
///    title and summary, and `auto_start`. Nothing recorded depends on them.
///
/// Refused, never clamped or rewritten: the author finishes or resets the quest
/// (or edits only what rule 3 allows), and the refusal names the step and both
/// conditions.
///
/// Not judged: a quest the active pack does not carry (it was removed, or never
/// shipped) has no baseline to compare, so a save row for it is admitted under
/// rule 1 only; the save keeps its row, and a later candidate that names it again
/// resumes it. Also not judged: whether a step's boss, encounter, flag or room id
/// names something that exists. The startup graph validator checks those
/// (`validate_quest_conditions`), and `candidate_content_graph` asks it of the
/// candidate.
pub(crate) fn candidate_quest_book(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<(), String> {
    let active = crate::pack::selected(world);
    let changes_quests = active.is_some_and(|active| {
        ambition_content_pack::changed_domains(active, pack)
            .iter()
            .any(|schema| schema.0 == ambition_persistence::quest::content_schema::QUEST_BOOK_SCHEMA)
    });
    if !changes_quests {
        return Ok(());
    }
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return Ok(());
    };
    let Some(book) = ambition_persistence::quest::content_schema::lowered_quest_book(pack) else {
        return Ok(());
    };
    let playing = active.and_then(ambition_persistence::quest::content_schema::lowered_quest_book);
    for spec in book {
        let (state, step) = save.data().quest(&spec.id);
        if !matches!(state, ambition_persistence::save_data::PersistedQuestState::InProgress) {
            continue;
        }
        let current = usize::from(step);
        if current >= spec.steps.len() {
            return Err(format!(
                "quest '{}' is in progress at step {step} (counting from 0) but the candidate gives it {} step(s)",
                spec.id,
                spec.steps.len()
            ));
        }
        let Some(before) = playing.and_then(|book| book.iter().find(|quest| quest.id == spec.id)) else {
            continue;
        };
        for index in 0..=current {
            let (was, now) = (before.steps.get(index), &spec.steps[index]);
            if was.is_some_and(|was| was.condition != now.condition) {
                return Err(format!(
                    "quest '{}' is in progress at step {step} (counting from 0), and the candidate changes the condition of step {index} \
                     ({}) from {:?} to {:?}; a step the player has done, or is doing, keeps its condition (its text and the later steps are free to edit)",
                    spec.id,
                    if index < current { "already completed" } else { "the current objective" },
                    was.map(|was| &was.condition),
                    now.condition,
                ));
            }
        }
    }
    Ok(())
}

/// Absent in the candidate means remove, not keep. See
/// [`publish_participant_families`]; `profile_for_level` treats absent as the
/// engine floor.
fn publish_fighter_ladder(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) {
    use ambition_characters::brain::fighter::AuthoredFighterLadder;
    match ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(pack) {
        Some(ladder) => world.insert_resource(AuthoredFighterLadder(ladder.clone())),
        None => {
            world.remove_resource::<AuthoredFighterLadder>();
        }
    }
}

/// Absent in the candidate means remove. `ItemCatalogRead` treats an absent
/// catalog as the built-in table, which is what a pack without `items.ron`
/// plays at startup too.
///
/// The bag (`OwnedItems`) is not changed. It holds counts by grid slot, and a
/// catalog revision changes what a slot is called and how it is used, not how
/// many the player has.
fn publish_item_catalog(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) {
    match ambition_items::content_schema::lowered_item_catalog(pack) {
        Some(catalog) => world.insert_resource(catalog.clone()),
        None => {
            world.remove_resource::<ambition_items::ItemCatalog>();
        }
    }
}

/// Absent in the candidate means remove. `authored_encounter_waves` treats
/// `None` as "fall back to one wave from the level's spawn markers".
fn publish_encounter_waves(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) {
    use ambition_encounter::EncounterWaveBook;
    match ambition_encounter::content_schema::lowered_encounter_waves(pack) {
        Some(timelines) => world.insert_resource(EncounterWaveBook(timelines.clone())),
        None => {
            world.remove_resource::<EncounterWaveBook>();
        }
    }
}

/// May the candidate's cutscene rows be written without erasing another
/// provider's?
///
/// ⛔ `CutsceneLibrary` is ONE map from id to script and records no writer, so
/// "the rows Ambition owns" is not stored anywhere: it is derived as the rows
/// the App's selected pack lowered (generation N) that the library still holds
/// unchanged. A candidate row may take an id only when the library has no row
/// there, or holds exactly generation N's row (Ambition's own, being replaced).
/// A row at that id that is anything else belongs to another provider (or to a
/// provider that overwrote ours), and the publication, which cannot refuse,
/// would overwrite it. So this refuses, naming the ids.
///
/// Asked again at the activation gate: a provider can add a row between the
/// request and the activation, and a check that held at the request must not be
/// trusted at the publication.
///
/// A family the candidate does not change writes nothing and is not asked.
pub(crate) fn candidate_cutscene_ownership(
    world: &bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) -> Result<(), String> {
    let Some(library) = world.get_resource::<ambition_cutscene::CutsceneLibrary>() else {
        return Ok(());
    };
    let previous = crate::pack::selected(world)
        .map(crate::dialogue::cutscene_defaults::cutscene_scripts_of)
        .unwrap_or_default();
    let next = crate::dialogue::cutscene_defaults::cutscene_scripts_of(pack);
    if previous == next {
        return Ok(());
    }
    let mut taken: Vec<&str> = next
        .iter()
        .filter(|script| match library.get(&script.id) {
            None => false,
            Some(held) => !previous.iter().any(|ours| ours.id == script.id && ours == held),
        })
        .map(|script| script.id.as_str())
        .collect();
    taken.sort_unstable();
    if taken.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the candidate's cutscene id(s) {} are held by another provider's row in the shared library; publishing would erase it",
            taken.join(", ")
        ))
    }
}

/// Replace the cutscene scripts THIS PACK owns, and only those.
///
/// ⛔ The library is a shared registry: `AmbitionDialogueContentPlugin` adds the
/// pack's rows at composition, and another plugin or a test may add its own, so
/// the publication is not "insert a new library". The rows Ambition owns are the
/// ones the App's selected pack (generation N, still selected at this point)
/// lowered; a row is removed only while the library still holds exactly that
/// script, so a foreign row that reuses an id survives the removal. The insert
/// would overwrite one, which is why [`candidate_cutscene_ownership`] refuses a
/// candidate that names an id a foreign row holds, before this runs.
///
/// A cutscene that is PLAYING is not touched: `drain_cutscene_triggers` clones
/// the script into the runtime when it starts, so the running cutscene finishes
/// as generation N wrote it and the next trigger reads N+1.
///
/// An unchanged cutscene family writes nothing.
fn publish_cutscene_library(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) {
    let previous = crate::pack::selected(world)
        .map(crate::dialogue::cutscene_defaults::cutscene_scripts_of)
        .unwrap_or_default();
    let next = crate::dialogue::cutscene_defaults::cutscene_scripts_of(pack);
    if previous == next {
        return;
    }
    let Some(mut library) = world.get_resource_mut::<ambition_cutscene::CutsceneLibrary>() else {
        return;
    };
    for script in &previous {
        if library.get(&script.id) == Some(script) {
            library.scripts.remove(&script.id);
        }
    }
    for script in next {
        library.insert(script);
    }
}

/// Publish every participating family that is a pure function of the candidate
/// pack — the whole of [`PACK_DERIVED_FAMILIES`], in one act.
///
/// These are derived at the boundary, not carried like `admitted_cast`.
/// [`PendingGeneration`] carries the admitted cast because admission asked
/// the world a question that can go stale. These families are total functions
/// of the pack, so re-deriving cannot disagree, and a carried copy would be a
/// second authority.
///
/// Absent in the candidate means remove. Keeping generation N's resource would
/// promote the pack while the family stays at N.
///
/// Nothing here can refuse; the commit path stays infallible.
///
/// No new ordering edge is needed. `project_authored_fighter_ladder` re-reads
/// every fighter and rewrites when the rung differs, so it does not depend on
/// when brains spawn. The `GameplaySessionSet::Providers` edge in [`register`]
/// is for the cast.
fn publish_participant_families(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) {
    for family in PACK_DERIVED_FAMILIES {
        (family.publish)(world, pack);
    }
}


/// Does this candidate change the family the staged road publishes?
///
/// Only participants that changed prepare. A waves-only or ladder-only edit
/// must not stage and admit every move table, and must not need
/// `InstalledTechniques` (a composition without combat can still reload its
/// waves). This uses the same `changed_domains` as the unsupported-domain
/// diff, so there is one source for "what changed".
///
/// No active pack means changed: a first publication must stage the cast.
fn moveset_changed(
    world: &bevy::ecs::world::World,
    candidate: &ambition_content_pack::PreparedContentPack,
) -> bool {
    let Some(active) = crate::pack::selected(world) else {
        return true;
    };
    ambition_content_pack::changed_domains(active, candidate)
        .iter()
        .any(|schema| schema.0 == ambition_characters::moveset_content_schema::MOVESET_SCHEMA)
}

/// Does this candidate change a character's platform-fighter facet?
fn fighter_facets_changed(
    world: &bevy::ecs::world::World,
    candidate: &ambition_content_pack::PreparedContentPack,
) -> bool {
    crate::pack::selected(world).is_some_and(|active| {
        ambition_content_pack::changed_domains(active, candidate)
            .iter()
            .any(|schema| schema.0 == ambition_characters::smash_fighter::SMASH_FIGHTER_SCHEMA)
    })
}

/// Domains this candidate changes that nothing can publish.
///
/// Read against the active pack, so it must run before anything is staged or
/// selected.
fn unsupported_changed_domains(
    world: &bevy::ecs::world::World,
    candidate: &ambition_content_pack::PreparedContentPack,
) -> Vec<String> {
    let Some(active) = crate::pack::selected(world) else {
        // No active pack is a first publication, not a change.
        return Vec::new();
    };
    ambition_content_pack::changed_domains(active, candidate)
        .into_iter()
        .filter(|schema| !participates(&schema.0))
        .map(|schema| schema.0)
        .collect()
}

/// May a content generation be published into this world right now?
///
/// Computed here, not passed as a parameter, so no caller can supply a wrong
/// answer and a second caller cannot grow a second opinion.
///
/// This is a total mapping of
/// `ambition_platformer2d::rollback::mechanical_mutation_boundary`, the single
/// authority for "mechanical mutation is legal around rollback" (used by both
/// `Q118` and `Q120`). Ownership is part of it, so the lease that re-asks this
/// during the pending interval checks ownership as well as health.
///
/// The callers differ only in lifetime: `Q120` asks once before the next GGRS
/// advance; `Q118` holds the answer as a lease for the pending generation.
///
/// No authority means legal: with no rollback there is no timeline to
/// invalidate. A stood-down timeline is legal because it is not speculating.
#[derive(Debug)]
enum PublicationBoundary {
    Legal,
    /// Live, healthy, and this host started it and may stop it.
    RebasableTimeline,
    /// Live and healthy, but owned by peers (`External`) or by a caller that did
    /// not ask for a content rebase.
    ForeignTimeline,
    Unhealthy(String),
}

/// May this host stop and rebuild the live rollback timeline for a content
/// publication? Forwards to the rollback subsystem's own predicate.
///
/// This is about ownership, not liveness. Only a locally maintained sync-test
/// session may be rebased. `External` (peer-owned) and `Caller`-owned sessions
/// must never be replaced; see `RollbackSessionOwnership`.
pub(crate) fn rebasable_local_timeline(world: &bevy::ecs::world::World) -> bool {
    ambition_platformer2d::rollback::locally_rebasable_timeline(world)
}

/// Stop the local timeline so the session owner rebases it onto the generation
/// just published.
///
/// Call this in the same exclusive step as the publication. A frame between
/// them would resimulate new content on the old timeline (the `Q118` desync).
/// Releasing ownership is enough: `maintain_local_session` starts a new
/// session next frame with the same policy and seating.
fn rebase_local_timeline_onto_the_new_generation(world: &mut bevy::ecs::world::World) {
    if !rebasable_local_timeline(world) || !ambition_platformer2d::rollback::session_is_active(world)
    {
        return;
    }
    ambition_platformer2d::rollback::stop_session(world);
    world
        .resource_mut::<ambition_platformer2d::rollback::local_session::LocalSessionOwnership>()
        .release();
    bevy::log::info!(
        target: "ambition_content::reload",
        "the published generation stopped the local rollback baseline; the session \
         owner will rebase it onto the new content"
    );
}

fn publication_boundary(world: &bevy::ecs::world::World) -> PublicationBoundary {
    use ambition_platformer2d::rollback::MechanicalMutationBoundary;

    match ambition_platformer2d::rollback::mechanical_mutation_boundary(world) {
        MechanicalMutationBoundary::NoTimeline => PublicationBoundary::Legal,
        MechanicalMutationBoundary::LocallyRebasable => PublicationBoundary::RebasableTimeline,
        MechanicalMutationBoundary::ForeignTimeline => PublicationBoundary::ForeignTimeline,
        MechanicalMutationBoundary::Unhealthy(reason) => PublicationBoundary::Unhealthy(reason),
    }
}

/// Publish a freshly compiled pack as a candidate prepared against what is live.
///
/// Convenience form for a caller that compiles and publishes without
/// yielding. A caller that does file I/O between must build the candidate with
/// the pack fingerprint it read.
#[cfg(test)]
pub(crate) fn reload_move_tables_selecting(
    world: &mut bevy::ecs::world::World,
    fresh: std::sync::Arc<ambition_content_pack::PreparedContentPack>,
) -> MoveReload {
    let candidate = ambition_content_pack::CandidateGeneration::prepared_against(fresh, None);
    publish_candidate(world, candidate)
}

/// What a reload request did.
///
/// A request, not a publication (unlike [`publish_candidate`]). It reuses the
/// shell lifecycle: `ShellEvent::PreparationRequested` →
/// `prepare_requested_sessions` → `prepare_platformer_content`, which
/// allocates the `ContentEpoch` and fingerprints the whole content before it
/// publishes at activation. Re-requesting the active route starts a fresh
/// transaction (`start_route` has no same-route guard), and the old
/// generation stays authoritative until the new one activates.
#[derive(Debug, Clone, PartialEq)]
pub enum ReloadRequest {
    /// The pack on disk does not compile. Nothing was selected and nothing was
    /// requested.
    PackRefused(String),
    /// The candidate is mechanically identical to the selected pack (whole pack).
    /// Nothing was requested: a re-preparation would consume an epoch, a
    /// publication and a reconstruction for content that did not change.
    Unchanged,
    /// The shell is not active on any route, so there is nothing to re-prepare.
    NoActiveRoute,
    /// The active route declares no preparation plan, so re-requesting it would
    /// never reach `prepare_platformer_content`.
    ///
    /// Same precondition as the retry road
    /// (`ambition_load_presentation::shell_adapter`).
    RouteHasNoPreparation(String),
    /// A rollback timeline is speculating, or its authority is unhealthy. See
    /// [`MoveReload::RefusedDuringLiveTimeline`].
    Refused(MoveReload),
    /// The candidate is identical to the selected pack and a generation was in
    /// flight: the author changed their mind back. The in-flight generation was
    /// cancelled (`cancelled` is its request identity) and nothing replaces it,
    /// so the live content stays what it is.
    CancelledInFlight {
        cancelled: ambition_platformer2d::game_shell::ShellRequestId,
    },
    /// The request was issued. Nothing is published yet; the new generation
    /// appears when the shell activates it.
    Requested {
        route: String,
        /// The identity this call minted, so the caller can correlate the
        /// transaction it started.
        request: ambition_platformer2d::game_shell::ShellRequestId,
        /// The generation this request SUPERSEDED, when one was in flight: the
        /// newest admitted candidate wins, and the older one was cancelled
        /// (its staged cast, its hold, its claim and its shell transaction).
        /// `None` when nothing was in flight.
        superseded: Option<ambition_platformer2d::game_shell::ShellRequestId>,
    },
}

/// Ask the running host to re-prepare its session against `candidate`.
///
/// Nothing is selected at request time. [`stage_pending_generation`] stores
/// the candidate in `PendingGeneration` and changes nothing else;
/// `crate::pack::install_selection` runs only in
/// [`commit_content_generation`], at activation. The preparation reads its
/// identity from the transaction-local claim (`PendingGenerationInputs`,
/// set at adoption), not from the App-wide selection.
///
/// The caller mints a `ShellRequestId` before it issues
/// `ShellCommand::ReplaceWith { route, request }`. The id travels through
/// `PendingShellRoute` → `ProviderLoadTransaction` →
/// `ShellEvent::TransactionEnded`. A route name is not a transaction identity.
pub fn request_reload(
    world: &mut bevy::ecs::world::World,
    candidate: ambition_content_pack::CandidateGeneration,
) -> ReloadRequest {
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteCatalog, ShellRouter};

    // ⭐ ONE GENERATION IN FLIGHT, AND THE NEWEST ADMITTED CANDIDATE WINS. A
    // second request used to be refused (`AlreadyPending`) because overwriting
    // the in-flight state let a second generation adopt the first one's load.
    // Nothing is overwritten now: the in-flight generation is CANCELLED first,
    // by the road the publication-lease breaker already uses (its content half
    // dropped, then `ShellCommand::CancelPending`), and only then is the new
    // one staged. Adoption matches on the request identity, so the cancelled
    // transaction's late events (`PreparationRequested`, `TransactionEnded`) name
    // a request that is no longer pending and are ignored.
    //
    // A candidate that is refused at ADMISSION (stale, speculating timeline)
    // does NOT cancel what is in flight: an invalid request mutates nothing.
    // One refused AFTER that point (an unknown character, a catalog, the
    // quest book, the content graph) does cancel it: the candidate is the whole
    // on-disk state, so the generation in flight describes a disk that no longer
    // exists, and those refusals discard the staged reload it shares. Nothing is
    // pending afterwards and the live content is untouched.
    let in_flight = world
        .get_resource::<PendingGeneration>()
        .map(|pending| pending.request.clone());

    // Shared preflight; see [`admit_candidate`].
    match admit_candidate(world, &candidate) {
        CandidateAdmission::Refused(answer) => return ReloadRequest::Refused(answer),
        CandidateAdmission::Unchanged => {
            // Identical to what is live. With a generation in flight this is a
            // REVERT, and leaving the in-flight one would publish the edit the
            // author just undid.
            return match in_flight {
                Some(cancelled) => {
                    cancel_in_flight_generation(world, cancelled.clone());
                    ReloadRequest::CancelledInFlight { cancelled }
                }
                None => ReloadRequest::Unchanged,
            };
        }
        CandidateAdmission::Proceed => {}
    }

    let Some(route) = world
        .get_resource::<ShellRouter>()
        .and_then(|router| router.active.as_ref())
        .map(|active| active.route_id.clone())
    else {
        return ReloadRequest::NoActiveRoute;
    };
    let prepares = world
        .get_resource::<ShellRouteCatalog>()
        .and_then(|catalog| catalog.get(&route))
        .is_some_and(|spec| spec.preparation.is_some());
    if !prepares {
        return ReloadRequest::RouteHasNoPreparation(route.as_str().to_string());
    }

    // The request will go through to staging, so what is in flight stops now:
    // before the cast is staged (a second revision cannot stage over the first
    // one's) and before the refusal paths below, which discard the staged
    // reload and would otherwise throw away THIS pending generation's state
    // while the old one lived on.
    let superseded = in_flight;
    if let Some(cancelled) = &superseded {
        cancel_in_flight_generation(world, cancelled.clone());
    }

    // Stage the cast revision here and publish it at activation.
    // `register_characters` runs once in `Plugin::build`, so re-preparing a
    // session alone changes no move table. The transaction stages the cast
    // and requests the re-preparation, and
    // [`publish_staged_reload_on_activation`] lands both together.
    //
    // Only participants that changed prepare; see [`moveset_changed`].
    let pack = candidate.into_pack();
    let stages_cast = moveset_changed(world, &pack);
    let section = stages_cast
        .then(|| ambition_characters::moveset_content_schema::lowered_movesets(&pack))
        .flatten();
    if let Some(section) = section {
        // No base is passed: `admit_candidate` already refused a stale base.
        // See `MoveReload::StaleGeneration`.
        let problems = stage_move_section(world, section);
        if problems
            .iter()
            .any(|p| matches!(p, MovesetRevisionError::NoStagedCast))
        {
            return ReloadRequest::Refused(MoveReload::NoCast);
        }
        if !problems.is_empty() {
            return ReloadRequest::Refused(MoveReload::UnknownCharacters(
                problems.iter().map(ToString::to_string).collect(),
            ));
        }
    }
    // A character catalog edit re-stages the whole cast from the candidate
    // catalog, and the revision is admitted against that catalog.
    let character_catalog = match candidate_character_catalog(world, &pack) {
        Ok(catalog) => catalog,
        Err(reason) => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::CharacterCatalogRefused(reason));
        }
    };
    if let Some(catalog) = &character_catalog {
        if let Err(reason) = stage_cast_from_catalog(world, &pack, &catalog.assembled.catalog) {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::CharacterCatalogRefused(reason));
        }
    }
    // A fighter facet folds into its character's definition
    // (`pack_facets::fold_character_facets`), so a facet edit re-stages the cast
    // from the LIVE catalog and the candidate pack, the road a catalog edit
    // takes with the candidate catalog. A catalog edit in the same candidate has
    // already staged every character with the candidate's facets folded in.
    let facets_change = character_catalog.is_none() && fighter_facets_changed(world, &pack);
    if facets_change {
        let live = world
            .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
            .cloned();
        let staged = match live {
            Some(live) => stage_cast_from_catalog(world, &pack, &live),
            None => Err("this App has no character catalog to fold the facets into".to_string()),
        };
        if let Err(reason) = staged {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::CharacterCatalogRefused(reason));
        }
    }
    let stages_cast = stages_cast || character_catalog.is_some() || facets_change;
    // Stage as pending; do not install as the selection. Installing here let a
    // failed preparation leave the App on a pack whose cast it never built,
    // and the next save then compared equal and requested nothing.
    //
    // Admission finishes here, before the request, not at the commit. By
    // `RouteActivated` the shell has committed the new route and session, so a
    // refusal there would leave half a transaction. `admit_staged_revision`
    // takes `&World` and mutates nothing.
    //
    // `Unchanged` and `NothingStaged` both proceed: the pack can change while
    // the move material does not.
    //
    // A missing technique table refuses here (see
    // `MoveReload::NoTechniqueSupport`), but only when this generation stages a
    // cast. A generation that stages no cast does not consult techniques.
    let support = if stages_cast {
        match world
            .get_resource::<ambition_combat::technique::InstalledTechniques>()
            .map(|installed| installed.0.clone())
        {
            Some(support) => Some(support),
            None => {
                discard_staged_reload(world);
                return ReloadRequest::Refused(MoveReload::NoTechniqueSupport);
            }
        }
    } else {
        None
    };
    // Keep the admitted value. Re-admitting at activation could still refuse
    // on the commit path.
    //
    // Take it, do not borrow it: `admit_staged_revision` leaves edits staged,
    // and an unrelated publication could otherwise drain this reload's staged
    // revision.
    // The boss catalog, admitted here for the same reason as the cast.
    let bosses = match candidate_bosses(world, &pack) {
        Ok(bosses) => bosses,
        Err(error) => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::BossCatalogRefused(error));
        }
    };
    let audio = match candidate_audio(world, &pack) {
        Ok(audio) => audio,
        Err(error) => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::AudioCatalogRefused(error));
        }
    };
    let adaptive_cues = match candidate_adaptive_cues(world, &pack) {
        Ok(cues) => cues,
        Err(error) => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::AudioCatalogRefused(error));
        }
    };
    if let Err(error) = candidate_quest_book(world, &pack) {
        discard_staged_reload(world);
        return ReloadRequest::Refused(MoveReload::QuestBookRefused(error));
    }
    if let Err(error) = candidate_cutscene_ownership(world, &pack) {
        discard_staged_reload(world);
        return ReloadRequest::Refused(MoveReload::CutsceneOwnershipRefused(error));
    }
    if let Err(errors) = candidate_content_graph(world, &pack) {
        discard_staged_reload(world);
        return ReloadRequest::Refused(MoveReload::ContentGraphRefused(errors));
    }
    let admitted_cast = match support
        .map(|support| match &character_catalog {
            Some(catalog) => {
                ambition_characters::prepared::take_admitted_revision_with_catalog(world, &support, catalog)
            }
            None => ambition_characters::prepared::take_admitted_revision(world, &support),
        })
        .unwrap_or(ambition_characters::prepared::RevisionAdmission::NothingStaged)
    {
        ambition_characters::prepared::RevisionAdmission::Refused { refusals, .. } => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::Refused(
                refusals.iter().map(|r| r.detail.clone()).collect(),
            ));
        }
        ambition_characters::prepared::RevisionAdmission::Admitted(admitted) => Some(admitted),
        // Nothing to publish for the cast is not a refusal: the move material
        // was identical, or this generation changes no moveset. The engine's
        // generation still has to move.
        ambition_characters::prepared::RevisionAdmission::NothingStaged
        | ambition_characters::prepared::RevisionAdmission::Unchanged { .. } => None,
    };
    // Mint the identity before writing the command, so the generation owns it
    // and does not adopt whatever the router announces for its route. The
    // route makes it readable in logs; the counter makes it unique.
    let request = ambition_platformer2d::game_shell::ShellRequestId::new(format!(
        "reload.{}.{}",
        route.as_str(),
        next_request_ordinal()
    ));
    stage_pending_generation(
        world,
        PendingGeneration {
            load_id: None,
            route: route.as_str().to_string(),
            request: request.clone(),
            pack,
            admitted_cast,
            bosses,
            audio,
            adaptive_cues,
        },
    );
    // `ReplaceWith`, not `GoTo`: a reload is not navigation and must not push
    // a history entry.
    world.write_message(ShellCommand::ReplaceWith {
        route: route.clone(),
        request: Some(request.clone()),
    });
    ReloadRequest::Requested {
        route: route.as_str().to_string(),
        request,
        superseded,
    }
}

/// Cancel the generation in flight: the content half (its staged cast, its
/// claim and its activation hold) and then the shell's transaction. The same two
/// steps, in the same order, as [`break_the_publication_lease_when_the_boundary_closes`]:
/// the content half goes first and unconditionally, because the shell's answer
/// can race with the transaction ending this frame, and `CancelPending` does
/// nothing when it already did.
fn cancel_in_flight_generation(
    world: &mut bevy::ecs::world::World,
    request: ambition_platformer2d::game_shell::ShellRequestId,
) {
    discard_staged_reload(world);
    world.write_message(ambition_platformer2d::game_shell::ShellCommand::CancelPending { request });
}

/// One pending generation: the transaction, the candidate, and the admitted
/// cast.
///
/// One resource, not several cooperating ones. A generation that owns its
/// admitted value cannot have it re-derived at the boundary, stranded when
/// `InstalledTechniques` is absent, overwritten by a second request, or
/// drained by an unrelated publication.
///
/// So the commit path cannot fail: publishing is
/// [`publish_admitted_revision`] plus an install. The only branch at the
/// boundary is "is this my transaction".
///
/// There are two names for one transaction. `request` is minted by the caller
/// before the command and covers the window before adoption. `load_id` is the
/// router's name; `ShellRouter::next_load_transaction` is private and mints it
/// in a later system, so it stays `None` until it is adopted from
/// `ShellEvent::PreparationRequested`.
#[derive(bevy::prelude::Resource)]
pub struct PendingGeneration {
    /// `None` until the router mints the transaction this reload asked for.
    load_id: Option<ambition_platformer2d::load::LoadId>,
    /// The route the request named, for reporting and for
    /// `ReloadRequest::AlreadyPending`. Not the correlator.
    route: String,
    /// This generation's request identity, minted before the command was
    /// written. See [`ambition_platformer2d::game_shell::ShellRequestId`].
    ///
    /// Adoption matches on this, not on the route: two `ReplaceWith("game")`
    /// in one frame mint two transactions and the second supersedes the
    /// first, so a route match could adopt the wrong or a cancelled one.
    request: ambition_platformer2d::game_shell::ShellRequestId,
    /// The candidate pack. Not the App's selection until activation, so a
    /// failed preparation leaves readers on the content the live cast uses.
    pack: std::sync::Arc<ambition_content_pack::PreparedContentPack>,
    /// The value admission computed. `None` means the candidate changed no
    /// move material; the engine's generation still has to move.
    admitted_cast: Option<ambition_characters::prepared::AdmittedRevision>,
    /// The boss catalog this generation publishes; `None` when it changes no
    /// boss domain.
    bosses: Option<CandidateBosses>,
    /// The audio catalog this generation publishes; `None` when it changes no
    /// audio domain.
    audio: Option<ambition_audio::catalog::AudioCatalogRegistry>,
    /// The adaptive music catalog this generation publishes; nothing when it
    /// changes no cue domain. Preparation sees only which providers have cues
    /// (`PendingGenerationInputs::adaptive_providers`); the registry itself is
    /// published at the commit.
    adaptive_cues: AdaptiveCues,
}

impl PendingGeneration {
    /// This transaction's request identity, which names its activation hold.
    /// Lets a test assert the hold carries this id.
    #[doc(hidden)]
    pub fn request_for_tests(&self) -> ambition_platformer2d::game_shell::ShellRequestId {
        self.request.clone()
    }
}

/// End a pending generation the way every content-side terminal path does.
///
/// Test-facing, and the same function the production paths call, so a test
/// also covers the hold release.
#[doc(hidden)]
pub fn take_pending_generation_for_tests(world: &mut bevy::ecs::world::World) {
    take_pending_generation(world);
}

/// A process-unique ordinal for a reload's request identity.
///
/// A counter, not a route name (ambiguous) or a clock (can repeat, not
/// reproducible). It is unique for the life of the process.
fn next_request_ordinal() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The candidate a re-preparation is about, if one is in flight.
pub fn pending_pack(
    world: &bevy::ecs::world::World,
) -> Option<&ambition_content_pack::PreparedContentPack> {
    world
        .get_resource::<PendingGeneration>()
        .map(|generation| generation.pack.as_ref())
}

/// Stage a pending generation. The App's selection is not changed.
///
/// Do not write `SelectedContentIdentity` here: unrelated route preparations
/// in the pending window would fingerprint against the candidate, and the
/// rollback timeline contract compares that identity. The claim is made at
/// adoption, keyed on the `LoadId`. Before adoption there is no claim, which
/// is correct.
fn stage_pending_generation(world: &mut bevy::ecs::world::World, generation: PendingGeneration) {
    world.insert_resource(generation);
}

/// Take the pending generation away, and its identity claim with it.
///
/// The claim is always removed too. A left-over claim would make a later
/// transaction with the same `LoadId` fingerprint against a discarded
/// candidate.
fn take_pending_generation(world: &mut bevy::ecs::world::World) -> Option<PendingGeneration> {
    let generation = world.remove_resource::<PendingGeneration>()?;
    world.remove_resource::<ambition_platformer2d_runtime::PendingGenerationInputs>();
    // Every content-side terminal path (cancel, supersede, refuse, discard)
    // ends here, so release this transaction's hold. A left-over hold blocks
    // every future reload of the route. Release by the transaction's own id so
    // a successor's hold on the same route (`ShellRouteHolds` is
    // `route → set<hold id>`) stays.
    //
    // The activation path releases in the shell, in the exclusive operation
    // that consumes the gate.
    release_the_publication_hold(world, &generation.request, &generation.route);
    Some(generation)
}

/// Drop one transaction's activation hold and its gate registration.
fn release_the_publication_hold(
    world: &mut bevy::ecs::world::World,
    request: &ambition_platformer2d::game_shell::ShellRequestId,
    route: &str,
) {
    let hold = publication_hold_for(request);
    if let Some(mut holds) =
        world.get_resource_mut::<ambition_platformer2d::game_shell::ShellRouteHolds>()
    {
        holds.release(
            &ambition_platformer2d::game_shell::ShellRouteId::new(route.to_string()),
            &hold,
        );
    }
    if let Some(mut gates) =
        world.get_resource_mut::<ambition_platformer2d::game_shell::ShellActivationGates>()
    {
        gates.forget(&hold);
    }
}

/// Install the reload transaction's publication half.
///
/// The run condition is part of the installation, so it is stated here once.
/// A composition without a game shell never registers `ShellEvent`, and a
/// `MessageReader` for an unregistered message fails parameter validation and
/// panics the schedule. The tests in this crate all register `ShellEvent`, so
/// only the default-feature run shows a missing condition. A host with no shell
/// has no activation boundary, so these systems do not run there.
pub fn register(app: &mut bevy::prelude::App) {
    use bevy::prelude::IntoScheduleConfigs;
    // Register the activation gate's evaluator once. The shell runs it in the
    // exclusive operation that emits `RouteActivated`; see
    // [`answer_the_publication_gate`]. Each transaction takes its hold at
    // adoption, against this evaluator.
    let evaluator = app.world_mut().register_system(answer_the_publication_gate);
    app.insert_resource(PublicationGateEvaluator(evaluator));
    // One condition for all systems here, so they cannot disagree about
    // whether the shell exists.
    let shell_is_installed = bevy::prelude::resource_exists::<
        bevy::ecs::message::Messages<ambition_platformer2d::game_shell::ShellEvent>,
    >;
    app.add_systems(
        bevy::prelude::Update,
        adopt_preparation_transaction
            // Before the preparation that reads the identity claim. This
            // system sets the claim from `ShellEvent::PreparationRequested`, and
            // `prepare_requested_sessions` reads the same message and
            // fingerprints against it. A late claim makes the preparation fall
            // back to the App's identity and stamp N+1's session with N.
            .before(ambition_platformer2d::provider::PlatformerPreparationSet)
            .run_if(shell_is_installed),
    )
    .add_systems(
        bevy::prelude::Update,
        commit_content_generation
            // After the set that produces `RouteActivated`; otherwise the commit
            // reads the activation a frame late. See
            // [`commit_content_generation`].
            .after(ambition_platformer2d::game_shell::AmbitionGameShellSet::Pending)
            // Before `Providers`, which orders the commit against
            // `adopt_candidate_platformer_session`. This edge no longer means
            // "before the world is built from the cast": construction happens
            // in `prepare_candidate_platformer_session`, before `Pending`, and
            // the commit must run after `Pending`. So a candidate is prepared
            // from the generation current before its activation commits. If
            // that is correct is an open question in `docs/planning/queue.md`.
            .before(ambition_platformer2d::game_shell::GameplaySessionSet::Providers)
            .run_if(shell_is_installed),
    )
    .add_systems(
        bevy::prelude::Update,
        break_the_publication_lease_when_the_boundary_closes
            // Before the commit it prevents: the pending generation is already
            // gone when `commit_content_generation` looks for it.
            .before(commit_content_generation)
            // Before `AmbitionGameShellSet::Commands`, where
            // `process_shell_commands` reads `ShellCommand::CancelPending`.
            // Without this edge the router can run first and the route
            // activates at N+1 while the content stays at N.
            //
            // This edge is not the full guarantee (see `Q118`): a boundary that
            // closes after this frame's breaker still activates. It only makes
            // sure the cancel does not miss the phase that reads it.
            .before(ambition_platformer2d::game_shell::AmbitionGameShellSet::Commands)
            .run_if(shell_is_installed),
    );
    // A build that reads its content off disk plays an edit of it. See
    // `crate::content_watch`. Before `Commands`, which reads the
    // `ReplaceWith` a request writes.
    #[cfg(not(feature = "static_content"))]
    {
        app.insert_resource(crate::content_watch::ContentSourceWatch::new(crate::pack::source_root()));
        app.add_systems(
            bevy::prelude::Update,
            crate::content_watch::watch_content_sources
                .before(break_the_publication_lease_when_the_boundary_closes)
                .before(ambition_platformer2d::game_shell::AmbitionGameShellSet::Commands)
                .run_if(shell_is_installed),
        );
        // The dialogue: the running Yarn project's own files. See
        // `crate::content_watch::yarn`. Only where a project is loaded.
        #[cfg(feature = "ui")]
        {
            app.insert_resource(crate::content_watch::YarnSourceWatch::new(
                crate::pack::source_root(),
                crate::dialogue::yarn::yarn_sources().iter().map(|(name, _)| (*name).to_string()),
            ));
            app.add_systems(
                bevy::prelude::Update,
                crate::content_watch::watch_yarn_sources
                    .run_if(bevy::prelude::resource_exists::<bevy_yarnspinner::prelude::YarnProject>),
            );
        }
    }
}

/// The hold id that blocks this transaction's route until publication is
/// legal.
///
/// It must be specific to the transaction. `ShellRouteHolds` is keyed
/// `route → set<hold id>`, and a reload re-prepares the current route. With a
/// constant id, a late terminal event from transaction A could free successor
/// B's hold, or a leaked A hold could block every later reload.
fn publication_hold_for(
    request: &ambition_platformer2d::game_shell::ShellRequestId,
) -> ambition_platformer2d::game_shell::ShellHoldId {
    ambition_platformer2d::game_shell::ShellHoldId::new(format!(
        "content-publication:{}",
        request.as_str()
    ))
}

/// The activation gate (`Q118`).
///
/// The shell runs this in the same exclusive operation that emits
/// `RouteActivated`, so it reads the boundary the activation happens under. A
/// hold released on an earlier check would leave a gap.
///
/// The mapping is `publication_boundary`'s. `Legal` and `RebasableTimeline`
/// are admitted, as in `admit_candidate`. `Unhealthy` is refused (publishing
/// would hide a recorded desync). `ForeignTimeline` is refused (this host
/// cannot rebase a timeline it does not own).
pub fn answer_the_publication_gate(
    world: &mut bevy::ecs::world::World,
) -> ambition_platformer2d::game_shell::ShellGateVerdict {
    use ambition_platformer2d::game_shell::ShellGateVerdict;
    match publication_boundary(world) {
        PublicationBoundary::Legal | PublicationBoundary::RebasableTimeline => {
            // ⭐ THE SAVE MOVES WHILE A GENERATION WAITS. The quest book was
            // admitted at request time against the save as it was, and the next
            // session rebuilds the registry from the save as it is at the
            // activation. Asked again here, with the same function, so a player
            // who reached a step the candidate no longer has cancels the
            // generation instead of being clamped by the rebuild. The selection
            // is still the live pack at this point, which is what the question
            // compares the candidate against.
            let verdict = world.get_resource::<PendingGeneration>().map(|pending| {
                candidate_quest_book(world, &pending.pack)
                    .and_then(|()| candidate_cutscene_ownership(world, &pending.pack))
            });
            if let Some(Err(why)) = verdict {
                bevy::log::warn!(
                    target: "ambition_content::reload",
                    "the route was refused at its activation: the save moved while \
                     the transaction was in flight ({why})."
                );
                return ShellGateVerdict::Refuse;
            }
            ShellGateVerdict::Admit
        }
        PublicationBoundary::Unhealthy(detail) => {
            bevy::log::warn!(
                target: "ambition_content::reload",
                "the route was refused at its activation: the rollback authority \
                 recorded a divergence while the transaction was in flight \
                 ({detail}). Publishing across it would launder the desync."
            );
            ShellGateVerdict::Refuse
        }
        PublicationBoundary::ForeignTimeline => {
            bevy::log::warn!(
                target: "ambition_content::reload",
                "the route was refused at its activation: the rollback timeline \
                 stopped being one this host may rebase while the transaction was \
                 in flight."
            );
            ShellGateVerdict::Refuse
        }
    }
}

/// The one registered evaluator, reused for every transaction's hold id.
///
/// `ShellActivationGates` maps a hold id to an evaluator. Every
/// transaction's hold uses this one system, so the answer lives in one place
/// and each block stays transaction-specific.
#[derive(bevy::prelude::Resource, Clone, Copy)]
pub struct PublicationGateEvaluator(
    pub  bevy::ecs::system::SystemId<(), ambition_platformer2d::game_shell::ShellGateVerdict>,
);

/// Break the publication authorization when the boundary that granted it
/// closes (`Q118`).
///
/// `admit_candidate` checks [`publication_boundary`] once, but the generation
/// then waits in [`PendingGeneration`] until `RouteActivated`, and
/// `commit_content_generation` asks nothing. The shipped host orders
/// `LocalSessionSet::Maintain` after the commit on one frame
/// (`the_rollback_session_start_is_ordered_after_the_generation_commit`), but
/// a session that starts on an earlier frame of the wait is not held by an
/// edge, so the boundary can change in between.
///
/// This cancels the whole shell transaction instead of refusing at the commit.
/// A fallible content half at the commit would activate the route at N+1 with
/// the cast at N. Cancelling early ends both halves.
///
/// It re-asks the same `publication_boundary` function; only the time is new.
///
/// ⭐ WHAT THIS OWNS BESIDE THE ACTIVATION GATE, measured 2026-10-05
/// (`an_edit_reaches_the_shipped_game::a_reload_whose_boundary_closes_while_it_waits_is_cancelled_whole`,
/// with this system removed and then the gate removed). The shell asks
/// [`answer_the_publication_gate`] on each frame that the route is ready but
/// for its holds. A refusal there cancels the route, and `TransactionEnded`
/// discards the staged generation, so the gate ends a transaction whose
/// boundary is closed on such a frame with the same result as this system.
/// With this system removed those arms do not change.
///
/// This system is the one owner of the frames before the route is ready: a
/// boundary that closes after the adoption, while the route still prepares.
/// If it is open again before the route is ready, the gate does not see it
/// and the reload publishes. If it stays closed, the gate cancels at the ready
/// frame and not on the frame after the close. That is the lease of the
/// `Q118` ruling: the admission holds for the life of the transaction, and
/// the gate asks again at the activation. Do not remove this system because a
/// test of a closed boundary stays green without it; run that arm.
pub fn break_the_publication_lease_when_the_boundary_closes(
    world: &mut bevy::ecs::world::World,
) {
    let Some(pending) = world.get_resource::<PendingGeneration>() else {
        return;
    };
    // Only a transaction the router has minted. Before adoption there is
    // nothing to cancel.
    if pending.load_id.is_none() {
        return;
    }
    let request = pending.request.clone();
    let boundary = publication_boundary(world);
    // Break only on `Unhealthy` or `ForeignTimeline`. A healthy, speculating
    // timeline this host may rebase is the normal state of the running game
    // (a reload re-prepares the current route), and the stop-and-rebase
    // lifecycle handles it. Breaking on it would disable hot reload.
    //
    // `Unhealthy` means a divergence was recorded, and publishing must not
    // hide it (`publishing_does_not_heal_an_unhealthy_rollback_authority`).
    let detail = match &boundary {
        // The normal frame: nothing to do.
        PublicationBoundary::Legal | PublicationBoundary::RebasableTimeline => return,
        PublicationBoundary::Unhealthy(detail) => format!(
            "the rollback authority recorded a divergence while its shell \
             transaction was in flight ({detail})"
        ),
        // Admission let the generation past a healthy timeline only because
        // this host may rebase it. If an `External`/P2P or `Caller`-owned
        // session replaced it, `rebase_local_timeline_onto_the_new_generation`
        // correctly does nothing, and publishing would desync. Ownership and
        // health are one enum value, so no caller can check only half.
        PublicationBoundary::ForeignTimeline => {
            "the rollback timeline stopped being one this host may rebase while \
             the transaction was in flight"
                .to_string()
        }
    };
    // Drop the content half first and unconditionally. The shell's response
    // to the cancel can race with the transaction ending this frame.
    take_pending_generation(world);
    bevy::log::warn!(
        target: "ambition_content::reload",
        "the pending content generation was cancelled: {detail}. Publishing \
         across it would either launder a recorded desync or hand new content \
         to a timeline this host is not permitted to rebase."
    );
    world.write_message(
        ambition_platformer2d::game_shell::ShellCommand::CancelPending { request },
    );
}


/// Adopt the transaction the router mints for this reload's request.
///
/// This is the first moment the transaction id exists and can be observed:
/// `ShellRouter::next_load_transaction` is private and mints the id in a later
/// system than the request.
///
/// This does not publish. [`commit_content_generation`] does, at the other end
/// of the frame; see its doc for why they are separate systems.
pub fn adopt_preparation_transaction(
    mut events: bevy::ecs::message::MessageReader<ambition_platformer2d::game_shell::ShellEvent>,
    mut commands: bevy::prelude::Commands,
) {
    use ambition_platformer2d::game_shell::ShellEvent;
    for event in events.read() {
        match event {
            // Adopt the transaction the router just minted.
            ShellEvent::PreparationRequested(transaction) => {
                // Match on the request identity, not the route. A transaction
                // with no request id is not ours; `None` is not a wildcard.
                let Some(requested_by) = transaction.request.clone() else {
                    continue;
                };
                let load_id = transaction.barrier.load_id.clone();
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    let claim = {
                        let Some(mut pending) = world.get_resource_mut::<PendingGeneration>()
                        else {
                            return;
                        };
                        if pending.load_id.is_some() || pending.request != requested_by {
                            return;
                        }
                        pending.load_id = Some(load_id.clone());
                        // Stake the candidate cast with the identity in one
                        // claim. `admitted_cast` holds the N+1 registry, which
                        // the App does not see until the commit, so the
                        // preparation needs it here to find its fighters.
                        //
                        // Clone, do not move: the commit consumes
                        // `admitted_cast`.
                        (
                            crate::pack::identity_line(&pending.pack),
                            pending
                                .admitted_cast
                                .as_ref()
                                .map(|admitted| admitted.candidate().clone()),
                            pending.bosses.as_ref().map(|bosses| bosses.catalog.clone()),
                            // And the catalog that cast is folded from: the
                            // preparation reads raw catalog facts too.
                            pending
                                .admitted_cast
                                .as_ref()
                                .and_then(|admitted| admitted.candidate_catalog().cloned()),
                            // And the audio registry, which preparation asks
                            // which providers have music and SFX.
                            pending.audio.clone(),
                            // And which providers have adaptive cues.
                            adaptive_providers_of(&pending.adaptive_cues),
                        )
                    };
                    // Hold the route from adoption. Only the gate's answer at
                    // activation releases it. See [`publication_hold_for`].
                    if let Some(evaluator) = world
                        .get_resource::<PublicationGateEvaluator>()
                        .map(|evaluator| evaluator.0)
                    {
                        let hold = publication_hold_for(&requested_by);
                        let route = world
                            .get_resource::<PendingGeneration>()
                            .map(|pending| pending.route.clone());
                        if let Some(route) = route {
                            let route_id =
                                ambition_platformer2d::game_shell::ShellRouteId::new(route);
                            if let Some(mut gates) = world
                                .get_resource_mut::<ambition_platformer2d::game_shell::ShellActivationGates>(
                                ) {
                                gates.register(hold.clone(), evaluator);
                            }
                            if let Some(mut holds) = world
                                .get_resource_mut::<ambition_platformer2d::game_shell::ShellRouteHolds>(
                                ) {
                                holds.hold(route_id, hold);
                            }
                        }
                    }
                    let (claim, characters, bosses, catalog, audio, adaptive_providers) = claim;
                    // The only place the claim is made: the transaction first
                    // has a name here.
                    world.insert_resource(ambition_platformer2d_runtime::PendingGenerationInputs {
                        load_id: load_id.to_string(),
                        identity: claim,
                        characters,
                        bosses,
                        catalog,
                        audio,
                        adaptive_providers,
                    });
                });
            }
            // All other events belong to the commit system. They are listed,
            // not wildcarded, so a new event is a compile error.
            ShellEvent::RouteActivated(_)
            | ShellEvent::ExperienceFailed { .. }
            | ShellEvent::CommandRejected(_)
            | ShellEvent::TransactionEnded { .. }
            | ShellEvent::WaitingForLoad { .. }
            | ShellEvent::RouteDeactivated(_)
            | ShellEvent::ExitRequested => {}
        }
    }
}


/// Commit the generation when the shell activates the transaction it was
/// requested for — and discard it if that transaction failed instead.
///
/// This is a separate system from [`adopt_preparation_transaction`] because
/// the two need opposite ends of the frame. Adoption must run before
/// `PlatformerPreparationSet` (in `AmbitionLoadSet::Contributors`), but
/// `advance_pending_route` emits `RouteActivated` later, in
/// `AmbitionGameShellSet::Pending`. One system could only see the activation
/// on the next frame, after N+1's world was built from N's
/// `PreparedCharacterRegistry`. The needed order is
/// `Pending → commit → Providers`; see [`register`].
///
/// This is the boundary the two halves share. The shell's activation publishes
/// the engine's generation (epoch, content fingerprint, rollback contract);
/// this publishes the cast and every pack-derived family.
///
/// A failed preparation discards the staged revision, so the next activation
/// does not apply it.
pub fn commit_content_generation(
    mut events: bevy::ecs::message::MessageReader<ambition_platformer2d::game_shell::ShellEvent>,
    mut commands: bevy::prelude::Commands,
) {
    use ambition_platformer2d::game_shell::ShellEvent;
    for event in events.read() {
        match event {
            // Adoption runs earlier in the frame; see
            // [`adopt_preparation_transaction`].
            ShellEvent::PreparationRequested(_) => {}
            ShellEvent::RouteActivated(active) => {
                // Only the activation of this reload's transaction.
                // `load_authorization` is the barrier the router authorized it
                // against; an unrelated activation carries another one or none.
                let authorized = active
                    .load_authorization
                    .as_ref()
                    .map(|barrier| barrier.load_id.clone());
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    if !reload_owns(world, authorized.as_ref()) {
                        return;
                    }
                    // Nothing on the commit path can refuse. The verdict,
                    // changed domains, rollback boundary, authored effects and
                    // cast base were all answered at request time; this value
                    // carries the answers. Re-admitting here could leave the App
                    // and engine at N+1 with the cast at N.
                    let Some(generation) = take_pending_generation(world) else {
                        return;
                    };
                    if let Some(admitted) = generation.admitted_cast {
                        let outcome = ambition_characters::prepared::publish_admitted_revision(
                            world, admitted,
                        );
                        bevy::log::info!("a reloaded cast was published: {outcome:?}");
                    }
                    if let Some(bosses) = generation.bosses {
                        world.insert_resource(bosses.registry);
                        world.insert_resource(bosses.catalog);
                    }
                    if let Some(audio) = generation.audio {
                        publish_audio(world, audio);
                    }
                    publish_adaptive_cues(world, generation.adaptive_cues);
                    // Every other participating family lands here too, from
                    // the same pack, in the same command.
                    publish_participant_families(world, &generation.pack);
                    // The pack lands at the same boundary, unconditionally.
                    crate::pack::install_selection(world, generation.pack);
                    // Rebase the timeline in the same step. Publishing across
                    // a speculating timeline desyncs the sync-test canary
                    // (`Q118`); stopping the baseline makes the next session
                    // start on the live generation. See
                    // `rebase_local_timeline_onto_the_new_generation`.
                    rebase_local_timeline_onto_the_new_generation(world);
                });
            }
            // Every way the request can end without activating. Listed, not
            // wildcarded, so a new terminal event is a compile error here.
            //
            // `TransactionEnded` names its owner, so supersession and failure
            // do not strand a pending generation.
            ShellEvent::TransactionEnded {
                barrier, request, ..
            } => {
                let load_id = barrier.load_id.clone();
                let request = request.clone();
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    // `reason` is not read: any end of this transaction means
                    // it will never activate, so discard. A new `TransactionEnd`
                    // variant is then handled correctly. A caller that must
                    // tell them apart (retry on `Failed`, not on `Superseded`)
                    // reads it.
                    //
                    // Either identity proves ownership: `request` matches before
                    // adoption, `load_id` after.
                    if !reload_issued(world, request.as_ref())
                        && !reload_owns(world, Some(&load_id))
                    {
                        return;
                    }
                    discard_staged_reload(world);
                });
            }
            // The one rejection that names a barrier: the commit succeeded but
            // no prepared session existed, so this transaction will never
            // activate.
            ShellEvent::CommandRejected(
                ambition_platformer2d::game_shell::ShellCommandRejection::PreparedSessionUnavailable(
                    barrier,
                ),
            ) => {
                let load_id = barrier.load_id.clone();
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    if !reload_owns(world, Some(&load_id)) {
                        return;
                    }
                    discard_staged_reload(world);
                });
            }
            // Ignored. `LoadFailed` carries no load id and `ExperienceFailed`
            // carries an activation id, so they cannot be matched to this
            // reload. Discarding when our load happens to be in flight would
            // drop an edit over an unrelated rejection. Every way this
            // transaction dies emits `TransactionEnded` (from `start_route`
            // or `advance_pending`).
            ShellEvent::ExperienceFailed { .. } | ShellEvent::CommandRejected(_) => {}
            ShellEvent::WaitingForLoad { .. }
            | ShellEvent::RouteDeactivated(_)
            | ShellEvent::ExitRequested => {}
        }
    }
}


/// Did this reload issue the request `request` names?
///
/// This works before the router has minted a load, which [`reload_owns`]
/// cannot do. A superseding command lands in that window.
///
/// `None` is not a match.
fn reload_issued(
    world: &bevy::ecs::world::World,
    request: Option<&ambition_platformer2d::game_shell::ShellRequestId>,
) -> bool {
    let Some(pending) = world.get_resource::<PendingGeneration>() else {
        return false;
    };
    request.is_some_and(|request| &pending.request == request)
}

/// Does the pending reload own the transaction `load_id` names?
///
/// An unadopted pending reload owns nothing yet, so an activation before
/// `PreparationRequested` is ignored. A world with no pending reload also owns
/// nothing, so the direct `publish_candidate` road does not take this road's
/// boundaries.
fn reload_owns(
    world: &bevy::ecs::world::World,
    load_id: Option<&ambition_platformer2d::load::LoadId>,
) -> bool {
    let Some(pending) = world.get_resource::<PendingGeneration>() else {
        return false;
    };
    match (&pending.load_id, load_id) {
        (Some(mine), Some(theirs)) => mine == theirs,
        _ => false,
    }
}

/// Throw away a staged cast revision whose preparation never activated.
fn discard_staged_reload(world: &mut bevy::ecs::world::World) {
    // Discard the pending pack as well. If only the cast revision is dropped,
    // the next identical save compares equal and requests nothing.
    let had_pending = take_pending_generation(world).is_some();
    if world
        .remove_resource::<ambition_characters::prepared::StagedCastRevision>()
        .is_some()
        || had_pending
    {
        bevy::log::warn!(
            "a requested reload did not activate, so its staged cast revision was \
             discarded rather than left for the next activation to apply"
        );
    }
}

#[cfg(test)]
#[path = "reload_tests.rs"]
mod reload_tests;
