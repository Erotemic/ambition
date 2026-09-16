//! Canonical prepared room construction.
//!
//! Every room lifecycle path prepares the same mutation-free
//! [`RoomConstructionPlan`] before it retires live entities. The plan freezes
//! target identity, authored geometry, resolved placement interpreters,
//! content-staged actor requests, catalogs, moving-platform starts, and the
//! expected authoritative roster. Startup, reset, ordinary transition, LDtk
//! hot reload, and snapshot reconstruction execute this one artifact.
//!
//! Snapshot reconstruction also executes this canonical construction plan; it
//! is not a second construction authority.

use ambition_combat::components::ActorFaction;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::prelude::{Commands, Resource};

use super::transaction;
use crate::features::{self, RoomFeatureConstructionPlan};
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::placements::PlacementLoweringRegistry;
use ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity;
use ambition_platformer2d_shared_tangle::lifecycle::{
    session_world_component_mut, SessionSpawnScope,
};
use ambition_platformer2d_world::platforms::MovingPlatformState;
use ambition_platformer2d_world::rooms::{RespawnRoomVisualsRequested, RoomSet, RoomSpec};

/// Stable same-build identity for one prepared construction artifact.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RoomConstructionPlanId(String);

impl RoomConstructionPlanId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why canonical room construction could not be prepared. Every variant is
/// detected before any live-room mutation.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomConstructionError {
    UnknownRoom {
        room: String,
    },
    InvalidFeatures {
        room: String,
        reason: features::RoomFeatureConstructionError,
    },
    /// ⛔⛤ **THE LIVE GENERATION'S MECHANICS ARE ABSENT IN A COMPOSITION THAT
    /// OWES THEM.** A shell-routed session's rooms are rebuilt from the
    /// registries its generation was prepared against; falling back to whatever
    /// the App holds now rebuilds a live world out of a generation it was never
    /// prepared for. ⇒ A road that rebuilds a LIVE room refuses instead. See
    /// `session::mechanics::GenerationMechanics::for_live_session`.
    LiveGenerationMechanicsMissing,
}

impl std::fmt::Display for RoomConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRoom { room } => {
                write!(f, "no room named `{room}` in the prepared RoomSet")
            }
            Self::InvalidFeatures { room, reason } => {
                write!(f, "room `{room}` construction is invalid: {reason}")
            }
            Self::LiveGenerationMechanicsMissing => write!(
                f,
                "this composition routes gameplay through a shell session, so its \
                 rooms are rebuilt from the generation's own registries — and \
                 `SessionMechanics` is absent. Rebuilding from the App's current \
                 registries would construct a world this session was never \
                 prepared for."
            ),
        }
    }
}

impl std::error::Error for RoomConstructionError {}

/// Last successfully scheduled room-construction commit.
///
/// This is developer evidence, not simulation authority: the active RoomSet and
/// spawned ECS entities remain authoritative. It lets diagnostics and tests join
/// a committed room to the exact immutable plan and root roster that produced it.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct LastRoomConstructionCommit {
    pub plan_id: RoomConstructionPlanId,
    pub room_id: String,
    pub authoritative_ids: BTreeSet<String>,
    pub moving_platform_count: usize,
}

/// What a committed room stamps on [`LastRoomConstructionCommit`] beyond its
/// receipt. A harness that builds a feature plan directly has no such identity
/// and passes `None`.
pub(crate) struct RoomCommitStamp {
    pub(crate) plan_id: RoomConstructionPlanId,
    pub(crate) room_id: String,
    pub(crate) moving_platform_count: usize,
}

/// Build one room's candidate population — INTO A WORLD THAT CAN HIDE ONE, and
/// into no other.
///
/// ⛔⛤ **THE PREREQUISITE AND THE ACTION IT AUTHORIZES HAVE ONE OWNER.** This is
/// ONE exclusive-world command: it consults `opening_refused` and applies its own
/// `CommandQueue` only if the opening decision was to proceed, so a world that
/// cannot hide a candidate performs ZERO candidate construction.
///
/// ⛔ **DO NOT QUEUE THE CONSTRUCTION ONTO THE CALLER'S `Commands` AGAIN.** Queued
/// behind `transaction::open`, an opening refusal cannot stop the commands
/// already sitting behind it — the roots get built, VISIBLY in the
/// filter-missing case, and cleaned up afterwards. A refusal that has to be
/// repaired is not a refusal, and the window is observable: component hooks and
/// lifecycle observers run during `queue.apply` even though no scheduled system
/// does. `verify_and_publish`'s retirement is the BACKSTOP, not the fix.
///
/// ⚠ Recipes gain no `World` access; only the queue's application point moved.
///
/// ⛔ A free function because the test harness calls it too — a harness holding
/// its own copy of this road is how an arm keeps passing after production stops
/// using it.
pub(crate) fn construct_room_candidate(
    commands: &mut Commands,
    publication: transaction::PublicationHandle,
    plan: &RoomFeatureConstructionPlan,
    session_scope: SessionSpawnScope,
    stamp: Option<RoomCommitStamp>,
    predicted: Option<BTreeSet<String>>,
) {
    let plan = plan.clone();
    commands.queue(move |world: &mut bevy::prelude::World| {
        if transaction::opening_refused(world, publication) {
            // ⛔ BUILD NOTHING. Not "build, then retire".
            return;
        }
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let receipt = {
            let mut inner = Commands::new(&mut queue, &*world);
            let receipt = features::spawn_room_feature_entities_from_plan(
                &mut inner,
                &plan,
                session_scope,
            );
            // no platform VISUAL is spawned here any more. The commit installs
            // platform STATE (the receipt counts it); the picture is reconciled
            // by a render family from `MovingPlatformSet`, like every other room
            // feature. That is what let the visual adapter leave the actor
            // monolith at all — see `world::platforms`.
            //
            // ⛔ AND IT IS INSIDE THE AUTHORIZED BRANCH: a room that was refused
            // committed nothing, so it must not leave a record saying it did.
            if let Some(stamp) = stamp {
                inner.insert_resource(LastRoomConstructionCommit {
                    plan_id: stamp.plan_id,
                    room_id: stamp.room_id,
                    authoritative_ids: receipt.authoritative_ids().clone(),
                    moving_platform_count: stamp.moving_platform_count,
                });
            }
            receipt
        };
        queue.apply(world);
        if let Some(predicted) = predicted {
            debug_assert_eq!(
                receipt.authoritative_ids(),
                &predicted,
                "room construction execution diverged from its prepared root roster",
            );
        }
        if let Ok(mut entity) = world.get_entity_mut(publication.0) {
            entity.insert(transaction::PendingConstructionReceipt(receipt));
        }
    });
}

/// The one prepared artifact for a room's authoritative simulation contents.
#[derive(Clone)]
pub struct RoomConstructionPlan {
    id: RoomConstructionPlanId,
    target_index: usize,
    features: RoomFeatureConstructionPlan,
    platform_states: Vec<MovingPlatformState>,
    session_scope: SessionSpawnScope,
}

impl std::fmt::Debug for RoomConstructionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomConstructionPlan")
            .field("id", &self.id)
            .field("target_index", &self.target_index)
            .field("room", &self.features.room().id)
            .field(
                "expected_authoritative_ids",
                self.features.expected_authoritative_ids(),
            )
            .field("platform_count", &self.platform_states.len())
            .field("session_scope", &self.session_scope)
            .finish()
    }
}

impl RoomConstructionPlan {
    /// Prepare from already-borrowed services. This is the system-facing seam
    /// used by activation, reset, transition, and hot reload.
    pub fn prepare_from_parts(
        rooms: &RoomSet,
        target_index: usize,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        boss_catalog: &ambition_boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
        construction: features::ActorConstructionContext<'_>,
    ) -> Result<Self, RoomConstructionError> {
        let spec = rooms.rooms.get(target_index).cloned().ok_or_else(|| {
            RoomConstructionError::UnknownRoom {
                room: format!("<room-index-{target_index}>"),
            }
        })?;
        Self::prepare_spec(
            target_index,
            spec,
            placement_lowering,
            content_staging,
            boss_catalog,
            session_scope,
            construction,
        )
    }

    /// Prepare a room whose containing `RoomSet` is itself a candidate artifact,
    /// as in transactional LDtk hot reload.
    pub fn prepare_spec(
        target_index: usize,
        spec: RoomSpec,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        boss_catalog: &ambition_boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
        // ⛔ WHO EXISTS AND WHAT THEY LOOK LIKE RIDE HERE. This signature used to
        // take the character catalog and the authored sheets as two more
        // positional arguments and pass them straight down, while three other
        // authorities from the SAME snapshot arrived on this one value. See
        // [`features::ActorConstructionContext::characters`].
        construction: features::ActorConstructionContext<'_>,
    ) -> Result<Self, RoomConstructionError> {
        let feature_plan = RoomFeatureConstructionPlan::prepare(
            &spec,
            placement_lowering,
            content_staging,
            boss_catalog,
            construction,
        )
        .map_err(|reason| RoomConstructionError::InvalidFeatures {
            room: spec.id.clone(),
            reason,
        })?;
        let platform_states =
            ambition_platformer2d_world::platforms::moving_platforms_for_room(&spec);
        let id = construction_plan_id(&spec, &feature_plan);
        Ok(Self {
            id,
            target_index,
            features: feature_plan,
            platform_states,
            session_scope,
        })
    }

    pub fn id(&self) -> &RoomConstructionPlanId {
        &self.id
    }

    pub fn target_index(&self) -> usize {
        self.target_index
    }

    pub fn room_id(&self) -> &str {
        &self.features.room().id
    }

    pub fn spec(&self) -> &RoomSpec {
        self.features.room()
    }

    /// Whether a currently installed room definition is byte-for-byte the
    /// authored spec this plan prepared. This rejects a same-id hot reload from
    /// committing a stale in-flight transition.
    pub fn matches_room_spec(&self, candidate: &RoomSpec) -> bool {
        let prepared =
            serde_json::to_vec(self.spec()).expect("prepared RoomSpec must remain serializable");
        let current =
            serde_json::to_vec(candidate).expect("candidate RoomSpec must remain serializable");
        prepared == current
    }

    pub fn platform_states(&self) -> &[MovingPlatformState] {
        &self.platform_states
    }

    pub fn predicted_authoritative_ids(&self) -> &BTreeSet<String> {
        self.features.expected_authoritative_ids()
    }

    /// The occurrence dispositions this plan was prepared against.
    ///
    /// A plan is frozen against a world that remembered exactly this much. A
    /// cached plan prepared while an authored object was in somebody's hands
    /// left that object out; committing it into a world where the object has
    /// since been put down and destroyed would leave the room permanently
    /// short. Anything that holds a plan across frames compares this before
    /// promoting it.
    ///
    /// it is the whole outlook, not a set of suppressed identities. A plan
    /// that placed a relocated object at one position is not the plan a world
    /// wants once that object rests at another, and an identity set cannot tell
    /// those two apart.
    pub fn occurrence_outlook(
        &self,
    ) -> &ambition_platformer2d_shared_tangle::lifecycle::RoomOccurrenceOutlook {
        self.features.occurrence_outlook()
    }

    pub fn content_staged_names(&self) -> Vec<String> {
        self.features.content_staged_names()
    }

    /// Rebuild one authored authoritative root through this plan's frozen
    /// interpreter/catalog decisions.
    pub fn respawn_authoritative_entity(&self, commands: &mut Commands, authored_id: &str) -> bool {
        self.features
            .respawn_authoritative_entity(commands, self.session_scope, authored_id)
    }

    /// Rebuild one PLANNED root by its stable identity — the only form that can
    /// name a derived row like a giant's hand (`SimId::spawned`), which no
    /// authored-id spelling reaches.
    pub fn respawn_authoritative_sim_id(
        &self,
        commands: &mut Commands,
        sim_id: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> bool {
        self.features
            .respawn_authoritative_sim_id(commands, self.session_scope, sim_id)
    }

    pub fn session_scope(&self) -> SessionSpawnScope {
        self.session_scope
    }

    /// Enqueue the prepared room contents without changing active-room
    /// resources. Session startup uses this after those resources are installed.
    ///
    /// This is the room transaction boundary. Everything the room is made of is queued
    /// between [`transaction::open`] and [`transaction::close`], so the verification that
    /// publishes `RoomLoaded` runs after ALL of it: the feature families, the planned roots,
    /// the planned relationships, the moving-platform bodies, and the last-commit receipt.
    ///
    /// The bracket sits HERE rather than inside the feature plan because the
    /// feature plan does not know when the room is complete — it is one
    /// participant. When it owned the bracket, the platform bodies and the
    /// commit receipt below were queued after its verification had already run
    /// and published, so `RoomLoaded` described a room that was still being
    /// built.
    ///
    /// ⚠ **THE CALLER DECLARES WHAT HAPPENS TO THE RECEIPT.** A caller that
    /// drops the handle says [`transaction::PublicationRetention::UntilTheVerdictIsRecorded`]
    /// and the publication ends with its verdict; a caller that will ask
    /// `publication_succeeded` later says `UntilOwnerRetires` and owes a
    /// [`transaction::retire_publication`].
    pub fn spawn_contents(
        &self,
        commands: &mut Commands,
        retention: transaction::PublicationRetention,
    ) -> transaction::PublicationHandle {
        let publication = transaction::begin_publication(
            commands,
            self.room_id().to_string(),
            self.features.construction_transactions(self.session_scope),
            retention,
        );
        self.spawn_contents_for(publication, commands);
        publication
    }

    /// As [`Self::spawn_contents`], into a publication the caller already began
    /// — which is how `replace_live_world` gets its staged world onto the SAME
    /// publication the transaction will verify.
    fn spawn_contents_for(
        &self,
        publication: transaction::PublicationHandle,
        commands: &mut Commands,
    ) {
        // ⛔ THE ROOM DECLARES WHAT IT IS REBUILDING. See `transaction::open`
        // for the measurement that this had no production caller at all.
        transaction::open(commands, publication, &self.features, self.session_scope);
        // ⛔⛤ **THE CANDIDATE ROAD IS THE ONLY ROAD — the rollout switch was
        // deleted 2026-09-15.** Every root in every lane is minted
        // `InactiveCandidate`, so nothing this room builds is visible to an
        // ordinary query until `transaction::close` admits it, and a refused
        // room is DROPPED rather than left standing half-built.
        //
        // ⭐⭐ **WHAT UNBLOCKED IT WAS A DECLARATION, NOT A RULING.** This
        // paragraph used to say the flag *"waits on a gameplay ruling alone"* —
        // `Q124`, what happens to a placement in your custody when a death
        // rebuilds the room that authored it — because turning it on refused
        // `death_restores_the_checkpoint` with
        //
        //     violations=[ Duplicated { placement:ground_gun_sword, count: 2 },
        //                  ReconstructedOldSurvived { stale: 514v0 } ]
        //
        // ⇒ **BOTH OF THOSE ARE THE VERIFIER ANSWERING A QUESTION NOBODY MEANT
        // TO ASK.** `reconstructing` states *"the old body should already be
        // gone"*, so a carried predecessor is a violation BY DEFINITION. The
        // transaction now splits its declaration per identity —
        // `superseding(id, id)` for one the baseline still holds,
        // `reconstructing` for the rest — and the coexistence it was refusing is
        // the A10 premise. See `transaction::open`.
        //
        // ⚠ And `Q124` really is a ruling, but a narrower one than this flag: the
        // baseline decides PER ITEM, and both halves already run — the room
        // re-authors the object in its world state, the custodian's own
        // retraction takes it out of the hand. See `retire_superseded` for the
        // measured reason publication must not do that half itself.
        //
        // ⛔⛤ **THE OTHER BLOCKER WAS BIGGER THAN THE RULING AND IS ALSO
        // CLOSED — 2026-09-13.** A census of every room-construction refusal
        // across the whole `app_it` suite found SIX, and the two largest were on
        // the HOT-RELOAD road rather than the death road: 8 placements and **18,
        // the whole of `central_hub_complex`**. The world log gave the mechanism:
        //
        //     f16  session-end   activation=2 scope=0
        //     f16  session-start activation=3 scope=1
        //          room-refused :: 18x Duplicated, 18x ReconstructedOldSurvived
        //
        // A session handoff retires the old scope and starts the new one IN THE
        // SAME FRAME, and `GameplaySessionSet::Providers` was ordered BEFORE
        // `SessionScopeSet::Cleanup`, so the incoming room's transaction captured
        // a baseline that still held every one of the outgoing scope's
        // placements. ⛔ NOT A RACE — a declared order: `Providers` is
        // `.before(Presentation)` and `Presentation` chained ahead of
        // `RetireAuthority -> Cleanup`.
        //
        // ⇒ `SessionScopeSet` now chains `RetireAuthority -> Cleanup -> Activate
        // -> Presentation`: the dying scope finishes dying before the live one is
        // born. **The census fell from 6 refusals to 4**, and all four that
        // remain were the single-placement custody shape above. Guarded by
        // `nothing_orders_the_retired_scopes_sweep_against_the_incoming_sessions_construction`
        // (the schedule) and by `an_edited_pack_reaches_the_cast_the_shipped_composition_plays`
        // (the production verdict).
        //
        // ⚠ It cost ALMOST nothing visible, which is why it survived that long —
        // and the "no production reader" half of that sentence was WRONG.
        // MEASURED 2026-09-14: `RoomLoaded` has three production readers through
        // `ambition_combat::events::FreshAttempt`, so a refusal also left staged
        // hits unvoided and per-attempt state un-re-armed. Both are the right
        // outcome (no attempt began), which is why nobody noticed. Under this
        // bracket it would have dropped the whole room and landed every hot
        // reload in an EMPTY WORLD.
        //
        // ✅ **AND THE BRACKET NOW BUYS THE WHOLE ROOM-SCOPE GUARANTEE.** This
        // paragraph read *"`replace_live_world` still retires the OUTGOING room
        // and writes `RoomSet` / `RoomGeometry` / the platform state BEFORE any of
        // this runs"*. It stages all of it now — see `PendingWorldReplacement` —
        // and a refusal leaves the session with the room it was already playing.
        // ⚠ The SESSION scope is a different transaction and is not started; see
        // `docs/planning/queue.md`'s A10 row for what each half covers.
        // ⛔⛤ **THE CONSTRUCTION BOUNDARY OWNS BOTH THE PREREQUISITE AND THE
        // ACTION IT AUTHORIZES — 2026-09-15 AUDIT, FINDING 4.**
        //
        // This used to call the spawn right here, which queued every candidate
        // command BEHIND `open`'s. `open` runs first at flush and can discover
        // that this world cannot hide a candidate — and could do nothing about
        // the commands already sitting behind it:
        //
        //     queue open -> queue construction -> queue close
        //     flush: open REFUSES; construction runs anyway (VISIBLY, because the
        //            refusal IS that nothing here is hidden); close cleans up
        //
        // A refusal that has to be repaired afterwards is not a refusal. Worse,
        // the window is not unobservable: `commit_inactive`'s own note records
        // that component hooks and lifecycle observers DO run during
        // `queue.apply`, even though no scheduled system does.
        //
        // ⇒ The whole construction is ONE exclusive-world command that consults
        // the opening decision first and applies its own command queue only if
        // that decision was to proceed. A world that cannot hide a candidate
        // performs ZERO candidate construction — there is nothing to clean up,
        // and `refuse`'s retirement stays as the backstop rather than the fix.
        //
        // ⚠ RECIPES GAIN NO `World` ACCESS FROM THIS. They still execute through
        // the constrained `RootScope`/`RelationScope` surface; what moved is
        // WHERE the queue they fill is applied.
        construct_room_candidate(
            commands,
            publication,
            &self.features,
            self.session_scope,
            Some(RoomCommitStamp {
                plan_id: self.id.clone(),
                room_id: self.room_id().to_string(),
                moving_platform_count: self.platform_states.len(),
            }),
            Some(self.predicted_authoritative_ids().clone()),
        );
        transaction::close(commands, publication, &self.features, self.session_scope);
    }

    /// Replace the live world with this prepared room, IF the room verifies.
    ///
    /// ⛔⛤ **THIS IS THE A10 BOUNDARY, AND AS OF 2026-09-14 IT KEEPS ITS
    /// PROMISE.** Its doc used to end: *"Collapsing the pair here does NOT
    /// remove the destructive window; it gives it one address. A10's stronger
    /// last-good-world guarantee is the fix … and when it lands it lands HERE."*
    /// It landed here. Nothing below changes the live world:
    ///
    /// ```text
    /// stage the whole replacement off to the side   (PendingWorldReplacement)
    ///   -> build the room as HIDDEN CANDIDATES      (spawn_contents)
    ///   -> declare what publication would do        (superseding / retiring)
    ///   -> PROJECT and VALIDATE that world          (verify_projected_roster)
    ///        refusal   -> candidates dropped, staged world dropped, N untouched
    ///        admission -> publish candidates, THEN sweep N and publish its state
    /// ```
    ///
    /// ⚠ **THE OUTGOING ROOM IS STILL STANDING WHILE THE CANDIDATE IS BUILT**,
    /// and that is deliberate rather than tolerated: it is what the candidate is
    /// being validated AGAINST. Two rooms' worth of bodies coexist for the length
    /// of one command flush, which no SCHEDULED system can observe — the same
    /// bound `publish_candidate` states, and the same reason the old order could
    /// be described as safe.
    ///
    /// `next_rooms` is `Some` when the caller is replacing the ROOM SET as well
    /// as the active room — a hot reload rebuilds the set from re-read content; a
    /// transition walks within the set it already has.
    ///
    /// `arrival` is where the transiting body lands. It is applied with the rest
    /// of the publication and never before it: see [`transaction::StagedArrival`]
    /// for the app-level arm that found the body being placed into a room its
    /// own transaction had refused.
    ///
    /// ⛔ **A CALLER MUST NOT READ THE LIVE GEOMETRY OR ROOM SET AFTER CALLING
    /// THIS AND EXPECT THE NEW ROOM.** It has not been written yet and may never
    /// be. Read the plan instead — [`Self::spec`] and `next_rooms` are the same
    /// values, named at their source.
    pub fn replace_live_world<'a>(
        &self,
        commands: &mut Commands,
        outgoing: impl IntoIterator<Item = (Entity, bool)> + 'a,
        carry_body: Option<Entity>,
        next_rooms: Option<RoomSet>,
        arrival: Option<transaction::StagedArrival>,
    ) -> transaction::PublicationHandle {
        // Collected HERE rather than inside the staged closure: the roster comes
        // from the caller's own query, which cannot outlive this call.
        let outgoing: Vec<(Entity, bool)> = outgoing
            .into_iter()
            .filter(|(entity, _)| carry_body != Some(*entity))
            .collect();
        let mut pending = transaction::PendingWorldReplacement::new(
            outgoing,
            next_rooms,
            self.target_index,
            self.spec().world.clone(),
            self.platform_states.clone(),
        );
        // ⛔ THE ARRIVING BODY IS A PUBLICATION EFFECT LIKE THE REST. A
        // transition that places it before the verdict leaves a refused player
        // standing at the coordinates of a room that does not exist.
        if let Some(arrival) = arrival {
            pending = pending.arriving(arrival);
        }
        // ⛔ **ON THE PUBLICATION ITSELF, and inserted BEFORE the transaction
        // opens**, because `transaction::open` READS it: the identities standing
        // on the outgoing bodies are what the transaction declares it is RETIRING,
        // and a declaration made after the baseline is captured would be a claim
        // about a world nobody looked at. Being ON the publication is what makes
        // `staged world(P)` and `baseline(P)` the same P by construction rather
        // than by a search that could find somebody else's.
        let publication = transaction::begin_publication(
            commands,
            self.room_id().to_string(),
            self.features.construction_transactions(self.session_scope),
            // ⛔ A WORLD REPLACEMENT ALWAYS HAS AN OWNER. Every caller of
            // `replace_live_world` reads this verdict — the transition finalizes
            // behind it, the hot reload and the reset queue their effects behind
            // it — so the receipt stands until that owner retires it.
            transaction::PublicationRetention::UntilOwnerRetires,
        );
        commands.entity(publication.0).insert(pending);
        self.spawn_contents_for(publication, commands);
        publication
    }

}

/// Identity of one prepared room-construction artifact, from EVERY frozen
/// world-defining preparation product — not just the authored source.
///
/// `deterministic_dump()` is the canonical rendering of exactly that derived surface (schema
/// version, content binding/epoch, every plan row with recipe + origin + parameter summary,
/// every relation with its canonical payload), so folding it in makes the id a function of the
/// complete frozen plan.
///
/// Moving platforms and kinematic paths are pure functions of the spec, so the spec JSON
/// already covers them. Deliberately EXCLUDED: `SessionSpawnScope` / `TransactionId`
/// (commit-time, not frozen-plan), `Entity` values, and anything process-local.
fn construction_plan_id(
    spec: &RoomSpec,
    features: &RoomFeatureConstructionPlan,
) -> RoomConstructionPlanId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    spec.id.hash(&mut hasher);
    // RoomSpec is the canonical authored room artifact; JSON avoids depending
    // on map insertion order because its fields are vectors/ordered values.
    serde_json::to_vec(spec)
        .expect("RoomSpec serialization must succeed for construction identity")
        .hash(&mut hasher);
    features.construction_deterministic_dump().hash(&mut hasher);
    RoomConstructionPlanId(format!("room-plan:{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::construction::ContentBinding;

    fn empty_spec(id: &str) -> RoomSpec {
        RoomSpec::new(
            id,
            ae::World::new(
                id,
                ae::Vec2::new(640.0, 480.0),
                ae::Vec2::new(96.0, 96.0),
                Vec::new(),
            ),
        )
    }

    /// THE FIXTURE CAST, because every body is built from a character (AC6)
    /// and a placement that names none is refused at construction.
    ///
    /// `'static`: `ActorConstructionContext` BORROWS the cast, so a per-test
    /// local would not outlive the plan it is handed to.
    fn fixture_cast() -> &'static ambition_characters::prepared::PreparedCharacterRegistry {
        static CAST: std::sync::OnceLock<ambition_characters::prepared::PreparedCharacterRegistry> =
            std::sync::OnceLock::new();
        CAST.get_or_init(|| {
            let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
            // `npc_giant_gnu_hands` is minted by `giant_cluster_rows` for the two
            // limb rows, so a cast without it refuses the whole giant cluster.
            for id in ["combatant", "npc_giant_gnu_hands"] {
                let mut definition =
                    ambition_characters::actor::definition::CharacterDefinition::new(
                        id, id, "test",
                    )
                    .with_locomotion(
                        ambition_characters::actor::CharacterLocomotion {
                            run_speed: 155.0,
                            ..Default::default()
                        },
                    );
                definition.vitals.max_health = Some(4);
                let finalized = crate::character_runtime::prepare_and_finalize_for_test(
                    definition,
                    &ambition_characters::prepared::CharacterBindings::default(),
                );
                registry.insert_prepared(finalized.prepared);
            }
            registry
        })
    }

    fn prepare(spec: RoomSpec) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, &catalog, &sheets, ContentBinding::content_unstated(Default::default()))
                .with_prepared(fixture_cast()),
        )
    }

    #[test]
    fn equivalent_room_construction_has_stable_identity() {
        let a = prepare(empty_spec("same")).expect("first plan");
        let b = prepare(empty_spec("same")).expect("second plan");
        assert_eq!(a.id(), b.id());
        assert_eq!(
            a.predicted_authoritative_ids(),
            b.predicted_authoritative_ids()
        );
    }

    #[cfg(feature = "portal")]
    #[test]
    fn portal_gun_is_a_capability_owned_construction_lane() {
        let mut spec = empty_spec("portal-lane");
        spec.portal_gun_spawns
            .push(ambition_platformer2d_world::rooms::PortalGunSpawnSpec {
                id: "gun".to_string(),
                name: "Aperture Device".to_string(),
                pos: ae::Vec2::new(120.0, 80.0),
                half_extent: ae::Vec2::new(8.0, 6.0),
                pair: 0,
            });
        let plan = prepare(spec).expect("portal-gun room plan");
        let gun = ambition_platformer2d_shared_tangle::sim_id::SimId::placement("gun");

        assert!(
            plan.features.construction().get(&gun).is_none(),
            "portal-gun vocabulary must not re-enter ActorConstructionParams",
        );
        assert!(
            plan.features.portal_construction().get(&gun).is_some(),
            "the portal capability owns the authored pickup row",
        );
        assert_eq!(
            plan.features.portal_construction().lane().as_str(),
            ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN,
        );

        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        // ⛔ THE COMPOSITION PRODUCTION BUILDS. Room construction builds
        // every root hidden, and `transaction::open` REFUSES a world that cannot
        // hide one rather than validating candidates in plain sight — so a bare
        // `App` here is a fixture that never reaches the subject.
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(
                &mut commands,
                transaction::PublicationRetention::UntilTheVerdictIsRecorded,
            );
        }
        app.world_mut().flush();

        let entity = {
            let mut query = app.world_mut().query::<(
                bevy::prelude::Entity,
                &ambition_platformer2d_shared_tangle::sim_id::SimId,
            )>();
            query
                .iter(app.world())
                .find_map(|(entity, id)| (id == &gun).then_some(entity))
                .expect("constructed portal-gun root")
        };
        assert!(app
            .world()
            .get::<ambition_portal2d::PortalGunPickup>(entity)
            .is_some());
        assert!(
            app.world()
                .resource::<crate::features::LastConstructionVerification>()
                .published,
            "all typed construction lanes must verify before RoomLoaded",
        );
    }

    /// As [`prepare`], but with an explicit CAST and content epoch — the two
    /// preparation inputs OUTSIDE the `RoomSpec` that shape the derived plan.
    ///
    /// it took a `&CharacterRoster` and the giant tests below handed it
    /// rows declaring `mount_class: Some("giant")`. A character states its mount
    /// now (AC6), so the cast is the input that decides whether a placement
    /// lowers to a limbed host.
    fn prepare_with(
        spec: RoomSpec,
        cast: &ambition_characters::prepared::PreparedCharacterRegistry,
        epoch: ae::ContentEpoch,
    ) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, &catalog, &sheets, ContentBinding::content_unstated(epoch)).with_prepared(cast),
        )
    }

    /// A cast whose `"giant_gnu"` is a `"giant"`-class limbed host.
    fn giant_cast(
        mount_class: Option<&str>,
    ) -> ambition_characters::prepared::PreparedCharacterRegistry {
        let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "giant_gnu",
            "Giant GNU",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 0.0,
            ..Default::default()
        });
        definition.vitals.max_health = Some(42);
        if let Some(class) = mount_class {
            definition.mount = Some(ambition_characters::actor::CharacterMount {
                class: Some(class.to_string()),
                ..Default::default()
            });
        }
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        // The hands travel with the host: `giant_cluster_rows` mints two limb
        // rows naming `npc_giant_gnu_hands`.
        let mut registry = fixture_cast().clone();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    fn giant_spec_sized(id: &str, half: f32) -> RoomSpec {
        let mut spec = empty_spec(id);
        let payload = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("giant_gnu".into()),
            "giant_gnu",
        );
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "gnu",
                "Giant GNU",
                ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(half)),
                payload,
            ));
        spec
    }

    fn giant_spec(id: &str) -> RoomSpec {
        giant_spec_sized(id, 60.0)
    }

    /// The plan id tracks the DERIVED construction surface, not just the
    /// authored spec. Two rosters that differ only in the giant's body size
    /// produce byte-identical `RoomSpec`s but different hand `home_offset`
    /// relation payloads — materially different prepared worlds. The previous id
    /// (spec JSON + authored id set) collided them.
    #[test]
    fn the_plan_id_tracks_the_derived_relation_payloads() {
        let small = prepare_with(
            giant_spec_sized("arena", 60.0),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("small-giant plan");
        let large = prepare_with(
            giant_spec_sized("arena", 70.0),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("large-giant plan");
        assert_ne!(
            small.id(),
            large.id(),
            "different hand offsets are different prepared worlds"
        );
    }

    /// The id also tracks the giant-vs-ordinary shape of the plan itself: the
    /// same spec whose brain key stops resolving as a `"giant"`-class host loses
    /// its host/hand rows AND their relations.
    #[test]
    fn the_plan_id_tracks_the_giant_expansion() {
        let giant = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("giant plan");
        // Same spec, but the cast has no idea "giant_gnu" is a giant.
        let plain = prepare_with(giant_spec("arena"), &giant_cast(None), ae::ContentEpoch(4))
            .expect("plain plan");
        assert_ne!(giant.id(), plain.id());
    }

    /// The id tracks the prepared-content epoch: the same room prepared against
    /// re-prepared content is a different transaction target.
    #[test]
    fn the_plan_id_tracks_the_content_epoch() {
        let four = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("epoch-4 plan");
        let five = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(5),
        )
        .expect("epoch-5 plan");
        assert_ne!(four.id(), five.id());
    }

    /// Frozen room path content reaches the id (through the spec AND through the
    /// giant host row that now carries the paths).
    #[test]
    fn the_plan_id_tracks_frozen_path_content() {
        let bare = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("pathless plan");
        let mut with_path = giant_spec("arena");
        with_path
            .kinematic_paths
            .push(ambition_platformer2d_world::rooms::KinematicPathSpec::new(
                "patrol",
                "patrol",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
                ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 24.0),
            ));
        let pathed = prepare_with(with_path, &giant_cast(Some("giant")), ae::ContentEpoch(4))
            .expect("pathed plan");
        assert_ne!(bare.id(), pathed.id());
    }

    /// One giant, every roster surface, one answer. The prepared plan, the
    /// predicted outer roster, the commit receipt, and the boundary verifier all
    /// name the same three-cluster — and the hands are welcome plan rows, not
    /// unexpected or legacy findings.
    #[test]
    fn a_giant_rooms_rosters_agree_from_plan_to_receipt_to_verifier() {
        let plan = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("giant plan");

        let host = ambition_platformer2d_shared_tangle::sim_id::SimId::placement("gnu");
        let cluster: BTreeSet<String> = [
            host.to_string(),
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(&host, 0).to_string(),
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(&host, 1).to_string(),
        ]
        .into();

        // Prepared plan: three giant-cluster identities.
        let planned: BTreeSet<String> = plan
            .features
            .construction()
            .planned_ids()
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        assert_eq!(planned, cluster, "host + two hands are the plan rows");
        // Predicted outer roster: the same three (nothing else in this room).
        assert_eq!(plan.predicted_authoritative_ids(), &cluster);

        let expected_plan_id = plan.id().clone();
        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        // ⛔ THE COMPOSITION PRODUCTION BUILDS. Room construction builds
        // every root hidden, and `transaction::open` REFUSES a world that cannot
        // hide one rather than validating candidates in plain sight — so a bare
        // `App` here is a fixture that never reaches the subject.
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(
                &mut commands,
                transaction::PublicationRetention::UntilTheVerdictIsRecorded,
            );
        }
        app.world_mut().flush();

        // Commit receipt: the same three.
        let commit = app.world().resource::<LastRoomConstructionCommit>();
        assert_eq!(commit.plan_id, expected_plan_id);
        assert_eq!(commit.authoritative_ids, cluster);

        // Boundary verifier: published, and NOTHING flagged — a hand read as
        // unexpected or legacy would appear here.
        let verification = app
            .world()
            .resource::<crate::world::rooms::LastConstructionVerification>();
        assert!(
            verification.published,
            "the giant room publishes: {:?}",
            verification.violations
        );
        assert_eq!(
            verification.violations,
            Vec::new(),
            "no hand is unexpected, legacy, or malformed"
        );
    }

    #[test]
    fn duplicate_authoritative_roots_fail_before_commit() {
        let mut spec = empty_spec("duplicate");
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0));
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "same-id",
                "first",
                aabb,
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                    "combatant",
                ),
            ));
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "same-id",
                "second",
                aabb,
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                    "combatant",
                ),
            ));
        let error = prepare(spec).expect_err("duplicate roots must fail preparation");
        assert!(matches!(
            error,
            RoomConstructionError::InvalidFeatures {
                reason: features::RoomFeatureConstructionError::DuplicateAuthoritativeId { .. },
                ..
            }
        ));
    }

    /// `RoomLoaded` is published only after the WHOLE room is applied.
    ///
    /// The transaction boundary is `spawn_contents`, not the feature plan. An observer reads
    /// the world the instant `RoomLoaded` is delivered and proves the platforms, the commit
    /// receipt, and the authoritative bodies are already present.
    #[test]
    fn room_loaded_observes_a_fully_committed_room() {
        let mut spec = empty_spec("published");
        spec.moving_platforms
            .push(MovingPlatformState::from_authored(
                ae::Vec2::new(0.0, 200.0),
                ae::Vec2::new(96.0, 16.0),
                120.0,
                60.0,
            ));
        // A CONTENT-STAGED actor, so it is a plan row: the executor stamps its
        // `SimId` during construction, which is what lets an observer at
        // publication time see it. An `enemy_spawn` gets its id from
        // `ensure_sim_id` in a later system that this minimal app does not run.
        let mut staging = features::RoomContentStagingRegistry::default();
        staging
            .register("published", "test_provider", "occ", "occ.v1", |_room| {
                vec![ambition_platformer2d_actor_spawn::SpawnActorRequest {
                    id: "occupant".into(),
                    name: "occupant".into(),
                    pos: ae::Vec2::ZERO,
                    half_size: ae::Vec2::splat(10.0),
                    faction: ambition_combat::components::ActorFaction::Npc,
                    grudge_against: None,
                    kind: ambition_platformer2d_actor_spawn::SpawnActorKind::Enemy {
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "combatant".into(),
                        ),
                        character: ambition_entity_catalog::CharacterId::from("combatant"),
                    },
                }]
            })
            .expect("stager registers");
        let recipes = crate::construction::engine_construction_registry();
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let plan = RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &staging,
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, &catalog, &sheets, ContentBinding::content_unstated(Default::default()))
                .with_prepared(fixture_cast()),
        )
        .expect("plan");

        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        // ⛔ THE COMPOSITION PRODUCTION BUILDS. Room construction builds
        // every root hidden, and `transaction::open` REFUSES a world that cannot
        // hide one rather than validating candidates in plain sight — so a bare
        // `App` here is a fixture that never reaches the subject.
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        app.add_message::<ambition_platformer2d_actor_spawn::SpawnActorRequest>();

        let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = observed.clone();
        app.add_systems(
            bevy::prelude::Update,
            move |mut reader: bevy::ecs::message::MessageReader<
                ambition_platformer2d_world::rooms::RoomLoaded,
            >,
                  commit: Option<bevy::prelude::Res<LastRoomConstructionCommit>>,
                  ids: bevy::prelude::Query<
                &ambition_platformer2d_shared_tangle::sim_id::SimId,
            >| {
                if reader.read().next().is_some() {
                    *sink.lock().unwrap() = Some((
                        commit.map(|c| c.moving_platform_count),
                        ids.iter().any(|id| id.as_str() == "placement:occupant"),
                    ));
                }
            },
        );

        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(
                &mut commands,
                transaction::PublicationRetention::UntilTheVerdictIsRecorded,
            );
        }
        app.update();

        let (commit_platforms, saw_occupant) = observed
            .lock()
            .unwrap()
            .expect("RoomLoaded must have published for a valid room");
        assert_eq!(
            commit_platforms,
            Some(1),
            "the last-commit receipt existed before RoomLoaded"
        );
        // What the test defends is unchanged — the receipt, and therefore the room's STATE, is
        // complete before `RoomLoaded` publishes.
        assert!(
            saw_occupant,
            "the authoritative occupant existed before RoomLoaded"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // A10 ACCEPTANCE: A REFUSED CANDIDATE LEAVES THE PLAYABLE WORLD INTACT
    // ═══════════════════════════════════════════════════════════════════════
    //
    // ⛔⛤ **THESE TWO ARMS ARE THE PROPERTY, NOT A HELPER'S BEHAVIOUR.** They go
    // through `replace_live_world` — the one production entry every room
    // lifecycle path takes — and assert on the WHOLE live world either side of
    // the verdict: the bodies, the geometry, the active room, and the
    // moving-platform state. A10 is not closed because candidates can coexist;
    // it is closed when a refusal costs the running game nothing.

    /// N: a session root carrying the OLD room's geometry and room set, the OLD
    /// platform state, and two bodies wearing authored identities.
    ///
    /// ⚠ The room set holds BOTH rooms and is active on index 0, so
    /// `set_active(1)` is an observable write rather than a no-op — a fixture
    /// whose "before" and "after" agree cannot fail.
    fn last_good_world(platform: MovingPlatformState) -> (bevy::prelude::App, Vec<Entity>) {
        use ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component;

        let mut app = bevy::prelude::App::new();
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        app.add_message::<ambition_platformer2d_actor_spawn::SpawnActorRequest>();
        app.insert_resource(ambition_platformer2d_world::collision::MovingPlatformSet(vec![
            platform,
        ]));
        insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(empty_spec("n").world.clone()),
        );
        insert_session_world_component(
            app.world_mut(),
            RoomSet::from_parts("n", vec![empty_spec("n"), candidate_spec()], Vec::new()),
        );
        let outgoing = ["n_body_a", "n_body_b"]
            .into_iter()
            .map(|id| {
                app.world_mut()
                    .spawn((
                        ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id),
                        RoomScopedEntity,
                    ))
                    .id()
            })
            .collect();
        (app, outgoing)
    }

    /// The room the candidate would become: a DIFFERENT geometry, at index 1,
    /// carrying its own platform and one content-staged occupant.
    fn candidate_spec() -> RoomSpec {
        let mut spec = RoomSpec::new(
            "candidate",
            ae::World::new(
                "candidate",
                ae::Vec2::new(1280.0, 960.0),
                ae::Vec2::new(64.0, 64.0),
                Vec::new(),
            ),
        );
        spec.moving_platforms
            .push(MovingPlatformState::from_authored(
                ae::Vec2::new(300.0, 300.0),
                ae::Vec2::new(64.0, 16.0),
                100.0,
                50.0,
            ));
        spec
    }

    fn candidate_plan() -> RoomConstructionPlan {
        candidate_plan_for(SessionSpawnScope::UNSCOPED)
    }

    /// The same plan, prepared under an explicit session.
    ///
    /// ⛔ A transaction's SESSION is what tells the verifiers which root it is
    /// publishing into — see `session_root_for_scope`. An `UNSCOPED` plan can
    /// only ask *"which root is live"*, which is the wrong question the moment
    /// the root it belongs to is a hidden candidate.
    fn candidate_plan_for(session: SessionSpawnScope) -> RoomConstructionPlan {
        let mut staging = features::RoomContentStagingRegistry::default();
        staging
            .register("candidate", "test_provider", "occ", "occ.v1", |_room| {
                vec![ambition_platformer2d_actor_spawn::SpawnActorRequest {
                    id: "occupant".into(),
                    name: "occupant".into(),
                    pos: ae::Vec2::ZERO,
                    half_size: ae::Vec2::splat(10.0),
                    faction: ambition_combat::components::ActorFaction::Npc,
                    grudge_against: None,
                    kind: ambition_platformer2d_actor_spawn::SpawnActorKind::Enemy {
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "combatant".into(),
                        ),
                        character: ambition_entity_catalog::CharacterId::from("combatant"),
                    },
                }]
            })
            .expect("stager registers");
        let recipes = crate::construction::engine_construction_registry();
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        RoomConstructionPlan::prepare_spec(
            1,
            candidate_spec(),
            &PlacementLoweringRegistry::default(),
            &staging,
            &ambition_boss_encounter::BossCatalog::default(),
            session,
            features::ActorConstructionContext::new(&recipes, &catalog, &sheets, ContentBinding::content_unstated(Default::default()))
                .with_prepared(fixture_cast()),
        )
        .expect("the candidate room plans")
    }

    /// What the live world holds right now, in the four places
    /// `replace_live_world` used to write before anyone had verified anything.
    fn live_world(app: &mut bevy::prelude::App) -> (String, usize, usize, Vec<String>) {
        let geometry = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
            ambition_platformer2d_core::RoomGeometry,
        >(app.world())
        .expect("the session root carries geometry")
        .0
        .name
        .clone();
        let active = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
            RoomSet,
        >(app.world())
        .expect("the session root carries a room set")
        .active;
        let platforms = app
            .world()
            .resource::<ambition_platformer2d_world::collision::MovingPlatformSet>()
            .0
            .len();
        let mut ids: Vec<String> = app
            .world_mut()
            .query::<&ambition_platformer2d_shared_tangle::sim_id::SimId>()
            .iter(app.world())
            .map(|id| id.as_str().to_string())
            .collect();
        ids.sort();
        (geometry, active, platforms, ids)
    }

    /// Every staged world still standing, whoever holds it.
    ///
    /// ⛔ Ask the PUBLICATION entity. The staged
    /// world is a `PendingWorldReplacement` component on the entity
    /// `begin_publication` spawns, and that entity is spawned plain — no
    /// disabling marker — so an ordinary query sees it. A query keyed on the
    /// candidate's transaction answers ZERO unconditionally: the publication
    /// carries no `TransactionId`.
    fn staged_worlds_alive(app: &mut bevy::prelude::App, _plan: &RoomConstructionPlan) -> usize {
        let world = app.world_mut();
        let mut query = world.query::<&transaction::PendingWorldReplacement>();
        query.iter(world).count()
    }

    fn stage_the_candidate(app: &mut bevy::prelude::App, plan: RoomConstructionPlan, outgoing: Vec<Entity>) {
        app.add_systems(
            bevy::prelude::Update,
            move |mut commands: Commands| {
                plan.replace_live_world(
                    &mut commands,
                    outgoing.iter().map(|entity| (*entity, false)),
                    None,
                    None,
                    None,
                );
            },
        );
        app.update();
    }

    /// Stage a candidate through the production road and KEEP its receipt.
    ///
    /// `run_system_once` rather than `add_systems` + `update`, because two
    /// staged candidates need two ONE-SHOT runs: a system added twice runs twice
    /// on the next update and would stage the first plan a second time.
    fn stage_and_keep_the_receipt(
        app: &mut bevy::prelude::App,
        plan: RoomConstructionPlan,
        outgoing: Vec<Entity>,
    ) -> super::transaction::PublicationHandle {
        bevy::ecs::system::RunSystemOnce::run_system_once(
            app.world_mut(),
            move |mut commands: Commands| {
                plan.replace_live_world(
                    &mut commands,
                    outgoing.iter().map(|entity| (*entity, false)),
                    None,
                    None,
                    None,
                )
            },
        )
        .expect("the staging system runs")
    }

    /// ⛔⛤ **AN UNRELATED PUBLICATION MUST NOT INVALIDATE AN OWNER'S RECEIPT.**
    ///
    /// `begin_publication` used to reap every finished publication in the world,
    /// so this sequence — A finishes, the caller still holds A's handle, B
    /// begins — silently turned `publication_succeeded(A)` from `true` into
    /// `false`. The owner's follow-up writes are conditional on that answer, so
    /// an unrelated room beginning to publish would have skipped them.
    ///
    /// ⚠ **BOTH PUBLICATIONS ARE REAL ONES**, from `replace_live_world` on the
    /// production road, not hand-spawned entities: what is under test is
    /// `begin_publication`'s own behaviour, and a fixture that spawned its own
    /// publications would certify nothing about it.
    #[test]
    fn a_later_publication_does_not_invalidate_an_earlier_owners_receipt() {
        use super::transaction::{publication_succeeded, retire_publication};

        let (mut app, outgoing) = last_good_world(MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(30.0, 8.0),
            40.0,
            5.0,
        ));

        let first = stage_and_keep_the_receipt(&mut app, candidate_plan(), outgoing);
        assert!(
            publication_succeeded(app.world(), first),
            "the fixture's premise: the first candidate publishes. Without that              this arm would be asking whether a refusal survives a later              publication, which is a different question"
        );

        // B begins — and finishes — while A's owner still holds A's handle.
        let second = stage_and_keep_the_receipt(&mut app, candidate_plan(), Vec::new());
        assert!(
            publication_succeeded(app.world(), first),
            "⛔ A LATER PUBLICATION ATE AN EARLIER OWNER'S RECEIPT. Every caller              whose writes are gated on `publication_succeeded` would skip them              because an unrelated room began publishing"
        );
        assert!(
            publication_succeeded(app.world(), second),
            "the second publication's own receipt"
        );

        // ⭐ AND RETIRING ONE ENDS EXACTLY ONE. This is the only thing that ends
        // a publication now.
        retire_publication(app.world_mut(), first);
        assert!(
            !publication_succeeded(app.world(), first),
            "a retired receipt is consumed: its owner has read it and said so"
        );
        assert!(
            publication_succeeded(app.world(), second),
            "⛔ RETIRING ONE PUBLICATION DESTROYED ANOTHER'S CONTROL PLANE"
        );
    }

    /// ⛔ **THE VERDICT READER, ASKED THE WAYS A CALLER CAN GET IT WRONG.**
    ///
    /// ⛔⛤ **IT USED TO BE KEYED BY ROOM NAME AND THAT WAS THE DEFECT.** The old
    /// reader asked `LastConstructionVerification` — last-writer-wins — whether
    /// the last verdict for a room with this NAME said published, so two
    /// operations on one room were indistinguishable and a caller could authorize
    /// its own follow-up mutations from somebody else's success. The reader is
    /// keyed by the exact publication the caller started, so "another
    /// publication's success" is not a case that needs checking: it is a
    /// different entity.
    ///
    /// ⚠ **THIS TESTS THE READER, NOT THE WIRING.** The dev hot reload's call is
    /// still not exercised — forcing a reload to be refused needs a reload
    /// harness this repository does not have.
    #[test]
    fn the_verdict_reader_answers_about_one_publication_and_not_about_any() {
        use super::transaction::{publication_succeeded, PublicationHandle, PublicationVerdict};

        let mut app = bevy::prelude::App::new();
        let mine = PublicationHandle(app.world_mut().spawn_empty().id());
        let theirs = PublicationHandle(app.world_mut().spawn_empty().id());

        assert!(
            !publication_succeeded(app.world(), mine),
            "a publication with NO verdict yet read as a success: a caller whose \
             writes are conditional on it would make them mid-flight"
        );

        app.world_mut()
            .entity_mut(theirs.0)
            .insert(PublicationVerdict { published: true });
        assert!(
            !publication_succeeded(app.world(), mine),
            "⛔ ANOTHER PUBLICATION'S SUCCESS READ AS THIS ONE'S"
        );
        assert!(publication_succeeded(app.world(), theirs));

        app.world_mut()
            .entity_mut(mine.0)
            .insert(PublicationVerdict { published: false });
        assert!(
            !publication_succeeded(app.world(), mine),
            "a REFUSED publication read as published"
        );

        app.world_mut().entity_mut(mine.0).despawn();
        assert!(
            !publication_succeeded(app.world(), mine),
            "a publication that no longer exists read as a success"
        );
    }

    /// ⛔⛤ **ANOTHER SESSION'S BODY IS NOT THIS TRANSACTION'S TO RETIRE.**
    ///
    /// Two sessions of one experience legitimately hold the same authored
    /// placement ids, and a candidate session prepared beside a live one puts both
    /// populations in the world at once. A whole-world BASELINE then finds the
    /// other session's body wearing an identity this room plans, declares it
    /// SUPERSEDED, and publication despawns it. MEASURED 2026-09-15 on the shipped
    /// handoff: *"18 declared departures retired"* one frame before the incoming
    /// session even started — the candidate destroying the world that was playing.
    ///
    /// ⭐ **THE CONTROL IS WHAT MAKES THIS ARM MEAN ANYTHING.** The SAME body,
    /// owned by THIS session, must still be retired: that is an ordinary
    /// supersession and it proves the identity really is one this room builds and
    /// that the retirement mechanism really is live. Without it, "the other
    /// session's body survived" would also be true of an id nothing plans.
    #[test]
    fn a_rooms_publication_retires_its_own_sessions_predecessor_and_not_another_sessions() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            SessionScopedEntity, SessionScopeId,
        };

        fn stage_beside(owner: SessionScopeId) -> (bool, bool) {
            let (mut app, outgoing) = last_good_world(MovingPlatformState::from_authored(
                ae::Vec2::new(10.0, 20.0),
                ae::Vec2::new(30.0, 8.0),
                40.0,
                5.0,
            ));
            // A body wearing an identity the candidate room is about to build,
            // owned by `owner`.
            let predecessor = app
                .world_mut()
                .spawn((
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement("occupant"),
                    SessionScopedEntity(owner),
                ))
                .id();
            stage_the_candidate(
                &mut app,
                candidate_plan_for(SessionSpawnScope::scoped(SessionScopeId(0))),
                outgoing,
            );
            let published = app
                .world()
                .resource::<crate::features::LastConstructionVerification>()
                .published;
            (published, app.world().get_entity(predecessor).is_ok())
        }

        // ⭐ THE CONTROL: this session's own predecessor IS superseded and retired.
        let (published, survived) = stage_beside(SessionScopeId(0));
        assert!(
            published,
            "the control's room did not publish, so it says nothing about retirement"
        );
        assert!(
            !survived,
            "the control did not collide: `placement:occupant` is not an identity \
             this room supersedes, so the arm below would be true of nothing"
        );

        // And the identical identity owned by ANOTHER session is left alone.
        let (published, survived) = stage_beside(SessionScopeId(99));
        assert!(published, "the room refused over another session's identity");
        assert!(
            survived,
            "⛔ A ROOM'S PUBLICATION DESPAWNED ANOTHER SESSION'S BODY. This is the \
             defect that made the candidate session destroy the world that was \
             playing: a whole-world baseline declares another session's identities \
             superseded"
        );
    }

    /// ⛔⛤ **A ROOM PUBLISHES INTO A SESSION THAT IS ITSELF STILL A CANDIDATE.**
    ///
    /// The session root carries a canonical `SimId` and no room `TransactionId`,
    /// because the transaction that owns it is the SESSION's publication. While
    /// that root is a hidden candidate, the room's own verifiers see an identity
    /// that is in no plan, in no baseline (the baseline is the LIVE world) and
    /// stamped by nobody — and call the room's own session a stray.
    ///
    /// ⚠ **THE SHIPPED ORDER AVOIDS THIS BY ACCIDENT, WHICH IS WHY THIS ARM
    /// EXISTS.** Session activation hides its root only after the room's
    /// transaction has opened, so the baseline sees it published and nothing
    /// complains. Nothing states that ordering as a rule, and the moment a
    /// candidate session is prepared off to the side — hidden at spawn, which is
    /// the whole point — the accident stops holding. MEASURED 2026-09-15:
    /// without the exemption this arm defends, the room refuses with
    /// `UnownedIdentity { sim_id: SimId("session:...") }`.
    #[test]
    fn a_room_publishes_into_a_session_root_that_is_still_a_hidden_candidate() {
        let (mut app, outgoing) = last_good_world(MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(30.0, 8.0),
            40.0,
            5.0,
        ));
        // The fixture's root, given the canonical identity a real session root
        // carries and then HIDDEN — the shape a candidate session's root has
        // while its first room is being verified.
        let root = ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(app.world())
            .expect("the fixture carries a session root");
        app.world_mut().entity_mut(root).insert(
            ambition_platformer2d_shared_tangle::sim_id::SimId::singleton("session", "7"),
        );
        bevy::ecs::system::RunSystemOnce::run_system_once(
            app.world_mut(),
            move |mut commands: Commands| {
                ambition_platformer2d_shared_tangle::construction::hide_candidate_session_root(
                    &mut commands,
                    root,
                );
            },
        )
        .expect("the hiding system runs");

        // ⛔ A SCOPED PLAN, because the question the verifiers must ask is *"which
        // root does THIS transaction publish into"*, and only a scoped plan can
        // pose it. The fixture's root is `SessionRoot(SessionScopeId(0))`.
        stage_the_candidate(
            &mut app,
            candidate_plan_for(SessionSpawnScope::scoped(
                ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0),
            )),
            outgoing,
        );

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            verification.published,
            "⛔ A ROOM COULD NOT PUBLISH INTO ITS OWN SESSION while that session \
             was still a hidden candidate: {verification:?}"
        );
        let about_the_session = |violations: &[String]| -> Vec<String> {
            violations
                .iter()
                .filter(|violation| violation.contains("session:7"))
                .cloned()
                .collect()
        };
        let roster: Vec<String> = verification
            .violations
            .iter()
            .map(|violation| format!("{violation:?}"))
            .collect();
        let projected: Vec<String> = verification
            .projection_violations
            .iter()
            .map(|violation| format!("{violation:?}"))
            .collect();
        assert!(
            about_the_session(&roster).is_empty() && about_the_session(&projected).is_empty(),
            "⛔ THE ROOM CALLED ITS OWN SESSION A STRAY. A session root is not a \
             room-constructed identity, published or hidden: roster {:?}, projected {:?}",
            about_the_session(&roster),
            about_the_session(&projected)
        );

        // ⚠ **AND THE STAGED-WORLD CHECK IS A SEPARATE, STILL-OPEN GAP**, named
        // rather than asserted away: `verify_staged_world` asks
        // `session_world_entity` — *"which root is LIVE"* — so a staged
        // replacement into a candidate session refuses with
        // `NoSessionRootToPublishInto`. This fixture's plan is `UNSCOPED`, so it
        // cannot even ask the scoped question. Recorded in the A10 row; it is the
        // same lesson as `verify_and_publish`'s own root precondition, one layer
        // further in.
    }

    /// ⛔⛤ **A REFUSED CANDIDATE LEAVES WORLD N EXACTLY AS IT WAS.**
    ///
    /// The refusal is a REAL production one: the room transaction compares the
    /// generation it was prepared under against the session's live
    /// `ActiveContentBinding` and refuses a stale room. Nothing test-only is
    /// wired into the construction path to produce it.
    #[test]
    fn a_refused_candidate_room_leaves_the_playable_world_untouched() {
        let platform = MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            64.0,
            10.0,
        );
        let (mut app, outgoing) = last_good_world(platform.clone());
        let before = live_world(&mut app);
        assert_eq!(
            before,
            (
                "n".to_string(),
                0,
                1,
                vec![
                    "placement:n_body_a".to_string(),
                    "placement:n_body_b".to_string()
                ]
            ),
            "the fixture is not the world this arm claims to be protecting"
        );

        // ⛔ THE INJECTED FAILURE: the session runs under a content generation
        // this plan was not prepared against.
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            super::transaction::ActiveContentBinding::content(ambition_platformer2d_core::ContentEpoch(7), Default::default()),
        );

        stage_the_candidate(&mut app, candidate_plan(), outgoing.clone());

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            !verification.published,
            "the fixture did not actually reach a refusal, so nothing below is \
             about a failed candidate: {verification:?}"
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<
                    ambition_platformer2d_world::rooms::RoomLoaded,
                >>()
                .drain()
                .count(),
            0,
            "a refused room published `RoomLoaded`"
        );

        assert_eq!(
            live_world(&mut app),
            before,
            "⛔ THE LAST-GOOD-WORLD GUARANTEE IS BROKEN. A candidate that was \
             REFUSED changed the world the session is playing: geometry, active \
             room, platform state or the live roster moved. N must be \
             byte-identical to what it was before the attempt."
        );
        for entity in &outgoing {
            assert!(
                app.world().get_entity(*entity).is_ok(),
                "a refused candidate swept the OUTGOING room's bodies: the \
                 session is now playing a room with nothing in it"
            );
        }
        // ⛔ **AND THE CANDIDATE-OWNED STATE IS GONE TOO, which is the half a
        // resource could not be asked about.** The staged world is an entity
        // under this room's transaction, so `retire_candidate` takes it with
        // everything else the candidate made.
        assert_eq!(
            staged_worlds_alive(&mut app, &candidate_plan()),
            0,
            "a refused candidate left its staged world standing; a leak like that \
             is invisible to every assertion about the LIVE world"
        );
    }

    /// ⛔⛤ **A ROOM THAT WOULD SEAT THE SESSION OUT OF RANGE IS REFUSED, AND THE
    /// DEFECT IT CATCHES IS SILENT.**
    ///
    /// `RoomSet::set_active` is `self.active = index.min(len - 1)`. An
    /// out-of-range index does not panic — it CLAMPS, and the session wakes in
    /// the last room of the set wearing the geometry of the one it was told to
    /// build. ⭐ MEASURED BY ACCIDENT 2026-09-14: a poison written to test
    /// something else staged `usize::MAX` and moved the active room instead of
    /// failing.
    ///
    /// ⚠ The clamp itself is NOT changed here. It has callers outside this road
    /// and its own contract; what changes is that this road refuses to hand it a
    /// value it would have to clamp.
    #[test]
    fn a_room_that_would_seat_the_session_out_of_range_is_refused() {
        let platform = MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            64.0,
            10.0,
        );
        let (mut app, outgoing) = last_good_world(platform);
        let before = live_world(&mut app);

        // A plan whose target index is past the end of the live set of two.
        let mut plan = candidate_plan();
        plan.target_index = 7;
        stage_the_candidate(&mut app, plan, outgoing);

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            !verification.published,
            "a room that would be seated out of range PUBLISHED: {verification:?}"
        );
        assert_eq!(
            verification.staged_violations,
            vec![super::transaction::StagedWorldViolation::TargetRoomOutOfRange {
                target: 7,
                rooms: 2,
            }]
        );
        assert_eq!(
            live_world(&mut app),
            before,
            "the refusal still moved the live world"
        );
    }

    /// ⛔ **AND A SESSION ROOT WITH NO ROOM SET IS REFUSED — THE THIRD AND LAST
    /// ACCESSOR THAT COULD ANSWER `None`.**
    ///
    /// ⚠ **THE ONLY ONE OF THE THREE I CANNOT NAME A PRODUCTION ROAD TO**, and
    /// saying so is the point: a session root always carries a room set. It is
    /// checked because the alternative is a PARTIAL publication reported as
    /// success — the geometry and platform state written, the active room
    /// silently left where it was, a session colliding against one room while
    /// believing it is in another.
    #[test]
    fn a_session_root_with_no_room_set_is_refused() {
        use ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component;

        let mut app = bevy::prelude::App::new();
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        app.add_message::<ambition_platformer2d_actor_spawn::SpawnActorRequest>();
        app.insert_resource(ambition_platformer2d_world::collision::MovingPlatformSet(
            Vec::new(),
        ));
        // A root that carries geometry and NOTHING ELSE.
        insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(empty_spec("n").world.clone()),
        );
        // ⛔ THE PREMISE, both halves: there IS a root (or this arm is the
        // no-root one wearing a different name), and it carries no room set.
        assert!(
            ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(app.world())
                .is_some(),
            "the fixture built no session root at all"
        );
        assert!(
            ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(
                app.world()
            )
            .is_none(),
            "the fixture's root carries a room set, so there is nothing to refuse"
        );

        stage_the_candidate(&mut app, candidate_plan(), Vec::new());

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            !verification.published,
            "a room published a world into a root that cannot seat it: {verification:?}"
        );
        assert!(
            verification.staged_violations.contains(
                &super::transaction::StagedWorldViolation::NoRoomSetToPublishInto
            ),
            "got {:?}",
            verification.staged_violations
        );
    }

    /// ⛔ **AND A ROOM WITH AUTHORED PLATFORMS AND NOWHERE TO PUT THEM IS
    /// REFUSED TOO — THE LAST SILENT SKIP IN THE PUBLICATION.**
    ///
    /// `apply_world_replacement` writes the platform state through
    /// `get_resource_mut`, which answers `None` when the resource is absent. A
    /// room with authored moving platforms would publish, report `room-loaded`,
    /// and leave the world without them — a room the player falls through.
    ///
    /// ⚠ **ONLY WHEN THERE IS SOMETHING TO PUBLISH.** The candidate room authors
    /// one platform, which is what makes this arm's subject exist; a room that
    /// states an empty vector means it, and a composition that has never needed
    /// the resource is not wrong for lacking one. The success arm above runs in a
    /// world that HAS the resource and proves the check is not simply always on.
    #[test]
    fn a_room_with_authored_platforms_and_no_platform_state_is_refused() {
        let platform = MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            64.0,
            10.0,
        );
        let (mut app, outgoing) = last_good_world(platform);
        // ⛔ THE INJECTION: the composition loses the authority the room's
        // platform state would be published into.
        app.world_mut()
            .remove_resource::<ambition_platformer2d_world::collision::MovingPlatformSet>();
        assert!(
            !candidate_plan().platform_states().is_empty(),
            "the candidate room authors no platforms, so this arm's subject does \
             not exist"
        );

        stage_the_candidate(&mut app, candidate_plan(), outgoing);

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            !verification.published,
            "a room published its platforms into nothing and called it success: \
             {verification:?}"
        );
        assert!(
            verification.staged_violations.contains(
                &super::transaction::StagedWorldViolation::NoPlatformStateToPublishInto
            ),
            "got {:?}",
            verification.staged_violations
        );
    }

    /// ⛔⛤ **A ROOM THAT STAGES A WORLD WITH NOWHERE TO PUT IT IS REFUSED.**
    ///
    /// `apply_world_replacement` writes through `session_world_component_mut`,
    /// which answers `None` when no root is live. Without this check the room
    /// would PUBLISH, report `room-loaded`, and leave the geometry, the active
    /// room and the platform state exactly as they were — and a caller that
    /// staged a whole world and got nothing would have no way to tell that from
    /// success.
    ///
    /// ⚠ **REACHABLE BY ORDERING, NOT ONLY BY MISUSE.** Session activation queues
    /// its room build BEFORE it spawns the session root, which is exactly why
    /// activation commits through `spawn_contents` and stages no world at all.
    #[test]
    fn a_room_that_stages_a_world_with_no_session_root_is_refused() {
        let mut app = bevy::prelude::App::new();
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        app.add_message::<ambition_platformer2d_actor_spawn::SpawnActorRequest>();
        app.insert_resource(
            ambition_platformer2d_world::collision::MovingPlatformSet::default(),
        );
        // ⛔ THE PREMISE: no session root at all, which is the whole subject.
        assert!(
            ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(app.world())
                .is_none(),
            "the fixture has a session root, so this arm is about something else"
        );

        stage_the_candidate(&mut app, candidate_plan(), Vec::new());

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            !verification.published,
            "a room published a world into nothing and called it success: \
             {verification:?}"
        );
        assert!(
            verification.staged_violations.contains(
                &super::transaction::StagedWorldViolation::NoSessionRootToPublishInto
            ),
            "got {:?}",
            verification.staged_violations
        );
    }

    /// ⛔ **AND A ROOM WHOSE GEOMETRY IS NOT ITS OWN IS REFUSED.**
    ///
    /// The index and the geometry travel together from one plan, so this is a
    /// caller pairing a plan with an index into a different SET — which is
    /// exactly what the hot-reload road does, and the one place they can
    /// disagree. Publishing it seats the session in one room and collides it
    /// against another.
    #[test]
    fn a_room_whose_geometry_is_not_its_own_is_refused() {
        let platform = MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            64.0,
            10.0,
        );
        let (mut app, outgoing) = last_good_world(platform);
        let before = live_world(&mut app);

        // Index 0 is `n`; the plan still carries the candidate room's geometry.
        let mut plan = candidate_plan();
        plan.target_index = 0;
        stage_the_candidate(&mut app, plan, outgoing);

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(
            !verification.published,
            "a room published one room's index with another's geometry: {verification:?}"
        );
        assert!(
            verification.staged_violations.iter().any(|violation| matches!(
                violation,
                super::transaction::StagedWorldViolation::GeometryIsNotTheTargetRoom { .. }
            )),
            "got {:?}",
            verification.staged_violations
        );
        assert_eq!(
            live_world(&mut app),
            before,
            "the refusal still moved the live world"
        );
    }

    /// ⭐ **AND THE SUCCESS ARM, WHICH IS WHAT MAKES THE ONE ABOVE FALSIFIABLE.**
    /// The identical fixture, minus the stale binding: the candidate publishes,
    /// N's bodies are swept, and all four pieces of world state move together.
    #[test]
    fn an_admitted_candidate_room_replaces_the_playable_world_completely() {
        let platform = MovingPlatformState::from_authored(
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            64.0,
            10.0,
        );
        let (mut app, outgoing) = last_good_world(platform);
        stage_the_candidate(&mut app, candidate_plan(), outgoing.clone());

        let verification = app
            .world()
            .resource::<crate::features::LastConstructionVerification>()
            .clone();
        assert!(verification.published, "{verification:?}");
        assert_eq!(
            app.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<
                    ambition_platformer2d_world::rooms::RoomLoaded,
                >>()
                .drain()
                .count(),
            1,
            "an admitted room publishes `RoomLoaded` exactly once"
        );

        let (geometry, active, platforms, ids) = live_world(&mut app);
        assert_eq!(geometry, "candidate", "the geometry never became the candidate's");
        assert_eq!(active, 1, "the session is still pointed at the old room");
        assert_eq!(platforms, 1, "the candidate's platform state never published");
        assert_eq!(
            ids,
            vec!["placement:occupant".to_string()],
            "the published roster is not exactly the candidate's: N's bodies \
             survived their own retirement, or the candidate never became visible"
        );
        for entity in &outgoing {
            assert!(
                app.world().get_entity(*entity).is_err(),
                "the outgoing room was never swept, so two rooms' bodies are live"
            );
        }
        // ⛔ PUBLICATION ADOPTS THE STATE AND THEN DROPS THE CARRIER. A published
        // entity still holding a replacement that has already been applied is a
        // second copy of the world waiting for somebody to apply again.
        assert_eq!(
            staged_worlds_alive(&mut app, &candidate_plan()),
            0,
            "publication left the staged world standing after adopting it"
        );
    }

    #[test]
    fn commit_receipt_matches_the_prepared_root_roster() {
        let mut spec = empty_spec("receipt");
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "enemy-1",
                "enemy",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0)),
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                    "combatant",
                ),
            ));
        let plan = prepare(spec).expect("plan");
        let expected = plan.predicted_authoritative_ids().clone();
        let expected_id = plan.id().clone();

        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        // ⛔ THE COMPOSITION PRODUCTION BUILDS. Room construction builds
        // every root hidden, and `transaction::open` REFUSES a world that cannot
        // hide one rather than validating candidates in plain sight — so a bare
        // `App` here is a fixture that never reaches the subject.
        ambition_platformer2d_shared_tangle::construction::register_inactive_candidate_filter(
            app.world_mut(),
        );
        app.add_message::<ambition_platformer2d_actor_spawn::SpawnActorRequest>();
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(
                &mut commands,
                transaction::PublicationRetention::UntilTheVerdictIsRecorded,
            );
        }
        app.world_mut().flush();

        let receipt = app.world().resource::<LastRoomConstructionCommit>();
        assert_eq!(receipt.plan_id, expected_id);
        assert_eq!(receipt.room_id, "receipt");
        assert_eq!(receipt.authoritative_ids, expected);

        // The roster speaks the `SimId` namespace now (it is derived from the
        // construction plan, whose derived rows have no authored spelling). A
        // family-loop enemy's body only receives its `SimId` from `ensure_sim_id`
        // AFTER verification, so map its authored `FeatureId` through the same
        // `placement:` spelling the roster uses for authored roots.
        let actual = {
            let mut query = app
                .world_mut()
                .query::<&ambition_combat::components::FeatureId>();
            query
                .iter(app.world())
                .map(|feature| {
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement(&feature.0)
                        .to_string()
                })
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            actual, expected,
            "the committed authoritative roots must match the prepared roster",
        );
    }

    #[test]
    fn plan_rejects_a_same_id_room_spec_changed_after_preparation() {
        let plan = prepare(empty_spec("mutable")).expect("plan");
        let mut changed = empty_spec("mutable");
        changed.world.spawn.x += 1.0;
        assert!(plan.matches_room_spec(plan.spec()));
        assert!(!plan.matches_room_spec(&changed));
    }

    /// Prepare room `index` of a WORLD, against what that world remembers about
    /// its occurrences. The pair is one argument on purpose — see
    /// [`features::OccurrenceContinuity`].
    fn prepare_in_world(
        world: &[RoomSpec],
        index: usize,
        remembered: &ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences,
    ) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let mut construction =
            features::ActorConstructionContext::new(&recipes, &catalog, &sheets, ContentBinding::content_unstated(Default::default()))
                .with_prepared(fixture_cast());
        construction.continuity = Some(features::OccurrenceContinuity {
            remembered,
            world,
            // no checkpoint behind this planner: it prepares a spec from the
            // world's own records, and `None` is the honest answer for a
            // composition that has taken none.
            minted: None,
        });
        RoomConstructionPlan::prepare_spec(
            index,
            world[index].clone(),
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            construction,
        )
    }

    /// Where one planned row would put the occurrence it builds.
    fn planned_position(
        plan: &RoomConstructionPlan,
        sim_id: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> Option<ae::Vec2> {
        plan.features
            .construction()
            .entities()
            .iter()
            .find(|entity| entity.sim_id() == sim_id)
            .map(|entity| match entity.parameters() {
                crate::construction::ActorConstructionParams::GroundItem { spec, .. } => spec.pos,
                other => {
                    panic!("the planned row is not the ground item it was authored as: {other:?}")
                }
            })
    }

    /// A ROOM REBUILDS AN OCCURRENCE WHOSE RECORD LIVES NEXT DOOR — and the
    /// room that owns the record does NOT rebuild it. One row, both halves.
    ///
    /// This is room construction ceasing to be a pure function of one
    /// `RoomSpec`: what a room owes the world is its current RESIDENCY, derived
    /// from the world's definitions plus the authoritative disposition of every
    /// occurrence, and an occurrence carried out of the room that minted it and
    /// put down elsewhere belongs to the room it is lying in.
    #[test]
    fn a_room_reinstates_an_occurrence_whose_record_lives_next_door() {
        let mut home = empty_spec("blink_run");
        home.ground_items
            .push(ambition_platformer2d_world::rooms::GroundItemSpec {
                id: "axe".into(),
                name: "Axe".into(),
                held_item: "gun_sword".into(),
                pos: ae::Vec2::new(10.0, 20.0),
                half_extent: ae::Vec2::splat(8.0),
            });
        let world = vec![home, empty_spec("portal_bridge")];
        let axe = ambition_platformer2d_shared_tangle::sim_id::SimId::placement("axe");
        let left_at = ae::Vec2::new(300.0, 64.0);

        // ── NOBODY HAS TOUCHED IT: the home room authors it where it says ────
        // The baseline that makes the two claims below changes rather than
        // coincidences.
        let untouched = Default::default();
        let plan = prepare_in_world(&world, 0, &untouched).expect("home plan");
        assert_eq!(
            planned_position(&plan, &axe),
            Some(ae::Vec2::new(10.0, 20.0)),
            "an untouched record is authored at its own coordinates"
        );
        assert!(prepare_in_world(&world, 1, &untouched)
            .expect("neighbour plan")
            .predicted_authoritative_ids()
            .is_empty());

        // ── IT WAS CARRIED NEXT DOOR AND PUT DOWN ───────────────────────────
        let mut remembered =
            ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences::default();
        // ⚠ CARRIED, then put down — in that order, because that is the road.
        // The ledger refuses a placement for an id it does not already hold as
        // a live occurrence, so a fixture that jumps straight to `Placed` is
        // modelling a relocation that cannot happen.
        remembered.republish_custody([axe.clone()].into_iter().collect());
        assert!(
            remembered
                .republish_placements(
                    "portal_bridge",
                    [(axe.clone(), left_at)].into_iter().collect(),
                )
                .is_empty(),
            "an occurrence that passed through custody may be put down"
        );

        // HALF ONE: the room it is lying in builds it, at the position it was
        // left, from a record that room does not own.
        let away = prepare_in_world(&world, 1, &remembered).expect("destination plan");
        assert_eq!(
            planned_position(&away, &axe),
            Some(left_at),
            "'portal_bridge' owes the world this occurrence: it is lying there, \
             and the only record that can rebuild it belongs to 'blink_run'"
        );
        let row = away
            .features
            .construction()
            .entities()
            .iter()
            .find(|entity| entity.sim_id() == &axe)
            .expect("the row asserted above");
        assert!(
            matches!(
                row.origin(),
                ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { source, .. }
                    if source.as_str() == "blink_run"
            ),
            "and its PROVENANCE still names the room that authored it — it was \
             moved, not re-created somewhere else: {:?}",
            row.origin(),
        );

        // HALF TWO: the room that authors the record does not build it.
        let home_again = prepare_in_world(&world, 0, &remembered).expect("home plan");
        assert_eq!(
            planned_position(&home_again, &axe),
            None,
            "'blink_run' must not mint a second occurrence of a record whose \
             first one is lying in 'portal_bridge'"
        );
    }
}
