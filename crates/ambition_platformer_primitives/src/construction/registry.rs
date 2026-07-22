//! The construction registry: stable recipe identities bound to the functions
//! that plan against them and execute them.
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
use bevy::prelude::Entity;

use super::{ConstructionDomain, ConstructionExecCtx, PlannedEntity, RecipeId};

/// Builds one planned entity and returns the entity it created.
///
/// **Infallible, deliberately.** A recipe consumes decisions preparation already
/// made, so there is nothing left for it to fail at: every lookup that could
/// miss belongs in the request builder, where failing is free and the live world
/// is still whole. Making that a type rather than a convention means a recipe
/// author cannot quietly move a content error inside the mutation.
///
/// A plain `fn` pointer, not a boxed closure: a recipe that captured state could
/// observe something at registration time that is no longer true at execution
/// time, which is the same class of bug. It also makes idempotent
/// re-registration decidable by address.
pub type RecipeFn<D> =
    for<'w, 's, 'a> fn(&PlannedEntity<D>, &mut ConstructionExecCtx<'w, 's, 'a, D>) -> Entity;

/// Wires one declared relation once both ends exist.
pub type RelationFn<D> =
    for<'w, 's, 'a> fn(Entity, Entity, &mut ConstructionExecCtx<'w, 's, 'a, D>);

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
        candidate_owner: String,
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
                candidate_owner,
            } => write!(
                f,
                "conflicting construction relation '{kind}': existing {existing_owner}, candidate \
                 {candidate_owner}"
            ),
        }
    }
}

impl std::error::Error for ConstructionRegistrationError {}

struct RecipeEntry<D: ConstructionDomain> {
    owner: String,
    source: String,
    schema_id: String,
    construct: RecipeFn<D>,
}

struct RelationEntry<D: ConstructionDomain> {
    owner: String,
    wire: RelationFn<D>,
}

/// App-installed registry of construction recipes and relation wirings.
///
/// Ordered storage (`BTreeMap`), so the dump does not depend on insertion order.
#[derive(Resource)]
pub struct ConstructionRegistry<D: ConstructionDomain> {
    recipes: BTreeMap<RecipeId, RecipeEntry<D>>,
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
    /// Register a construction recipe. Re-registering byte-identical ownership
    /// with the same function is idempotent; anything else conflicts.
    pub fn try_register_recipe(
        &mut self,
        recipe: RecipeId,
        owner: impl Into<String>,
        source: impl Into<String>,
        schema_id: impl Into<String>,
        construct: RecipeFn<D>,
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
                && existing.schema_id == schema_id
                && std::ptr::fn_addr_eq(existing.construct, construct);
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
                construct,
            },
        );
        Ok(())
    }

    /// Register the wiring for one relation kind.
    pub fn try_register_relation(
        &mut self,
        kind: RelationKind,
        owner: impl Into<String>,
        wire: RelationFn<D>,
    ) -> Result<(), ConstructionRegistrationError> {
        let owner = owner.into();
        non_empty(&[("id", kind.as_str()), ("owner", owner.as_str())])?;
        if let Some(existing) = self.relations.get(&kind) {
            let identical = existing.owner == owner && std::ptr::fn_addr_eq(existing.wire, wire);
            return if identical {
                Ok(())
            } else {
                Err(ConstructionRegistrationError::ConflictingRelation {
                    kind,
                    existing_owner: existing.owner.clone(),
                    candidate_owner: owner,
                })
            };
        }
        self.relations.insert(kind, RelationEntry { owner, wire });
        Ok(())
    }

    pub(super) fn recipe(&self, recipe: &RecipeId) -> Option<RecipeFn<D>> {
        self.recipes.get(recipe).map(|entry| entry.construct)
    }

    pub(super) fn relation(&self, kind: &RelationKind) -> Option<RelationFn<D>> {
        self.relations.get(kind).map(|entry| entry.wire)
    }

    /// Stable owner/source/schema rows for prepared-content assembly.
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
            out.push_str(&format!("relation\t{kind}\t{}\n", entry.owner));
        }
        out
    }
}
