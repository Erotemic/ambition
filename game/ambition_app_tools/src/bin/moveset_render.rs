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

use ambition_sim_harness::combat_observation::{CombatObservation, ScenarioRoles};
use ambition_sim_harness::move_exercise;

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;
use ambition_sim_harness::{AdapterPreference, DeterministicCaptureSession};
use move_exercise::{verb_named, VERBS};

const USAGE: &str = "\
moveset_render — render a fighter performing one move, one PNG per simulation tick.

USAGE:
    moveset_render --character ID --verb VERB [--target ID] [--target-behavior WHICH]
                   [--spacing PX] [--out DIR] [--frames N] [--stride K]
                   [--combat-overlay on|off] [--adapter auto|hardware|software]

OPTIONS:
    --character ID   catalog id of the fighter
    --verb VERB      repertoire verb to perform (see below)
    --spacing PX     walk the subject to within PX of the target before the press
                     [default: the match's own seat placement]
    --target ID      who the move is performed against  [default: the fighter]
    --target-behavior WHICH
                     passive | cpu                      [default: passive]
    --combat-overlay WHICH
                     on | off                           [default: on]
                     `on` draws the engine's own combat geometry — hurtboxes,
                     live strike volumes, the move readout — over the real
                     rendered art, from the SAME execution. That is the whole
                     point of this binary: one picture, one simulation, no
                     second coordinate system.
    --out DIR        directory for the PNGs and manifest.json  [default: /tmp/moveset_render]
    --frames N       how many pictures                          [default: 24]
    --stride K       simulation ticks between pictures          [default: 1]
    --adapter WHICH  auto | hardware | software                  [default: auto]
                     ⭐ `software` pins Lavapipe, so a CI or agent job does not
                     change behaviour when a driver appears on the machine.
    -h, --help       print this and exit

NOTES:
    Needs a GPU: it boots the real OffscreenGpu composition and reads pixels back.

    Every PNG names the exact `SimTick` it was captured on, and the manifest
    records the intended move against what the engine actually played. A press
    is a request; if the move that came out is not the one asked for, that is
    reported rather than cached under the requested name.

    The manifest also carries the SEMANTIC geometry of every shot — hurtboxes,
    strike volumes, roles, the move clock — sampled before the shutter, so a
    reader knows exactly what the picture shows without measuring pixels.
";

/// Hold the combat overlay on for the whole run.
///
/// The three gates the gizmo pass reads are `ambition_dev_tools`'
/// business, not this binary's — see `force_combat_overlay` there.
fn force_combat_overlay(
    mut dev_state: Option<ResMut<ambition_platformer2d::dev_tools::DeveloperRuntimeState>>,
    mut developer: Option<ResMut<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
) {
    if let (Some(dev_state), Some(developer)) = (dev_state.as_mut(), developer.as_mut()) {
        ambition_platformer2d::dev_tools::force_combat_overlay(dev_state, developer);
    }
}

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
                "--character"
                    | "--verb"
                    | "--out"
                    | "--frames"
                    | "--stride"
                    | "--adapter"
                    | "--target"
                    | "--target-behavior"
                    | "--combat-overlay"
                    | "--spacing"
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
    // ⛔ A MIRROR MATCH IS THE DEFAULT AND IT IS A CHOICE, the same one the
    // recorder makes: the two tools must stage the same scenario or their
    // pictures describe different fights.
    let target = arg("--target").unwrap_or_else(|| character.clone());
    let passive_target = match arg("--target-behavior").as_deref() {
        None | Some("passive") => true,
        Some("cpu") => false,
        Some(word) => {
            eprintln!(
                "moveset_render: unknown --target-behavior '{word}'; expected passive or cpu"
            );
            std::process::exit(2);
        }
    };
    let spacing: Option<f32> = match arg("--spacing") {
        None => None,
        Some(word) => match word.parse::<f32>() {
            Ok(px) if px >= 0.0 => Some(px),
            _ => {
                eprintln!("moveset_render: --spacing wants a non-negative number of pixels");
                std::process::exit(2);
            }
        },
    };
    let combat_overlay = match arg("--combat-overlay").as_deref() {
        None | Some("on") => true,
        Some("off") => false,
        Some(word) => {
            eprintln!("moveset_render: unknown --combat-overlay '{word}'; expected on or off");
            std::process::exit(2);
        }
    };
    let size = UVec2::new(480, 360);

    // ⛔ BEFORE THE APP IS BUILT. Bevy reads the adapter environment when it
    // creates the device, in plugin `finish()`, so a preference applied later
    // silently does nothing.
    let adapter = match arg("--adapter").as_deref() {
        None => AdapterPreference::Auto,
        Some(word) => match AdapterPreference::parse(word) {
            Some(pref) => pref,
            None => {
                eprintln!(
                    "moveset_render: unknown --adapter '{word}'; expected auto, hardware or software"
                );
                std::process::exit(2);
            }
        },
    };
    adapter.apply();

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
    if combat_overlay {
        // ⭐⭐ THE ART AND THE GEOMETRY IN ONE IMAGE, FROM ONE EXECUTION. The
        // engine already draws `CombatGeometryView` over its own presentation —
        // that is the production developer overlay — so the picture a reader
        // gets here is the real renderer's, with the real runtime's volumes on
        // it. Nothing in this tool draws a box.
        //
        // ⛔ FORCED EVERY FRAME, not once at startup: the settings load and the
        // developer-tools default both write this state, so a startup-only
        // write is a race against whichever runs last.
        app.add_systems(Update, force_combat_overlay);
    }
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(size.x as f32, size.y as f32),
        ),
    );
    // ⭐ THE SESSION OWNS THE HARD PART; this binary owns the composition and the
    // ordering, because `PresentationSetupSet` is the product shell's and the
    // harness sits below it.
    DeterministicCaptureSession::install(&mut app, size, out_dir.join("frame.png"));
    app.add_systems(
        Startup,
        ambition_platformer2d::capture::setup_capture_target
            .after(ambition_app::app::PresentationSetupSet),
    );

    // ⛔⛔ FINALIZE BEFORE STEPPING. Bevy builds the render device in plugin
    // `finish()`, which `App::run()` performs and a hand-driven loop never does;
    // without it this panics in `bevy_pbr`'s skin batching with
    // "Res<RenderDevice> failed validation".
    ambition_platformer2d::runtime::finalize(&mut app);
    let canonical = ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    let camera = DeterministicCaptureSession::adopt(canonical, size);

    for _ in 0..30 {
        app.update();
    }
    // ⛔⛔ THE TARGET IS A SEAT WITH A STAND-STILL BRAIN, not a frozen body. A
    // live opponent walks into the strike being photographed, so the pictures
    // stop being about the move.
    let roster = if passive_target {
        ambition_demo_smash::smash_roster_with_passive_targets([
            character.as_str(),
            target.as_str(),
        ])
    } else {
        ambition_demo_smash::smash_roster([character.as_str(), target.as_str()])
    };
    app.world_mut().insert_resource(roster);
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
        // ⛔⛔ NAME THE CONDITION THAT FAILED, NOT THE LAST ONE IN THE `&&`.
        // This said *"no live rollback session"* whichever of the three was
        // false, and the common failure is the FIRST one: a character id that is
        // not on the smash grid seats nobody, so `staged` stays 0 and the message
        // sent the reader (and a 2026-08-29 review) looking for a missing GPU or
        // a broken session. ⇒ the loop above already distinguishes the three;
        // the report has to as well. The comment on that loop learned this exact
        // lesson one layer down and the message repeated it one layer up.
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
        let rollback_session = ambition_platformer2d::rollback::session_is_active(app.world());
        if staged == 0 {
            eprintln!(
                "moveset_render: '{character}' seated nobody — the match staged 0 \
                 fighters, so this is almost certainly a character id the smash \
                 grid does not carry rather than anything about rendering.\n\
                 \x20 seatable ids: run `moveset_export` and read `characters[].id` \
                 from its bundle."
            );
        } else if held > 0 {
            eprintln!(
                "moveset_render: '{character}' is seated ({staged}) but the opening \
                 ceremony still holds {held} of the cast under ScriptedControl \
                 after 1200 updates — a press driven now would be discarded."
            );
        } else if !rollback_session {
            eprintln!(
                "moveset_render: '{character}' is seated and free, but no rollback \
                 session became active in 1200 updates."
            );
        } else {
            eprintln!("moveset_render: '{character}' never became drivable");
        }
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
    // ⛔ SPACING BEFORE POSTURE, exactly as the recorder does it: walking closes
    // the gap on the ground, and an aerial verb takes off from where it arrived.
    // The two tools must stage ONE scenario or their pictures describe different
    // fights.
    let closed = spacing.map(|px| move_exercise::approach(&mut app, px));
    let prepared = quiet && move_exercise::prepare(&mut app, verb);

    // ⭐⭐ WHAT THIS PRESS IS SUPPOSED TO PRODUCE, from the composed host's own
    // verb binding. Without it the only question a driver can ask is "did ANY
    // move play", which calls the known back-air-resolves-as-forward-air case a
    // success and files the forward air under `attack_air_back`.
    // ⛔ AT THE PRESS, WHICH IS WHERE THE NAME SAYS. Read after the run it is the
    // gap at the END, and a connect LAUNCHES the target — so a press thrown at 33
    // px would be reported as 70.
    let spacing_at_press = move_exercise::gap_to_seat(&mut app, 1).map(f32::abs);
    let intended = move_exercise::intended_move(&mut app, &character, verb.verb);
    // ⛔⛔ AFTER STAGING. Roles are entity identities and the cast is spawned by
    // the route; asking before it is asking an empty stage who the subject is.
    let scenario = ScenarioRoles::from_seats(app.world_mut(), 0, 1);

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
        let tick = sim_tick(&app);
        let at_action = action_tick;
        action_tick += 1;

        // ⭐⭐ THE TICK'S SEMANTICS ARE READ **BEFORE THE SHUTTER**, and written
        // into the manifest afterwards from these saved values.
        //
        // ⛔⛔ THEY USED TO BE RE-QUERIED AFTER THE PUMP LOOP, which describes a
        // different moment. `SimTick` is frozen across a zero-duration pump —
        // that is the whole scheme — but frozen time is not a frozen WORLD: the
        // pump runs `Update` every iteration, and anything there that does not
        // gate on the fixed clock keeps running. So the manifest's `move` and
        // `grounded` described the world after N passes of the render service
        // loop while claiming to describe `sim_tick`. Whatever a later pump does
        // is not what the picture shows.
        let at_shutter = move_exercise::subject(&mut app);
        let shot_move = at_shutter.as_ref().and_then(|s| s.playing.clone());
        let shot_grounded = at_shutter.as_ref().and_then(|s| s.grounded);
        let shot_pose = at_shutter.as_ref().and_then(|s| s.pose);
        let shot_clip = at_shutter.and_then(|s| s.clip.clone());
        // ⭐⭐ AND THE GEOMETRY THE PICTURE SHOWS, IN NUMBERS. The overlay draws
        // it; this is the same tick's volumes, roles and move clock written
        // down, so a reader — or an agent that cannot see the PNG at all — knows
        // exactly what is in the frame instead of measuring pixels.
        //
        // ⛔ SAMPLED BEFORE THE SHUTTER, for the same reason `move` and
        // `grounded` are: the pump loop runs `Update` at zero duration, and
        // whatever a later pump does is not what the picture shows.
        let roles = scenario.resolve(app.world_mut());
        let observation = CombatObservation::capture(app.world_mut(), &roles).to_json();
        if let Some(id) = shot_move.clone() {
            observed.insert(id);
        }

        // ⭐⭐ ONE CALL, AND IT IS THE WHOLE SCHEME: arm the shot, service the GPU
        // with zero-duration pumps, refuse the frame if the fixed clock moved.
        // Extracted 2026-08-29 — it was the only reusable thing in this binary
        // and it was not reusable.
        let captured = match camera.capture(&mut app, out_dir.join(format!("frame.{shot:04}.png")))
        {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("moveset_render: shot {shot}: {error}");
                std::process::exit(1);
            }
        };
        debug_assert_eq!(captured.sim_tick, tick, "the session photographs the tick it was given");
        pumps_total += captured.pumps;
        shots.push(serde_json::json!({
            "file": format!("frame.{shot:04}.png"),
            // The absolute tick this picture belongs to...
            "sim_tick": tick,
            // ...and the tick of the EXERCISE, which is what a recorded take's
            // frame index also means. Two separate runs share no absolute
            // origin; they share this.
            "action_tick": at_action,
            // Saved at the shutter, not re-read after the pump — see above.
            "move": shot_move,
            // ⭐⭐ WHAT THE ENGINE MEANT TO DRAW ON THIS TICK, beside the picture
            // of it. Before 2026-08-29 no headless road could answer this for a
            // match fighter — `CharacterAnimator` needs a render app, and
            // `BodyPoseView` was gated on a marker a seat never receives — so
            // the inspector reconstructed the frame cursor from sprite sheets.
            // Now the PNG and the engine's own animation decision travel
            // together, and a mismatch between them is visible rather than
            // arguable.
            "pose": shot_pose.map(|p| format!("{p:?}")),
            "clip": shot_clip,
            // ⭐ THE POSTURE OF THE TICK IN THE PICTURE. `prepared` says the
            // exercise ESTABLISHED a posture before the press; this says what the
            // body was doing when the shutter opened, which is the thing a
            // reader is actually looking at. Without it "is that an airborne
            // up-B?" is answered by squinting at a 480x360 PNG.
            "grounded": shot_grounded,
            // The same tick's combat truth, in the one schema every recorder
            // writes: bodies with roles and hurtboxes, live strikes, move clock.
            "observation": observation,
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
    // ⭐ WHAT IT ACTUALLY RENDERED ON, not what it asked for. A preference steers
    // WGPU and does not command it, so the run reports the adapter it got — which
    // is the only answer a reader comparing two machines' pixels can use.
    let adapter_used = app
        .world()
        .get_resource::<bevy::render::renderer::RenderAdapterInfo>()
        .map(|info| format!("{} ({:?})", info.0.name, info.0.device_type))
        .unwrap_or_else(|| "unknown".to_string());
    let manifest = serde_json::json!({
        "character": character,
        "verb": verb.verb,
        "verb_label": verb.label,
        // ⭐ WHO WAS IN THE SCENARIO, in the recorder's vocabulary. The two
        // tools stage the same fight and now say so in the same words.
        "subject": character,
        "target": target,
        "target_behavior": if passive_target { "passive" } else { "cpu" },
        // ⛔ WHETHER THE PICTURES CARRY GEOMETRY. A reader looking at a PNG with
        // no boxes must be able to tell "this move has no hitbox" from "the
        // overlay was off".
        "combat_overlay": combat_overlay,
        "requested_spacing": spacing,
        // ⛔ ASKED FOR AND REACHED ARE TWO NUMBERS. A move that could not close
        // the gap is a finding, not a footnote.
        "spacing_closed": closed,
        "spacing_at_press": spacing_at_press,
        "observation_schema": ambition_sim_harness::OBSERVATION_SCHEMA,
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
        // ⭐ THE ADAPTER THAT DREW THESE PIXELS, beside the one that was asked
        // for. Two runs whose PNGs differ are not comparable unless both say the
        // same thing here, and `auto` means the machine decided.
        "adapter_requested": format!("{adapter:?}").to_lowercase(),
        "adapter_used": adapter_used,
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
