//! Record what the REAL simulation does when a fighter throws each of its moves.
//!
//! ⭐⭐ THIS IS THE HALF OF THE INSPECTOR THAT CANNOT BE FAKED. Jon, 2026-08-27:
//! *"This should let us 'prove' that up-b works because to build this we run the
//! characters in the real engine and use control frames to show how the game
//! reacts to their inputs and we will see things like the pirate flying around
//! on the shark."* A frame-data table reports what a move DECLARES; a take
//! reports what the engine DID with it — where the body went, which hitboxes
//! were live, what the move spawned, and whether the fighter ended up riding it.
//!
//! ⛔ ONE APP, ONE PROCESS, for the reason `shark_ride_probe` writes down: the
//! tracing subscriber is process-global, so a tool that builds several Apps
//! cannot keep the log. This builds one and seats every take in it.
//!
//! ⛔ A MOVE THAT DOES NOT COME OUT IS STILL RECORDED. A take whose `move` field
//! stays empty is the honest report that the press did not reach the move — a
//! posture gate, a spent recovery, a shield. Dropping those would make the
//! inspector show only the moves that already work.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use ambition_platformer2d::engine_core::ControlFrame;

/// Ticks recorded per take. Long enough for a five-second shark ride to show its
/// shape without every take carrying the tail of an idle stage.
const TAKE_TICKS: usize = 150;
/// The longest a take will wait for the stage to go quiet before the next press.
///
/// ⛔⛔ A FIXED SETTLE IS NOT A SETTLE. Forty-five ticks was less than the
/// admiral's forward smash owes, so `smash_up`, `smash_down` and `special_up`
/// each landed inside the previous move's recovery, were dropped, and were
/// reported as moves that produced nothing — three false findings from one
/// constant. The condition is "the body is idle and standing", which the world
/// already publishes, so the wait ASKS instead of counting.
/// ⛔ ABOVE THE LONGEST RIDE. A 240-tick limit is four seconds and the shark
/// carries its rider for five, so the take after the up-B started while the
/// admiral was still airborne on a mount and reported two moves as producing
/// nothing. A settle that gives up before the previous take finishes is a
/// settle that manufactures findings.
const SETTLE_LIMIT: usize = 480;

/// One press, as the genre spells it.
struct Verb {
    /// The repertoire verb this drives, which is the key the UI files it under.
    verb: &'static str,
    label: &'static str,
    axis_x: f32,
    axis_y: f32,
    button: Button,
    /// Jump first, and wait for the apex. An aerial pressed from the ground
    /// reaches the grounded chain instead, and reports the wrong move.
    airborne: bool,
    /// The direction to hold while climbing, so the press has a facing to be
    /// relative TO. `attack_air_back` is the only verb that needs one: BACK is
    /// defined against the body's facing, and a body that took off facing
    /// nowhere in particular answers a back-air press with the forward one.
    climb_x: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Attack,
    Smash,
    Special,
    Grab,
    Taunt,
}

/// The full press table. Directions follow the engine's own convention:
/// `axis_y = -1` is UP (the same value `shark_ride_probe` drives for the up-B).
const VERBS: &[Verb] = &[
    Verb { verb: "attack", label: "Jab", axis_x: 0.0, axis_y: 0.0, button: Button::Attack, airborne: false, climb_x: 0.0 },
    Verb { verb: "attack_forward", label: "F-tilt", axis_x: 1.0, axis_y: 0.0, button: Button::Attack, airborne: false, climb_x: 0.0 },
    Verb { verb: "attack_up", label: "U-tilt", axis_x: 0.0, axis_y: -1.0, button: Button::Attack, airborne: false, climb_x: 0.0 },
    Verb { verb: "attack_down", label: "D-tilt", axis_x: 0.0, axis_y: 1.0, button: Button::Attack, airborne: false, climb_x: 0.0 },
    Verb { verb: "smash_forward", label: "F-smash", axis_x: 1.0, axis_y: 0.0, button: Button::Smash, airborne: false, climb_x: 0.0 },
    Verb { verb: "smash_up", label: "U-smash", axis_x: 0.0, axis_y: -1.0, button: Button::Smash, airborne: false, climb_x: 0.0 },
    Verb { verb: "smash_down", label: "D-smash", axis_x: 0.0, axis_y: 1.0, button: Button::Smash, airborne: false, climb_x: 0.0 },
    Verb { verb: "attack_air", label: "N-air", axis_x: 0.0, axis_y: 0.0, button: Button::Attack, airborne: true, climb_x: 0.0 },
    Verb { verb: "attack_air_forward", label: "F-air", axis_x: 1.0, axis_y: 0.0, button: Button::Attack, airborne: true, climb_x: 0.0 },
    Verb { verb: "attack_air_back", label: "B-air", axis_x: -1.0, axis_y: 0.0, button: Button::Attack, airborne: true, climb_x: 1.0 },
    Verb { verb: "attack_air_up", label: "U-air", axis_x: 0.0, axis_y: -1.0, button: Button::Attack, airborne: true, climb_x: 0.0 },
    Verb { verb: "attack_air_down", label: "D-air", axis_x: 0.0, axis_y: 1.0, button: Button::Attack, airborne: true, climb_x: 0.0 },
    Verb { verb: "special", label: "Neutral B", axis_x: 0.0, axis_y: 0.0, button: Button::Special, airborne: false, climb_x: 0.0 },
    Verb { verb: "special_forward", label: "Side B", axis_x: 1.0, axis_y: 0.0, button: Button::Special, airborne: false, climb_x: 0.0 },
    // ⭐ THE UP-B IS RECORDED FROM THE AIR, which is the only place it is the
    // move Jon is asking about. A grounded up-B answers the same press and shows
    // none of the recovery.
    Verb { verb: "special_up", label: "Up B (airborne)", axis_x: 0.0, axis_y: -1.0, button: Button::Special, airborne: true, climb_x: 0.0 },
    Verb { verb: "special_down", label: "Down B", axis_x: 0.0, axis_y: 1.0, button: Button::Special, airborne: false, climb_x: 0.0 },
    Verb { verb: "special_air_down", label: "Down B (air)", axis_x: 0.0, axis_y: 1.0, button: Button::Special, airborne: true, climb_x: 0.0 },
    Verb { verb: "grab", label: "Grab", axis_x: 0.0, axis_y: 0.0, button: Button::Grab, airborne: false, climb_x: 0.0 },
    Verb { verb: "taunt", label: "Taunt", axis_x: 0.0, axis_y: 0.0, button: Button::Taunt, airborne: false, climb_x: 0.0 },
];

/// How far the stick goes for a TILT.
///
/// ⛔⛔ THIS IS THE DIFFERENCE BETWEEN A TILT AND A SMASH, and driving `1.0`
/// silently recorded the smash for every directional tilt — four takes that
/// looked like working data. `resolve_attack_gesture` arms a flick at
/// `flick_threshold` 0.8 and calls a press that matches a recent flick a SMASH;
/// a magnitude above `directional_deadzone` 0.5 and below 0.8 is directional and
/// never arms one, which is exactly what a tilt input is.
const TILT_AXIS: f32 = 0.65;

/// The press, aimed relative to the body's CURRENT facing.
///
/// ⛔⛔ `axis_x` IS "FORWARD", NOT "RIGHT", and driving it as a world direction
/// recorded the forward air for every back-air take. `attack_dir_from_axis`
/// resolves a press against `BodyKinematics::facing`, so a fighter that happened
/// to be pointing left answered a left press with FORWARD — correctly, and the
/// take reported it as a back-air. The facing is a fact the world publishes;
/// this reads it rather than assuming the body starts pointing right.
fn press(v: &Verb, edge: bool, facing: f32) -> ControlFrame {
    let reach = if v.button == Button::Smash { 1.0 } else { TILT_AXIS };
    let mut frame = ControlFrame {
        axis_x: v.axis_x * reach * facing.signum(),
        axis_y: v.axis_y * reach,
        ..Default::default()
    };
    match v.button {
        Button::Attack => {
            frame.attack_pressed = edge;
            frame.attack_held = true;
        }
        Button::Smash => {
            frame.attack_pressed = edge;
            frame.attack_held = true;
            // The gesture that tells a tilt from a smash. Without it every
            // "smash" take records the tilt, which looks like working data.
            frame.attack_strong_hint = true;
        }
        Button::Special => {
            frame.special_pressed = edge;
            frame.special_held = true;
        }
        Button::Grab => frame.grab_pressed = edge,
        Button::Taunt => frame.taunt_pressed = edge,
    }
    frame
}

/// Everything one recorded tick says.
#[derive(Default)]
struct Frame {
    bodies: Vec<serde_json::Value>,
    hitboxes: Vec<serde_json::Value>,
    projectiles: Vec<serde_json::Value>,
    move_id: Option<String>,
    grounded: Option<bool>,
    subject_pos: Option<(f32, f32)>,
    subject_vel: Option<(f32, f32)>,
    riding: Option<String>,
    /// Which way the body is pointing. A directional press is resolved against
    /// this, so a take that came out forward when back was driven is only
    /// readable with it on the recording.
    facing: Option<f32>,
    /// The gesture the engine resolved from the press, e.g. `Back/Tilt/Airborne`.
    gesture: Option<String>,
}

/// Read the world once. Everything here is a read; nothing is mutated, so a
/// take can never be the reason a run diverges.
fn sample(world: &mut World, subject_seat: usize) -> Frame {
    let mut frame = Frame::default();

    let mut bodies = world.query::<(
        Entity,
        &ambition_platformer2d::engine_core::BodyKinematics,
        Option<&ambition_platformer2d::actor::MatchSeat>,
        Option<&ambition_platformer2d::character::WornCharacter>,
        Option<&ambition_platformer2d::combat::moveset::MovePlayback>,
        Option<&ambition_platformer2d::mount::RidingOn>,
        Option<&ambition_platformer2d::mount::MountSlot>,
        Option<&ambition_platformer2d::engine_core::BodyGroundState>,
        // ⭐ WHAT THE ENGINE UNDERSTOOD THE PRESS TO BE. The recording already
        // shows which move came out; this shows why. A take that drove BACK and
        // played the forward air is unreadable without it — the direction is
        // resolved against facing, a turnaround flips that facing, and none of
        // it is visible from the move id alone.
        Option<&ambition_platformer2d::characters::actor::attack_gesture::ResolvedAttackGesture>,
    )>();
    let rows: Vec<_> = bodies
        .iter(world)
        .map(|(e, kin, seat, worn, play, riding, slot, ground, gesture)| {
            (
                e,
                (kin.pos.x, kin.pos.y),
                (kin.vel.x, kin.vel.y),
                (kin.size.x * 0.5, kin.size.y * 0.5),
                kin.facing,
                seat.map(|s| s.0),
                worn.map(|w| w.id().to_string()),
                play.map(|p| p.spec.id.clone()),
                riding.map(|r| r.mount),
                slot.is_some(),
                ground.map(|g| g.on_ground),
                gesture.and_then(|g| g.pressed).map(|i| {
                    format!("{:?}/{:?}/{:?}", i.direction, i.strength, i.posture)
                }),
            )
        })
        .collect();

    let mut owner_pos = std::collections::HashMap::new();
    for (entity, pos, ..) in &rows {
        owner_pos.insert(*entity, *pos);
    }

    for (entity, pos, vel, half, facing, seat, worn, playing, riding, is_mount, on_ground, gesture) in
        &rows
    {
        let subject = *seat == Some(subject_seat);
        if subject {
            frame.subject_pos = Some(*pos);
            frame.subject_vel = Some(*vel);
            frame.move_id = playing.clone();
            frame.grounded = *on_ground;
            frame.facing = Some(*facing);
            frame.gesture = gesture.clone();
            // ⛔⛔ THE MOUNT'S LABEL, NOT ITS WORN CHARACTER. Reading the ride
            // through `WornCharacter` reported `riding: null` for a real,
            // boarded shark — a summoned mount wears no catalog character, so
            // the `and_then` erased the very fact this take exists to show. The
            // ride is `RidingOn` existing; the label is a nicety.
            frame.riding = riding.map(|mount| {
                rows.iter()
                    .find(|(e, ..)| *e == mount)
                    .and_then(|(.., worn, _, _, _, _, _)| worn.clone())
                    .unwrap_or_else(|| format!("{mount}"))
            });
        }
        frame.bodies.push(serde_json::json!({
            "pos": [pos.0, pos.1],
            "half": [half.0, half.1],
            "seat": seat,
            "label": worn.clone().unwrap_or_else(|| format!("{entity}")),
            // A summoned mount is neither a seat nor scenery, and a viewer that
            // could not tell it apart would draw the shark as another fighter.
            "kind": if *is_mount { "summon" } else if seat.is_some() { "fighter" } else { "body" },
            "move": playing.clone(),
        }));
    }

    // ⭐ A RANGED MOVE'S DAMAGE IS ITS PROJECTILE, and a take that recorded only
    // hitboxes showed the pirate's new side-B as a move that fires nothing.
    // Projectiles are excluded from every actor-generic query by construction
    // (`ProjectileGameplay` is the marker that keeps them out), so they have to
    // be asked for by name.
    let mut shots = world.query::<(
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::platformer::projectile::ProjectileGameplay,
    )>();
    let flying: Vec<_> = shots
        .iter(world)
        .map(|(kin, shot)| (kin.pos, kin.vel, kin.size, shot.damage))
        .collect();
    for (pos, vel, size, damage) in flying {
        frame.projectiles.push(serde_json::json!({
            "pos": [pos.x, pos.y],
            "vel": [vel.x, vel.y],
            "half": [size.x * 0.5, size.y * 0.5],
            "damage": damage,
        }));
    }

    let mut hitboxes = world.query::<&ambition_platformer2d::combat::strike::Hitbox>();
    let boxes: Vec<_> = hitboxes.iter(world).cloned().collect();
    for hitbox in boxes {
        let anchor = owner_pos
            .get(&hitbox.owner)
            .copied()
            .unwrap_or((0.0, 0.0));
        // The SAME resolution the combat runtime uses, so a recorded box is the
        // box that could hit somebody rather than a redrawn approximation.
        let aabb =
            hitbox.world_aabb(ambition_platformer2d::engine_core::Vec2::new(anchor.0, anchor.1));
        frame.hitboxes.push(serde_json::json!({
            "pos": [(aabb.min.x + aabb.max.x) * 0.5, (aabb.min.y + aabb.max.y) * 0.5],
            "half": [(aabb.max.x - aabb.min.x) * 0.5, (aabb.max.y - aabb.min.y) * 0.5],
            "damage": hitbox.damage,
        }));
    }

    frame
}

fn platforms(app: &mut App) -> Vec<serde_json::Value> {
    app.world_mut()
        .run_system_once(
            |world: ambition_platformer2d::world::collision::CollisionWorld| -> Vec<serde_json::Value> {
                let Some(solids) = world.solids() else {
                    return Vec::new();
                };
                solids
                    .blocks
                    .iter()
                    .map(|b| {
                        serde_json::json!([
                            (b.aabb.min.x + b.aabb.max.x) * 0.5,
                            (b.aabb.min.y + b.aabb.max.y) * 0.5,
                            (b.aabb.max.x - b.aabb.min.x) * 0.5,
                            (b.aabb.max.y - b.aabb.min.y) * 0.5,
                        ])
                    })
                    .collect()
            },
        )
        .unwrap_or_default()
}

/// Put a clean match on the stage.
///
/// ⛔⛔ A TAKE THAT STARTS FROM A CORPSE MEASURES NOTHING. Two takes reported
/// their move as producing nothing because the previous one had knocked the
/// admiral off the stage: the recording showed a body frozen below the floor
/// with `grounded: false` forever, and the press went to somebody who was not
/// there. The settle can detect that state; only a re-seat can fix it.
fn reseat(app: &mut App, character: &str) {
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([character, character]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..240 {
        app.update();
    }
}

/// `verb -> move id` for one character, read from the composed host.
fn verb_table(app: &mut App, character: &str) -> std::collections::BTreeMap<String, String> {
    app.world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
        .and_then(|registry| registry.get(character))
        .and_then(|prepared| prepared.kit.projectable_moveset())
        .map(|set| set.verbs.clone())
        .unwrap_or_default()
}

/// The move a verb is bound to, or `None` when the fighter binds nothing there
/// (an unbound slot is answered by the directional chain, and whatever it
/// reaches is the right answer rather than a mismatch).
fn intended_move<'a>(
    bound: &'a std::collections::BTreeMap<String, String>,
    verb: &str,
) -> Option<&'a str> {
    bound.get(verb).map(String::as_str)
}

fn drive(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
    app.update();
}

/// Jump, and wait until the world says the body has left the ground.
///
/// ⛔ ASK, DO NOT COUNT. A fixed wait recorded `attack_air_down` as `smash_down`
/// and `attack_air_up` as nothing at all, because the press landed on a body the
/// engine still called grounded and the directional chain walked past every
/// aerial. The ground state is a fact the world publishes.
fn ensure_airborne(app: &mut App) -> bool {
    if sample(app.world_mut(), 0).grounded == Some(false) {
        return true;
    }
    drive(
        app,
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
    for _ in 0..40 {
        drive(
            app,
            ControlFrame {
                jump_held: true,
                ..Default::default()
            },
        );
        if sample(app.world_mut(), 0).grounded == Some(false) {
            return true;
        }
    }
    false
}

/// Wait for a body that is standing, idle and not riding anything.
fn settle(app: &mut App) -> bool {
    for _ in 0..SETTLE_LIMIT {
        drive(app, ControlFrame::default());
        let now = sample(app.world_mut(), 0);
        if now.move_id.is_none() && now.grounded == Some(true) && now.riding.is_none() {
            return true;
        }
    }
    false
}

/// Sample the world and append it to a take.
fn record(app: &mut App, frames: &mut Vec<serde_json::Value>) {
    let frame = sample(app.world_mut(), 0);
    frames.push(serde_json::json!({
        "bodies": frame.bodies,
        "hitboxes": frame.hitboxes,
        "projectiles": frame.projectiles,
        "move": frame.move_id,
        "grounded": frame.grounded,
        "subject_pos": frame.subject_pos.map(|p| vec![p.0, p.1]),
        "subject_vel": frame.subject_vel.map(|v| vec![v.0, v.1]),
        "facing": frame.facing,
        "gesture": frame.gesture,
        "riding": frame.riding,
    }));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.windows(2)
            .find(|w| w[0] == name)
            .map(|w| w[1].clone())
    };
    let out = arg("--out")
        .unwrap_or_else(|| "tools/ambition_moveset_inspector/data/takes/takes.json".to_string());
    let who: Vec<String> = arg("--characters")
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_else(|| vec!["npc_pirate_admiral".to_string()]);

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.add_plugins(bevy::log::LogPlugin::default());
    for _ in 0..30 {
        app.update();
    }

    let mut takes = Vec::new();

    for character in &who {
        // A partner, so contact rules and targeting behave as they do in a
        // match. A solo stage is a different simulation from the one the
        // inspector claims to be showing.
        reseat(&mut app, character);
        let stage = platforms(&mut app);
        // The fighter's own verb table, so a take can say which move the press
        // was SUPPOSED to reach rather than only which one came out.
        let bound = verb_table(&mut app, character);

        for verb in VERBS {
            let mut settled = settle(&mut app);
            if !settled {
                // The previous take left the stage in a state this one cannot
                // start from — a body off the stage, a ride still running, a
                // fighter mid-respawn. A fresh match is the only reset.
                reseat(&mut app, character);
                settled = settle(&mut app);
            }
            if !settled {
                println!(
                    "[take] {character:<24} {:<16} WARNING - the stage would not go quiet \
                     even after a re-seat; read this take with that in mind",
                    verb.verb
                );
            }
            if verb.airborne {
                // ⛔⛔ AIRBORNE AT THE PRESS, CHECKED AT THE PRESS. Jumping and
                // then doing anything else is not the same claim: an aim settle,
                // a fast-fall, or simply a short hop meant the body was standing
                // again by the time the button went down, and the take recorded
                // the grounded move under the aerial's name. So the takeoff and
                // the aim settle are one loop that ends only when the body is
                // both airborne and pointing the right way.
                let mut ready = false;
                for _attempt in 0..3 {
                    if !ensure_airborne(&mut app) {
                        continue;
                    }
                    // Settle a HORIZONTAL aim only: a back-air driven the tick
                    // the stick reversed resolves as FORWARD, because
                    // `resolve_attack_gestures` reads `-facing` while a
                    // turnaround runs (the pivot rule, which is correct and is
                    // why a pivot grab needs no move of its own). Holding DOWN
                    // for the same settle would fast-fall back to the floor.
                    if verb.axis_x != 0.0 {
                        for _ in 0..8 {
                            let aim = sample(app.world_mut(), 0).facing.unwrap_or(1.0);
                            drive(
                                &mut app,
                                ControlFrame {
                                    axis_x: verb.axis_x * TILT_AXIS * aim.signum(),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    if sample(app.world_mut(), 0).grounded == Some(false) {
                        ready = true;
                        break;
                    }
                }
                if !ready {
                    println!(
                        "[take] {character:<24} {:<16} WARNING - could not be airborne at the \
                         press; this take records the GROUNDED answer to that button",
                        verb.verb
                    );
                }
            }
            let mut frames: Vec<serde_json::Value> = Vec::new();
            let facing = sample(app.world_mut(), 0).facing.unwrap_or(1.0);
            drive(&mut app, press(verb, true, facing));
            // ⛔ THE PRESS TICK IS FRAME ZERO. `ResolvedAttackGesture::pressed`
            // is set on the press tick and cleared after, so a recording that
            // started one tick later showed `gesture: null` on every frame of
            // every take — the one field that says what the engine understood
            // the input to be, absent from all of them.
            record(&mut app, &mut frames);
            for tick in 1..TAKE_TICKS {
                // A charge move releases when the button comes up. Half the take
                // held, half released, so both the hold and the payoff are on
                // the recording.
                let held = tick < TAKE_TICKS / 4;
                drive(
                    &mut app,
                    if held {
                        press(verb, false, facing)
                    } else {
                        ControlFrame::default()
                    },
                );
                record(&mut app, &mut frames);
            }

            let moves: std::collections::BTreeSet<String> = frames
                .iter()
                .filter_map(|f| f["move"].as_str().map(str::to_string))
                .collect();
            let rode = frames.iter().any(|f| !f["riding"].is_null());
            let live = frames
                .iter()
                .map(|f| f["hitboxes"].as_array().map_or(0, Vec::len))
                .max()
                .unwrap_or(0);
            let shots = frames
                .iter()
                .map(|f| f["projectiles"].as_array().map_or(0, Vec::len))
                .max()
                .unwrap_or(0);

            // The view: the stage plus everything this take reached, padded.
            // Computed per take rather than per frame, so scrubbing does not
            // make a rising fighter look stationary while the world slides.
            let (mut x0, mut y0, mut x1, mut y1) =
                (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for block in &stage {
                let v: Vec<f32> = block
                    .as_array()
                    .map(|a| a.iter().filter_map(|n| n.as_f64().map(|f| f as f32)).collect())
                    .unwrap_or_default();
                if v.len() == 4 {
                    x0 = x0.min(v[0] - v[2]);
                    y0 = y0.min(v[1] - v[3]);
                    x1 = x1.max(v[0] + v[2]);
                    y1 = y1.max(v[1] + v[3]);
                }
            }
            for f in &frames {
                for b in f["bodies"].as_array().into_iter().flatten() {
                    let (Some(px), Some(py)) = (b["pos"][0].as_f64(), b["pos"][1].as_f64()) else {
                        continue;
                    };
                    x0 = x0.min(px as f32 - 40.0);
                    y0 = y0.min(py as f32 - 40.0);
                    x1 = x1.max(px as f32 + 40.0);
                    y1 = y1.max(py as f32 + 40.0);
                }
            }
            if !x0.is_finite() {
                (x0, y0, x1, y1) = (-320.0, -240.0, 320.0, 240.0);
            }

            // ⛔⛔ THE TAKE SAYS WHETHER IT REACHED THE MOVE IT DROVE. Eighteen
            // of nineteen verbs do; `attack_air_back` does not, and that is an
            // ENGINE finding rather than a driver one — the recorded gesture
            // reads `Forward/Tilt/Airborne`, so the fighter turned to face the
            // back input before the press was read, and the back air is
            // unreachable for every fighter in the cast. Reporting it as "the
            // forward air, under the back air's name" is what a tool that
            // silently relabels its own failures does.
            let intended = intended_move(&bound, verb.verb);
            let reached = intended.is_none_or(|id| moves.contains(id));
            println!(
                "[take] {character:<24} {:<16} moves={:?} hitboxes<={live} shots<={shots} rode={rode}{}",
                verb.verb,
                moves,
                if reached {
                    String::new()
                } else {
                    format!(
                        " MISMATCH: drove {} but the engine played {:?}",
                        intended.unwrap_or("?"),
                        moves
                    )
                }
            );
            takes.push(serde_json::json!({
                "character": character,
                "verb": verb.verb,
                "label": verb.label,
                "seat": 0,
                "view": [x0, y0, x1, y1],
                "platforms": stage,
                // What the ENGINE did, which is the whole claim this file makes.
                // A take that reached no move says so here rather than looking
                // like a move with nothing in it.
                "moves_seen": moves.iter().cloned().collect::<Vec<_>>(),
                "rode_a_mount": rode,
                "max_live_hitboxes": live,
                "max_live_projectiles": shots,
                "intended_move": intended,
                "reached_intended_move": reached,
                "frames": frames,
            }));
        }
    }

    let bundle = serde_json::json!({
        "schema": "ambition.moveset_takes.v1",
        "sim_hz": 60.0,
        "takes": takes,
    });
    let path = std::path::Path::new(&out);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("the output directory is creatable");
    }
    std::fs::write(path, serde_json::to_string(&bundle).expect("the takes serialize"))
        .expect("the takes are writable");
    println!(
        "[moveset-takes] {} take(s) -> {out}",
        bundle["takes"].as_array().map_or(0, Vec::len)
    );
}
