//! Authored condition-gated lock walls.
//!
//! Each `LockWall` may author `gated_by`; this system evaluates it through the
//! shared [`ConditionCatalog`] rather than reading save data directly.
//!
//! ⭐ THE FIELD NAMES ITS OWN CONDITION. `gated_by` is an authored condition
//! LINE — `"inventory.holds axe"` — in exactly the form
//! [`CommandCatalog::prepare_line`](ambition_platformer2d_shared_tangle::authored_logic::CommandCatalog::prepare_line)
//! already documents for an authored field: *"a level author writes one string
//! and the number of arguments is the verb's business rather than the field's."*
//! ⛔ IT USED TO BE HARDCODED to `world.flag_set` with the field as its only
//! ARGUMENT, so the item/equipment gate family was published (`inventory.holds`,
//! `held.is_held`) and unreachable from a route: a wall could not ask it however
//! well the condition was written. Widening the field, not adding a condition,
//! is what made the other families reachable.
//!
//! ⚠ A BARE VALUE IS STILL A FLAG — `"bob_field_survey_received"` means
//! `world.flag_set bob_field_survey_received`, which is what both authored rows
//! in the shipped worlds say. The discriminator is SYNTACTIC and never repairs:
//! a first token that parses as a `domain.question` id names a condition, and
//! anything else is a flag name. ⇒ A flag id containing a `.` is therefore not
//! addressable in the bare form and must be written out in full.
//!
//! Evaluation is exclusive
//! because condition callbacks receive `&World`; if this grows to many independent
//! rules, evaluate conditions once and publish outcomes for parallel readers.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    ConditionCatalog, ConditionId, ConditionOutcome, PreparedCondition,
};

/// The block-name prefix a gated wall contributes under.
pub const GATED_LOCK_BLOCK_PREFIX: &str = "gated_lock:";

/// One authored wall that is waiting on something.
#[derive(Clone, Debug, PartialEq)]
pub struct GatedLockWall {
    pub id: String,
    pub gated_by: String,
    pub min: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
}

/// Every `LockWall` in `room` that authors a `gated_by`.
///
/// pure, and takes the ROOM rather than the world, so the selection policy
/// stays testable without an ECS. That separation is inherited from the content
/// system this replaces and was the good part of it.
pub fn authored_gated_lock_walls(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Vec<GatedLockWall> {
    room.lock_walls
        .iter()
        .filter_map(|wall| {
            // a `LockWall` with no `gated_by` is not this system's business —
            // it is either an encounter's (whose phase drives it) or inert.
            // Skipping silently is correct: the field is optional precisely so
            // the other consumers keep working.
            let gated_by = wall.gated_by.as_ref()?;
            let id = wall.id.trim();
            if id.is_empty() {
                return None;
            }
            Some(GatedLockWall {
                id: id.to_string(),
                gated_by: gated_by.clone(),
                min: ambition_platformer2d_core::Vec2::new(wall.min.x, wall.min.y),
                size: ambition_platformer2d_core::Vec2::new(wall.size.x, wall.size.y),
            })
        })
        .collect()
}

/// One cached wall and the question it asks.
///
/// ⭐⭐ THE QUESTION IS PREPARED ONCE, WITH THE WALL. `PreparedCondition` has no
/// public constructor, so holding one is a structural claim that the condition
/// the author named exists and takes exactly these arguments — made when the
/// room is cached rather than re-spelled and re-minted on every frame this wall
/// is on screen.
///
/// ⛔ `None` MEANS THE QUESTION COULD NOT BE PREPARED, and it is retried. A
/// provider can register after the first room is cached, so a permanent `None`
/// would be a wall that stands forever because of startup ORDER. The retry costs
/// one preparation per frame per unpreparable wall, which is the population that
/// is supposed to be empty.
#[derive(Clone, Debug)]
struct CachedWall {
    wall: GatedLockWall,
    question: Option<PreparedCondition>,
}

/// Per-frame cache — see the module header on why its three inputs are three.
#[derive(Resource, Default)]
pub struct GatedLockWallCache {
    room: Option<String>,
    walls: Vec<CachedWall>,
}

/// WHY each authored gated wall of the active room stands or does not — the
/// last verdict of its prepared question, keyed by wall id (M5: a standing wall
/// explains itself as structure, not as a log line).
///
/// Rebuilt from the verdicts on every tick the gate runs, so it is a DERIVED
/// read model, never an authority: nothing decides a wall from it, and a rewind
/// that does not restore it costs one tick of stale explanation.
#[derive(Resource, Default, Debug)]
pub struct GatedLockWallVerdicts {
    pub by_wall: std::collections::BTreeMap<String, ConditionOutcome>,
}

impl GatedLockWallVerdicts {
    /// The structured reason `wall` stands, if it stands for a reason a domain
    /// stated (`None` while it is open, or unanswerable/unpreparable).
    pub fn why_standing(
        &self,
        wall: &str,
    ) -> Option<&ambition_platformer2d_shared_tangle::authored_logic::WhyNot> {
        self.by_wall.get(wall).and_then(ConditionOutcome::why_not)
    }
}

/// Contribute a solid for every authored gated wall whose condition is not yet
/// satisfied.
///
/// `is_satisfied()` and not "not unsatisfied": an unanswerable condition
/// leaves the wall STANDING, which is the safe direction. A gate that opened
/// because nobody could answer its question would open in exactly the situations
/// where the world is least well understood.
/// The room-set query, built once and reused.
///
/// ⭐ `World::query_filtered` (cite-ok: bevy's World, not ours) BUILDS A FRESH `QueryState` on every call, which
/// re-matches every archetype in the world. In an exclusive system that runs
/// each frame, that is a per-frame archetype scan to read a single component
/// off one entity. `Local` is an `ExclusiveSystemParam`, so the state can just
/// live across frames; `QueryState::iter` updates archetypes itself, so a
/// cached state still sees entities that appeared since the last run.
type RoomSetQuery = bevy::ecs::query::QueryState<
    bevy::prelude::Ref<'static, ambition_platformer2d_world::rooms::RoomSet>,
    bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
>;

/// Retract the published verdicts rather than leaving the previous room's.
///
/// ⛔ `GatedLockWallVerdicts` IS A STATEMENT ABOUT THE ACTIVE ROOM, so every
/// road out of the sync owes it an answer. A room with no gated walls, a world
/// with no session root, a world with no catalog — each of those is "nothing
/// stands here", and each used to leave the last room's map published, which
/// reads to every consumer as walls that are still standing in a room that has
/// none. Retract by RESETTING, never by removing: a consumer that reads the
/// resource must keep reading it.
fn retract_gated_lock_wall_verdicts(world: &mut World) {
    let Some(mut published) = world.get_resource_mut::<GatedLockWallVerdicts>() else {
        // Never published in this world; absence and empty say the same thing,
        // and inserting one here would only make a resource nobody asked for.
        return;
    };
    if !published.by_wall.is_empty() {
        published.by_wall.clear();
    }
}

pub fn sync_authored_gated_lock_walls(
    world: &mut World,
    mut rooms: bevy::prelude::Local<Option<RoomSetQuery>>,
) {
    // the room set is a COMPONENT on the session root, not a resource — the `SessionWorldRef` a
    // normal system takes is a `Single<Ref<T>, With<SessionRoot>>`. An exclusive system has to
    // ask for it the long way.
    let rooms = rooms.get_or_insert_with(|| RoomSetQuery::new(world));

    // ⭐ THE ROOM'S WALLS ARE READ ONLY WHEN THE CACHE IS ACTUALLY BEING
    // REFRESHED. `authored_gated_lock_walls` allocates a `Vec<GatedLockWall>`
    // with a `String` and a condition id cloned per wall; computing it every
    // frame and then discarding it unless `rooms_changed || stale` defeated the
    // cache it feeds. Room identity and the change tick are cheap, so decide
    // first and pay for the walls second.
    let (active_room_id, rooms_changed) = {
        let Some(set) = rooms.iter(world).next() else {
            retract_gated_lock_wall_verdicts(world);
            return;
        };
        (set.active_spec().id.clone(), set.is_changed())
    };
    if world.get_resource::<ConditionCatalog>().is_none() {
        retract_gated_lock_wall_verdicts(world);
        return;
    }

    // ── refresh the cache if any of its inputs moved ─────────────────────────
    //
    // The walls come off the room set now, so THAT is what has to be watched: a hot reload that
    // rebuilds rooms under an unchanged room ID would otherwise leave a stale cache, which is
    // the exact case the old signal existed for. This caches the walls that EXIST, and whether
    // each stands is asked fresh every frame through its condition. So the cached value is a
    // pure function of (project, room) — which is also why it can be declared derived to
    // rollback rather than registered: neither input can change inside a rollback window, since
    // a room transition commits only on a confirmed frame.
    // ⛔ THE CATALOG IS AN INPUT TO THE CACHE, so a catalog that moved makes it
    // stale exactly as a changed room does. This is what the per-frame retry
    // below used to stand in for: a provider that publishes its condition AFTER
    // the first room was cached left every wall keyed on it unpreparable
    // forever, and re-preparing on every tick was the workaround. Rebuild once
    // on the edge instead — the question is asked at the same moment either way,
    // and the per-tick preparation road goes.
    let catalog_moved = world.is_resource_changed::<ConditionCatalog>();
    let stale = {
        let cache = world.get_resource::<GatedLockWallCache>();
        cache.is_none_or(|cache| cache.room.as_deref() != Some(active_room_id.as_str()))
    };
    if rooms_changed || stale || catalog_moved {
        let walls = {
            let Some(set) = rooms.iter(world).next() else {
                retract_gated_lock_wall_verdicts(world);
                return;
            };
            authored_gated_lock_walls(set.active_spec())
        };
        let catalog = world.resource::<ConditionCatalog>().clone();
        let prepared: Vec<CachedWall> = walls
            .into_iter()
            .map(|wall| CachedWall {
                question: prepare_question(&active_room_id, &catalog, &wall),
                wall,
            })
            .collect();
        let mut cache = world.get_resource_or_insert_with(GatedLockWallCache::default);
        cache.walls = prepared;
        cache.room = Some(active_room_id.clone());
    }

    // ⛔ A ROOM WITH NO GATED WALLS IS THE COMMON CASE, and everything below it
    // — two catalog clones, a cache clone, an overlay lookup — is work whose
    // only possible result is an empty `standing`. Leave before paying for it,
    // but RETRACT FIRST: skipping the publication below is not the same as
    // publishing nothing, and the difference is the previous room's verdicts
    // outliving the room they describe.
    if world
        .get_resource::<GatedLockWallCache>()
        .is_none_or(|cache| cache.walls.is_empty())
    {
        retract_gated_lock_wall_verdicts(world);
        return;
    }

    // ── ask, then contribute ─────────────────────────────────────────────────
    //
    // The catalog and the cache are cloned out because evaluating needs `&World`
    // and pushing needs `&mut` on the overlay; a borrow that spanned both would
    // not compile, and cloning a handful of rows once a frame is not the cost
    // worth contorting the code to avoid.
    let catalog = world.resource::<ConditionCatalog>().clone();
    let walls = world
        .get_resource::<GatedLockWallCache>()
        .map(|cache| cache.walls.clone())
        .unwrap_or_default();
    let mut verdicts = std::collections::BTreeMap::new();
    let standing: Vec<&GatedLockWall> = walls
        .iter()
        .filter(|cached| {
            // ⛔ AN UNPREPARABLE QUESTION LEAVES THE WALL STANDING, the same
            // direction an unanswerable one does, and for the same reason: a gate
            // that opened because nobody could ask its question would open in
            // exactly the situations where the world is least well understood.
            //
            // ⭐ NO RE-PREPARATION HERE. The question was prepared when the cache
            // was built, and the cache rebuilds when the catalog moves, so the
            // late-provider case the old per-frame retry existed for is handled
            // on that edge instead of by parsing on every tick for every wall.
            let Some(question) = cached.question.clone() else {
                verdicts.insert(
                    cached.wall.id.clone(),
                    ConditionOutcome::unanswerable("the wall's question could not be prepared"),
                );
                return true;
            };
            let verdict = catalog.ask(world, &question);
            let stands = !verdict.is_satisfied();
            verdicts.insert(cached.wall.id.clone(), verdict);
            stands
        })
        .map(|cached| &cached.wall)
        .collect();
    // Published whether or not anything stands: an open wall's verdict is the
    // answer to "why is it open". A room with no walls never reaches here — it
    // publishes the same empty map through `retract_gated_lock_wall_verdicts`.
    {
        let mut published = world.get_resource_or_insert_with(GatedLockWallVerdicts::default);
        if published.by_wall != verdicts {
            published.by_wall = verdicts;
        }
    }
    if standing.is_empty() {
        return;
    }
    let Some(mut overlay) = world.get_resource_mut::<
        ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay,
    >() else {
        return;
    };
    for wall in standing {
        overlay
            .gate_solids
            .push(ambition_platformer2d_core::Block::solid(
                format!("{GATED_LOCK_BLOCK_PREFIX}{}", wall.id),
                wall.min,
                wall.size,
            ));
    }
}

/// Prepare one wall's authored question, or `None` when the catalog cannot yet
/// answer for it.
///
/// ⚠ SILENT ON FAILURE, because the caller's answer to `None` is already the safe
/// one and a per-frame retry would make a warning here a per-frame warning. The
/// visible symptom of a permanently unpreparable wall is a wall that never opens,
/// which is the same symptom the unanswerable path has always had.
/// Prepare one wall's question, REPORTING the authored fault when it cannot be.
///
/// ⛔⛔ THE `.ok()` THIS REPLACES WAS A SILENT SOFT-LOCK. `prepare` returns a
/// `PreparationError` carrying the authored source and the reason — its own doc
/// says it keeps the source because *"a diagnostic an author cannot act on"* is
/// useless — and this threw all of it away. A misspelt `gated_by` therefore
/// produced a wall that stands FOREVER, in a room the player cannot finish, with
/// nothing written anywhere. That is the worst shape a failure can take here:
/// the level is wrong, the engine knows exactly why, and nobody is told.
///
/// The wall still stands (an unanswerable gate must not open — see the caller),
/// so this changes no behaviour. It changes whether anyone can find out.
/// ⭐⭐ THE ONE DECISION ABOUT WHAT AN AUTHORED `gated_by` VALUE MEANS.
///
/// Public because a SECOND surface needs it and must not restate it: a content
/// check that asks whether every authored gate in the shipped worlds can be
/// prepared has to make exactly this choice, and a copy of five lines in a test
/// would validate a rule the game had stopped applying. That is the defect
/// `prepare_authored_arg` records one crate over — two authorities on what an
/// authored value means, differing by a spelling accepted on one road and not
/// the other.
///
/// The rule: a first token shaped like `domain.question` names a whole condition
/// LINE and the rest are its arguments; anything else is a flag id for
/// `world.flag_set`. See [`names_its_own_condition`] for why that test is
/// syntactic and never repairs.
pub fn prepare_authored_gate(
    catalog: &ConditionCatalog,
    authored: &str,
) -> Result<PreparedCondition, ambition_platformer2d_shared_tangle::authored_logic::PreparationError>
{
    if names_its_own_condition(authored) {
        catalog.prepare_line(authored)
    } else {
        catalog.prepare(ConditionId::new("world", "flag_set"), &[authored])
    }
}

fn prepare_question(
    room: &str,
    catalog: &ConditionCatalog,
    wall: &GatedLockWall,
) -> Option<PreparedCondition> {
    let authored = wall.gated_by.as_str();
    match prepare_authored_gate(catalog, authored) {
        Ok(prepared) => Some(prepared),
        Err(error) => {
            // Room, wall and authored text: the three facts an author needs to
            // find the row in the level. `reason` is the substrate's own words
            // ("takes 2 arguments, got 1", an unknown id) and is quoted rather
            // than re-worded.
            bevy::log::error!(
                target: "ambition_platformer2d::gated_lock_walls",
                "room `{room}` wall `{}` is gated by `{}`, which cannot be prepared: {} \
                 (authored source `{}`). The wall STANDS until this is fixed — a gate \
                 whose question cannot be asked must not open. A `gated_by` whose first \
                 token is a `domain.question` id is read as a whole condition line; \
                 anything else is a flag name for `world.flag_set`.",
                wall.id,
                wall.gated_by,
                error.reason(),
                error.source(),
            );
            None
        }
    }
}

/// Does this authored value NAME its condition, or is it a bare flag?
///
/// ⛔ SYNTACTIC, AND IT NEVER REPAIRS — the same rule
/// [`ConditionId::parse`] holds itself to. A first token shaped like
/// `domain.question` names a condition and the rest of the line is its
/// arguments; anything else is a flag name. Deliberately NOT "is a condition
/// with this id published": an author who names a condition that does not exist
/// must get the catalog's diagnostic and a wall that stands, not a silent
/// demotion to a flag lookup that will never be satisfied either.
fn names_its_own_condition(authored: &str) -> bool {
    authored
        .split_whitespace()
        .next()
        .and_then(ConditionId::parse)
        .is_some()
}

#[cfg(test)]
mod tests;
