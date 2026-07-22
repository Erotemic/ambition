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
//! interpreter table). Recipes are plain `fn` pointers, so they capture nothing,
//! compare by address for idempotent re-registration, and cannot smuggle state
//! between planning and execution.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::{Commands, Component, Entity};

use crate::sim_id::SimId;

mod registry;
#[cfg(test)]
mod tests;

pub use registry::{
    ConstructionRegistrationError, ConstructionRegistry, RecipeFn, RelationFn, RelationKind,
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
    Dynamic {
        parent: Option<SimId>,
        sequence: u64,
    },
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
            Self::Dynamic { parent, .. } => parent.as_ref(),
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
            Self::Dynamic { parent, sequence } => format!(
                "dynamic\t{}\t{sequence}",
                parent.as_ref().map_or("-", SimId::as_str)
            ),
        }
    }
}

/// The domain a construction plan is written against.
///
/// Core plans, validates, orders, and dumps; the domain supplies the payload and
/// the frozen services its recipes read. Keeping this an associated-type pair
/// rather than type erasure means a recipe never downcasts and a plan cannot be
/// executed against the wrong world.
pub trait ConstructionDomain: Send + Sync + 'static {
    /// What one planned row carries into its recipe.
    type Parameters: Clone + Send + Sync + 'static;
    /// Frozen services recipes read at execution time. Whatever a domain puts
    /// here is captured before the plan commits, so execution has no fallible
    /// lookup left.
    type Services;

    /// Byte-stable one-line rendering of a row's parameters for the plan dump.
    /// Must not include tabs or newlines.
    fn canonical_summary(parameters: &Self::Parameters) -> String;
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
    /// The activation generation this plan was prepared against.
    ///
    /// **Recorded, not yet enforced.** It appears in the dump, so a plan can be
    /// joined to the content that produced it, and a plan carried across an
    /// epoch bump is visibly stale. Turning that into a REFUSAL belongs to the
    /// commit boundary, which Phase 4 owns — nothing here holds both the plan
    /// and the live world at once. `ContentEpoch::default()` reads as "no
    /// generation stated": a fixture, or a plan built and committed inside one
    /// tick, which cannot outlive a reload.
    pub content_epoch: ambition_engine_core::ContentEpoch,
    /// The room being constructed, when the plan is a room's contents.
    pub room: Option<String>,
}

/// One requested entity, before validation.
pub struct ConstructionRequest<D: ConstructionDomain> {
    pub sim_id: SimId,
    pub recipe: RecipeId,
    pub origin: SpawnOrigin,
    pub parent: Option<SimId>,
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
    recipe: RecipeId,
    origin: SpawnOrigin,
    parent: Option<SimId>,
    parameters: D::Parameters,
    construct: RecipeFn<D>,
}

impl<D: ConstructionDomain> Clone for PlannedEntity<D> {
    fn clone(&self) -> Self {
        Self {
            sim_id: self.sim_id.clone(),
            recipe: self.recipe.clone(),
            origin: self.origin.clone(),
            parent: self.parent.clone(),
            parameters: self.parameters.clone(),
            construct: self.construct,
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
    pub fn parent(&self) -> Option<&SimId> {
        self.parent.as_ref()
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
            let Some(construct) = registry.recipe(&request.recipe) else {
                return Err(ConstructionError::UnknownRecipe {
                    sim_id: request.sim_id,
                    recipe: request.recipe,
                });
            };
            if let Some(parent) = &request.parent {
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
                recipe: request.recipe,
                origin: request.origin,
                parent: request.parent,
                parameters: request.parameters,
                construct,
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

    /// Construct one planned entity through its frozen recipe.
    ///
    /// **This is the only constructor.** Ordinary construction calls it for
    /// every row; reconstruction of a single entity calls it for one. There is
    /// no second path that could drift from this one, which is the property the
    /// whole plan exists to buy.
    pub fn construct_one(
        &self,
        sim_id: &SimId,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Result<Entity, ConstructionError> {
        let planned = self
            .get(sim_id)
            .ok_or_else(|| ConstructionError::NotInPlan {
                sim_id: sim_id.clone(),
            })?;
        Ok(Self::commit_entity(planned, ctx))
    }

    /// Construct every planned entity, then wire every planned relation.
    ///
    /// Relations run second because a relation names identities, and an
    /// identity has no entity until its row has been committed. That ordering
    /// is what lets a plan express a mutual pair (two duellists grudging each
    /// other) without either row needing the other to exist first.
    pub fn commit(&self, ctx: &mut ConstructionExecCtx<'_, '_, '_, D>) -> ConstructionReceipt {
        let mut receipt = ConstructionReceipt::default();
        for planned in &self.entities {
            let entity = Self::commit_entity(planned, ctx);
            receipt.committed.insert(planned.sim_id.clone(), entity);
        }
        for relation in &self.relations {
            // Both ends are rows in this plan — preparation rejects anything
            // else — and every row above is now committed, so a miss here is a
            // planner bug rather than a content error. It must not be swallowed.
            let (Some(from), Some(to)) = (
                receipt.committed.get(&relation.from).copied(),
                receipt.committed.get(&relation.to).copied(),
            ) else {
                unreachable!(
                    "planned relation {} -> {} names an identity this plan did not commit",
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
        receipt
    }

    fn commit_entity(
        planned: &PlannedEntity<D>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Entity {
        let entity = (planned.construct)(planned, ctx);
        // Identity and provenance are stamped by the executor, not by each
        // recipe: a recipe that forgot would produce an entity nothing could
        // reconstruct, and the omission would be invisible until a restore.
        ctx.commands
            .entity(entity)
            .insert((planned.sim_id.clone(), planned.origin.clone()));
        entity
    }

    /// Byte-stable inspection surface, in the same tab-delimited shape as
    /// `PreparedContent::deterministic_dump`. Two plans over equivalent input
    /// produce identical bytes regardless of request order.
    pub fn deterministic_dump(&self) -> String {
        use std::fmt::Write as _;
        let mut out = format!(
            "construction-plan-v{CONSTRUCTION_PLAN_SCHEMA_VERSION}\n{}\nroom\t{}\n",
            self.scope.content_epoch,
            self.scope.room.as_deref().unwrap_or("-"),
        );
        for entity in &self.entities {
            let _ = writeln!(
                out,
                "entity\t{}\t{}\t{}\t{}\t{}",
                entity.sim_id,
                entity.recipe,
                entity.parent.as_ref().map_or("-", SimId::as_str),
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
pub const CONSTRUCTION_PLAN_SCHEMA_VERSION: u32 = 1;
