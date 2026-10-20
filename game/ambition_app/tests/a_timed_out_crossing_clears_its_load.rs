//! A crossing that times out ends the load and the cover opened for it, on a
//! rollback host (review b2c36ae4, finding 2).
//!
//! The passage gives its body back when the crossing does not arrive in time,
//! and withdraws the crossing's intent. A rollback host may not read that
//! absence as the end of the load, because the absence can be speculative. So
//! the load stayed open, and its cover stayed on the screen, with no end. The
//! withdrawal is now rollback state with its own frame, and the host retires
//! the load when that frame is confirmed.

use ambition_platformer2d::engine_core::AabbExt as _;
use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

use ambition_platformer2d::actors::rooms::passage::ZonePassage;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::input::{InputParticipant, Platformer2dInputActionMonolith};
use ambition_platformer2d::load::{LoadCommand, LoadCoordinator, LoadWorkSpec};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;

use crate::shell_host_rendered::{rendered_app, settle};

/// Is the transition cover on screen? Matched on its debug name, as
/// `room_boundary_unclaimed_views::cover_is_up` does: the marker is private.
fn cover_is_up(app: &mut App) -> bool {
    let world = app.world_mut();
    world
        .query::<&Name>()
        .iter(world)
        .any(|name| name.as_str().starts_with("room transition cover"))
}

fn transaction(app: &App) -> Option<(u64, ambition_platformer2d::load::LoadId)> {
    app.world()
        .resource::<RoomTransitionLoadState>()
        .active
        .as_ref()
        .map(|active| (active.sequence, active.barrier.load_id.clone()))
}

fn in_passage(app: &mut App) -> bool {
    let world = app.world_mut();
    world
        .query_filtered::<(), (With<PrimaryPlayer>, With<ZonePassage>)>()
        .iter(world)
        .next()
        .is_some()
}

/// Control: the crossing opens a load and its cover goes up, and the body is
/// in its passage. The load cannot finish (a required work item never
/// completes). When the passage gives up, the load leaves the coordinator, the
/// transaction ends and the cover comes down.
#[test]
fn a_timed_out_crossing_clears_the_host_load_and_its_cover() {
    let mut app = rendered_app();
    settle(&mut app);
    crate::common::the_hub_intro_has_already_played(app.world_mut());
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    for _ in 0..60 {
        app.update();
    }
    assert!(
        app.world()
            .resource::<ambition_platformer2d::runtime::SimulationHost>()
            .is_rollback(),
        "premise: the shipped composition is a rollback host"
    );

    let interact = {
        let world = app.world_mut();
        world
            .query_filtered::<&InputMap<Platformer2dInputActionMonolith>, With<InputParticipant>>()
            .iter(world)
            .next()
            .and_then(|map| map.get_buttonlike(&Platformer2dInputActionMonolith::Interact))
            .and_then(|bindings| bindings.first().cloned())
            .expect("Interact has a binding")
    };
    // Stand in a door of the live room. Rollback is dormant in ordinary play
    // (zero check distance), so the write is not rewound.
    {
        let world = app.world_mut();
        let live = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
            .expect("the session has a live room");
        let door = world
            .query::<&ambition_platformer2d::world::rooms::RoomSet>()
            .iter(world)
            .next()
            .expect("a live room set")
            .spec(live)
            .loading_zones
            .iter()
            .find(|zone| zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door)
            .cloned()
            .expect("the start room authors a door");
        let mut player = world.query_filtered::<&mut ambition_platformer2d::actor::BodyKinematics, With<PrimaryPlayer>>();
        let mut kin = player.single_mut(world).expect("one primary player");
        kin.pos = door.aabb.center();
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    }

    interact.press(app.world_mut());
    let mut opened = None;
    for _ in 0..120 {
        app.update();
        if let Some(open) = transaction(&app) {
            opened = Some(open);
            break;
        }
    }
    interact.release(app.world_mut());
    let (sequence, load_id) = opened.expect("premise: the door crossing opens a room-transition load");
    // The load cannot finish: one more required item on its barrier, which
    // nothing completes.
    {
        let barrier = app
            .world()
            .resource::<RoomTransitionLoadState>()
            .active
            .as_ref()
            .map(|active| active.barrier.barrier_id.clone())
            .expect("the transaction is open");
        let mut loads = app.world_mut().resource_mut::<LoadCoordinator>();
        let _ = loads.apply(LoadCommand::UpsertWork {
            load_id: load_id.clone(),
            spec: LoadWorkSpec::required("test.never_ready", "A load that never finishes", barrier),
        });
    }

    let mut saw_cover = false;
    let mut passage_frames = 0;
    for _ in 0..600 {
        app.update();
        saw_cover |= cover_is_up(&mut app);
        if !in_passage(&mut app) {
            break;
        }
        passage_frames += 1;
    }
    assert!(saw_cover, "premise: the cover went up for the load");
    assert!(!in_passage(&mut app), "premise: the passage gave up within 600 frames");
    assert!(passage_frames > 120, "premise: the passage waited for the crossing ({passage_frames} frames)");

    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        transaction(&app).filter(|(open, _)| *open == sequence).map(|(open, _)| open),
        None,
        "the passage gave up its crossing, and the load it opened is still the active transaction"
    );
    assert!(
        !app.world().resource::<LoadCoordinator>().contains(&load_id),
        "the load `{load_id:?}` stayed in the coordinator after its crossing was withdrawn"
    );
    assert!(!cover_is_up(&mut app), "the cover stayed up after the crossing was withdrawn");
}
