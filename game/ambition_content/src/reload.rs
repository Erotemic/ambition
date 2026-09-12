//! Reload every move table from disk into a RUNNING host.
//!
//! ⭐⭐ **THIS IS THE CUSTOMER THE REVISION ROAD DID NOT HAVE.** MEASURED
//! 2026-09-11 before writing it: `activate_staged_revision` and
//! `stage_character_revision` had ZERO callers outside `prepared.rs` and its own
//! tests. A transactional cast revision — staged, admitted against installed
//! technique support, published under a new generation, refused without touching
//! the last-good cast, and now refused when it was prepared against a cast that
//! has since moved — existed complete and was reached by nothing. Same shape as
//! `EntityCatalogDoc`: the road was built and the traffic never arrived.
//!
//! ⛔⛔ **IT DOES NOT USE `pack::prepared()`, AND THAT IS THE POINT.** That
//! function is a `OnceLock`: the first caller compiles the pack and every later
//! caller gets the same value forever, which is correct for a process that reads
//! its content once and wrong for one that re-reads it. Reload compiles a FRESH
//! pack off disk, and the `OnceLock` keeps serving the boot-time value to
//! everything that has not been migrated to a generation-aware read — which is
//! fast-iteration I3 step 1's *"replace process-global mutable-generation
//! assumptions from the OnceLock content route for migrated families with
//! App-scoped selection"*, and is NOT done here.
//!
//! ⚠ **SO A RELOAD MOVES THE CAST AND NOT THE PACK.** Move tables reach the
//! running game through the character registry, which this republishes. Every
//! other family in that pack — items, encounters, audio, boss profiles — is
//! still whatever the boot-time compile produced, and asking this function to
//! reload one of them would be asking it to lie.

use ambition_characters::prepared::{stage_move_section, MovesetRevisionError};
// ⚠ THE FIXTURE ROAD PUBLISHES DIRECTLY; THE PRODUCTION ROAD NEVER DOES. The
// transaction's commit path holds an already-admitted value and calls
// `publish_admitted_revision`, which has no refusal in it — so the combined
// admit-and-publish entry point is reachable from tests only.
#[cfg(test)]
use ambition_characters::prepared::{activate_staged_revision, RevisionOutcome};
// ⚠ ONLY THE FIXTURE ROAD TAKES A CAST BASE AS A PARAMETER. `request_reload`
// reads the live generation itself, at the moment it stages, which is the only
// moment the claim can be true.
#[cfg(test)]
use ambition_characters::prepared::CharacterCatalogGeneration;

/// What a reload attempt did.
///
/// ⭐ EVERY VARIANT NAMES A STATE THE ROAD CAN ACTUALLY REACH, and each one that
/// is not `Activated` states what happened to the LIVE cast — because "the
/// reload failed" and "the reload failed and took the running game's fighters
/// with it" are the two outcomes a caller has to tell apart.
#[derive(Debug, Clone, PartialEq)]
pub enum MoveReload {
    /// The pack on disk does not compile. **Nothing was staged and the live cast
    /// is untouched** — the refusal carries the content compiler's own
    /// diagnostic, which names every problem rather than the first.
    PackRefused(String),
    /// The pack compiled and carries no move section at all.
    ///
    /// ⛔ NOT an error and NOT silence: a pack whose `moveset` sources were all
    /// removed is a legitimate edit, and replacing the live cast's movesets with
    /// nothing is not what it asks for. It is reported so a caller can tell it
    /// from a successful reload of zero changes.
    NoMoveSection,
    /// The preparation barrier has not run, so there is no cast to revise.
    ///
    /// ⛔⛤ **SEPARATE FROM `UnknownCharacters`, AND MY FIRST DRAFT MERGED
    /// THEM.** `stage_move_section` returns one error list, and mapping all of it
    /// to "unknown characters" would report a host that has not booted its cast
    /// as a CONTENT problem — sending an author to edit files over a lifecycle
    /// fact about the caller.
    NoCast,
    /// The section names characters this build did not prepare. Nothing was
    /// staged — `stage_move_section` is all-or-nothing.
    UnknownCharacters(Vec<String>),
    /// The cast was revised.
    Activated { generation: u64, changed: usize },
    /// Every table on disk is what the live cast was already built from.
    ///
    /// ⭐ A FILE WATCHER FIRES ON A SAVE, NOT ON A CHANGE, so this is the
    /// COMMON case in the loop this exists for, not an edge one.
    Unchanged { generation: u64 },
    /// The revision was refused at admission: an authored effect names a
    /// technique this composition did not install. The previous cast is still
    /// the published one.
    Refused(Vec<String>),
    /// The revision was prepared against a cast that is no longer live —
    /// another reload, or a tool, published in between. **Nothing was
    /// published**; the caller re-reads and tries again.
    Stale { prepared_against: u64, active: u64 },
    /// The CANDIDATE was prepared against a content generation that is no longer
    /// selected. **Nothing was published** — not the cast, not the selection.
    ///
    /// ⛔ SEPARATE FROM `Stale`, WHICH IS THE CAST'S CLOCK. That these are two
    /// variants is the honest report of a real defect in the architecture, not a
    /// design: I3 wants ONE generation identity, and until the epoch binding
    /// lands a caller can be stale against either clock independently.
    StaleGeneration {
        prepared_against: String,
        active: String,
    },
    /// A rollback timeline is LIVE over this world, so a content generation may
    /// not be published into it.
    ///
    /// ⭐⭐ **THE REFUSAL IS DERIVED, NOT INVENTED — and that is the whole point
    /// of it being a refusal rather than a new rule.** MEASURED 2026-09-11:
    /// `ambition_platformer2d_rollback_ggrs`'s per-frame contract check already
    /// INVALIDATES a live GGRS timeline when the prepared content identity
    /// changes under it (*"prepared content changed while the GGRS session was
    /// active"*), because a timeline promised the identity it rewinds. ⇒ Publish
    /// anyway and the architecture's own answer is a desync diagnosis; refusing
    /// is strictly better than publishing and being invalidated, and it is the
    /// explicit contract a remote session needs rather than behaviour that
    /// "mostly works locally".
    RefusedDuringLiveTimeline,
    /// The rollback authority governing this world is UNHEALTHY.
    ///
    /// ⛔⛔ **AND PUBLISHING MUST NOT HEAL IT.** `RollbackTimelineStatus::carried_from`
    /// hands an unhealthy timeline's reason to the timeline that replaces it, and
    /// the only sanctioned way to clear one is `acknowledge_and_clear` — *"a tool
    /// that has shown the divergence to a human and been told to carry on"*. A
    /// content publication that established a fresh timeline would otherwise
    /// launder a desync into health by a side door.
    RefusedWhileRollbackUnhealthy(String),
    /// The candidate changes a mechanical domain that does NOT participate in
    /// the generation transaction, so publishing it would make the engine's
    /// content identity a lie.
    ///
    /// ⛔⛤ **MEASURED 2026-09-11: ELEVEN OF TWELVE DOMAINS DO NOT PARTICIPATE.**
    /// `item_catalog`, `encounter_waves`, `fighter_brain_ladder`,
    /// `character_catalog`, the two audio registries and the four boss families
    /// are all installed in `AmbitionContentPlugin::build` or a registration
    /// function from the process-global `pack::prepared()`, and no reload road
    /// replaces any of them. `moveset` is the only domain whose reader takes a
    /// pack PARAMETER and the only one with a revision road — and those are the
    /// same fact, not two.
    ///
    /// ⇒ Publishing an items-only candidate would leave
    /// `PreparedContentIdentity` and the selected pack naming generation N+1
    /// while the live item catalog served N. That is WORSE than not supporting
    /// item reload: the canonical identity would claim mechanical content is
    /// active when it is not.
    ///
    /// ⭐ THE RULE FAILS SAFE. Everything except the participating domain is
    /// refused, so a family added later is refused by DEFAULT rather than
    /// forgotten into silent falsity. When a domain joins the transaction, the
    /// test that pins its refusal flips to pinning its publication — which is a
    /// much stronger validation of the abstraction than either arm alone.
    ///
    /// ⚠ SOUND ABOUT DOMAINS, NOT ABOUT FIELDS. The per-source digest this is
    /// built on is live for every domain (the compiler refuses a schema that
    /// lowers and defines nothing), but a handler can still define a row whose
    /// canonical string omits a field it lowered, and no compiler check sees
    /// that.
    RefusedUnsupportedChangedDomain(Vec<String>),
    /// This world installs no technique table at all.
    ///
    /// ⛔⛤ **ABSENT IS NOT EMPTY, AND MY FIRST VERSION CONFLATED THEM.** An
    /// EMPTY `TechniqueSupport` is a legitimate value — `activate_staged_revision`
    /// says so in its own signature — meaning *"this host installs nothing"*, and
    /// admitting against it correctly refuses every authored effect. A world with
    /// no `InstalledTechniques` resource has not installed the combat capability
    /// at all, and `unwrap_or_default()` there would report a roster-wide
    /// technique refusal for a composition that was never asked the question.
    NoTechniqueSupport,
}

/// Republish the cast's move tables from an ALREADY-COMPILED pack.
///
/// ⭐⭐ **SPLIT OUT SO THE WITNESS DOES NOT HAVE TO EDIT THE REPOSITORY.**
/// [`reload_move_tables`] reads the shipped asset tree, so a test of "an edited
/// file changes what the host plays" would have to write into
/// `game/ambition_content/assets` and put it back — a guard whose subject
/// MUTATES THE TREE, which this repository has already been bitten by. A caller
/// that can build a pack can build an EDITED one.
/// ⛔⛤ **`compiled_against` IS WHAT MAKES [`MoveReload::Stale`] REACHABLE, AND
/// FINDING THAT OUT COST A WITNESS.** My first version read the live generation
/// inside the staging road. Activation is the only thing that publishes after
/// the preparation barrier and it DRAINS the staged transaction atomically, so
/// a stamp taken at stage time always equalled the generation the fold landed
/// on — a refusal branch that could not fire, which is this repository's most
/// repeated instrument failure wearing a different hat.
///
/// ⇒ The generation that can disagree is the one the CALLER's pack was read
/// against, which only the caller knows: compiling a pack is file I/O, a reload
/// loop does it off the main thread, and the cast can move in between. `None`
/// makes no claim and is right for a caller that compiled and applied without
/// yielding.
#[cfg(test)]
pub(crate) fn reload_move_tables_from(
    world: &mut bevy::ecs::world::World,
    fresh: &ambition_content_pack::PreparedContentPack,
    compiled_against: Option<CharacterCatalogGeneration>,
) -> MoveReload {
    let Some(section) = ambition_characters::moveset_content_schema::lowered_movesets(fresh) else {
        return MoveReload::NoMoveSection;
    };

    let problems = stage_move_section(world, section, compiled_against);
    if problems
        .iter()
        .any(|p| matches!(p, MovesetRevisionError::NoStagedCast))
    {
        return MoveReload::NoCast;
    }
    if !problems.is_empty() {
        return MoveReload::UnknownCharacters(problems.iter().map(ToString::to_string).collect());
    }

    // ⚠ THE COMPOSITION OWNS THE SUPPORT TABLE. See `MoveReload::NoTechniqueSupport`
    // for why an absent resource is reported rather than defaulted.
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
        RevisionOutcome::Stale {
            prepared_against,
            active,
        } => MoveReload::Stale {
            prepared_against: prepared_against.get(),
            active: active.get(),
        },
        // ⚠ UNREACHABLE BY CONSTRUCTION rather than by assertion: the section is
        // non-empty (it lowered) and staging reported no problem, so something
        // is staged. Reported rather than panicked — a reload loop must not take
        // the game down over a shape it did not expect.
        RevisionOutcome::NothingStaged => MoveReload::NoMoveSection,
    }
}

/// Reload from a CONTENT DIRECTORY rather than from this build's own sources.
///
/// ⭐⭐ **THE FAST-ITERATION LOOP, END TO END, WITH NO COMPILER IN IT.** Edit a
/// `.ron` under `root`, call this, and the running host's cast plays the edit —
/// which is fast-iteration I2's acceptance in its own words: *"a prebuilt host
/// plays the edited artifact without invoking Cargo or its linker."*
///
/// ⛔ A ROOT SUPPLIES THE WHOLE PACK OR NONE OF IT. See
/// [`crate::pack::compile_pack_from`]: a per-file fallback to the binary's own
/// text would compile a mixed pack out of a directory and a build, and
/// [`crate::pack::export_sources_to`] is how a caller starts from a complete one.
#[cfg(test)]
pub(crate) fn reload_move_tables_from_dir(
    world: &mut bevy::ecs::world::World,
    root: &std::path::Path,
) -> MoveReload {
    // ⚠ THE GENERATION IS READ BEFORE THE FILE I/O, not after. That is the whole
    // reason the claim exists: reading a directory takes time, and anything that
    // publishes while we read it moves the cast under the pack we are building.
    let compiled_against = world
        .get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
        .map(ambition_characters::prepared::PreparedCharacterRegistry::generation);
    match crate::pack::compile_pack_from(root) {
        Ok(pack) => {
            reload_move_tables_selecting(world, std::sync::Arc::new(pack), compiled_against)
        }
        Err(refusal) => MoveReload::PackRefused(refusal),
    }
}

/// **May this candidate generation proceed at all?** — asked ONCE, for both
/// roads.
///
/// ⛔⛤ **THIS WAS SPELLED TWICE AND THAT WAS THE SECOND AUTHORITY.** The verdict,
/// the unsupported-domain diff and the publication boundary were written out in
/// `publish_candidate` and again in `request_reload`, in the same order, with
/// two different wrappers around the same answers. Two copies of a rule make
/// each other untestable: a change to one leaves the other green, and the road a
/// test takes decides which rule it certifies.
///
/// ⛔ **THE ORDER IS THE CONTRACT, AND IT IS THE HALF THAT WAS A DEFECT.** The
/// VERDICT comes first: a mechanically identical candidate publishes nothing,
/// allocates nothing and reconstructs nothing, so it cannot invalidate a
/// timeline — asking the boundary first reported a no-op as
/// `RefusedDuringLiveTimeline`, and a watcher fires on every SAVE, which makes
/// the no-op the common case. `Stale` does not need the boundary either: it is a
/// refusal about the candidate's own base, true whatever the timeline is doing.
/// Only `Publish` needs permission.
enum CandidateAdmission {
    /// It may not, and this is the answer BOTH roads report.
    Refused(MoveReload),
    /// There is nothing to do. Not a refusal — a real mechanical no-op.
    ///
    /// ⚠ IT CARRIES NOTHING. The cast generation a no-op reports is the DIRECT
    /// road's answer, read from the registry there; a request road that returns
    /// `Unchanged` requested nothing and has no generation to name.
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

    // ⛔ READ AGAINST THE ACTIVE PACK, WHICH IS WHY IT RUNS BEFORE ANYTHING IS
    // STAGED OR SELECTED: diffing after the candidate became the selection would
    // be diffing it against itself.
    let unsupported = unsupported_changed_domains(world, candidate.pack());
    if !unsupported.is_empty() {
        return CandidateAdmission::Refused(MoveReload::RefusedUnsupportedChangedDomain(
            unsupported,
        ));
    }

    match publication_boundary(world) {
        PublicationBoundary::Legal => CandidateAdmission::Proceed,
        PublicationBoundary::LiveTimeline => {
            CandidateAdmission::Refused(MoveReload::RefusedDuringLiveTimeline)
        }
        PublicationBoundary::Unhealthy(reason) => {
            CandidateAdmission::Refused(MoveReload::RefusedWhileRollbackUnhealthy(reason))
        }
    }
}

/// Publish a COMPLETE candidate generation, or refuse it, as one act.
///
/// ⭐⭐ **THE DECISION IS MADE ON THE WHOLE PACK'S IDENTITY, NOT ON THE MOVE
/// FAMILY'S.** [`ambition_content_pack::CandidateGeneration::verdict`] answers
/// stale / complete-no-op / publish from the pack's own `ContentFingerprint`,
/// which covers every content id, schema, capability, asset and reference. The
/// move family's own outcome is then a CONSEQUENCE of that decision rather than
/// an input to it.
///
/// ⛔⛤ **AND THAT IS THE FIX FOR A DEFECT I SHIPPED HOURS EARLIER.** This
/// function used to conclude `Unchanged` from the MOVE material and install the
/// whole newly-loaded pack anyway:
///
/// ```text
/// generation N   moves = A   items = X
/// candidate      moves = A   items = Y
/// → "Unchanged", and the whole candidate pack became the App's selection.
/// ```
///
/// One subsystem believing nothing changed while another can observe new
/// mechanical content. ⇒ It is NOT fixed by special-casing `Unchanged` — that
/// hides the missing abstraction. The complete identity is the abstraction.
///
/// ⚠ **A CHANGED PACK WHOSE MOVES ARE IDENTICAL NOW PUBLISHES THE SELECTION AND
/// LEAVES THE CAST GENERATION ALONE**, which is the correct pair of answers and
/// was not expressible before: the pack really did change, and the cast really
/// did not.
///
/// ⛔ **WHAT THIS STILL DOES NOT DO** (fast-iteration I3's remaining half): it
/// establishes no `ContentEpoch`, no `PreparedContentIdentity` and no rollback
/// timeline boundary, and it carries TWO base clocks — the pack fingerprint and
/// [`CharacterCatalogGeneration`] — where the architecture wants one. Both are
/// additions at this one seam rather than rewrites, which is why the decision
/// was moved here first.
#[cfg(test)]
pub(crate) fn publish_candidate(
    world: &mut bevy::ecs::world::World,
    candidate: ambition_content_pack::CandidateGeneration,
    cast_base: Option<CharacterCatalogGeneration>,
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
    let outcome = reload_move_tables_from(world, &pack, cast_base);
    // ⛔ THE SELECTION FOLLOWS THE CAST'S ADMISSION, not the compile. A refused
    // or stale revision must leave the App reading the pack its cast was
    // actually built from.
    if matches!(
        outcome,
        MoveReload::Activated { .. } | MoveReload::Unchanged { .. }
    ) {
        crate::pack::install_selection(world, pack);
    }
    outcome
}

/// The one mechanical domain that participates in the generation transaction.
///
/// ⛔ ONE ID RATHER THAN A PARTICIPATION TABLE. A table lists what participates
/// and is therefore wrong the moment somebody adds a family and forgets the row;
/// naming the participant refuses everything else by construction.
const PARTICIPATING_DOMAIN: &str = ambition_characters::moveset_content_schema::MOVESET_SCHEMA;

/// Domains this candidate changes that nothing can publish.
///
/// ⚠ READ AGAINST THE ACTIVE PACK, WHICH IS WHY IT MUST RUN BEFORE ANYTHING IS
/// STAGED OR SELECTED. Diffing after the candidate became the selection would be
/// diffing it against itself.
fn unsupported_changed_domains(
    world: &bevy::ecs::world::World,
    candidate: &ambition_content_pack::PreparedContentPack,
) -> Vec<String> {
    let Some(active) = crate::pack::selected(world) else {
        // ⚠ NO ACTIVE PACK IS A FIRST PUBLICATION, not a change: there is no
        // generation for a domain to disagree with.
        return Vec::new();
    };
    ambition_content_pack::changed_domains(active, candidate)
        .into_iter()
        .filter(|schema| schema.0 != PARTICIPATING_DOMAIN)
        .map(|schema| schema.0)
        .collect()
}

/// May a content generation be published into this world right now?
///
/// ⛔⛤ **COMPUTED HERE RATHER THAN TAKEN AS A PARAMETER, AND I CHANGED MY MIND
/// ABOUT THAT.** A `legality: PublicationBoundary` argument would have been a
/// precondition every caller could answer wrongly — and a reload road that grew
/// a second caller would grow a second opinion about when publishing is legal.
/// The authority is a resource; reading it is not a dependency the composition
/// has to thread.
///
/// ⚠ NO AUTHORITY MEANS LEGAL, which is the right answer and not a hole: a
/// composition that installs no rollback has no timeline to invalidate. A stood-
/// down timeline is legal for the same reason — it is not speculating.
enum PublicationBoundary {
    Legal,
    LiveTimeline,
    Unhealthy(String),
}

fn publication_boundary(world: &bevy::ecs::world::World) -> PublicationBoundary {
    use ambition_platformer2d_runtime::rollback::{
        ActiveRollbackAuthority, RollbackConfirmationState,
    };
    let Some(authority) = world.get_resource::<ActiveRollbackAuthority>() else {
        return PublicationBoundary::Legal;
    };
    // ⛔ THE AUTHORITY'S OWN SCOPE, never a stranger's. `confirmation_for` returns
    // `Unavailable` for a scope it does not govern, and reading a stranger's
    // answer would report every world as publishable.
    match authority.confirmation_for(authority.owner()) {
        RollbackConfirmationState::Unavailable => PublicationBoundary::Legal,
        RollbackConfirmationState::Healthy => PublicationBoundary::LiveTimeline,
        RollbackConfirmationState::Unhealthy => PublicationBoundary::Unhealthy(
            authority
                .status()
                .invalidation
                .clone()
                .unwrap_or_else(|| format!("{:?}", authority.status().mismatch_frames)),
        ),
    }
}

/// Publish a freshly compiled pack as a candidate prepared against what is live.
///
/// ⚠ THE CONVENIENCE FORM, for a caller that compiled and published without
/// yielding. A caller that did file I/O in between must build the candidate
/// itself with the identity it READ, or its base claim is a fiction.
#[cfg(test)]
pub(crate) fn reload_move_tables_selecting(
    world: &mut bevy::ecs::world::World,
    fresh: std::sync::Arc<ambition_content_pack::PreparedContentPack>,
    compiled_against: Option<CharacterCatalogGeneration>,
) -> MoveReload {
    let candidate = ambition_content_pack::CandidateGeneration::prepared_against(fresh, None);
    publish_candidate(world, candidate, compiled_against)
}

/// What a reload REQUEST did — the road that reuses the engine's own lifecycle.
///
/// ⭐⭐ **A REQUEST, NOT A PUBLICATION, AND THAT IS THE WHOLE DIFFERENCE FROM
/// [`publish_candidate`].** Fast-iteration I3 step 4 says *"file watching calls
/// the same request path"*, and MEASURED 2026-09-11 that path exists:
/// `ShellEvent::PreparationRequested` → `prepare_requested_sessions` →
/// `prepare_platformer_content`, which allocates the `ContentEpoch` as its final
/// non-fallible step and fingerprints the whole content — including the
/// `content.pack` section — before publishing at the activation boundary.
///
/// ⇒ **SO A RELOAD NEEDS NO SECOND LIFECYCLE.** Re-requesting the route the shell
/// is already ACTIVE on mints a fresh transaction (`start_route` has no same-route
/// guard; witnessed in `ambition_game_shell`), and the old generation stays
/// authoritative until the new one activates — which is the model the
/// architecture review asks for, in the existing lifecycle's own terms.
#[derive(Debug, Clone, PartialEq)]
pub enum ReloadRequest {
    /// The pack on disk does not compile. Nothing was selected and nothing was
    /// requested.
    PackRefused(String),
    /// The candidate is mechanically identical to the selected pack, WHOLE PACK.
    /// Nothing was requested: a re-preparation would consume an epoch, a
    /// publication and a reconstruction for content that did not change.
    Unchanged,
    /// The shell is not active on any route, so there is nothing to re-prepare.
    NoActiveRoute,
    /// The active route declares no preparation plan, so re-requesting it would
    /// never reach `prepare_platformer_content`.
    ///
    /// ⛔ THE SAME PRECONDITION THE RETRY ROAD ALREADY CHECKS
    /// (`ambition_load_presentation::shell_adapter`), and for the same reason:
    /// a route with no plan is not a route a preparation can be asked of.
    RouteHasNoPreparation(String),
    /// A rollback timeline is speculating, or its authority is unhealthy. See
    /// [`MoveReload::RefusedDuringLiveTimeline`].
    Refused(MoveReload),
    /// A generation is ALREADY in flight, so this one was refused.
    ///
    /// ⛔⛤ **AN EXPLICIT REFUSAL RATHER THAN ACCIDENTAL LAST-WRITE-WINS.** The
    /// three resources this replaced were singletons: a second request
    /// overwrote the first's pack and its (still unadopted) transaction, so when
    /// the router announced the FIRST request's `LoadId` the SECOND generation
    /// adopted it — two generations coordinating by overwriting each other, and
    /// a file watcher makes closely spaced saves entirely ordinary.
    ///
    /// ⚠ REFUSING IS NOT THE ONLY DEFENSIBLE POLICY — supersession through a
    /// real cancellation transaction is better — but it is the only one that is
    /// STATED. An accidental coalescing nobody chose is not a policy.
    AlreadyPending { route: String },
    /// The request was issued. **Nothing is published yet** — the new generation
    /// appears when the shell activates it, and the current one is authoritative
    /// until then.
    Requested { route: String },
}

/// Ask the running host to re-prepare its session against `candidate`.
///
/// ⚠ **THE SELECTION IS INSTALLED BEFORE THE REQUEST, AND THAT ORDER IS FORCED.**
/// `prepare_platformer_content` reads `SelectedContentIdentity` to fingerprint,
/// and `authored_intrinsics` reads `SelectedContentPack` when the cast is
/// registered — both happen INSIDE the preparation this asks for, so the
/// selection is an input to it rather than a result of it.
///
/// ⛔ THE WINDOW THIS OPENS, NAMED RATHER THAN HIDDEN: between the selection and
/// the activation, the App's selected pack is the NEW one while the live cast is
/// still built from the old. Nothing published has changed — the cast, its
/// generation and the prepared content identity are all untouched — but a reader
/// that asks `pack::selected` during that window gets the incoming answer.
/// Closing it means making selection part of the activation transaction, which
/// is the rest of I3 and is not done here.
pub fn request_reload(
    world: &mut bevy::ecs::world::World,
    candidate: ambition_content_pack::CandidateGeneration,
) -> ReloadRequest {
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteCatalog, ShellRouter};

    // ⛔ ONE GENERATION IN FLIGHT AT A TIME, and this is asked FIRST. A second
    // request that got as far as staging would have overwritten the first's
    // candidate before anything noticed.
    if let Some(pending) = world.get_resource::<PendingGeneration>() {
        return ReloadRequest::AlreadyPending {
            route: pending.route.clone(),
        };
    }

    // ⛔ ONE PREFLIGHT, SHARED. See [`admit_candidate`] — this was spelled out
    // here a second time, in the same order, and two copies of a rule make each
    // other untestable.
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

    // ⛔⛔ **THE CAST REVISION IS STAGED HERE AND PUBLISHED AT THE ACTIVATION, NOT
    // BEFORE IT.** MEASURED 2026-09-11: `register_declared_cast` runs in
    // `Plugin::build`, ONCE, so a session re-preparation moves the `ContentEpoch`,
    // the content fingerprint and the rollback contract and changes NOT ONE move
    // table the live cast plays. Re-preparing is necessary and not sufficient;
    // the two roads are different mechanisms by necessity, because nothing can
    // re-run `Plugin::build`.
    //
    // ⇒ So the transaction stages one and requests the other, and
    // [`publish_staged_reload_on_activation`] lands them together.
    let pack = candidate.into_pack();
    if let Some(section) = ambition_characters::moveset_content_schema::lowered_movesets(&pack) {
        let cast_base = world
            .get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .map(ambition_characters::prepared::PreparedCharacterRegistry::generation);
        let problems = stage_move_section(world, section, cast_base);
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
    // ⛔⛤ **STAGED AS PENDING, NOT INSTALLED AS THE SELECTION.** Installing it
    // here is what let a FAILED preparation leave the App selecting a pack whose
    // cast it never built — after which the next save of the same file compared
    // against that selection, reported `Unchanged`, and requested nothing. The
    // game stayed split for the session while the reload said all was well.
    // ⛔⛤ **ADMISSION FINISHES BEFORE THE REQUEST IS ISSUED, NOT AT THE COMMIT
    // BOUNDARY.** `admit_staged_revision` takes `&World` and mutates nothing, so
    // this asks "would this publish?" and is free to answer no. Without it, an
    // authored effect naming a technique this composition never installed would
    // be discovered by `RouteActivated` — at which point the shell has already
    // committed the new route and prepared session, the engine's half of the
    // generation is N+1, and the cast's half refuses. That half-transaction is
    // the thing I3 exists to prevent, and a commit path is not where a candidate
    // may learn it is invalid.
    //
    // ⚠ `Unchanged` AND `NothingStaged` BOTH PROCEED. The move material being
    // identical does not mean the pack is: the participating domain can change in
    // a character no buildable cast member wears, and the engine's generation
    // still has to move.
    //
    // ⛔⛤ **AND A COMPOSITION WITH NO TECHNIQUE TABLE REFUSES HERE RATHER THAN AT
    // THE BOUNDARY.** `InstalledTechniques` ABSENT is not an empty table — an
    // empty one is a legitimate value meaning "this host installs nothing", and
    // admitting against it correctly refuses every authored effect. An absent
    // resource means the composition never installed the combat capability, so
    // nothing here can admit the revision at all. Letting the request through
    // meant the activation reached a branch with no answer, returned, and left
    // the pending pack staged forever while the engine's half had moved.
    let support = world
        .get_resource::<ambition_combat::technique::InstalledTechniques>()
        .map(|installed| installed.0.clone());
    let Some(support) = support else {
        discard_staged_reload(world);
        return ReloadRequest::Refused(MoveReload::NoTechniqueSupport);
    };
    // ⛔⛤ **THE ADMITTED VALUE IS KEPT, AND THROWING IT AWAY WAS THE DEFECT.**
    // This used to admit, discard the `AdmittedRevision`, and let the ACTIVATION
    // admit all over again against whatever the world held by then — so
    // "admission finishes before activation is authorized" was really only
    // "admission succeeded once before activation was requested". Those are
    // different sentences, and the gap between them is a commit path that can
    // still refuse.
    //
    // ⚠ AND IT IS *TAKEN*, NOT BORROWED. `admit_staged_revision` deliberately
    // mutates nothing, which also leaves the edits staged for whoever activates
    // next: measured 2026-09-11, an unrelated publication DRAINED a pending
    // reload's staged revision and the reload's own boundary then found
    // `NothingStaged`. A generation that owns its edit cannot have it absorbed.
    let admitted_cast = match ambition_characters::prepared::take_admitted_revision(world, &support)
    {
        ambition_characters::prepared::RevisionAdmission::Refused { refusals, .. } => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::Refused(
                refusals.iter().map(|r| r.detail.clone()).collect(),
            ));
        }
        ambition_characters::prepared::RevisionAdmission::Stale {
            prepared_against,
            active,
        } => {
            discard_staged_reload(world);
            return ReloadRequest::Refused(MoveReload::Stale {
                prepared_against: prepared_against.get(),
                active: active.get(),
            });
        }
        ambition_characters::prepared::RevisionAdmission::Admitted(admitted) => Some(admitted),
        // ⚠ NOTHING TO PUBLISH ON THE CAST'S SIDE IS NOT A REFUSAL. The move
        // material being identical does not mean the pack is, and the engine's
        // generation still has to move.
        ambition_characters::prepared::RevisionAdmission::NothingStaged
        | ambition_characters::prepared::RevisionAdmission::Unchanged { .. } => None,
    };
    stage_pending_generation(
        world,
        PendingGeneration {
            load_id: None,
            route: route.as_str().to_string(),
            pack,
            admitted_cast,
        },
    );
    // ⛔ `ReplaceWith`, NEVER `GoTo`. A reload is not navigation and must not push
    // a history entry: a player who reloaded three times and pressed back would
    // otherwise walk back through three copies of the room they are standing in.
    world.write_message(ShellCommand::ReplaceWith(route.clone()));
    ReloadRequest::Requested {
        route: route.as_str().to_string(),
    }
}

/// **ONE PENDING GENERATION: the transaction, the candidate, and the ADMITTED
/// cast.**
///
/// ⛔⛤ **THIS WAS THREE COOPERATING RESOURCES AND THE COOPERATION WAS THE BUG.**
/// `PendingContentPack`, `StagedCastRevision` and `PendingReloadTransaction`
/// were separate world state expected to stay mutually coherent, and four
/// distinct defects came out of that expectation:
///
/// 1. The admitted value was computed at request time and THROWN AWAY, so the
///    commit boundary re-admitted against whatever the world held by then — and
///    could still refuse after the engine's half was committed.
/// 2. A world with no `InstalledTechniques` let the request through and then
///    found no answer at the boundary, stranding all three.
/// 3. A SECOND request overwrote the first's pack and transaction singletons
///    while the first was still in flight, so the second could adopt the first's
///    `LoadId` — accidental last-write-wins coordination between two generations.
/// 4. An unrelated publication DRAINED the shared `StagedCastRevision`, silently
///    absorbing the reload's edit; the boundary then found `NothingStaged`.
///
/// ⇒ **ALL FOUR ARE THE SAME MISSING VALUE.** A generation that OWNS what was
/// admitted cannot have it re-derived, stranded, overwritten or absorbed.
///
/// ⛔ **AND THE COMMIT PATH IS THEREFORE INFALLIBLE BY CONSTRUCTION.** Publishing
/// one of these is [`publish_admitted_revision`] plus an install; neither can
/// say no. The only branch left at the boundary is "is this my transaction",
/// which is a question about identity, not about content.
///
/// ⚠ `load_id` IS `None` UNTIL THE ROUTER MINTS IT. MEASURED 2026-09-11:
/// `ShellRouter::next_load_transaction` is private, the id is minted in a LATER
/// system than the request, and `ShellCommand::ReplaceWith` carries no slot for a
/// correlator — so the requester can neither read nor predict its own
/// transaction. It is ADOPTED from `ShellEvent::PreparationRequested`, the first
/// moment the identity exists and is observable. A route name is not that
/// identity: two generations can target one route, which is exactly what a
/// reload does.
#[derive(bevy::prelude::Resource)]
pub struct PendingGeneration {
    /// `None` until the router mints the transaction this reload asked for.
    load_id: Option<ambition_platformer2d::load::LoadId>,
    /// The route the request named, for adopting the right
    /// `PreparationRequested` when several are in flight.
    route: String,
    /// The candidate pack. **Not the App's selection** — it becomes that only at
    /// the activation, and a failed preparation must leave every reader
    /// answering with the content the live cast was actually built from.
    pack: std::sync::Arc<ambition_content_pack::PreparedContentPack>,
    /// ⭐⭐ **THE VALUE ADMISSION ALREADY COMPUTED.** `None` means the candidate
    /// changed no move material — a legitimate pending generation, since the
    /// participating domain can change in a character no buildable cast member
    /// wears and the engine's generation still has to move.
    admitted_cast: Option<ambition_characters::prepared::AdmittedRevision>,
}

/// The candidate a re-preparation is about, if one is in flight.
pub fn pending_pack(
    world: &bevy::ecs::world::World,
) -> Option<&ambition_content_pack::PreparedContentPack> {
    world
        .get_resource::<PendingGeneration>()
        .map(|generation| generation.pack.as_ref())
}

/// Stage a pending generation. **The App's selection is untouched.**
///
/// ⛔⛤ **THIS USED TO OVERWRITE `SelectedContentIdentity` AND THAT WAS A CRITICAL
/// SCOPE ERROR** (2026-09-11 review item 3). `prepare_platformer_content` reads
/// an identity to fingerprint the generation it is preparing, and the only road
/// to it was the App-wide selection — so a pending generation reported
/// `SelectedContentPack = N` with `SelectedContentIdentity = N+1`, and every
/// UNRELATED route preparation running in that window inherited a candidate
/// stamp for content it never prepared. The identity is exactly what the
/// rollback timeline contract compares.
///
/// ⇒ The claim is made at ADOPTION instead, keyed on the `LoadId`, because that
/// is the first moment the transaction has a name. Between the request and the
/// router's announcement there is no claim at all — and that is correct: a
/// preparation nobody has correlated to this generation must not use it.
fn stage_pending_generation(world: &mut bevy::ecs::world::World, generation: PendingGeneration) {
    world.insert_resource(generation);
}

/// Take the pending generation away, and its identity claim with it.
///
/// ⛔ THE CLAIM GOES TOO, ALWAYS. A claim left behind names a `LoadId` whose
/// generation no longer exists, and the next transaction to reuse that id —
/// or a retry of the same one — would be fingerprinted against a candidate this
/// App threw away.
fn take_pending_generation(world: &mut bevy::ecs::world::World) -> Option<PendingGeneration> {
    let generation = world.remove_resource::<PendingGeneration>()?;
    world.remove_resource::<ambition_platformer2d_runtime::PendingContentIdentity>();
    Some(generation)
}

/// Install the reload transaction's publication half.
///
/// ⭐⭐ **ONE STATEMENT OF HOW THIS SYSTEM IS INSTALLED, because the condition is
/// half of the installation.** A composition without a game shell never registers
/// `ShellEvent`, and a `MessageReader` for an unregistered message does not read
/// nothing — it FAILS PARAMETER VALIDATION and panics the schedule. If the
/// plugin spelled the condition and a test spelled it again, the plugin could
/// drop it and the test would stay green.
///
/// ⛔⛤ **THE WORKSPACE LANE IS WHAT TAUGHT ME THAT, one commit after registering
/// the system.** Every test in this crate registers `ShellEvent` itself, so the
/// whole crate was green while the shipped default-feature run panicked.
///
/// ⇒ A RUN CONDITION RATHER THAN AN `Option` PARAMETER: a host with no shell has
/// no activation boundary, so the right behaviour is NOT TO RUN — not to run and
/// find nothing.
pub fn register(app: &mut bevy::prelude::App) {
    use bevy::prelude::IntoScheduleConfigs;
    app.add_systems(
        bevy::prelude::Update,
        publish_staged_reload_on_activation
            // ⛔⛔ **BEFORE THE WORLD IS BUILT FROM THE CAST, OR GENERATION N+1's
            // SESSION IS CONSTRUCTED FROM GENERATION N's MOVES.**
            //
            // `activate_prepared_platformer_sessions` is `in_set(
            // GameplaySessionSet::Providers)` on this same schedule, and its
            // `PlatformerSessionBuilder` reads `PreparedCharacterRegistry`. The
            // shell's `RouteActivated` and the session bridge's
            // `GameplaySessionEvent::Activated` are the SAME frame, so without
            // this edge the two systems raced: the activation that authorized
            // the reload could construct the new world's actors out of the cast
            // the reload was replacing, and nothing in either half would say so.
            //
            // ⚠ THE EDGE IS WHAT MAKES IT IMPOSSIBLE RATHER THAN CHECKED. Bevy
            // inserts the sync point, so the queued publication has applied
            // before any provider constructs anything.
            .before(ambition_platformer2d::game_shell::GameplaySessionSet::Providers)
            // ⛔⛔ **AND BEFORE THE PREPARATION THAT READS ITS IDENTITY CLAIM.**
            // The ADOPTION half of this system stakes the claim from
            // `ShellEvent::PreparationRequested`; `prepare_requested_sessions`
            // reads the SAME message and fingerprints against that claim. With
            // no edge between them the claim could arrive a frame late — and a
            // preparation that missed it silently falls back to the App's active
            // identity, stamping generation N+1's session with N's content.
            // That is a wrong answer, not a missing one, which is why it is an
            // edge and not a retry.
            .before(ambition_platformer2d::provider::PlatformerPreparationSet)
            .run_if(
                bevy::prelude::resource_exists::<
                    bevy::ecs::message::Messages<ambition_platformer2d::game_shell::ShellEvent>,
                >,
            ),
    );
}

/// Publish the staged cast revision when the shell activates the route it was
/// requested for — and discard it if the route failed instead.
///
/// ⭐⭐ **THIS IS THE BOUNDARY THE TWO HALVES SHARE.** The shell's activation
/// publishes the engine's new generation (epoch, content fingerprint, rollback
/// contract); this publishes the cast's. Landing them anywhere else is a
/// half-transaction: only the first is a new generation of the same moves, only
/// the second is new moves under an unchanged generation.
///
/// ⛔ A FAILED PREPARATION DISCARDS THE STAGED REVISION RATHER THAN LEAVING IT.
/// A revision that stayed staged would be applied by whatever activation came
/// next — the content nobody asked for, arriving at a boundary nobody connected
/// it to. The staleness stamp cannot save it: nothing published, so its base is
/// still current.
pub fn publish_staged_reload_on_activation(
    mut events: bevy::ecs::message::MessageReader<ambition_platformer2d::game_shell::ShellEvent>,
    mut commands: bevy::prelude::Commands,
) {
    use ambition_platformer2d::game_shell::ShellEvent;
    for event in events.read() {
        match event {
            // ⭐⭐ **ADOPT THE TRANSACTION THE ROUTER JUST MINTED.** This is the
            // first moment the identity exists and is observable, and the only
            // one before the activation that carries it. A pending reload with
            // no adopted id yet takes the first request for ITS route.
            ShellEvent::PreparationRequested(transaction) => {
                let route = transaction.route_id.as_str().to_string();
                let load_id = transaction.barrier.load_id.clone();
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    let claim = {
                        let Some(mut pending) = world.get_resource_mut::<PendingGeneration>()
                        else {
                            return;
                        };
                        if pending.load_id.is_some() || pending.route != route {
                            return;
                        }
                        pending.load_id = Some(load_id.clone());
                        crate::pack::identity_line(&pending.pack)
                    };
                    // ⭐ THE CLAIM IS MADE HERE AND NOWHERE ELSE, because this is
                    // the first moment the transaction has a name to claim.
                    world.insert_resource(ambition_platformer2d_runtime::PendingContentIdentity {
                        load_id: load_id.to_string(),
                        identity: claim,
                    });
                });
            }
            ShellEvent::RouteActivated(active) => {
                // ⛔ ONLY THE ACTIVATION OF THE TRANSACTION THIS RELOAD ASKED
                // FOR. `load_authorization` is the barrier the router authorized
                // this activation against; an unrelated route's activation
                // carries a different one (or none) and must be irrelevant here.
                let authorized = active
                    .load_authorization
                    .as_ref()
                    .map(|barrier| barrier.load_id.clone());
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    if !reload_owns(world, authorized.as_ref()) {
                        return;
                    }
                    // ⭐⭐ **THE COMMIT PATH, AND THERE IS NOTHING IN IT THAT CAN
                    // SAY NO.** Every question that could refuse — the verdict,
                    // the changed domains, the rollback boundary, the authored
                    // effects, the cast base — was asked and answered at request
                    // time, and the ANSWERS are what this value carries.
                    //
                    // ⛔ IT USED TO RE-ADMIT HERE, against whatever the world
                    // held by then, with the pack PROMOTED FIRST. A composition
                    // that changed in flight therefore left the App selecting
                    // N+1, the engine prepared at N+1 and the cast at N — the
                    // exact half-transaction this whole road exists to prevent,
                    // written down as a logged error.
                    let Some(generation) = take_pending_generation(world) else {
                        return;
                    };
                    if let Some(admitted) = generation.admitted_cast {
                        let outcome = ambition_characters::prepared::publish_admitted_revision(
                            world, admitted,
                        );
                        bevy::log::info!("a reloaded cast was published: {outcome:?}");
                    }
                    // ⛔ AND THE PACK LANDS AT THE SAME BOUNDARY, unconditionally,
                    // because nothing above it could have failed.
                    crate::pack::install_selection(world, generation.pack);
                });
            }
            // ⛔ EVERY WAY THE REQUEST CAN END WITHOUT ACTIVATING, and they are
            // listed rather than caught by a wildcard: a new terminal event must
            // be a compile error here, not a staged revision nobody discards.
            //
            // ⛔⛤ **AND THIS HALF CANNOT BE CORRELATED THE WAY THE OTHER ONE IS,
            // WHICH IS A FACT ABOUT THE SHELL'S EVENTS AND NOT A SHORTCUT.**
            // MEASURED 2026-09-11: `CommandRejected::LoadFailed` and
            // `LoadCommitRejected` carry NO barrier at all, and
            // `ExperienceFailed` carries an activation id rather than one — so a
            // `load_id`-stamped reload has nothing to match on for the two most
            // likely REAL failures. Requiring a match would leak the pending
            // generation forever, staged, with the next save comparing against
            // it.
            //
            // ⇒ The router's own `pending` is the correlator instead: on a
            // terminal barrier it sets `terminal_reported` and KEEPS the pending
            // route, so at the moment a rejection is observed its `barrier` still
            // names the load that failed. A reload discards only when that load
            // is ITS load, or when nothing is pending at all.
            ShellEvent::ExperienceFailed { .. } | ShellEvent::CommandRejected(_) => {
                commands.queue(|world: &mut bevy::ecs::world::World| {
                    let failing = world
                        .get_resource::<ambition_platformer2d::game_shell::ShellRouter>()
                        .and_then(|router| router.pending.as_ref())
                        .map(|pending| pending.barrier.load_id.clone());
                    // ⚠ NOTHING PENDING means the shell is not waiting on any
                    // transaction, so a reload waiting on one will never see it
                    // activate — discard rather than leak.
                    if failing.is_some() && !reload_owns(world, failing.as_ref()) {
                        return;
                    }
                    discard_staged_reload(world);
                });
            }
            ShellEvent::WaitingForLoad { .. }
            | ShellEvent::RouteDeactivated(_)
            | ShellEvent::ExitRequested => {}
        }
    }
}

/// Does the pending reload own the transaction `load_id` names?
///
/// ⚠ **AN UNADOPTED PENDING RELOAD OWNS NOTHING YET.** Between the request and
/// the router's `PreparationRequested`, a reload has no id to compare — so an
/// activation arriving in that window belongs to something else and must be
/// ignored rather than allowed to publish it.
///
/// ⚠ AND A WORLD WITH NO PENDING RELOAD OWNS NOTHING EITHER, which is what makes
/// `publish_candidate`'s direct road and this one able to share a system without
/// stealing each other's boundaries.
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
    // ⛔⛤ **THE PENDING PACK GOES WITH IT, AND THAT HALF WAS MISSING.** Discarding
    // only the cast revision left the App selecting a candidate whose cast it
    // never built, and the next identical save then compared against that
    // selection, reported `Unchanged` and requested nothing — the game split for
    // the rest of the session while the reload said all was well.
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
