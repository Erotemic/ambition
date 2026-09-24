//! Seat a real smash match, press up-B, and let the engine's own log say what
//! happened.
//!
//! The integration suite can seat a match and drive a press, but
//! `build_visible_app` drops `LogPlugin` from windowless modes (tests build
//! several Apps per process, and the tracing subscriber is process-global),
//! so engine diagnostics are invisible there. `capture_scene` keeps the log
//! but cannot seat anybody. This binary does both: one App (so the global
//! subscriber is safe), the demo's roster builder, and a driven control frame.

use bevy::prelude::*;

fn main() {
    // Add the log plugin in the compose hook. A `NoWindow` build finishes and
    // cleans up its plugins before returning, and Bevy 0.19 panics on
    // `add_plugins` after that ("Plugins cannot be added after
    // App::cleanup() or App::finish() has been called"). One App, one
    // process, so the global subscriber is safe.
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
    // Wait for the round, not for a frame count. A count encodes the
    // ceremony's length, and dev mode (D248) runs it 10x fast. The condition
    // is observable: a cast exists, and nothing in it is held by
    // `ControlHolds`.
    {
        let mut live = false;
        for _ in 0..900 {
            app.update();
            let (seated, held) = {
                let world = app.world_mut();
                let mut all = world.query::<&ambition_platformer2d::versus_match::MatchSeat>();
                let seated = all.iter(world).count();
                let mut q = world.query_filtered::<
                    &ambition_platformer2d::versus_match::MatchSeat,
                    bevy::prelude::With<ambition_platformer2d::characters::control::ControlHolds>,
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

    // One press frame, then held. `special_pressed` is a rising edge; holding
    // it true would press every tick.
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
    // Someone swings. A passive match proves only that the ride ends; the
    // failure to test is a shark deleted by a hit. This lands the admiral's
    // forward smash: 17 damage x `smash_charge_mult` 1.7 = 29, the hardest hit
    // the fighter under test can produce. It is not the hardest in the game
    // (George Booul's is 21 x 1.7 = 36);
    // `a_recovery_mount_cannot_be_deleted_by_one_hit` checks the whole cast.
    //
    // It waits for a ride instead of assuming one: a swing right after the
    // press lands before the boarding.
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
                // The hostile side: a `Player`-sourced strike on a
                // Player-faction body is friendly fire and is refused.
                source: ambition_platformer2d::vfx::HitSide::Enemy,
                // Anchored to the rider, which is welded to the mount, so a
                // moving pair cannot outrun a stored point.
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
    // A verdict, not a pile of lines. This is not a test (nothing fails a
    // build); it says plainly what it saw.
    let world = app.world_mut();
    let mut mounts = world
        .query_filtered::<Entity, bevy::prelude::With<ambition_platformer2d::mount::Mountable>>();
    let all: Vec<Entity> = mounts.iter(world).collect();
    let left_over = all.len();
    // A departing shark and a loitering shark look the same in a count.
    // `Departing` is the difference.
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
