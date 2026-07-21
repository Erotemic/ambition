//! **The level-1 acceptance run.** One deterministic scripted play-through of
//! the real demo app, through the real control seam.
//!
//! This is the demo's acceptance gate, and it is deliberately not a unit test of
//! anything: every piece it touches already has its own focused coverage. What
//! it proves is that the pieces are CONNECTED — that a player holding right
//! actually reaches the pipe, that the pipe actually drops her into the vault,
//! that the vault's coins are actually collectable by the shared economy, and
//! that the tally actually reaches the HUD. Each of those is a seam between two
//! systems owned by different crates, and a seam is exactly what a focused test
//! cannot see.
//!
//! It drives `ControlFrame` — the sim's input seam — rather than poking
//! positions, because "she can get there" is the claim. Where it does place her
//! directly, it says so and explains why.
//!
//! # Why this is gated off the `input` feature
//!
//! Under `input` the participant pipeline OWNS `ControlFrame`: it repopulates it
//! from device state every frame, so a scripted write is erased before the sim
//! sees it. That is correct — a real device is the authority when one is
//! composed — and it means scripting at this seam is only meaningful in the
//! headless sim composition, which is the one a deterministic run wants anyway.
//! Driving the participant/device layer itself is a different claim, and
//! `app_it::participant_input` already owns it.
#![cfg(not(feature = "input"))]

use ambition::input::ControlFrame;
use ambition::platformer::markers::PrimaryPlayer;
use ambition_demo_mary_o::MaryOLevelState;
use ambition_demo_mary_o_app::build_demo_app;
use bevy::prelude::*;

/// The scripted stick. Republished every frame in `PreUpdate`, because Bevy
/// runs the fixed-timestep loop BEFORE `Update` — intent written any later is
/// not seen by the tick it was meant to drive.
#[derive(Resource, Clone, Copy, Default)]
struct ScriptedStick(ControlFrame);

fn apply_scripted_stick(stick: Res<ScriptedStick>, mut frame: ResMut<ControlFrame>) {
    *frame = stick.0;
}

/// Drive one frame with the given control frame.
fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ScriptedStick>().0 = frame;
    app.update();
}

fn hold_right() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_x = 1.0;
    frame.right_pressed = true;
    frame
}

fn press_interact() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.interact_pressed = true;
    frame
}

fn player_pos(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition::engine_core::BodyKinematics, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .pos
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app
        .world_mut()
        .query_filtered::<&mut ambition::engine_core::BodyKinematics, With<PrimaryPlayer>>();
    let world = app.world_mut();
    let mut kin = query.iter_mut(world).next().expect("a primary player");
    kin.pos = pos;
    kin.vel = Vec2::ZERO;
}

fn level(app: &mut App) -> (u8, u32, f32) {
    let mut query = app.world_mut().query::<&MaryOLevelState>();
    let state = query
        .iter(app.world())
        .next()
        .expect("the mode owner exists in gameplay");
    (state.lives, state.score, state.time_remaining)
}

fn wallet(app: &mut App) -> i32 {
    app.world()
        .resource::<ambition::sim_view::PlayerHudFacts>()
        .balance
}

fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// The run: boot into gameplay, walk, take the secret pipe, bank its coins,
/// surface, and finish on the flag.
#[test]
fn a_scripted_run_walks_takes_the_secret_banks_its_coins_and_finishes() {
    let mut app = build_demo_app();
    // DETERMINISM, and the reason this run is worth anything. The demo is a
    // fixed-tick host, so without a manual clock the number of sim ticks per
    // `app.update()` depends on how fast the test machine got round the loop —
    // the same script then walks a different distance on every run, and the
    // suite fails only sometimes. One tick per update, always.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_resource::<ScriptedStick>();
    app.add_systems(PreUpdate, apply_scripted_stick);
    settle(&mut app);

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
    place_player(&mut app, ambition_demo_mary_o::pipe_arrival());
    step(&mut app, ControlFrame::default());
    step(&mut app, press_interact());
    settle(&mut app);

    let vault = ambition_demo_mary_o::vault_bounds();
    let inside = player_pos(&mut app);
    assert!(
        inside.x > vault.min.x
            && inside.x < vault.max.x
            && inside.y > vault.min.y
            && inside.y < vault.max.y,
        "Interact on the pipe drops her into the vault: {inside:?} vs {vault:?}"
    );

    // ── The vault pays out through the SHARED economy ──────────────────────
    //
    // No demo collection code exists; the coins are ordinary `currency`
    // placements. Walking the length of the vault should bank them, and the
    // balance is read from the same `PlayerHudFacts` the HUD's COINS readout
    // draws — so this covers the whole chain from placement to screen.
    let before = wallet(&mut app);
    for _ in 0..240 {
        step(&mut app, hold_right());
    }
    let after = wallet(&mut app);
    assert!(
        after > before,
        "walking the vault collects its coins through the shared economy \
         ({before} -> {after}) — nothing in this demo collects them by hand"
    );

    // ── And she can get back out ───────────────────────────────────────────
    place_player(&mut app, {
        let exit = ambition_demo_mary_o::vault_exit();
        (exit.min + exit.max) * 0.5
    });
    step(&mut app, ControlFrame::default());
    step(&mut app, press_interact());
    settle(&mut app);
    let surfaced = player_pos(&mut app);
    assert!(
        surfaced.y < vault.min.y,
        "Interact at the vault exit surfaces her above ground: {surfaced:?}"
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

/// **A stomped crony really does leave a workable shell.**
///
/// The shell shipped broken and every focused test was green, because the
/// fixtures hand-built a `Name` while the production spawner writes
/// `"Feature actor enemy: {name}"` onto `Name` and the bare name onto
/// `FeatureName`. The tag never fired, so shells spawned inert. This drives the
/// REAL spawn path — request in, engine spawns, demo tags — which is the only
/// thing that would have caught it.
#[test]
fn a_stomped_crony_leaves_a_shell_the_demo_actually_recognises() {
    use ambition_demo_mary_o::crony::{MaryOShell, SHELL_DISPLAY_NAME};

    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    settle(&mut app);

    // Ask the engine for a shell exactly as the stomp does.
    app.world_mut()
        .write_message(ambition::actors::features::SpawnActorRequest {
            id: "scripted_shell".to_string(),
            name: SHELL_DISPLAY_NAME.to_string(),
            pos: Vec2::new(600.0, 300.0),
            half_size: Vec2::new(14.0, 12.0),
            faction: ambition::actors::combat::components::ActorFaction::Enemy,
            grudge_against: None,
            kind: ambition::actors::features::SpawnActorKind::Enemy {
                brain: ambition::entity_catalog::placements::CharacterBrain::Custom(
                    ambition_demo_mary_o::crony::SHELL_BRAIN_KEY.to_string(),
                ),
            },
        });
    settle(&mut app);

    let tagged = {
        let mut query = app.world_mut().query::<&MaryOShell>();
        query.iter(app.world()).count()
    };
    assert_eq!(
        tagged, 1,
        "the engine spawned the shell and the demo TAGGED it — an untagged \
         shell is an inert prop, which is exactly how this shipped broken"
    );
}
