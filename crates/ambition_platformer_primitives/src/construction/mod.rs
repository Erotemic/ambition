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
//! buildable: [`ConstructionDomain::recipe_of`], which derives a row's recipe
//! from what it carries, and [`ConstructionDomain::construct`], one exhaustive
//! match that populates a root the executor allocated. Neither the caller nor
//! the recipe chooses the pairing or the entity, so neither can get it wrong.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::{Commands, Component, Entity};

use crate::sim_id::SimId;

mod registry;
#[cfg(test)]
mod tests;

pub use registry::{ConstructionRegistrationError, ConstructionRegistry, RelationFn, RelationKind};

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
    /// Frozen services recipes read at execution time. Whatever a domain puts
    /// here is captured before the plan commits, so execution has no fallible
    /// lookup left.
    type Services;

    /// Which recipe builds this row — **a pure function of what the row
    /// carries**, never an independent choice a caller makes.
    ///
    /// This is why [`ConstructionRequest`] has no `recipe` field. When the
    /// recipe was supplied separately, a perfectly valid public request could
    /// name one recipe and carry another's parameters; that pairing passed
    /// preparation and surfaced inside the constructor, mid-commit, as a panic.
    /// Deriving it removes the second value that could disagree.
    fn recipe_of(parameters: &Self::Parameters) -> RecipeId;

    /// Populate the root the executor allocated for this row.
    ///
    /// **Exhaustive over `Parameters`, and that is the point.** A domain writes
    /// one match with an arm per variant, so "this recipe cannot build from
    /// these parameters" is a compile error (a non-exhaustive match) rather
    /// than a runtime `unreachable!` reached after earlier rows have already
    /// mutated the world. Nothing here can fail: every lookup that could miss
    /// resolved in the request builder.
    ///
    /// The root already exists and already carries its `SimId` and
    /// `SpawnOrigin`. A recipe inserts onto it; it cannot choose it, return a
    /// different one, or hand back something that was already alive.
    fn construct(
        parameters: &Self::Parameters,
        root: ConstructionRoot,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, Self>,
    );

    /// Byte-stable one-line rendering of a row's parameters for the plan dump.
    /// Must not include tabs or newlines.
    fn canonical_summary(parameters: &Self::Parameters) -> String;
}

/// The authoritative entity the executor allocated for one planned row.
///
/// A recipe receives this instead of creating its own body, which is what makes
/// "one planned row, one authoritative root" a property of the machinery rather
/// than of every recipe author's care. The inner `Entity` is reachable — a
/// recipe legitimately needs it to insert components and to parent deliberate
/// child entities — but only [`ConstructionPlan`] can mint one, so a recipe
/// cannot nominate a pre-existing entity as a row's root.
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

/// One requested entity, before validation.
///
/// **There is no `recipe` field either.** Which recipe builds a row is derived
/// from its parameters by [`ConstructionDomain::recipe_of`], so a request that
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
    pub relations: Vec<RelationRequest>,
}

/// A declared relation from the requesting entity onto another identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationRequest {
    pub kind: RelationKind,
    pub to: SimId,
}

/// One validated entity with its interpreter already resolved.
///
/// The recipe pointer is stored beside the row for the same reason
/// `PlannedPlacement` stores its lowering function: commit does not repeat
/// registry lookup, and cannot discover a missing recipe after the outgoing
/// world has begun to retire.
pub struct PlannedEntity<D: ConstructionDomain> {
    sim_id: SimId,
    /// Derived once at preparation via [`ConstructionDomain::recipe_of`] and
    /// kept for the dump. Not a dispatch key: construction goes through the
    /// domain's exhaustive match.
    recipe: RecipeId,
    origin: SpawnOrigin,
    parameters: D::Parameters,
}

impl<D: ConstructionDomain> Clone for PlannedEntity<D> {
    fn clone(&self) -> Self {
        Self {
            sim_id: self.sim_id.clone(),
            recipe: self.recipe.clone(),
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

/// One validated relation with its wiring function already resolved.
pub struct PlannedRelation<D: ConstructionDomain> {
    from: SimId,
    kind: RelationKind,
    to: SimId,
    wire: RelationFn<D>,
}

impl<D: ConstructionDomain> Clone for PlannedRelation<D> {
    fn clone(&self) -> Self {
        Self {
            from: self.from.clone(),
            kind: self.kind.clone(),
            to: self.to.clone(),
            wire: self.wire,
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
    /// A relation names a kind the registry does not know how to wire.
    UnknownRelationKind { from: SimId, kind: RelationKind },
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
                "`{from}` declares relation `{kind}`, which no registered wiring handles"
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
        for request in requests {
            // Derived, not supplied — so it always matches the payload.
            let recipe = D::recipe_of(&request.parameters);
            if !registry.has_recipe(&recipe) {
                return Err(ConstructionError::UnknownRecipe {
                    sim_id: request.sim_id,
                    recipe,
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
                let Some(wire) = registry.relation(&relation.kind) else {
                    return Err(ConstructionError::UnknownRelationKind {
                        from: request.sim_id.clone(),
                        kind: relation.kind.clone(),
                    });
                };
                if !planned_ids.contains(&relation.to) {
                    return Err(ConstructionError::UnresolvedRelation {
                        from: request.sim_id.clone(),
                        kind: relation.kind.clone(),
                        to: relation.to.clone(),
                    });
                }
                relations.push(PlannedRelation {
                    from: request.sim_id.clone(),
                    kind: relation.kind.clone(),
                    to: relation.to.clone(),
                    wire,
                });
            }
            entities.push(PlannedEntity {
                sim_id: request.sim_id,
                recipe,
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

    /// The exact roster this plan will commit. Compared against
    /// [`ConstructionReceipt::committed_ids`] to prove parity.
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
    /// Reconstruction of a single entity. Refuses — before mutating — if that
    /// entity declares a relation, because the far end is not being rebuilt
    /// alongside it; see [`ConstructionError::RelationOutsideSubset`].
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
    /// leaves the world exactly as it found it. A relation whose `from` is being
    /// rebuilt but whose `to` is not is such a refusal: quietly rebuilding the
    /// body without the wiring is the silent drop this module exists to delete.
    /// A relation pointing *into* the subset from a row outside it is not — that
    /// relation belongs to the outside row, which is not being rebuilt and still
    /// holds it.
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
            (relation.wire)(from, to, ctx);
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
        // Identity and provenance go on before the recipe runs, so a recipe
        // cannot forget them and reconstruction never sees a body without
        // provenance. A recipe that inspects its own root finds them already
        // there.
        ctx.commands
            .entity(root)
            .insert((planned.sim_id.clone(), planned.origin.clone()));
        D::construct(&planned.parameters, ConstructionRoot(root), ctx);
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
                "relation\t{}\t{}\t{}",
                relation.from, relation.kind, relation.to
            );
        }
        out
    }
}

/// Bumped when the plan dump's shape changes. The dump is an inspection and
/// comparison surface, so its shape is a compatibility contract.
pub const CONSTRUCTION_PLAN_SCHEMA_VERSION: u32 = 2;
