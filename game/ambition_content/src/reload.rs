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

use ambition_characters::prepared::{
    activate_staged_revision, stage_move_section, CharacterCatalogGeneration, MovesetRevisionError,
    RevisionOutcome,
};

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

/// Recompile this provider's pack from disk and republish the cast's move
/// tables.
///
/// ⛔ THE HOST MUST HAVE RUN ITS PREPARATION BARRIER. Before that there is no
/// cast to revise, and [`MoveReload::NoCast`] says so rather than inventing one.
pub fn reload_move_tables(world: &mut bevy::ecs::world::World) -> MoveReload {
    match crate::pack::compile_pack() {
        Ok(pack) => reload_move_tables_selecting(world, std::sync::Arc::new(pack), None),
        // ⛔ THE WHOLE DIAGNOSTIC, not a summary. A reload that says "it did not
        // compile" and drops the compiler's per-problem list makes the author
        // re-run a CLI to learn what this function already knew.
        Err(failure) => MoveReload::PackRefused(failure.to_string()),
    }
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
pub fn reload_move_tables_from(
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
pub fn reload_move_tables_from_dir(
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
pub fn publish_candidate(
    world: &mut bevy::ecs::world::World,
    candidate: ambition_content_pack::CandidateGeneration,
    cast_base: Option<CharacterCatalogGeneration>,
) -> MoveReload {
    // ⛔ THE BOUNDARY IS ASKED BEFORE ANYTHING IS READ OR STAGED. A publication
    // that is not legal now must leave the world exactly as it found it, and the
    // cheapest way to guarantee that is to answer the question first.
    match publication_boundary(world) {
        PublicationBoundary::Legal => {}
        PublicationBoundary::LiveTimeline => return MoveReload::RefusedDuringLiveTimeline,
        PublicationBoundary::Unhealthy(reason) => {
            return MoveReload::RefusedWhileRollbackUnhealthy(reason)
        }
    }
    let active = crate::pack::selected(world).map(|pack| pack.fingerprint);
    match candidate.verdict(active) {
        ambition_content_pack::CandidateVerdict::Stale {
            prepared_against,
            active,
        } => MoveReload::StaleGeneration {
            prepared_against: prepared_against.hex(),
            active: active.hex(),
        },
        // ⛔ A COMPLETE NO-OP TOUCHES NOTHING — not the cast, not the selection.
        // A file watcher fires on a SAVE, not on a CHANGE, so this is the common
        // case in the loop this exists for.
        ambition_content_pack::CandidateVerdict::Unchanged { .. } => {
            let generation = world
                .get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
                .map(|registry| registry.generation().get())
                .unwrap_or_default();
            MoveReload::Unchanged { generation }
        }
        ambition_content_pack::CandidateVerdict::Publish { .. } => {
            let pack = candidate.into_pack();
            let outcome = reload_move_tables_from(world, &pack, cast_base);
            // ⛔ THE SELECTION FOLLOWS THE CAST'S ADMISSION, not the compile. A
            // refused or stale revision must leave the App reading the pack its
            // cast was actually built from.
            if matches!(
                outcome,
                MoveReload::Activated { .. } | MoveReload::Unchanged { .. }
            ) {
                world.insert_resource(crate::pack::SelectedContentPack(pack));
            }
            outcome
        }
    }
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
pub fn reload_move_tables_selecting(
    world: &mut bevy::ecs::world::World,
    fresh: std::sync::Arc<ambition_content_pack::PreparedContentPack>,
    compiled_against: Option<CharacterCatalogGeneration>,
) -> MoveReload {
    let candidate = ambition_content_pack::CandidateGeneration::prepared_against(fresh, None);
    publish_candidate(world, candidate, compiled_against)
}

#[cfg(test)]
#[path = "reload_tests.rs"]
mod reload_tests;
