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
/// ⛔⛤ **FOUR OF THESE ARE `#[cfg(test)]`, WHICH IS A STATEMENT ABOUT PRODUCTION
/// RATHER THAN ABOUT TESTS.** `PackRefused`, `NoMoveSection`, `Activated` and
/// `Unchanged` are produced ONLY by the fixture road — `publish_candidate` and
/// the `reload_move_tables*` helpers — which became `#[cfg(test)]` when the
/// direct publication road stopped being a production authority. Left visible in
/// a shipped build they would hand every production consumer four states it can
/// never be given, and the consumer that matters is
/// [`ReloadRequest::Refused`]: it would be able to spell `Refused(Activated)`,
/// which is not a refusal at all.
///
/// ⚠ THE REST ARE ALL REACHABLE FROM `request_reload`, several of them through
/// the shared [`admit_candidate`] rather than from its own body — a variant's
/// producer being one call away is not the same as it being unreachable.
pub enum MoveReload {
    /// The pack on disk does not compile. **Nothing was staged and the live cast
    /// is untouched** — the refusal carries the content compiler's own
    /// diagnostic, which names every problem rather than the first.
    #[cfg(test)]
    PackRefused(String),
    /// The pack compiled and carries no move section at all.
    ///
    /// ⛔ NOT an error and NOT silence: a pack whose `moveset` sources were all
    /// removed is a legitimate edit, and replacing the live cast's movesets with
    /// nothing is not what it asks for. It is reported so a caller can tell it
    /// from a successful reload of zero changes.
    #[cfg(test)]
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
    #[cfg(test)]
    Activated { generation: u64, changed: usize },
    /// Every table on disk is what the live cast was already built from.
    ///
    /// ⭐ A FILE WATCHER FIRES ON A SAVE, NOT ON A CHANGE, so this is the
    /// COMMON case in the loop this exists for, not an edge one.
    #[cfg(test)]
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
    /// The candidate stops naming characters whose authored moveset the live
    /// cast is playing. **Nothing was staged and nothing was published.**
    ///
    /// ⛔ THE PARTICIPATING DOMAIN CHANGED IN A WAY THE PARTICIPANT CANNOT APPLY.
    /// See [`dropped_moveset_entities`]: staging is per-entity over the
    /// CANDIDATE's keys, so a character the candidate stops naming keeps the old
    /// generation's moves and is re-published under the new one — the pack says
    /// one thing and the cast plays another, silently, from deleting one entity.
    RefusedDroppedMovesetEntities(Vec<String>),
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

    // ⛔ AND THE PARTICIPATING DOMAIN HAS TO BE APPLICABLE, not merely permitted.
    // Asked here rather than at staging because staging cannot SEE it: it
    // iterates the candidate's keys, so a dropped entity is not a thing it fails
    // to do — it is a thing it is never asked to do.
    if let Some(active) = crate::pack::selected(world) {
        let dropped = dropped_moveset_entities(active, candidate.pack());
        if !dropped.is_empty() {
            return CandidateAdmission::Refused(MoveReload::RefusedDroppedMovesetEntities(dropped));
        }
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

/// Does this schema id name a family the generation transaction can carry?
///
/// ⛔ A `match`, NOT A TABLE. A table of participating ids is wrong the moment
/// somebody adds a family and forgets the row; this refuses everything it does
/// not name, so the failure of forgetting is a REFUSED reload rather than a
/// family that promotes and never lands.
///
/// ⭐⭐ **TWO FAMILIES, AND THE SECOND ONE IS THE POINT.** The architecture
/// review's gate was *"the SECOND mechanical content family — as validation that
/// the transaction absorbs it with no new authority"*. `fighter_brain_ladder` is
/// what the census picked, and it absorbed with NO new resource, NO new ordering
/// edge and NO new refusal: see [`publish_participant_families`].
///
/// ⚠ AND IT IS NOT ITEMS, which is what the fixtures assume. MEASURED
/// 2026-09-12: `install_item_catalog` writes a second process-global `OnceLock`
/// whose own comment reports that a different second catalog *"was IGNORED"*,
/// and the read side returns `&'static str` across ~80 external uses. A borrow
/// whose lifetime is the `OnceLock` is a structural claim that there is one
/// generation forever — the signature cannot express N+1, so no ordering can
/// make it participate.
fn participates(domain: &str) -> bool {
    domain == ambition_characters::moveset_content_schema::MOVESET_SCHEMA
        || PACK_DERIVED_FAMILIES
            .iter()
            .any(|family| family.domain == domain)
}

/// One mechanical family whose whole publication is a function of the candidate
/// pack.
///
/// ⛔⛔ **THE SCHEMA ID AND THE PUBLICATION ARE ONE DECLARATION, AND THAT IS THE
/// POINT.** [`PARTICIPATING_DOMAIN`] was a bare const, and the shape it would
/// have grown into — a LIST of permitted ids, with the publications somewhere
/// else — is the drift this type exists to make impossible. A family named on a
/// list of ids but missing from the publication is a family whose domain passes
/// [`unsupported_changed_domains`], whose pack promotes, and which then stays at
/// generation N forever: the first row of `dropped_moveset_entities`'s own
/// transition table, reintroduced for a new family.
///
/// ⇒ A row cannot name a domain it cannot publish, because the publisher IS the
/// row. And a family added to NEITHER is refused rather than silently dropped:
/// [`participates`] scans this table, so an unlisted domain is an unsupported
/// changed domain and the reload says no.
struct PackDerivedFamily {
    domain: &'static str,
    publish: fn(&mut bevy::ecs::world::World, &ambition_content_pack::PreparedContentPack),
}

/// Every family the transaction publishes from the pack alone.
///
/// ⚠ `moveset` IS DELIBERATELY ABSENT. Its publication is not a function of the
/// pack: the cast revision is ADMITTED against world state — the installed
/// technique table and the live cast generation — and [`PendingGeneration`]
/// carries that admitted value precisely because an answer computed against the
/// world goes stale when the world moves. A row here would have to re-derive it
/// at the boundary, which is the defect the review's item 1 named.
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

/// ⛔ ABSENT IN THE CANDIDATE MEANS REMOVE, NOT KEEP — see
/// [`publish_participant_families`]. `profile_for_level` reads an `Option` and
/// states that absent means the engine floor.
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

/// ⛔ SAME RULE, AND THE RUNTIME ALREADY STATES IT: `authored_encounter_waves`
/// takes `Option<&EncounterWaveBook>` and says *"`None` means the adapter should
/// fall back to one wave assembled from the level's own spawn markers"*. A
/// candidate that declares no waves is asking for that fallback, so keeping
/// generation N's book would answer a question the candidate stopped asking.
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
/// ⭐⭐ **DERIVED AT THE BOUNDARY RATHER THAN CARRIED, AND THE DIFFERENCE FROM
/// `admitted_cast` IS THE WHOLE ARGUMENT.** [`PendingGeneration`] carries the
/// admitted cast because admission asked the WORLD a question — which techniques
/// this composition installed, which generation the cast is on — and an answer
/// computed against world state goes stale when the world moves. The fighter
/// ladder asks the world nothing: it is `lowered_fighter_brain_ladder(pack)`, a
/// total function of a value this generation already owns. Re-deriving it here
/// cannot disagree with anything, so carrying a second copy would be a second
/// authority for one fact.
///
/// ⛔ **ABSENT IN THE CANDIDATE MEANS REMOVE, NOT KEEP.** A candidate that drops
/// the `fighter_brain_ladder` section is asking for the engine floor —
/// `profile_for_level` states that rule and reads `Option` for exactly this
/// reason. Leaving generation N's resource behind is the defect the moveset
/// family's own transition table records in its first row: the pack promotes and
/// the family silently stays at N. The ladder can express its own removal, and
/// that is why it was the clean second family to take.
///
/// ⛔ NOTHING HERE CAN SAY NO. The commit path is infallible by construction and
/// this does not change that: a resource write and a resource removal are both
/// unconditional.
///
/// ⚠ **AND NO NEW ORDERING EDGE.** The projection that puts a rung onto a brain
/// (`project_authored_fighter_ladder`) runs on `Added<Brain>`, and a reload
/// reconstructs the session — so the fighters that read this are spawned by
/// `GameplaySessionSet::Providers`, which [`register`] already orders this
/// system before, for the cast's sake.
fn publish_participant_families(
    world: &mut bevy::ecs::world::World,
    pack: &ambition_content_pack::PreparedContentPack,
) {
    for family in PACK_DERIVED_FAMILIES {
        (family.publish)(world, pack);
    }
}

/// Characters whose authored moveset the live cast is playing that the candidate
/// STOPS NAMING.
///
/// ⛔⛤ **"SUPPORTED DOMAIN" MEANT "THIS SCHEMA ID IS ON THE ALLOW-LIST", AND IT
/// HAS TO MEAN "THIS PARTICIPANT CAN APPLY THIS TRANSITION COMPLETELY."**
/// `moveset` is the participating domain, so ANY moveset change was let through
/// — including three that leave the pack and the cast disagreeing. MEASURED
/// 2026-09-12 by reading the road end to end:
///
/// | transition | what happens today |
/// |---|---|
/// | the moveset section is REMOVED | `lowered_movesets` returns `None`, `request_reload` skips the whole staging block, the pack promotes and every character keeps generation N's moves |
/// | the section is EMPTIED | `Some({})`, `stage_move_section` runs and its unknown-character check passes VACUOUSLY over nothing, same result |
/// | an ENTITY is dropped from the candidate | `stage_move_section` iterates the CANDIDATE's keys, so that character is never visited; the fold is `active.clone()` plus the staged set, so its OLD moveset is RE-PUBLISHED under the new generation |
///
/// ⚠ THE THIRD IS THE ONE NO ALLOW-LIST ROW CAN SEE, and it is reachable by
/// ordinary authoring: `cellular_automaton.ron` carries two entities, so removing
/// one does not even require deleting a file. Nothing at any layer can represent
/// *"this entity's authored moveset is gone"* — `MovesetRevisionError` has two
/// variants and both are about the candidate NAMING something.
///
/// ⇒ **CONTAINMENT, NOT EQUALITY.** Every entity whose authored moveset the live
/// cast is playing must still be named by the candidate. Adding an entity stays
/// legal — that is a different question, and `MovesetRevisionError::UnknownCharacter`
/// already refuses the build that cannot host it.
///
/// ⚠ AND THIS IS NOT THE PER-SOURCE DIGEST INSTRUMENT. `changed_domains` answers
/// *"did this domain change"*; this answers *"can the change be applied"*. Same
/// family, different question, and collapsing them would make an ordinary retime
/// unpublishable.
pub(crate) fn dropped_moveset_entities(
    base: &ambition_content_pack::PreparedContentPack,
    candidate: &ambition_content_pack::PreparedContentPack,
) -> Vec<String> {
    use ambition_characters::moveset_content_schema::lowered_movesets;
    let Some(base) = lowered_movesets(base) else {
        // ⚠ THE BASE AUTHORED NONE, so there is nothing to lose. A first
        // publication of the family is not a removal.
        return Vec::new();
    };
    match lowered_movesets(candidate) {
        None => base.keys().cloned().collect(),
        Some(candidate) => base
            .keys()
            .filter(|id| !candidate.contains_key(*id))
            .cloned()
            .collect(),
    }
}

/// Does this candidate change the family the STAGED road publishes?
///
/// ⛔⛤ **THE TRANSACTION USED TO REQUIRE MOVESET ADMISSION WHATEVER CHANGED,
/// AND THAT MADE IT A MOVE RELOAD WITH OTHER FAMILIES PUBLISHED BESIDE IT.**
/// `request_reload` staged the move section, demanded `InstalledTechniques` and
/// admitted a revision even when `changed_domains` was exactly
/// `{"fighter_brain_ladder"}` or `{"encounter_waves"}`. Two consequences, both
/// wrong:
///
/// * a composition with legitimate encounter-wave content and NO combat
///   capability could not reload its own family — it was refused
///   `NoTechniqueSupport` for a family that has nothing to do with techniques;
/// * an unrelated family's edit re-staged and re-admitted every unchanged move
///   table, so the cost and the refusal surface of a waves edit were the
///   moveset's.
///
/// ⇒ **ONLY PARTICIPANTS THAT CHANGED PREPARE.** This is the read that decides
/// it, and it is the same `changed_domains` the unsupported-domain diff already
/// asks — so the generation plan has ONE source for "what moved".
///
/// ⚠ NO ACTIVE PACK MEANS CHANGED. A first publication has no generation to
/// diff against, and declining to stage there would silently skip the cast on
/// the one road that has never published it.
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
        .filter(|schema| !participates(&schema.0))
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
    Requested {
        route: String,
        /// ⭐ **THE IDENTITY THIS CALL MINTED**, handed back so the caller can
        /// correlate the transaction it just started. Without it the caller owns
        /// an identity it cannot see, and correlating means guessing the ordinal
        /// or reading a private resource — which is the inference
        /// `ShellRequestId` exists to remove, one layer up.
        request: ambition_platformer2d::game_shell::ShellRequestId,
    },
}

/// Ask the running host to re-prepare its session against `candidate`.
///
/// ⛔⛤ **THIS PARAGRAPH USED TO SAY "THE SELECTION IS INSTALLED BEFORE THE
/// REQUEST, AND THAT ORDER IS FORCED", AND IT IS NO LONGER TRUE.** It described
/// a window in which the App's selected pack was N+1 while the live cast was
/// still N. The pending-generation work closed that: nothing is selected at
/// request time. [`stage_pending_generation`] stores the candidate in
/// `PendingGeneration` and TOUCHES NOTHING ELSE, and
/// `crate::pack::install_selection` runs only inside
/// [`commit_content_generation`], at the activation.
///
/// ⚠ CORRECTED RATHER THAN DELETED, because a weaker reader arriving at this
/// function needs to know that the old sentence was load-bearing and is gone:
/// the preparation reads its identity from the TRANSACTION-LOCAL claim
/// (`PendingContentIdentity`, staked at adoption) rather than from the App-wide
/// selection, which is why the selection no longer has to move first.
///
/// ⛔ WHAT IS STILL OPEN is the request's own identity: the reload writes
/// `ShellCommand::ReplaceWith` with no correlator and then ADOPTS the first
/// `PreparationRequested` for its route, so a second command for the same route
/// queued in the same frame can hand it a transaction it did not issue. A route
/// name is not a transaction identity, and this road still uses one for the
/// window before adoption.
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
    // ⛔⛔ **ONLY PARTICIPANTS THAT CHANGED PREPARE.** See [`moveset_changed`]: a
    // ladder-only or waves-only generation must not require the combat
    // capability, and must not re-stage every unchanged move table to publish
    // somebody else's family.
    let pack = candidate.into_pack();
    let stages_cast = moveset_changed(world, &pack);
    let section = stages_cast
        .then(|| ambition_characters::moveset_content_schema::lowered_movesets(&pack))
        .flatten();
    if let Some(section) = section {
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
    // ⚠ AND THE TECHNIQUE TABLE IS ONLY REQUIRED BY THE FAMILY THAT USES IT.
    // A generation that stages no cast asks the combat capability nothing, so
    // demanding it would refuse a waves edit for the absence of something it
    // never consults.
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
        // ⚠ NOTHING TO PUBLISH ON THE CAST'S SIDE IS NOT A REFUSAL, and it is
        // now reached two ways: the move material was identical, OR this
        // generation changes no moveset at all and never staged one.
        ambition_characters::prepared::RevisionAdmission::NothingStaged
        | ambition_characters::prepared::RevisionAdmission::Unchanged { .. } => None,
    };
    // ⭐⭐ **THE IDENTITY IS MINTED BEFORE THE COMMAND IS WRITTEN**, which is the
    // whole point: the generation owns it from birth rather than adopting
    // whatever the router announces for its route.
    //
    // ⚠ SEEDED FROM THE ROUTE AND A PROCESS-UNIQUE COUNTER. The route makes it
    // readable in a log; the counter is what makes it an identity, because the
    // route alone is exactly the thing that was not unique.
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
    // ⛔ `ReplaceWith`, NEVER `GoTo`. A reload is not navigation and must not push
    // a history entry: a player who reloaded three times and pressed back would
    // otherwise walk back through three copies of the room they are standing in.
    world.write_message(ShellCommand::ReplaceWith {
        route: route.clone(),
        request: Some(request.clone()),
    });
    ReloadRequest::Requested {
        route: route.as_str().to_string(),
        request,
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
    /// The route the request named — for reporting, and for
    /// `ReloadRequest::AlreadyPending`. **Not the correlator any more.**
    route: String,
    /// ⭐⭐ **THIS GENERATION'S OWN REQUEST IDENTITY, MINTED BEFORE THE COMMAND
    /// WAS WRITTEN.** See [`ambition_platformer2d::game_shell::ShellRequestId`].
    ///
    /// ⛔⛤ **ADOPTION USED TO MATCH ON THE ROUTE NAME, AND THE COMMENT BESIDE IT
    /// SAID THAT WAS WRONG.** It read *"a route name is not that identity: two
    /// generations can target one route, which is exactly what a reload does"* —
    /// and then matched on the route. Two `ReplaceWith("game")` queued in one
    /// frame mint `shell.game.N` and `shell.game.N+1`, the second SUPERSEDING
    /// the first, so a route-matching reload could adopt a transaction it did
    /// not issue or one already cancelled, and then wait forever for an
    /// activation that cannot come.
    request: ambition_platformer2d::game_shell::ShellRequestId,
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

/// A process-unique ordinal for a reload's request identity.
///
/// ⛔ NOT A ROUTE NAME AND NOT A CLOCK. The route is what was ambiguous; a clock
/// can repeat under a coarse timer and is not reproducible in a test. A counter
/// is unique for the life of the process, which is exactly as long as a pending
/// generation can live.
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
    // ⛔ ONE CONDITION, SPELLED ONCE, FOR BOTH HALVES. A composition without a
    // game shell registers no `ShellEvent`, and a `MessageReader` for an
    // unregistered message FAILS PARAMETER VALIDATION and panics the schedule.
    // Two systems must not grow two opinions about whether the shell exists.
    let shell_is_installed = bevy::prelude::resource_exists::<
        bevy::ecs::message::Messages<ambition_platformer2d::game_shell::ShellEvent>,
    >;
    app.add_systems(
        bevy::prelude::Update,
        adopt_preparation_transaction
            // ⛔⛔ **BEFORE THE PREPARATION THAT READS ITS IDENTITY CLAIM.** This
            // half stakes the claim from `ShellEvent::PreparationRequested`;
            // `prepare_requested_sessions` reads the SAME message and
            // fingerprints against that claim. With no edge between them the
            // claim could arrive a frame late — and a preparation that missed it
            // silently falls back to the App's active identity, stamping
            // generation N+1's session with N's content. A wrong answer, not a
            // missing one, which is why it is an edge and not a retry.
            .before(ambition_platformer2d::provider::PlatformerPreparationSet)
            .run_if(shell_is_installed),
    )
    .add_systems(
        bevy::prelude::Update,
        commit_content_generation
            // ⛔⛔⛔ **AFTER THE SET THAT PRODUCES `RouteActivated`.** Without this
            // edge the commit runs BEFORE the activation exists and reads it a
            // FRAME LATE — see [`commit_content_generation`] for the measurement.
            // This is the end of the relationship the old graph test could not
            // see, because an edge to a late set says nothing about a message
            // produced in a set before it.
            .after(ambition_platformer2d::game_shell::AmbitionGameShellSet::Pending)
            // ⛔⛔ **AND BEFORE THE WORLD IS BUILT FROM THE CAST.**
            // `activate_prepared_platformer_sessions` is
            // `in_set(GameplaySessionSet::Providers)` and its
            // `PlatformerSessionBuilder` reads `PreparedCharacterRegistry`. Bevy
            // inserts the sync point, so the queued publication has applied
            // before any provider constructs anything.
            .before(ambition_platformer2d::game_shell::GameplaySessionSet::Providers)
            .run_if(shell_is_installed),
    );
}


/// Adopt the transaction the router mints for this reload's request.
///
/// ⭐⭐ **THE FIRST MOMENT THE IDENTITY EXISTS AND IS OBSERVABLE.** MEASURED
/// 2026-09-11: `ShellRouter::next_load_transaction` is private, the id is minted
/// in a LATER system than the request, and `ShellCommand::ReplaceWith` carries no
/// slot for a correlator — so the requester can neither read nor predict its own
/// transaction and must ADOPT the one the router announces.
///
/// ⛔ IT DOES NOT PUBLISH. [`commit_content_generation`] does, at the OTHER end
/// of the frame, and the two were one system until a measured one-frame split
/// forced them apart.
pub fn adopt_preparation_transaction(
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
                // ⛔⛔ **MATCHED ON THE REQUEST IDENTITY, NOT ON THE ROUTE.** A
                // transaction carrying no request id belongs to nobody who is
                // correlating, and `None` is NOT a wildcard — treating it as one
                // is the inference this field exists to remove.
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
            // ⛔ EVERY OTHER EVENT BELONGS TO THE COMMIT HALF, which runs at the
            // OTHER end of the frame. Listed rather than wildcarded so a new
            // terminal event is a compile error in one of the two systems rather
            // than silence in both.
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
/// ⛔⛔⛔ **THIS AND [`adopt_preparation_transaction`] WERE ONE SYSTEM, AND THAT
/// WAS A PRODUCTION BUG.** MEASURED 2026-09-12 in the shipped composition, by
/// driving `build_visible_app` with nothing injected: the activation landed on
/// frame 2, `activate_prepared_platformer_sessions` built the new world on frame
/// 2, and the family published on frame **3**. One frame late, every time.
///
/// ⇒ The cause is that the two jobs want OPPOSITE ends of the frame. Adoption
/// must precede `PlatformerPreparationSet`, which is
/// `in_set(AmbitionLoadSet::Contributors)`; the shell chain is `Contributors →
/// Commands → AmbitionGameShellSet::{Commands, Pending}`, and
/// `advance_pending_route` pushes `RouteActivated` in `Pending`. A system early
/// enough to adopt therefore CANNOT see the activation on the frame it happens —
/// it reads the message next frame, by which time N+1's world has been built out
/// of N's `PreparedCharacterRegistry`. That is exactly the N/N+1 split this road
/// exists to prevent.
///
/// ⛔⛤ **AND THE GRAPH TEST WAS SATISFIED VACUOUSLY.** It asked for `publisher →
/// Providers` and got it — because the publisher ran far EARLIER than both ends.
/// The relationship that matters is `Pending → commit → Providers`, and only
/// naming BOTH ends says so.
///
/// ⭐⭐ **THIS IS THE BOUNDARY THE TWO HALVES SHARE.** The shell's activation
/// publishes the engine's new generation (epoch, content fingerprint, rollback
/// contract); this publishes the cast's and every pack-derived family's. Landing
/// them anywhere else is a half-transaction.
///
/// ⛔ A FAILED PREPARATION DISCARDS THE STAGED REVISION RATHER THAN LEAVING IT.
/// A revision that stayed staged would be applied by whatever activation came
/// next — the content nobody asked for, at a boundary nobody connected it to.
pub fn commit_content_generation(
    mut events: bevy::ecs::message::MessageReader<ambition_platformer2d::game_shell::ShellEvent>,
    mut commands: bevy::prelude::Commands,
) {
    use ambition_platformer2d::game_shell::ShellEvent;
    for event in events.read() {
        match event {
            // ⛔ ADOPTION IS THE OTHER SYSTEM'S JOB and happens EARLIER in the
            // same frame — see [`adopt_preparation_transaction`].
            ShellEvent::PreparationRequested(_) => {}
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
                    // ⛔ EVERY OTHER PARTICIPATING FAMILY LANDS HERE TOO, from the
                    // SAME pack value, in the same queued command. A family
                    // published one boundary later is a half-transaction wearing
                    // a different hat.
                    publish_participant_families(world, &generation.pack);
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
            // ⭐⭐ **THE TERMINAL SIGNAL THAT NAMES ITS OWNER**, which is the one
            // the vocabulary did not have. A superseded transaction used to emit
            // nothing at all, so a pending generation was stranded with no
            // signal and every later save answered `AlreadyPending`.
            ShellEvent::TransactionEnded {
                barrier, request, ..
            } => {
                let load_id = barrier.load_id.clone();
                let request = request.clone();
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    // ⛔ EITHER IDENTITY IS PROOF, AND NEITHER IS INFERRED.
                    // `request` matches a transaction this reload ISSUED even
                    // before the router announced a load for it; `load_id`
                    // matches one it has already adopted. A transaction carrying
                    // neither belongs to somebody else.
                    if !reload_issued(world, request.as_ref())
                        && !reload_owns(world, Some(&load_id))
                    {
                        return;
                    }
                    discard_staged_reload(world);
                });
            }
            // ⭐ **THE ONE REJECTION THAT NAMES A BARRIER.** The barrier went
            // Ready and the commit succeeded, but no prepared session existed to
            // consume — so this transaction will never activate, and it says
            // exactly which transaction.
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
            // ⛔⛔ **IGNORED, AND THE OLD CODE HERE WAS THE REVIEW'S FINDING 3.**
            // It asked `ShellRouter.pending` which load was failing and
            // discarded the staged generation whenever that load was ours —
            // which makes an UNRELATED rejection (`HostNotConfigured`,
            // `UnknownRoute`, a stale activation, a commit rejection) throw away
            // an edit it has nothing to do with, purely because our load
            // happened to be the one in flight. `LoadFailed` carries no load id
            // and `ExperienceFailed` carries an activation id, so neither can be
            // correlated from the event.
            //
            // ⇒ AND NEITHER NEEDS TO BE: every way this transaction actually
            // dies now emits `TransactionEnded` above — supersession and an
            // already-terminal barrier from `start_route`, a barrier that goes
            // terminal while the shell waits from `advance_pending`. An
            // uncorrelatable rejection is somebody else's.
            ShellEvent::ExperienceFailed { .. } | ShellEvent::CommandRejected(_) => {}
            ShellEvent::WaitingForLoad { .. }
            | ShellEvent::RouteDeactivated(_)
            | ShellEvent::ExitRequested => {}
        }
    }
}


/// Did this reload ISSUE the request `request` names?
///
/// ⭐ **ANSWERABLE BEFORE THE ROUTER HAS MINTED A LOAD**, which is what
/// [`reload_owns`] cannot do: between the command and the announcement a reload
/// has no `LoadId` to compare, and that window is exactly where a superseding
/// command lands.
///
/// ⚠ `None` IS NOT A MATCH. A transaction carrying no request id belongs to
/// nobody who is correlating, and treating it as ours is the inference
/// `ShellRequestId` exists to remove.
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
