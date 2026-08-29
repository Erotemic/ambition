//! D255/R17: the Author's Revision is ONE teleport, so it is one blink.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ ONE TELEPORT, ONE CUE.
///
/// ⛔⛔ IT WAS TWO, AND BOTH ROADS WERE RIGHT ON THEIR OWN.
/// `apply_authored_teleports` emits `player.blink` at the transit for EVERY
/// authored teleport — that is the executor being the one authority, which is
/// what it already is for every other teleport in the game. The Author's
/// up-B ALSO carried a `player.blink` on its own move timeline at the same
/// instant, so the same frame asked for the same cue down two roads
///. The authored one is gone; nothing counted the result.
///
/// ⛔ COUNTED, NOT "AT LEAST ONE". A duplicate is exactly what "at least one"
/// cannot see, and it is the only failure this arm exists for.
///
/// ⚠ THE OTHER `player.blink` AUTHORSHIPS ARE NOT THIS, and the test is whether
/// the move RUNS THE EXECUTOR. The Actor's trap (`author_trapdoor`) and Alice's
/// side-B (an `impulse`) never do, so their cue is chosen rather than
/// duplicated. ⛔ naming a move here without checking that is how a second
/// duplicate hid inside this exemption once already — her wire IS an
/// `author_teleport`, and was on this list.
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

/// A TELEPORT RECOVERY IS INTANGIBLE WHILE THE FIGHTER IS NOWHERE — AND NOT
/// AFTER.
///
/// ⭐⭐ THE MECHANISM WAS SHIPPED AND NOBODY AUTHORED ONE. `WindowTag::Invuln`
/// becomes `Invulnerability::MOVE` in `project_move_defense_windows`, and until
/// 2026-08-28 exactly one move in the game authored a window: the Actress's
/// trapdoor, where being underground is the reason. A teleport has the other
/// one — the fighter is off-stage, committed, and for a few frames has no honest
/// position at all, so the frame that decides the stock is a coin flip.
///
/// ⛔ BOTH ARMS, BECAUSE ONLY THE PAIR IS THE RULE. "It goes intangible" alone
/// is satisfied by a window that never closes, which would hand back the
/// commitment a recovery is supposed to cost. The tail after the window is what
/// an edgeguarder who reads the move still wins.
#[test]
fn the_authors_revision_is_intangible_through_the_vanish_and_not_through_the_landing() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
    use bevy::prelude::*;

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
    let held_by_move = |app: &App| -> bool {
        app.world()
            .get::<BodyHealth>(author)
            .is_some_and(|h| h.health.invulnerable.holds(Invulnerability::MOVE))
    };
    // ⛔ THE FLOOR. A fighter standing on the stage is hittable; if this were
    // already true the arms below would be measuring something else entirely.
    assert!(
        !held_by_move(&app),
        "the author is already move-intangible before pressing anything"
    );

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

    let mut intangible_ticks = 0usize;
    let mut moved = 0.0_f32;
    // The window opens at the transit and the move outlives it, so a run this
    // long sees both halves.
    for _ in 0..60 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                special_pressed: false,
                ..up_special
            },
        );
        app.update();
        if held_by_move(&app) {
            intangible_ticks += 1;
        }
        moved = moved.max(
            app.world()
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(author)
                .map_or(0.0, |kin| kin.pos.distance(before)),
        );
    }

    // ⛔ THE PREMISE, the same one the blink count needs: he actually teleported.
    assert!(
        moved > 120.0,
        "the author moved {moved:.0}px, which is not a 250px teleport — so the \
         intangibility below is about a move that did not happen"
    );
    assert!(
        intangible_ticks > 0,
        "the author's Revision granted no move intangibility at all: the \
         authored `WindowTag::Invuln` is not reaching `Invulnerability::MOVE`"
    );
    assert!(
        !held_by_move(&app),
        "the author is STILL move-intangible sixty ticks after the transit — a \
         window that does not close is a recovery with no commitment"
    );
}

/// A RELEASED STICK IS A RECOVERY THAT GOES UP.
///
/// ⭐⭐ THE STYLE, END TO END. Jon, on what the Author's up-B should be:
/// *"there is a small window to input any direction and the user can aim the
/// teleport like that but it defaults to up."* Every other arm in this file
/// HOLDS up for the whole move, which is not what a player does — the stick
/// centres the moment the input is made, and the transit is eleven frames
/// later.
///
/// ⛔⛔ AND THAT IS EXACTLY THE BUG THIS ARM WAS WRITTEN FOR, reported from
/// play: *"it seems to just blink me to the side or to the ledge when I'm on
/// the stage."* The aim came back through the held-item helper, whose neutral
/// answer is the body's FACING — so a released stick teleported him
/// horizontally, and the ledge assist then caught the arrival and put him on
/// the lip. Held-stick arms cannot see it: they never let the fallback run.
///
/// ⛔ THE RISE AND THE DRIFT ARE BOTH ASSERTED. "He went up" alone is satisfied
/// by a diagonal that also carries him 250px off the side of the stage, which is
/// the failure being fixed.
#[test]
fn the_authors_revision_rises_when_the_stick_is_released_after_the_press() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

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
    let position = |app: &App| -> ambition_platformer2d::engine_core::Vec2 {
        app.world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(author)
            .map(|kin| kin.pos)
            .expect("the author has kinematics")
    };

    let before = position(&app);
    ambition_platformer2d::sim::drive_control_frame(
        app.world_mut(),
        ambition_platformer2d::engine_core::ControlFrame {
            axis_y: -1.0,
            special_pressed: true,
            special_held: true,
            ..Default::default()
        },
    );
    app.update();

    // ⛔ AND THEN NOTHING AT ALL — no stick, no buttons. The window closes on a
    // player who has already let go, which is the input this arm is about.
    let mut highest = before.y;
    let mut widest = 0.0_f32;
    for _ in 0..60 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
        let now = position(&app);
        // `+y` is gravity-down, so the highest point is the smallest `y`. The
        // two extremes are tracked apart on purpose: a teleport fired sideways
        // never rises at all, so a drift read AT the highest point would report
        // the start and say nothing.
        highest = highest.min(now.y);
        widest = widest.max((now.x - before.x).abs());
    }

    let rise = before.y - highest;
    let drift = widest;
    assert!(
        rise > 120.0,
        "the author rose {rise:.0}px from a teleport that carries 250 — a \
         recovery nobody aimed must go UP, and this one went {drift:.0}px \
         sideways instead"
    );
    assert!(
        drift < 60.0,
        "the author drifted {drift:.0}px sideways on a teleport nobody aimed. \
         The default is UP; a fighter who asked for nothing has not asked to be \
         fired off whichever side of the stage he happens to be facing"
    );
}
