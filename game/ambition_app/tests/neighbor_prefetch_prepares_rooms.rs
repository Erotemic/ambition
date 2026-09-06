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
    let app = gameplay_after_startup();

    let (source, neighbours) = {
        let room_set = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(app.world())
        .expect("a direct-gameplay session installs one live room set");
        let source = room_set
            .rooms
            .get(room_set.active)
            .expect("the active room index names a room")
            .id
            .clone();
        let neighbours = room_set
            .neighboring_room_indices()
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
        .filter(|room| !prefetch.holds(room))
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
