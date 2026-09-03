//! The construction registry: stable recipe identities, and the wiring
//! functions for the relation kinds between constructed entities.
//!
//! Follows the registration lifecycle every other prepared registry in the tree
//! uses (`PlacementLoweringRegistry`, `RoomContentStagingRegistry`): registration
//! happens during App/plugin build, identity fields are validated, byte-identical
//! re-registration is idempotent, a conflicting registration is rejected
//! transactionally rather than overwriting, and storage is ordered so equivalent
//! plugin insertion orders produce the same dump and the same fingerprint
//! contribution.
//!
//! ⭐ Since 2026-09-03 that lifecycle is spelled by `ambition_registry_core`
//! rather than restated here: the entry IS a [`RegistrationMeta`], a second
//! registration is [`classify`]d, and the dump is [`canonical_row`]s. This
//! registry was the one of thirty-one that answered all four protocol
//! questions on purpose, so it is the first to read them from the shared
//! vocabulary — the point being that the next registry cannot answer them
//! differently by accident.

use std::collections::BTreeMap;

use ambition_registry_core::{
    canonical_row, canonical_section, classify, Classification, RegistrationMeta,
};

use bevy::ecs::resource::Resource;
use bevy::prelude::{Entity, World};

use super::{ConstructionDomain, ConstructionExecCtx, RecipeId};

/// Wires one declared relation once both ends exist.
///
/// A bidirectional relation wires BOTH sides here. `Limb`/`LimbRig` and
/// `RidingOn`/`MountSlot` are each two components that must agree, and the way
/// they have historically disagreed is one site writing one side and forgetting
/// the other — the old frame-later mount resolver inserted `MountSlot` while the
/// post-rollback reconcile only `get_mut`s it, so a mount whose slot did not
/// survive ends up pointing nowhere while the rider still points at it. One
/// function writing both ends makes that particular half-write unspellable.
pub type RelationFn<D> = for<'w, 's, 'a> fn(
    Entity,
    Entity,
    &<D as ConstructionDomain>::Relation,
    &mut ConstructionExecCtx<'w, 's, 'a, D>,
);

/// The counterpart to [`RelationFn`], and deliberately its twin: a relation is
/// two facts — how to install it and what installed looks like — and splitting
/// them across unrelated functions is how the earlier duplicated-fact bugs in
/// this module started. They travel together in one [`RelationOps`], are
/// registered together, and are frozen together onto a planned row.
///
/// Reads components, never debug strings. "The wiring function ran" is what a
/// receipt already records; this answers the different question of whether the
/// world now holds the relation the plan described.
pub type RelationVerifyFn<D> =
    fn(&World, Entity, Entity, &<D as ConstructionDomain>::Relation) -> RelationCheck;

/// What inspecting a wired relation in the committed world found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationCheck {
    /// The source holds this relation, onto exactly the planned target.
    Installed,
    /// The source holds no relation of this kind. A no-op wiring function, a
    /// relation removed after wiring, and a relation installed on some other
    /// entity all land here — from the planned source's point of view they are
    /// the same absence.
    NotInstalled,
    /// The source holds the relation, but onto something else — another entity,
    /// or the pre-reconstruction generation of the right one. `found` is what it
    /// points at, which is what distinguishes "overwritten" from "stale".
    WrongTarget { found: Option<Entity> },
    /// A bidirectional relation whose forward side is right and whose reverse
    /// side disagrees. Checked separately because a half-wired pair passes every
    /// forward-only test while leaving one side of the world lying.
    ReverseMismatch { found: Option<Entity> },
    /// Both ends name each other, but a value the PAIRING carries did not land:
    /// a limb wired into the wrong slot, a home offset overwritten after wiring.
    ///
    /// `field` labels which one for the diagnostic. It is not how the check was
    /// performed — the verifier read the component and compared it to the planned
    /// value — so this is a structured finding with a human label, not
    /// verification by string.
    PayloadMismatch { field: &'static str },
    /// The relation's own components agree, but an entity is missing a component
    /// the relation's semantics require of it: a rider without `Mounted`, a
    /// mount without `Mountable`, a would-be pilot without `CanPilot`. A pair
    /// that names each other and cannot function is still a broken relation.
    MissingCapability { component: &'static str },
    /// The reverse side names the source MORE THAN ONCE — a limb appended to its
    /// host's rig twice. Every forward check and every "is it in there" check
    /// passes; the host simply drives the limb twice per frame.
    DuplicateMembership { count: usize },
}

pub struct RelationOps<D: ConstructionDomain> {
    pub wire: RelationFn<D>,
    pub verify: RelationVerifyFn<D>,
}

impl<D: ConstructionDomain> Clone for RelationOps<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: ConstructionDomain> Copy for RelationOps<D> {}

/// A stable identity for a kind of relation between two constructed entities.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationKind(String);

impl RelationKind {
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RelationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructionRegistrationError {
    EmptyIdentity {
        field: &'static str,
    },
    ConflictingRecipe {
        recipe: RecipeId,
        existing_owner: String,
        existing_source: String,
        existing_schema: String,
        candidate_owner: String,
        candidate_source: String,
        candidate_schema: String,
    },
    ConflictingRelation {
        kind: RelationKind,
        existing_owner: String,
        existing_source: String,
        existing_schema: String,
        candidate_owner: String,
        candidate_source: String,
        candidate_schema: String,
    },
}

impl std::fmt::Display for ConstructionRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentity { field } => {
                write!(f, "construction recipe {field} must not be empty")
            }
            Self::ConflictingRecipe {
                recipe,
                existing_owner,
                existing_source,
                existing_schema,
                candidate_owner,
                candidate_source,
                candidate_schema,
            } => write!(
                f,
                "conflicting construction recipe for '{recipe}': existing \
                 {existing_owner}/{existing_source} schema '{existing_schema}', candidate \
                 {candidate_owner}/{candidate_source} schema '{candidate_schema}'"
            ),
            Self::ConflictingRelation {
                kind,
                existing_owner,
                existing_source,
                existing_schema,
                candidate_owner,
                candidate_source,
                candidate_schema,
            } => write!(
                f,
                "conflicting construction relation '{kind}': existing \
                 {existing_owner}/{existing_source} schema '{existing_schema}', candidate \
                 {candidate_owner}/{candidate_source} schema '{candidate_schema}'"
            ),
        }
    }
}

impl std::error::Error for ConstructionRegistrationError {}

/// What a registered recipe declares about itself.
///
/// There is no function here. Construction dispatches through
/// [`ConstructionDomain::dispatch`], one exhaustive match yielding both a row's
/// recipe identity and its constructor, so a recipe cannot be paired with
/// parameters it cannot build from — that pairing is not representable rather
/// than checked. Preparation freezes the resolved constructor onto the row, so
/// commit never re-asks. A recipe
/// identity earns a registry entry for the ADR-0026 reasons only: stable
/// ownership, idempotent re-registration, conflict rejection, and an ordered
/// contribution to the prepared-content fingerprint.
///
/// That stored the same variant-compatibility fact twice and then called the result proved, which
/// it was not: the two could disagree, and an acceptance function that wrongly returned `true`
/// still reached the constructor's `unreachable!` mid-commit.
///
/// A recipe entry is exactly a [`RegistrationMeta`]: owner, source, schema id.
type RecipeEntry = RegistrationMeta;

/// What a registered relation declares about itself.
///
/// Executable behaviour now comes from [`ConstructionDomain::dispatch_relation`]
/// — one exhaustive match in the domain that owns the relation enum — so there
/// is no table for an outside registration to win a race in. This entry does
/// what a recipe entry does: stable ownership, idempotent re-registration,
/// conflict rejection, and an ordered fingerprint contribution.
type RelationEntry = RegistrationMeta;

/// App-installed registry of construction recipe identities and relation
/// wirings.
///
/// Ordered storage (`BTreeMap`), so the dump does not depend on insertion order
/// — which matters because that dump is hashed into the prepared-content
/// fingerprint, and a fingerprint sensitive to plugin insertion order would be
/// unusable.
///
/// Recipes here are METADATA ONLY. Whether a domain is extensible by an
/// outside provider is the domain's business: the actor domain is closed, so
/// registering a recipe id there does not make it executable.
#[derive(Resource)]
pub struct ConstructionRegistry<D: ConstructionDomain> {
    recipes: BTreeMap<RecipeId, RecipeEntry>,
    relations: BTreeMap<RelationKind, RelationEntry>,
    domain: std::marker::PhantomData<fn() -> D>,
}

impl<D: ConstructionDomain> Default for ConstructionRegistry<D> {
    fn default() -> Self {
        Self {
            recipes: BTreeMap::new(),
            relations: BTreeMap::new(),
            domain: std::marker::PhantomData,
        }
    }
}

fn non_empty(fields: &[(&'static str, &str)]) -> Result<(), ConstructionRegistrationError> {
    ambition_registry_core::require_non_empty(fields)
        .map_err(|empty| ConstructionRegistrationError::EmptyIdentity { field: empty.field })
}

impl<D: ConstructionDomain> ConstructionRegistry<D> {
    /// Register a construction recipe identity. Re-registering byte-identical
    /// ownership is idempotent; anything else conflicts.
    pub fn try_register_recipe(
        &mut self,
        recipe: RecipeId,
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
    ) -> Result<(), ConstructionRegistrationError> {
        let (owner, source, schema_id) = (owner.into(), source.into(), schema_id.into());
        non_empty(&[
            ("id", recipe.as_str()),
            ("owner", owner.as_str()),
            ("source", source.as_str()),
            ("schema id", schema_id.as_str()),
        ])?;
        let incoming = RecipeEntry {
            owner,
            source,
            schema_id,
        };
        match classify(self.recipes.get(&recipe), &incoming) {
            Classification::Idempotent => Ok(()),
            Classification::Conflict { existing } => {
                Err(ConstructionRegistrationError::ConflictingRecipe {
                    recipe,
                    existing_owner: existing.owner.clone(),
                    existing_source: existing.source.clone(),
                    existing_schema: existing.schema_id.clone(),
                    candidate_owner: incoming.owner,
                    candidate_source: incoming.source,
                    candidate_schema: incoming.schema_id,
                })
            }
            Classification::New => {
                self.recipes.insert(recipe, incoming);
                Ok(())
            }
        }
    }

    /// Register a relation kind's IDENTITY. Re-registering byte-identical
    /// ownership is idempotent; anything else conflicts.
    ///
    /// Neither is needed now. Executable behaviour is resolved by
    /// [`ConstructionDomain::dispatch_relation`], one exhaustive match owned by
    /// the domain that defines the relation enum, so a relation's wiring is not
    /// something a registration can supply, replace, or race for. What an
    /// outside provider contributes here is what it can honestly contribute:
    /// identity, ownership, a schema version, and therefore a prepared-content
    /// fingerprint contribution.
    pub fn try_register_relation(
        &mut self,
        kind: RelationKind,
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
    ) -> Result<(), ConstructionRegistrationError> {
        let (owner, source, schema_id) = (owner.into(), source.into(), schema_id.into());
        non_empty(&[
            ("id", kind.as_str()),
            ("owner", owner.as_str()),
            ("source", source.as_str()),
            ("schema id", schema_id.as_str()),
        ])?;
        let incoming = RelationEntry {
            owner,
            source,
            schema_id,
        };
        match classify(self.relations.get(&kind), &incoming) {
            Classification::Idempotent => Ok(()),
            Classification::Conflict { existing } => {
                Err(ConstructionRegistrationError::ConflictingRelation {
                    kind,
                    existing_owner: existing.owner.clone(),
                    existing_source: existing.source.clone(),
                    existing_schema: existing.schema_id.clone(),
                    candidate_owner: incoming.owner,
                    candidate_source: incoming.source,
                    candidate_schema: incoming.schema_id,
                })
            }
            Classification::New => {
                self.relations.insert(kind, incoming);
                Ok(())
            }
        }
    }

    /// Whether this recipe identity is registered. Preparation refuses a row
    /// whose derived recipe nothing declared, which is what keeps the registry
    /// meaningful now that it no longer dispatches.
    pub(super) fn has_recipe(&self, recipe: &RecipeId) -> bool {
        self.recipes.contains_key(recipe)
    }

    /// Whether this relation kind is registered. Preparation refuses a relation
    /// whose kind nothing declared — the same rule recipes get, and the reason
    /// the table still matters now that it does not dispatch.
    pub(super) fn has_relation(&self, kind: &RelationKind) -> bool {
        self.relations.contains_key(kind)
    }

    /// Stable owner/source/schema rows for prepared-content assembly, for
    /// recipes. Relations contribute through [`Self::deterministic_dump`].
    pub fn schema_descriptors(&self) -> Vec<(String, String, String, String)> {
        self.recipes
            .iter()
            .map(|(recipe, entry)| {
                (
                    recipe.as_str().to_owned(),
                    entry.owner.clone(),
                    entry.source.clone(),
                    entry.schema_id.clone(),
                )
            })
            .collect()
    }

    /// Recipes then relations, each ordered by key: the section grammar the
    /// prepared-content fingerprint hashes, so the two row kinds' relative
    /// order is part of it and is not sorted away.
    pub fn deterministic_dump(&self) -> String {
        let rows: Vec<String> = self
            .recipes
            .iter()
            .map(|(recipe, entry)| {
                canonical_row(&[
                    "recipe",
                    recipe.as_str(),
                    &entry.owner,
                    &entry.source,
                    &entry.schema_id,
                ])
            })
            .chain(self.relations.iter().map(|(kind, entry)| {
                canonical_row(&[
                    "relation",
                    kind.as_str(),
                    &entry.owner,
                    &entry.source,
                    &entry.schema_id,
                ])
            }))
            .collect();
        canonical_section(None, rows.iter().map(String::as_str))
    }
}
