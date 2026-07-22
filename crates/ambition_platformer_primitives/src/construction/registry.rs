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

use std::collections::BTreeMap;

use bevy::ecs::resource::Resource;
use bevy::prelude::{Entity, World};

use super::{ConstructionDomain, ConstructionExecCtx, RecipeId};

/// Wires one declared relation once both ends exist.
///
/// **A bidirectional relation wires BOTH sides here.** `Limb`/`LimbRig` and
/// `RidingOn`/`MountSlot` are each two components that must agree, and the way
/// they have historically disagreed is one site writing one side and forgetting
/// the other — `resolve_pending_mount_links` inserts `MountSlot` while the
/// post-rollback reconcile only `get_mut`s it, so a mount whose slot did not
/// survive ends up pointing nowhere while the rider still points at it. One
/// function writing both ends makes that particular half-write unspellable.
pub type RelationFn<D> = for<'w, 's, 'a> fn(
    Entity,
    Entity,
    &<D as ConstructionDomain>::RelationPayload,
    &mut ConstructionExecCtx<'w, 's, 'a, D>,
);

/// Proves, against the committed world, that a wired relation actually landed.
///
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
    fn(&World, Entity, Entity, &<D as ConstructionDomain>::RelationPayload) -> RelationCheck;

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
}

/// The two frozen halves of one relation kind: how to install it, and how to
/// prove it landed.
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
/// **There is no function here.** Construction dispatches through
/// [`ConstructionDomain::dispatch`], one exhaustive match yielding both a row's
/// recipe identity and its constructor, so a recipe cannot be paired with
/// parameters it cannot build from — that pairing is not representable rather
/// than checked. Preparation freezes the resolved constructor onto the row, so
/// commit never re-asks. A recipe
/// identity earns a registry entry for the ADR-0026 reasons only: stable
/// ownership, idempotent re-registration, conflict rejection, and an ordered
/// contribution to the prepared-content fingerprint.
///
/// This used to hold a `RecipeFn` plus an `AcceptsFn`. That stored the same
/// variant-compatibility fact twice and then called the result proved, which it
/// was not: the two could disagree, and an acceptance function that wrongly
/// returned `true` still reached the constructor's `unreachable!` mid-commit.
struct RecipeEntry {
    owner: String,
    source: String,
    schema_id: String,
}

/// What a registered relation declares about itself.
///
/// Carries the same canonical metadata a recipe does, and for the same reason:
/// a relation's WIRING BEHAVIOUR can change while its kind and owner stay put,
/// and without a schema id that change is invisible to the prepared-content
/// fingerprint. The function pointers are never canonicalised — they are runtime
/// execution state, not content identity.
struct RelationEntry<D: ConstructionDomain> {
    owner: String,
    source: String,
    schema_id: String,
    ops: RelationOps<D>,
}

/// App-installed registry of construction recipe identities and relation
/// wirings.
///
/// Ordered storage (`BTreeMap`), so the dump does not depend on insertion order
/// — which matters because that dump is hashed into the prepared-content
/// fingerprint, and a fingerprint sensitive to plugin insertion order would be
/// unusable.
///
/// ⚠ Recipes here are METADATA ONLY. Whether a domain is extensible by an
/// outside provider is the domain's business: the actor domain is closed, so
/// registering a recipe id there does not make it executable.
#[derive(Resource)]
pub struct ConstructionRegistry<D: ConstructionDomain> {
    recipes: BTreeMap<RecipeId, RecipeEntry>,
    relations: BTreeMap<RelationKind, RelationEntry<D>>,
}

impl<D: ConstructionDomain> Default for ConstructionRegistry<D> {
    fn default() -> Self {
        Self {
            recipes: BTreeMap::new(),
            relations: BTreeMap::new(),
        }
    }
}

fn non_empty(fields: &[(&'static str, &str)]) -> Result<(), ConstructionRegistrationError> {
    for (field, value) in fields {
        if value.trim().is_empty() {
            return Err(ConstructionRegistrationError::EmptyIdentity { field });
        }
    }
    Ok(())
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
        if let Some(existing) = self.recipes.get(&recipe) {
            let identical = existing.owner == owner
                && existing.source == source
                && existing.schema_id == schema_id;
            return if identical {
                Ok(())
            } else {
                Err(ConstructionRegistrationError::ConflictingRecipe {
                    recipe,
                    existing_owner: existing.owner.clone(),
                    existing_source: existing.source.clone(),
                    existing_schema: existing.schema_id.clone(),
                    candidate_owner: owner,
                    candidate_source: source,
                    candidate_schema: schema_id,
                })
            };
        }
        self.recipes.insert(
            recipe,
            RecipeEntry {
                owner,
                source,
                schema_id,
            },
        );
        Ok(())
    }

    /// Register the wiring and the postcondition check for one relation kind.
    ///
    /// **Registration identity is metadata, and only metadata.** This compared
    /// `std::ptr::fn_addr_eq(existing.wire, wire)` as part of the idempotence
    /// test, which is not a property a registry contract can rest on: the
    /// compiler is free to merge two identical functions to one address and to
    /// emit one function at several addresses across codegen units, so the same
    /// registration could conflict or not depending on optimisation level. A
    /// function address is an execution detail, not a semantic identity.
    ///
    /// The rule instead: same kind, owner, source, and schema id is idempotent,
    /// whatever the pointers; any metadata disagreement is a conflict. What the
    /// functions DO is governed by the schema contract, so changing their
    /// behaviour means bumping `schema_id` — which is also exactly what makes
    /// the change visible to the prepared-content fingerprint. A silent
    /// behaviour swap under a fixed schema id was never detectable by pointer
    /// comparison anyway: editing a function's body does not move it.
    pub fn try_register_relation(
        &mut self,
        kind: RelationKind,
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
        ops: RelationOps<D>,
    ) -> Result<(), ConstructionRegistrationError> {
        let (owner, source, schema_id) = (owner.into(), source.into(), schema_id.into());
        non_empty(&[
            ("id", kind.as_str()),
            ("owner", owner.as_str()),
            ("source", source.as_str()),
            ("schema id", schema_id.as_str()),
        ])?;
        if let Some(existing) = self.relations.get(&kind) {
            let identical = existing.owner == owner
                && existing.source == source
                && existing.schema_id == schema_id;
            return if identical {
                Ok(())
            } else {
                Err(ConstructionRegistrationError::ConflictingRelation {
                    kind,
                    existing_owner: existing.owner.clone(),
                    existing_source: existing.source.clone(),
                    existing_schema: existing.schema_id.clone(),
                    candidate_owner: owner,
                    candidate_source: source,
                    candidate_schema: schema_id,
                })
            };
        }
        self.relations.insert(
            kind,
            RelationEntry {
                owner,
                source,
                schema_id,
                ops,
            },
        );
        Ok(())
    }

    /// Whether this recipe identity is registered. Preparation refuses a row
    /// whose derived recipe nothing declared, which is what keeps the registry
    /// meaningful now that it no longer dispatches.
    pub(super) fn has_recipe(&self, recipe: &RecipeId) -> bool {
        self.recipes.contains_key(recipe)
    }

    pub(super) fn relation(&self, kind: &RelationKind) -> Option<RelationOps<D>> {
        self.relations.get(kind).map(|entry| entry.ops)
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

    pub fn deterministic_dump(&self) -> String {
        let mut out: String = self
            .schema_descriptors()
            .into_iter()
            .map(|(recipe, owner, source, schema)| {
                format!("recipe\t{recipe}\t{owner}\t{source}\t{schema}\n")
            })
            .collect();
        for (kind, entry) in &self.relations {
            out.push_str(&format!(
                "relation\t{kind}\t{}\t{}\t{}\n",
                entry.owner, entry.source, entry.schema_id
            ));
        }
        out
    }
}
