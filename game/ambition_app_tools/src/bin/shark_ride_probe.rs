//! Seat a real smash match, press up-B, and let the ENGINE'S OWN LOG say what
//! happened.
//!
//! ⛔⛔ THE INSTRUMENT THAT DID NOT EXIST, and its absence cost a day. The
//! integration suite can seat a match and drive a press, but `build_visible_app`
//! drops `LogPlugin` from every windowless mode — *"tests build several Apps per
//! process; the tracing subscriber is process-global"* — so every diagnostic the
//! engine emits is invisible there. Meanwhile `capture_scene`, which DOES keep
//! the log, cannot seat anybody: its own notes record `--route smash_gameplay`
//! photographing an empty stage.
//!
//! So a real bug lived in the gap: the suite could reach the behaviour and not
//! see the log; the capture could see the log and not reach the behaviour. Four
//! wrong hypotheses were argued across that gap before a player's log settled it.
//!
//! ⭐ THIS BINARY IS THE OVERLAP. One App, so the process-global subscriber is
//! safe — the same reasoning `capture_scene` already writes down — plus the
//! demo's own roster builder and a driven control frame.

use bevy::prelude::*;

fn main() {
    // ⛔ THE LOG PLUGIN GOES IN THE COMPOSE HOOK, NOT AFTER. A `NoWindow` build
    // finishes and cleans up its plugins before returning (so `Plugin::finish`
    // runs, which `App::update()` never does), and Bevy 0.19 PANICS on
    // `add_plugins` after that: "Plugins cannot be added after App::cleanup()
    // or App::finish() has been called." This binary did exactly that and died
    // on its third line.
    // ⭐ ONE APP, ONE PROCESS: the exact condition that makes a global tracing
    // subscriber safe here and unsafe in the test binary.
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |app| {
            app.add_plugins(bevy::log::LogPlugin::default());
        },
    );

    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // ⛔⛔ WAIT FOR THE ROUND, NOT FOR A NUMBER. 240 was a count a little longer
    // than the 3-second opening ceremony, so it silently encoded the ceremony's
    // LENGTH — and D248's dev mode runs that ceremony 10x fast, which has now
    // broken four fixture families the same way. The condition is observable: a
    // cast exists, and nothing in it is still held by `ScriptedControl`.
    {
        let mut live = false;
        for _ in 0..900 {
            app.update();
            let (seated, held) = {
                let world = app.world_mut();
                let mut all =
                    world.query::<&ambition_platformer2d::actors::character_runtime::MatchSeat>();
                let seated = all.iter(world).count();
                let mut q = world.query_filtered::<
                    &ambition_platformer2d::actors::character_runtime::MatchSeat,
                    bevy::prelude::With<ambition_platformer2d::characters::control::ScriptedControl>,
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

    // ⛔ ONE press FRAME, then HELD. `special_pressed` is a rising EDGE: holding
    // it true would be a press every tick.
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
    // ⛔⛔ AND SOMEBODY SWINGS. A passive match proves the ride ENDS correctly and
    // says nothing about the failure Jon actually hit, which was a shark deleted
    // by a hit. What lands here is the admiral's forward smash — 17 damage x
    // `smash_charge_mult` 1.7 = 29.
    //
    // ⚠ IT IS NOT THE WORST CONNECTION IN THE GAME, and this comment said it was
    // until 2026-08-27. George Booul's forward smash is 21 x 1.7 = 36. The claim
    // came from censusing the admiral's own moveset, which is the same mistake
    // `a_recovery_mount_cannot_be_deleted_by_one_hit` was making — that test
    // reads the whole cast now and is where the real floor is enforced. 29 is
    // still the right thing for THIS probe to throw: it is the hardest hit the
    // fighter under test can produce on himself, which is what a self-contained
    // ride probe can stage.
    //
    // ⛔ IT WAITS FOR A RIDE RATHER THAN ASSUMING ONE. The first version swung
    // once, immediately after the press, and the strike never landed because the
    // board had not happened yet on that frame -- a swing at nobody, reported as
    // a clean run.
    let mut struck = false;
    for _ in 0..120 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
        if struck {
            continue;
        }
        let world = app.world_mut();
        let mut seats = world.query::<(Entity, &ambition_platformer2d::actor::MatchSeat)>();
        let rows: Vec<(Entity, usize)> = seats.iter(world).map(|(e, s)| (e, s.0)).collect();
        let Some(rider) = rows.iter().find(|(_, s)| *s == 0).map(|(e, _)| *e) else {
            continue;
        };
        let Some(rival) = rows.iter().find(|(_, s)| *s == 1).map(|(e, _)| *e) else {
            continue;
        };
        if world
            .get::<ambition_platformer2d::mount::RidingOn>(rider)
            .is_none()
        {
            continue;
        }
        world.spawn((
            ambition_platformer2d::combat::strike::Hitbox {
                owner: rival,
                // The HOSTILE side: a `Player`-sourced strike on a Player-faction
                // body reads as friendly fire and is refused.
                source: ambition_platformer2d::vfx::HitSide::Enemy,
                // Anchored to the RIDER, which is welded to the mount, so a
                // moving pair cannot outrun a remembered point.
                anchor: ambition_platformer2d::combat::strike::HitboxAnchor::FollowOwner {
                    local_offset: ambition_platformer2d::engine_core::Vec2::ZERO,
                },
                half_extent: ambition_platformer2d::engine_core::Vec2::new(400.0, 400.0),
                shape: None,
                facing: 1.0,
                damage: 29,
                knockback: ambition_platformer2d::combat::strike::HitboxKnockback::FeelScale(0.0),
                launch_dir: None,
                frame_down: ambition_platformer2d::engine_core::Vec2::new(0.0, 1.0),
                reaction: None,
                strike_sfx: None,
            },
            ambition_platformer2d::combat::strike::HitboxHits::default(),
        ));
        struck = true;
        eprintln!("shark_ride_probe: landed a 29-damage strike on the ridden pair");
    }
    // Long enough for the whole ride: board, five seconds of lease, departure.
    for _ in 0..600 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }
    // ⭐ A VERDICT, NOT A PILE OF LINES. This binary exists so a person can see
    // what the engine did; making them count `boarded:` lines to find out puts
    // the reading back on them. ⛔ It is still not a TEST — nothing here fails a
    // build — it just says plainly what it saw.
    let world = app.world_mut();
    let mut mounts = world
        .query_filtered::<Entity, bevy::prelude::With<ambition_platformer2d::mount::Mountable>>();
    let all: Vec<Entity> = mounts.iter(world).collect();
    let left_over = all.len();
    // A shark on its way out and a shark loitering look the same in a count, and
    // "boxes piling up" was the original report. `Departing` is the difference.
    let departing = all
        .iter()
        .filter(|e| {
            world
                .get::<ambition_demo_smash::shark_ride::Departing>(**e)
                .is_some()
        })
        .count();
    let ridden = all
        .iter()
        .filter(|e| {
            world
                .get::<ambition_platformer2d::mount::MountSlot>(**e)
                .is_some_and(|slot| slot.rider.is_some())
        })
        .count();
    let loitering = left_over.saturating_sub(departing + ridden);
    let mut riders = world.query::<&ambition_platformer2d::mount::RidingOn>();
    let still_riding = riders.iter(world).count();
    eprintln!(
        "shark_ride_probe: {left_over} shark(s) on the stage - {ridden} ridden, {departing} departing, {loitering} LOITERING; {still_riding} rider(s) aboard"
    );
    if loitering > 0 {
        eprintln!(
            "shark_ride_probe: WARN {loitering} shark(s) are neither ridden nor leaving, which is the 'boxes piling up' shape"
        );
    }
    eprintln!(
        "shark_ride_probe: read the log above for `boarded:`,          `shark departing (rider left)` and — if anything went wrong —          `mount DIED under its rider`, `mount VANISHED from the saddle lookup`          or `lethal blow: damage=N`"
    );
    eprintln!("shark_ride_probe: done");
}
