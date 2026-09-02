//! ECS-feature spawn facade.
//!
//! Room-level orchestration and public dynamic-mob entry points stay here, while
//! the concrete family-specific spawn helpers live in smaller sibling modules.
//! This keeps the active ECS path readable without changing the entity shapes
//! or scheduling surfaces that callers use.

use super::spawn_actors::EncounterMobSeed;
use ambition_boss_encounter::BossCatalog;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use bevy::prelude::Commands;
use std::collections::BTreeSet;

mod capability_lanes;
mod gravity_construction;
#[cfg(feature = "portal")]
mod portal_construction;

mod character_spawn_plan;
pub(crate) use character_spawn_plan::{
    report_unprepared_character, CharacterSpawnPlan, SpawnContext,
};

mod content_staging;
pub use content_staging::{
    RoomContentStagingError, RoomContentStagingRegistrationError, RoomContentStagingRegistry,
};

pub(crate) use super::spawn_actors::{spawn_runtime_minion, spawn_runtime_minion_into};

/// Spawn ECS-native feature entities for every authored static
/// feature in a room. One loop per family.

/// A room's authored paths under every spelling they answer to, for the
/// lowering roads that resolve a path reference by string.
///
/// It delegates to `kinematic_path_lookup` now: there is ONE alias set, and a second one cannot
/// drift back in. See that function for the shipped patrol the drift had standing still.
///
/// Lives spawn-side: `RoomSpec` is world-IR vocabulary the combat kit must not
/// name (E2).
pub(crate) fn room_spec_paths(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Vec<(String, ambition_platformer2d_core::KinematicPath)> {
    ambition_platformer2d_world::rooms::kinematic_path_lookup(&room.kinematic_paths)
}

/// A mutation-free room feature construction failure.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomFeatureConstructionError {
    Placement(ambition_platformer2d_world::placements::PlacementLoweringError),
    ContentStaging(RoomContentStagingError),
    DuplicateAuthoritativeId {
        room: String,
        id: String,
    },
    /// One typed construction lane could not be resolved into a valid plan.
    Construction(ambition_platformer2d_shared_tangle::construction::ConstructionError),
    /// Actor-lane parameters could not be resolved from content — for example
    /// an authored ground item naming a held item no registry provides.
    ActorConstruction(crate::construction::ActorConstructionError),
}

impl std::fmt::Display for RoomFeatureConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Placement(error) => error.fmt(f),
            Self::ContentStaging(error) => error.fmt(f),
            Self::DuplicateAuthoritativeId { room, id } => write!(
                f,
                "room `{room}` constructs authoritative id `{id}` more than once",
            ),
            Self::Construction(error) => error.fmt(f),
            Self::ActorConstruction(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for RoomFeatureConstructionError {}

/// The complete feature-side artifact prepared before a room mutation begins.
///
/// Interpreter lookup, path flattening, content-stager execution, roster
/// validation, and catalog selection all happen here. Execution only applies
/// these frozen decisions, so startup, reset, transition, hot reload, and
/// restore cannot drift into different room-construction behavior.
#[derive(Clone)]
pub struct RoomFeatureConstructionPlan {
    room: ambition_platformer2d_world::rooms::RoomSpec,
    content_requests: Vec<super::spawn_actors::SpawnActorRequest>,
    /// The primary actor-domain construction lane. Every actor-owned
    /// authoritative family is planned here; optional capabilities compose
    /// separate typed lanes beside it instead of entering this domain enum.
    construction: crate::construction::ActorConstructionPlan,
    /// The capability lanes, as ONE composed value. Each is still
    /// independently typed, planned, committed and verified; what this field
    /// removes is the room plan reimplementing all six of those operations once
    /// per family. See [`capability_lanes::CapabilityLanes`] — every operation
    /// there destructures exhaustively, so a third lane is a compile error at
    /// each step it has to join.
    capability_lanes: capability_lanes::CapabilityLanes,
    /// The frozen catalogs this plan reads — character catalog, hostile roster,
    /// boss profiles. THE copy: actor recipes read it through
    /// `ConstructionExecCtx`, so a cached plan holds one coherent snapshot.
    construction_services: crate::construction::ActorConstructionServices,
    expected_authoritative_ids: BTreeSet<String>,
    /// What this room POINTS AT and did not find. Empty for a clean room.
    ///
    /// Carried on the plan rather than returned as an error: an unresolved reference is a content
    /// defect, not a reason to refuse to build the room — the placeholder art and passive fallbacks
    /// are deliberate, and a blind run must still show something.
    binding_report: ambition_platformer2d_shared_tangle::binding::BindingReport,
    /// The occurrence dispositions this plan was prepared against — the
    /// identities it deliberately did NOT plan, and the ones it planned
    /// somewhere other than where the record puts them. See
    /// [`Self::occurrence_outlook`].
    outlook: ambition_platformer2d_shared_tangle::lifecycle::RoomOccurrenceOutlook,
}

/// What the world remembers about its occurrences, and the definitions a room
/// needs in order to act on what it remembers.
///
/// the two travel as ONE value because acting on half the ledger deletes objects. A
/// `Placed` row is a single fact with two consequences: the room whose record minted the
/// occurrence must not mint it again, and the room the occurrence is lying in must rebuild it —
/// even though that room does not own the record. Stating the ledger and the world definitions
/// separately would make that road expressible; this type is what makes it not.
///
/// Nothing is derived from a room the ledger does not name — see
/// [`RoomFeatureConstructionPlan::prepare`](RoomFeatureConstructionPlan::prepare) — so the cost
/// of a large world is a slice, not a scan.
#[derive(Clone, Copy)]
pub struct OccurrenceContinuity<'a> {
    /// Horizon 1: where the occurrences this world has already minted actually
    /// are. THE authority; nothing else may hold a second opinion.
    pub remembered: &'a ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences,
    /// Every room's authored records — the world DEFINITIONS the ledger's rows
    /// refer to. A room being built reaches into these only for identities the
    /// ledger says are lying in it and that its own records do not produce.
    pub world: &'a [ambition_platformer2d_world::rooms::RoomSpec],
    /// THE SECOND DESCRIBER — how to rebuild what no record can.
    ///
    /// a RUNTIME-MINTED occurrence has no authored record in any room, so the reinstatement above
    /// can never settle its debt: the loop searches `world` for a record to relocate and finds
    /// none.
    ///
    /// the two describers are disjoint populations, not a preference order:
    /// the capture takes only `SpawnOrigin::Dynamic` rows and an authored record
    /// can never spell one. `None` is the honest answer for a composition with
    /// no checkpoint behind it.
    pub minted: Option<&'a crate::items::pickup::minted_horizon::MintedItemBaseline>,
}

/// What construction planning needs beyond the room's authored content: the
/// recipe table, and the content generation the plan is being prepared against.
#[derive(Clone, Copy)]
pub struct ActorConstructionContext<'a> {
    pub recipes: &'a crate::construction::ActorConstructionRegistry,
    /// Which generation of prepared content this room plan is bound to. A room
    /// is always content-derived, so this is always
    /// [`ContentBinding::Content`] — the enum exists because the planner also
    /// serves runtime-dynamic construction, which is not.
    pub binding: ambition_platformer2d_shared_tangle::construction::ContentBinding,
    /// The prepared cast, when the caller has one — so a lowered NPC can be asked what its
    /// CHARACTER's default autonomous profile is.
    ///
    /// `Option`, and an absent registry is a legal answer rather than a
    /// degraded one: it means no character states a default, which is exactly
    /// what this path assumed before a definition could state one. Every
    /// existing caller keeps its behaviour by saying nothing.
    pub prepared: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    /// The published controller policies, so a PLACEMENT may name one
    /// (`EnemySpawnSpec::brain_profile`). Same `Option` contract as the cast: an
    /// absent registry means this composition publishes no shared policies,
    /// which is what every level assumed before a placement could name one.
    pub brain_profiles:
        Option<&'a ambition_characters::actor::character_catalog::BrainProfileRegistry>,
    /// What the world remembers about the occurrences it has already minted,
    /// and the definitions needed to act on it — so a rebuild neither mints a
    /// second occurrence for a record whose first one is alive somewhere else,
    /// nor forgets an occurrence that is lying in this room under a record this
    /// room does not own.
    ///
    /// same `Option` contract as the cast and the policies: absent means "this
    /// composition remembers nothing", which is the honest answer for startup,
    /// for provider activation, for a hot reload that replaces the content
    /// wholesale — and for a RESET, which destroys the occurrences a
    /// disposition is about and must therefore rebuild the room from the
    /// authored records alone.
    pub continuity: Option<OccurrenceContinuity<'a>>,
}

impl<'a> ActorConstructionContext<'a> {
    pub fn new(
        recipes: &'a crate::construction::ActorConstructionRegistry,
        content_epoch: ambition_platformer2d_core::ContentEpoch,
    ) -> Self {
        Self {
            recipes,
            binding: ambition_platformer2d_shared_tangle::construction::ContentBinding::Content(
                content_epoch,
            ),
            prepared: None,
            brain_profiles: None,
            continuity: None,
        }
    }

    /// Every authority a ROOM's construction may consult, stated at once.
    ///
    /// SEVEN ROADS BUILT THIS CONTEXT BY HAND AND FOUR OF THEM WERE INCOMPLETE. Startup, reset,
    /// transition, hot reload, provider activation, the exclusive-world rebuild and the neighbour
    /// prefetch each assembled their own `new(..).with_prepared(..).with_brain_profiles(..)` chain,
    /// and a road that forgot one link did not fail to compile — it silently constructed rooms
    /// against an authority it did not have.
    ///
    /// so the authorities are PARAMETERS, not opt-in builder calls. A
    /// caller must say what it has, including saying `None`, and the next
    /// authority a room may consult is one signature change that breaks every
    /// road at once instead of seven chances to forget. `Option` still means
    /// "this composition publishes none" — that is a legal answer and always
    /// was; what is no longer possible is failing to answer.
    pub fn for_room_construction(
        recipes: &'a crate::construction::ActorConstructionRegistry,
        content_epoch: ambition_platformer2d_core::ContentEpoch,
        // The generation the SESSION is actually running, when the caller knows
        // it. A room is rebuilt from content the active binding already
        // defines, so stating a default sentinel instead makes every plan a
        // stale-looking stranger to the epoch it will commit under.
        active_binding: Option<&crate::rooms::ActiveContentBinding>,
        prepared: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
        brain_profiles: Option<
            &'a ambition_characters::actor::character_catalog::BrainProfileRegistry,
        >,
        // What became of the occurrences this world minted before, and the
        // definitions that let this room act on it. A road that rebuilds a room
        // the session has been LIVING in states it; a road that builds a world
        // from nothing, or destroys one to rebuild it, states `None` and means
        // it. See [`Self::continuity`].
        continuity: Option<OccurrenceContinuity<'a>>,
    ) -> Self {
        let mut context = Self::new(recipes, content_epoch);
        if let Some(active) = active_binding {
            context.binding = active.0;
        }
        context.prepared = prepared;
        context.brain_profiles = brain_profiles;
        context.continuity = continuity;
        context
    }

    /// Supply the prepared cast for this construction. See [`Self::prepared`].
    ///
    /// for construction that is not a ROOM's — a summon, a runtime spawn,
    /// a focused fixture. A room goes through [`Self::for_room_construction`],
    /// which is where forgetting an authority stopped being possible.
    #[must_use]
    pub fn with_prepared(
        mut self,
        prepared: &'a ambition_characters::prepared::PreparedCharacterRegistry,
    ) -> Self {
        self.prepared = Some(prepared);
        self
    }

    /// Supply the published controller policies, so a placement may name
    /// one. See [`Self::brain_profiles`].
    #[must_use]
    pub fn with_brain_profiles(
        mut self,
        profiles: &'a ambition_characters::actor::character_catalog::BrainProfileRegistry,
    ) -> Self {
        self.brain_profiles = Some(profiles);
        self
    }
}

/// Inspectable receipt for the authoritative roots scheduled by one feature plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomFeatureConstructionReceipt {
    authoritative_ids: BTreeSet<String>,
    construction: ambition_platformer2d_shared_tangle::construction::ConstructionReceipt,
    capability_lanes: capability_lanes::CapabilityReceipts,
}

impl RoomFeatureConstructionReceipt {
    pub fn authoritative_ids(&self) -> &BTreeSet<String> {
        &self.authoritative_ids
    }

    /// What the primary actor lane committed, keyed by stable identity.
    pub fn construction(
        &self,
    ) -> &ambition_platformer2d_shared_tangle::construction::ConstructionReceipt {
        &self.construction
    }
}

/// A room plan's `Debug` leads with the construction plan's canonical dump —
/// the roster it would commit — because that is what is worth reading when a
/// room appears in a failure message.
impl std::fmt::Debug for RoomFeatureConstructionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomFeatureConstructionPlan")
            .field("room", &self.room.id)
            .field(
                "expected_authoritative_ids",
                &self.expected_authoritative_ids,
            )
            .field("construction", &self.construction_deterministic_dump())
            .finish()
    }
}

/// Claim one lane's planned identities into the room's roster, refusing a
/// collision with any lane already claimed.
///
/// cross-lane identity collisions are the one thing independently typed lanes
/// cannot check for themselves: each lane's own preparation refuses duplicates
/// WITHIN it, and nothing inside a lane can see another lane's `SimId`s. This is
/// that check, and it is a fold rather than a pairwise intersection so a third
/// lane does not owe the room a third comparison.
fn claim_lane_ids(
    room: &str,
    ids: &BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId>,
    roster: &mut BTreeSet<String>,
) -> Result<(), RoomFeatureConstructionError> {
    for id in ids {
        if !roster.insert(id.to_string()) {
            return Err(RoomFeatureConstructionError::DuplicateAuthoritativeId {
                room: room.to_string(),
                id: id.to_string(),
            });
        }
    }
    Ok(())
}

impl RoomFeatureConstructionPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        room: &ambition_platformer2d_world::rooms::RoomSpec,
        registry: &crate::world::placements::PlacementLoweringRegistry,
        content_staging: &RoomContentStagingRegistry,
        catalog: &CharacterCatalog,
        sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        boss_catalog: &BossCatalog,
        construction: ActorConstructionContext<'_>,
    ) -> Result<Self, RoomFeatureConstructionError> {
        let paths = room_spec_paths(room);
        // ⭐ THE MEASUREMENT QUOTA'S TRANSACTION BOUNDARY. `plan_room` runs
        // exactly once per room build, so opening the budget here gives it a
        // structural lifetime — a reload of the SAME room gets a fresh quota,
        // which a room-id-keyed counter did not. Inert unless
        // `AMBITION_ACTOR_POPULATION_CAP` is set.
        ambition_dev_tools::population_cap::begin_room_lowering();
        let placements = registry
            .plan_room(&room.id, &paths, &room.placements)
            .map_err(RoomFeatureConstructionError::Placement)?;
        let owned_content_requests = content_staging
            .try_owned_requests_for(room)
            .map_err(RoomFeatureConstructionError::ContentStaging)?;
        let content_requests: Vec<_> = owned_content_requests
            .iter()
            .map(|(_, request)| request.clone())
            .collect();
        // THE CHARACTER NAMESPACE LEFT THIS SWEEP WITH THE ROSTER (AC6). It resolved each
        // placement's BRAIN KEY against `roster.brain_keys()`, because the lookup behind it
        // could not fail and a misspelling became the generic `combatant` body with the right
        // name on it.
        //
        // Held items are deliberately absent for the same reason:
        // `authored_ground_item_requests` REFUSES a room that names an unregistered one.
        let bindings = crate::rooms::RoomBindings::default();
        let binding_report = bindings.sweep(room);
        binding_report.log(&format!("room `{}` construction", room.id));
        // Authored-id uniqueness across actor-owned families, checked in the RAW
        // authored namespace while the outgoing room is still whole. This stays
        // separate from the plan-derived roster below so duplicate diagnostics
        // remain in the author's source vocabulary before records expand into
        // derived rows. Capability lanes perform the same SimId-level check when
        // their own plans prepare, and cross-lane collisions are refused below.
        let authored_ids = room
            .placements
            .iter()
            .map(|placement| placement.id.0.clone())
            .chain(room.enemy_spawns.iter().map(|enemy| enemy.id.clone()))
            .chain(room.boss_spawns.iter().map(|boss| boss.id.clone()))
            .chain(room.ground_items.iter().map(|item| item.id.clone()))
            .chain(content_requests.iter().map(|request| request.id.clone()));
        let mut seen_authored_ids = BTreeSet::new();
        for id in authored_ids {
            if !seen_authored_ids.insert(id.clone()) {
                return Err(RoomFeatureConstructionError::DuplicateAuthoritativeId {
                    room: room.id.clone(),
                    id,
                });
            }
        }

        // The actor-owned construction lane. Resolution failures (an authored
        // ground item naming a held item nothing provides) and identity/relation
        // failures surface HERE, while the outgoing room is still whole.
        let mut requests = crate::construction::authored_ground_item_requests(room)
            .map_err(RoomFeatureConstructionError::ActorConstruction)?;
        for (provider, request) in &owned_content_requests {
            requests.extend(crate::construction::staged_actor_requests(
                &room.id,
                provider,
                std::slice::from_ref(request),
                construction.prepared,
            ));
        }
        // Phase 4c: EVERY authored placement is a plan row carrying its frozen
        // interpreter; the executor allocates each root and the interpreter
        // populates it. Records that spawn nothing (Door interactables, inert
        // Custom payloads) are skipped by the builder, preserving behavior.
        requests.extend(crate::construction::placement_requests(
            &placements,
            &room.id,
            &paths,
        ));
        requests.extend(crate::construction::authored_static_requests(room));
        requests.extend(crate::construction::authored_actor_requests(
            room,
            &paths,
            construction.prepared,
        ));
        // Continuity decides whether each authored occurrence is rebuilt at its
        // authored position, reinstated where it was left, or suppressed because
        // it lives elsewhere/is gone. The outlook also contributes foreign
        // occurrences that now belong to this room.
        let outlook = construction
            .continuity
            .map(|continuity| continuity.remembered.outlook_for(&room.id))
            .unwrap_or_default();
        let suppressed = outlook.suppressed();
        if !outlook.is_empty() {
            requests.retain_mut(|request| {
                match outlook.disposition(&request.sim_id) {
                    ambition_platformer2d_shared_tangle::lifecycle::OccurrenceDisposition::Authored => true,
                    ambition_platformer2d_shared_tangle::lifecycle::OccurrenceDisposition::Reinstated { at } => {
                        // A family with no position to move REFUSES to pretend it
                        // moved: the request stays exactly as authored and says so,
                        // rather than being built at the wrong coordinates or
                        // dropped. Only the families a producer can write a `Placed`
                        // row for can be reinstated, and today that is one.
                        if !crate::construction::relocate_request(request, at) {
                            bevy::log::warn!(
                                target: "ambition_platformer2d::construction",
                                "room `{}` remembers `{:?}` at a relocated position, but \
                                 its construction request has no position to relocate; \
                                 building it as authored",
                                room.id,
                                request.sim_id,
                            );
                        }
                        true
                    }
                    ambition_platformer2d_shared_tangle::lifecycle::OccurrenceDisposition::Suppressed => false,
                }
            });
        }
        // ── THE OCCURRENCES THIS ROOM OWES AND DOES NOT AUTHOR ──────────────
        //
        // Whatever the room's own records did not answer for is an occurrence
        // lying HERE whose record lives somewhere else. The world's definitions
        // are in front of us, so the room stops being a pure function of ONE
        // `RoomSpec` and becomes what it always had to be: current residency,
        // reconstructed from the world's definitions plus the authoritative
        // disposition of every occurrence.
        //
        // only rooms the ledger's obligation actually reaches are touched.
        // The loop stops the moment the debt is settled and never runs at all
        // for the overwhelmingly common empty ledger, so a big world costs a
        // slice bound, not a scan.
        if let Some(continuity) = construction.continuity {
            let mut owed = outlook.reinstatements();
            for request in &requests {
                owed.remove(&request.sim_id);
            }
            for foreign in continuity.world {
                if owed.is_empty() {
                    break;
                }
                if foreign.id == room.id {
                    continue;
                }
                // a foreign room that cannot yield its records REFUSES this build, rather
                // than quietly dropping the occurrence. The world promised the player an object
                // back and cannot produce it; that is a preflight failure — raised while the
                // outgoing room is still whole, like every other one here — and not a silent
                // deletion.
                let candidates = crate::construction::reinstatable_authored_requests(foreign)
                    .map_err(RoomFeatureConstructionError::ActorConstruction)?;
                for mut request in candidates {
                    let Some(at) = owed.remove(&request.sim_id) else {
                        continue;
                    };
                    if !crate::construction::relocate_request(&mut request, at) {
                        bevy::log::warn!(
                            target: "ambition_platformer2d::construction",
                            "room `{}` owes `{:?}`, whose record in room `{}` has no \
                             position to reinstate it at; it cannot come back here",
                            room.id,
                            request.sim_id,
                            foreign.id,
                        );
                        continue;
                    }
                    requests.push(request);
                }
            }
            // ── the SECOND DESCRIBER: a debt no RECORD can settle, that the
            //    CHECKPOINT can. ─────────────────────────────────────────────
            //
            // a runtime mint has no authored record anywhere, so the search
            // above could never settle its debt; what rebuilds it is the
            // description the checkpoint captured. The position is the ledger's
            // `at`, which is the whole point — this is an object lying where
            // somebody dropped it, and nothing else supplies where.
            for (sim_id, at) in owed {
                let described = continuity
                    .minted
                    .and_then(|minted| minted.description_of(&sim_id));
                if let Some(description) = described {
                    // `held_spec_by_id`, NOT `ambition_characters::brain::held_item_by_id`.
                    // The narrow one knows only the brain's registry; a mint that
                    // came out of the INVENTORY resolves through the item catalog
                    // (`Item::from_held_item_id`) and the narrow lookup answers
                    // `None` for it — which sent a javelin down the "no item spec
                    // answers to that id" arm and lost it a second time.
                    match crate::items::pickup::held_spec_by_id(&description.held_item) {
                        Some(held) => {
                            requests.push(crate::construction::ActorConstructionRequest {
                                sim_id: sim_id.clone(),
                                // the occurrence's OWN provenance, carried
                                // verbatim: a rebuilt mint with no
                                // `SpawnOrigin::Dynamic` is invisible to the NEXT
                                // capture, so it would survive exactly one death
                                // and then become unrecoverable.
                                origin: description.origin.clone(),
                                parameters:
                                    crate::construction::ActorConstructionParams::GroundItem {
                                        spec: ambition_platformer2d_world::rooms::GroundItemSpec {
                                            id: sim_id.as_str().to_string(),
                                            name: format!("Ground item: {}", description.held_item),
                                            held_item: description.held_item.clone(),
                                            pos: at,
                                            half_extent:
                                                crate::items::pickup::MINTED_ITEM_HALF_EXTENT,
                                        },
                                        held,
                                    },
                                relations: Vec::new(),
                            });
                            continue;
                        }
                        None => {
                            // a CONTENT change: the spec has been edited out of
                            // the catalog since the checkpoint was taken.
                            bevy::log::warn!(
                                target: "ambition_platformer2d::construction",
                                "room `{}` remembers minted `{sim_id:?}` lying at {at:?} as a \
                                 `{}`, and no item spec answers to that id any more",
                                room.id,
                                description.held_item,
                            );
                            continue;
                        }
                    }
                }
                // Refusing to build the room would make it permanently unenterable, so this is
                // loud and the room is built without it.
                bevy::log::warn!(
                    target: "ambition_platformer2d::construction",
                    "room `{}` remembers occurrence `{sim_id:?}` lying at {at:?}, no room in \
                     this world authors a record that can rebuild it, and the checkpoint \
                     carries no description of it either",
                    room.id,
                );
            }
        }
        // Authored mount links are planned `ambition.mount` relations between
        // those rows; a link naming nobody fails HERE instead of being retried
        // forever by the deleted frame-later resolver.
        crate::construction::attach_authored_mount_links(room, &mut requests)
            .map_err(RoomFeatureConstructionError::ActorConstruction)?;
        // Actor-domain relation semantics, checked while the outgoing room is
        // still whole: cardinality (one host per limb, one rider per mount),
        // family legality, and pilot/mount class compatibility. The generic
        // planner below enforces the structural rules; these are the ones only
        // this domain can state.
        crate::construction::preflight_actor_relations(
            &requests,
            boss_catalog,
            construction.prepared,
        )
        .map_err(RoomFeatureConstructionError::ActorConstruction)?;
        crate::construction::preflight_planned_bodies(&requests, construction.prepared)
            .map_err(RoomFeatureConstructionError::ActorConstruction)?;
        let construction_scope =
            ambition_platformer2d_shared_tangle::construction::ConstructionScope {
                binding: construction.binding,
                room: Some(room.id.clone()),
            };
        let construction_plan = crate::construction::ActorConstructionPlan::prepare(
            construction_scope.clone(),
            requests,
            // An occurrence in somebody's custody crosses the boundary alive, so a room CAN be
            // prepared while one of the identities it authors is already out there.
            //
            // the `retain` above is the FIX; passing the same set here is the
            // GUARD. The planner refuses `IdentityAlreadyLive` — so a future
            // road that acquires a request for a suppressed identity by some
            // other route gets a loud refusal during preflight, while the
            // outgoing room is still whole, instead of two live things behind
            // one `SimId`.
            &suppressed,
            construction.recipes,
        )
        .map_err(RoomFeatureConstructionError::Construction)?;

        let capability_lanes = capability_lanes::CapabilityLanes::prepare(
            &construction_scope,
            room,
            &outlook,
            &suppressed,
        )
        .map_err(RoomFeatureConstructionError::Construction)?;

        let actor_ids = construction_plan.planned_ids();

        // The authoritative roster the room PREDICTS, derived from every typed
        // construction lane. Lanes remain independently typed and verified; the
        // room composes only their stable identities.
        //
        // Claiming into one set answers both questions at once and cannot be half-updated: a lane
        // that is composed is a lane that is checked.
        let mut expected_authoritative_ids: BTreeSet<String> = BTreeSet::new();
        claim_lane_ids(&room.id, &actor_ids, &mut expected_authoritative_ids)?;
        capability_lanes.claim_planned_ids(&room.id, &mut expected_authoritative_ids)?;

        let mut placement_context =
            crate::world::placements::ActorPlacementContext::new(catalog, sheets);
        if let Some(prepared) = construction.prepared {
            placement_context = placement_context.with_prepared(prepared);
        }
        if let Some(profiles) = construction.brain_profiles {
            placement_context = placement_context.with_brain_profiles(profiles);
        }
        Ok(Self {
            room: room.clone(),
            construction_services: crate::construction::ActorConstructionServices {
                context: placement_context,
                boss_catalog: boss_catalog.clone(),
            },
            content_requests,
            construction: construction_plan,
            capability_lanes,
            expected_authoritative_ids,
            binding_report,
            outlook,
        })
    }

    /// The dispositions this plan was prepared against.
    ///
    /// A frozen plan is only valid for the world that produced it: a plan
    /// prepared while an authored object was being carried OMITS that object,
    /// and committing it into a world where the object has since been put down
    /// would leave the room permanently missing it. Stated on the artifact so a
    /// cache can compare rather than guess — see the prefetch promotion check.
    ///
    /// the whole outlook, not just the suppressed ids. A plan prepared
    /// while a relocated object rested at one position is not the plan this
    /// world wants once it rests at another, and a set of identities cannot tell
    /// those two apart.
    pub fn occurrence_outlook(
        &self,
    ) -> &ambition_platformer2d_shared_tangle::lifecycle::RoomOccurrenceOutlook {
        &self.outlook
    }

    /// Every reference this room makes and does not keep.
    ///
    /// Empty means each id the room points at was found.
    pub fn binding_report(&self) -> &ambition_platformer2d_shared_tangle::binding::BindingReport {
        &self.binding_report
    }

    /// The primary actor-owned construction lane for this room.
    pub fn construction(&self) -> &crate::construction::ActorConstructionPlan {
        &self.construction
    }

    /// `cfg(test)`, both of these: production never asks a room WHICH LANE
    /// built something. It asks the room for its roster, and each lane verifies
    /// itself against the shared baseline. A lane accessor exists so a test can
    /// prove an identity lives in one lane and not another — which is a claim
    /// only a test makes, and the reason these were dead code in every build.
    #[cfg(all(test, feature = "portal"))]
    pub(crate) fn portal_construction(&self) -> &ambition_portal2d::PortalGunConstructionPlan {
        self.capability_lanes.portal()
    }

    #[cfg(test)]
    pub(crate) fn gravity_construction(
        &self,
    ) -> &ambition_platformer2d_shared_tangle::gravity::construction::GravityZoneConstructionPlan
    {
        self.capability_lanes.gravity()
    }

    /// Canonical construction fingerprint material for every lane this room
    /// prepared. The actor lane remains primary; optional capabilities append
    /// independently typed named lanes without entering the actor enum.
    pub fn construction_deterministic_dump(&self) -> String {
        use std::fmt::Write as _;

        let mut out = String::from("room-construction-lanes-v1\n");
        let actor = self.construction.deterministic_dump();
        let _ = writeln!(
            out,
            "domain\t{}\t{}",
            crate::construction::ACTOR_CONSTRUCTION_DOMAIN,
            actor.len()
        );
        out.push_str(&actor);
        self.capability_lanes.write_deterministic_dump(&mut out);
        out
    }

    pub(crate) fn construction_binding(
        &self,
    ) -> ambition_platformer2d_shared_tangle::construction::ContentBinding {
        let binding = self.construction.scope().binding;
        self.capability_lanes.debug_assert_binding(binding);
        binding
    }

    /// Verify every independently typed construction lane against the same
    /// pre-transaction baseline. The room transaction owns publication; each
    /// lane keeps its own parameter, service, and postcondition vocabulary.
    pub(crate) fn verify_committed_construction(
        &self,
        receipt: &RoomFeatureConstructionReceipt,
        baseline: &ambition_platformer2d_shared_tangle::construction::TransactionBaseline,
        world: &mut bevy::prelude::World,
        session: SessionSpawnScope,
    ) -> Vec<ambition_platformer2d_shared_tangle::construction::RosterViolation> {
        use ambition_platformer2d_shared_tangle::construction::{
            verify_committed_roster, AuthoritativeScope,
        };

        let actor_transaction = self.construction.transaction(session);
        let actor_scope = AuthoritativeScope::gather(world, &actor_transaction);
        let mut violations = verify_committed_roster(
            &self.construction,
            &receipt.construction,
            baseline,
            &actor_scope,
            world,
        )
        .err()
        .unwrap_or_default();
        violations.extend(crate::construction::verify_rig_composition(
            &self.construction,
            &receipt.construction,
            world,
        ));

        self.capability_lanes.verify(
            &receipt.capability_lanes,
            baseline,
            world,
            session,
            &mut violations,
        );

        violations.sort_by_key(|violation| format!("{violation:?}"));
        violations.dedup();
        violations
    }

    pub fn room(&self) -> &ambition_platformer2d_world::rooms::RoomSpec {
        &self.room
    }

    pub fn expected_authoritative_ids(&self) -> &BTreeSet<String> {
        &self.expected_authoritative_ids
    }

    /// Every character this room stages, by the token `CharacterLoadDemand`
    /// accepts — the provider-owned content requests AND the actors the
    /// placement lowering planned (`NpcSpawn`, programmatic enemies).
    ///
    /// The second half was added 2026-09-02 while chasing the hall's 111
    /// placeholder rectangles. ⚠ It was NOT that bug's cause — the room's own
    /// `Interactable(Npc)` loop already named those ids; the cause was the
    /// per-frame load ration dropping its remainder (see
    /// `demand_room_character_sheets`). It stays because a plan-staged actor
    /// that is not an authored placement (a programmatic spawn) had no other
    /// road into the reveal barrier's demand.
    pub fn content_staged_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .content_requests
            .iter()
            .map(|request| request.name.clone())
            .collect();
        for entity in self.construction.entities() {
            if let crate::construction::ActorConstructionParams::StagedActor(request) =
                entity.parameters()
            {
                names.push(request.name.clone());
                // The kind may carry the catalog id the display name is not.
                if let super::spawn_actors::SpawnActorKind::Enemy { character, .. } =
                    &request.kind
                {
                    names.push(character.to_string());
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    /// Rebuild one authored authoritative root through the exact interpreter
    /// and catalogs frozen by this plan.
    ///
    /// This uses the same typed construction plan ordinary room construction
    /// commits. Capability-owned lanes are consulted after the actor lane.
    pub fn respawn_authoritative_entity(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        authored_id: &str,
    ) -> bool {
        let planned_id = ambition_platformer2d_shared_tangle::sim_id::SimId::placement(authored_id);
        self.respawn_authoritative_sim_id(commands, session_scope, &planned_id)
    }

    /// Rebuild one PLANNED authoritative root by its stable identity — the form
    /// that can name a derived row.
    ///
    /// [`Self::respawn_authoritative_entity`] converts its authored id through
    /// `SimId::placement`, which can never spell a `SimId::spawned` identity —
    /// so a giant's HAND was planned, closable, and yet unreachable through the
    /// production API. Dynamic and derived authoritative roots need
    /// reconstruction exactly as much as authored ones; this is their entry
    /// point, and the authored-id form is now a convenience wrapper over the
    /// same closure commit.
    ///
    /// Rebuilds the RELATION CLOSURE, not the bare row. A row at either end of a
    /// planned relation cannot be rebuilt alone — rebuilding one end strands the
    /// other on a dead `Entity` handle
    /// (`ConstructionError::RelationCutBySubset`). A giant host and its two
    /// hands are exactly such a cluster: asking for ANY one of the three — host,
    /// left hand, right hand — rebuilds all three. For an unrelated row the
    /// closure is just itself, so this is a plain single-row commit.
    pub fn respawn_authoritative_sim_id(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        sim_id: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> bool {
        if self.construction.get(sim_id).is_some() {
            let closure = self
                .construction
                .relation_closure(&std::collections::BTreeSet::from([sim_id.clone()]));
            let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
                commands,
                scope: self.construction.scope(),
                session: session_scope,
                services: &self.construction_services,
            };
            return match self.construction.commit_subset(&closure, &mut ctx) {
                Ok(_) => true,
                Err(error) => {
                    bevy::log::error!(
                        target: "ambition_platformer2d::construction",
                        "`{sim_id}` is planned in the actor lane but its reconstruction closure \
                         could not be rebuilt: {error}"
                    );
                    false
                }
            };
        }

        if let Some(outcome) = self
            .capability_lanes
            .respawn(sim_id, commands, session_scope)
        {
            return outcome;
        }

        false
    }

    /// Apply the exact feature decisions captured by [`Self::prepare`].
    ///
    /// A feature plan is one participant in a room transaction, not the transaction, so it
    /// cannot know when the room is complete. The bracket lives with the outer artifact that
    /// does — see [`crate::world::rooms::transaction`].
    pub fn spawn(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
    ) -> RoomFeatureConstructionReceipt {
        // Every actor-owned authoritative family is a plan row, committed below
        // with its relations. Capability-owned families use sibling lanes.
        commands.insert_resource(crate::features::FactionRelations::default());

        let construction = {
            let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
                commands,
                scope: self.construction.scope(),
                session: session_scope,
                services: &self.construction_services,
            };
            self.construction.commit(&mut ctx)
        };
        debug_assert_eq!(
            construction.committed_ids(),
            self.construction.planned_ids(),
            "construction execution diverged from its prepared roster",
        );

        let capability_receipts = self.capability_lanes.commit(commands, session_scope);

        // The COMMITTED roster: the union of every independently typed lane.
        // The outer predicted-vs-committed cross-check in `stage::spawn_contents`
        // therefore reduces to one question: did every planned root commit?
        let mut authoritative_ids: BTreeSet<String> = construction
            .committed_ids()
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        capability_receipts.extend_committed_ids(&mut authoritative_ids);

        RoomFeatureConstructionReceipt {
            authoritative_ids,
            construction,
            capability_lanes: capability_receipts,
        }
    }
}

pub fn spawn_room_feature_entities_from_plan(
    commands: &mut Commands,
    plan: &RoomFeatureConstructionPlan,
    session_scope: SessionSpawnScope,
) -> RoomFeatureConstructionReceipt {
    plan.spawn(commands, session_scope)
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
/// [`EncounterMobSeed`] says which body, which character, and which brain — and
/// documents why those are three answers rather than one.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    prepared: &ambition_characters::prepared::PreparedCharacterRegistry,
    session_scope: SessionSpawnScope,
    encounter_id: impl Into<String>,
    mob: EncounterMobSeed<'_>,
) {
    super::spawn_actors::spawn_encounter_mob(
        commands,
        catalog,
        authored_sheets,
        prepared,
        session_scope,
        encounter_id,
        mob,
    );
}

#[cfg(test)]
mod tests;
