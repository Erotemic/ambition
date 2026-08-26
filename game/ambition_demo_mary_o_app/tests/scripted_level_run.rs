//! Deterministic seam test for Mary-O level 1.
//!
//! The run drives the real control seam and verifies movement, pipe travel,
//! coin collection, and HUD tally integration. It uses `transit_body` for a few
//! direct placements, so it is not the full level-1 acceptance gate: it does
//! not traverse the whole level under input or exercise a powerup through the
//! shared pickup/equipment path.

use ambition_demo_mary_o::MaryOLevelState;
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// The scripted stick.

/// Drive one frame with the given control frame.
fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn hold_right() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_x = 1.0;
    frame.right_pressed = true;
    frame
}

/// Press DOWN (screen-down = `axis_y > 0`): the verb that drops you INTO the entry
/// pipe. The warp is directional, so a plain Interact never warps.
fn press_down() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_y = 1.0;
    frame
}

/// Press UP (screen-up = `axis_y < 0`): the verb that surfaces you at the vault's
/// return pipe.
fn press_up() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_y = -1.0;
    frame
}

/// Her collider, so a setup beat can stand her ON a face instead of dropping her
/// centre onto it — half a body inside a pipe is not a place a player can be.
fn player_size(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<PrimaryPlayer>>(
        );
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .size
}

fn player_pos(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<PrimaryPlayer>>(
        );
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .pos
}

/// Move the player for SETUP, as a discrete transit rather than a position poke.
///
/// Writing `kin.pos` directly is the exact anti-pattern this demo's own warp
/// calls out (`ambition_demo_mary_o::lib.rs`, ADR 0024): the motion model keeps
/// private attachment and ledge state that belongs to the DEPARTURE point, so a
/// raw poke can start a beat still clinging to a wall that is no longer there.
/// `transit_body` is the engine authority and reconciles that state — the same
/// thing `SimHarness::teleport_player` does for harness-based tests.
fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

fn level(app: &mut App) -> (i8, u32, f32) {
    let mut query = app.world_mut().query::<&MaryOLevelState>();
    let state = query
        .iter(app.world())
        .next()
        .expect("the mode owner exists in gameplay");
    (state.lives, state.score, state.time_remaining)
}

fn wallet(app: &mut App) -> i32 {
    app.world()
        .resource::<ambition_platformer2d::sim_view::PlayerHudFacts>()
        .balance
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// Assert that scripted input survives the composed participant pipeline.
/// Probe `ControlFrame` immediately after `Update`: the scripted delivery counter
/// observes the slot table from `FixedUpdate` and can lag the first press. The
/// scripted writer is ordered after normal input routing so a composed input
/// feature cannot erase the test press.
#[track_caller]
fn assert_scripted_input_reaches_the_sim(app: &mut App) {
    let probe = ControlFrame {
        aim_x: 1.0,
        ..ControlFrame::default()
    };
    for _ in 0..20 {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
            .0 = probe;
        app.update();
        if app.world().resource::<ControlFrame>().aim_x > 0.5 {
            app.world_mut()
                .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
                .0 = ControlFrame::default();
            app.update();
            return;
        }
    }
    panic!(
        "a scripted press did not survive into the simulation, so every assertion \
         after this point would pass on a body nobody was driving"
    );
}

/// `ManualDuration` pins the sim clock, which makes the sim deterministic — but
/// it does NOT pin boot. Session activation and asset loading advance on real
/// I/O over a variable number of frames, so "8 frames is enough to be playing"
/// is a bet on machine load, and it loses: under a parallel `./run_tests.sh`
/// this run flaked with the body at spawn `x` and a falling `y` — gravity
/// integrating while input was still gated, i.e. boot had not finished and the
/// walk was scripted into a suspended game.
///
/// A live, DECREASING clock is the honest readiness signal: it means the level
/// owner exists and the rules are ticking, which is exactly the precondition
/// "holding right moves her" depends on.
fn settle_until_playable(app: &mut App) {
    let mut previous = None;
    for _ in 0..600 {
        app.update();
        let mut query = app.world_mut().query::<&MaryOLevelState>();
        let Some(now) = query.iter(app.world()).next().map(|s| s.time_remaining) else {
            continue;
        };
        if previous.is_some_and(|before| now < before) {
            return;
        }
        previous = Some(now);
    }
    panic!("the demo never reached a playable level with a running clock");
}

/// what it uniquely covered is real and is NOT covered elsewhere: a whole
/// playthrough, spawn to flagpole, on the production schedule. The mechanics it
/// touches are covered against the authored level by unit probes (the bonk, the
/// stomp, the brick break, the warp), and 1-1's SHAPE is covered by invariants
/// (`the pit rhythm must widen`, `every authored enemy has ground under it`).
/// The gap is the end-to-end run, and it stays a gap until the fixture lands.
///
/// Queue row `G1 PICK 11`.
///
/// same cause as `she_plays_level_one_from_spawn_to_the_pole_and_it_replays`.
#[ignore = "route tuned to 1-1's old arrangement; replaced by a fixture course (queue G1 PICK 11)"]
#[test]
fn a_scripted_run_walks_takes_the_secret_banks_its_coins_and_finishes() {
    let mut app = build_demo_app();
    // DETERMINISM, and the reason this run is worth anything. One tick per update, always.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    settle_until_playable(&mut app);

    // Scripting the sim's input seam is only meaningful when the sim's input
    // seam is what feeds the sim. When a participant pipeline is composed it
    // legitimately owns `ControlFrame`, and driving the DEVICE layer is a
    // different claim that `app_it::participant_input` already owns. Skip
    // loudly rather than assert something this composition cannot answer.
    assert_scripted_input_reaches_the_sim(&mut app);

    // ── Boot lands in gameplay with a live level ────────────────────────────
    let (lives, score, time) = level(&mut app);
    assert_eq!(lives, 3, "a fresh run starts on three lives");
    assert_eq!(score, 0, "and no score");
    assert!(time > 0.0, "and a running clock");

    // ── She WALKS. Held right actually moves the body through the real
    //    control seam, which is the one thing a scripted run must not fake.
    let start = player_pos(&mut app);
    for _ in 0..60 {
        step(&mut app, hold_right());
    }
    let walked = player_pos(&mut app);
    assert!(
        walked.x > start.x + 32.0,
        "holding right moves her a real distance: {start:?} -> {walked:?}"
    );

    // ── The clock is a threat, not decoration ──────────────────────────────
    let (_, _, time_after) = level(&mut app);
    assert!(
        time_after < time,
        "the level clock counts down while she plays"
    );

    // ── The secret pipe ────────────────────────────────────────────────────
    //
    // Placed on the pipe rather than walked there: crossing two pits under
    // scripted input is a platforming-precision test, not a connectivity one,
    // and it would make this run fragile to any jump-arc tuning change. Where
    // she stands is set up; what the pipe DOES is the claim.
    // A mouth is a pipe's open FACE, and you enter one by TOUCHING it — so stand
    // her ON that face (the band's centre line), not with her centre on it, which
    // would bury half of her in the pipe's own collider.
    // She is a ONE-HIT body, and a Solid Snake patrols the corridor between the
    // two pipes — so standing on a mouth waiting out a transit is, for her,
    // standing still in a patrol lane. Whether she can handle that is the
    // ACCEPTANCE run's claim (she stomps it there, and arrives wearing the cap);
    // this run's claim is that the pipe WORKS. Take the enemy out of the
    // question the same way this file takes the pits out of it — by setting up
    // what is not being asked.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyHealth, With<PrimaryPlayer>>();
        let world = app.world_mut();
        for mut health in q.iter_mut(world) {
            health.health.invulnerable.set(
                ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED,
                true,
            );
        }
    }

    let half_body = player_size(&mut app).y * 0.5;
    place_player(&mut app, {
        let mouth = ambition_demo_mary_o::pipe_mouth();
        let face = (mouth.min.y + mouth.max.y) * 0.5;
        Vec2::new((mouth.min.x + mouth.max.x) * 0.5, face - half_body)
    });
    step(&mut app, ControlFrame::default());
    step(&mut app, press_down());
    // A warp is a MOVE: the press starts the slide in and out, which takes about
    // a second, so hold frames until she is through rather than reading the
    // world on the very next tick.
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }

    let vault = ambition_demo_mary_o::vault_bounds();
    let inside = player_pos(&mut app);
    assert!(
        inside.x > vault.min.x
            && inside.x < vault.max.x
            && inside.y > vault.min.y
            && inside.y < vault.max.y,
        "DOWN on the entry pipe drops her into the vault: {inside:?} vs {vault:?}"
    );

    // ── The vault pays out through the SHARED economy ──────────────────────
    //
    // No demo collection code exists; the coins are ordinary `currency`
    // placements. Walking the length of the vault should bank them, and the
    // balance is read from the same `PlayerHudFacts` the HUD's COINS readout
    // draws — so this covers the whole chain from placement to screen.
    // Walk only as far as the COINS, and stop short of the vault's far wall.
    //
    // The wallet assertion below still passed (the coins are collected long before the shaft), so
    // the run went green here and failed three beats later in a room where `vault_exit()` and
    // `goal_pole()` mean nothing — which read as a broken return pipe and was nothing of the kind.
    let before = wallet(&mut app);
    let surface = ambition_demo_mary_o::level_1_1();
    let far_wall = surface
        .world
        .blocks
        .iter()
        .find(|block| block.name == "vault_wall_1")
        .expect("the vault is closed by an authored `vault_wall_1`")
        .aabb;
    let stop_x = far_wall.min.x - player_size(&mut app).x;
    for _ in 0..240 {
        if player_pos(&mut app).x >= stop_x {
            break;
        }
        step(&mut app, hold_right());
    }
    let after = wallet(&mut app);
    assert!(
        after > before,
        "walking the vault collects its coins through the shared economy \
         ({before} -> {after}) — nothing in this demo collects them by hand"
    );

    // The premise the beats below depend on: she is STILL IN THE VAULT. Without
    // this, walking out of the room is indistinguishable from the return pipe
    // failing, and the next assertion blames the wrong mechanism.
    let after_walk = player_pos(&mut app);
    assert!(
        after_walk.x > vault.min.x
            && after_walk.x < vault.max.x
            && after_walk.y > vault.min.y
            && after_walk.y < vault.max.y,
        "the coin walk must leave her inside the vault: {after_walk:?} vs {vault:?}. \
         Past the far end is the descent shaft to World 1-2, and every beat after \
         this one is written about 1-1"
    );

    // ── And she can get back out ───────────────────────────────────────────
    // Same rule at the other end, upside down: the return pipe hangs from the
    // ceiling, so touching its mouth means her HEAD is at the lip.
    let half_body = player_size(&mut app).y * 0.5;
    place_player(&mut app, {
        let exit = ambition_demo_mary_o::vault_exit();
        let face = (exit.min.y + exit.max.y) * 0.5;
        Vec2::new((exit.min.x + exit.max.x) * 0.5, face + half_body)
    });
    step(&mut app, ControlFrame::default());
    step(&mut app, press_up());
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }
    let surfaced = player_pos(&mut app);
    assert!(
        surfaced.y < vault.min.y,
        "UP at the vault return pipe surfaces her above ground: {surfaced:?}"
    );

    // ── The flag ends the level, and the level cycles ──────────────────────
    //
    // Same reasoning as the pipe: reaching the pole is a traversal test the
    // reachability suites own. What matters here is that arriving at it runs
    // the sequence through to a settled tally and a fresh level.
    let pole = ambition_demo_mary_o::goal_pole();
    place_player(&mut app, Vec2::new(pole.x, pole.base_y - 48.0));
    for _ in 0..600 {
        step(&mut app, hold_right());
        let (_, _, remaining) = level(&mut app);
        if (remaining - ambition_demo_mary_o::STARTING_TIME).abs() < 0.001 {
            // The clock refilled: the tally settled and the level cycled.
            return;
        }
    }
    panic!("the flag sequence never settled into a level cycle within 10 seconds");
}

/// A spawned snake is really RECOGNISED by the demo that owns its shell.
///
/// The shell mechanic shipped broken once and every focused test was green,
/// because the fixtures hand-built a `Name` while the production spawner writes
/// `"Feature actor enemy: {name}"` onto `Name` and the bare name onto
/// `FeatureName`. The tag never fired, so the enemy spawned inert. This drives
/// the REAL spawn path — request in, engine spawns, demo tags — which is the
/// only thing that would have caught it.
#[test]
fn a_spawned_snake_is_tagged_by_the_demo_that_owns_its_shell() {
    use ambition_demo_mary_o::snake::{
        SnakeShell, SNAKE_BRAIN_KEY, SNAKE_DISPLAY_NAME, SNAKE_SHEET_TARGET,
    };

    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    settle(&mut app);

    let before = {
        let mut query = app.world_mut().query::<&SnakeShell>();
        query.iter(app.world()).count()
    };

    // Ask the engine for a snake exactly as the level's staging does.
    app.world_mut()
        .write_message(ambition_platformer2d::actors::features::SpawnActorRequest {
            id: "scripted_snake".to_string(),
            name: SNAKE_DISPLAY_NAME.to_string(),
            pos: Vec2::new(600.0, 300.0),
            half_size: Vec2::new(14.0, 16.0),
            faction: ambition_platformer2d::combat::components::ActorFaction::Enemy,
            grudge_against: None,
            kind: ambition_platformer2d::actors::features::SpawnActorKind::Enemy {
                brain: ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(
                    SNAKE_BRAIN_KEY.to_string(),
                ),
                // the character, exactly as 1-1's placements author it. The subject under test
                // is the demo's TAG pass, and it reads `ActorConfig.brain` either way.
                character: SNAKE_SHEET_TARGET.into(),
            },
        });
    settle(&mut app);

    let tagged = {
        let mut query = app.world_mut().query::<&SnakeShell>();
        query.iter(app.world()).count()
    };
    assert_eq!(
        tagged,
        before + 1,
        "the engine spawned the snake and the demo TAGGED it — an untagged snake \
         never shells, which is exactly how this shipped broken"
    );
}
