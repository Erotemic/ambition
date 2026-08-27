//! Actor construction planner for authored, provider-staged, and runtime-dynamic origins.
//! Requests are preflighted before entities are spawned; relations and external references are
//! validated against the plan and live world. Optional capabilities may contribute their own
//! closed [`ConstructionDomain`] through construction federation.

use ambition_boss_encounter::behavior::BossBehaviorProfileExt;
use ambition_characters::actor::limb::{Limb, LimbRig, LimbSlot};
use ambition_platformer2d_shared_tangle::construction::{
    ConstructionDomain, ConstructionExecCtx, ConstructionPlan, ConstructionRegistrationError,
    ConstructionRegistry, ConstructionRequest, ConstructionRoot, RecipeDispatch, RecipeId,
    RelationCheck, RelationDispatch, RelationKind, RelationOps, SpawnOrigin,
};
use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
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
/// A driven limb belonging to a host body's rig. Bidirectional: `Limb` on
/// the limb, an entry in the host's `LimbRig` going back.
pub const RELATION_LIMB: &str = "ambition.limb";
/// A rider seated on a mount. Bidirectional: `RidingOn` on the rider,
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

/// What one declared actor relation IS — the kind and everything the pairing
/// carries, in one value.
///
/// `Limb` carries the slot and the home offset because both are stated
/// relative to the HOST. `LimbSlot::HAND_LEFT` is meaningless without saying
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
        spec: ambition_platformer2d_world::rooms::GroundItemSpec,
        held: ambition_characters::brain::HeldItemSpec,
    },
    StagedActor(SpawnActorRequest),
    SummonedMinion(SummonedMinionParams),
    /// A `"giant"`-class limbed host: an ordinary authored enemy body plus the host-side rig state
    /// its hands' limb relations attach to.
    GiantHost {
        authored: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec>,
        faction: crate::features::ActorFaction,
        paths: Vec<(String, ambition_platformer2d_core::KinematicPath)>,
    },
    /// One hand of a giant host. The body is built here; its `Limb` component and
    /// the host's rig entry are installed by the `ambition.limb` relation.
    GiantHand {
        authored: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec>,
    },
    /// An ordinary authored enemy. Every authored enemy is a plan row, built by
    /// the same populate function the former family loop used.
    AuthoredEnemy {
        authored: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec>,
        paths: Vec<(String, ambition_platformer2d_core::KinematicPath)>,
    },
    /// An authored boss. Every authored boss is a plan row, built by the same
    /// populate function the former boss loop used, with default overrides.
    AuthoredBoss {
        authored: ambition_platformer2d_world::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
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
        spec: ambition_platformer2d_world::rooms::ShrineSpec,
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
    /// Health for this occurrence, overriding the character's authored vitals.
    /// See `ambition_vfx::SummonSpec::health`.
    pub health: Option<u32>,
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
        slot: String,
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
    // ⭐ INLINE, and that is the whole point of the move. This was one call into
    // `features/ecs/spawn_static.rs` — a twenty-line component insert that lived
    // under "spawning static things" by TOPIC while naming `items` and `shrine`,
    // never `spawn` — and it was one of only TWO production references
    // `construction` made into `features/ecs` at all. Both had this file as their
    // only caller, so the domain edge existed to serve nobody.
    ctx.commands.insert_room_in_session(
        ctx.session,
        root.entity(),
        (
            bevy::prelude::Name::new(format!("Ground item: {}", spec.name)),
            crate::items::pickup::GroundItem {
                spec: held.clone(),
                pos: spec.pos,
                vel: ambition_platformer2d_core::Vec2::ZERO,
                half_extent: spec.half_extent,
            },
            // ⭐⭐ AN AUTHORED PLACEMENT IS ALREADY AT REST, and saying so is what
            // lets everything else fall. An author put this object where it is;
            // it is not necessarily standing on collision geometry the physics
            // predicate can see, and stepping it drops the whole authored
            // population out of the world (measured: a room rebuild came back
            // with zero ground items where it had fifteen).
            crate::items::pickup::SettledItem,
        ),
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
        //  the cast this construction context has carried all along — the
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
        minion.health,
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
    // ⭐ THE HOST IS AN ORDINARY ENEMY BODY PLUS THE LIMB ROUTING STATE, and
    // saying so here is the point of the move. This was `populate_giant_host_into`
    // in `features/ecs/spawn_actors.rs` — a five-line wrapper whose only caller in
    // the tree was this function, holding GIANT-CREATURE construction knowledge
    // inside the generic spawn file. The shared call it wraps stays where it is;
    // what is gone is a name crossing the boundary to say something only
    // construction cares about.
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
        *faction,
    );
    ctx.commands.entity(root.entity()).insert((
        ambition_characters::actor::limb::LimbIntents::default(),
        ambition_characters::actor::limb::LimbRouteState::default(),
    ));
}

fn construct_giant_hand(
    parameters: &ActorConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let ActorConstructionParams::GiantHand { authored } = parameters else {
        unreachable!("dispatch pairs this fn with GiantHand parameters")
    };
    //  THE SAME ROAD THE HOST TAKES, and that is load-bearing history: the
    // hand used to be built from an ARCHETYPE row while the giant beside it was
    // built from its character, so two limbs of one creature came down two
    // construction paths. Non-hostile by construction — the limb fan-out is its
    // only driver, and targeting must ignore it. The `Limb` component and the
    // host's rig entry come from the `ambition.limb` relation, not from here.
    crate::features::spawn_enemy_with_faction_into(
        ctx.commands,
        &ctx.services.context.characters,
        &ctx.services.context.sheets,
        &ctx.services.context.prepared,
        &ctx.services.context.brain_profiles,
        ctx.session,
        root.entity(),
        authored,
        &[],
        crate::features::ActorFaction::Enemy,
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
    // The other half of the same move — see `construct_authored_ground_item`.
    ctx.commands.insert_room_in_session(
        ctx.session,
        root.entity(),
        (
            bevy::prelude::Name::new(format!("Heal/save shrine: {}", spec.name)),
            crate::shrine::HealShrine {
                pos: spec.pos,
                half_extent: spec.half_extent,
            },
        ),
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

/// Stable dump/diagnostic key for a limb slot — now just the authored name,
/// since `LimbSlot` IS that name. The hand-written match this replaced had to
/// gain an arm per anatomy, which is the cost an open slot id removes.
fn limb_slot_key(slot: LimbSlot) -> String {
    slot.as_str().to_owned()
}

/// Wire a limb to its host: `Limb` on the limb, an entry in the host's
/// `LimbRig` going back. One function writes both ends.
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
/// on the mount going back. One function writes both ends.
fn wire_mount(rider: Entity, mount: Entity, _relation: &ActorRelation, ctx: &mut Ctx<'_, '_, '_>) {
    ctx.commands.entity(rider).insert((
        ambition_mount::RidingOn { mount },
        ambition_mount::Mounted,
    ));
    ctx.commands
        .entity(mount)
        .insert(ambition_mount::MountSlot { rider: Some(rider) });
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
    let Some(riding) = world.get::<ambition_mount::RidingOn>(rider) else {
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
    if world.get::<ambition_mount::Mounted>(rider).is_none() {
        return RelationCheck::MissingCapability {
            component: "Mounted",
        };
    }
    // Both ends must still carry the capabilities the preflight approved them
    // on. A recipe that stripped `Mountable` leaves a link whose class nothing
    // can re-check, and `steer_mount_from_rider` reads `Mountable` to route.
    let Some(mountable) = world.get::<ambition_mount::Mountable>(mount) else {
        return RelationCheck::MissingCapability {
            component: "Mountable",
        };
    };
    match world.get::<ambition_mount::CanPilot>(rider) {
        Some(pilot) if pilot.can_pilot(&mountable.class) => {}
        Some(_) => {
            return RelationCheck::PayloadMismatch {
                field: "mount_class",
            };
        }
        None => {
            return RelationCheck::MissingCapability {
                component: "CanPilot",
            };
        }
    }
    match world
        .get::<ambition_mount::MountSlot>(mount)
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
/// This domain is CLOSED. `ActorConstructionParams` is a closed enum and
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
/// read when they install [`ambition_mount::Mountable`] /
/// [`ambition_mount::CanPilot`], so a preflight decision here predicts the
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
    // The prepared cast, when the caller has one. A placement that names a
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
            //  THE CHARACTER FIRST, exactly like the authored arm below.
            //
            //  the same call the authored road makes, so the two roads cannot
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
        //  a summoned minion names its body by STRING, and a boss casting a
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

/// What a placement can ride and be ridden as — the CHARACTER's answer, and
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

/// Which planned rows build their body FROM a character, and what each one
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
/// Prove every planned body can actually be built, before anything is
/// mutated.
///
///  THIS IS THE HALF AC6 LEFT LATE. Deleting the enemy-archetype ontology made an
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
///  it does not restore a fallback and must not grow into one. The three
/// refusals below are the three ways a body cannot be built; each names the
/// placement and the character so the diagnostic is actionable, and the world is
/// whole when it is reported.
///
/// an absent registry is an EMPTY cast, not an exemption. `prepared: None` becomes a default
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

/// Validate actor-domain relation semantics before spawning any entity.
///
/// Reject conflicting limb slots/hosts, conflicting riders/mounts, self-mounts,
/// invalid endpoint families, and incompatible pilot/mount classes.
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

/// Build this record somewhere other than where it says.
///
/// The world remembers that the occurrence this record minted was carried
/// across the room and put down; rebuilding the room owes it back at the
/// position it was left, with the record's own identity. Only the position
/// moves: the recipe, the identity and the provenance are the record's, which
/// is what makes the result the SAME occurrence rather than a copy.
///
///  answers FALSE for a family that has no position of its own, rather
/// than guessing one or silently ignoring the request. A `Placed` row can only
/// be written by a producer that read a position off a live occurrence, so a
/// false here means the ledger and the plan disagree about what kind of thing an
/// identity names — worth a caller's warning, never a silent authoring at the
/// wrong coordinates.
///
///  a free function because `ActorConstructionRequest` is an alias for a
/// generic in another crate, so it cannot carry inherent methods here.
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

/// Return authored requests that may need reconstruction outside their source room.
///
/// A family belongs here only if occurrence state can record a placed position
/// and [`relocate_request`] can rebuild that request there. The exhaustive
/// `RoomSpec` destructure forces new authored families to be classified.
pub fn reinstatable_authored_requests(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Result<Vec<ActorConstructionRequest>, ActorConstructionError> {
    // Exhaustive so every new authored family must be classified as
    // reinstatable or fixed to its source room.
    let ambition_platformer2d_world::rooms::RoomSpec {
        // ── the family this function offers ──────────────────────────────
        ground_items: _,

        // Potentially portable families with no `Placed` producer/rebuilder.
        portal_gun_spawns: _,
        enemy_spawns: _,
        boss_spawns: _,
        placements: _,

        // Fixed room geometry, presentation, graph, and trigger content.
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

/// Find the authored construction request that minted an occurrence.
///
/// This supports checkpoint reconstruction for occurrences not resident in a
/// room. It shares [`reinstatable_authored_requests`] with room reconstruction,
/// skips rooms whose authored records cannot resolve, and returns `None` when
/// the source record no longer exists.
pub fn authored_occurrence_request(
    world: &[ambition_platformer2d_world::rooms::RoomSpec],
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
    room: &ambition_platformer2d_world::rooms::RoomSpec,
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
    // The prepared cast, when the caller has one. Planning asks the
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
            //  the CHARACTER decides, and it is the only thing that does.
            // This also asked the roster, whose lookup could not fail — so an
            // unresolvable key answered the `combatant` row, which is not a
            // limbed host, so the two agreed by luck rather than by design.
            if crate::features::is_limbed_host(resolve_planned_character(prepared, character)) {
                let aabb = ambition_platformer2d_core::Aabb::new(request.pos, request.half_size);
                // Invisible while an archetype row could answer for the brain key; a refusal the
                // moment they went (AC6).
                let host_payload =
                    ambition_platformer2d_world::rooms::EnemySpawnSpec::new(brain.clone(), character.clone());
                let host_authored = ambition_platformer2d_world::rooms::Authored::new(
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
    room: &ambition_platformer2d_world::rooms::RoomSpec,
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
    host_authored: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec>,
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
                    let mut authored: ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::EnemySpawnSpec> =
                        ambition_platformer2d_world::rooms::Authored::new(
                            hand.feature_id.clone(),
                            "Giant GNU Hand",
                            hand.aabb,
                            ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                                ambition_entity_catalog::placements::CharacterBrain::Custom(
                                    "giant_gnu_hands".into(),
                                ),
                                //  the hand NAMES its character at
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
pub fn authored_static_requests(room: &ambition_platformer2d_world::rooms::RoomSpec) -> Vec<ActorConstructionRequest> {
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
    room: &ambition_platformer2d_world::rooms::RoomSpec,
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
