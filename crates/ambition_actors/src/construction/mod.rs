//! **The actor construction domain: three origins, one planner.**
//!
//! `docs/planning/engine/immutable-content-and-transactional-construction.md`
//! Phase 3 asks for one authored placement, one provider-staged actor, and one
//! runtime-dynamic family to share a pure, preflightable planner and a
//! recipe-backed reconstruction path. These are those three:
//!
//! | recipe | origin | family |
//! |---|---|---|
//! | [`RECIPE_AUTHORED_GROUND_ITEM`] | [`SpawnOrigin::Authored`] | an LDtk-authored `GroundItemSpec` |
//! | [`RECIPE_STAGED_ACTOR`] | [`SpawnOrigin::ProviderStaged`] | a `SpawnActorRequest` from `RoomContentStagingRegistry` |
//! | [`RECIPE_SUMMONED_MINION`] | [`SpawnOrigin::Dynamic`] | a minion materialized from `Effect::Summon` |
//!
//! They were chosen because each one is genuinely a different *kind* of origin
//! rather than three flavours of the same one, and because each was losing
//! something real to the absence of a plan:
//!
//! - **The ground item silently vanished.** `spawn_ground_item` resolved its
//!   held-item registry id at spawn time and `return`ed on a miss, so an
//!   authored pickup naming an unregistered or feature-gated item produced no
//!   entity and no diagnostic. Resolution now happens while planning, where a
//!   miss is a [`ActorConstructionError::UnknownHeldItem`] that fails the room
//!   before it is torn down.
//! - **The staged duel's grudge silently dropped.** `wire_staged_grudges`
//!   skipped a `grudge_against` naming an actor outside the batch, so a typo
//!   produced two fighters who ignored each other. It is a
//!   [`RELATION_GRUDGE`] now, validated against the plan's own roster plus the
//!   live world before anything spawns.
//! - **The summoned minion lied about where it came from.** It carries a
//!   `FeatureId`, so `ensure_sim_id` gave it an id in the *authored*
//!   `placement:` namespace — the one namespace it categorically is not in.
//!   It now takes a proper `SimId::spawned` under its summoner and states its
//!   parent in [`SpawnOrigin::Dynamic`] rather than implying it by spelling.

use ambition_platformer_primitives::construction::{
    ConstructionDomain, ConstructionExecCtx, ConstructionPlan, ConstructionRegistrationError,
    ConstructionRegistry, ConstructionRequest, ConstructionRoot, RecipeDispatch, RecipeId,
    RelationCheck, RelationKind, RelationOps, SpawnOrigin,
};
use ambition_platformer_primitives::sim_id::SimId;
use bevy::prelude::{Entity, World};

use crate::boss_encounter::BossCatalog;
use crate::features::{SpawnActorKind, SpawnActorRequest};
use crate::world::placements::ActorPlacementContext;

#[cfg(test)]
mod tests;

/// An LDtk-authored ground item (a walk-into pickup).
pub const RECIPE_AUTHORED_GROUND_ITEM: &str = "ambition.authored-ground-item";
/// An actor a provider staged into a room during construction.
pub const RECIPE_STAGED_ACTOR: &str = "ambition.staged-actor";
/// A minion the running simulation summoned.
pub const RECIPE_SUMMONED_MINION: &str = "ambition.summoned-minion";
/// A personal grudge from one constructed actor onto another.
pub const RELATION_GRUDGE: &str = "ambition.grudge";

const OWNER: &str = "ambition_actors";
const SCHEMA: &str = "actor-construction-v1";

pub fn recipe_authored_ground_item() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_GROUND_ITEM)
}
pub fn recipe_staged_actor() -> RecipeId {
    RecipeId::new(RECIPE_STAGED_ACTOR)
}
pub fn recipe_summoned_minion() -> RecipeId {
    RecipeId::new(RECIPE_SUMMONED_MINION)
}
pub fn relation_grudge() -> RelationKind {
    RelationKind::new(RELATION_GRUDGE)
}

/// What one planned actor-domain row carries into its recipe.
///
/// Every variant holds values that are already fully resolved: the ground
/// item's `HeldItemSpec`, not its registry id; the minion's faction, not the
/// `HitSide` it was authored as. Resolution belongs to planning, so execution
/// has no lookup that can fail.
#[derive(Clone, Debug)]
pub enum ActorConstructionParams {
    GroundItem {
        spec: crate::rooms::GroundItemSpec,
        held: ambition_characters::brain::HeldItemSpec,
    },
    StagedActor(SpawnActorRequest),
    SummonedMinion(SummonedMinionParams),
}

/// A minion resolved from `Effect::Summon`.
#[derive(Clone, Debug)]
pub struct SummonedMinionParams {
    /// Stable feature id, which is what per-entity systems (targeting,
    /// encounter bookkeeping) join on. Distinct from the row's `SimId`, which
    /// is the summoner-relative spawned identity.
    pub feature_id: String,
    pub name: String,
    pub pos: ambition_engine_core::Vec2,
    pub half_size: ambition_engine_core::Vec2,
    pub archetype_id: String,
    pub encounter_id: String,
    pub faction: crate::features::ActorFaction,
}

/// Frozen catalogs the actor recipes read at execution time.
///
/// Built ONCE, when the plan is prepared. Session ownership is deliberately not
/// in here: it varies per commit, and folding it in would mean rebuilding these
/// catalogs — `BossCatalog` alone is seven `BTreeMap`s — once per entity during
/// a reconstruction sweep. It rides on `ConstructionExecCtx::session` instead.
#[derive(Clone)]
pub struct ActorConstructionServices {
    /// Character catalog + roster, the same pair authored placement lowering
    /// captures.
    pub context: ActorPlacementContext,
    pub boss_catalog: BossCatalog,
}

/// The actor construction domain.
pub struct ActorConstruction;

impl ConstructionDomain for ActorConstruction {
    type Parameters = ActorConstructionParams;
    type Services = ActorConstructionServices;

    /// ONE match: each arm names both the recipe identity and the function that
    /// builds it, so the label and the behaviour cannot drift apart. Adding a
    /// variant without an arm is a compile error.
    fn dispatch(parameters: &Self::Parameters) -> RecipeDispatch<Self> {
        match parameters {
            ActorConstructionParams::GroundItem { .. } => RecipeDispatch {
                recipe: recipe_authored_ground_item(),
                construct: construct_authored_ground_item,
            },
            ActorConstructionParams::StagedActor(_) => RecipeDispatch {
                recipe: recipe_staged_actor(),
                construct: construct_staged_actor,
            },
            ActorConstructionParams::SummonedMinion(_) => RecipeDispatch {
                recipe: recipe_summoned_minion(),
                construct: construct_summoned_minion,
            },
        }
    }

    fn canonical_summary(parameters: &Self::Parameters) -> String {
        match parameters {
            ActorConstructionParams::GroundItem { spec, held } => {
                format!("ground-item {} {}", spec.id, held.id)
            }
            ActorConstructionParams::StagedActor(request) => format!(
                "staged-actor {} {} {}",
                request.id,
                request.name,
                match request.kind {
                    SpawnActorKind::Boss { .. } => "boss",
                    SpawnActorKind::Enemy { .. } => "enemy",
                }
            ),
            ActorConstructionParams::SummonedMinion(minion) => {
                format!("minion {} {}", minion.feature_id, minion.archetype_id)
            }
        }
    }
}

pub type ActorConstructionRegistry = ConstructionRegistry<ActorConstruction>;
pub type ActorConstructionPlan = ConstructionPlan<ActorConstruction>;
pub type ActorConstructionRequest = ConstructionRequest<ActorConstruction>;
type Ctx<'w, 's, 'a> = ConstructionExecCtx<'w, 's, 'a, ActorConstruction>;

/// Why an actor-domain request could not be turned into a planned row. These
/// are the failures that used to be silent skips at spawn time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActorConstructionError {
    UnknownHeldItem { authored_id: String, item: String },
}

impl std::fmt::Display for ActorConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownHeldItem { authored_id, item } => write!(
                f,
                "authored ground item `{authored_id}` names held item `{item}`, which no held-item \
                 registry entry provides"
            ),
        }
    }
}

impl std::error::Error for ActorConstructionError {}

// ── Recipes ──────────────────────────────────────────────────────────────────
//
// Each is paired with its identity in `dispatch` above and reached only through
// it, so the `unreachable!` arms are unreachable by the same decision that
// selected the function. `every_parameter_variant_matches_its_descriptor`
// asserts that pairing per variant behaviourally.

fn construct_authored_ground_item(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::GroundItem { spec, held } = parameters else {
        unreachable!("dispatch pairs this fn with GroundItem parameters")
    };
    crate::features::ecs::spawn_static::spawn_ground_item_resolved_into(
        ctx.commands,
        ctx.session,
        root.entity(),
        spec,
        held.clone(),
    );
}

fn construct_staged_actor(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::StagedActor(request) = parameters else {
        unreachable!("dispatch pairs this fn with StagedActor parameters")
    };
    crate::features::spawn_staged_actor_into(
        ctx.commands,
        &ctx.services.context.characters,
        &ctx.services.context.roster,
        &ctx.services.boss_catalog,
        ctx.session,
        root.entity(),
        request,
    );
}

fn construct_summoned_minion(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::SummonedMinion(minion) = parameters else {
        unreachable!("dispatch pairs this fn with SummonedMinion parameters")
    };
    crate::features::spawn_runtime_minion_into(
        ctx.commands,
        &ctx.services.context.characters,
        &ctx.services.context.roster,
        ctx.session,
        root.entity(),
        minion.feature_id.clone(),
        minion.name.clone(),
        minion.pos,
        minion.half_size,
        &minion.archetype_id,
        minion.encounter_id.clone(),
        minion.faction,
        crate::features::ActorAggression::hostile(),
    );
}

// ── Relations ────────────────────────────────────────────────────────────────

/// The grudge's wiring and its postcondition check, exposed so provider-side
/// fingerprint tests can build a registry that matches the real one rather than
/// a lookalike.
pub fn grudge_ops_for_tests() -> RelationOps<ActorConstruction> {
    grudge_ops()
}

/// The two halves of the grudge relation. Registered together and frozen
/// together onto a planned row, so "how it is installed" and "what installed
/// looks like" cannot be edited apart.
fn grudge_ops() -> RelationOps<ActorConstruction> {
    RelationOps {
        wire: wire_grudge,
        verify: verify_grudge,
    }
}

/// Wire a personal grudge. Re-inserting `ActorAggression` is safe: staged
/// fighters spawn `hostile()` already, so this only adds the grudge.
fn wire_grudge(from: Entity, to: Entity, ctx: &mut Ctx<'_, '_, '_>) {
    ctx.commands
        .entity(from)
        .insert(crate::features::ActorAggression {
            grudge: Some(to),
            ..crate::features::ActorAggression::hostile()
        });
}

/// Prove the grudge landed, by reading the component rather than trusting that
/// [`wire_grudge`] was called.
///
/// The distinction matters because the two are separately fallible: the wiring
/// runs through deferred `Commands`, so a later command in the same flush can
/// overwrite `ActorAggression` wholesale, and the receipt records the call
/// either way. A grudge onto a stale pre-reconstruction entity also reads as
/// `WrongTarget` here — `found` names the corpse, which is what makes that case
/// diagnosable rather than merely wrong.
fn verify_grudge(world: &World, from: Entity, to: Entity) -> RelationCheck {
    match world.get::<crate::features::ActorAggression>(from) {
        None => RelationCheck::NotInstalled,
        Some(aggression) => match aggression.grudge {
            None => RelationCheck::NotInstalled,
            Some(found) if found == to => RelationCheck::Installed,
            found => RelationCheck::WrongTarget { found },
        },
    }
}

/// A standalone registry holding the engine's own recipes.
///
/// ⚠ **This domain is CLOSED.** `ActorConstructionParams` is a closed enum and
/// [`ActorConstruction::dispatch`] a closed match, so a provider registering
/// into this table contributes recipe METADATA — identity, ownership, schema
/// version, and therefore a prepared-content fingerprint contribution — and
/// cannot contribute executable construction behaviour. Callers that need a
/// registry of their own (fixtures, tools, a preflight outside a live App) build
/// one here rather than re-listing the recipes and drifting from the real table.
pub fn engine_construction_registry() -> ActorConstructionRegistry {
    let mut registry = ActorConstructionRegistry::default();
    install_actor_construction_recipes(&mut registry)
        .expect("the engine's own construction recipes cannot conflict with each other");
    registry
}

/// Install the engine's actor recipes. Idempotent, so a host that composes the
/// plugin twice is not an error.
pub fn install_actor_construction_recipes(
    registry: &mut ActorConstructionRegistry,
) -> Result<(), ConstructionRegistrationError> {
    registry.try_register_recipe(
        recipe_authored_ground_item(),
        OWNER,
        "authored-room",
        SCHEMA,
    )?;
    registry.try_register_recipe(recipe_staged_actor(), OWNER, "content-staging", SCHEMA)?;
    registry.try_register_recipe(recipe_summoned_minion(), OWNER, "summon-effect", SCHEMA)?;
    registry.try_register_relation(relation_grudge(), OWNER, "aggression", SCHEMA, grudge_ops())?;
    Ok(())
}

// ── Request builders ─────────────────────────────────────────────────────────

/// Turn a room's authored ground items into construction requests, resolving
/// each held item while nothing has been mutated.
pub fn authored_ground_item_requests(
    room: &crate::rooms::RoomSpec,
) -> Result<Vec<ActorConstructionRequest>, ActorConstructionError> {
    room.ground_items
        .iter()
        .map(|spec| {
            let held =
                ambition_characters::brain::held_item_by_id(&spec.held_item).ok_or_else(|| {
                    ActorConstructionError::UnknownHeldItem {
                        authored_id: spec.id.clone(),
                        item: spec.held_item.clone(),
                    }
                })?;
            Ok(ActorConstructionRequest {
                sim_id: SimId::placement(&spec.id),
                origin: SpawnOrigin::Authored {
                    source: room.id.clone(),
                    instance: spec.id.clone(),
                },
                parameters: ActorConstructionParams::GroundItem {
                    spec: spec.clone(),
                    held,
                },
                relations: Vec::new(),
            })
        })
        .collect()
}

/// Turn the room's content-staged actors into construction requests. A
/// `grudge_against` becomes a declared relation, so an id naming nobody fails
/// the plan instead of being dropped.
pub fn staged_actor_requests(
    room_id: &str,
    provider: &str,
    requests: &[SpawnActorRequest],
) -> Vec<ActorConstructionRequest> {
    requests
        .iter()
        .map(|request| ActorConstructionRequest {
            sim_id: SimId::placement(&request.id),
            origin: SpawnOrigin::ProviderStaged {
                provider: provider.to_string(),
                room: room_id.to_string(),
                instance: request.id.clone(),
            },
            parameters: ActorConstructionParams::StagedActor(request.clone()),
            relations: request
                .grudge_against
                .iter()
                .map(
                    |foe| ambition_platformer_primitives::construction::RelationRequest {
                        kind: relation_grudge(),
                        to: SimId::placement(foe),
                    },
                )
                .collect(),
        })
        .collect()
}

/// Build the request for one summoned minion.
///
/// `summoner` and `sequence` come from the summoning body's own `SimId` and
/// `SimIdCounter`, which is what makes the resulting identity deterministic and
/// its provenance explicit rather than implied by the id's spelling.
pub fn summoned_minion_request(
    summoner: &SimId,
    sequence: u64,
    params: SummonedMinionParams,
) -> ActorConstructionRequest {
    ActorConstructionRequest {
        sim_id: SimId::spawned(summoner, sequence),
        origin: SpawnOrigin::Dynamic {
            parent: summoner.clone(),
            sequence,
        },
        parameters: ActorConstructionParams::SummonedMinion(params),
        relations: Vec::new(),
    }
}
