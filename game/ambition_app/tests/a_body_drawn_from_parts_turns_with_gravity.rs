//! A body drawn from its parts turns with the gravity of its room.
//!
//! The simulation turns a body to stand along its gravity (`ActorRoll`), and
//! `sync_visuals` writes that turn on the body's own transform. A body drawn
//! as one baked quad shows it, because the quad is drawn through that
//! transform. A body drawn directly from its parts draws nothing on its own
//! entity: its parts are under a separate owner, and the owner took only the
//! body's translation. So in a room with sideways gravity the collision box
//! lay along the wall and the player's art stood level (Jon, 2026-10-05).
//!
//! The placement of each part under a turned root is held in `ambition_render`
//! (`a_direct_part_lands_where_the_baked_frame_would_for_either_anchor`). This
//! arm holds that the shipped game's player is on that road and that the turn
//! reaches the owner of its parts.

use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::platformer::lifecycle::PlayerVisual;
use ambition_platformer2d::render::rendering::actors::rigged::{RiggedPresentation, RiggedPresentations};
use bevy::prelude::*;

/// The turn of a transform about the view axis, in radians.
fn turn_of(transform: &Transform) -> f32 {
    transform.rotation.to_euler(EulerRot::ZYX).0
}

#[test]
fn the_players_parts_turn_with_sideways_gravity() {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |_| {},
    );
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    let mut player = None;
    for _ in 0..1200 {
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PlayerVisual>>();
        player = players.iter(world).next();
        if player.is_some() {
            break;
        }
    }
    let player = player.expect("the Ambition route has a player");
    // The art binds, and the body settles on the floor.
    for _ in 0..60 {
        app.update();
    }
    let owner = *app
        .world()
        .resource::<RiggedPresentations>()
        .0
        .get(&player)
        .expect("premise: the shipped game draws its player from parts");
    assert!(
        turn_of(app.world().get::<Transform>(player).unwrap()).abs() < 0.01,
        "premise: the player stands level in normal gravity"
    );

    // Gravity toward +x, as the side arms of the symmetry room have it.
    {
        let world = app.world_mut();
        let room = ambition_platformer2d::session::sole_live_room_component::<
            ambition_platformer2d::world::rooms::LiveRoomInstance,
        >(world)
        .copied();
        world
            .resource_mut::<ambition_platformer2d::world::BaseGravity>()
            .turn(room, Vec2::new(1.0, 0.0));
    }
    // The body turns at a finite rate, and it falls while it turns. Each frame
    // on which it is turned and drawn directly is a sample.
    let mut samples = 0;
    for _ in 0..90 {
        app.update();
        let world = app.world();
        let root = *world.get::<Transform>(player).expect("the player has a transform");
        let presentation = world.get::<RiggedPresentation>(owner).expect("the owner of its parts");
        if turn_of(&root).abs() < 1.0 || presentation.impostor.is_some() {
            continue;
        }
        samples += 1;
        let parts = *world.get::<Transform>(owner).expect("the owner has a transform");
        assert!(
            root.rotation.angle_between(parts.rotation) < 1.0e-3,
            "the player is turned {:.3} rad and its parts are turned {:.3} rad: a body drawn from \
             its parts must turn with its gravity",
            turn_of(&root),
            turn_of(&parts),
        );
    }
    assert!(
        samples >= 10,
        "premise: the player was turned by sideways gravity and drawn directly on {samples} frames of 90"
    );
}
