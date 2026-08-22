//! **The actor construction domain: three origins, one planner.**
//!
//! `docs/planning/engine/immutable-content-and-transactional-construction.md`
//! Phase 3 asks for one authored placement, one provider-staged actor, and one
//! runtime-dynamic family to share a pure, preflightable planner and a
//! recipe-backed reconstruction path. These are those three:
//!
//! That table is the historical seed of this domain, not a claim that every
//! authoritative room family belongs here. The completed migration put every
//! actor-owned family on the planner; construction federation now lets optional
//! capabilities keep their own closed `ConstructionDomain` and compose a named
//! room lane instead. The portal-gun pickup is the first such departure.
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

use ambition_boss_encounter::behavior::BossBehaviorProfileExt;
use ambition_characters::actor::limb::{Limb, LimbRig, LimbSlot};
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionDomain, ConstructionExecCtx, ConstructionPlan, ConstructionRegistrationError,
    ConstructionRegistry, ConstructionRequest, ConstructionRoot, RecipeDispatch, RecipeId,
    RelationCheck, RelationDispatch, RelationKind, RelationOps, SpawnOrigin,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::{Entity, World};

use crate::features::{SpawnActorKind, SpawnActorRequest};
use crate::world::placements::ActorPlacementContext;
use ambition_boss_encounter::BossCatalog;

#[cfg(test)]
mod tests;

/// An LDtk-authored ground item (a walk-into pickup).
pub const RECIPE_AUTHORED_GROUND_ITEM: &str = "ambition.authored-ground-item";
/// An actor a provider staged into a room during construction.
pub const RECIPE_STAGED_ACTOR: &str = "ambition.staged-actor";
/// A minion the running simulation summoned.
pub const RECIPE_SUMMONED_MINION: &str = "ambition.summoned-minion";
/// A `"giant"`-class limbed host — an authored enemy that carries a rig.
pub const RECIPE_GIANT_HOST: &str = "ambition.giant-host";
/// One hand of a giant host.
pub const RECIPE_GIANT_HAND: &str = "ambition.giant-hand";
/// An ordinary authored enemy pulled into the planner because a relation
/// (today: an authored mount link) names it.
pub const RECIPE_AUTHORED_ENEMY: &str = "ambition.authored-enemy";
/// One authored placement record, lowered through its frozen interpreter.
pub const RECIPE_AUTHORED_PLACEMENT: &str = "ambition.authored-placement";
/// An authored heal/save shrine.
pub const RECIPE_AUTHORED_SHRINE: &str = "ambition.authored-shrine";
/// An authored gravity zone.
/// An authored boss pulled into the planner because a relation names it.
pub const RECIPE_AUTHORED_BOSS: &str = "ambition.authored-boss";
/// A personal grudge from one constructed actor onto another.
pub const RELATION_GRUDGE: &str = "ambition.grudge";
/// A driven limb belonging to a host body's rig. **Bidirectional**: `Limb` on
/// the limb, an entry in the host's `LimbRig` going back.
pub const RELATION_LIMB: &str = "ambition.limb";
/// A rider seated on a mount. **Bidirectional**: `RidingOn` on the rider,
/// `MountSlot` on the mount going back.
pub const RELATION_MOUNT: &str = "ambition.mount";

pub const ACTOR_CONSTRUCTION_DOMAIN: &str = "actor";

const OWNER: &str = "ambition_platformer2d_actor_monolith";
// v2: relation wiring and postconditions changed — the rig became slot-keyed, and limb/mount
// verification now checks home offset, `Mounted`, and mount capabilities.
const SCHEMA: &str = "actor-construction-v2";

pub fn recipe_authored_ground_item() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_GROUND_ITEM)
}
pub fn recipe_staged_actor() -> RecipeId {
    RecipeId::new(RECIPE_STAGED_ACTOR)
}
pub fn recipe_summoned_minion() -> RecipeId {
    RecipeId::new(RECIPE_SUMMONED_MINION)
}
pub fn recipe_giant_host() -> RecipeId {
    RecipeId::new(RECIPE_GIANT_HOST)
}
pub fn recipe_giant_hand() -> RecipeId {
    RecipeId::new(RECIPE_GIANT_HAND)
}
pub fn recipe_authored_enemy() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_ENEMY)
}
pub fn recipe_authored_boss() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_BOSS)
}
pub fn recipe_authored_placement() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_PLACEMENT)
}
pub fn recipe_authored_shrine() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_SHRINE)
}
pub fn relation_grudge() -> RelationKind {
    RelationKind::new(RELATION_GRUDGE)
}
pub fn relation_limb() -> RelationKind {
    RelationKind::new(RELATION_LIMB)
}
pub fn relation_mount() -> RelationKind {
    RelationKind::new(RELATION_MOUNT)
}

/// **What one declared actor relation IS** — the kind and everything the pairing
/// carries, in one value.
///
/// **`Limb` carries the slot and the home offset because both are stated
/// relative to the HOST.** `LimbSlot::HandLeft` is meaningless without saying
/// left hand *of what*, and `home_offset` is documented as a "host-local
/// (body-frame) idle anchor" — it is read as `host.pos + gravity_frame(offset)`.
/// Neither is a property the limb owns on its own, so neither belongs in the
/// limb's construction parameters: that would put host-relative data on a body
/// that does not learn its host until the relation is wired.
///
/// This was `ActorRelationPayload`, requested alongside a separately-supplied
/// `RelationKind`. [`ActorConstruction::dispatch_relation`] derives the kind from
/// the variant now, so `kind: ambition.limb` beside `payload: Grudge` — which
/// passed preparation and blew up inside the wiring function mid-commit — is no
/// longer expressible.
#[derive(Clone, Debug, PartialEq)]
pub enum ActorRelation {
    /// A grudge is fully described by who resents whom.
    Grudge,
    /// Which slot of the host's rig this limb fills, and where it rests.
    Limb {
        slot: LimbSlot,
        home_offset: ambition_platformer2d_core::Vec2,
    },
    /// A rider seated on a mount. Fully described by who rides what: the saddle
    /// offset and the control grant are properties of the MOUNT's archetype
    /// (`Mountable`), not of the pairing.
    Mount,
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
    /// A `"giant"`-class limbed host: an ordinary authored enemy body plus the host-side rig state
    /// its hands' limb relations attach to.
    GiantHost {
        authored: crate::rooms::Authored<crate::rooms::EnemySpawnSpec>,
        faction: crate::features::ActorFaction,
        paths: Vec<(String, ambition_platformer2d_core::KinematicPath)>,
    },
    /// One hand of a giant host. The body is built here; its `Limb` component and
    /// the host's rig entry are installed by the `ambition.limb` relation.
    GiantHand {
        authored: crate::rooms::Authored<crate::rooms::EnemySpawnSpec>,
    },
    /// An ordinary authored enemy. Every authored enemy is a plan row, built by
    /// the same populate function the former family loop used.
    AuthoredEnemy {
        authored: crate::rooms::Authored<crate::rooms::EnemySpawnSpec>,
        paths: Vec<(String, ambition_platformer2d_core::KinematicPath)>,
    },
    /// An authored boss. Every authored boss is a plan row, built by the same
    /// populate function the former boss loop used, with default overrides.
    AuthoredBoss {
        authored: crate::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
    },
    /// One authored placement record beside its ALREADY-RESOLVED interpreter —
    /// the exact `(record, fn)` pair `PlacementLoweringPlan` froze at
    /// preparation, promoted to a plan row so the executor allocates the root
    /// and stamps identity/provenance/ownership on the same body the
    /// interpreter populates. The fn pointer never reaches the canonical dump
    /// or the plan id ([`ActorConstruction::canonical_summary`] prints the
    /// record identity + kind); it is executable freight, exactly like every
    /// row's frozen `construct`.
    Placement {
        record: crate::world::placements::PlacementRecord,
        paths: Vec<(String, ambition_platformer2d_core::KinematicPath)>,
        lower: crate::world::placements::LoweringFn,
    },
    /// Now it is a plan row like everything else in the room.
    Shrine {
        spec: crate::rooms::ShrineSpec,
    },
}

/// A minion resolved from `Effect::Summon`.
#[derive(Clone, Debug)]
pub struct SummonedMinionParams {
    /// Stable feature id, which is what per-entity systems (targeting,
    /// encounter bookkeeping) join on. Distinct from the row's `SimId`, which
    /// is the summoner-relative spawned identity.
    pub feature_id: String,
    pub name: String,
    pub pos: ambition_platformer2d_core::Vec2,
    pub half_size: ambition_platformer2d_core::Vec2,
    pub character_id: String,
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
    type Relation = ActorRelation;
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
            ActorConstructionParams::GiantHost { .. } => RecipeDispatch {
                recipe: recipe_giant_host(),
                construct: construct_giant_host,
            },
            ActorConstructionParams::GiantHand { .. } => RecipeDispatch {
                recipe: recipe_giant_hand(),
                construct: construct_giant_hand,
            },
            ActorConstructionParams::AuthoredEnemy { .. } => RecipeDispatch {
                recipe: recipe_authored_enemy(),
                construct: construct_authored_enemy,
            },
            ActorConstructionParams::AuthoredBoss { .. } => RecipeDispatch {
                recipe: recipe_authored_boss(),
                construct: construct_authored_boss,
            },
            ActorConstructionParams::Placement { .. } => RecipeDispatch {
                recipe: recipe_authored_placement(),
                construct: construct_placement,
            },
            ActorConstructionParams::Shrine { .. } => RecipeDispatch {
                recipe: recipe_authored_shrine(),
                construct: construct_shrine,
            },
        }
    }

    /// ONE match: each arm names the relation's stable kind AND the two frozen
    /// halves of its behaviour. The kind is therefore a function of the variant,
    /// which is what makes a kind/payload mismatch unrepresentable — and the ops
    /// come from here rather than from a registry lookup, so nothing outside this
    /// crate can supply, replace, or race to install actor relation wiring.
    fn dispatch_relation(relation: &Self::Relation) -> RelationDispatch<Self> {
        match relation {
            ActorRelation::Grudge => RelationDispatch {
                kind: relation_grudge(),
                ops: RelationOps {
                    wire: wire_grudge,
                    verify: verify_grudge,
                },
            },
            ActorRelation::Limb { .. } => RelationDispatch {
                kind: relation_limb(),
                ops: RelationOps {
                    wire: wire_limb,
                    verify: verify_limb,
                },
            },
            ActorRelation::Mount => RelationDispatch {
                kind: relation_mount(),
                ops: RelationOps {
                    wire: wire_mount,
                    verify: verify_mount,
                },
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
                format!("minion {} {}", minion.feature_id, minion.character_id)
            }
            ActorConstructionParams::GiantHost { authored, .. } => {
                format!("giant-host {} {}", authored.id, authored.name)
            }
            ActorConstructionParams::GiantHand { authored } => {
                format!("giant-hand {}", authored.id)
            }
            ActorConstructionParams::AuthoredEnemy { authored, .. } => {
                format!("authored-enemy {} {}", authored.id, authored.name)
            }
            ActorConstructionParams::AuthoredBoss { authored } => {
                format!("authored-boss {} {}", authored.id, authored.name)
            }
            ActorConstructionParams::Placement { record, .. } => {
                format!(
                    "placement {} {}",
                    record.id.as_str(),
                    record.kind().stable_id()
                )
            }
            ActorConstructionParams::Shrine { spec } => format!("shrine {}", spec.id),
        }
    }

    fn canonical_relation_summary(relation: &Self::Relation) -> String {
        match relation {
            ActorRelation::Grudge => "-".to_string(),
            ActorRelation::Limb { slot, home_offset } => format!(
                "{} {} {}",
                limb_slot_key(*slot),
                home_offset.x,
                home_offset.y,
            ),
            ActorRelation::Mount => "-".to_string(),
        }
    }
}

pub type ActorConstructionRegistry = ConstructionRegistry<ActorConstruction>;
pub type ActorConstructionPlan = ConstructionPlan<ActorConstruction>;
pub type ActorConstructionRequest = ConstructionRequest<ActorConstruction>;
type Ctx<'w, 's, 'a> = ConstructionExecCtx<'w, 's, 'a, ActorConstruction>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActorConstructionError {
    /// A ground item names a held-item spec no registry provides.
    ///
    /// Carries the binding failure verbatim rather than restating it, so the one authority that
    /// REFUSES the room is also the one with the good diagnostic — who declared it, what was
    /// available, and the likely typo.
    UnknownHeldItem(Box<ambition_platformer2d_shared_tangle::binding::UnresolvedRef>),
    /// One limb declares two hosts. A limb is a part OF a body; two hosts is not
    /// a configuration with a degraded meaning, it is a contradiction.
    LimbHasTwoHosts { limb: SimId, hosts: Vec<SimId> },
    /// Two limbs claim the same slot of the same host. The rig is keyed by slot,
    /// so committing this would silently drop one of them.
    LimbSlotTaken {
        host: SimId,
        slot: &'static str,
        limbs: Vec<SimId>,
    },
    /// One rider declares two mounts.
    RiderOnTwoMounts { rider: SimId, mounts: Vec<SimId> },
    /// Two riders claim the same mount. `MountSlot` holds ONE rider, so
    /// committing this would leave whichever lost pointing at a mount that
    /// points at the other.
    MountHasTwoRiders { mount: SimId, riders: Vec<SimId> },
    /// An entity declares itself its own mount.
    SelfMount { rider: SimId },
    /// A relation endpoint names a row whose construction family cannot hold
    /// that end of the relation — a ground item cannot be a mount.
    WrongFamilyForRelation {
        sim_id: SimId,
        relation: &'static str,
        end: &'static str,
        family: &'static str,
    },
    /// The rider's archetype does not list the mount's class among the classes it
    /// can pilot. Checked while planning, so an illegal pairing never reaches a
    /// world — the live path drops it silently instead.
    IncompatibleMountClass {
        rider: SimId,
        mount: SimId,
        mount_class: String,
        rider_classes: Vec<String>,
    },
    /// An authored mount link names an id no enemy or boss spawn in the room
    /// carries. The live resolver retried such a pair forever, silently; a
    /// typo'd link is a content error and fails the room while it is whole.
    MountLinkNamesNobody {
        room: String,
        end: &'static str,
        id: String,
    },
    /// A row whose body is built FROM a character names no character at all.
    ///
    /// A row names a character this composition has not registered.
    BodyCharacterNotRegistered { sim_id: SimId, character: String },
    /// A row names a REGISTERED character whose definition cannot build a body.
    ///
    /// Distinct from the variant above because the fix is different: the
    /// character exists and is missing facts (see `MissingCharacterFacts`),
    /// rather than being absent from this composition's cast.
    BodyCharacterIsIncomplete {
        sim_id: SimId,
        character: String,
        missing: String,
    },
}

impl std::fmt::Display for ActorConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownHeldItem(unresolved) => write!(f, "{unresolved}"),
            Self::LimbHasTwoHosts { limb, hosts } => write!(
                f,
                "limb `{limb}` declares {} hosts ({}); a limb belongs to exactly one body",
                hosts.len(),
                join_ids(hosts),
            ),
            Self::LimbSlotTaken { host, slot, limbs } => write!(
                f,
                "host `{host}` has {} limbs claiming slot `{slot}` ({}); the rig holds one limb \
                 per slot",
                limbs.len(),
                join_ids(limbs),
            ),
            Self::RiderOnTwoMounts { rider, mounts } => write!(
                f,
                "rider `{rider}` declares {} mounts ({}); a rider is seated on one",
                mounts.len(),
                join_ids(mounts),
            ),
            Self::MountHasTwoRiders { mount, riders } => write!(
                f,
                "mount `{mount}` is claimed by {} riders ({}); a mount seats one",
                riders.len(),
                join_ids(riders),
            ),
            Self::SelfMount { rider } => {
                write!(f, "`{rider}` declares itself as its own mount")
            }
            Self::WrongFamilyForRelation {
                sim_id,
                relation,
                end,
                family,
            } => write!(
                f,
                "`{sim_id}` is the {end} of relation `{relation}` but is constructed as a \
                 `{family}`, which cannot hold that end"
            ),
            Self::IncompatibleMountClass {
                rider,
                mount,
                mount_class,
                rider_classes,
            } => write!(
                f,
                "rider `{rider}` cannot pilot mount `{mount}` of class `{mount_class}`: it pilots \
                 [{}]",
                rider_classes.join(", "),
            ),
            Self::MountLinkNamesNobody { room, end, id } => write!(
                f,
                "room `{room}` authors a mount link whose {end} `{id}` matches no enemy or boss \
                 spawn in the room"
            ),
            Self::BodyCharacterNotRegistered { sim_id, character } => write!(
                f,
                "`{sim_id}` names character `{character}`, which this composition has not \
                 registered, and nothing else can build this body — it would spawn as a stranger \
                 wearing that character's name. Register the character, or name one this \
                 composition publishes"
            ),
            Self::BodyCharacterIsIncomplete {
                sim_id,
                character,
                missing,
            } => write!(
                f,
                "`{sim_id}` names character `{character}`, which is registered but cannot build a \
                 body: {missing}"
            ),
        }
    }
}

fn join_ids(ids: &[SimId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
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
        &ctx.services.context.sheets,
        // ⭐ the cast this construction context has carried all along — the
        // staged path simply never asked for it.
        &ctx.services.context.prepared,
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
        &ctx.services.context.sheets,
        &ctx.services.context.prepared,
        ctx.session,
        root.entity(),
        minion.feature_id.clone(),
        minion.name.clone(),
        minion.pos,
        minion.half_size,
        &minion.character_id,
        minion.encounter_id.clone(),
        minion.faction,
        crate::features::ActorAggression::hostile(),
    );
}

fn construct_giant_host(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::GiantHost {
        authored,
        faction,
        paths,
    } = parameters
    else {
        unreachable!("dispatch pairs this fn with GiantHost parameters")
    };
    crate::features::populate_giant_host_into(
        ctx.commands,
        &ctx.services.context.characters,
        &ctx.services.context.sheets,
        &ctx.services.context.prepared,
        &ctx.services.context.brain_profiles,
        ctx.session,
        root.entity(),
        authored,
        paths,
        *faction,
    );
}

fn construct_giant_hand(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::GiantHand { authored } = parameters else {
        unreachable!("dispatch pairs this fn with GiantHand parameters")
    };
    crate::features::populate_giant_hand_into(
        ctx.commands,
        &ctx.services.context.characters,
        &ctx.services.context.sheets,
        &ctx.services.context.prepared,
        &ctx.services.context.brain_profiles,
        ctx.session,
        root.entity(),
        authored,
    );
}

fn construct_authored_enemy(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::AuthoredEnemy { authored, paths } = parameters else {
        unreachable!("dispatch pairs this fn with AuthoredEnemy parameters")
    };
    crate::features::spawn_enemy_with_faction_into(
        ctx.commands,
        &ctx.services.context.characters,
        &ctx.services.context.sheets,
        &ctx.services.context.prepared,
        &ctx.services.context.brain_profiles,
        ctx.session,
        root.entity(),
        authored,
        paths,
        crate::features::ActorFaction::Enemy,
    );
}

fn construct_authored_boss(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::AuthoredBoss { authored } = parameters else {
        unreachable!("dispatch pairs this fn with AuthoredBoss parameters")
    };
    crate::features::spawn_boss_with_overrides_into(
        ctx.commands,
        &ctx.services.boss_catalog,
        ctx.session,
        root.entity(),
        authored,
        &ambition_boss_encounter::BossOverrides::default(),
    );
}

fn construct_shrine(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::Shrine { spec } = parameters else {
        unreachable!("dispatch pairs this fn with Shrine parameters")
    };
    crate::features::ecs::spawn_static::spawn_shrine_into(
        ctx.commands,
        ctx.session,
        root.entity(),
        spec,
    );
}

fn construct_placement(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::Placement {
        record,
        paths,
        lower,
    } = parameters
    else {
        unreachable!("dispatch pairs this fn with Placement parameters")
    };
    let mut lowering = crate::world::placements::LoweringCtx {
        commands: ctx.commands,
        room_id: ctx.scope.room.as_deref().unwrap_or(""),
        paths,
        session_scope: ctx.session,
        root: root.entity(),
        context: &ctx.services.context,
    };
    lower(record, &mut lowering);
}

// ── Relations ────────────────────────────────────────────────────────────────

/// Stable dump/diagnostic key for a limb slot. It is independent of variant names
/// so renaming a `LimbSlot` does not rewrite recorded plans.
fn limb_slot_key(slot: LimbSlot) -> &'static str {
    match slot {
        LimbSlot::HandLeft => "hand_left",
        LimbSlot::HandRight => "hand_right",
    }
}

/// Wire a limb to its host: `Limb` on the limb, an entry in the host's
/// `LimbRig` going back. **One function writes both ends.**
///
/// The rig is keyed by slot, so this INSERTS AT THE SLOT rather than appending —
/// a host with two hands is two relations filling two keys. Iteration order is
/// therefore the slot order, a property of the content, and neither the relation
/// order nor the spawn order can perturb it.
///
/// The insert needs the host's CURRENT rig, which deferred `Commands` cannot
/// read, so it queues an exclusive-world step. That step runs in queue order
/// alongside every other relation's, which is what keeps the composition
/// deterministic.
fn wire_limb(limb: Entity, host: Entity, relation: &ActorRelation, ctx: &mut Ctx<'_, '_, '_>) {
    let ActorRelation::Limb { slot, home_offset } = relation else {
        unreachable!("dispatch_relation pairs this fn with the Limb variant")
    };
    let (slot, home_offset) = (*slot, *home_offset);
    ctx.commands.entity(limb).insert(Limb {
        of: host,
        slot,
        home_offset,
    });
    ctx.commands.queue(move |world: &mut World| {
        let Ok(mut host_ref) = world.get_entity_mut(host) else {
            return;
        };
        if let Some(mut rig) = host_ref.get_mut::<LimbRig>() {
            rig.limbs.insert(slot, limb);
        } else {
            host_ref.insert(LimbRig::from_pairs([(slot, limb)]));
        }
    });
}

/// Checking only `Limb.of` would accept a limb the host's rig does not drive —
/// the fan-out iterates the RIG, so a limb missing from it is inert while
/// looking perfectly attached from its own side. Checking the slot only on the
/// limb would accept a rig that files it under a different one. And the home
/// offset is checked because it is the limb's entire idle behaviour: a limb
/// wired correctly with a corrupted anchor station-keeps to the wrong place
/// forever, which no structural check would ever notice.
fn verify_limb(
    world: &World,
    limb: Entity,
    host: Entity,
    relation: &ActorRelation,
) -> RelationCheck {
    let ActorRelation::Limb { slot, home_offset } = relation else {
        unreachable!("dispatch_relation pairs this fn with the Limb variant")
    };
    let Some(attached) = world.get::<Limb>(limb) else {
        return RelationCheck::NotInstalled;
    };
    if attached.of != host {
        return RelationCheck::WrongTarget {
            found: Some(attached.of),
        };
    }
    if attached.slot != *slot {
        return RelationCheck::PayloadMismatch { field: "slot" };
    }
    if attached.home_offset != *home_offset {
        return RelationCheck::PayloadMismatch {
            field: "home_offset",
        };
    }
    let Some(rig) = world.get::<LimbRig>(host) else {
        return RelationCheck::ReverseMismatch { found: None };
    };
    // The rig must file this limb under the planned slot, and nowhere else. A
    // slot-keyed map cannot hold the same key twice, but it CAN hold one limb
    // under two different slots — which drives it from two intent streams.
    let occupants: Vec<LimbSlot> = rig
        .limbs
        .iter()
        .filter(|(_, &entity)| entity == limb)
        .map(|(&slot, _)| slot)
        .collect();
    match occupants.as_slice() {
        [] => RelationCheck::ReverseMismatch {
            found: rig.get(*slot),
        },
        [found] if found == slot => RelationCheck::Installed,
        [_] => RelationCheck::PayloadMismatch { field: "rig_slot" },
        many => RelationCheck::DuplicateMembership { count: many.len() },
    }
}

pub fn planned_rig_for_host(
    plan: &ActorConstructionPlan,
    host: &SimId,
) -> std::collections::BTreeMap<LimbSlot, SimId> {
    plan.relations()
        .iter()
        .filter(|relation| relation.to() == host)
        .filter_map(|relation| match relation.relation() {
            ActorRelation::Limb { slot, .. } => Some((*slot, relation.from().clone())),
            ActorRelation::Grudge | ActorRelation::Mount => None,
        })
        .collect()
}

/// Verify that every committed [`LimbRig`] exactly matches the construction plan.
///
/// Per-relation checks cannot detect surplus rig entries, so this pass compares the
/// full slot-to-entity map, rejects duplicate or unplanned membership, and verifies
/// each occupant's forward [`Limb`] agrees on host and slot. Violations are returned
/// as [`RosterViolation::RigComposition`].
pub fn verify_rig_composition(
    plan: &ActorConstructionPlan,
    receipt: &ambition_platformer2d_shared_tangle::construction::ConstructionReceipt,
    world: &World,
) -> Vec<ambition_platformer2d_shared_tangle::construction::RosterViolation> {
    use ambition_platformer2d_shared_tangle::construction::RosterViolation;
    let mut violations = Vec::new();
    for row in plan.entities() {
        let host_sim = row.sim_id();
        let planned = planned_rig_for_host(plan, host_sim);
        let Some(host_entity) = receipt.entity(host_sim) else {
            // Never committed: the generic roster pass already reports it, and
            // there is no world-side rig to compare.
            continue;
        };
        let committed: std::collections::BTreeMap<LimbSlot, Entity> = world
            .get::<LimbRig>(host_entity)
            .map(|rig| rig.limbs.clone())
            .unwrap_or_default();
        if planned.is_empty() && committed.is_empty() {
            continue;
        }
        let mut fault = |detail: String| {
            violations.push(RosterViolation::RigComposition {
                host: host_sim.clone(),
                detail,
            });
        };
        // Slot-keyed comparison over the UNION of both sides, so surplus slots
        // are as visible as missing ones.
        let slots: std::collections::BTreeSet<LimbSlot> =
            planned.keys().chain(committed.keys()).copied().collect();
        for slot in slots {
            match (planned.get(&slot), committed.get(&slot)) {
                (Some(limb_sim), Some(&occupant)) => {
                    match receipt.entity(limb_sim) {
                        Some(expected) if expected == occupant => {
                            // Right occupant; now the forward half must agree.
                            match world.get::<Limb>(occupant) {
                                None => fault(format!(
                                    "slot {slot:?} occupant `{limb_sim}` carries no Limb component"
                                )),
                                Some(limb) if limb.of != host_entity => fault(format!(
                                    "slot {slot:?} occupant `{limb_sim}` answers to \
                                     {:?}, not its host {host_entity:?}",
                                    limb.of
                                )),
                                Some(limb) if limb.slot != slot => fault(format!(
                                    "slot {slot:?} occupant `{limb_sim}` believes it fills \
                                     {:?}",
                                    limb.slot
                                )),
                                Some(_) => {}
                            }
                        }
                        Some(expected) => fault(format!(
                            "slot {slot:?} holds {occupant:?}, but the plan committed \
                             `{limb_sim}` onto {expected:?}"
                        )),
                        None => fault(format!(
                            "slot {slot:?} names planned limb `{limb_sim}` which never committed"
                        )),
                    }
                }
                (Some(limb_sim), None) => {
                    fault(format!(
                        "planned slot {slot:?} (limb `{limb_sim}`) is empty"
                    ));
                }
                (None, Some(&occupant)) => {
                    fault(format!(
                        "slot {slot:?} holds {occupant:?}, which the plan never described"
                    ));
                }
                (None, None) => unreachable!("slot came from the union of both maps"),
            }
        }
        // A limb entity answering to two slots is one body wearing two names —
        // invisible to the per-slot pass when each slot individually "matches".
        let mut seen: std::collections::BTreeMap<Entity, LimbSlot> =
            std::collections::BTreeMap::new();
        for (&slot, &occupant) in &committed {
            if let Some(&first) = seen.get(&occupant) {
                violations.push(RosterViolation::RigComposition {
                    host: host_sim.clone(),
                    detail: format!("{occupant:?} occupies both {first:?} and {slot:?}"),
                });
            } else {
                seen.insert(occupant, slot);
            }
        }
    }
    violations
}

/// Wire a rider onto a mount: `RidingOn` + `Mounted` on the rider, `MountSlot`
/// on the mount going back. **One function writes both ends.**
fn wire_mount(rider: Entity, mount: Entity, _relation: &ActorRelation, ctx: &mut Ctx<'_, '_, '_>) {
    ctx.commands.entity(rider).insert((
        crate::features::RidingOn { mount },
        crate::features::Mounted,
    ));
    ctx.commands
        .entity(mount)
        .insert(crate::features::MountSlot { rider: Some(rider) });
}

/// That leaves a rider pointing at a mount that does not point back, and
/// `steer_mount_from_rider` queries `With<MountSlot>`, so the mount stops obeying while every
/// rider-side assertion still passes.
fn verify_mount(
    world: &World,
    rider: Entity,
    mount: Entity,
    _relation: &ActorRelation,
) -> RelationCheck {
    let Some(riding) = world.get::<crate::features::RidingOn>(rider) else {
        return RelationCheck::NotInstalled;
    };
    if riding.mount != mount {
        return RelationCheck::WrongTarget {
            found: Some(riding.mount),
        };
    }
    // `Mounted` is not decoration: `steer_mount_from_rider` queries
    // `With<Mounted>`, so a rider linked without it sits on a mount that never
    // receives its intent — a pair that points at each other and does nothing.
    if world.get::<crate::features::Mounted>(rider).is_none() {
        return RelationCheck::MissingCapability {
            component: "Mounted",
        };
    }
    // Both ends must still carry the capabilities the preflight approved them
    // on. A recipe that stripped `Mountable` leaves a link whose class nothing
    // can re-check, and `steer_mount_from_rider` reads `Mountable` to route.
    let Some(mountable) = world.get::<crate::features::Mountable>(mount) else {
        return RelationCheck::MissingCapability {
            component: "Mountable",
        };
    };
    match world.get::<crate::features::CanPilot>(rider) {
        Some(pilot) if pilot.can_pilot(&mountable.class) => {}
        Some(_) => {
            return RelationCheck::PayloadMismatch {
                field: "mount_class",
            }
        }
        None => {
            return RelationCheck::MissingCapability {
                component: "CanPilot",
            }
        }
    }
    match world
        .get::<crate::features::MountSlot>(mount)
        .and_then(|slot| slot.rider)
    {
        Some(back) if back == rider => RelationCheck::Installed,
        found => RelationCheck::ReverseMismatch { found },
    }
}

/// Wire a personal grudge. Re-inserting `ActorAggression` is safe: staged
/// fighters spawn `hostile()` already, so this only adds the grudge.
fn wire_grudge(from: Entity, to: Entity, _relation: &ActorRelation, ctx: &mut Ctx<'_, '_, '_>) {
    ctx.commands
        .entity(from)
        .insert(crate::features::ActorAggression {
            grudge: Some(to),
            ..crate::features::ActorAggression::hostile()
        });
}

/// The distinction matters because the two are separately fallible: the wiring
/// runs through deferred `Commands`, so a later command in the same flush can
/// overwrite `ActorAggression` wholesale, and the receipt records the call
/// either way. A grudge onto a stale pre-reconstruction entity also reads as
/// `WrongTarget` here — `found` names the corpse, which is what makes that case
/// diagnosable rather than merely wrong.
fn verify_grudge(
    world: &World,
    from: Entity,
    to: Entity,
    _relation: &ActorRelation,
) -> RelationCheck {
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
/// **This domain is CLOSED.** `ActorConstructionParams` is a closed enum and
/// [`ActorConstruction::dispatch`] a closed match, so the actor registry contains
/// metadata only for recipes the actor domain can actually dispatch. An outside
/// capability does not add a metadata-only alias here: unreachable schema entries
/// would be dead declarations with no executable construction behind them.
///
/// A capability that owns authoritative roots owns its own [`ConstructionDomain`],
/// typed registry, and named construction lane, then contributes only its stable
/// registry dump to `ConstructionSchemaCatalog` for prepared-content fingerprinting.
/// The portal-gun lane is the first production example. Callers that need an actor
/// registry of their own (fixtures, tools, or preflight outside a live `App`) build
/// one here rather than re-listing the actor recipes and drifting from the real table.
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
    registry.try_register_recipe(recipe_giant_host(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_giant_hand(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_authored_enemy(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_authored_boss(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_authored_placement(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_authored_shrine(), OWNER, "authored-room", SCHEMA)?;
    // Metadata only — the wiring and the checks come from
    // `ActorConstruction::dispatch_relation`, so there is nothing here for an
    // outside registration to replace or to win an insertion-order race for.
    registry.try_register_relation(relation_grudge(), OWNER, "aggression", SCHEMA)?;
    registry.try_register_relation(relation_limb(), OWNER, "limb-rig", SCHEMA)?;
    registry.try_register_relation(relation_mount(), OWNER, "mount-link", SCHEMA)?;
    Ok(())
}

// ── Relation preflight ───────────────────────────────────────────────────────

/// The mount capabilities a planned row will carry once it is constructed.
///
/// Derived from the same archetype data `attach_mount_role` and `spawn_boss`
/// read when they install [`crate::features::Mountable`] /
/// [`crate::features::CanPilot`], so a preflight decision here predicts the
/// world the commit will produce rather than guessing at it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlannedMountCapabilities {
    /// The class this row is rideable AS, if its archetype makes it a mount.
    pub mount_class: Option<String>,
    /// The classes this row may pilot.
    pub pilots: Vec<String>,
}

/// What a row will be able to do, mount-wise, once built.
pub fn mount_capabilities_of(
    parameters: &ActorConstructionParams,
    bosses: &BossCatalog,
    // **The prepared cast, when the caller has one.** A placement that names a
    // character takes its mount facts from the prepared definition. A placement
    // with no prepared character contributes no character-owned mount capability;
    // there is no archetype fallback. This keeps preflight on the same authority
    // the construction commit will use.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
) -> PlannedMountCapabilities {
    match parameters {
        // A pickup is neither rideable nor a pilot.
        ActorConstructionParams::GroundItem { .. } => PlannedMountCapabilities::default(),
        ActorConstructionParams::StagedActor(request) => match &request.kind {
            // ⭐ **THE CHARACTER FIRST, exactly like the authored arm below.**
            //
            // ⚠ the same call the authored road makes, so the two roads cannot
            // disagree about whether a body is rideable — which is the whole
            // point of there being one construction path.
            SpawnActorKind::Enemy { character, .. } => {
                authored_mount_capabilities(resolve_planned_character(prepared, character))
            }
            // A boss takes `CanPilot` from its behaviour profile and is never
            // itself a mount — `spawn_boss` installs no `Mountable`. Resolved
            // through the SAME pair `BossClusterScratch::new` uses, so the
            // preflight reads the profile the commit will read.
            SpawnActorKind::Boss { brain, .. } => PlannedMountCapabilities {
                mount_class: None,
                pilots: ambition_boss_encounter::behavior::BossBehaviorProfile::for_authored_boss(
                    bosses,
                    &ambition_boss_encounter::behavior::canonical_boss_id_from(
                        &request.name,
                        brain,
                    ),
                )
                .pilotable_mount_classes
                .clone(),
            },
        },
        // ⛔ a summoned minion names its body by STRING, and a boss casting a
        // spell for something nobody authored is the case
        // `every_summoned_minion_id_resolves_a_body` guards. That string names a
        // CHARACTER now — the summon road refuses anything else — so the plan
        // reads the same authority the commit will.
        ActorConstructionParams::SummonedMinion(minion) => authored_mount_capabilities(
            prepared.and_then(|cast| cast.get(minion.character_id.as_str())),
        ),
        // A giant host may be a mount when its prepared character authors that
        // capability; its hands are neither mount nor pilot.
        ActorConstructionParams::GiantHost { authored, .. }
        | ActorConstructionParams::AuthoredEnemy { authored, .. } => authored_mount_capabilities(
            resolve_planned_character(prepared, &authored.payload.character_id),
        ),
        ActorConstructionParams::GiantHand { .. } => PlannedMountCapabilities::default(),
        // Same profile resolution as the staged boss arm above — and never a
        // mount: the boss populate function installs no `Mountable`.
        // A placement is never a mount-link end today (links name enemy/boss
        // ids); an NPC that should ride something becomes an enemy/boss row.
        ActorConstructionParams::Placement { .. } => PlannedMountCapabilities::default(),
        ActorConstructionParams::Shrine { .. } => PlannedMountCapabilities::default(),
        ActorConstructionParams::AuthoredBoss { authored } => PlannedMountCapabilities {
            mount_class: None,
            pilots: ambition_boss_encounter::behavior::BossBehaviorProfile::for_authored_boss(
                bosses,
                &ambition_boss_encounter::behavior::canonical_boss_id_from(
                    &authored.name,
                    &authored.payload,
                ),
            )
            .pilotable_mount_classes
            .clone(),
        },
    }
}

/// **What a placement can ride and be ridden as** — the CHARACTER's answer, and
/// as of AC6 there is no other one.
///
/// A body that states no mount rides nothing, which is what a body that says nothing has always
/// meant.
fn authored_mount_capabilities(
    character: Option<&crate::character_runtime::PreparedCharacterDefinition>,
) -> PlannedMountCapabilities {
    character
        .and_then(|definition| definition.mount.as_ref())
        .map_or_else(PlannedMountCapabilities::default, |mount| {
            PlannedMountCapabilities {
                mount_class: mount.class.clone(),
                pilots: mount.pilotable_classes.clone(),
            }
        })
}

/// Which construction family a row is, for diagnostics and family-legality rules.
fn family_of(parameters: &ActorConstructionParams) -> &'static str {
    match parameters {
        ActorConstructionParams::GroundItem { .. } => "ground-item",
        ActorConstructionParams::StagedActor(_) => "staged-actor",
        ActorConstructionParams::SummonedMinion(_) => "summoned-minion",
        ActorConstructionParams::GiantHost { .. } => "giant-host",
        ActorConstructionParams::GiantHand { .. } => "giant-hand",
        ActorConstructionParams::AuthoredEnemy { .. } => "authored-enemy",
        ActorConstructionParams::AuthoredBoss { .. } => "authored-boss",
        ActorConstructionParams::Placement { .. } => "placement",
        ActorConstructionParams::Shrine { .. } => "shrine",
    }
}

/// **Which planned rows build their body FROM a character**, and what each one
/// names.
///
/// `None` means the family builds no character body — a shrine, a ground item,
/// a gravity zone — or builds it from another authority entirely (a boss reads
/// the [`BossCatalog`]; a placement lowers through the NPC road, which still has
/// a catalog-bodied fallback of its own).
/// The character each planned row builds its body from, when the row builds one.
///
/// The struct's last field is the return value.
fn planned_body_character(parameters: &ActorConstructionParams) -> Option<&str> {
    match parameters {
        ActorConstructionParams::AuthoredEnemy { authored, .. }
        | ActorConstructionParams::GiantHost { authored, .. }
        | ActorConstructionParams::GiantHand { authored } => {
            Some(authored.payload.character_id.as_str())
        }
        ActorConstructionParams::StagedActor(request) => match &request.kind {
            crate::features::SpawnActorKind::Enemy { character, .. } => Some(character.as_str()),
            // A staged boss builds from the boss catalog, like an authored one.
            crate::features::SpawnActorKind::Boss { .. } => None,
        },
        ActorConstructionParams::SummonedMinion(minion) => Some(minion.character_id.as_str()),
        ActorConstructionParams::AuthoredBoss { .. }
        | ActorConstructionParams::Placement { .. }
        | ActorConstructionParams::GroundItem { .. }
        | ActorConstructionParams::Shrine { .. } => None,
    }
}

/// One planned row's claim on a character, as the preflight needs to read it.
/// **Prove every planned body can actually be built, before anything is
/// mutated.**
///
/// ⛔⛔ **THIS IS THE HALF AC6 LEFT LATE.** Deleting the enemy-archetype ontology made an
/// unresolvable character honest — there is no generic `combatant` left to settle for — but the
/// refusal it became lives inside `spawn_enemy_with_faction_into`, which runs as a construction
/// RECIPE.
///
/// The construction contract says the opposite in [`ConstructionDomain::dispatch`]'s
/// own words — *"nothing here can fail: every lookup that could miss resolved in
/// the request builder"*. This is that resolution, performed against the SAME
/// registry the recipes will read (`construction.prepared` is what becomes
/// `ActorConstructionServices::context.prepared`), so passing here means the
/// execution-time lookup cannot miss.
///
/// ⛔ **it does not restore a fallback and must not grow into one.** The three
/// refusals below are the three ways a body cannot be built; each names the
/// placement and the character so the diagnostic is actionable, and the world is
/// whole when it is reported.
///
/// **an absent registry is an EMPTY cast, not an exemption.** `prepared: None` becomes a default
/// (empty) `PreparedCharacterRegistry` in the frozen services, so a composition that publishes no
/// cast cannot build a character body either.
pub fn preflight_planned_bodies(
    requests: &[ActorConstructionRequest],
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
) -> Result<(), ActorConstructionError> {
    for request in requests {
        let Some(character) = planned_body_character(&request.parameters) else {
            continue;
        };
        let Some(definition) = prepared.and_then(|cast| cast.get(character)) else {
            return Err(ActorConstructionError::BodyCharacterNotRegistered {
                sim_id: request.sim_id.clone(),
                character: character.to_string(),
            });
        };
        if let Err(missing) = definition.body_blueprint() {
            return Err(ActorConstructionError::BodyCharacterIsIncomplete {
                sim_id: request.sim_id.clone(),
                character: character.to_string(),
                missing: missing.to_string(),
            });
        }
    }
    Ok(())
}

/// Reject illegal actor relation configurations **before any entity is
/// spawned**.
///
/// The generic planner already refuses a duplicate `(from, kind, to)` and an
/// unresolved endpoint. Those are structural. The rules here are the actor
/// domain's own semantics, and each one names a way the live world silently
/// coped instead of refusing:
///
/// - a limb with two hosts, or two limbs in one slot: the slot-keyed rig would
///   drop one of them at commit and the plan would still claim both;
/// - a rider with two mounts, or two riders on one mount: `MountSlot` holds ONE
///   rider, so the loser ends up pointing at a mount that points elsewhere —
///   exactly the half-linked pair this campaign keeps finding;
/// - a self-mount: a body steering itself through `steer_mount_from_rider`;
/// - an endpoint whose family cannot hold that end: a ground item is not a body;
/// - an incompatible pilot/mount class: the deleted frame-later resolver
///   checked this too, and DROPPED the link with no diagnostic, so an authored
///   typo produced a rider standing next to its mount and no explanation.
///
/// Runs on requests, so a refusal happens while the outgoing room is whole.
pub fn preflight_actor_relations(
    requests: &[ActorConstructionRequest],
    bosses: &BossCatalog,
    // See [`mount_capabilities_of`].
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
) -> Result<(), ActorConstructionError> {
    use std::collections::BTreeMap;

    let family: BTreeMap<&SimId, &ActorConstructionParams> = requests
        .iter()
        .map(|request| (&request.sim_id, &request.parameters))
        .collect();

    // Ordered accumulators: a diagnostic that names "the two limbs in this slot"
    // must name them in the same order every run.
    let mut hosts_of_limb: BTreeMap<&SimId, Vec<SimId>> = BTreeMap::new();
    let mut limbs_in_slot: BTreeMap<(&SimId, LimbSlot), Vec<SimId>> = BTreeMap::new();
    let mut mounts_of_rider: BTreeMap<&SimId, Vec<SimId>> = BTreeMap::new();
    let mut riders_of_mount: BTreeMap<&SimId, Vec<SimId>> = BTreeMap::new();

    for request in requests {
        for declared in &request.relations {
            match &declared.relation {
                ActorRelation::Grudge => {}
                ActorRelation::Limb { slot, .. } => {
                    hosts_of_limb
                        .entry(&request.sim_id)
                        .or_default()
                        .push(declared.to.clone());
                    limbs_in_slot
                        .entry((&declared.to, *slot))
                        .or_default()
                        .push(request.sim_id.clone());
                }
                ActorRelation::Mount => {
                    if request.sim_id == declared.to {
                        return Err(ActorConstructionError::SelfMount {
                            rider: request.sim_id.clone(),
                        });
                    }
                    mounts_of_rider
                        .entry(&request.sim_id)
                        .or_default()
                        .push(declared.to.clone());
                    riders_of_mount
                        .entry(&declared.to)
                        .or_default()
                        .push(request.sim_id.clone());
                }
            }
        }
    }

    for (limb, hosts) in &hosts_of_limb {
        if hosts.len() > 1 {
            return Err(ActorConstructionError::LimbHasTwoHosts {
                limb: (*limb).clone(),
                hosts: hosts.clone(),
            });
        }
    }
    for ((host, slot), limbs) in &limbs_in_slot {
        if limbs.len() > 1 {
            return Err(ActorConstructionError::LimbSlotTaken {
                host: (*host).clone(),
                slot: limb_slot_key(*slot),
                limbs: limbs.clone(),
            });
        }
    }
    for (rider, mounts) in &mounts_of_rider {
        if mounts.len() > 1 {
            return Err(ActorConstructionError::RiderOnTwoMounts {
                rider: (*rider).clone(),
                mounts: mounts.clone(),
            });
        }
    }
    for (mount, riders) in &riders_of_mount {
        if riders.len() > 1 {
            return Err(ActorConstructionError::MountHasTwoRiders {
                mount: (*mount).clone(),
                riders: riders.clone(),
            });
        }
    }

    // Family legality and pilot/mount compatibility.
    for (rider, mounts) in &mounts_of_rider {
        let Some(mount) = mounts.first() else {
            continue;
        };
        let rider_params = family.get(*rider).copied();
        let mount_params = family.get(mount).copied();
        let (Some(rider_params), Some(mount_params)) = (rider_params, mount_params) else {
            continue;
        };
        let rider_caps = mount_capabilities_of(rider_params, bosses, prepared);
        let mount_caps = mount_capabilities_of(mount_params, bosses, prepared);
        let Some(mount_class) = mount_caps.mount_class.clone() else {
            return Err(ActorConstructionError::WrongFamilyForRelation {
                sim_id: mount.clone(),
                relation: RELATION_MOUNT,
                end: "mount",
                family: family_of(mount_params),
            });
        };
        if rider_caps.pilots.is_empty() {
            return Err(ActorConstructionError::WrongFamilyForRelation {
                sim_id: (*rider).clone(),
                relation: RELATION_MOUNT,
                end: "rider",
                family: family_of(rider_params),
            });
        }
        if !rider_caps.pilots.contains(&mount_class) {
            return Err(ActorConstructionError::IncompatibleMountClass {
                rider: (*rider).clone(),
                mount: mount.clone(),
                mount_class,
                rider_classes: rider_caps.pilots.clone(),
            });
        }
    }

    Ok(())
}

// ── Request builders ─────────────────────────────────────────────────────────

/// **Build this record somewhere other than where it says.**
///
/// The world remembers that the occurrence this record minted was carried
/// across the room and put down; rebuilding the room owes it back at the
/// position it was left, with the record's own identity. Only the position
/// moves: the recipe, the identity and the provenance are the record's, which
/// is what makes the result the SAME occurrence rather than a copy.
///
/// ⭐ **answers FALSE for a family that has no position of its own**, rather
/// than guessing one or silently ignoring the request. A `Placed` row can only
/// be written by a producer that read a position off a live occurrence, so a
/// false here means the ledger and the plan disagree about what kind of thing an
/// identity names — worth a caller's warning, never a silent authoring at the
/// wrong coordinates.
///
/// ⚠ **a free function because `ActorConstructionRequest` is an alias for a
/// generic in another crate**, so it cannot carry inherent methods here.
pub fn relocate_request(
    request: &mut ActorConstructionRequest,
    at: ambition_platformer2d_core::Vec2,
) -> bool {
    match &mut request.parameters {
        ActorConstructionParams::GroundItem { spec, .. } => {
            spec.pos = at;
            true
        }
        _ => false,
    }
}

/// **The records of `room` that ANOTHER room may have to build.**
///
/// An occurrence that was carried out of the room whose record minted it and
/// put down next door has to be rebuilt by the room it is lying in, from a
/// record that room does not own. This is the seam that hands it over: the
/// room being built asks each room of the world for the records it might owe,
/// keeps only the identities the ledger says are lying in it, and relocates
/// them.
///
/// ⭐ **the list is bounded by [`relocate_request`], and the two are one list
/// seen from two sides.** An occurrence gets a `Placed` row only from a producer
/// that read a POSITION off it, and it can be rebuilt at that position only if
/// `relocate_request` accepts its request — so a family joins both functions in
/// the same change. Today the list is authored ground items.
///
/// ⚠ **two gates hold that pairing, and they catch different mistakes.** The
/// exhaustive `RoomSpec` destructure in the body catches a family that is never
/// CONSIDERED — the compiler refuses a new field until someone classifies it.
/// `every_reinstatable_record_can_be_relocated` catches a family that is
/// considered and offered but cannot be put back where it was left; it walks the
/// requests a fixture room produces, so it is silent about any family that
/// fixture does not author, and a family joining this list owes that fixture a
/// row as well.
///
/// ⚠ **deliberately NOT the room's whole request derivation.** Lowered
/// placements, staged content and authored actors carry relations to rows the
/// asking room is not building, and dragging a subset of a foreign room's
/// relation graph across a boundary is a design question the customer that
/// forces it should answer. Nothing can write a `Placed` row for any of them
/// today.
pub fn reinstatable_authored_requests(
    room: &crate::rooms::RoomSpec,
) -> Result<Vec<ActorConstructionRequest>, ActorConstructionError> {
    // ⛔⛔ **EXHAUSTIVE ON PURPOSE: this is where the compiler asks whether a new
    // kind of authored content can be carried out of the room that authored it.**
    // The question is easy to answer and impossible to remember to ask, and the
    // cost of not asking is silent — a family that can leave the room but is not
    // offered here is simply never rebuilt by the room it is lying in, which
    // looks exactly like the object having been picked up.
    //
    // ⚠ **the paired test cannot ask it.** `every_reinstatable_record_can_be_relocated`
    // walks the requests a FIXTURE room produces, so it proves the pairing for
    // the families that fixture authors and is silent about every other one. It
    // is the second gate, not the first; this destructure is the first.
    let crate::rooms::RoomSpec {
        // ── the family this function offers ──────────────────────────────
        ground_items: _,

        // ── could be carried, and nothing can write a `Placed` row for one ──
        // Adding any of these here means adding a producer that records where
        // the occurrence was left AND an arm to `relocate_request`; see this
        // function's docs for why that is a design question rather than a
        // mechanical extension.
        portal_gun_spawns: _,
        enemy_spawns: _,
        boss_spawns: _,
        placements: _,

        // ── fixed to the room: geometry, graph, presentation and triggers ──
        // None of these is an object anybody can pick up and put down next
        // door, so none of them can be lying in a room that did not author it.
        id: _,
        world: _,
        loading_zones: _,
        metadata: _,
        camera_zones: _,
        kinematic_paths: _,
        moving_platforms: _,
        props: _,
        shrines: _,
        gravity_zones: _,
        debug_labels: _,
        mount_links: _,
        encounter_triggers: _,
        lock_walls: _,
        switch_commands: _,
    } = room;

    authored_ground_item_requests(room)
}

/// **THE RECORD THAT MINTED AN OCCURRENCE, wherever in the world it lives.**
///
/// ⭐ **an identity is not enough to rebuild something, and a ROOM is not enough
/// to find the recipe.** Every other reconstruction road in this engine starts
/// from a room and asks what it owes; this one starts from an OCCURRENCE that is
/// resident in no room at all — the checkpoint says a body was carrying it, and
/// the entity behind it was destroyed when some unrelated room unloaded. No room
/// build will ever produce it, because `outlook_for` correctly answers
/// `Suppressed` in every room for something that is supposed to be in a hand. So
/// the definition has to be reachable BY IDENTITY, and this is that reach.
///
/// ⭐ **built on [`reinstatable_authored_requests`], deliberately, so the
/// families stay ONE list.** The same pairing rule holds: a family becomes
/// materializable exactly when it becomes reinstatable, and neither road can
/// grow a family the other has not heard of.
///
/// ⚠ **a room whose own records refuse to resolve is SKIPPED, not fatal.** The
/// caller is a death restoring a checkpoint and has no way to refuse; the room
/// in question is already unbuildable and says so on its own next load, which is
/// a louder and better-placed report than aborting a search that was probably
/// not even about it.
///
/// ⚠ **`None` is a real answer**: the record an occurrence was minted from can
/// have been edited out of the content since the checkpoint was taken. The
/// caller states that loss rather than inventing a replacement.
pub fn authored_occurrence_request(
    world: &[crate::rooms::RoomSpec],
    occurrence: &SimId,
) -> Option<ActorConstructionRequest> {
    for room in world {
        let candidates = match reinstatable_authored_requests(room) {
            Ok(candidates) => candidates,
            Err(error) => {
                bevy::log::warn!(
                    target: "ambition_platformer2d::construction",
                    "room `{}` cannot yield its reinstatable records while looking for \
                     `{occurrence:?}`, so it is skipped: {error}",
                    room.id,
                );
                continue;
            }
        };
        if let Some(request) = candidates
            .into_iter()
            .find(|request| &request.sim_id == occurrence)
        {
            return Some(request);
        }
    }
    None
}

/// Turn a room's authored ground items into construction requests, resolving
/// each held item while nothing has been mutated.
pub fn authored_ground_item_requests(
    room: &crate::rooms::RoomSpec,
) -> Result<Vec<ActorConstructionRequest>, ActorConstructionError> {
    // Built once for the room, and only when it has ground items at all: this is
    // the registry the refusal below is measured against, and the list it names
    // when the reference misses.
    let registry = (!room.ground_items.is_empty()).then(|| {
        ambition_platformer2d_shared_tangle::binding::Resolver::<
                crate::rooms::binding::HeldItemId,
            >::new(ambition_characters::brain::held_item_ids())
    });
    room.ground_items
        .iter()
        .map(|spec| {
            let held =
                ambition_characters::brain::held_item_by_id(&spec.held_item).ok_or_else(|| {
                    let unresolved = registry
                        .as_ref()
                        .expect("a room with ground items built the registry")
                        .explain(&spec.held_item, format!("ground item `{}`", spec.id));
                    ActorConstructionError::UnknownHeldItem(Box::new(unresolved))
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
    // **The prepared cast, when the caller has one.** Planning asks the
    // CHARACTER whether a placement is a limbed host before it asks the roster
    // — see `features::is_limbed_host`. `None` is the host that has no cast
    // prepared, and it plans exactly as it did before.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
) -> Vec<ActorConstructionRequest> {
    let mut rows = Vec::new();
    for request in requests {
        let host_sim = SimId::placement(&request.id);
        let grudges: Vec<_> = request
            .grudge_against
            .iter()
            .map(
                |foe| ambition_platformer2d_shared_tangle::construction::RelationRequest {
                    to: SimId::placement(foe),
                    relation: ActorRelation::Grudge,
                },
            )
            .collect();
        // A staged `"giant"`-class enemy lowers to the SAME host + two hand rows
        // an authored giant does, through the one shared cluster helper — so a
        // giant is never a handless host regardless of which origin staged it.
        // (The pre-`e164f22` staged path routed every enemy through
        // `spawn_enemy_with_faction_into`, which no longer spawns hands, so a
        // staged giant lost its rig entirely.)
        if let SpawnActorKind::Enemy { brain, character } = &request.kind {
            // ⭐ **the CHARACTER decides, and it is the only thing that does.**
            // This also asked the roster, whose lookup could not fail — so an
            // unresolvable key answered the `combatant` row, which is not a
            // limbed host, so the two agreed by luck rather than by design.
            if crate::features::is_limbed_host(resolve_planned_character(prepared, character)) {
                let aabb = ambition_platformer2d_core::Aabb::new(request.pos, request.half_size);
                // Invisible while an archetype row could answer for the brain key; a refusal the
                // moment they went (AC6).
                let host_payload =
                    crate::rooms::EnemySpawnSpec::new(brain.clone(), character.clone());
                let host_authored = crate::rooms::Authored::new(
                    request.id.clone(),
                    request.name.clone(),
                    aabb,
                    host_payload,
                );
                let hands = crate::features::giant_hand_plans(&request.id, aabb);
                let room = room_id.to_string();
                let provider_owned = provider.to_string();
                let host_origin = SpawnOrigin::ProviderStaged {
                    provider: provider_owned.clone(),
                    room: room.clone(),
                    instance: request.id.clone(),
                };
                let mut cluster = giant_cluster_rows(
                    host_sim,
                    host_authored,
                    request.faction,
                    // Staged enemies carry no room-authored kinematic paths, the
                    // same as the pre-migration staged spawn (it passed `&[]`).
                    Vec::new(),
                    hands,
                    host_origin,
                    move |hand| SpawnOrigin::ProviderStaged {
                        provider: provider_owned.clone(),
                        room: room.clone(),
                        instance: hand.feature_id.clone(),
                    },
                );
                // The host keeps any declared grudge; the hands never carry one.
                if let Some(host) = cluster.first_mut() {
                    host.relations.extend(grudges);
                }
                rows.append(&mut cluster);
                continue;
            }
        }
        rows.push(ActorConstructionRequest {
            sim_id: host_sim,
            origin: SpawnOrigin::ProviderStaged {
                provider: provider.to_string(),
                room: room_id.to_string(),
                instance: request.id.clone(),
            },
            parameters: ActorConstructionParams::StagedActor(request.clone()),
            relations: grudges,
        });
    }
    rows
}

/// Turn EVERY authored enemy and boss into construction rows — the Phase-4
/// family migration for the two actor families.
///
/// An ordinary enemy is one [`ActorConstructionParams::AuthoredEnemy`] row; a `"giant"`-class
/// limbed host expands to one host row plus two hand rows joined by `ambition.limb` relations; a
/// boss is one [`ActorConstructionParams::AuthoredBoss`] row. Each row is built by the SAME
/// populate function the deleted family loop called, so being planned changes who allocates the
/// root, stamps identity/provenance/ownership, and wires/verifies relations — not what the actor
/// is.
pub fn authored_actor_requests(
    room: &crate::rooms::RoomSpec,
    paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    // See [`staged_actor_requests`].
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
) -> Vec<ActorConstructionRequest> {
    let mut requests = Vec::new();
    for enemy in &room.enemy_spawns {
        // See the twin in `staged_actor_requests`: a character states its limbs,
        // and nothing else does.
        if crate::features::is_limbed_host(resolve_planned_character(
            prepared,
            &enemy.payload.character_id,
        )) {
            let giant_sim = SimId::placement(&enemy.id);
            let hands = crate::features::giant_hand_plans(&enemy.id, enemy.aabb);
            let source = room.id.clone();
            let hand_source = source.clone();
            requests.append(&mut giant_cluster_rows(
                giant_sim,
                enemy.clone(),
                crate::features::ActorFaction::Enemy,
                // The host receives the SAME frozen room paths an ordinary
                // authored enemy does; the pre-`e164f22` migration dropped
                // them with `paths: Vec::new()`.
                paths.to_vec(),
                hands,
                SpawnOrigin::Authored {
                    source: source.clone(),
                    instance: enemy.id.clone(),
                },
                move |hand| SpawnOrigin::Authored {
                    source: hand_source.clone(),
                    instance: hand.feature_id.clone(),
                },
            ));
        } else {
            requests.push(ActorConstructionRequest {
                sim_id: SimId::placement(&enemy.id),
                origin: SpawnOrigin::Authored {
                    source: room.id.clone(),
                    instance: enemy.id.clone(),
                },
                parameters: ActorConstructionParams::AuthoredEnemy {
                    authored: enemy.clone(),
                    paths: paths.to_vec(),
                },
                relations: Vec::new(),
            });
        }
    }
    for boss in &room.boss_spawns {
        requests.push(ActorConstructionRequest {
            sim_id: SimId::placement(&boss.id),
            origin: SpawnOrigin::Authored {
                source: room.id.clone(),
                instance: boss.id.clone(),
            },
            parameters: ActorConstructionParams::AuthoredBoss {
                authored: boss.clone(),
            },
            relations: Vec::new(),
        });
    }
    requests
}

/// The shared lowering for a `"giant"`-class host: one `GiantHost` row plus two
/// `GiantHand` rows joined by `ambition.limb` relations. Both the authored-enemy
/// origin ([`authored_giant_requests`]) and the provider-staged origin
/// ([`staged_actor_requests`]) lower through this ONE function, so a giant is the
/// same three-row cluster regardless of where it entered — the property that
/// makes "every plan origin builds a giant the same way" true rather than
/// aspirational. Origins that do not go through the planner at all (summon,
/// encounter, runtime minion, boss) reject giant-class specs during preparation
/// rather than producing a handless host.
#[allow(clippy::too_many_arguments)]
fn giant_cluster_rows(
    host_sim: SimId,
    host_authored: crate::rooms::Authored<crate::rooms::EnemySpawnSpec>,
    faction: crate::features::ActorFaction,
    paths: Vec<(String, ambition_platformer2d_core::KinematicPath)>,
    hands: Vec<crate::features::GiantHandPlan>,
    host_origin: SpawnOrigin,
    mut hand_origin: impl FnMut(&crate::features::GiantHandPlan) -> SpawnOrigin,
) -> Vec<ActorConstructionRequest> {
    let mut rows = vec![ActorConstructionRequest {
        sim_id: host_sim.clone(),
        origin: host_origin,
        parameters: ActorConstructionParams::GiantHost {
            authored: host_authored,
            faction,
            paths,
        },
        relations: Vec::new(),
    }];
    for hand in &hands {
        rows.push(ActorConstructionRequest {
            sim_id: SimId::spawned(&host_sim, hand.ordinal),
            origin: hand_origin(hand),
            parameters: ActorConstructionParams::GiantHand {
                authored: {
                    let mut authored: crate::rooms::Authored<crate::rooms::EnemySpawnSpec> =
                        crate::rooms::Authored::new(
                            hand.feature_id.clone(),
                            "Giant GNU Hand",
                            hand.aabb,
                            crate::rooms::EnemySpawnSpec::new(
                                ambition_entity_catalog::placements::CharacterBrain::Custom(
                                    "giant_gnu_hands".into(),
                                ),
                                // ⭐ **the hand NAMES its character** at
                                // construction now, so its body comes from a
                                // definition like every other creature and the
                                // spec cannot exist without one.
                                "npc_giant_gnu_hands",
                            ),
                        );
                    // A limb is not a combatant: the rider's routed strikes are what hurt, and the
                    // hand itself must never be targeted.
                    authored.payload.disposition =
                        Some(ambition_entity_catalog::placements::SpawnDisposition::Peaceful);
                    authored
                },
            },
            relations: vec![
                ambition_platformer2d_shared_tangle::construction::RelationRequest {
                    to: host_sim.clone(),
                    relation: ActorRelation::Limb {
                        slot: hand.slot,
                        home_offset: hand.home_offset,
                    },
                },
            ],
        });
    }
    rows
}

/// The authored ids this room constructs as `"giant"`-class hosts, so the
/// family loop that still builds ordinary enemies can skip them — a giant is a
/// plan row now, and building it on the loop too would duplicate it.
/// The prepared definition a placement names, if it names one the cast knows.
fn resolve_planned_character<'a>(
    prepared: Option<&'a crate::character_runtime::PreparedCharacterRegistry>,
    character: &ambition_entity_catalog::CharacterId,
) -> Option<&'a crate::character_runtime::PreparedCharacterDefinition> {
    prepared.and_then(|cast| cast.get(character.as_str()))
}

// It filtered a room's enemy spawns down to the limbed hosts, asking the prepared character
// first and the archetype row second. The compiler-backed census (`probe_dead_public_fns.py`, )
// reports ZERO call sites in the workspace or any excluded consumer — it is a projection nobody
// projects, and it kept a `&CharacterRoster` parameter alive for nothing.
//
// ⚠ **read before deleting, as that tool insists**: it pins no invariant. The
// limbed-host QUESTION still has an owner (`features::is_limbed_host`, called
// from the spawn road), so what goes is the unused room-wide roll-up, not the
// rule.

/// Turn a room's FROZEN placement-lowering decisions into construction rows —
/// the Phase-4 migration for the placement family (hazard, interactable/NPC,
/// pickup, chest, breakable, portal).
///
/// Each row carries the `(record, interpreter)` pair the lowering registry
/// resolved at preparation, so commit repeats no lookup; the executor
/// allocates the root and the interpreter populates it (`LoweringCtx::root`).
/// Records that spawn NOTHING today are skipped rather than planned: a `Door`
/// interactable is world-transition data, and the inner Chest/Pickup/Breakable
/// interaction kinds plus an unparseable `Custom` payload have no spawning
/// branch — planning them would turn a long-standing silent no-op into a fatal
/// missing-row verdict. (Upgrading the unparseable-Custom case to a planning
/// ERROR is deliberate future work; this slice is behavior-preserving.)
pub fn placement_requests(
    placements: &crate::world::placements::PlacementLoweringPlan<
        crate::world::placements::ActorPlacementContext,
    >,
    room_id: &str,
    paths: &[(String, ambition_platformer2d_core::KinematicPath)],
) -> Vec<ActorConstructionRequest> {
    use ambition_entity_catalog::placements::{InteractionKindSpec, PlacementSchema};
    let mut requests = Vec::new();
    for (record, lower) in placements.planned() {
        if let PlacementSchema::Interactable(spec) = &record.schema {
            let spawns = match &spec.kind {
                InteractionKindSpec::Npc { .. } => true,
                InteractionKindSpec::Custom(payload) => {
                    ambition_encounter::SwitchActivation::parse_custom(payload).is_some()
                }
                InteractionKindSpec::Door { .. }
                | InteractionKindSpec::Chest
                | InteractionKindSpec::Pickup
                | InteractionKindSpec::Breakable => false,
            };
            if !spawns {
                continue;
            }
        }
        requests.push(ActorConstructionRequest {
            sim_id: SimId::placement(record.id.as_str()),
            origin: SpawnOrigin::Authored {
                source: room_id.to_string(),
                instance: record.id.as_str().to_string(),
            },
            parameters: ActorConstructionParams::Placement {
                record: record.clone(),
                paths: paths.to_vec(),
                lower,
            },
            relations: Vec::new(),
        });
    }
    requests
}

/// Their specs always carried stable authored ids; the entities now wear them. Capability-owned
/// families compose their own typed construction lanes beside this actor lane.
pub fn authored_static_requests(room: &crate::rooms::RoomSpec) -> Vec<ActorConstructionRequest> {
    let mut requests = Vec::new();
    for shrine in &room.shrines {
        requests.push(ActorConstructionRequest {
            sim_id: SimId::placement(&shrine.id),
            origin: SpawnOrigin::Authored {
                source: room.id.clone(),
                instance: shrine.id.clone(),
            },
            parameters: ActorConstructionParams::Shrine {
                spec: shrine.clone(),
            },
            relations: Vec::new(),
        });
    }
    requests
}

/// Fold the room's authored mount links into the request batch as planned
/// `ambition.mount` relations.
pub fn attach_authored_mount_links(
    room: &crate::rooms::RoomSpec,
    requests: &mut Vec<ActorConstructionRequest>,
) -> Result<(), ActorConstructionError> {
    for (rider_id, mount_id) in &room.mount_links {
        for (end, id) in [("mount", mount_id), ("rider", rider_id)] {
            let sim = SimId::placement(id);
            if !requests.iter().any(|request| request.sim_id == sim) {
                return Err(ActorConstructionError::MountLinkNamesNobody {
                    room: room.id.clone(),
                    end,
                    id: id.clone(),
                });
            }
        }
        let rider_sim = SimId::placement(rider_id);
        let rider_row = requests
            .iter_mut()
            .find(|request| request.sim_id == rider_sim)
            .expect("checked above");
        rider_row.relations.push(
            ambition_platformer2d_shared_tangle::construction::RelationRequest {
                to: SimId::placement(mount_id),
                relation: ActorRelation::Mount,
            },
        );
    }
    Ok(())
}

// A parameter kept for symmetry makes every caller thread an authority it does not need, which
// is how a roster reaches code that has no question for it.

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
