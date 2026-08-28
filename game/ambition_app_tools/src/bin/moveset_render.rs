//! ⭐⭐ RENDER A FIGHTER ACTUALLY PERFORMING A MOVE, one PNG per exact sim tick.
//!
//! Jon asked for this at the start: *"we will see things like the pirate flying
//! around on the shark."* The inspector's GPU endpoint has until now photographed
//! a fighter STANDING in `hall_of_characters`, because nothing drove a move.
//!
//! ⛔⛔ IT OWNS ITS OWN LOOP, WHICH IS WHY IT IS NOT `capture_scene`. That tool
//! calls `App::run()`, so the RUNNER decides what a frame costs — and its
//! `--frames` cadence proves the cost: `request_capture` returns early while a
//! readback is pending but the app keeps updating, so shots are spaced by
//! `stride + however long the GPU took`. For a room burst that is invisible; for
//! a move it means startup or active frames pass while a PNG is in flight.
//!
//! ⭐ SO SIMULATION TIME AND GPU TIME ARE SEPARATED. The sim advances only at the
//! canonical manual period; a readback is serviced with `ManualDuration(ZERO)`,
//! which runs the schedules and moves no clock. Measured by the spike: a real
//! offscreen readback completes in three zero-time pumps with `SimTick` frozen.
//! Every PNG therefore names the tick it was taken on, and GPU latency cannot
//! change which ticks are captured.
//!
//! ⛔ AND IT REPORTS WHAT ACTUALLY CAME OUT. A press is a REQUEST; the engine
//! decides. The manifest carries the intended move and the observed ones, and a
//! mismatch is reported rather than cached under the name that was asked for.

#[path = "support/move_exercise.rs"]
mod move_exercise;

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;
use move_exercise::{verb_named, VERBS};

const USAGE: &str = "\
moveset_render — render a fighter performing one move, one PNG per simulation tick.

USAGE:
    moveset_render --character ID --verb VERB [--out DIR] [--frames N] [--stride K]

OPTIONS:
    --character ID   catalog id of the fighter
    --verb VERB      repertoire verb to perform (see below)
    --out DIR        directory for the PNGs and manifest.json  [default: /tmp/moveset_render]
    --frames N       how many pictures                          [default: 24]
    --stride K       simulation ticks between pictures          [default: 1]
    -h, --help       print this and exit

NOTES:
    Needs a GPU: it boots the real OffscreenGpu composition and reads pixels back.

    Every PNG names the exact `SimTick` it was captured on, and the manifest
    records the intended move against what the engine actually played. A press
    is a request; if the move that came out is not the one asked for, that is
    reported rather than cached under the requested name.
";

fn sim_tick(app: &App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    let arg = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    if let Some(bad) = args
        .iter()
        .skip(1)
        .filter(|a| a.starts_with('-'))
        .find(|a| {
            !matches!(
                a.as_str(),
                "--character" | "--verb" | "--out" | "--frames" | "--stride"
            )
        })
    {
        eprintln!("moveset_render: unknown option '{bad}'\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    let Some(character) = arg("--character") else {
        eprintln!("moveset_render: --character is required\n");
        print!("{USAGE}");
        std::process::exit(2);
    };
    let Some(verb_name) = arg("--verb") else {
        eprintln!("moveset_render: --verb is required\n");
        print!("{USAGE}");
        std::process::exit(2);
    };
    let Some(verb) = verb_named(&verb_name) else {
        // ⛔ NAME WHAT IS SUPPORTED. A capture-state move (a pummel, a throw)
        // needs a grabbed opponent, which this exercise cannot set up — so it is
        // absent rather than silently producing a mismatch.
        eprintln!(
            "moveset_render: '{verb_name}' is not a verb this exercise can perform.\n\
             known: {}\n",
            VERBS.iter().map(|v| v.verb).collect::<Vec<_>>().join(", ")
        );
        std::process::exit(2);
    };
    let out_dir =
        std::path::PathBuf::from(arg("--out").unwrap_or_else(|| "/tmp/moveset_render".to_string()));
    let frames: usize = arg("--frames")
        .and_then(|v| v.parse().ok())
        .unwrap_or(24)
        .max(1);
    let stride: u64 = arg("--stride")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    let size = UVec2::new(480, 360);

    std::fs::create_dir_all(&out_dir).expect("the output directory is creatable");
    for stale in std::fs::read_dir(&out_dir).into_iter().flatten().flatten() {
        if stale.path().extension().is_some_and(|e| e == "png") {
            let _ = std::fs::remove_file(stale.path());
        }
    }

    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        true,
        |_app| {},
    );
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(size.x as f32, size.y as f32),
        ),
    );
    app.insert_resource(ambition_platformer2d::render::capture::CaptureSettings {
        output: out_dir.join("frame.png"),
        size,
        include_ui: false,
    });
    app.init_resource::<ambition_platformer2d::render::capture::CaptureProgress>();
    app.add_systems(
        Startup,
        ambition_platformer2d::render::capture::setup_capture_target
            .after(ambition_app::app::PresentationSetupSet),
    );
    app.add_systems(
        Update,
        ambition_platformer2d::render::capture::adopt_cameras_into_capture_target,
    );

    // ⛔⛔ FINALIZE BEFORE STEPPING. Bevy builds the render device in plugin
    // `finish()`, which `App::run()` performs and a hand-driven loop never does;
    // without it this panics in `bevy_pbr`'s skin batching with
    // "Res<RenderDevice> failed validation".
    ambition_platformer2d::runtime::finalize(&mut app);
    let canonical = ambition_platformer2d::sim::enable_manual_stepping(&mut app);

    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            character.as_str(),
            character.as_str(),
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut live = false;
    for _ in 0..1200 {
        app.update();
        // ⛔⛔ THREE CONDITIONS. Seated is not running, and running is not
        // ACTING: the opening ceremony holds the cast with `ScriptedControl`,
        // and a press driven while it holds is discarded. The first version of
        // this driver waited only for a seat and a session, pressed into the
        // ceremony, and reported "no move ever became active" — which reads as a
        // broken press rather than a press nobody was listening to.
        let (staged, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&ambition_platformer2d::actor::MatchSeat>();
            let staged = all.iter(world).count();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actor::MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (staged, q.iter(world).count())
        };
        if staged > 0
            && held == 0
            && ambition_platformer2d::rollback::session_is_active(app.world())
        {
            live = true;
            break;
        }
    }
    if !live {
        eprintln!("moveset_render: no live rollback session for '{character}'");
        std::process::exit(1);
    }

    // ── SETTLE, THEN PREPARE ──
    //
    // ⛔⛔ THE SETTLE IS NOT OPTIONAL, AND LEAVING IT OUT MADE `prepared` LIE.
    // `session_is_active` is true while the cast is still DROPPING IN, so this
    // pressed at whatever moment the readiness loop happened to break —
    // `take_off` saw a falling body, called it airborne, returned true without
    // ever jumping, and the fighter landed before the press. The manifest said
    // `prepared: true` and every photograph showed a GROUNDED up-B, which is the
    // one posture this whole campaign exists to look past. The recorder settles
    // first and always did; only the renderer had half the algorithm.
    if move_exercise::subject(&mut app).is_none() {
        eprintln!("moveset_render: nobody reached seat zero for '{character}'");
        std::process::exit(1);
    }
    let quiet = move_exercise::settle(&mut app);
    let prepared = quiet && move_exercise::prepare(&mut app, verb);

    // ⭐⭐ WHAT THIS PRESS IS SUPPOSED TO PRODUCE, from the composed host's own
    // verb binding. Without it the only question a driver can ask is "did ANY
    // move play", which calls the known back-air-resolves-as-forward-air case a
    // success and files the forward air under `attack_air_back`.
    let intended = move_exercise::intended_move(&mut app, &character, verb.verb);

    // ── PERFORM, AND PHOTOGRAPH ON EXACT TICKS ──
    //
    // ⛔⛔ THE EXERCISE IS A TICK SCHEDULE; `--frames` AND `--stride` ONLY CHOOSE
    // WHAT IS OBSERVED. This held while `shot < frames / 4`, so the hold depended
    // on how many pictures were asked for: 24 frames at stride 2 held ~12 ticks,
    // a 12-frame run ~6, and the recorder's own exercise ~37. Asking for more
    // pictures charged the smash differently — the two tools were photographing
    // different moves and neither said so.
    let facing = move_exercise::facing_of(&mut app);
    let mut observed: std::collections::BTreeSet<String> = Default::default();
    let mut shots: Vec<serde_json::Value> = Vec::new();
    let mut pumps_total = 0usize;
    let mut action_tick = 0usize;

    for shot in 0..frames {
        move_exercise::step(
            &mut app,
            move_exercise::action_frame(verb, action_tick, facing),
        );
        if let Some(id) = move_exercise::playing_move(&mut app) {
            observed.insert(id);
        }
        let tick = sim_tick(&app);
        let at_action = action_tick;
        action_tick += 1;

        {
            let world = app.world_mut();
            let target = world
                .remove_resource::<ambition_platformer2d::render::capture::CaptureTarget>()
                .expect("the capture target exists");
            let mut progress = ambition_platformer2d::render::capture::CaptureProgress::default();
            world.insert_resource(ambition_platformer2d::render::capture::CaptureSettings {
                output: out_dir.join(format!("frame.{shot:04}.png")),
                size,
                include_ui: false,
            });
            let mut commands = world.commands();
            ambition_platformer2d::render::capture::request_capture(
                &mut commands,
                &target,
                &mut progress,
            );
            world.insert_resource(target);
            world.insert_resource(progress);
            world.flush();
        }

        // ⭐⭐ SERVICE THE GPU AT ZERO COST. Every pump runs the schedules and
        // moves no clock, so the picture belongs to `tick` and to no other.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
        let mut pumps = 0;
        let mut done = false;
        while pumps < 600 {
            app.update();
            pumps += 1;
            debug_assert_eq!(
                sim_tick(&app),
                tick,
                "a zero-duration pump advanced the sim"
            );
            if app
                .world()
                .get_resource::<ambition_platformer2d::render::capture::CaptureProgress>()
                .is_some_and(|p| p.completed)
            {
                done = true;
                break;
            }
        }
        pumps_total += pumps;
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(canonical));
        if !done {
            eprintln!("moveset_render: readback never completed for shot {shot}");
            std::process::exit(1);
        }
        shots.push(serde_json::json!({
            "file": format!("frame.{shot:04}.png"),
            // The absolute tick this picture belongs to...
            "sim_tick": tick,
            // ...and the tick of the EXERCISE, which is what a recorded take's
            // frame index also means. Two separate runs share no absolute
            // origin; they share this.
            "action_tick": at_action,
            "move": move_exercise::playing_move(&mut app),
            // ⭐ THE POSTURE OF THE TICK IN THE PICTURE. `prepared` says the
            // exercise ESTABLISHED a posture before the press; this says what the
            // body was doing when the shutter opened, which is the thing a
            // reader is actually looking at. Without it "is that an airborne
            // up-B?" is answered by squinting at a 480x360 PNG.
            "grounded": move_exercise::subject(&mut app).and_then(|s| s.grounded),
        }));

        // Advance the rest of the stride on the same schedule.
        for _ in 1..stride {
            move_exercise::step(
                &mut app,
                move_exercise::action_frame(verb, action_tick, facing),
            );
            action_tick += 1;
            if let Some(id) = move_exercise::playing_move(&mut app) {
                observed.insert(id);
            }
        }
    }

    // ⛔⛔ A SHORT REQUEST MUST NOT SILENTLY SHORTEN THE MOVE. `--frames 4
    // --stride 2` executes action ticks 0..8 and exits, so it never reaches the
    // release at tick 37 — and a manifest that printed `hold_ticks: 37` said what
    // the SCHEDULE is while proving nothing about what this run DID. So the
    // exercise is carried past the release WITHOUT taking pictures, and what was
    // seen after the last shot is reported separately from what was photographed.
    let observed_in_shots = observed.clone();
    let mut after_capture: std::collections::BTreeSet<String> = Default::default();
    while !move_exercise::released_by(action_tick) {
        move_exercise::step(
            &mut app,
            move_exercise::action_frame(verb, action_tick, facing),
        );
        action_tick += 1;
        if let Some(id) = move_exercise::playing_move(&mut app) {
            after_capture.insert(id.clone());
            observed.insert(id);
        }
    }
    let last_action_tick = action_tick.saturating_sub(1);

    // ⛔⛔ SUCCESS IS THE INTENDED MOVE APPEARING **IN THE POSTURE ASKED FOR**,
    // not any move appearing. This reported `reached = !observed.is_empty()`
    // under a comment claiming a mismatch would be caught — a promise the code
    // did not keep — and then, once that was fixed, still ignored `prepared`. A
    // grounded up-B and an airborne one can be the SAME move id, so the move
    // that came out cannot tell them apart, and the airborne one is the only one
    // anybody opens this view to look at.
    // ⛔⛔ AND THE POSTURE CLAIM IS CHECKED AGAINST THE PICTURES. `prepare` can
    // only report what was true BEFORE the press; a body that was airborne by a
    // hair lands on the next tick and the shutter opens on a grounded fighter.
    // The first shot is the press tick, so its ground state is the strongest
    // available answer to "is this the aerial the reader asked for" — and it is
    // the answer that caught `prepared: true` over a photograph of a standing
    // admiral.
    let airborne_at_press = shots
        .first()
        .and_then(|shot| shot["grounded"].as_bool())
        .map(|grounded| !grounded);
    let posture_held = !verb.airborne || airborne_at_press == Some(true);
    let verdict = move_exercise::outcome(prepared && posture_held, intended.as_deref(), &observed);
    let manifest = serde_json::json!({
        "character": character,
        "verb": verb.verb,
        "verb_label": verb.label,
        "prepared": prepared,
        "settled": quiet,
        // What the body was doing on the press tick, which is the posture the
        // pictures actually show.
        "airborne_at_press": airborne_at_press,
        "posture_held": posture_held,
        "intended_move": intended,
        "observed_moves": observed.iter().cloned().collect::<Vec<_>>(),
        "observed_in_shots": observed_in_shots.iter().cloned().collect::<Vec<_>>(),
        "observed_after_capture": after_capture.iter().cloned().collect::<Vec<_>>(),
        "reached_intended_move": verdict.reached(),
        "outcome": verdict.as_str(),
        "hold_ticks": move_exercise::HOLD_TICKS,
        // What this RUN did, as opposed to what the schedule says. A capture
        // horizon shorter than the release is legitimate; claiming the charge
        // paid out is not.
        "last_action_tick": last_action_tick,
        "release_reached": move_exercise::released_by(last_action_tick),
        "frames": shots.len(),
        "stride": stride,
        "shots": shots,
        "renderer": "moveset_render",
        "zero_time_pumps": pumps_total,
    });
    std::fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("the manifest serializes"),
    )
    .expect("the manifest is writable");

    println!(
        "[moveset-render] {character} {} -> {} frame(s) in {}, intended {:?}, observed {:?}, {} zero-time pump(s)",
        verb.verb,
        manifest["frames"],
        out_dir.display(),
        intended,
        observed,
        pumps_total,
    );
    if !verdict.reached() {
        println!(
            "[moveset-render] {}: `{}` is bound to {:?}, prepared={prepared}, and the engine played {:?}",
            verdict.as_str().to_uppercase(),
            verb.verb,
            intended,
            observed
        );
    }
}
