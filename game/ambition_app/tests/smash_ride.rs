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
    use ambition_platformer2d::mount::{MountReservedFor, Mountable, RideLease, RidingOn};
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
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

    // ⛔⛔ SHARKS THIS ADMIRAL OWNS, not sharks on the stage. This counted every
    // `Mountable` in the world, which is a premise only a rival who never acts
    // can satisfy — and the OTHER admiral in this mirror match is a CPU that can
    // summon its own. It held only because the 3-second opening ceremony used up
    // most of the 240-frame settle above; shortening the ceremony made a
    // perfectly correct rival look like a broken assertion.
    //
    // ⇒ attribute by SUMMONER. A shark is this admiral's if it is reserved for
    // them or if they are on it, which is exactly what every assertion below
    // means by "the shark".
    let sharks = |app: &mut App, rider: Entity| -> usize {
        let ridden = app.world().get::<RidingOn>(rider).map(|on| on.mount);
        let world = app.world_mut();
        let mut q = world.query_filtered::<(Entity, Option<&MountReservedFor>), With<Mountable>>();
        q.iter(world)
            .filter(|(mount, reserved)| {
                ridden == Some(*mount) || reserved.is_some_and(|reserved| reserved.rider == rider)
            })
            .count()
    };
    // ⛔ THE PREMISE: this admiral has no shark before the press.
    assert_eq!(
        sharks(&mut app, seat0),
        0,
        "this admiral already owns a mountable, so the summon below proves nothing"
    );

    // ── PRESS UP + SPECIAL. Held, because a one-tick press assumes the body
    //    steps after the frame is committed inside one update. ──
    let press =
        |app: &mut App, frames: usize, frame: ambition_platformer2d::engine_core::ControlFrame| {
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
        sharks(&mut app, seat0),
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
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

    // ⛔⛔ THE SUMMONER'S SHARK, not the stage's only one. This took the sole
    // `Mountable` in the world, which is a premise only a rival who never acts
    // can satisfy — and the rival here is a CPU admiral who can summon too. It
    // held because the 3-second opening ceremony ate most of the 240-frame
    // settle above, so shortening the ceremony turned a correct rival into a
    // failing assertion. A test about WHOSE shark this is cannot identify it by
    // being the only one.
    let shark = {
        // ⚠ RESERVED-FOR **OR** RIDDEN, because the board may already have
        // happened and `board_reserved_mounts` spends the reservation when it
        // does — which is the same both-are-correct reasoning the assertion
        // below states in words.
        let ridden = app.world().get::<RidingOn>(summoner).map(|on| on.mount);
        let world = app.world_mut();
        let mut q = world.query_filtered::<(Entity, Option<&MountReservedFor>), With<Mountable>>();
        let mine: Vec<Entity> = q
            .iter(world)
            .filter(|(mount, reserved)| {
                ridden == Some(*mount)
                    || reserved.is_some_and(|reserved| reserved.rider == summoner)
            })
            .map(|(mount, _)| mount)
            .collect();
        assert_eq!(
            mine.len(),
            1,
            "the summoner's up-B left him {} sharks, so nothing below is about \
             one of them",
            mine.len()
        );
        mine[0]
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
    // ⛔ NOT ON **THIS** SHARK. This asked whether the rival was riding at ALL,
    // which is a different claim and not the rule: the rival is an admiral too
    // and summoning its own shark is exactly what it should do. The rule is that
    // it may not be on the one somebody else called.
    let rival_rides = app
        .world()
        .get::<RidingOn>(rival)
        .map(|riding| riding.mount);
    assert_ne!(
        rival_rides,
        Some(shark),
        "the rival admiral is riding the shark the OTHER admiral summoned"
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for 300 frames. The fixed count was
    // a number that happened to be a little longer than the 3-second opening
    // ceremony, so it silently encoded the ceremony's LENGTH: shortening the
    // ceremony (D248's dev mode) handed every CPU on the stage ~2.7 extra
    // seconds of free play before the press below, and the charge-crash NPC
    // used them to kill the shark this test is about.
    //
    // ⭐ the condition is observable — a fighter still held by the ceremony
    // carries `ScriptedControl` — so the press now happens right after GO
    // however long GO takes.
    let mut live = false;
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
        // BOTH halves: a cast that does not exist yet is not a cast whose hold
        // has come off.
        if seated > 0 && held == 0 {
            live = true;
            break;
        }
    }
    assert!(
        live,
        "the opening ceremony never released the cast, so nothing below is \
         about a live round"
    );

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
        can.as_ref()
            .is_some_and(|c| c.can_pilot(&ambition_platformer2d::mount::MountClass(
                ambition_platformer2d::characters::smash_ride::SHARK_CLASS.to_string()
            ))),
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
        let _ = mount;
        // ⭐⭐ A REAL HITBOX, NOT A SYNTHETIC `damage()` CALL. The point of this
        // arm is that an OPPONENT'S SWING reaches the shark: it is Neutral, and
        // `CombatRelation::damage_lands` is true for `Foe | Neutral` — nobody
        // AIMS at it (`is_target` is Foe-only) but everything HITS it. Poking
        // the health pool directly would prove the pool's arithmetic and skip
        // the question that matters.
        world.spawn((
            ambition_platformer2d::combat::strike::Hitbox {
                // The seat that is not the rider: an opponent's swing.
                owner: seat0,
                source: ambition_platformer2d::vfx::HitSide::Player,
                // ⛔ ANCHORED TO THE SHARK, NOT A REMEMBERED POINT. A `World`
                // box at a position read a moment earlier MISSES a mount that is
                // being flown — measured: the same arm landed when the press was
                // from a standing start and stopped landing once the admiral
                // jumped first, which made a survivability assertion pass
                // because nothing hit it.
                anchor: ambition_platformer2d::combat::strike::HitboxAnchor::FollowOwner {
                    local_offset: ambition_platformer2d::engine_core::Vec2::ZERO,
                },
                half_extent: ambition_platformer2d::engine_core::Vec2::new(400.0, 400.0),
                shape: None,
                facing: 1.0,
                // ⛔ THE WORST SINGLE HIT IN THE GAME, not a middling one. This
                // arm used to land 10 and passed at 24 HP — while Jon's shark
                // was being deleted by a fully charged forward smash at 17 x
                // 1.7 = 29. A survivability test that lands less than the
                // biggest thing that can land proves the pool survives
                // something nobody was worried about.
                damage: 29,
                knockback: ambition_platformer2d::combat::strike::HitboxKnockback::FeelScale(0.0),
                launch_dir: None,
                frame_down: ambition_platformer2d::engine_core::Vec2::new(0.0, 1.0),
                reaction: None,
                strike_sfx: None,
            },
            ambition_platformer2d::combat::strike::HitboxHits::default(),
        ));
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

/// ⭐⭐ TWO THRESHOLDS, AND THEY ARE NOT THE SAME ONE.
///
/// Jon named both and they are easy to conflate: a hit that FLINCHES refreshes
/// the up-B, and a hit that LAUNCHES takes you off the shark. A jab that flinches
/// a rider leaves it aboard. `BodyMotionFacts::tumbling` is the engine's own word
/// for *"launched with no control"*, which is why `dismount_launched_riders`
/// reads it rather than a knockback magnitude somebody would have to defend.
///
/// ⛔⛔ NEITHER HALF WAS TESTED. The rule shipped as an authored condition and a
/// comment; nothing asked whether a flinching hit leaves the rider aboard, and
/// nothing asked whether a launch takes it off. A rule with no arm on either side
/// of its threshold is a rule the next refactor is free to move.
#[test]
fn a_flinch_leaves_the_admiral_aboard_and_a_launch_takes_him_off() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::{Mountable, RidingOn};
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven, so the press below reaches nobody"
    );
    // ⛔ THE ATTACKER MUST NOT BE THE VICTIM. A hitbox skips self-hits, so a
    // strike owned by the rider cannot land on the rider — the first version of
    // this test did exactly that and measured a fighter at full health.
    let rival = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("the match seats a second fighter")
    };

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
        app.world().get::<RidingOn>(seat0).is_some(),
        "the admiral never boarded, so neither threshold below is being measured"
    );
    let sharks = |app: &mut App| -> usize {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    assert!(sharks(&mut app) >= 1, "no shark to be thrown off");

    // ── A HIT THAT DOES NOT LAUNCH LEAVES HIM ABOARD. ──
    //
    // ⛔⛔ THIS ARM USED NOT TO HIT HIM AT ALL — it waited twenty frames and
    // asserted `RidingOn`, which proves only that an undisturbed pirate stays
    // mounted. A GPT 5.6 review Jon relayed on 2026-08-27 caught it.
    //
    // ⚠ AND THE NOTE LEFT HERE AT THE TIME WAS WRONG about why it was hard.
    // It recorded that a weak volume "did not land — `BodyHealth::damage_taken()`
    // never moved", and inferred something about mounted riders refusing soft
    // hits. Measured again building the box exactly like the launch box below
    // and differing only in `damage` and `knockback`: it lands, first try. The
    // earlier attempt differed some other way that was never isolated, and the
    // conclusion drawn from it went into this file as fact for a day.
    //
    // ⛔ THE SECOND BLOCKER IS STILL REAL AND STILL UNCLAIMED: a mounted admiral
    // holds a FULL recovery charge, so "the flinch refunded the charge" remains
    // a check that cannot fail and is deliberately NOT asserted here. That is
    // D204/D250 territory.
    // A REAL HIT, built exactly like the launch box below and differing only in
    // the two numbers that decide whether it launches. If it does not land, the
    // probe below says so rather than the arm passing on an undisturbed rider.
    // Percent this body has taken, read off its own health.
    let damage_taken = |app: &mut App, body: Entity| -> i32 {
        app.world()
            .get::<ambition_platformer2d::characters::actor::BodyHealth>(body)
            .map_or(0, |h| h.damage_taken())
    };
    let before = damage_taken(&mut app, seat0);
    {
        let world = app.world_mut();
        let rider_pos = world
            .get::<ambition_platformer2d::actor::BodyKinematics>(seat0)
            .expect("the rider has kinematics")
            .pos;
        world.spawn((
            ambition_platformer2d::combat::strike::Hitbox {
                owner: rival,
                source: ambition_platformer2d::vfx::HitSide::Enemy,
                anchor: ambition_platformer2d::combat::strike::HitboxAnchor::World {
                    center: rider_pos,
                },
                half_extent: ambition_platformer2d::engine_core::Vec2::new(60.0, 60.0),
                shape: None,
                facing: 1.0,
                damage: 4,
                // Soft: a flinch, not a launch. This is the half of Jon's pair
                // that must leave him aboard.
                knockback: ambition_platformer2d::combat::strike::HitboxKnockback::FeelScale(0.35),
                launch_dir: None,
                frame_down: ambition_platformer2d::engine_core::Vec2::new(0.0, 1.0),
                reaction: None,
                strike_sfx: None,
            },
            ambition_platformer2d::combat::strike::HitboxHits::default(),
        ));
    }
    for _ in 0..20 {
        app.update();
    }
    // ⛔ THE HIT MUST HAVE LANDED, or "he stayed aboard" is a claim about an
    // undisturbed fighter — which is what this arm used to be.
    let flinched = damage_taken(&mut app, seat0) - before;
    assert!(
        flinched > 0,
        "the soft hit never reached the mounted admiral (damage_taken moved by \
         {flinched}), so staying aboard proves nothing about flinching"
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "a flinch took the admiral off the shark — only a launch may do that"
    );

    // ── AND A LAUNCH TAKES HIM OFF. ──
    // ⛔⛔ LAUNCHED FOR REAL, NOT BY SETTING THE FACT. The first version wrote
    // `BodyMotionFacts::tumbling = true` by hand and failed — because that fact
    // is DERIVED by the movement pass every tick, so the write was recomputed
    // away before the rule ever read it. A test that sets a derived value is
    // testing its own assignment.
    {
        let world = app.world_mut();
        let rider_pos = world
            .get::<ambition_platformer2d::actor::BodyKinematics>(seat0)
            .expect("the rider has kinematics")
            .pos;
        world.spawn((
            ambition_platformer2d::combat::strike::Hitbox {
                owner: rival,
                // ⛔ THE HOSTILE SIDE. A `Player`-sourced strike against a
                // Player-faction rider reads as friendly fire and is refused;
                // the shark took one only because `Neutral` always does.
                source: ambition_platformer2d::vfx::HitSide::Enemy,
                anchor: ambition_platformer2d::combat::strike::HitboxAnchor::World {
                    center: rider_pos,
                },
                half_extent: ambition_platformer2d::engine_core::Vec2::new(60.0, 60.0),
                shape: None,
                facing: 1.0,
                damage: 14,
                // Hard enough to tumble: this is the launch half of Jon's pair.
                knockback: ambition_platformer2d::combat::strike::HitboxKnockback::FeelScale(6.0),
                launch_dir: Some(ambition_platformer2d::engine_core::Vec2::new(1.0, -1.0)),
                frame_down: ambition_platformer2d::engine_core::Vec2::new(0.0, 1.0),
                reaction: None,
                strike_sfx: None,
            },
            ambition_platformer2d::combat::strike::HitboxHits::default(),
        ));
    }
    for _ in 0..10 {
        app.update();
    }
    // ⛔ THE PREMISE: the hit actually LAUNCHED him. `tumbling` is what the rule
    // reads, so a strike that damaged without launching would make the assertion
    // below pass for the wrong reason — or fail for one.
    assert!(
        app.world()
            .get::<ambition_platformer2d::engine_core::BodyMotionFacts>(seat0)
            .is_some_and(|facts| facts.tumbling),
        "the strike did not launch the admiral, so this measures nothing about \
         the launch threshold"
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_none(),
        "the admiral was launched and stayed in the saddle — `tumbling` is the \
         engine's word for 'hit hard enough to lose control', and it is the \
         whole of the rule that takes a rider off"
    );
    // ⭐ AND HE ACTUALLY TRAVELS — the half the launch arm never asserted.
    //
    // ⛔⛔ THIS DOES NOT GUARD THE KERNEL RULE, and saying so is the point.
    // The defect a review reported here — the saddle pin writes the rider's
    // velocity to ZERO every tick, and the kernel used to spend the launch into
    // that same velocity — is real, and it is fixed in
    // `MotionStepContext::pose_owned_externally`. But POISON-CHECKED both ways,
    // this arm stays GREEN with the fix reverted: the admiral leaves at about
    // (-1884, -1690) either way, so whatever produces that travel is not the
    // launch this hit staged. The guard that actually fails without the fix is
    // `a_pose_owned_body_keeps_its_pending_launch_until_it_is_released` in the
    // kernel's own tests.
    //
    // ⚠ SO WHAT THIS ARM IS FOR: a launched rider must come off MOVING. It would
    // catch a future change that zeroed a released body outright, which the
    // `tumbling` and `RidingOn` assertions above cannot — they were both true
    // throughout the reported bug.
    //
    // ⛔ THE LATERAL SPEED, NOT THE MAGNITUDE AND NOT THE SIGN. `length() > 50`
    // passes on GRAVITY alone four frames after a dismount, so it was a check
    // that could not fail; and the authored `launch_dir` of (1, -1) does not
    // mean the body ends up moving +x. Nothing but knockback moves the x axis.
    for _ in 0..4 {
        app.update();
    }
    let released = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(seat0)
        .map(|kin| kin.vel)
        .expect("the released admiral has kinematics");
    assert!(
        released.x.abs() > 50.0,
        "the admiral came off the shark carrying {released:?} — no lateral \
         travel means the launch was spent into a velocity the saddle then \
         erased, so the hit ended the ride and moved nobody"
    );
}

/// ⭐⭐ THE RIDE ENDS ON ITS OWN CLOCK, and the shark leaves when it does.
///
/// Jon: *"Let's say the move lasts for 5 seconds before the shark forces the
/// pirate to jump off and it flys away."* That is the whole shape of the
/// ability's cost, and nothing tested it: the lease was authored, the tick
/// system had a unit test, and no arm ever asked whether a ride actually ENDS.
///
/// ⛔ FIVE SECONDS IS NOT ASSERTED AS A NUMBER, it is asserted as a boundary:
/// still aboard well before it, off well after. Pinning the exact tick would
/// make this a test of the frame clock, and the rule is "about five seconds",
/// not "at frame 300".
#[test]
fn the_ride_ends_when_its_lease_runs_out_and_the_shark_leaves() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::{Mountable, RideLease, RidingOn};
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven, so the press below reaches nobody"
    );

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

    let ridden = app
        .world()
        .get::<RidingOn>(seat0)
        .map(|r| r.mount)
        .expect("the admiral boarded, so there is a ride to time");
    let lease = app
        .world()
        .get::<RideLease>(seat0)
        .map(|l| l.remaining)
        .expect("a summoned ride carries a clock");
    assert!(
        lease > 3.0,
        "the ride opened with {lease:.2}s left, which is not the five seconds \
         the ability is designed around"
    );

    // ── STILL ABOARD WELL INSIDE THE LEASE. ──
    for _ in 0..120 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "the ride ended about two seconds in — a lease that expires early is a \
         recovery that does not reach the ledge"
    );

    // ── AND OFF WELL AFTER IT. ──
    for _ in 0..300 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }
    assert!(
        app.world().get::<RidingOn>(seat0).is_none(),
        "the lease ran out and the admiral is still aboard — the ride has no end, \
         which is flight rather than a recovery"
    );
    // ⭐ AND THE SHARK GOES. Jon asked for it to fly away when the ride ends;
    // asserted about THIS body, because seat 1 is an admiral too and summons its
    // own.
    for _ in 0..240 {
        app.update();
    }
    assert!(
        app.world().get_entity(ridden).is_err(),
        "the shark that carried the admiral is still on the stage after its \
         lease expired"
    );
    let _ = |app: &mut App| -> usize {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
}

/// ⭐⭐ THE RECOVERY SHARK IS NOT A HAZARD, AND `Neutral` IS NOT WHAT SAYS SO.
///
/// Jon: *"No, the shark doesn't have contact damage in smash."* The code claimed
/// `HitSide::Neutral` delivered that and it does not: `damage_lands` is true for
/// `Foe | Neutral` — correctly, because an opponent must be able to gimp a
/// recovery. What kept the hazard quiet was that a neutral body acquires no
/// target, so it had nobody to touch: a coincidence of the targeting rules, and
/// one a future grudge rule could undo without anybody noticing.
///
/// ⛔ SO THE TUNING IS ASSERTED, not the absence of observed damage. "No hits
/// happened" is what the old accident already produced; what changed is that the
/// occurrence DECLINES the trait, and that is the thing worth pinning.
#[test]
fn the_summoned_shark_carries_no_contact_hazard() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::Mountable;
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven, so the press below reaches nobody"
    );

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

    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<Mountable>>();
    let sharks: Vec<Entity> = q.iter(world).collect();
    assert!(
        !sharks.is_empty(),
        "the up-B summoned no shark, so there is no occurrence to inspect"
    );
    for shark in sharks {
        let config = world
            .get::<ambition_platformer2d::combat::actor_tuning::ActorConfig>(shark)
            .expect("a summoned actor carries its config");
        assert!(
            !config.tuning.body_contact_damage,
            "the summoned shark still carries the contact hazard its character \
             authors — `Neutral` never removed it, and a targeting rule that ever \
             hands this body a foe would make an old hazard live under a design \
             that says it should not exist"
        );
    }
}

/// ⭐⭐ KILL THE SHARK AND THE ADMIRAL FALLS OFF — and can summon another.
///
/// Jon: *"If the shark is hit 3 times, then the shark dies and the rider would
/// fall off."* Two halves, and the second is the one with teeth: ADR 0020
/// deliberately KEEPS `RidingOn` attached when a mount dies, so a same-room reset
/// can re-mount an authored pair. A summoned shark never comes back, so without
/// `dissolve_the_ride_when_the_shark_dies` the admiral would be left logically
/// riding a corpse forever — and `translate_shark_summons` refuses anybody
/// already carrying `RidingOn`. One dead shark, no more sharks, for the match.
#[test]
fn killing_the_shark_puts_the_admiral_down_and_frees_the_up_b() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::{Mountable, RidingOn};
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven, so the press below reaches nobody"
    );

    let press_up_b = |app: &mut App| {
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
    };
    press_up_b(&mut app);
    let ridden = app
        .world()
        .get::<RidingOn>(seat0)
        .map(|r| r.mount)
        .expect("the admiral boarded, so there is a shark to kill");

    // ── KILL IT. Enough damage that the pool is emptied whatever it is. ──
    {
        let world = app.world_mut();
        let mut hp = world
            .get_mut::<ambition_platformer2d::actor::BodyHealth>(ridden)
            .expect("the shark carries its own pool");
        hp.damage(9_999);
    }
    for _ in 0..30 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }

    assert!(
        app.world().get::<RidingOn>(seat0).is_none(),
        "the shark died and the admiral is still logically riding it — ADR 0020 \
         keeps the link across a mount's death on purpose, which is right for an \
         authored pair whose shark respawns and wrong for a summon that never \
         comes back"
    );
    // ⭐ AND THE UP-B IS FREE AGAIN. Without the dissolution above,
    // `translate_shark_summons` would refuse forever: one dead shark, no more
    // sharks, for the rest of the match.
    let before = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    for _ in 0..180 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }
    press_up_b(&mut app);
    let after = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    assert!(
        after > before || app.world().get::<RidingOn>(seat0).is_some(),
        "the admiral could not summon again after its shark died ({before} \
         before, {after} after) — a dead mount had taken the ability with it"
    );
}

/// ⭐⭐ TWO ADMIRALS, TWO SHARKS, EACH ON ITS OWN.
///
/// Jon: *"If a shark is flying away and the pirate has met all of the conditions
/// to be able to use the move again, an additional shark can be summoned, i.e.
/// multiple sharks are allowed on the screen at once."* Nothing tested that the
/// feature is per-RIDER rather than per-STAGE: a lease, a reservation or a
/// departure keyed to "the shark" instead of "this shark" would work perfectly
/// in every single-rider test in this file and fall apart the first time two
/// admirals pressed up-B.
///
/// ⛔ IT ASSERTS THE PAIRINGS, not the count. Two sharks existing proves nothing
/// if both riders ended up on the same one, or if one rider's lease put the
/// other down.
#[test]
fn two_admirals_ride_their_own_sharks_at_the_same_time() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::mount::{MountSlot, Mountable, RidingOn};
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
    // ⛔⛔ WAIT FOR THE ROUND TO GO LIVE, not for a fixed 240 frames. That count
    // was a little longer than the 3-second opening ceremony, so it encoded the
    // ceremony's LENGTH — and D248's dev mode, which runs the ceremony 10x fast,
    // turned every one of these settles into a different starting world. The
    // condition is observable: a cast exists, and nothing in it is still held by
    // `ScriptedControl`. BOTH halves, because a cast that does not exist yet is
    // not a cast whose hold has come off.
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(
            live,
            "the opening ceremony never released the cast, so nothing below is \
             about a live round"
        );
    }

    let seat = |app: &mut App, want: usize| -> Entity {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, s)| s.0 == want)
            .map(|(entity, _)| entity)
            .expect("the match seats this fighter")
    };
    let human = seat(&mut app, 0);
    let cpu = seat(&mut app, 1);

    // The human presses; the CPU admiral summons on its own, which is what makes
    // this two riders rather than one pressing twice.
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
    // Long enough for the CPU to take its own turn.
    for _ in 0..240 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }

    let mounts: Vec<(usize, Option<Entity>)> = [human, cpu]
        .iter()
        .enumerate()
        .map(|(seat, rider)| (seat, app.world().get::<RidingOn>(*rider).map(|r| r.mount)))
        .collect();
    let ridden: Vec<Entity> = mounts.iter().filter_map(|(_, m)| *m).collect();
    assert!(
        ridden.len() >= 2,
        "fewer than two admirals were aboard a shark at once ({mounts:?}), so \
         this measures nothing about simultaneous rides"
    );
    // ⛔ AND THEY ARE DIFFERENT SHARKS, each holding its own rider.
    //
    // ⚠ THIS ARM IS A GUARD, NOT A PROOF, and the difference is worth stating:
    // removing `board`'s occupied-saddle refusal leaves this test GREEN, because
    // each admiral summons and boards its own reservation and neither ever
    // reaches for the other's. The refusal itself is pinned by
    // `a_summoned_shark_refuses_the_other_admiral_in_a_mirror_match`, which asks
    // for it directly. What this arm catches is a lease, a reservation or a
    // departure keyed to "the shark" rather than "this shark" — state that would
    // work in every single-rider test in this file and collapse the moment two
    // admirals are aboard.
    assert_ne!(
        ridden[0], ridden[1],
        "both admirals are riding the SAME shark — a mount holds one rider, and \
         a second board should have been refused"
    );
    for (seat, mount) in mounts.iter().filter_map(|(s, m)| m.map(|m| (*s, m))) {
        let rider = if seat == 0 { human } else { cpu };
        let slot = app
            .world()
            .get::<MountSlot>(mount)
            .and_then(|slot| slot.rider);
        assert_eq!(
            slot,
            Some(rider),
            "seat {seat}'s shark does not name it as the occupant, so the pair is \
             welded one way only"
        );
    }
    let sharks = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    assert!(
        sharks >= 2,
        "only {sharks} shark(s) on a stage carrying two riders"
    );
}

/// ⭐⭐ A RECOVERY MOUNT MAY BE GIMPED, BUT NOT DELETED IN ONE HIT.
///
/// ⛔⛔ THIS IS THE ARITHMETIC THE FIRST SURVIVABILITY FIX GOT WRONG. Jon's rule
/// is a count — *"hitting it 'enough' … roughly three hits"* — and a pool below
/// the largest single hit makes that false on its face. The summon was given 24;
/// the admiral's own forward smash is 17 damage with `smash_charge_mult = 1.7`,
/// which lands at 28.9. Jon reported the shark still dying instantly on a build
/// carrying the 24, and that is exactly what a 24 HP body does when the thing
/// hitting it deals 29.
///
/// ⭐ COMPUTED FROM THE MOVESET, NOT PINNED TO A NUMBER. A new move, or a bigger
/// charge multiplier, moves the floor — and this fails then, which is the whole
/// point. The FIGURE above the floor is Jon's to choose; the floor is not.
#[test]
fn a_recovery_mount_cannot_be_deleted_by_one_hit() {
    // ⛔⛔ THE WHOLE SELECTABLE CAST, not the fighter who summons it. This
    // scanned `pirate_admiral_moveset()` alone and so asserted a property about
    // ONE fighter while reading as a statement about the game — and it was
    // false the whole time: George Booul's forward smash is `damage: 21` with
    // `smash_charge_mult = 1.7`, which is 36, EXACTLY the shark's pool (GPT
    // 5.6, 2026-08-27). Anyone can delete the recovery in one connection.
    //
    // ⚠ TWO CRATES, because the cast is authored in two: `ambition_content`
    // keeps the shared roster and `ambition_demo_smash` authors its own
    // fighter. A census that reads one crate narrows silently when the other
    // gains a move.
    let mut cast = ambition_content::authored_movesets::tables();
    cast.push((
        "george_booul",
        ambition_demo_smash::george_booul_moveset::george_booul_moveset(),
    ));

    let (who, worst) = cast
        .iter()
        .flat_map(|(name, moveset)| {
            moveset.moves.iter().flat_map(move |spec| {
                let mult = spec.smash_charge_mult.max(1.0);
                spec.windows.iter().flat_map(move |window| {
                    window
                        .volumes
                        .iter()
                        .map(move |volume| (*name, (volume.damage as f32 * mult).ceil() as u32))
                })
            })
        })
        .max_by_key(|(_, damage)| *damage)
        .expect("the cast authors hit volumes");

    let pool = ambition_demo_smash::shark_ride::SUMMON_SHARK_HEALTH;
    assert!(
        pool > worst,
        "the recovery shark carries {pool} HP and the biggest single hit in the \
         cast is {worst}, from `{who}` — a mount that dies to ONE connection is \
         not gimpable, it is deletable, and 'about three hits' is false"
    );
}

/// ⭐⭐ THE ADMIRAL FLIES THE SHARK AROUND. Jon: *"a test where the pirate —
/// headlessly — does its up-b in the real game, and then controls the shark to
/// fly around, and we test that we actually were able to fly around."*
///
/// ⛔⛔ WHY THIS IS NOT COVERED BY THE ARMS ABOVE. They hold ONE direction for
/// 60 ticks and ask whether the pair moved. That passes on a shark with momentum
/// and no steering, on a shark that is falling, and on a pair drifting together
/// because the weld pins them — three ways to travel 40px while answering
/// nothing the stick said. Flight is the claim that the MOUNT OBEYS, and the only
/// evidence for obedience is REVERSAL: the same body going the other way when
/// the stick does, and going UP against gravity when asked.
///
/// ⛔⛔ AND IT WATCHES EVERY TICK RATHER THAN THE ENDPOINTS. Jon's failure was a
/// ride that ended ~20ms after it began; an assertion at the end of a 150-tick
/// leg cannot tell "flown for two and a half seconds" from "put down instantly
/// and walked there". The loop below names the tick the link broke and the
/// mount's own pool at that moment, so a red here reads as a diagnosis.
#[test]
fn the_admiral_flies_the_shark_around_the_stage_under_his_own_stick() {
    use ambition_demo_smash::select::{SlotOccupant, SmashRoster, SmashSelect};
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::ControlFrame;
    use ambition_platformer2d::mount::RidingOn;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    // ── THE ROAD A PLAYER TRAVELS: pick off the grid, not `smash_roster`. The
    //    up-B shipped broken twice on this road while the shortcut stayed green.
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_SELECT_ROUTE,
        )));
    for _ in 0..120 {
        app.update();
    }
    let admiral_index = {
        let grid = app
            .world()
            .get_resource::<SmashRoster>()
            .expect("the select screen assembled its grid");
        grid.0
            .iter()
            .position(|id| id == ambition_demo_smash::SMASH_SHARK_RIDER)
            .unwrap_or_else(|| {
                panic!("the shark rider is not on the grid: {:?}", grid.0)
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
        assert!(select.ready(), "two decided seats did not make a startable match");
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::select_screen::StartRequested(true));
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(live, "the opening ceremony never released the cast");
    }
    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven, so every control frame below reaches nobody"
    );

    // ── UP-B FROM THE AIR, which is how a recovery is pressed. ──
    ambition_platformer2d::sim::drive_control_frame(
        app.world_mut(),
        ControlFrame { jump_pressed: true, jump_held: true, ..Default::default() },
    );
    app.update();
    for _ in 0..12 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame { jump_held: true, ..Default::default() },
        );
        app.update();
    }
    let up_special = ControlFrame {
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
            ControlFrame { special_pressed: false, ..up_special },
        );
        app.update();
    }
    for _ in 0..20 {
        app.update();
    }

    let mount = app
        .world()
        .get::<RidingOn>(seat0)
        .map(|riding| riding.mount)
        .expect(
            "the admiral is not on a shark after an airborne up-B, so nothing \
             below is about flying one",
        );

    // ⭐ ONE LEG OF FLIGHT: hold a stick, step the app, and refuse to reach the
    // end of the leg with the pair broken. Returns how far the RIDER travelled
    // on each axis, which is the fact each assertion below is about.
    //
    // ⛔ THE RIDER'S POSITION, NOT THE MOUNT'S, because the rider is what the
    // player is: a mount that flies away from underneath its rider is a failed
    // weld, not a flight, and reading the mount would call it a success.
    let fly = |app: &mut App, leg: &str, ticks: usize, frame: ControlFrame| -> ambition_platformer2d::engine_core::Vec2 {
        let from = app
            .world()
            .get::<BodyKinematics>(seat0)
            .expect("the rider has kinematics")
            .pos;
        for tick in 0..ticks {
            ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
            app.update();
            if app.world().get::<RidingOn>(seat0).is_none() {
                // ⭐⭐ THE POOL AT THE MOMENT IT BROKE. "He came off" is the same
                // sentence for a mount that died, a lease that ran out and a
                // rider that bailed, and Jon spent a day inside that ambiguity.
                let pool = app
                    .world()
                    .get::<BodyHealth>(mount)
                    .map(|h| format!("{}/{} ({} damage taken)", h.current(), h.max(), h.damage_taken()))
                    .unwrap_or_else(|| "the mount no longer exists".to_string());
                panic!(
                    "the ride ended {tick} tick(s) into the `{leg}` leg — the \
                     admiral cannot fly the shark around because he is not on it \
                     for the trip. The mount's pool at that moment: {pool}. A \
                     full pool says he was PUT OFF (lease, launch or bail); a \
                     drained one says the mount was KILLED under him; a 0/0 says \
                     it was never alive."
                );
            }
        }
        app.world()
            .get::<BodyKinematics>(seat0)
            .expect("the rider has kinematics")
            .pos
            - from
    };

    // ── RIGHT, THEN LEFT. The reversal is the whole proof: momentum, gravity and
    //    a passive weld all travel, and none of them turn around when the stick
    //    does. ──
    // ⛔ 40px IN 45 TICKS (0.75s) IS UNAMBIGUOUS TRAVEL, not drift, and the bar is
    // the same for both legs on purpose. Measured 2026-08-27: +76.7 right, -57.4
    // left. The asymmetry is expected and is NOT worth equalising — the second leg
    // spends part of its budget reversing momentum the first leg built, so a bar
    // tuned to the outbound number would be a bar the return leg fails for a
    // reason that is physics rather than a defect.
    let right = fly(&mut app, "hold right", 45, ControlFrame { axis_x: 1.0, ..Default::default() });
    assert!(
        right.x > 40.0,
        "holding right moved the pair {:.1}px on x — the mount is not answering \
         the rider's stick, which is the whole of 'fly around'",
        right.x
    );
    let left = fly(&mut app, "hold left", 45, ControlFrame { axis_x: -1.0, ..Default::default() });
    assert!(
        left.x < -40.0,
        "holding left moved the pair {:.1}px on x after holding right moved it \
         {:.1}px — a ride that only ever travels one way is carrying momentum, \
         not taking direction",
        left.x,
        right.x
    );

    // ── AND UP, AGAINST GRAVITY. `+y` is gravity-DOWN, so a recovery that
    //    recovers makes y SMALLER. This is the arm that separates a mount from a
    //    fancy fall: everything above is true of a body drifting sideways as it
    //    sinks. ──
    let up = fly(&mut app, "hold up", 60, ControlFrame { axis_y: -1.0, ..Default::default() });
    assert!(
        up.y < -40.0,
        "holding up moved the pair {:.1}px on y (`+y` is gravity-down, so a climb \
         is NEGATIVE) — the shark is not carrying its rider upward, and a recovery \
         that cannot gain height is not a recovery",
        up.y
    );

    // ── THE WELD HELD FOR THE WHOLE TRIP. ──
    {
        let world = app.world_mut();
        let rider = world
            .get::<BodyKinematics>(seat0)
            .expect("the rider has kinematics")
            .pos;
        let saddle = world
            .get::<BodyKinematics>(mount)
            .expect("the mount the admiral is still riding has kinematics")
            .pos;
        let gap = saddle.distance(rider);
        assert!(
            gap < 200.0,
            "after 150 ticks of flight the rider sits {gap:.1}px from the shark \
             he is riding — the pair travelled, but not together"
        );
    }
    // ── AND IT IS THE SAME SHARK, not a second one summoned mid-flight. ──
    assert_eq!(
        app.world().get::<RidingOn>(seat0).map(|riding| riding.mount),
        Some(mount),
        "the admiral finished the trip on a DIFFERENT shark than he started it \
         on, so the legs above measured two rides rather than one flight"
    );
    // ── AND NOTHING TOOK A BITE OUT OF IT. This is the arm that speaks directly
    //    to what Jon saw in play: his shark died under him within a frame of
    //    boarding, with no `lethal blow` in the log to say what hit it. A pool
    //    that quietly loses health during an uneventful flight is that bug
    //    caught early. ──
    {
        let pool = app
            .world()
            .get::<BodyHealth>(mount)
            .copied()
            .expect("the mount the admiral is still riding has a health pool");
        assert_eq!(
            pool.damage_taken(),
            0,
            "the shark took {} damage across a flight in which nothing struck it \
             — the pool is being drained by something that is not a hit, which is \
             exactly the shape of a mount dying under its rider for no reason \
             anybody can read out of the log",
            pool.damage_taken()
        );
        assert!(
            pool.current() > 0,
            "the shark the admiral is still riding is at {}/{} — it is dead and \
             carrying him anyway",
            pool.current(),
            pool.max()
        );
    }
}

/// ⭐⭐ A SUMMON'S DEATH IS NOT WRITTEN DOWN — the bug Jon hit in play, and the
/// only one in this file that a fresh save cannot reproduce.
///
/// ⛔⛔ WHAT WENT WRONG. A summoned body inherits its character's
/// `RespawnPolicy`, which DEFAULTS to `DeadStaysDead`: on death, set the save
/// flag `enemy_<id>_dead` forever. That is a sentence about a PLACEMENT — one
/// authored actor, in one room, that the player killed. A summon has no room, is
/// built fresh on every press, and every instance shares ONE `config.id`. So the
/// first recovery shark that ever died wrote `enemy_smash_ride_shark_dead`, and
/// `sync_ecs_actors_with_save` — which runs EVERY SIM TICK, not on load — zeroed
/// the pool of every shark summoned afterwards on its first tick.
///
/// ⛔⛔ AND IT IS INVISIBLE FROM THE LOG. Nothing hit the shark, so no `lethal
/// blow` line exists to point at; the mount is simply dead the tick after the
/// rider boards it, and the link enforcer can only report that the pool reached
/// zero. Jon's log reads exactly that, five times, at five different positions.
///
/// ⭐ THE FLAG IS SET BY HAND HERE because a test that first kills a shark would
/// be measuring the WRITE and the READ at once, and the write is the half that
/// is easy to move. This states the world a returning player wakes up in: the
/// save already carries the flag.
#[test]
fn a_shark_summoned_into_a_save_that_remembers_a_dead_one_is_still_alive() {
    use ambition_demo_smash::select::{SlotOccupant, SmashRoster, SmashSelect};
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::ControlFrame;
    use ambition_platformer2d::mount::RidingOn;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    // ⛔⛔ THE FLAG THE OLD POLICY WOULD HAVE WRITTEN, keyed the way the death
    // path keys it: `enemy_<config.id>_dead`, and the summon's `config.id` is the
    // `SummonSpec` id — one fixed string for every shark this move ever makes.
    {
        let mut save = app
            .world_mut()
            .get_resource_mut::<AmbitionGameSave>()
            .expect("the shipped composition carries a save");
        save.data_mut().set_flag(
            &format!("enemy_{}_dead", ambition_demo_smash::shark_ride::SUMMON_SHARK_ID),
            true,
        );
    }

    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_SELECT_ROUTE,
        )));
    for _ in 0..120 {
        app.update();
    }
    let admiral_index = {
        let grid = app
            .world()
            .get_resource::<SmashRoster>()
            .expect("the select screen assembled its grid");
        grid.0
            .iter()
            .position(|id| id == ambition_demo_smash::SMASH_SHARK_RIDER)
            .unwrap_or_else(|| panic!("the shark rider is not on the grid: {:?}", grid.0))
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
        assert!(select.ready(), "two decided seats did not make a startable match");
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::select_screen::StartRequested(true));
    {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(live, "the opening ceremony never released the cast");
    }
    // ⛔ THE PREMISE: the flag survived the route change. A save the shell reset
    // on its way into the match would make every arm below pass for the wrong
    // reason — there would be nothing to be poisoned BY.
    assert!(
        app.world()
            .get_resource::<AmbitionGameSave>()
            .expect("the save is still there")
            .data()
            .flag(&format!(
                "enemy_{}_dead",
                ambition_demo_smash::shark_ride::SUMMON_SHARK_ID
            )),
        "the match cleared the dead flag on its way in, so this test is riding a \
         shark nothing was ever going to kill"
    );

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    let up_special = ControlFrame {
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
            ControlFrame { special_pressed: false, ..up_special },
        );
        app.update();
    }
    for _ in 0..20 {
        app.update();
    }

    let mount = app
        .world()
        .get::<RidingOn>(seat0)
        .map(|riding| riding.mount)
        .expect(
            "the admiral is not on a shark in a save that remembers a dead one — \
             which is the failure Jon reported in play, and the reason no test \
             saw it is that every test here starts from a save that has never \
             lost a shark",
        );
    let pool = app
        .world()
        .get::<BodyHealth>(mount)
        .copied()
        .expect("the shark he is riding has a health pool");
    assert!(
        pool.current() > 0,
        "the summoned shark is at {}/{} on the tick after it was built — a save \
         flag from a PREVIOUS shark's death is deciding this one's liveness, and \
         no survivability number can answer that",
        pool.current(),
        pool.max()
    );

    // ⛔⛔ AND IT STAYS ALIVE, because the sweep that zeroes it runs every sim
    // tick rather than once at load: a body that survived construction can still
    // be killed on tick two, and asserting only at the moment of boarding would
    // pass against a bug that arrives one frame later.
    for _ in 0..90 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame::default(),
        );
        app.update();
    }
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "the admiral was put off within a second and a half of boarding a shark \
         summoned into a poisoned save"
    );
    let pool = app
        .world()
        .get::<BodyHealth>(mount)
        .copied()
        .expect("the shark he is still riding has a health pool");
    assert_eq!(
        pool.damage_taken(),
        0,
        "nothing struck the shark and it has taken {} damage, so its pool is \
         being written by something that is not combat",
        pool.damage_taken()
    );
}
