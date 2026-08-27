//! D207 end-to-end: the admiral calls a burning flying shark and rides it.
//!
//! ⛔ IN THE SHIPPED COMPOSITION, not the demo shell. Same reason the goblin/PCA
//! census gives: the demo shell's catalog carries George and two stand-ins, so
//! `npc_pirate_admiral` cannot be seated there and a rig that tried would
//! measure an empty stage.

use ambition_platformer2d::characters::control::DrivingParticipant;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ THE ADMIRAL CALLS A SHARK, RIDES IT, AND COMES OFF WHEN HE JUMPS.
///
/// D207, end to end through the real composition: an authored effect key on a
/// move's timeline becomes a summoned mount, the summoner is welded into its
/// saddle, the mount carries him, and a jump press puts him down.
///
/// ⛔⛔ EVERY ARM HERE HAS A PREMISE, because almost every way this could
/// measure nothing looks exactly like it working. "No shark appeared" and "the
/// shark appeared and the pirate did not board" and "he boarded and the mount
/// never moved" all end with a pirate standing on a stage.
#[test]
fn the_admirals_up_b_summons_a_shark_he_rides_until_he_jumps_off() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use ambition_platformer2d::mount::{Mountable, RideLease, RidingOn};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    // BOTH seats the admiral: seat 0 is the human this test drives, and seat 1
    // is a CPU that simply has to exist for the match to stage.
    // ⛔⛔ `smash_roster`, NOT `smash_roster_at_levels`. The levelled helper
    // overwrites EVERY participant as a CPU, so the first version of this test
    // drove `drive_control_frame` at a slot nobody owned and measured a fighter
    // that never received the press. It looked exactly like a broken up-B.
    // `smash_roster` is the one that seats slot 0 as a human.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..240 {
        app.update();
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    // ⛔⛔ THE PREMISE THAT COST A DAY. The first version used
    // `smash_roster_at_levels`, which overwrites EVERY participant as a CPU —
    // including seat 0 — so `drive_control_frame` drove `PlayerSlot::PRIMARY`
    // at a slot nobody owned. Every downstream assertion then reported a
    // perfectly good up-B as broken. A rig that drives a seat must first say
    // the seat is drivable.
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven by a participant, so `drive_control_frame` is \
         talking to nobody and nothing below measures the up-B"
    );

    let sharks = |app: &mut App| -> usize {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    // ⛔ THE PREMISE: no shark before the press. Without this the count below
    // could be counting an authored stage prop.
    assert_eq!(sharks(&mut app), 0, "the stage already carries a mountable");

    // ── PRESS UP + SPECIAL. Held, because a one-tick press assumes the body
    //    steps after the frame is committed inside one update. ──
    let press = |app: &mut App, frames: usize, frame: ambition_platformer2d::engine_core::ControlFrame| {
        for _ in 0..frames {
            ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
            app.update();
        }
    };
    // ⛔ ONE press FRAME, then HELD. `special_pressed` is a rising EDGE: holding
    // it true for ten frames is ten presses, which repopulates the special
    // buffer every tick. The direction comes from `axis_y` — the fighter brain
    // folds it into `attack_axis` and the special resolver reads that — so
    // `up_pressed` is irrelevant to picking an up-special.
    let up_special = ambition_platformer2d::engine_core::ControlFrame {
        axis_y: -1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    press(&mut app, 1, up_special);
    press(
        &mut app,
        9,
        ambition_platformer2d::engine_core::ControlFrame {
            special_pressed: false,
            ..up_special
        },
    );
    // Let the summon commit and the saddle weld.
    for _ in 0..20 {
        app.update();
    }

    assert_eq!(
        sharks(&mut app),
        1,
        "the up-B summoned no shark, so nothing below is about riding one"
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "a shark exists and the admiral is not on it — the board was refused, \
         which is a `CanPilot` or a mount-class problem rather than a summon one"
    );
    assert!(
        app.world().get::<RideLease>(seat0).is_some(),
        "the ride has no clock on it, so it would never end"
    );

    // ── IT CARRIES HIM. Hold the stick and the pair travels. ──
    let x_of = |app: &mut App, e: Entity| -> f32 {
        app.world()
            .get::<BodyKinematics>(e)
            .map(|kin| kin.pos.x)
            .expect("the rider has kinematics")
    };
    let before = x_of(&mut app, seat0);
    press(
        &mut app,
        60,
        ambition_platformer2d::engine_core::ControlFrame {
            axis_x: 1.0,
            ..Default::default()
        },
    );
    let travelled = x_of(&mut app, seat0) - before;
    assert!(
        travelled > 40.0,
        "the admiral moved {travelled:.1}px while holding right in the saddle — \
         the mount is not answering the rider's stick, which is the whole of \
         'effectively fly around using the control stick'"
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "he came off during ordinary steering"
    );
    // Held so the departure below can be asserted about THIS body.
    let ridden = app
        .world()
        .get::<RidingOn>(seat0)
        .map(|r| r.mount)
        .expect("just asserted");

    // ── A JUMP PUTS HIM DOWN, and the shark leaves. ──
    press(
        &mut app,
        4,
        ambition_platformer2d::engine_core::ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_none(),
        "the admiral jumped and stayed in the saddle"
    );
    // The departure is a flight, not a despawn, so give it its clock.
    for _ in 0..180 {
        app.update();
    }
    // ⛔⛔ THIS SHARK, not "how many sharks exist". Seat 1 is an admiral too and
    // summons its own; counting the stage measured the CPU's shark and went red
    // the moment the recovery shark stopped dying on its own. The contract is
    // that the body THIS rider left is gone.
    assert!(
        app.world().get_entity(ridden).is_err(),
        "the shark stayed on the stage after losing its rider — an unridden \
         mount running its own brain around a platform fighter is exactly what \
         the departure exists to prevent"
    );
}

/// ⭐⭐ A SUMMONED SHARK BELONGS TO WHOEVER CALLED IT.
///
/// ⛔⛔ THE MIRROR MATCH IS THE CASE THAT MAKES THIS A RULE. Jon: *"in a mirror
/// match, with two admirals, if one summons a shark, the other should not be
/// able to ride it."* A class licence cannot express that and should not try —
/// `CanPilot` says *"I can ride sharks"*, which is true of BOTH admirals and has
/// to stay true, because the admiral rides sharks in Ambition as well. What
/// stops the theft is that this particular shark is spoken for.
///
/// ⭐ AND THE RESERVATION IS ALSO THE SPLIT. Construction no longer boards; it
/// hands over a `MountReservedFor` and `board_reserved_mounts` decides on
/// arrival. Today the shark arrives instantly because it is summoned underfoot,
/// which is why this test can still assert a ride one moment later.
#[test]
fn a_summoned_shark_refuses_the_other_admiral_in_a_mirror_match() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::{MountReservedFor, Mountable, RidingOn};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..240 {
        app.update();
    }

    let seat = |app: &mut App, want: usize| -> Entity {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == want)
            .map(|(entity, _)| entity)
            .expect("the match seats this fighter")
    };
    let summoner = seat(&mut app, 0);
    let rival = seat(&mut app, 1);
    assert!(
        app.world().get::<DrivingParticipant>(summoner).is_some(),
        "seat 0 is not driven, so the press below reaches nobody"
    );
    // ⛔ THE PREMISE: BOTH admirals can pilot sharks. If only one could, the
    // refusal below would prove nothing — it would be the licence talking.
    for (who, name) in [(summoner, "the summoner"), (rival, "the rival")] {
        let can = app
            .world()
            .get::<ambition_platformer2d::mount::CanPilot>(who)
            .expect("an admiral states what it can board");
        assert!(
            can.can_pilot(&ambition_platformer2d::mount::MountClass(
                ambition_platformer2d::characters::smash_ride::SHARK_CLASS.to_string()
            )),
            "{name} cannot pilot a shark at all, so this test cannot tell a \
             reservation from a missing licence"
        );
    }

    let up_special = ambition_platformer2d::engine_core::ControlFrame {
        axis_y: -1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), up_special);
    app.update();
    for _ in 0..9 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                special_pressed: false,
                ..up_special
            },
        );
        app.update();
    }
    // ⭐ ONE TICK ONLY, so the reservation is caught BEFORE `board_reserved_mounts`
    // spends it. The reservation is the thing under test; the ride it becomes is
    // the subject of the test above.
    app.update();

    let shark = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        let all: Vec<Entity> = q.iter(world).collect();
        assert_eq!(
            all.len(),
            1,
            "the up-B summoned {} sharks, so nothing below is about one of them",
            all.len()
        );
        all[0]
    };

    // ⭐ THE RESERVATION NAMES ITS SUMMONER. Either it is still held — the board
    // has not happened yet — or it has already become that admiral's ride.
    // Both are correct; what must never be true is the rival on the shark.
    let reserved_for = app
        .world()
        .get::<MountReservedFor>(shark)
        .map(|held| held.rider);
    let boarded = app
        .world()
        .get::<RidingOn>(summoner)
        .map(|riding| riding.mount);
    assert!(
        reserved_for == Some(summoner) || boarded == Some(shark),
        "the summoned shark is neither held for the admiral that called it nor \
         already carrying them: reserved_for={reserved_for:?} boarded={boarded:?}"
    );
    assert!(
        app.world().get::<RidingOn>(rival).is_none(),
        "the rival admiral is riding a shark it did not summon"
    );

    // ── AND THE RIVAL IS REFUSED IF IT ASKS. ──
    let stolen = {
        let world = app.world_mut();
        ambition_platformer2d::mount::board(world, rival, shark)
    };
    assert!(
        !stolen,
        "the second admiral boarded the first one's summoned shark — a mount \
         held for one rider accepted another"
    );
}

/// ⭐⭐ THE ROAD A PLAYER ACTUALLY TRAVELS: the character-select grid.
///
/// ⛔⛔ EVERY OTHER TEST IN THIS FILE TAKES A SHORTCUT. They insert a
/// `MatchParticipantRoster` built by `smash_roster` and jump straight to the
/// gameplay route. A human picks fighters on the select screen, which builds its
/// participants from scratch in `SmashSelect::roster_seeded` — and that road is
/// where the up-B shipped broken twice. The first time it was the pilot licence,
/// which `smash_roster` granted and the screen did not. Jon then reported the
/// same failure against two builds carrying the repair, while these tests stayed
/// green — which is exactly what a test that never travels the real road looks
/// like.
///
/// ⭐ IT SEATS THE SAME FIGHTER TWICE for the same reason the shortcut test
/// does: seat 0 is the human this test drives and seat 1 only has to exist.
#[test]
fn an_admiral_picked_off_the_grid_can_ride_the_shark_it_summons() {
    use ambition_demo_smash::select::{SlotOccupant, SmashRoster, SmashSelect};
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::{Mountable, RidingOn};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    // ── ONTO THE SELECT SCREEN, the way the title screen sends you. ──
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_SELECT_ROUTE,
        )));
    for _ in 0..120 {
        app.update();
    }

    // ⛔ THE PREMISE: the grid offers the admiral. Without this the picks below
    // would seat whoever happens to sit at index 0 and prove nothing.
    let admiral_index = {
        let grid = app
            .world()
            .get_resource::<SmashRoster>()
            .expect("the select screen assembled its grid");
        grid.0
            .iter()
            .position(|id| id == ambition_demo_smash::SMASH_SHARK_RIDER)
            .unwrap_or_else(|| {
                panic!(
                    "the shark rider is not on the grid, so this test cannot pick it: {:?}",
                    grid.0
                )
            })
    };

    {
        let mut select = app
            .world_mut()
            .get_resource_mut::<SmashSelect>()
            .expect("the select screen has its state");
        select.set_occupant(0, SlotOccupant::Controller { device: 0 });
        select.set_pick(0, admiral_index);
        select.set_occupant(1, SlotOccupant::Cpu);
        select.set_pick(1, admiral_index);
        assert!(
            select.ready(),
            "two decided seats did not make a startable match"
        );
    }
    // ── AND PRESS START, through the screen's own request. ──
    app.world_mut()
        .insert_resource(ambition_demo_smash::select_screen::StartRequested(true));
    for _ in 0..300 {
        app.update();
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the select screen started a match that seats a first fighter")
    };
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven, so the press below reaches nobody"
    );
    // ⛔⛔ THE ASSERTION THAT WOULD HAVE CAUGHT THE ORIGINAL BUG. The licence now
    // comes from the character, so it should be here on every road — but that is
    // the claim under test, not an assumption this test may make.
    let can = app
        .world()
        .get::<ambition_platformer2d::mount::CanPilot>(seat0)
        .cloned();
    assert!(
        can.as_ref().is_some_and(|c| c.can_pilot(
            &ambition_platformer2d::mount::MountClass(
                ambition_platformer2d::characters::smash_ride::SHARK_CLASS.to_string()
            )
        )),
        "an admiral seated FROM THE GRID cannot pilot a shark: {can:?}"
    );

    let sharks = |app: &mut App| -> usize {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    // ⛔⛔ THE CPU ADMIRAL ACTS FIRST, and pinning a count here is what made the
    // first version of this test fail for the wrong reason. Seat 1 is an admiral
    // too: by the time the match settles it has already summoned, BOARDED one
    // shark and sent a second away. That is the feature working, measured on
    // this very road — so what this test owes is that the HUMAN's press mounts
    // the HUMAN, not that the stage is empty.
    assert!(
        app.world().get::<RidingOn>(seat0).is_none(),
        "seat 0 is already riding before it pressed anything"
    );
    let before = sharks(&mut app);

    // ⛔⛔ AIRBORNE, because that is how a recovery is actually pressed and how
    // Jon pressed it: his log reads `grounded=false` on nearly every summon,
    // and every test here had been pressing from a standing start. A shark
    // summoned under a falling body meets different geometry, different contact
    // and a different frame than one summoned under a body at rest.
    ambition_platformer2d::sim::drive_control_frame(
        app.world_mut(),
        ambition_platformer2d::engine_core::ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
    app.update();
    for _ in 0..12 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                jump_held: true,
                ..Default::default()
            },
        );
        app.update();
    }

    let up_special = ambition_platformer2d::engine_core::ControlFrame {
        axis_y: -1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), up_special);
    app.update();
    for _ in 0..9 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                special_pressed: false,
                ..up_special
            },
        );
        app.update();
    }
    for _ in 0..20 {
        app.update();
    }

    assert!(
        sharks(&mut app) > before,
        "the up-B summoned no shark on the grid road ({before} before, {} after)",
        sharks(&mut app)
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "a shark exists and the admiral picked off the GRID is not on it"
    );

    // ⭐ AND THE RIDE SURVIVES SUSTAINED FLIGHT.
    //
    // ⚠ WHAT THIS ARM PROVES AND WHAT IT DOES NOT. It exercises a mounted pair
    // being flown for a second and a half, which no other test here did. It does
    // NOT pin the charge-crash wiring: forcing that guard off leaves this green,
    // because a shark being steered by its rider never reaches the
    // stopped-dead-at-charge-speed geometry the crash predicate wants. The
    // occupied-versus-riderless wall impact is a separate poison and is owed.
    // ⭐⭐ AND IT SURVIVES BEING HIT, which is the arm that reproduces the real
    // failure. Jon's shark died about twenty milliseconds after every board and
    // no local hypothesis explained it, because this fixture's CPU is not
    // reliably swinging during the summon window and a real opponent is. The
    // authored shark carries 6 HP against a move table that runs 2 to 17, and
    // the summon puts it exactly where its rider is — mid-fight, exactly where
    // the hits are. One clean connection deleted it.
    //
    // ⛔ TEN DAMAGE IS A MIDDLING HIT, not a worst case: the admiral's own table
    // has several at or above it. At the authored 6 this kills the shark and the
    // ride ends; at `SUMMON_SHARK_HEALTH` it must not.
    {
        let world = app.world_mut();
        let mount = world
            .get::<RidingOn>(seat0)
            .map(|r| r.mount)
            .expect("just asserted the admiral is aboard");
        let mut hp = world
            .get_mut::<ambition_platformer2d::actor::BodyHealth>(mount)
            .expect("a summoned shark carries its own pool");
        hp.damage(10);
    }
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "one middling hit killed the recovery shark out from under its rider — \
         which is what the authored 6 HP does in a platform fighter, and is why \
         the summon states its own survivability"
    );

    // ⛔⛔ FLOWN INTO THE STAGE, not merely flown. The shark's self-detonation
    // fires when a fast charge is stopped dead by geometry, so a pair drifting
    // in open air never reaches the condition — which is why the earlier version
    // of this arm stayed green with the guard forced off and proved nothing.
    // Jon's presses were airborne and hit something immediately.
    for _ in 0..120 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                axis_x: 1.0,
                axis_y: 1.0,
                ..Default::default()
            },
        );
        app.update();
    }
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "the admiral was put off the shark within a second and a half of \
         boarding it — the mount died under him, which is what a charge-crash \
         suicide that cannot see its own rider does"
    );
    // ⛔⛔ AND IT BOARDED WITH ROOM TO SPARE. Measured here because the summon
    // does NOT get the position it asks for: it names the rider's own centre,
    // and `actor_spawn_center_for_collision` then preserves the authored BOTTOM
    // EDGE, so the shark's centre lands 62px away for a body this size. That is
    // inside `SUMMON_BOARD_RADIUS` and was inside it by 34px before anybody had
    // measured either number — which is the kind of margin that holds until one
    // sprite changes. Pinning the gap makes a placement change say so here
    // rather than in play.
    {
        let world = app.world_mut();
        let rider = world
            .get::<ambition_platformer2d::actor::BodyKinematics>(seat0)
            .expect("the rider has kinematics")
            .pos;
        let mount = world
            .get::<RidingOn>(seat0)
            .map(|r| r.mount)
            .expect("just asserted");
        let gap = world
            .get::<ambition_platformer2d::actor::BodyKinematics>(mount)
            .expect("the mount has kinematics")
            .pos
            .distance(rider);
        assert!(
            gap < 200.0,
            "the ridden shark sits {gap:.1}px from its rider, which says the \
             saddle weld is not holding the pair together"
        );
    }
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "a shark exists and the admiral picked off the GRID is not on it — which \
         is exactly what Jon reported in play while the shortcut tests stayed green"
    );
}
