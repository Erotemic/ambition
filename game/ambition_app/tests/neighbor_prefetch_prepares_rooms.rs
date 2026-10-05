//! THE NEIGHBOUR PREFETCH PREPARES ROOMS THE PLAYER CAN ACTUALLY WALK INTO.
//!
//! ```text
//! WARN could not prefetch construction for neighbor room 'basement_enemies':
//!   `placement:EnemySpawn-0140` names character `goblin`, which this composition
//!   has not registered
//! ```
//!
//! `goblin` is registered — it is in `character_catalog.ron`, and walking into
//! that room works. What the prefetch did not have was the PREPARED CAST: it
//! hand-built its `ActorConstructionContext` and never called `with_prepared`,
//! and `preflight_planned_bodies` treats an absent registry as an EMPTY one
//! ("not an exemption", as its own doc says). So every room containing a
//! character-built body failed preflight, was forgotten, and was re-prepared
//! from scratch on the very next frame — a full room plan per neighbour per
//! frame, thrown away, forever.
//!
//! Three sites hand-assembled the same context, two learned, and nobody counted the third. The
//! context is built in ONE place now, so the next authority cannot be added to two roads out of
//! three.

use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// Prefetch is a host system on the FEEL clock, so a few frames of settled
/// gameplay is all it takes — the assertion is about what it produced, not when.
fn gameplay_after_startup() -> bevy::prelude::App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, false);
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }
    app
}

#[test]
fn every_neighbour_of_the_starting_room_gets_a_prepared_plan() {
    let mut app = gameplay_after_startup();
    // The prefetch ranks a room's neighbours nearest door first from where
    // the players stand (`RoomSet::neighbors_nearest_first`).
    let standing = {
        let body = alice(&mut app);
        app.world().get::<ambition_platformer2d::actor::BodyKinematics>(body).expect("Alice has a body").pos
    };

    let (source, neighbours) = {
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(app.world())
            .expect("the session has a live room");
        let room_set = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .expect("a direct-gameplay session installs one live room set");
        let source = room_set
            .rooms
            .get(live_definition.index())
            .expect("the active room index names a room")
            .id
            .clone();
        let neighbours = room_set
            .neighbors_nearest_first(live_definition.index(), &[standing])
            .iter()
            .filter_map(|&index| room_set.rooms.get(index))
            .map(|room| room.id.clone())
            .collect::<Vec<_>>();
        (source, neighbours)
    };

    assert!(
        !neighbours.is_empty(),
        "the starting room '{source}' has no neighbours, so this test proves nothing \
         about prefetch — point it at a room with exits"
    );

    // The host caps how many neighbours it prepares; a room beyond the cap is an
    // ordinary miss and not a failure. Only what the host actually attempted is
    // asserted on.
    const NEIGHBOR_PREFETCH_ROOM_BUDGET: usize = 4;
    let attempted = &neighbours[..neighbours.len().min(NEIGHBOR_PREFETCH_ROOM_BUDGET)];

    let prefetch = app
        .world()
        .resource::<ambition_platformer2d::runtime::room_transition::RoomConstructionPlanPrefetch>(
    );
    let missing = attempted
        .iter()
        .filter(|room| !prefetch.holds(&source, room))
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "standing in '{source}', the neighbour prefetch prepared no plan for {missing:?} \
         (it attempted {attempted:?}). A refused room is re-prepared from scratch every \
         frame and never cached, so the transition into it takes the uncovered path AND \
         the host burns a full room plan per frame in the meantime. The cause is almost \
         always an authority the prefetch's construction context does not carry — the \
         prepared cast and the published brain profiles are the two that have gone \
         missing before."
    );
}

/// Once neighbour prefetch settles, keeping the cache warm must not prepare the
/// same neighbourhood every frame. Count preparations directly; promotion hit
/// rate alone cannot distinguish a retained entry from one rebuilt moments ago.
#[test]
fn a_settled_neighbourhood_stops_preparing_itself() {
    let mut app = gameplay_after_startup();
    // Let the neighbourhood finish its first pass, so what follows measures
    // steady state rather than the legitimate initial preparation.
    for _ in 0..30 {
        app.update();
    }

    let baseline = ambition_app::app::prefetch_preparations(app.world());
    const FRAMES: usize = 60;
    for _ in 0..FRAMES {
        app.update();
    }
    let after = ambition_app::app::prefetch_preparations(app.world());

    assert_eq!(
        after,
        baseline,
        "the prefetch performed {} room preparation(s) across {FRAMES} idle frames with \
         nothing in the world changing — it was zero when this was written. Each one is a \
         whole `RoomConstructionPlan` plus an asset manifest, thrown away and rebuilt; for \
         a neighbourhood containing the Hall that is an 18ms manifest EVERY FRAME, and the \
         hit rate would still look perfect. Suspect the refresh condition: `RoomSet` is \
         rollback-registered, so `is_changed()` under a rollback host can mean 'a restore \
         happened' rather than 'the content changed' — compare the room ids by value, as \
         `advance_room_transition_content_epoch_system` already does",
        after - baseline,
    );
}

/// ⛔⛔ **EVERY PREFETCHED PLAN REMEMBERS NOTHING, AND A CHECKPOINT RESTORE'S
/// SAFETY RESTS ON IT.**
///
/// The prefetch prepares each neighbour with NO occurrence continuity
/// (`ActorConstructionContext::for_live_room_construction(.., None)`), so every
/// cached plan carries the DEFAULT outlook. `RoomConstructionPlanPrefetch::
/// promote` then refuses any plan whose outlook differs from the one it is
/// handed. Together those two facts mean a cached plan can only ever be promoted
/// for a reconstruction whose outlook is ALSO empty — one that has nothing to say
/// about that room's population — which is exactly the reconstruction an
/// empty-outlook plan is correct for.
///
/// ⇒ That is the STRUCTURAL argument for the checkpoint protocol's *"same room,
/// different checkpoint occurrence outlook"* acceptance row, which asks that a
/// plan prepared against the live population cannot serve a reconstruction about
/// a different one. ⛔ **IT IS SUPPORTING EVIDENCE AND NOT THE CLOSE** —
/// `a_checkpoint_outlook_refuses_a_plan_prepared_without_one` below is the
/// witness. A structural close was accepted for the prepare-failure row of the
/// same packet and covered half of it.
///
/// ⚠ THIS TEST IS THE HALF THAT CAN ROT. `promote`'s refusal is one comparison
/// in one place; "the prefetch supplies no continuity" is a `None` in an
/// argument list that somebody could reasonably decide to fill in — to raise the
/// cache's hit rate, which the source already notes an object left anywhere
/// destroys. This fires first, and names the argument that dissolves.
#[test]
fn every_prefetched_plan_carries_an_empty_occurrence_outlook() {
    let app = gameplay_after_startup();
    let room_ids: Vec<String> = {
        let room_set = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .expect("a direct-gameplay session installs one live room set");
        room_set.rooms.iter().map(|room| room.id.clone()).collect()
    };
    let source = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("the session has a live room")
        .id
        .clone();
    let cache = app
        .world()
        .resource::<ambition_platformer2d::runtime::room_transition::RoomConstructionPlanPrefetch>(
        );
    let held: Vec<&String> = room_ids.iter().filter(|id| cache.holds(&source, id)).collect();

    // ⛔ THE PREMISE: something was actually prefetched, or the loop below is a
    // check that cannot fail.
    assert!(
        !held.is_empty(),
        "no plan was prefetched at all, so this test asserts nothing about what a \
         prefetched plan remembers"
    );
    for room in held {
        let plan = cache
            .peek(&source, room)
            .expect("the cache said it holds a plan for this room");
        assert!(
            plan.occurrence_outlook().is_empty(),
            "the prefetched plan for '{room}' remembers a population. A cached \
             plan that states dispositions can be promoted for a reconstruction \
             that shares them by coincidence — and a checkpoint restore's \
             destination would then be rebuilt from a plan prepared against the \
             LIVE world rather than against the checkpoint it is about"
        );
    }
}

/// ⛔⛔ **THE ACCEPTANCE ROW ITSELF, END TO END, AGAINST THE SHIPPED HOST.**
///
/// The checkpoint protocol's *"same room, different checkpoint occurrence
/// outlook"* row asks that a plan prepared against the LIVE population cannot be
/// promoted for a reconstruction that is about a different one. Until now that
/// was argued structurally — the prefetch supplies no continuity, so every cached
/// plan carries the default outlook, so `promote` can only ever hand one to a
/// reconstruction whose outlook is also empty. The argument is sound and it is
/// still recorded above. It is not a witness.
///
/// ⛔ **AND A STRUCTURAL CLOSE HAS ALREADY BEEN WRONG ONCE IN THIS PACKET.** The
/// prepare-failure row was closed on "nothing destructive runs before the
/// commit", which is true and covers half the row; terminalization and
/// no-permanent-retry were both broken behind it. That is the reason this one
/// gets a fixture.
///
/// ⭐ WHAT MAKES IT A WITNESS RATHER THAN A RESTATEMENT: it never calls
/// `promote`, never fabricates a cache identity, and asserts nothing about the
/// outlook comparison. It walks the shipped `begin_room_transition_load_system`
/// twice into the SAME room from the SAME cache, changing one thing — whether an
/// accepted checkpoint operation pins a population — and reads the host's own
/// `prefetch_hit`. The control arm is the anti-vacuity floor: without it, a cache
/// that never promotes anything would pass.
#[test]
fn a_checkpoint_outlook_refuses_a_plan_prepared_without_one() {
    let promoted_for_an_ordinary_crossing = cross_into_a_cached_neighbour(false);
    assert!(
        promoted_for_an_ordinary_crossing,
        "⛔ THE PREMISE FAILED, so the checkpoint arm below would pass against a \
         cache that promotes NOTHING. An ordinary crossing into a room the \
         prefetch holds must take the cached plan; if it does not, this fixture \
         cannot say whether the checkpoint arm's refusal is the outlook \
         comparison or a cache miss for some unrelated reason"
    );

    let promoted_for_a_checkpoint_restore = cross_into_a_cached_neighbour(true);
    assert!(
        !promoted_for_a_checkpoint_restore,
        "a room transition that IS a checkpoint restore promoted a plan the \
         prefetch prepared against the live population, while the operation's \
         pinned ledger says an occurrence is in custody. That plan authors the \
         object the checkpoint remembers in a hand, so committing it puts a \
         second copy in the room — or, with the ledger the other way round, \
         leaves the room permanently short of a thing it authors. The plan a \
         reconstruction uses must be prepared against the population that \
         reconstruction is about"
    );
}

/// Walk one crossing into a neighbour the prefetch already holds and report
/// whether the host promoted the cached plan.
///
/// `as_checkpoint_restore` accepts a checkpoint operation for the very same
/// crossing, whose pinned ledger remembers one occurrence in custody — the
/// smallest population a checkpoint can disagree with the live world about, and
/// enough to give every room a non-default outlook.
fn cross_into_a_cached_neighbour(as_checkpoint_restore: bool) -> bool {
    use ambition_platformer2d::actors::session::checkpoint::{
        AcceptedCheckpointRestore, AcceptedRestore, SessionCheckpointOperations,
    };
    use ambition_platformer2d::actors::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };
    use ambition_platformer2d::platformer::lifecycle::{
        AuthoredOccurrences, OccurrenceBaseline, OccurrenceWhereabouts,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::runtime::room_transition::{
        RoomConstructionPlanPrefetch, RoomTransitionLoadState,
    };

    let mut app = gameplay_after_startup();
    // Let the neighbourhood finish its first pass so the cache is warm for the
    // room this crossing targets.
    for _ in 0..30 {
        app.update();
    }

    let neighbour = {
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(app.world())
            .expect("the session has a live room");
        let room_set = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .expect("a direct-gameplay session installs one live room set");
        let held = app.world().resource::<RoomConstructionPlanPrefetch>();
        let source = &room_set.spec(live_definition).id;
        room_set
            .neighboring_room_indices_of(live_definition.index())
            .iter()
            .filter_map(|&index| room_set.rooms.get(index))
            .map(|room| room.id.clone())
            .find(|id| held.holds(source, id))
            .expect(
                "the prefetch holds no neighbour of the starting room, so there is \
                 no cached plan for a crossing to promote or refuse",
            )
    };

    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&SimId, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.single(world).expect("one primary player").clone()
    };
    // The subject's live identity: its id in the live room it is in.
    let subject = {
        let world = app.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &ambition_platformer2d::platformer::sim_id::SimId)>();
        let entity = q
            .iter(world)
            .find(|(_, id)| **id == subject)
            .map(|(entity, _)| entity)
            .expect("the subject is a live body");
        ambition_platformer2d::platformer::lifecycle::LiveBodyId::of_entity(world, entity).expect("the subject has a SimId")
    };
    let intent = RoomTransitionIntent {
        subject: subject.clone(),
        target_room: neighbour.clone(),
        arrival: ambition_platformer2d::engine_core::Vec2::ZERO,
        edge_exit: false,
        zone_sfx: None,
        participant: None,
    };

    if as_checkpoint_restore {
        let mut pinned = AuthoredOccurrences::default();
        pinned.adopt_rows(
            [(
                SimId::placement("an_occurrence_the_checkpoint_remembers_in_a_hand"),
                OccurrenceWhereabouts::InCustody,
            )]
            .into_iter()
            .collect(),
        );
        let mut occurrences = OccurrenceBaseline::default();
        occurrences.adopt(pinned);

        let world = app.world_mut();
        let scope = world
            .get_resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
            .and_then(|scope| scope.current());
        let key = world
            .resource_mut::<SessionCheckpointOperations>()
            .admit(scope)
            .expect("a live session can still mint an operation key");
        world
            .resource_mut::<AcceptedCheckpointRestore>()
            .accept(AcceptedRestore {
                key,
                frame: 0,
                intent: LifecycleIntent::Transition(intent.clone()),
                lifecycle: Some(ambition_platformer2d::platformer::lifecycle::CheckpointRestoreInputs {
                    occurrences,
                    custody: Default::default(),
                }),
                item: None,
                fresh: false,
                replay: None,
            });
    }

    assert!(
        app.world_mut()
            .resource_mut::<PendingLifecycleCommit>()
            .record(0, LifecycleIntent::Transition(intent))
            .admitted(),
        "the lifecycle slot refused the staged crossing, so no transaction opens"
    );

    // `prefetch_hit` is decided when the transaction opens and lives on the load
    // record; read it while the load exists rather than after it retires.
    let mut observed = None;
    for _ in 0..120 {
        app.update();
        if let Some(active) = app.world().resource::<RoomTransitionLoadState>().active.as_ref() {
            observed = Some(active.prefetch_hit);
            break;
        }
    }
    observed.expect(
        "no room transition transaction opened for the staged crossing in 120 \
         frames, so nothing consulted the prefetch cache at all",
    )
}


/// ⛔ **A CHECKPOINT RESTORE WITH NO LIFECYCLE HALF LEAVES THE LIVE LEDGER IN
/// CHARGE OF THE ROOM IT REBUILDS.**
///
/// The occurrence ledger belongs to the held-item domain and exists without the
/// lifecycle horizon, so a restore that pinned `lifecycle: None` did not rewind
/// it. Room preparation once built such a restore's room from NO ledger, and an
/// authored occurrence the live ledger said was lying in another room was built
/// again in its authoring room: one identity, two places.
///
/// Two arms, one crossing each. Without the relocation the room's own ground
/// item is built, which proves the room authors it. With the relocation it is
/// not built.
#[test]
fn a_restore_with_no_lifecycle_half_rebuilds_its_room_from_the_live_ledger() {
    assert!(
        rebuilt_room_holds_its_ground_item(false),
        "the target room did not build its own ground item on an ordinary \
         checkpoint crossing, so the relocated arm below proves nothing",
    );
    assert!(
        !rebuilt_room_holds_its_ground_item(true),
        "the live ledger puts this ground item in another room, and a checkpoint \
         restore that pinned no lifecycle half built it again in its authoring \
         room: room preparation read no ledger instead of the live one",
    );
}

/// Cross into a room that authors a ground item, as a checkpoint restore whose
/// lifecycle half is `None`. With `relocated`, the LIVE ledger first says the
/// item lies in another room. Answers whether the rebuilt room holds the item.
fn rebuilt_room_holds_its_ground_item(relocated: bool) -> bool {
    use ambition_platformer2d::actors::session::checkpoint::{
        AcceptedCheckpointRestore, AcceptedRestore, SessionCheckpointOperations,
    };
    use ambition_platformer2d::actors::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };
    use ambition_platformer2d::platformer::lifecycle::{AuthoredOccurrences, OccurrenceWhereabouts};
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;

    let mut app = gameplay_after_startup();
    let (target, elsewhere, item) = {
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(app.world())
            .expect("the session has a live room");
        let room_set = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .expect("a direct-gameplay session installs one live room set");
        let active = room_set.spec(live_definition).id.clone();
        let room = room_set
            .rooms
            .iter()
            .find(|room| room.id != active && !room.ground_items.is_empty())
            .expect("no room other than the starting room authors a ground item");
        (
            room.id.clone(),
            active,
            SimId::placement(&room.ground_items[0].id),
        )
    };

    if relocated {
        let mut ledger = app.world_mut().resource_mut::<AuthoredOccurrences>();
        let mut rows: std::collections::BTreeMap<_, _> = ledger
            .rows()
            .map(|(id, whereabouts)| (id.clone(), whereabouts.clone()))
            .collect();
        rows.insert(
            item.clone(),
            OccurrenceWhereabouts::Placed {
                room: elsewhere,
                at: ambition_platformer2d::engine_core::Vec2::ZERO,
            },
        );
        ledger.adopt_rows(rows);
    }

    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&SimId, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.single(world).expect("one primary player").clone()
    };
    // The subject's live identity: its id in the live room it is in.
    let subject = {
        let world = app.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &ambition_platformer2d::platformer::sim_id::SimId)>();
        let entity = q
            .iter(world)
            .find(|(_, id)| **id == subject)
            .map(|(entity, _)| entity)
            .expect("the subject is a live body");
        ambition_platformer2d::platformer::lifecycle::LiveBodyId::of_entity(world, entity).expect("the subject has a SimId")
    };
    let intent = LifecycleIntent::Transition(RoomTransitionIntent {
        subject: subject.clone(),
        target_room: target.clone(),
        arrival: ambition_platformer2d::engine_core::Vec2::ZERO,
        edge_exit: false,
        zone_sfx: None,
        participant: None,
    });
    {
        let world = app.world_mut();
        let scope = world
            .get_resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
            .and_then(|scope| scope.current());
        let key = world
            .resource_mut::<SessionCheckpointOperations>()
            .admit(scope)
            .expect("a live session can still mint an operation key");
        world
            .resource_mut::<AcceptedCheckpointRestore>()
            .accept(AcceptedRestore {
                key,
                frame: 0,
                intent: intent.clone(),
                lifecycle: None,
                item: None,
                fresh: false,
                replay: None,
            });
    }
    assert!(
        app.world_mut()
            .resource_mut::<PendingLifecycleCommit>()
            .record(0, intent)
            .admitted(),
        "the lifecycle slot refused the staged crossing, so no transaction opens"
    );

    let mut opened = false;
    for _ in 0..240 {
        app.update();
        let loading = app
            .world()
            .resource::<RoomTransitionLoadState>()
            .active
            .is_some();
        opened |= loading;
        let arrived = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
            .is_some_and(|spec| spec.id == target);
        if opened && arrived && !loading {
            let world = app.world_mut();
            let mut ids = world.query::<&SimId>();
            return ids.iter(world).any(|id| *id == item);
        }
    }
    panic!("the checkpoint crossing into '{target}' did not complete in 240 frames");
}

// ───────────────────────────────────────────────────────────────────────────
// Two live rooms.
// ───────────────────────────────────────────────────────────────────────────

use ambition_platformer2d::characters::control::PlayerSlot;
type LiveBodyId = ambition_platformer2d::platformer::lifecycle::LiveBodyId;

const BOB: &str = "bob";

/// The room definitions that are live, by room id.
pub(crate) fn live_room_ids(app: &mut bevy::prelude::App) -> Vec<String> {
    use ambition_platformer2d::platformer::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    let world = app.world_mut();
    let definitions: Vec<_> = world
        .query_filtered::<
            (&LiveRoomInstance, &ambition_platformer2d::world::rooms::LiveRoomDefinition),
            bevy::prelude::With<RoomInstanceRoot>,
        >()
        .iter(world)
        .map(|(live, definition)| (*live, *definition))
        .collect();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    let mut named: Vec<_> = definitions
        .into_iter()
        .map(|(live, definition)| (live, rooms.spec(definition).id.clone()))
        .collect();
    named.sort();
    named.into_iter().map(|(_, id)| id).collect()
}

/// The room ids next to `room`, in the order of the room graph.
pub(crate) fn neighbours_of(app: &bevy::prelude::App, room: &str) -> Vec<String> {
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(app.world())
    .expect("the session keeps its room set");
    let index = rooms
        .rooms
        .iter()
        .position(|spec| spec.id == room)
        .unwrap_or_else(|| panic!("the room set has no room `{room}`"));
    rooms
        .neighboring_room_indices_of(index)
        .iter()
        .filter_map(|&index| rooms.rooms.get(index))
        .map(|spec| spec.id.clone())
        .collect()
}

pub(crate) fn alice(app: &mut bevy::prelude::App) -> bevy::prelude::Entity {
    let world = app.world_mut();
    world
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("one primary player")
}

pub(crate) fn bob(app: &mut bevy::prelude::App) -> Option<bevy::prelude::Entity> {
    let world = app.world_mut();
    world
        .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
        .iter(world)
        .find(|(_, feature)| feature.0 == BOB)
        .map(|(entity, _)| entity)
}

/// The room id of the live room `body` is in.
pub(crate) fn room_of(app: &bevy::prelude::App, body: bevy::prelude::Entity) -> Option<String> {
    ambition_platformer2d::world::rooms::live_room_spec_of(app.world(), body).map(|spec| spec.id.clone())
}

/// Stage the crossing of `body` to `target` and run it to its end. Answers
/// whether the transaction promoted a prefetched plan.
pub(crate) fn cross(
    app: &mut bevy::prelude::App,
    body: bevy::prelude::Entity,
    slot: PlayerSlot,
    target: &str,
) -> bool {
    use ambition_platformer2d::actors::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };
    use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;

    let subject = LiveBodyId::of_entity(app.world(), body).expect("the body has a SimId");
    let intent = LifecycleIntent::Transition(RoomTransitionIntent {
        subject,
        target_room: target.to_owned(),
        arrival: ambition_platformer2d::engine_core::Vec2::new(200.0, 200.0),
        edge_exit: false,
        zone_sfx: None,
        participant: Some(slot),
    });
    assert!(
        app.world_mut()
            .resource_mut::<PendingLifecycleCommit>()
            .record(0, intent)
            .admitted(),
        "the lifecycle slot refused the crossing to `{target}`"
    );
    let mut hit = None;
    for _ in 0..600 {
        app.update();
        let active = app.world().resource::<RoomTransitionLoadState>().active.as_ref();
        match (active, hit) {
            (Some(active), _) => hit = Some(active.prefetch_hit),
            (None, Some(hit)) if room_of(app, body).as_deref() == Some(target) => return hit,
            _ => {}
        }
    }
    panic!("the crossing to `{target}` did not end in 600 frames (hit so far: {hit:?})");
}

/// A presentation host with Bob, driven by slot 1, beside Alice in the start
/// room. Returns the start room id.
fn host_with_bob_beside_alice(start_room: &str) -> (bevy::prelude::App, String) {
    let mut app = ambition_app::app::build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(ambition_app::app::StartRoomOverride(start_room.to_owned()));
        app.insert_resource(ambition_app::app::StartRoomMustResolve);
    });
    for _ in 0..ambition_app::app::shared_host_startup_ticks() + 30 {
        app.update();
    }
    let start = put_bob_beside_alice(&mut app);
    (app, start)
}

/// Put Bob, driven by slot 1, beside Alice in the one live room. Returns the
/// id of that room.
pub(crate) fn put_bob_beside_alice(app: &mut bevy::prelude::App) -> String {
    use ambition_platformer2d::actor::{ActorFaction, SpawnActorKind, SpawnActorRequest};
    use ambition_platformer2d::character::{CharacterBrain, CharacterId};

    let alice = alice(app);
    let start = room_of(app, alice).expect("Alice is in a live room");
    let at = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(alice)
        .expect("Alice has a body")
        .pos;
    app.world_mut().write_message(SpawnActorRequest {
        id: BOB.to_owned(),
        name: "Bob".to_owned(),
        pos: ambition_platformer2d::engine_core::Vec2::new(at.x + 40.0, at.y),
        half_size: ambition_platformer2d::engine_core::Vec2::new(12.0, 16.0),
        faction: ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: CharacterBrain::Passive,
            character: CharacterId::from("npc_puppy_slug"),
        },
    });
    for _ in 0..8 {
        app.update();
    }
    let bob = bob(app).expect("Bob's body reached the world");
    let room = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    >(app.world_mut())
    .expect("the session has one live room");
    app.world_mut().entity_mut(bob).insert((
        ambition_platformer2d::platformer::lifecycle::InRoomInstance(room),
        ambition_platformer2d::characters::control::DrivingParticipant(PlayerSlot(1)),
    ));
    for _ in 0..8 {
        app.update();
    }
    start
}

/// The rooms of the two-room walk, chosen from the room graph.
///
/// Two live rooms share the budget in turn, so the first two neighbours of
/// each are always prepared (a neighbour that is live is not counted, and the
/// walk does not cross into one). Alice goes from `start` to `alice_first` and then
/// to `alice_second`. Bob goes from `start` to `bob_target`.
struct TwoRoomWalk {
    alice_first: String,
    alice_second: String,
    bob_target: String,
}

impl TwoRoomWalk {
    fn from(app: &bevy::prelude::App, start: &str) -> Self {
        let first_two = |room: &str| -> Vec<String> {
            neighbours_of(app, room).into_iter().take(2).collect()
        };
        let from_start = first_two(start);
        from_start
            .iter()
            .find_map(|alice_first| {
                let bob_target = from_start.iter().find(|room| *room != alice_first)?;
                let alice_second = first_two(alice_first)
                    .into_iter()
                    .find(|room| room != start && room != bob_target)?;
                Some(Self {
                    alice_first: alice_first.clone(),
                    alice_second,
                    bob_target: bob_target.clone(),
                })
            })
            .unwrap_or_else(|| {
                panic!(
                    "the first two neighbours of `{start}` ({from_start:?}) do not give the walk \
                     its rooms: one of them must have, in its own first two, a room that is \
                     not `{start}` and not the other"
                )
            })
    }
}

/// ⭐ EACH LIVE ROOM KEEPS THE PLANS OF ITS OWN NEIGHBOURS.
///
/// Alice and Bob start in one room and Alice leaves, so two rooms are live.
/// A crossing from either room, in either order, promotes a prefetched plan,
/// and the crossing of one body does not remove the plans of the other body's
/// room.
///
/// Before, the caches held ONE source room and the producer read the sole
/// live room. With two live rooms it did not run. Measured then: Alice's
/// second crossing missed in both orders, and Bob's crossing hit only when it
/// came first, from the plans that were left from the time of one room.
#[test]
fn each_live_room_keeps_the_plans_of_its_own_neighbours() {
    const START: &str = "drain_alley";
    const SETTLE: usize = 60;

    #[derive(Clone, Copy, Debug)]
    enum Order {
        BobThenAlice,
        AliceThenBob,
    }

    let mut wrong: Vec<String> = Vec::new();
    for order in [Order::BobThenAlice, Order::AliceThenBob] {
        let (mut app, start) = host_with_bob_beside_alice(START);
        assert_eq!(start, START, "the host did not start in the room that was asked for");
        let walk = TwoRoomWalk::from(&app, &start);

        // ⛔ THE PREMISE: one live room, and the crossing out of it hits. If
        // this is a miss, no later reading is about two rooms.
        let alice_body = alice(&mut app);
        assert!(
            cross(&mut app, alice_body, PlayerSlot(0), &walk.alice_first),
            "{order:?}: with one live room, the crossing to `{}` was not a prefetch hit",
            walk.alice_first
        );
        assert_eq!(
            live_room_ids(&mut app),
            vec![start.clone(), walk.alice_first.clone()],
            "{order:?}: Alice's crossing did not leave two live rooms"
        );
        for _ in 0..SETTLE {
            app.update();
        }

        // Whether the plan cache held the plan of a crossing before it began.
        let held = |app: &bevy::prelude::App, source: &str, target: &str| {
            app.world()
                .resource::<ambition_platformer2d::runtime::room_transition::RoomConstructionPlanPrefetch>()
                .holds(source, target)
        };
        let alice_crosses = |app: &mut bevy::prelude::App, wrong: &mut Vec<String>| {
            let body = alice(app);
            let held = held(app, &walk.alice_first, &walk.alice_second);
            if !cross(app, body, PlayerSlot(0), &walk.alice_second) {
                wrong.push(format!(
                    "{order:?}: Alice's crossing `{}` -> `{}` was not a prefetch hit \
                     (the cache held its plan before it began: {held})",
                    walk.alice_first, walk.alice_second
                ));
            }
        };
        let bob_crosses = |app: &mut bevy::prelude::App, wrong: &mut Vec<String>| {
            let body = bob(app).expect("Bob is in the world");
            assert_eq!(
                room_of(app, body).as_deref(),
                Some(start.as_str()),
                "{order:?}: Bob is not in the start room before his crossing"
            );
            let held = held(app, &start, &walk.bob_target);
            if !cross(app, body, PlayerSlot(1), &walk.bob_target) {
                wrong.push(format!(
                    "{order:?}: Bob's crossing `{start}` -> `{}` was not a prefetch hit \
                     (the cache held its plan before it began: {held})",
                    walk.bob_target
                ));
            }
        };
        match order {
            Order::BobThenAlice => {
                bob_crosses(&mut app, &mut wrong);
                for _ in 0..SETTLE {
                    app.update();
                }
                alice_crosses(&mut app, &mut wrong);
            }
            Order::AliceThenBob => {
                alice_crosses(&mut app, &mut wrong);
                for _ in 0..SETTLE {
                    app.update();
                }
                bob_crosses(&mut app, &mut wrong);
            }
        }
        // The start room is not live now, so its plans are retired. A crossing
        // does not clear them: the producer does.
        for _ in 0..SETTLE {
            app.update();
        }
        let stale: Vec<String> = neighbours_of(&app, &start)
            .into_iter()
            .filter(|room| held(&app, &start, room))
            .collect();
        if !stale.is_empty() {
            wrong.push(format!(
                "{order:?}: `{start}` is not live, and the cache still holds its plans for {stale:?}"
            ));
        }
        // Each body is in the room it crossed to, and both rooms are live.
        let mut expected = vec![walk.alice_second.clone(), walk.bob_target.clone()];
        expected.sort();
        let mut live = live_room_ids(&mut app);
        live.sort();
        assert_eq!(live, expected, "{order:?}: the two crossings did not end in two live rooms");
    }
    assert!(
        wrong.is_empty(),
        "with two live rooms, the prefetch did not keep the plans of each live room and no others:\n  {}",
        wrong.join("\n  ")
    );
}

/// ⭐ A NEIGHBOUR THAT IS LIVE GETS NO PLAN, AND USES NO BUDGET.
///
/// Alice and Bob are in two live rooms that are neighbours of each other. A
/// crossing into a live room joins it and builds no room, so a plan for it is
/// work that nothing uses. The budget goes to the rooms that are not live.
///
/// Before, the first neighbour of each live room was the other live room, so
/// two of the four rooms of the budget were live rooms.
#[test]
fn a_live_neighbour_gets_no_prefetched_plan() {
    const START: &str = "drain_alley";
    const BUDGET: usize = 4;

    let (mut app, start) = host_with_bob_beside_alice(START);
    let walk = TwoRoomWalk::from(&app, &start);
    let alice_body = alice(&mut app);
    assert!(
        cross(&mut app, alice_body, PlayerSlot(0), &walk.alice_first),
        "with one live room, the crossing to `{}` was not a prefetch hit",
        walk.alice_first
    );
    let live = vec![start.clone(), walk.alice_first.clone()];
    assert_eq!(live_room_ids(&mut app), live, "Alice's crossing did not leave two live rooms");
    // ⛔ THE PREMISE: each live room is a neighbour of the other. If not, no
    // live room is a candidate and the readings below are true of any build.
    assert!(
        neighbours_of(&app, &start).contains(&walk.alice_first)
            && neighbours_of(&app, &walk.alice_first).contains(&start),
        "the two live rooms are not neighbours of each other"
    );
    for _ in 0..60 {
        app.update();
    }

    let all_rooms: Vec<String> = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(app.world())
    .expect("the session keeps its room set")
    .rooms
    .iter()
    .map(|room| room.id.clone())
    .collect();
    let held: Vec<(String, String)> = {
        let cache = app
            .world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomConstructionPlanPrefetch>();
        live.iter()
            .flat_map(|source| {
                all_rooms
                    .iter()
                    .filter(|target| cache.holds(source, target))
                    .map(move |target| (source.clone(), target.clone()))
            })
            .collect()
    };
    let mut wrong: Vec<String> = Vec::new();
    let to_live: Vec<&(String, String)> =
        held.iter().filter(|(_, target)| live.contains(target)).collect();
    if !to_live.is_empty() {
        wrong.push(format!("the cache holds plans for rooms that are live: {to_live:?}"));
    }
    // The rooms that are not live and are a neighbour of a live room.
    let candidates: std::collections::BTreeSet<String> = live
        .iter()
        .flat_map(|room| neighbours_of(&app, room))
        .filter(|room| !live.contains(room))
        .collect();
    assert!(
        candidates.len() >= BUDGET,
        "the live rooms have {} neighbours that are not live, so the budget of {BUDGET} is not tested",
        candidates.len()
    );
    let prepared: std::collections::BTreeSet<&String> = held
        .iter()
        .map(|(_, target)| target)
        .filter(|target| !live.contains(target))
        .collect();
    if prepared.len() != BUDGET {
        wrong.push(format!(
            "{} rooms that are not live have a plan, and the budget is {BUDGET}: {prepared:?}",
            prepared.len()
        ));
    }

    // Bob joins Alice's room. No plan was prepared for it, and the crossing
    // does not need one.
    let bob_body = bob(&mut app).expect("Bob is in the world");
    if cross(&mut app, bob_body, PlayerSlot(1), &walk.alice_first) {
        wrong.push(format!(
            "Bob's crossing into the live room `{}` promoted a prefetched plan",
            walk.alice_first
        ));
    }
    assert_eq!(
        (live_room_ids(&mut app), room_of(&app, bob_body)),
        (vec![walk.alice_first.clone()], Some(walk.alice_first.clone())),
        "Bob's crossing did not join Alice's room and leave it the one live room"
    );
    assert!(wrong.is_empty(), "the prefetch spent work on a live room:\n  {}", wrong.join("\n  "));
}
