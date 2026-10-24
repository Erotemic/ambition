//! A committed crossing names the live room it arrived in, in the shipped
//! (rollback) composition, so its cover waits on that room's art (review
//! b2c36ae4, finding 7).
//!
//! The cover asked for every live room of the target's definition, and two
//! live rooms can be of one definition. The commit now records the subject's
//! live room on the transaction (`ActiveRoomTransitionLoad::arrived_in`), and
//! the cover waits on it alone. Which rooms it then picks is the unit test
//! `a_cover_waits_on_the_live_room_it_arrived_in`; this is the record.

use ambition_platformer2d::engine_core::AabbExt as _;
use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::input::{InputParticipant, Platformer2dInputActionMonolith};
use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::runtime::room_transition::{RoomTransitionLoadPhase, RoomTransitionLoadState};

use crate::shell_host_rendered::{rendered_app, settle};

/// Control: the transaction knows no room before its commit. At the commit
/// it names the live room the player stands in after the crossing.
#[test]
fn a_committed_crossing_names_the_live_room_it_arrived_in() {
    let mut app = rendered_app();
    settle(&mut app);
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    for _ in 0..60 {
        app.update();
    }
    // A windowless host cannot settle a room's asset barrier: take the
    // headless road, as `room_boundary_unclaimed_views` does.
    app.world_mut()
        .remove_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionAssetContributor>();

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
    let mut before_commit = Vec::new();
    let mut at_commit = None;
    for _ in 0..900 {
        app.update();
        let Some(active) = app.world().resource::<RoomTransitionLoadState>().active.clone() else {
            continue;
        };
        if active.phase == RoomTransitionLoadPhase::Committed {
            at_commit = Some(active.arrived_in);
            break;
        }
        before_commit.push(active.arrived_in);
    }
    interact.release(app.world_mut());
    assert!(!before_commit.is_empty(), "premise: the crossing opened a transaction before its commit");
    assert!(
        before_commit.iter().all(Option::is_none),
        "control: a transaction named a room before its commit: {before_commit:?}"
    );
    let arrived_in = at_commit.expect("premise: the crossing committed with its cover up");
    let player_room = {
        let world = app.world_mut();
        world
            .query_filtered::<&InRoomInstance, With<PrimaryPlayer>>()
            .single(world)
            .ok()
            .map(|room| room.0)
    };
    assert!(player_room.is_some(), "premise: the player stands in a live room");
    assert_eq!(arrived_in, player_room, "the committed transaction names a room the player did not arrive in");
}
