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
    RelationCheck, RelationDispatch, RelationKind, RelationOps, SpawnOrigin,
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
/// A `"giant"`-class limbed host — an authored enemy that carries a rig.
pub const RECIPE_GIANT_HOST: &str = "ambition.giant-host";
/// One hand of a giant host.
pub const RECIPE_GIANT_HAND: &str = "ambition.giant-hand";
/// An ordinary authored enemy pulled into the planner because a relation
/// (today: an authored mount link) names it.
pub const RECIPE_AUTHORED_ENEMY: &str = "ambition.authored-enemy";
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

const OWNER: &str = "ambition_actors";
// v2: relation wiring and postconditions changed — the rig became slot-keyed,
// and limb/mount verification now checks home offset, `Mounted`, and mount
// capabilities. Behaviour change under a fixed schema id would be invisible to
// the prepared-content fingerprint, so the id moves with the behaviour.
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
        slot: crate::features::LimbSlot,
        home_offset: ambition_engine_core::Vec2,
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
    /// A `"giant"`-class limbed host: an ordinary authored enemy body plus the
    /// host-side rig state its hands' limb relations attach to. Its two hands are
    /// separate [`Self::GiantHand`] rows, joined by `ambition.limb` relations —
    /// they used to be minted inside the enemy spawn helper as authoritative
    /// roots no plan named (the last legacy family).
    GiantHost {
        authored: crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
        faction: crate::features::ActorFaction,
        paths: Vec<(String, ambition_engine_core::KinematicPath)>,
    },
    /// One hand of a giant host. The body is built here; its `Limb` component and
    /// the host's rig entry are installed by the `ambition.limb` relation.
    GiantHand {
        authored: crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    },
    /// An ordinary authored enemy, planned because an authored mount link names
    /// it as rider or mount. Built by the SAME populate function the enemy
    /// family loop calls (`spawn_enemy_with_faction_into`, faction `Enemy`),
    /// so being planned changes WHO wires its relations, not what it is. The
    /// rest of the enemy family stays on the loop until Phase 4 migrates it.
    AuthoredEnemy {
        authored: crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
        paths: Vec<(String, ambition_engine_core::KinematicPath)>,
    },
    /// An authored boss, planned because an authored mount link names it as the
    /// rider (the gnu_ton_rider pattern). Built by the same populate function
    /// the boss loop calls, with default overrides — identical body, planned
    /// identity.
    AuthoredBoss {
        authored: crate::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
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
                format!("minion {} {}", minion.feature_id, minion.archetype_id)
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

/// Why an actor-domain request could not be turned into a planned row. These
/// are the failures that used to be silent skips at spawn time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActorConstructionError {
    UnknownHeldItem {
        authored_id: String,
        item: String,
    },
    /// One limb declares two hosts. A limb is a part OF a body; two hosts is not
    /// a configuration with a degraded meaning, it is a contradiction.
    LimbHasTwoHosts {
        limb: SimId,
        hosts: Vec<SimId>,
    },
    /// Two limbs claim the same slot of the same host. The rig is keyed by slot,
    /// so committing this would silently drop one of them.
    LimbSlotTaken {
        host: SimId,
        slot: &'static str,
        limbs: Vec<SimId>,
    },
    /// One rider declares two mounts.
    RiderOnTwoMounts {
        rider: SimId,
        mounts: Vec<SimId>,
    },
    /// Two riders claim the same mount. `MountSlot` holds ONE rider, so
    /// committing this would leave whichever lost pointing at a mount that
    /// points at the other.
    MountHasTwoRiders {
        mount: SimId,
        riders: Vec<SimId>,
    },
    /// An entity declares itself its own mount.
    SelfMount {
        rider: SimId,
    },
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
}

impl std::fmt::Display for ActorConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownHeldItem { authored_id, item } => write!(
                f,
                "authored ground item `{authored_id}` names held item `{item}`, which no held-item \
                 registry entry provides"
            ),
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
        &ctx.services.context.roster,
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
        &ctx.services.context.roster,
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
        &ctx.services.context.roster,
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
        &crate::features::BossOverrides::default(),
    );
}

// ── Relations ────────────────────────────────────────────────────────────────

/// Stable dump/diagnostic key for a limb slot. Byte-stable because it reaches
/// the plan dump, and matching the `snake_case` the route authoring already uses.
fn limb_slot_key(slot: crate::features::LimbSlot) -> &'static str {
    match slot {
        crate::features::LimbSlot::HandLeft => "hand_left",
        crate::features::LimbSlot::HandRight => "hand_right",
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
    ctx.commands.entity(limb).insert(crate::features::Limb {
        of: host,
        slot,
        home_offset,
    });
    ctx.commands.queue(move |world: &mut World| {
        let Ok(mut host_ref) = world.get_entity_mut(host) else {
            return;
        };
        if let Some(mut rig) = host_ref.get_mut::<crate::features::LimbRig>() {
            rig.limbs.insert(slot, limb);
        } else {
            host_ref.insert(crate::features::LimbRig::from_pairs([(slot, limb)]));
        }
    });
}

/// Prove the limb link landed on BOTH sides, with the slot AND the home offset
/// the plan named.
///
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
    let Some(attached) = world.get::<crate::features::Limb>(limb) else {
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
    let Some(rig) = world.get::<crate::features::LimbRig>(host) else {
        return RelationCheck::ReverseMismatch { found: None };
    };
    // The rig must file this limb under the planned slot, and nowhere else. A
    // slot-keyed map cannot hold the same key twice, but it CAN hold one limb
    // under two different slots — which drives it from two intent streams.
    let occupants: Vec<crate::features::LimbSlot> = rig
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

/// The exact rig a plan describes for one host: every limb relation naming it,
/// as slot → limb identity.
///
/// Separate from [`verify_limb`] because it is a different question. That one
/// asks "did MY relation land"; a host whose rig gained an extra limb from
/// somewhere else answers yes to every such question while carrying a rig the
/// plan never described. Callers compare this against the committed
/// [`crate::features::LimbRig`] to check composition rather than membership.
pub fn planned_rig_for_host(
    plan: &ActorConstructionPlan,
    host: &SimId,
) -> std::collections::BTreeMap<crate::features::LimbSlot, SimId> {
    plan.relations()
        .iter()
        .filter(|relation| relation.to() == host)
        .filter_map(|relation| match relation.relation() {
            ActorRelation::Limb { slot, .. } => Some((*slot, relation.from().clone())),
            ActorRelation::Grudge | ActorRelation::Mount => None,
        })
        .collect()
}

/// Compare every planned row's committed rig against the EXACT composition the
/// plan described for it — the composition question [`verify_limb`] cannot ask.
///
/// The per-relation postcondition pass proves each planned limb landed; it is
/// structurally blind to a rig that ALSO holds something the plan never named
/// (an extra slot, a duplicated limb entity, a second intent stream's leftover),
/// because no planned relation points at the surplus. This pass derives the
/// expected slot→identity map from the plan ([`planned_rig_for_host`]), resolves
/// identities to entities through the receipt (which pins generation, not just
/// index), and demands slot-for-slot equality both ways:
///
/// - every planned slot occupied, by exactly the planned limb's committed entity;
/// - no slot the plan did not describe;
/// - no limb entity appearing in two slots;
/// - each occupant's forward [`crate::features::Limb`] agreeing on host AND slot
///   (which catches a stale host generation: `Limb.of` carries the full
///   `Entity`, so an old generation compares unequal);
/// - a row with no planned limbs carrying no rig entries at all.
///
/// Runs over ALL planned rows, not just giant hosts, so an unplanned rig
/// appearing on any committed row is a finding. Read-only; violations surface as
/// [`RosterViolation::RigComposition`], which is fatal.
pub fn verify_rig_composition(
    plan: &ActorConstructionPlan,
    receipt: &ambition_platformer_primitives::construction::ConstructionReceipt,
    world: &World,
) -> Vec<ambition_platformer_primitives::construction::RosterViolation> {
    use ambition_platformer_primitives::construction::RosterViolation;
    let mut violations = Vec::new();
    for row in plan.entities() {
        let host_sim = row.sim_id();
        let planned = planned_rig_for_host(plan, host_sim);
        let Some(host_entity) = receipt.entity(host_sim) else {
            // Never committed: the generic roster pass already reports it, and
            // there is no world-side rig to compare.
            continue;
        };
        let committed: std::collections::BTreeMap<crate::features::LimbSlot, Entity> = world
            .get::<crate::features::LimbRig>(host_entity)
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
        let slots: std::collections::BTreeSet<crate::features::LimbSlot> =
            planned.keys().chain(committed.keys()).copied().collect();
        for slot in slots {
            match (planned.get(&slot), committed.get(&slot)) {
                (Some(limb_sim), Some(&occupant)) => {
                    match receipt.entity(limb_sim) {
                        Some(expected) if expected == occupant => {
                            // Right occupant; now the forward half must agree.
                            match world.get::<crate::features::Limb>(occupant) {
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
        let mut seen: std::collections::BTreeMap<Entity, crate::features::LimbSlot> =
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

/// Prove the mount link landed on BOTH sides.
///
/// The reverse check is the one that matters here, because the half-write it
/// catches is a defect that exists in the tree today: `attach_mount_role` never
/// inserts `MountSlot`, and `reconcile_autonomous_actors` re-establishes the
/// link with `world.get_mut::<MountSlot>(..)` — a mutation that silently does
/// nothing when the component is absent — while inserting `RidingOn`
/// unconditionally. That leaves a rider pointing at a mount that does not point
/// back, and `steer_mount_from_rider` queries `With<MountSlot>`, so the mount
/// stops obeying while every rider-side assertion still passes.
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

/// Prove the grudge landed, by reading the component rather than trusting that
/// [`wire_grudge`] was called.
///
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
    registry.try_register_recipe(recipe_giant_host(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_giant_hand(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_authored_enemy(), OWNER, "authored-room", SCHEMA)?;
    registry.try_register_recipe(recipe_authored_boss(), OWNER, "authored-room", SCHEMA)?;
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
    roster: &crate::features::CharacterRoster,
    bosses: &BossCatalog,
) -> PlannedMountCapabilities {
    match parameters {
        // A pickup is neither rideable nor a pilot.
        ActorConstructionParams::GroundItem { .. } => PlannedMountCapabilities::default(),
        ActorConstructionParams::StagedActor(request) => match &request.kind {
            SpawnActorKind::Enemy { brain } => {
                let spec = roster.spec_for_brain(brain);
                PlannedMountCapabilities {
                    mount_class: spec.mount_class.clone(),
                    pilots: spec.pilotable_mount_classes.clone(),
                }
            }
            // A boss takes `CanPilot` from its behaviour profile and is never
            // itself a mount — `spawn_boss` installs no `Mountable`. Resolved
            // through the SAME pair `BossClusterScratch::new` uses, so the
            // preflight reads the profile the commit will read.
            SpawnActorKind::Boss { brain, .. } => PlannedMountCapabilities {
                mount_class: None,
                pilots: crate::boss_encounter::behavior::BossBehaviorProfile::for_authored_boss(
                    bosses,
                    &crate::boss_encounter::behavior::canonical_boss_id_from(&request.name, brain),
                )
                .pilotable_mount_classes
                .clone(),
            },
        },
        ActorConstructionParams::SummonedMinion(minion) => {
            let spec = roster.spec_for_brain(
                &ambition_entity_catalog::placements::CharacterBrain::Custom(
                    minion.archetype_id.clone(),
                ),
            );
            PlannedMountCapabilities {
                mount_class: spec.mount_class.clone(),
                pilots: spec.pilotable_mount_classes.clone(),
            }
        }
        // A giant host is a mount (its archetype carries `mount_class`); its hands
        // are neither mount nor pilot.
        ActorConstructionParams::GiantHost { authored, .. } => {
            let spec = roster.spec_for_brain(&authored.payload);
            PlannedMountCapabilities {
                mount_class: spec.mount_class.clone(),
                pilots: spec.pilotable_mount_classes.clone(),
            }
        }
        ActorConstructionParams::GiantHand { .. } => PlannedMountCapabilities::default(),
        ActorConstructionParams::AuthoredEnemy { authored, .. } => {
            let spec = roster.spec_for_brain(&authored.payload);
            PlannedMountCapabilities {
                mount_class: spec.mount_class.clone(),
                pilots: spec.pilotable_mount_classes.clone(),
            }
        }
        // Same profile resolution as the staged boss arm above — and never a
        // mount: `spawn_boss` installs no `Mountable`.
        ActorConstructionParams::AuthoredBoss { authored } => PlannedMountCapabilities {
            mount_class: None,
            pilots: crate::boss_encounter::behavior::BossBehaviorProfile::for_authored_boss(
                bosses,
                &crate::boss_encounter::behavior::canonical_boss_id_from(
                    &authored.name,
                    &authored.payload,
                ),
            )
            .pilotable_mount_classes
            .clone(),
        },
    }
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
    }
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
    roster: &crate::features::CharacterRoster,
    bosses: &BossCatalog,
) -> Result<(), ActorConstructionError> {
    use std::collections::BTreeMap;

    let family: BTreeMap<&SimId, &ActorConstructionParams> = requests
        .iter()
        .map(|request| (&request.sim_id, &request.parameters))
        .collect();

    // Ordered accumulators: a diagnostic that names "the two limbs in this slot"
    // must name them in the same order every run.
    let mut hosts_of_limb: BTreeMap<&SimId, Vec<SimId>> = BTreeMap::new();
    let mut limbs_in_slot: BTreeMap<(&SimId, crate::features::LimbSlot), Vec<SimId>> =
        BTreeMap::new();
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

    // Family legality and pilot/mount compatibility. Both ends must be rows —
    // the generic planner guarantees that — so a missing entry here is a planner
    // bug rather than a content error, and is treated as "cannot hold that end"
    // rather than silently skipped.
    for (rider, mounts) in &mounts_of_rider {
        let Some(mount) = mounts.first() else {
            continue;
        };
        let rider_params = family.get(*rider).copied();
        let mount_params = family.get(mount).copied();
        let (Some(rider_params), Some(mount_params)) = (rider_params, mount_params) else {
            continue;
        };
        let rider_caps = mount_capabilities_of(rider_params, roster, bosses);
        let mount_caps = mount_capabilities_of(mount_params, roster, bosses);
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
    roster: &crate::features::CharacterRoster,
) -> Vec<ActorConstructionRequest> {
    let mut rows = Vec::new();
    for request in requests {
        let host_sim = SimId::placement(&request.id);
        let grudges: Vec<_> = request
            .grudge_against
            .iter()
            .map(
                |foe| ambition_platformer_primitives::construction::RelationRequest {
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
        if let SpawnActorKind::Enemy { brain } = &request.kind {
            let spec = roster.spec_for_brain(brain);
            if crate::features::spec_is_limbed_host(&spec) {
                let aabb = ambition_engine_core::Aabb::new(request.pos, request.half_size);
                let host_authored = crate::rooms::Authored::new(
                    request.id.clone(),
                    request.name.clone(),
                    aabb,
                    brain.clone(),
                );
                let hands = crate::features::giant_hand_plans(&request.id, aabb, &spec);
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

/// Turn a room's authored `"giant"`-class enemies into construction rows: one
/// host row plus two hand rows each, joined by `ambition.limb` relations.
///
/// **This is the migration that empties `KNOWN_LEGACY_FAMILIES`.** The hands used
/// to be minted inside the enemy spawn helper as authoritative roots no plan
/// named. Here they are prepared with the host, before anything spawns, so the
/// reconstruction closure of the host or either hand includes all three and the
/// boundary verifier sees a rig it planned rather than a legacy warning.
///
/// The hand identities are unchanged — `SimId::spawned(giant, ordinal)`, with the
/// feature id a pure function of the giant's authored id — so a snapshot taken
/// before this migration still restores. `roster` resolves each enemy's
/// archetype; only limbed hosts (`spec_is_limbed_host`) produce rows here.
pub fn authored_giant_requests(
    room: &crate::rooms::RoomSpec,
    roster: &crate::features::CharacterRoster,
    paths: &[(String, ambition_engine_core::KinematicPath)],
) -> Vec<ActorConstructionRequest> {
    let mut requests = Vec::new();
    for enemy in &room.enemy_spawns {
        let spec = roster.spec_for_brain(&enemy.payload);
        if !crate::features::spec_is_limbed_host(&spec) {
            continue;
        }
        let giant_sim = SimId::placement(&enemy.id);
        let hands = crate::features::giant_hand_plans(&enemy.id, enemy.aabb, &spec);
        let source = room.id.clone();
        let hand_source = source.clone();
        requests.append(&mut giant_cluster_rows(
            giant_sim,
            enemy.clone(),
            crate::features::ActorFaction::Enemy,
            // The host receives the SAME frozen room paths an ordinary authored
            // enemy does (`spawn_enemy(.., &self.paths)`); the pre-`e164f22`
            // migration dropped them with `paths: Vec::new()`.
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
///
/// The hand identities are `SimId::spawned(host, ordinal)`, with the feature id a
/// pure function of the host's authored id, so a snapshot taken before the
/// explicit-hand migration still restores.
#[allow(clippy::too_many_arguments)]
fn giant_cluster_rows(
    host_sim: SimId,
    host_authored: crate::rooms::Authored<ambition_entity_catalog::placements::CharacterBrain>,
    faction: crate::features::ActorFaction,
    paths: Vec<(String, ambition_engine_core::KinematicPath)>,
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
                authored: crate::rooms::Authored {
                    id: hand.feature_id.clone(),
                    name: "Giant GNU Hand".to_string(),
                    aabb: hand.aabb,
                    payload: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "giant_gnu_hands".into(),
                    ),
                },
            },
            relations: vec![
                ambition_platformer_primitives::construction::RelationRequest {
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
pub fn planned_giant_host_ids(
    room: &crate::rooms::RoomSpec,
    roster: &crate::features::CharacterRoster,
) -> std::collections::BTreeSet<String> {
    room.enemy_spawns
        .iter()
        .filter(|enemy| {
            crate::features::spec_is_limbed_host(&roster.spec_for_brain(&enemy.payload))
        })
        .map(|enemy| enemy.id.clone())
        .collect()
}

/// Fold the room's authored mount links into the request batch as planned
/// `ambition.mount` relations, pulling each named actor into the planner.
///
/// **This is the migration that deletes `PendingMountLinks`.** The live
/// resolver matched `(rider_id, mount_id)` pairs by `FeatureId` a frame after
/// spawn, retried missing actors forever, and DROPPED an incompatible pair
/// with no diagnostic. Here every named actor becomes a plan row — an
/// [`ActorConstructionParams::AuthoredEnemy`] or
/// [`ActorConstructionParams::AuthoredBoss`], built by the SAME populate
/// functions the family loops call — the rider row declares the relation, the
/// engine-owned `wire_mount` installs BOTH ends at commit, and `verify_mount`
/// plus the roster verifier prove it landed. A link naming a `"giant"`-class
/// enemy rides on the giant host row [`authored_giant_requests`] already
/// planned (the request batch is shared, so the endpoint resolves), and the
/// gnu_ton_rider boss becomes a planned boss row with its `CanPilot` profile.
///
/// Mutates `requests` in place: existing rows gain the relation, missing rows
/// are appended. A link naming nobody fails the room while it is whole.
pub fn attach_authored_mount_links(
    room: &crate::rooms::RoomSpec,
    roster: &crate::features::CharacterRoster,
    paths: &[(String, ambition_engine_core::KinematicPath)],
    requests: &mut Vec<ActorConstructionRequest>,
) -> Result<(), ActorConstructionError> {
    for (rider_id, mount_id) in &room.mount_links {
        for (end, id) in [("mount", mount_id), ("rider", rider_id)] {
            let sim = SimId::placement(id);
            if requests.iter().any(|request| request.sim_id == sim) {
                continue;
            }
            if let Some(enemy) = room.enemy_spawns.iter().find(|enemy| &enemy.id == id) {
                requests.push(ActorConstructionRequest {
                    sim_id: sim,
                    origin: SpawnOrigin::Authored {
                        source: room.id.clone(),
                        instance: enemy.id.clone(),
                    },
                    parameters: ActorConstructionParams::AuthoredEnemy {
                        authored: enemy.clone(),
                        // The same frozen room paths the enemy loop passes.
                        paths: paths.to_vec(),
                    },
                    relations: Vec::new(),
                });
            } else if let Some(boss) = room.boss_spawns.iter().find(|boss| &boss.id == id) {
                requests.push(ActorConstructionRequest {
                    sim_id: sim,
                    origin: SpawnOrigin::Authored {
                        source: room.id.clone(),
                        instance: boss.id.clone(),
                    },
                    parameters: ActorConstructionParams::AuthoredBoss {
                        authored: boss.clone(),
                    },
                    relations: Vec::new(),
                });
            } else {
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
            .expect("the loop above guarantees the rider row exists");
        rider_row.relations.push(
            ambition_platformer_primitives::construction::RelationRequest {
                to: SimId::placement(mount_id),
                relation: ActorRelation::Mount,
            },
        );
    }
    Ok(())
}

/// The authored ENEMY ids this room constructs as plan rows — the giants plus
/// every enemy an authored mount link pulled in — so the enemy family loop
/// skips them. Mirrors [`planned_authored_boss_ids`] for the boss loop.
pub fn planned_authored_enemy_ids(
    room: &crate::rooms::RoomSpec,
    roster: &crate::features::CharacterRoster,
) -> std::collections::BTreeSet<String> {
    let mut ids = planned_giant_host_ids(room, roster);
    for (rider_id, mount_id) in &room.mount_links {
        for id in [rider_id, mount_id] {
            if room.enemy_spawns.iter().any(|enemy| &enemy.id == id) {
                ids.insert(id.clone());
            }
        }
    }
    ids
}

/// The authored BOSS ids this room constructs as plan rows (mount-link riders,
/// e.g. gnu_ton_rider), so the boss loop skips them.
pub fn planned_authored_boss_ids(
    room: &crate::rooms::RoomSpec,
) -> std::collections::BTreeSet<String> {
    let mut ids = std::collections::BTreeSet::new();
    for (rider_id, mount_id) in &room.mount_links {
        for id in [rider_id, mount_id] {
            if room.boss_spawns.iter().any(|boss| &boss.id == id) {
                ids.insert(id.clone());
            }
        }
    }
    ids
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
