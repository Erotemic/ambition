//! D254/R17: the Author's Revision is ONE teleport, so it is one blink.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ ONE TELEPORT, ONE CUE.
///
/// ⛔⛔ IT WAS TWO, AND BOTH ROADS WERE RIGHT ON THEIR OWN.
/// `apply_authored_teleports` emits `player.blink` at the transit for EVERY
/// authored teleport — that is the executor being the one authority, which is
/// what it already is for every other teleport in the game. The Author's
/// up-B ALSO carried a `player.blink` on its own move timeline at the same
/// instant, so the same frame asked for the same cue down two roads (GPT 5.6,
/// 2026-08-27). The authored one is gone; nothing counted the result.
///
/// ⛔ COUNTED, NOT "AT LEAST ONE". A duplicate is exactly what "at least one"
/// cannot see, and it is the only failure this arm exists for.
///
/// ⚠ THE OTHER `player.blink` AUTHORSHIPS ARE NOT THIS. The Actor's trap and
/// wire and Alice's side-B author the cue for moves that never run the teleport
/// executor; those are the cue being CHOSEN, not duplicated.
#[test]
fn the_authors_revision_asks_for_exactly_one_blink() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
    use bevy::ecs::message::{MessageCursor, Messages};
    use bevy::prelude::*;

    let blink = ambition_platformer2d::sfx::SfxId::from_static("player.blink");

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster(["author", "author"]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 0 && held == 0 {
            break;
        }
    }
    let author = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    // ⛔ THE CUE IS COUNTED FOR THIS BODY ONLY. Seat 1 is an Author too and
    // teleports on its own schedule; counting every blink on the stage would
    // measure the CPU.
    let mut cursor = MessageCursor::<OwnedSfxMessage>::default();
    let drain = |app: &mut App, cursor: &mut MessageCursor<OwnedSfxMessage>| -> usize {
        let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
        cursor
            .read(messages)
            .filter(|owned| match &owned.request {
                SfxMessage::Play { id, .. } => *id == blink,
                _ => false,
            })
            .count()
    };
    // Discard whatever the opening produced so the count below is the move's.
    let _ = drain(&mut app, &mut cursor);

    let up_special = ambition_platformer2d::engine_core::ControlFrame {
        axis_y: -1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    let before = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(author)
        .map(|kin| kin.pos)
        .expect("the author has kinematics");
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), up_special);
    app.update();
    let mut blinks = drain(&mut app, &mut cursor);
    let mut moved = 0.0_f32;
    for _ in 0..60 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                special_pressed: false,
                ..up_special
            },
        );
        app.update();
        blinks += drain(&mut app, &mut cursor);
        moved = moved.max(
            app.world()
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(author)
                .map_or(0.0, |kin| kin.pos.distance(before)),
        );
    }

    // ⛔ THE PREMISE: he actually teleported. A count of one over a move that
    // never fired is a count of zero dressed up.
    assert!(
        moved > 120.0,
        "the author moved {moved:.0}px, which is not a 250px teleport — so the \
         cue count below is about a move that did not happen"
    );
    assert_eq!(
        blinks, 1,
        "the author's Revision asked for {blinks} blink cues. The teleport \
         executor emits one for every authored teleport; a move that authors \
         its own on the same instant makes it two, and two of one sound on one \
         frame is a flam nobody wrote"
    );
}
