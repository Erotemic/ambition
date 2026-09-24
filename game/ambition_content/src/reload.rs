//! Reload every move table from disk into a running host.
//!
//! This is the production caller of the staged cast revision road
//! (`stage_move_section`, admission, publication).
//!
//! It does not use `pack::prepared()`. That function is a `OnceLock` that
//! serves the boot-time pack forever. Reload compiles a fresh pack from disk.
//! Readers that still use the `OnceLock` keep the boot-time value; moving them
//! to an App-scoped selection is fast-iteration I3 step 1 and is not done here.
//!
//! A reload publishes the selected pack together with every participating
//! domain: see [`participates`] (the moveset schema plus
//! [`PACK_DERIVED_FAMILIES`]). [`ReloadRequest`] refuses a candidate that
//! changes any other domain (items, audio, boss profiles, character catalog),
//! because the canonical identity would then name generation N+1 while that
//! catalog still serves N. A family added later is refused by default.

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
    /// keep a list here. Domains installed in `AmbitionContentPlugin::build`
    /// from `pack::prepared()` (`item_catalog`, `character_catalog`, the audio
    /// registries, the boss families) do not participate, because no reload
    /// road replaces them. An items-only candidate would make
    /// `PreparedContentIdentity` name N+1 while the item catalog serves N.
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
    let pack = candidate.into_pack();
    let outcome = reload_move_tables_from(world, &pack);
    // The selection follows the cast's admission, not the compile. A refused
    // or stale revision leaves the App on the pack its cast was built from.
    if matches!(
        outcome,
        MoveReload::Activated { .. } | MoveReload::Unchanged { .. }
    ) {
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
///
/// Items cannot participate. `install_item_catalog` writes a process-global
/// `OnceLock`, and its readers return `&'static str`, so the type cannot
/// express a generation N+1.
fn participates(domain: &str) -> bool {
    domain == ambition_characters::moveset_content_schema::MOVESET_SCHEMA
        || PACK_DERIVED_FAMILIES
            .iter()
            .any(|family| family.domain == domain)
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
];

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
    /// A generation is already in flight, so this one was refused.
    ///
    /// Stated refusal, not last-write-wins. Overwriting the in-flight state let
    /// a second generation adopt the first request's `LoadId`, and a file
    /// watcher makes close saves common. Supersession through a real
    /// cancellation would also be valid, but is not implemented.
    AlreadyPending { route: String },
    /// The request was issued. Nothing is published yet; the new generation
    /// appears when the shell activates it.
    Requested {
        route: String,
        /// The identity this call minted, so the caller can correlate the
        /// transaction it started.
        request: ambition_platformer2d::game_shell::ShellRequestId,
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

    // One generation in flight at a time. Check first, before staging can
    // overwrite the first candidate.
    if let Some(pending) = world.get_resource::<PendingGeneration>() {
        return ReloadRequest::AlreadyPending {
            route: pending.route.clone(),
        };
    }

    // Shared preflight; see [`admit_candidate`].
    match admit_candidate(world, &candidate) {
        CandidateAdmission::Refused(answer) => return ReloadRequest::Refused(answer),
        CandidateAdmission::Unchanged => return ReloadRequest::Unchanged,
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

    // Stage the cast revision here and publish it at activation.
    // `register_declared_cast` runs once in `Plugin::build`, so re-preparing a
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
    let admitted_cast = match support
        .map(|support| ambition_characters::prepared::take_admitted_revision(world, &support))
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
    }
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
/// `commit_content_generation` asks nothing. The shipped schedule does not
/// order `LocalSessionSet::Maintain` against the commit
/// (`nothing_orders_the_rollback_session_start_against_the_generation_commit`),
/// so the boundary can change in between.
///
/// This cancels the whole shell transaction instead of refusing at the commit.
/// A fallible content half at the commit would activate the route at N+1 with
/// the cast at N. Cancelling early ends both halves.
///
/// It re-asks the same `publication_boundary` function; only the time is new.
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
                    let (claim, characters) = claim;
                    // The only place the claim is made: the transaction first
                    // has a name here.
                    world.insert_resource(ambition_platformer2d_runtime::PendingGenerationInputs {
                        load_id: load_id.to_string(),
                        identity: claim,
                        characters,
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
