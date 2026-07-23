//! **Explicit spawn provenance and the one construction plan.**
//!
//! `docs/planning/engine/immutable-content-and-transactional-construction.md`
//! §6.2–6.3 and Phase 3. Two rules drive everything here:
//!
//! > **Plan before mutation.** Construction is decided as a pure value first;
//! > execution consumes that value instead of rediscovering authoritative
//! > decisions while the live world is already half-replaced.
//!
//! > **Provenance is data.** Who requested an entity, and how to rebuild it, is
//! > a component you can read — never a fact recovered by parsing the identity
//! > string the sim happened to generate.
//!
//! ## Why this sits beside `SimId`
//!
//! [`SimId`](crate::sim_id::SimId) is *identity*: which entity this is.
//! [`SpawnOrigin`] is *provenance*: where it came from and what would make it
//! again. They were the same fact for as long as the id's spelling encoded its
//! family — `placement:duel_pca/0` says "the duellist's zeroth child" to a human
//! and, until this module existed, to `heal_projectile_owners` as well, by way
//! of `rsplit_once('/')`. That coupling means the id grammar cannot change
//! without silently changing reconstruction, and it means an entity whose
//! spelling lies about its family (every summoned minion, which lands in the
//! authored `placement:` namespace) is unreconstructable in principle. Splitting
//! the two is the whole point: ids stay legible, provenance stays *readable*.
//!
//! ## What a domain supplies
//!
//! This module is content-free. A domain ([`ConstructionDomain`]) names the two
//! things core cannot know: what one planned row carries
//! ([`ConstructionDomain::Parameters`]) and what its recipes need in hand to
//! execute ([`ConstructionDomain::Services`] — frozen catalogs, a frozen
//! interpreter table). It also supplies the two functions that make a row
//! buildable: [`ConstructionDomain::dispatch`], ONE exhaustive match yielding
//! both a row's recipe identity and the function that populates the root the
//! executor allocated. Neither the caller nor the recipe chooses the pairing or
//! the entity, so neither can get it wrong.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::{Commands, Component, Entity, World};

use crate::sim_id::SimId;

mod registry;
#[cfg(test)]
mod tests;

pub use registry::{
    ConstructionRegistrationError, ConstructionRegistry, RelationCheck, RelationFn, RelationKind,
    RelationOps, RelationVerifyFn,
};

/// A stable internal identity for a construction recipe.
///
/// Internal on purpose: it names a *way of building something*, which is an
/// engine-side decision, not authored content. Authored data selects a recipe;
/// it never spells one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecipeId(String);

impl RecipeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RecipeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where an entity came from, and what would make it again.
///
/// A `Component`, because reconstruction reads it. The three variants are the
/// three origin categories a world can have: something an author declared,
/// something a provider staged into a room, and something the running
/// simulation minted.
///
/// **The recipe is deliberately not repeated here.** The doc's sketch carried a
/// `RecipeId` inside two of the variants, but the planned row already names the
/// recipe; storing it twice creates a state where the two can disagree and
/// nothing says which wins.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpawnOrigin {
    /// An authored declaration: `source` is the authored artifact it was
    /// declared in (a room id), `instance` the stable declaration within it (an
    /// LDtk iid, a placement id).
    Authored { source: String, instance: String },
    /// A provider staged this occupant into a room during construction. It is
    /// not in the authored artifact, so its durable identity is the provider
    /// plus the key the provider staged it under.
    ProviderStaged {
        provider: String,
        room: String,
        instance: String,
    },
    /// The running simulation minted this: a projectile, a summoned minion, a
    /// dropped item. `parent` is the spawner's identity — the fact that used to
    /// be recoverable only by splitting the child's own id string.
    ///
    /// **`parent` is not optional.** A dynamic entity states which spawner it
    /// descends from or it is unreconstructable, so "dynamic, parent unknown"
    /// is not a state worth being able to spell. A spawn site that cannot name
    /// its spawner's identity must refuse to spawn rather than mint a
    /// provenance that says nothing.
    Dynamic { parent: SimId, sequence: u64 },
}

impl SpawnOrigin {
    /// Stable construction-schema identity for the origin category. Unlike
    /// `Debug`, this is a compatibility contract: it is written into plan dumps
    /// and snapshot blobs.
    pub const fn canonical_kind(&self) -> &'static str {
        match self {
            Self::Authored { .. } => "authored",
            Self::ProviderStaged { .. } => "provider-staged",
            Self::Dynamic { .. } => "dynamic",
        }
    }

    /// The spawner this entity descends from, if any. This is the accessor that
    /// replaces parsing a spawned id's `/`-delimited parent prefix.
    pub const fn parent(&self) -> Option<&SimId> {
        match self {
            Self::Dynamic { parent, .. } => Some(parent),
            Self::Authored { .. } | Self::ProviderStaged { .. } => None,
        }
    }

    /// Byte-stable single-line rendering, tab-delimited like every other
    /// canonical dump in the tree. `-` is the explicit absent-value placeholder.
    pub fn canonical_summary(&self) -> String {
        match self {
            Self::Authored { source, instance } => {
                format!("authored\t{source}\t{instance}")
            }
            Self::ProviderStaged {
                provider,
                room,
                instance,
            } => format!("provider-staged\t{provider}\t{room}\t{instance}"),
            Self::Dynamic { parent, sequence } => format!("dynamic\t{parent}\t{sequence}"),
        }
    }
}

/// The domain a construction plan is written against.
///
/// Core plans, validates, orders, and dumps; the domain supplies the payload and
/// the frozen services its recipes read. Keeping this an associated-type pair
/// rather than type erasure means a recipe never downcasts and a plan cannot be
/// executed against the wrong world.
pub trait ConstructionDomain: Send + Sync + 'static + Sized {
    /// What one planned row carries into its recipe.
    type Parameters: Clone + Send + Sync + 'static;
    /// **What one declared RELATION is** — its kind and everything the pairing
    /// carries, as ONE value.
    ///
    /// Some relationships are pure adjacency — a grudge is fully described by
    /// who resents whom. Others are not: a limb's slot and its host-local idle
    /// anchor are both stated relative to the host, so they are facts about the
    /// pairing rather than about either body. Storing them on the limb would put
    /// host-relative data on an entity that does not know its host until the
    /// relation is wired, which is the same shape of mistake as a `parent` field
    /// beside a `SpawnOrigin::Dynamic`.
    ///
    /// **This used to be a payload sitting BESIDE a caller-supplied
    /// `RelationKind`**, and that pair was exactly the mistake this module keeps
    /// finding: two halves of one fact stored where they can disagree. A request
    /// naming `ambition.limb` while carrying a `Grudge` payload passed
    /// preparation, passed validation, passed the registry check — and reached
    /// `unreachable!` inside the wiring function, mid-commit, with the outgoing
    /// room already retired. The kind is now DERIVED from this value by
    /// [`ConstructionDomain::dispatch_relation`], so the mismatch is not a state
    /// that can be written down.
    type Relation: Clone + Send + Sync + 'static;
    /// Frozen services recipes read at execution time. Whatever a domain puts
    /// here is captured before the plan commits, so execution has no fallible
    /// lookup left.
    type Services;

    /// Resolve what builds this row: **its recipe identity and its executor,
    /// from one exhaustive match**.
    ///
    /// Returning both together is the point. This started as two methods — one
    /// deriving a `RecipeId`, one performing construction — and two matches over
    /// the same enum can drift while still compiling: a variant could be
    /// labelled with one recipe's identity and built by another's code, and
    /// nothing would object. One arm now names both, so the label and the
    /// behaviour are chosen in the same place or not at all.
    ///
    /// Exhaustive over `Parameters`, so a new variant with no arm is a compile
    /// error rather than a runtime surprise. And nothing here can fail: every
    /// lookup that could miss resolved in the request builder.
    fn dispatch(parameters: &Self::Parameters) -> RecipeDispatch<Self>;

    /// Resolve what a declared relation IS: **its stable kind, its wiring, and
    /// its postcondition check, from one exhaustive match**.
    ///
    /// The relation counterpart of [`Self::dispatch`], and it exists for the
    /// same reason plus one more. The same one: a kind chosen in one place and
    /// behaviour chosen in another can drift while still compiling. The extra
    /// one: this is also what makes relation wiring **engine-owned**. Ops are
    /// resolved here rather than looked up in the registry, so there is no table
    /// an outside plugin can register executable behaviour into — and therefore
    /// no insertion-order race deciding which implementation of a kind runs.
    ///
    /// Exhaustive over [`Self::Relation`], so a new relation variant with no arm
    /// is a compile error. Infallible, like `dispatch`: every fallible lookup
    /// resolved in the request builder.
    fn dispatch_relation(relation: &Self::Relation) -> RelationDispatch<Self>;

    /// Byte-stable one-line rendering of a row's parameters for the plan dump.
    /// Must not include tabs or newlines.
    fn canonical_summary(parameters: &Self::Parameters) -> String;

    /// Byte-stable rendering of a relation's carried facts for the plan dump.
    /// Must not include tabs or newlines.
    ///
    /// In the dump because it is content: two plans whose limbs fill different
    /// slots describe different worlds, and a dump that rendered them
    /// identically would call them the same plan. The KIND is dumped separately
    /// from [`RelationDispatch::kind`], so this renders only what the kind does
    /// not already say.
    fn canonical_relation_summary(relation: &Self::Relation) -> String;
}

/// What one exhaustive relation dispatch yields: the relation's stable identity
/// and the two frozen halves of its behaviour.
pub struct RelationDispatch<D: ConstructionDomain> {
    /// Stable identity for the dump, the registry check, and the fingerprint.
    pub kind: RelationKind,
    /// How to install it, and how to prove it landed — see [`RelationOps`].
    pub ops: RelationOps<D>,
}

/// What one exhaustive dispatch decision yields: the row's recipe identity and
/// the function that populates its root.
pub struct RecipeDispatch<D: ConstructionDomain> {
    /// Stable identity for the dump, the registry check, and the fingerprint.
    pub recipe: RecipeId,
    /// Populates the root the executor allocated. The root already exists and
    /// already carries its `SimId` and `SpawnOrigin`; this inserts onto it.
    pub construct: ConstructFn<D>,
}

/// Populates one planned row's already-allocated root.
///
/// A recipe cannot choose the entity, return a different one, or hand back
/// something that was already alive — it receives a [`ConstructionRoot`] the
/// executor minted. It also cannot fail: it returns nothing.
pub type ConstructFn<D> = for<'w, 's, 'a> fn(
    &<D as ConstructionDomain>::Parameters,
    ConstructionRoot,
    &mut ConstructionExecCtx<'w, 's, 'a, D>,
);

/// The authoritative entity the executor allocated for one planned row.
///
/// A recipe receives this instead of creating its own body. The inner `Entity`
/// is reachable — a recipe legitimately needs it to insert components and to
/// parent deliberate child entities — but only [`ConstructionPlan`] can mint
/// one, so a recipe cannot nominate a pre-existing entity as a row's root.
///
/// ⚠ **This is the executor invariant, and it is narrower than "one planned
/// row, one authoritative root".** What is mechanically guaranteed is that the
/// executor allocates each nominal planned root and freezes its constructor.
/// What is NOT guaranteed is that the recipe leaves that root alone or refrains
/// from creating authoritative entities beside it: it holds raw `Commands`, so
/// it can despawn the root, restamp it, or spawn ten more. Those are caught
/// after the fact by [`verify_committed_roster`] — the verification invariant —
/// not prevented here. Making every authoritative root an explicit plan row is
/// the future structural invariant, and it is Phase-4 work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstructionRoot(Entity);

impl ConstructionRoot {
    /// The allocated entity, for inserting components onto it.
    pub fn entity(self) -> Entity {
        self.0
    }
}

/// What one plan describes: which content generation, and which room.
///
/// **Session ownership is deliberately absent.** It is a commit-time fact — one
/// prepared room plan is committed by whichever activation requested it, which
/// is why `PlacementLoweringPlan` also takes its `SessionSpawnScope` at
/// `lower_all` rather than at `plan_room`. A domain that needs it carries it in
/// [`ConstructionDomain::Services`], where it is captured alongside the other
/// frozen facts execution reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionScope {
    /// What generation of content this plan is bound to, if any.
    pub binding: ContentBinding,
    /// The room being constructed, when the plan is a room's contents.
    pub room: Option<String>,
}

/// Whether a plan is bound to a generation of prepared content, and which.
///
/// **This replaces a bare `ContentEpoch` whose zero value meant three different
/// things**: "a fixture stated nothing", "a reset rebuilds the content already
/// active so states no new generation", and "a summon is not content at all".
/// Only the last is genuinely not content-bound; the other two were content-
/// bound plans that had simply lost track of which generation they belonged to,
/// and no commit boundary could tell them apart from a legitimately generation-
/// free one. Phase 4 turns staleness into a refusal, and a refusal cannot be
/// built on a sentinel that three unrelated callers spell the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentBinding {
    /// Prepared against one exact generation of prepared content. A commit must
    /// refuse this plan if the active generation has moved on.
    Content(ambition_engine_core::ContentEpoch),
    /// Not derived from prepared content at all — a summon, a projectile, a
    /// dropped item. Built and committed inside a single tick, so it cannot
    /// outlive a reload and has no generation to be stale against.
    RuntimeDynamic,
}

impl ContentBinding {
    /// The generation this plan names, for a commit boundary to compare against
    /// the live one. `None` means the plan is not content-derived and staleness
    /// does not apply to it — NOT that its generation is unknown.
    pub const fn content_epoch(self) -> Option<ambition_engine_core::ContentEpoch> {
        match self {
            Self::Content(epoch) => Some(epoch),
            Self::RuntimeDynamic => None,
        }
    }

    /// Byte-stable rendering for the plan dump.
    pub fn canonical_summary(self) -> String {
        match self {
            Self::Content(epoch) => format!("{epoch}"),
            Self::RuntimeDynamic => "runtime-dynamic".to_string(),
        }
    }
}

impl ConstructionScope {
    /// The ownership token every root this scope constructs under `session` will
    /// carry.
    ///
    /// Derived from the scope and the committing session rather than drawn from
    /// a counter, so it needs no clock, no randomness, and no rollback-registered
    /// state: committing the same room, at the same content generation, in the
    /// same session, twice yields the same token — which is what a deterministic
    /// simulation requires, and what makes a same-room reconstruction recognise
    /// its own previous roots instead of calling them foreign.
    ///
    /// ⚠ **The session is part of the key, and it has to be.** This was
    /// `binding + room` alone, which is a CONSTRUCTION-SCOPE identity, not a
    /// transaction identity: the shell host runs two gameplay sessions in one
    /// process, so two sessions committing the same room at the same content
    /// epoch minted the same token. Each would then classify the other's roots
    /// as [`ScopeClassification::TransactionAuthoritative`] — its own — and
    /// report every one of them as [`RosterViolation::Unplanned`], while a root
    /// genuinely belonging to the other session would be accepted as this one's.
    /// Session ownership is a commit-time fact, so it enters here at commit time
    /// rather than being folded into [`ConstructionScope`].
    pub fn transaction(&self, session: crate::lifecycle::SessionSpawnScope) -> TransactionId {
        TransactionId(format!(
            "{}\t{}\t{}",
            self.binding.canonical_summary(),
            self.room.as_deref().unwrap_or("-"),
            match session.id() {
                Some(id) => format!("session:{id:?}"),
                None => "unscoped".to_string(),
            },
        ))
    }
}

/// Which construction transaction owns an authoritative root.
///
/// **Stamped by the executor, on every root it allocates.** This is what lets
/// verification ask the WORLD which roots are in scope instead of trusting a
/// caller to list them — a caller that forgets the root a recipe invented is
/// exactly the caller whose transaction most needs checking.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(String);

impl TransactionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Declares an identity-bearing entity to be deliberately NOT authoritative:
/// a presentation child, a helper body, a visual double.
///
/// Opt-out rather than opt-in, and that asymmetry is the point. If scope
/// membership required a positive marker, every entity a recipe invented
/// without one would fall silently outside verification — which is precisely
/// the failure being hunted. An identity-bearing entity is authoritative until
/// something says otherwise, so forgetting to classify is a loud violation
/// instead of a quiet exemption.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationOnly;

/// Declares an identity-bearing entity to have been built by a **named,
/// enumerated, not-yet-migrated construction family** rather than by the planner.
///
/// This exists so that "unowned" can stop meaning "probably legacy, carry on".
/// It used to: every entity carrying a `SimId` and no ownership stamp was
/// reported at [`Severity::Unmigrated`] and published anyway, which made the
/// check unable to distinguish a known un-migrated family from a recipe that
/// invented an authoritative entity nobody planned — the exact failure the
/// verifier exists to catch. Now a legacy family must SAY it is one, by name
/// from [`KNOWN_LEGACY_FAMILIES`], and anything else is fatal.
///
/// The list is finite and shrinking, and Phase 4's last step deletes this type
/// along with [`Severity::Unmigrated`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LegacyConstructionRoot {
    /// Which enumerated family. A value outside [`KNOWN_LEGACY_FAMILIES`] is
    /// fatal rather than tolerated — an unrecognised claim of legacy status is
    /// not evidence of legacy status.
    pub family: String,
}

impl LegacyConstructionRoot {
    pub fn new(family: impl Into<String>) -> Self {
        Self {
            family: family.into(),
        }
    }

    pub fn is_known(&self) -> bool {
        KNOWN_LEGACY_FAMILIES.contains(&self.family.as_str())
    }
}

/// The construction families that still mint authoritative identities outside
/// the planner, by name.
///
/// **The campaign's migration ledger, as code.** Each entry is a documented
/// temporary exemption from [`Severity::Fatal`]; the count only goes down, and
/// when it reaches zero both this constant and [`Severity::Unmigrated`] are
/// deleted. Kept here rather than in a domain crate because the exemption is an
/// engine-level policy about what verification tolerates, and because a list a
/// domain could extend at will would not be a shrinking list.
pub const KNOWN_LEGACY_FAMILIES: &[&str] = &[
    // A `giant`-class enemy's two hand limbs, minted inside
    // `spawn_giant_hand_limbs` with `SimId::spawned` under the giant. Removed by
    // Checkpoint B, which makes each hand an explicit plan row.
    "giant-hand-limb",
];

/// One requested entity, before validation.
///
/// **There is no `recipe` field either.** Which recipe builds a row is derived
/// from its parameters by [`ConstructionDomain::dispatch`], so a request that
/// names one recipe while carrying another's payload is not a thing that can be
/// written down.
///
/// **There is no `parent` field.** The spawner an entity descends from is
/// already stated by [`SpawnOrigin::Dynamic`], and a request that carried it
/// twice would have a state where the validated parent and the recorded
/// provenance disagree — with nothing to say which one reconstruction should
/// believe. Preparation validates [`SpawnOrigin::parent`] directly, so the fact
/// that is checked is the same fact that reaches the world.
pub struct ConstructionRequest<D: ConstructionDomain> {
    pub sim_id: SimId,
    pub origin: SpawnOrigin,
    pub parameters: D::Parameters,
    /// Relations this entity declares onto others. Validated against the plan's
    /// own roster plus the live roster before anything is spawned.
    pub relations: Vec<RelationRequest<D>>,
}

/// A declared relation from the requesting entity onto another identity.
///
/// **There is no `kind` field.** It is derived from `relation` by
/// [`ConstructionDomain::dispatch_relation`], for the same reason
/// [`ConstructionRequest`] has no `recipe`: a request that names one kind while
/// carrying another's facts is not a thing that can be written down. It used to
/// be, and the mismatch was caught nowhere — preparation checked the kind
/// against the registry, the registry knew nothing about payloads, and the
/// disagreement surfaced as an `unreachable!` inside the wiring function during
/// commit, after the outgoing room was already gone.
pub struct RelationRequest<D: ConstructionDomain> {
    pub to: SimId,
    /// What this relation IS — see [`ConstructionDomain::Relation`].
    pub relation: D::Relation,
}

impl<D: ConstructionDomain> Clone for RelationRequest<D> {
    fn clone(&self) -> Self {
        Self {
            to: self.to.clone(),
            relation: self.relation.clone(),
        }
    }
}

/// One validated entity with its interpreter already resolved.
///
/// The recipe pointer is stored beside the row for the same reason
/// `PlannedPlacement` stores its lowering function: commit does not repeat
/// registry lookup, and cannot discover a missing recipe after the outgoing
/// world has begun to retire.
pub struct PlannedEntity<D: ConstructionDomain> {
    sim_id: SimId,
    /// Resolved once at preparation via [`ConstructionDomain::dispatch`], and
    /// what the dump, the registry check, and the fingerprint all name.
    recipe: RecipeId,
    /// **The resolved constructor, frozen beside its identity.**
    ///
    /// Commit runs THIS, and never asks the domain again. `dispatch` is
    /// expected to be a pure function of the parameters, but nothing in the
    /// type system makes it one: an implementation may read an atomic, an
    /// environment variable, or any other mutable process state. Re-resolving
    /// at commit therefore allowed a plan to validate recipe A, dump recipe A,
    /// fingerprint recipe A — and execute constructor B. Freezing it here is
    /// what makes "prepared" mean prepared.
    ///
    /// Deliberately absent from every canonical surface: a `fn` address is
    /// runtime execution state, not content identity. The dump and the
    /// fingerprint carry [`Self::recipe`] instead.
    construct: ConstructFn<D>,
    origin: SpawnOrigin,
    parameters: D::Parameters,
}

impl<D: ConstructionDomain> Clone for PlannedEntity<D> {
    fn clone(&self) -> Self {
        Self {
            sim_id: self.sim_id.clone(),
            recipe: self.recipe.clone(),
            construct: self.construct,
            origin: self.origin.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

impl<D: ConstructionDomain> PlannedEntity<D> {
    pub fn sim_id(&self) -> &SimId {
        &self.sim_id
    }
    pub fn recipe(&self) -> &RecipeId {
        &self.recipe
    }
    pub fn origin(&self) -> &SpawnOrigin {
        &self.origin
    }
    /// The spawner this row descends from — read from its provenance, which is
    /// the only place it is stored.
    pub fn parent(&self) -> Option<&SimId> {
        self.origin.parent()
    }
    pub fn parameters(&self) -> &D::Parameters {
        &self.parameters
    }
}

/// One validated relation with **both** its wiring function and its
/// postcondition check already resolved.
///
/// Frozen at preparation for the same reason [`PlannedEntity::construct`] is:
/// commit runs what the plan validated, not whatever a later lookup returns.
/// The pair travels together because a relation is two halves of one fact, and
/// this module's recurring bug has been letting two halves of one fact live in
/// places that can disagree.
pub struct PlannedRelation<D: ConstructionDomain> {
    from: SimId,
    /// Derived at preparation by [`ConstructionDomain::dispatch_relation`] and
    /// frozen here. Commit never redispatches, so the kind that was dumped,
    /// deduplicated, registry-checked, and ordered is the kind that executes.
    kind: RelationKind,
    to: SimId,
    relation: D::Relation,
    ops: RelationOps<D>,
}

impl<D: ConstructionDomain> Clone for PlannedRelation<D> {
    fn clone(&self) -> Self {
        Self {
            from: self.from.clone(),
            kind: self.kind.clone(),
            to: self.to.clone(),
            relation: self.relation.clone(),
            ops: self.ops,
        }
    }
}

impl<D: ConstructionDomain> PlannedRelation<D> {
    pub fn from(&self) -> &SimId {
        &self.from
    }
    pub fn kind(&self) -> &RelationKind {
        &self.kind
    }
    pub fn to(&self) -> &SimId {
        &self.to
    }
    pub fn relation(&self) -> &D::Relation {
        &self.relation
    }
}

/// Why a construction plan could not be prepared, or a single row could not be
/// re-executed. Every variant is detected before any world mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructionError {
    /// Two rows claim the same identity.
    DuplicateIdentity { sim_id: SimId },
    /// A row claims an identity that a live entity already holds.
    IdentityAlreadyLive { sim_id: SimId },
    /// A row names a recipe the registry does not have.
    UnknownRecipe { sim_id: SimId, recipe: RecipeId },
    /// A row's parent resolves to neither a planned nor a live identity.
    UnresolvedParent { sim_id: SimId, parent: SimId },
    /// A relation's target names nothing this plan knows about.
    ///
    /// Both ends of a relation must be rows in the same plan. A target that is
    /// merely *live* is rejected rather than accepted-and-skipped: commit wires
    /// relations from the identities it just constructed, so it has no entity
    /// for an outsider, and quietly dropping the relation would recreate the
    /// exact silent-skip this planner exists to remove. Relating to a live
    /// entity is a real need — it is Phase 4's, alongside the commit boundary
    /// that will hold a live identity index.
    UnresolvedRelation {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// A relation resolves to a kind the registry does not declare.
    UnknownRelationKind { from: SimId, kind: RelationKind },
    /// Two rows declare the same relation between the same two identities.
    ///
    /// Refused rather than deduplicated, because the two are not the same
    /// outcome. Executing a duplicate runs the wiring TWICE while the receipt —
    /// a `BTreeSet` keyed on exactly this triple — records it once, so the
    /// receipt says "wired" and the world holds it applied twice. For an
    /// accumulating relation that is a real corruption: a limb appended to its
    /// host's rig twice is driven twice per frame, and every "is the limb in the
    /// rig" check still passes.
    DuplicateRelation {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// A single-row re-execution named an identity this plan does not contain.
    NotInPlan { sim_id: SimId },
    /// A partial commit would have cut a relation: exactly one of its two ends
    /// is being rebuilt.
    ///
    /// **Both directions are refused, and the reason is that a relation is an
    /// `Entity` handle.** Rebuilding the SOURCE alone leaves it unwired, which
    /// is obvious. Rebuilding the TARGET alone is worse and was briefly allowed
    /// here on the reasoning that the relation "belongs to" the untouched
    /// source: it does, but what the source holds is a handle to the entity
    /// that just died, so the source is left pointing at a corpse. In both
    /// cases the roster is the right length and only the wiring is wrong, which
    /// is the failure mode that survives every count-based check.
    ///
    /// [`ConstructionPlan::relation_closure`] turns a seed set into one that
    /// cannot be refused for this reason.
    RelationCutBySubset {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
}

impl std::fmt::Display for ConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateIdentity { sim_id } => {
                write!(f, "two planned entities claim identity `{sim_id}`")
            }
            Self::IdentityAlreadyLive { sim_id } => write!(
                f,
                "planned identity `{sim_id}` is already held by a live entity"
            ),
            Self::UnknownRecipe { sim_id, recipe } => write!(
                f,
                "`{sim_id}` names construction recipe `{recipe}`, which is not registered"
            ),
            Self::UnresolvedParent { sim_id, parent } => write!(
                f,
                "`{sim_id}` declares parent `{parent}`, which is neither planned nor live"
            ),
            Self::UnresolvedRelation { from, kind, to } => write!(
                f,
                "`{from}` declares relation `{kind}` onto `{to}`, which is neither planned nor live"
            ),
            Self::UnknownRelationKind { from, kind } => write!(
                f,
                "`{from}` declares relation `{kind}`, which no registration declares"
            ),
            Self::DuplicateRelation { from, kind, to } => write!(
                f,
                "relation `{from}` -`{kind}`-> `{to}` is declared more than once: it would be \
                 wired twice and receipted once"
            ),
            Self::NotInPlan { sim_id } => {
                write!(f, "this plan contains no entity `{sim_id}`")
            }
            Self::RelationCutBySubset { from, kind, to } => write!(
                f,
                "this subset cuts relation `{from}` -`{kind}`-> `{to}`: rebuilding one end alone \
                 leaves the other holding a stale entity handle"
            ),
        }
    }
}

impl std::error::Error for ConstructionError {}

/// Execution context handed to a recipe. Mirrors `LoweringCtx`: exactly the
/// facts a recipe needs today, growable by explicit need.
pub struct ConstructionExecCtx<'w, 's, 'a, D: ConstructionDomain> {
    pub commands: &'a mut Commands<'w, 's>,
    /// What the plan describes — content generation and room.
    pub scope: &'a ConstructionScope,
    /// Gameplay-session ownership, captured when this commit was requested.
    ///
    /// Here rather than in [`ConstructionDomain::Services`] for the same reason
    /// `LoweringCtx` carries it beside its context: it is the one fact that
    /// varies between two commits of the SAME frozen plan, so folding it into
    /// the services would force a domain to rebuild them — deep-cloning its
    /// catalogs — once per entity during a reconstruction sweep.
    pub session: crate::lifecycle::SessionSpawnScope,
    pub services: &'a D::Services,
}

/// What execution actually committed. Compared against the plan to prove
/// plan-to-world parity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstructionReceipt {
    committed: BTreeMap<SimId, Entity>,
    relations_wired: BTreeSet<(SimId, RelationKind, SimId)>,
}

impl ConstructionReceipt {
    pub fn committed_ids(&self) -> BTreeSet<SimId> {
        self.committed.keys().cloned().collect()
    }

    pub fn entity(&self, sim_id: &SimId) -> Option<Entity> {
        self.committed.get(sim_id).copied()
    }

    pub fn relations_wired(&self) -> &BTreeSet<(SimId, RelationKind, SimId)> {
        &self.relations_wired
    }

    pub fn len(&self) -> usize {
        self.committed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.committed.is_empty()
    }
}

/// The one prepared artifact for a set of entities and the relations between
/// them. Immutable once prepared: every fallible decision is already made.
pub struct ConstructionPlan<D: ConstructionDomain> {
    scope: ConstructionScope,
    entities: Vec<PlannedEntity<D>>,
    relations: Vec<PlannedRelation<D>>,
}

impl<D: ConstructionDomain> Clone for ConstructionPlan<D> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            entities: self.entities.clone(),
            relations: self.relations.clone(),
        }
    }
}

/// A plan's `Debug` is its canonical dump. When a plan appears in a failure
/// message, the thing worth reading is the roster it would have committed —
/// not a field-by-field rendering of the machinery around it.
impl<D: ConstructionDomain> std::fmt::Debug for ConstructionPlan<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.deterministic_dump())
    }
}

impl<D: ConstructionDomain> ConstructionPlan<D> {
    /// Validate and freeze a set of requests. Pure: it borrows the registry and
    /// the live roster and mutates neither, so a rejected plan cannot have
    /// touched the world.
    ///
    /// Rows are ordered canonically by identity, not by arrival, so two callers
    /// that request the same set in different orders produce byte-identical
    /// plans. Spawn order therefore does not carry meaning — which is the same
    /// rule `SimId` already imposes on snapshot rows.
    pub fn prepare(
        scope: ConstructionScope,
        requests: impl IntoIterator<Item = ConstructionRequest<D>>,
        live: &BTreeSet<SimId>,
        registry: &ConstructionRegistry<D>,
    ) -> Result<Self, ConstructionError> {
        let mut requests: Vec<ConstructionRequest<D>> = requests.into_iter().collect();
        requests.sort_by(|a, b| a.sim_id.cmp(&b.sim_id));

        // Identity first: a duplicate makes every later diagnostic ambiguous.
        let mut planned_ids: BTreeSet<SimId> = BTreeSet::new();
        for request in &requests {
            if live.contains(&request.sim_id) {
                return Err(ConstructionError::IdentityAlreadyLive {
                    sim_id: request.sim_id.clone(),
                });
            }
            if !planned_ids.insert(request.sim_id.clone()) {
                return Err(ConstructionError::DuplicateIdentity {
                    sim_id: request.sim_id.clone(),
                });
            }
        }

        // A PARENT may be live: a summoner outlives the summon it plans. A
        // RELATION target may not — see `ConstructionError::UnresolvedRelation`.
        let parent_resolvable = |id: &SimId| planned_ids.contains(id) || live.contains(id);

        let mut entities = Vec::with_capacity(requests.len());
        let mut relations: Vec<PlannedRelation<D>> = Vec::new();
        let mut declared_relations: BTreeSet<(SimId, RelationKind, SimId)> = BTreeSet::new();
        for request in requests {
            // Derived, not supplied — so it always matches the payload — and
            // resolved together with the executor that will build it.
            let dispatch = D::dispatch(&request.parameters);
            if !registry.has_recipe(&dispatch.recipe) {
                return Err(ConstructionError::UnknownRecipe {
                    sim_id: request.sim_id,
                    recipe: dispatch.recipe,
                });
            }
            // The parent comes from the provenance, not from a second field
            // beside it: the fact validated here is the fact the world receives.
            if let Some(parent) = request.origin.parent() {
                if !parent_resolvable(parent) {
                    return Err(ConstructionError::UnresolvedParent {
                        sim_id: request.sim_id.clone(),
                        parent: parent.clone(),
                    });
                }
            }
            for relation in &request.relations {
                // Derived, not supplied — so the kind always matches what the
                // relation carries — and resolved together with the wiring and
                // the check that will run.
                let dispatch = D::dispatch_relation(&relation.relation);
                if !registry.has_relation(&dispatch.kind) {
                    return Err(ConstructionError::UnknownRelationKind {
                        from: request.sim_id.clone(),
                        kind: dispatch.kind,
                    });
                }
                if !planned_ids.contains(&relation.to) {
                    return Err(ConstructionError::UnresolvedRelation {
                        from: request.sim_id.clone(),
                        kind: dispatch.kind,
                        to: relation.to.clone(),
                    });
                }
                // A duplicate endpoint/kind row is refused HERE, before ordering.
                // Two rows that sort equal would otherwise execute twice and
                // collapse into ONE receipt entry — so the receipt would show a
                // relation wired once while the world had it applied twice, which
                // for an accumulating relation (a limb appended to a rig) is a
                // real corruption that every count-based check passes. Refusing
                // also makes `(from, kind, to)` a total order, so request arrival
                // order cannot reach the dump or the execution sequence.
                if !declared_relations.insert((
                    request.sim_id.clone(),
                    dispatch.kind.clone(),
                    relation.to.clone(),
                )) {
                    return Err(ConstructionError::DuplicateRelation {
                        from: request.sim_id.clone(),
                        kind: dispatch.kind,
                        to: relation.to.clone(),
                    });
                }
                relations.push(PlannedRelation {
                    from: request.sim_id.clone(),
                    kind: dispatch.kind,
                    to: relation.to.clone(),
                    relation: relation.relation.clone(),
                    ops: dispatch.ops,
                });
            }
            entities.push(PlannedEntity {
                sim_id: request.sim_id,
                recipe: dispatch.recipe,
                construct: dispatch.construct,
                origin: request.origin,
                parameters: request.parameters,
            });
        }
        relations.sort_by(|a, b| (&a.from, &a.kind, &a.to).cmp(&(&b.from, &b.kind, &b.to)));

        Ok(Self {
            scope,
            entities,
            relations,
        })
    }

    pub fn scope(&self) -> &ConstructionScope {
        &self.scope
    }

    pub fn entities(&self) -> &[PlannedEntity<D>] {
        &self.entities
    }

    pub fn relations(&self) -> &[PlannedRelation<D>] {
        &self.relations
    }

    /// The identities this plan NOMINATES.
    ///
    /// ⚠ **Not "the exact committed roster".** It is what the plan asked for,
    /// which is a different thing from what the world ends up holding: a recipe
    /// can despawn its root, duplicate an identity onto a second body, or spawn
    /// authoritative entities of its own, and none of that moves this set.
    /// Comparing it to [`ConstructionReceipt::committed_ids`] compares the
    /// executor's bookkeeping against itself and would stay green through every
    /// one of those. [`verify_committed_roster`] compares against the WORLD,
    /// which is the only comparison that can fail for a real reason.
    pub fn planned_ids(&self) -> BTreeSet<SimId> {
        self.entities
            .iter()
            .map(|entity| entity.sim_id.clone())
            .collect()
    }

    pub fn get(&self, sim_id: &SimId) -> Option<&PlannedEntity<D>> {
        self.entities.iter().find(|entity| &entity.sim_id == sim_id)
    }

    /// Grow a seed set until no planned relation crosses its boundary.
    ///
    /// This is the set a caller must despawn and rebuild together for the
    /// result to be correctly wired, and it is what makes
    /// [`ConstructionError::RelationCutBySubset`] a solvable refusal rather
    /// than a dead end: ask for the closure, rebuild that.
    ///
    /// Relations are undirected for this purpose. Rebuilding a target strands
    /// its sources just as surely as rebuilding a source leaves it unwired,
    /// because both sides of the wiring are `Entity` handles minted by the
    /// commit that built them.
    pub fn relation_closure(&self, seeds: &BTreeSet<SimId>) -> BTreeSet<SimId> {
        let mut closed = seeds.clone();
        // Each pass can only add, and the plan is finite, so this terminates in
        // at most one pass per relation.
        loop {
            let mut grew = false;
            for relation in &self.relations {
                let has_from = closed.contains(&relation.from);
                let has_to = closed.contains(&relation.to);
                if has_from != has_to {
                    closed.insert(if has_from {
                        relation.to.clone()
                    } else {
                        relation.from.clone()
                    });
                    grew = true;
                }
            }
            if !grew {
                return closed;
            }
        }
    }

    /// Construct one planned entity through its frozen recipe.
    ///
    /// Reconstruction of a single entity. Refuses — before mutating — if this
    /// row sits at EITHER end of a planned relation, because rebuilding one end
    /// alone strands the other on a dead `Entity` handle; see
    /// [`ConstructionError::RelationCutBySubset`] and
    /// [`ConstructionPlan::relation_closure`].
    pub fn construct_one(
        &self,
        sim_id: &SimId,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Result<Entity, ConstructionError> {
        let subset = BTreeSet::from([sim_id.clone()]);
        let receipt = self.commit_subset(&subset, ctx)?;
        Ok(receipt
            .entity(sim_id)
            .unwrap_or_else(|| unreachable!("a one-row commit that succeeded committed its row")))
    }

    /// Construct every planned entity, then wire every planned relation.
    ///
    /// Relations run second because a relation names identities, and an
    /// identity has no entity until its row has been committed. That ordering
    /// is what lets a plan express a mutual pair (two duellists grudging each
    /// other) without either row needing the other to exist first.
    pub fn commit(&self, ctx: &mut ConstructionExecCtx<'_, '_, '_, D>) -> ConstructionReceipt {
        self.execute(None, ctx).unwrap_or_else(|error| {
            unreachable!(
                "committing a plan in full names only its own rows and encloses every relation, \
                 so it cannot be refused: {error}"
            )
        })
    }

    /// Construct the named rows, and wire exactly the relations that lie wholly
    /// within them.
    ///
    /// **This is the only executor.** A full commit is this over every row; a
    /// single-entity rebuild is this over one. Ordinary construction and
    /// reconstruction cannot drift because there is nothing for them to drift
    /// between.
    ///
    /// Every refusal happens before the first recipe runs, so a rejected subset
    /// leaves the world exactly as it found it. A subset containing exactly ONE
    /// end of a planned relation is such a refusal, in either direction:
    /// rebuilding the source alone leaves it unwired, and rebuilding the target
    /// alone leaves the untouched source holding a handle to the entity that
    /// just died. See [`ConstructionError::RelationCutBySubset`], and
    /// [`ConstructionPlan::relation_closure`] for the set that cannot be cut.
    pub fn commit_subset(
        &self,
        ids: &BTreeSet<SimId>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Result<ConstructionReceipt, ConstructionError> {
        self.execute(Some(ids), ctx)
    }

    /// `None` means every row — which is why a full commit allocates nothing to
    /// describe itself and skips validation that cannot fail. Naming the whole
    /// roster explicitly would clone every `SimId` in the plan, and a
    /// reconstruction sweep calling `construct_one` per entity would pay that
    /// once per entity.
    fn execute(
        &self,
        subset: Option<&BTreeSet<SimId>>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Result<ConstructionReceipt, ConstructionError> {
        let included = |id: &SimId| subset.is_none_or(|ids| ids.contains(id));
        if let Some(ids) = subset {
            if let Some(missing) = ids
                .iter()
                .find(|id| !self.entities.iter().any(|e| &e.sim_id == *id))
            {
                return Err(ConstructionError::NotInPlan {
                    sim_id: missing.clone(),
                });
            }
            // A relation must be wholly in or wholly out. Cutting it either way
            // strands an `Entity` handle — see `RelationCutBySubset`.
            for relation in &self.relations {
                if ids.contains(&relation.from) != ids.contains(&relation.to) {
                    return Err(ConstructionError::RelationCutBySubset {
                        from: relation.from.clone(),
                        kind: relation.kind.clone(),
                        to: relation.to.clone(),
                    });
                }
            }
        }

        let mut receipt = ConstructionReceipt::default();
        for planned in self.entities.iter().filter(|e| included(&e.sim_id)) {
            let entity = Self::commit_entity(planned, ctx);
            receipt.committed.insert(planned.sim_id.clone(), entity);
        }
        for relation in self.relations.iter().filter(|r| included(&r.from)) {
            // Both ends are rows in this subset — the refusal above guarantees
            // it — and every row is now committed, so a miss here is a planner
            // bug rather than a content error. It must not be swallowed.
            let (Some(from), Some(to)) = (
                receipt.committed.get(&relation.from).copied(),
                receipt.committed.get(&relation.to).copied(),
            ) else {
                unreachable!(
                    "planned relation {} -> {} names an identity this commit did not build",
                    relation.from, relation.to
                )
            };
            (relation.ops.wire)(from, to, &relation.relation, ctx);
            receipt.relations_wired.insert((
                relation.from.clone(),
                relation.kind.clone(),
                relation.to.clone(),
            ));
        }
        Ok(receipt)
    }

    /// Allocate this row's authoritative root, stamp it, and hand it to the
    /// domain to populate.
    ///
    /// **The executor creates the entity; the recipe never does.** This
    /// previously ran the recipe and trusted whatever `Entity` came back,
    /// guarded only by a deferred check that the returned entity did not
    /// already hold a `SimId`. That guard was weak in three ways a redesign
    /// removes rather than patches: a pre-existing entity WITHOUT a `SimId`
    /// passed it and was silently commandeered; the check ran at flush, so it
    /// was a panic after other rows had queued their mutations rather than a
    /// refusal; and nothing tied the returned entity to this commit at all.
    ///
    /// Allocating here makes freshness structural. `spawn_empty` yields an
    /// entity that by definition nothing else holds, so one planned row is one
    /// distinct new root, and there is no check to get wrong.
    fn commit_entity(
        planned: &PlannedEntity<D>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Entity {
        let root = ctx.commands.spawn_empty().id();
        // Identity, provenance, and transaction ownership go on before the
        // recipe runs, so a recipe cannot forget them and reconstruction never
        // sees a body without provenance. A recipe that inspects its own root
        // finds them already there. The ownership stamp is what lets
        // verification enumerate this transaction's roots from the world.
        ctx.commands.entity(root).insert((
            planned.sim_id.clone(),
            planned.origin.clone(),
            ctx.scope.transaction(ctx.session),
        ));
        // The constructor preparation resolved — NOT a fresh dispatch. A domain
        // whose `dispatch` reads mutable state would otherwise let commit run a
        // different constructor than the one the plan validated and dumped.
        (planned.construct)(&planned.parameters, ConstructionRoot(root), ctx);
        root
    }

    /// Byte-stable inspection surface, in the same tab-delimited shape as
    /// `PreparedContent::deterministic_dump`. Two plans over equivalent input
    /// produce identical bytes regardless of request order.
    pub fn deterministic_dump(&self) -> String {
        use std::fmt::Write as _;
        let mut out = format!(
            "construction-plan-v{CONSTRUCTION_PLAN_SCHEMA_VERSION}\n{}\nroom\t{}\n",
            self.scope.binding.canonical_summary(),
            self.scope.room.as_deref().unwrap_or("-"),
        );
        for entity in &self.entities {
            // No separate parent column: `canonical_summary` already carries it
            // for the one origin that has one, and printing it twice would let
            // a dump disagree with itself.
            let _ = writeln!(
                out,
                "entity\t{}\t{}\t{}\t{}",
                entity.sim_id,
                entity.recipe,
                entity.origin.canonical_summary(),
                D::canonical_summary(&entity.parameters),
            );
        }
        for relation in &self.relations {
            let _ = writeln!(
                out,
                "relation\t{}\t{}\t{}\t{}",
                relation.from,
                relation.kind,
                relation.to,
                D::canonical_relation_summary(&relation.relation),
            );
        }
        out
    }
}

/// Check that the world a commit produced is the world the plan described.
///
/// **This is a detector, not a preventer, and the distinction is the whole
/// point of having it.** The executor allocates each row's root, but a recipe
/// receives raw `Commands` and the root `Entity`, so it can despawn that root,
/// strip or rewrite its `SimId`/`SpawnOrigin`, stamp a second entity with a
/// planned identity, or spawn further authoritative entities of its own — the
/// giant hand limbs already do the last of these. None of that is structurally
/// prevented today, so a transaction that intends to publish a room must ask.
///
/// ⚠ **Bevy commands do not roll back.** By the time this can run, the
/// construction commands have applied. A violation here therefore cannot be
/// undone — it can only stop the transaction being PUBLISHED as successful, and
/// leaves the world in whatever state the offending recipe produced. That is
/// strictly better than publishing a room nobody can describe, and strictly
/// worse than the structural fix (every authoritative root an explicit plan
/// row), which is Phase-4 work.
///
/// **The scope is read from the world, not supplied.** An earlier version took
/// a caller-curated `&[(SimId, Entity)]`, which made the check exactly as
/// complete as the caller's imagination: the roots most worth catching are the
/// ones nobody thought to list. [`AuthoritativeScope::gather`] queries instead,
/// and treats an unclassified identity-bearing entity as a finding rather than
/// as absent.
pub fn verify_committed_roster<D: ConstructionDomain>(
    plan: &ConstructionPlan<D>,
    receipt: &ConstructionReceipt,
    baseline: &TransactionBaseline,
    scope: &AuthoritativeScope,
    world: &World,
) -> Result<(), Vec<RosterViolation>> {
    let mut violations = Vec::new();
    let live = |entity: Entity| world.get_entity(entity).is_ok();

    // Counted, not set-compared: a duplicate identity is invisible to a set,
    // and "the identity set is exactly right while two bodies answer to one of
    // them" is the failure this whole function exists for. Presentation-only
    // entities are excluded by classification, not by their spelling.
    let mut occupants: BTreeMap<&SimId, Vec<Entity>> = BTreeMap::new();
    for member in scope.members() {
        if member.classification != ScopeClassification::PresentationOnly {
            occupants
                .entry(&member.sim_id)
                .or_default()
                .push(member.entity);
        }
    }
    let occupants_of = |sim_id: &SimId| occupants.get(sim_id).map_or(&[][..], Vec::as_slice);

    let planned_ids = plan.planned_ids();

    // ── Baseline preservation ────────────────────────────────────────────────
    //
    // Every identity that was live when the transaction opened, and that the
    // transaction did not declare it was retiring or reconstructing, must come
    // out the far side untouched: same identity, one occupant, the SAME entity
    // it started on, and the same provenance. Checking the identity alone would
    // accept a baseline root despawned and replaced by a look-alike, which is
    // the case that motivated capturing entities in the first place.
    for (sim_id, entry) in baseline.entries() {
        if baseline.is_retired(sim_id) {
            if !occupants_of(sim_id).is_empty() {
                violations.push(RosterViolation::RetiredSurvived {
                    sim_id: sim_id.clone(),
                });
            }
            continue;
        }
        if baseline.is_reconstructed(sim_id) {
            // The declared-reconstruction contract: the old body is gone, the
            // new one is the receipt's, and there is exactly one of it.
            if live(entry.entity) {
                violations.push(RosterViolation::ReconstructedOldSurvived {
                    sim_id: sim_id.clone(),
                    stale: entry.entity,
                });
            }
            match occupants_of(sim_id) {
                [] => violations.push(RosterViolation::Missing {
                    sim_id: sim_id.clone(),
                }),
                [found] => {
                    if receipt.entity(sim_id) != Some(*found) {
                        violations.push(RosterViolation::MovedRoot {
                            sim_id: sim_id.clone(),
                        });
                    }
                }
                found => violations.push(RosterViolation::Duplicated {
                    sim_id: sim_id.clone(),
                    count: found.len(),
                }),
            }
            continue;
        }
        // Not retired, not reconstructed: preserved.
        if planned_ids.contains(sim_id) {
            violations.push(RosterViolation::PlannedOverBaseline {
                sim_id: sim_id.clone(),
            });
        }
        match occupants_of(sim_id) {
            [] => violations.push(RosterViolation::BaselineLost {
                sim_id: sim_id.clone(),
            }),
            [found] if *found == entry.entity => {
                let now = world.get::<SpawnOrigin>(entry.entity).cloned();
                if now != entry.origin {
                    violations.push(RosterViolation::BaselineProvenanceChanged {
                        sim_id: sim_id.clone(),
                        expected: entry.origin.clone(),
                        found: now,
                    });
                }
            }
            // The identity survived on a DIFFERENT entity: something despawned
            // the baseline body and minted a replacement wearing its name. A
            // set comparison sees a perfectly intact roster here.
            [found] => violations.push(RosterViolation::BaselineReplaced {
                sim_id: sim_id.clone(),
                expected: entry.entity,
                found: *found,
            }),
            found => violations.push(RosterViolation::Duplicated {
                sim_id: sim_id.clone(),
                count: found.len(),
            }),
        }
    }

    // ── Planned rows ─────────────────────────────────────────────────────────
    for planned in plan.entities() {
        let expected_root = receipt.entity(&planned.sim_id);
        if expected_root.is_none() {
            // Not part of this commit's subset; its relations are skipped below
            // for the same reason.
            continue;
        }
        match occupants_of(&planned.sim_id) {
            [] => violations.push(RosterViolation::Missing {
                sim_id: planned.sim_id.clone(),
            }),
            [found] => {
                if expected_root != Some(*found) {
                    violations.push(RosterViolation::MovedRoot {
                        sim_id: planned.sim_id.clone(),
                    });
                }
            }
            found => violations.push(RosterViolation::Duplicated {
                sim_id: planned.sim_id.clone(),
                count: found.len(),
            }),
        }
        if let Some(root) = expected_root {
            if !live(root) {
                violations.push(RosterViolation::Missing {
                    sim_id: planned.sim_id.clone(),
                });
            } else {
                // The executor stamped this before the recipe ran; a recipe that
                // overwrote or removed it produced a body no restore can place.
                let found = world.get::<SpawnOrigin>(root).cloned();
                if found.as_ref() != Some(&planned.origin) {
                    violations.push(RosterViolation::ProvenanceChanged {
                        sim_id: planned.sim_id.clone(),
                        expected: planned.origin.clone(),
                        found,
                    });
                }
                // Ownership is stamped in the same insert as identity and
                // provenance, so checking those two and not this one left the
                // stamp that DRIVES scope classification as the only part of the
                // executor's mark nothing confirmed.
                let owner = world.get::<TransactionId>(root);
                if owner != Some(scope.transaction()) {
                    violations.push(RosterViolation::OwnershipLost {
                        sim_id: planned.sim_id.clone(),
                        expected: scope.transaction().clone(),
                        found: owner.cloned(),
                    });
                }
            }
        }
    }

    // ── Everything else the world holds in this scope ────────────────────────
    for member in scope.members() {
        if member.classification == ScopeClassification::PresentationOnly {
            continue;
        }
        if planned_ids.contains(&member.sim_id) || baseline.contains(&member.sim_id) {
            continue;
        }
        violations.push(match &member.classification {
            // Stamped by THIS transaction's executor, yet no plan row named it.
            ScopeClassification::TransactionAuthoritative => RosterViolation::Unplanned {
                sim_id: member.sim_id.clone(),
            },
            // Identity-bearing, appeared during this transaction, and nothing in
            // the world says what built it. FATAL — see `RosterViolation::severity`.
            ScopeClassification::Unowned => RosterViolation::UnownedIdentity {
                sim_id: member.sim_id.clone(),
            },
            ScopeClassification::UnknownLegacyFamily { family } => {
                RosterViolation::UnknownLegacyFamily {
                    sim_id: member.sim_id.clone(),
                    family: family.clone(),
                }
            }
            ScopeClassification::KnownLegacy { family } => RosterViolation::LegacyConstruction {
                sim_id: member.sim_id.clone(),
                family: family.clone(),
            },
            ScopeClassification::ForeignScope(_) | ScopeClassification::PresentationOnly => {
                continue
            }
        });
    }

    // ── Relation postconditions ──────────────────────────────────────────────
    //
    // The receipt records that a wiring function was CALLED. That is a fact
    // about the executor, not about the world: a wiring function that does
    // nothing, writes to the wrong entity, or is overwritten by a later command
    // produces an identical receipt. Each relation's frozen verifier reads the
    // committed components instead.
    for relation in plan.relations() {
        let key = (
            relation.from.clone(),
            relation.kind.clone(),
            relation.to.clone(),
        );
        // Which relations this commit OWED is derived from the identities it
        // actually committed, not from the receipt's own account of what it did.
        // A subset commit encloses every relation it touches (see
        // `RelationCutBySubset`), so both-endpoints-committed is exactly "this
        // relation was in scope" — for a full commit that is every planned
        // relation, and for a subset it is the ones wholly inside it.
        let (Some(from), Some(to)) = (receipt.entity(&relation.from), receipt.entity(&relation.to))
        else {
            continue;
        };
        if !receipt.relations_wired().contains(&key) {
            // In scope and not wired. This used to `continue`, which meant the
            // postcondition pass inspected every relation EXCEPT the ones the
            // executor had failed to attempt.
            violations.push(RosterViolation::RelationMissingFromReceipt {
                from: relation.from.clone(),
                kind: relation.kind.clone(),
                to: relation.to.clone(),
            });
            continue;
        }
        if !live(from) || !live(to) {
            violations.push(RosterViolation::DanglingRelation {
                from: relation.from.clone(),
                kind: relation.kind.clone(),
                to: relation.to.clone(),
            });
            continue;
        }
        let check = (relation.ops.verify)(world, from, to, &relation.relation);
        if check != RelationCheck::Installed {
            violations.push(RosterViolation::RelationNotEstablished {
                from: relation.from.clone(),
                kind: relation.kind.clone(),
                to: relation.to.clone(),
                expected: to,
                check,
            });
        }
    }

    violations.sort_by_key(|violation| format!("{violation:?}"));
    violations.dedup();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// How a committed plan failed to match the world it was supposed to build.
///
/// Structured rather than logged, because the caller's correct response is to
/// refuse the transaction, not to carry on with a world it cannot describe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RosterViolation {
    /// A planned row produced no live entity carrying its identity.
    Missing { sim_id: SimId },
    /// More than one live entity carries one identity. **This is the case a
    /// `BTreeSet<SimId>` comparison cannot see**: the set of identities looks
    /// exactly right while two bodies answer to one of them.
    Duplicated { sim_id: SimId, count: usize },
    /// The identity exists, but not on the entity the executor allocated for
    /// it — the recipe moved it, or despawned the root and rebuilt elsewhere.
    MovedRoot { sim_id: SimId },
    /// The root lost or had rewritten the provenance the executor stamped.
    ProvenanceChanged {
        sim_id: SimId,
        expected: SpawnOrigin,
        found: Option<SpawnOrigin>,
    },
    /// A transaction-scoped authoritative root exists that no plan row named.
    /// Recipes that create authoritative entities internally land here.
    Unplanned { sim_id: SimId },
    /// An identity-bearing entity appeared during this transaction carrying no
    /// ownership stamp and no explicit classification. **Fatal.**
    ///
    /// This was tolerated on the reasoning that it meant "a family that has not
    /// migrated". It did not mean that — it meant nobody knew what the entity
    /// was, which is equally the signature of a recipe inventing an
    /// authoritative root. A genuine legacy family now says so with
    /// [`LegacyConstructionRoot`], leaving this variant to mean what it says.
    UnownedIdentity { sim_id: SimId },
    /// An entity claims [`LegacyConstructionRoot`] with a family name that is not
    /// in [`KNOWN_LEGACY_FAMILIES`]. **Fatal**, so the marker cannot become a
    /// universal opt-out from verification.
    UnknownLegacyFamily { sim_id: SimId, family: String },
    /// An explicitly-marked known-legacy root. Reported, not fatal, temporary.
    LegacyConstruction { sim_id: SimId, family: String },
    /// A planned root does not carry this transaction's ownership stamp.
    ///
    /// The executor stamps it before the recipe runs, so this means a recipe
    /// removed it, overwrote it, or moved the identity onto a body that never
    /// had it. An unowned planned root is invisible to the next transaction's
    /// scope gathering, so it would be counted as somebody else's problem
    /// forever.
    OwnershipLost {
        sim_id: SimId,
        expected: TransactionId,
        found: Option<TransactionId>,
    },
    /// A planned relation whose two endpoints were both committed does not
    /// appear in the receipt: the executor never wired it.
    ///
    /// Skipped silently before, which made the relation postcondition pass
    /// vacuous for exactly the relations that failed hardest — a relation the
    /// executor never attempted has no receipt entry, so "verify the ones that
    /// were wired" verified everything except the broken one.
    RelationMissingFromReceipt {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// A baseline identity the transaction did not declare it was touching is
    /// no longer in the world.
    BaselineLost { sim_id: SimId },
    /// A baseline identity survived, **on a different entity**. Something
    /// despawned the original and minted a replacement wearing its name. The
    /// roster is exactly the right length and every identity is present, which
    /// is why identity-only comparison is not enough.
    BaselineReplaced {
        sim_id: SimId,
        expected: Entity,
        found: Entity,
    },
    /// An untouched baseline entity's provenance was rewritten under it.
    ///
    /// Separate from [`Self::ProvenanceChanged`] because a baseline entity may
    /// legitimately carry none — a persistent player is not a construction
    /// product — so `expected` is an `Option` here and is not one there.
    BaselineProvenanceChanged {
        sim_id: SimId,
        expected: Option<SpawnOrigin>,
        found: Option<SpawnOrigin>,
    },
    /// An identity the transaction declared it was retiring is still present.
    RetiredSurvived { sim_id: SimId },
    /// A declared reconstruction left the pre-reconstruction body alive, so two
    /// generations of one identity coexist and dependants may hold either.
    ReconstructedOldSurvived { sim_id: SimId, stale: Entity },
    /// A plan row names an identity that was already live and was not declared
    /// a reconstruction, so committing it creates a second body for it.
    PlannedOverBaseline { sim_id: SimId },
    /// A wired relation names an entity that is not live.
    DanglingRelation {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// The wiring function ran and the world does not hold the relation.
    ///
    /// **The receipt cannot see this.** It records that a function was called,
    /// which a no-op, a write to the wrong entity, and a later overwrite all
    /// satisfy identically.
    RelationNotEstablished {
        from: SimId,
        kind: RelationKind,
        to: SimId,
        expected: Entity,
        check: RelationCheck,
    },
}

/// Whether a violation means the transaction is unpublishable, or names a
/// known un-migrated family.
///
/// **This distinction is load-bearing and temporary.** Nine authoritative
/// families still construct roots through family-specific loops rather than as
/// plan rows — the giant's hand limbs mint a `SimId` directly, for one — so
/// treating every unowned identity as fatal today would refuse rooms that are
/// working exactly as designed. Reporting them keeps the finding honest and
/// visible without pretending the migration is finished; as each family becomes
/// a plan row the class empties on its own, and Phase 4's last step is to delete
/// [`Severity::Unmigrated`] and let the remainder be fatal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The transaction built something other than what it planned. Do not
    /// publish.
    Fatal,
    /// A known-unmigrated family produced this. Report it; publish anyway.
    Unmigrated,
}

impl RosterViolation {
    pub const fn severity(&self) -> Severity {
        match self {
            // The ONE remaining tolerated class, and it is tolerated only
            // because the family named itself and the name is enumerated.
            Self::LegacyConstruction { .. } => Severity::Unmigrated,
            Self::UnownedIdentity { .. }
            | Self::UnknownLegacyFamily { .. }
            | Self::OwnershipLost { .. }
            | Self::RelationMissingFromReceipt { .. }
            | Self::Missing { .. }
            | Self::Duplicated { .. }
            | Self::MovedRoot { .. }
            | Self::ProvenanceChanged { .. }
            | Self::Unplanned { .. }
            | Self::BaselineLost { .. }
            | Self::BaselineReplaced { .. }
            | Self::BaselineProvenanceChanged { .. }
            | Self::RetiredSurvived { .. }
            | Self::ReconstructedOldSurvived { .. }
            | Self::PlannedOverBaseline { .. }
            | Self::DanglingRelation { .. }
            | Self::RelationNotEstablished { .. } => Severity::Fatal,
        }
    }
}

impl std::fmt::Display for RosterViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { sim_id } => {
                write!(f, "planned identity `{sim_id}` is not in the world")
            }
            Self::Duplicated { sim_id, count } => write!(
                f,
                "planned identity `{sim_id}` is on {count} entities; exactly one was expected"
            ),
            Self::MovedRoot { sim_id } => write!(
                f,
                "planned identity `{sim_id}` is not on the root the executor allocated for it"
            ),
            Self::ProvenanceChanged {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "`{sim_id}` should carry provenance `{}` but carries `{}`",
                expected.canonical_kind(),
                found.as_ref().map_or("none", SpawnOrigin::canonical_kind),
            ),
            Self::Unplanned { sim_id } => write!(
                f,
                "authoritative identity `{sim_id}` exists in this transaction but no plan row \
                 named it"
            ),
            Self::UnownedIdentity { sim_id } => write!(
                f,
                "`{sim_id}` appeared during this transaction carrying no construction ownership \
                 and no classification: nothing in the world says what built it"
            ),
            Self::UnknownLegacyFamily { sim_id, family } => write!(
                f,
                "`{sim_id}` claims legacy construction family `{family}`, which is not one of the \
                 enumerated un-migrated families"
            ),
            Self::LegacyConstruction { sim_id, family } => write!(
                f,
                "`{sim_id}` was built by known un-migrated family `{family}`, which is not a plan \
                 row yet"
            ),
            Self::OwnershipLost {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "planned root `{sim_id}` should be owned by transaction `{expected}` but carries \
                 `{}`",
                found.as_ref().map_or("none", TransactionId::as_str),
            ),
            Self::RelationMissingFromReceipt { from, kind, to } => write!(
                f,
                "planned relation `{from}` -`{kind}`-> `{to}` has both endpoints committed but \
                 the executor never wired it"
            ),
            Self::BaselineLost { sim_id } => write!(
                f,
                "`{sim_id}` was live when this transaction opened and this transaction did not \
                 declare it was touching it, but it is gone"
            ),
            Self::BaselineReplaced {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "`{sim_id}` was live on {expected:?} and is now on {found:?}: the original was \
                 replaced by a different entity wearing its identity"
            ),
            Self::BaselineProvenanceChanged {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "`{sim_id}` was untouched by this transaction but its provenance changed from \
                 `{}` to `{}`",
                expected
                    .as_ref()
                    .map_or("none", SpawnOrigin::canonical_kind),
                found.as_ref().map_or("none", SpawnOrigin::canonical_kind),
            ),
            Self::RetiredSurvived { sim_id } => write!(
                f,
                "`{sim_id}` was declared retired by this transaction but is still in the world"
            ),
            Self::ReconstructedOldSurvived { sim_id, stale } => write!(
                f,
                "`{sim_id}` was reconstructed but its previous body {stale:?} is still alive, so \
                 two generations of one identity coexist"
            ),
            Self::PlannedOverBaseline { sim_id } => write!(
                f,
                "`{sim_id}` is a plan row and was already live, without being declared a \
                 reconstruction"
            ),
            Self::DanglingRelation { from, kind, to } => write!(
                f,
                "wired relation `{from}` -`{kind}`-> `{to}` names an entity that is not live"
            ),
            Self::RelationNotEstablished {
                from,
                kind,
                to,
                expected,
                check,
            } => write!(
                f,
                "relation `{from}` -`{kind}`-> `{to}` was wired but the world does not hold it \
                 onto {expected:?}: {check:?}"
            ),
        }
    }
}

impl std::error::Error for RosterViolation {}

/// What one baseline identity was sitting on when the transaction opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineEntry {
    /// The exact entity. **Not just the identity** — a baseline root despawned
    /// and replaced by another entity carrying the same `SimId` leaves the
    /// identity set untouched, so an identity-only baseline cannot see it.
    pub entity: Entity,
    /// Its provenance at capture, so a transaction that quietly rewrites an
    /// untouched entity's origin is a finding rather than a surprise later.
    pub origin: Option<SpawnOrigin>,
}

/// The world a transaction opened against: which identities were live, **on
/// which entities**, carrying which provenance — plus what the transaction
/// declared it was going to do to them.
///
/// Explicit rather than inferred: nothing here parses a `SimId`. And permission
/// to remove or replace an identity is *declared*, never deduced from the
/// candidate plan — inferring it would mean any plan naming an identity thereby
/// authorised destroying whatever already held it, which is the opposite of a
/// check.
#[derive(Clone, Debug, Default)]
pub struct TransactionBaseline {
    entries: BTreeMap<SimId, BaselineEntry>,
    retired: BTreeSet<SimId>,
    reconstructed: BTreeSet<SimId>,
}

/// Why a baseline could not be captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaselineCaptureError {
    /// Two live entities already held one identity before the transaction even
    /// started. Captured as a refusal rather than silently collapsed, because
    /// every later multiplicity check would be measured against a baseline that
    /// had already lost the duplicate.
    DuplicateIdentity {
        sim_id: SimId,
        entities: Vec<Entity>,
    },
}

impl std::fmt::Display for BaselineCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateIdentity { sim_id, entities } => write!(
                f,
                "identity `{sim_id}` is already on {} entities before this transaction began: \
                 {entities:?}",
                entities.len()
            ),
        }
    }
}

impl std::error::Error for BaselineCaptureError {}

impl TransactionBaseline {
    /// Capture every AUTHORITATIVE identity-bearing entity in the world, with
    /// its entity and provenance. Duplicates are a refusal, not a merge.
    ///
    /// [`PresentationOnly`] entities are excluded, and must be: verification
    /// counts occupants with the same exclusion, so a presentation-only entity
    /// admitted here would be looked for among occupants that structurally
    /// cannot contain it and reported as [`RosterViolation::BaselineLost`] every
    /// time. The two filters are the same filter or the baseline is measuring
    /// something the verifier is not.
    pub fn capture(world: &mut World) -> Result<Self, BaselineCaptureError> {
        let mut found: BTreeMap<SimId, Vec<(Entity, Option<SpawnOrigin>)>> = BTreeMap::new();
        let mut query = world.query_filtered::<
            (Entity, &SimId, Option<&SpawnOrigin>),
            bevy::prelude::Without<PresentationOnly>,
        >();
        for (entity, sim_id, origin) in query.iter(world) {
            found
                .entry(sim_id.clone())
                .or_default()
                .push((entity, origin.cloned()));
        }
        Self::from_occupants(found)
    }

    /// Capture from explicit pairs, for fixtures and for callers that already
    /// hold the roster. Duplicates refuse exactly as they do in [`Self::capture`].
    pub fn from_pairs(
        pairs: impl IntoIterator<Item = (SimId, Entity, Option<SpawnOrigin>)>,
    ) -> Result<Self, BaselineCaptureError> {
        let mut found: BTreeMap<SimId, Vec<(Entity, Option<SpawnOrigin>)>> = BTreeMap::new();
        for (sim_id, entity, origin) in pairs {
            found.entry(sim_id).or_default().push((entity, origin));
        }
        Self::from_occupants(found)
    }

    fn from_occupants(
        found: BTreeMap<SimId, Vec<(Entity, Option<SpawnOrigin>)>>,
    ) -> Result<Self, BaselineCaptureError> {
        let mut entries = BTreeMap::new();
        for (sim_id, mut occupants) in found {
            if occupants.len() > 1 {
                occupants.sort_by_key(|(entity, _)| *entity);
                return Err(BaselineCaptureError::DuplicateIdentity {
                    sim_id,
                    entities: occupants.into_iter().map(|(entity, _)| entity).collect(),
                });
            }
            let (entity, origin) = occupants.remove(0);
            entries.insert(sim_id, BaselineEntry { entity, origin });
        }
        Ok(Self {
            entries,
            retired: BTreeSet::new(),
            reconstructed: BTreeSet::new(),
        })
    }

    /// Declare that this transaction intends to remove these identities without
    /// replacing them.
    pub fn retiring(mut self, ids: impl IntoIterator<Item = SimId>) -> Self {
        self.retired.extend(ids);
        self
    }

    /// Declare that this transaction intends to despawn these identities' bodies
    /// and build new ones for the same identities.
    pub fn reconstructing(mut self, ids: impl IntoIterator<Item = SimId>) -> Self {
        self.reconstructed.extend(ids);
        self
    }

    pub fn entries(&self) -> &BTreeMap<SimId, BaselineEntry> {
        &self.entries
    }

    pub fn contains(&self, sim_id: &SimId) -> bool {
        self.entries.contains_key(sim_id)
    }

    pub fn is_retired(&self, sim_id: &SimId) -> bool {
        self.retired.contains(sim_id)
    }

    pub fn is_reconstructed(&self, sim_id: &SimId) -> bool {
        self.reconstructed.contains(sim_id)
    }
}

/// How one identity-bearing entity relates to the transaction being verified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeClassification {
    /// Stamped with this transaction's ownership by the executor.
    TransactionAuthoritative,
    /// Owned by a different construction transaction, and therefore none of
    /// this one's business. Another room's persistent contents live here.
    ForeignScope(TransactionId),
    /// Explicitly declared non-authoritative by [`PresentationOnly`].
    PresentationOnly,
    /// Explicitly marked [`LegacyConstructionRoot`] with a family name that is
    /// in [`KNOWN_LEGACY_FAMILIES`]. Reported, published anyway, temporary.
    KnownLegacy { family: String },
    /// Marked [`LegacyConstructionRoot`] with a family nobody enumerated. Fatal:
    /// an unrecognised claim of legacy status is not evidence of one, and
    /// accepting it would turn the marker into a universal opt-out.
    UnknownLegacyFamily { family: String },
    /// Carries an identity, no ownership stamp, and no explicit classification
    /// at all. **Fatal.** This used to be the tolerated case, which meant a
    /// recipe that invented an authoritative entity was indistinguishable from a
    /// known un-migrated family — so the check could not fail for the one reason
    /// it was built to fail for.
    Unowned,
}

/// One identity-bearing entity in the world, and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeMember {
    pub sim_id: SimId,
    pub entity: Entity,
    pub classification: ScopeClassification,
}

/// Every identity-bearing entity in the world, classified against one
/// transaction.
///
/// **Gathered by querying, never curated.** The whole point of reading the
/// world is that the roots most worth catching are the ones a caller would not
/// have thought to list — a recipe that invents an authoritative entity does
/// not also add itself to the caller's array.
#[derive(Clone, Debug)]
pub struct AuthoritativeScope {
    transaction: TransactionId,
    members: Vec<ScopeMember>,
}

impl AuthoritativeScope {
    /// Query the world for every entity carrying a [`SimId`] and classify each
    /// against `transaction`.
    ///
    /// Classification is by component, never by identity spelling: an entity is
    /// [`ScopeClassification::PresentationOnly`] because it says so, and
    /// authoritative because the executor stamped it, not because its `SimId`
    /// starts with one prefix or another.
    pub fn gather(world: &mut World, transaction: &TransactionId) -> Self {
        let mut members = Vec::new();
        let mut query = world.query::<(
            Entity,
            &SimId,
            Option<&TransactionId>,
            Option<&PresentationOnly>,
            Option<&LegacyConstructionRoot>,
        )>();
        for (entity, sim_id, owner, presentation, legacy) in query.iter(world) {
            // Order matters: an explicit ownership stamp outranks a legacy claim,
            // so a family that has migrated cannot keep an exemption it no longer
            // needs by leaving a stale marker behind.
            let classification = if presentation.is_some() {
                ScopeClassification::PresentationOnly
            } else {
                match (owner, legacy) {
                    (Some(owner), _) if owner == transaction => {
                        ScopeClassification::TransactionAuthoritative
                    }
                    (Some(other), _) => ScopeClassification::ForeignScope(other.clone()),
                    (None, Some(legacy)) if legacy.is_known() => ScopeClassification::KnownLegacy {
                        family: legacy.family.clone(),
                    },
                    (None, Some(legacy)) => ScopeClassification::UnknownLegacyFamily {
                        family: legacy.family.clone(),
                    },
                    (None, None) => ScopeClassification::Unowned,
                }
            };
            members.push(ScopeMember {
                sim_id: sim_id.clone(),
                entity,
                classification,
            });
        }
        // Query iteration order is not stable across runs; violations derived
        // from this must be, so sort by the pair that is.
        members.sort_by(|a, b| (&a.sim_id, a.entity).cmp(&(&b.sim_id, b.entity)));
        Self {
            transaction: transaction.clone(),
            members,
        }
    }

    /// Build a scope from explicit members, for fixtures.
    pub fn from_members(transaction: TransactionId, members: Vec<ScopeMember>) -> Self {
        let mut members = members;
        members.sort_by(|a, b| (&a.sim_id, a.entity).cmp(&(&b.sim_id, b.entity)));
        Self {
            transaction,
            members,
        }
    }

    pub fn transaction(&self) -> &TransactionId {
        &self.transaction
    }

    pub fn members(&self) -> &[ScopeMember] {
        &self.members
    }
}

/// Bumped when the plan dump's shape changes. The dump is an inspection and
/// comparison surface, so its shape is a compatibility contract.
pub const CONSTRUCTION_PLAN_SCHEMA_VERSION: u32 = 3;
