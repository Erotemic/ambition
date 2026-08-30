//! WHICH WAY DOES THE OFFICER'S ROUND ACTUALLY GO?
//!
//! `cargo run -p ambition_app_tools --bin officer_probe -- right`
//! `cargo run -p ambition_app_tools --bin officer_probe -- left`
//!
//! ⭐⭐ THIS EXISTS BECAUSE THE DRAW WAS FIXED AND STILL FIRES BACKWARDS. Jon,
//! 2026-08-30: *"the officer is still firing backwards, I thought we fixed that
//! bug yesterday."* `fb9230363` pinned WHEN the round leaves — frame 6, 0.348s,
//! where the muzzle flares — and added a test for it. Nothing anywhere states
//! which WAY it leaves, so the fix and the complaint are about different facts.
//!
//! ⛔ A MOVESET TEST WOULD NOT CATCH THIS, which is `wire_probe`'s first lesson
//! and the reason this is a probe. `MoveEventKind::Ranged` carries no direction:
//! the spec says "fire the owner's ranged weapon" and the direction is chosen
//! later, in `moveset/mod.rs`, from three sources in priority order:
//!
//!   1. `control.fire` — a live fire edge
//!   2. `playback.aim` — the aim captured when the move STARTED
//!   3. `kin.facing.signum()` — the body's facing
//!
//! A test over `officer_moveset()` proves the spec and never reaches any of
//! them. This drives the production input road on a real host and reads the
//! round that comes out.
//!
//! ⛔ IT REPORTS TWO SIGNS AND COMPARES THEM. The round's `vel.x` and the
//! officer's `facing` are each meaningless alone — the question is only ever
//! whether they agree. A probe that printed the velocity would leave the reader
//! doing the comparison that is the whole point.
//!
//! ⚠ AND IT RUNS BOTH WAYS. A direction bug that reverses the shot looks
//! identical to a correct shot if you only ever fire the way the character
//! happens to start facing.

#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use bevy::prelude::*;

/// Frame 6 of a 12-frame clip at 58ms is 0.348s; at 60Hz that is tick ~21.
/// Watch well past it so a round that arrives late is still seen.
const WATCH_TICKS: usize = 90;

/// ⛔⛔ THE OFFSET COLUMN WAS READING ONE TICK OF TRAVEL AS A MUZZLE OFFSET.
/// A round is only VISIBLE to this probe on the tick after it spawns, by which
/// point it has already moved `speed × dt` — for the Officer's 560 px/s that is
/// 560 ÷ 60 = **9.33 px**, which is exactly the "+9.3 (ahead)" this probe
/// reported and a handoff then cited as proof the spawn origin was correct.
/// It was not measuring the spawn origin at all: the shot was born at
/// `Muzzle::BodyOrigin`, a purely VERTICAL offset, i.e. horizontally ON him.
///
/// ⇒ back the travel out before judging where the round was BORN. An instrument
/// that cannot distinguish "left from the barrel" from "left from his sternum
/// and flew for one tick" cannot answer the question it exists to answer.
const SIM_DT: f32 = 1.0 / 60.0;

fn main() {
    let steer: f32 = match std::env::args().nth(1).as_deref() {
        Some("left") => -1.0,
        _ => 1.0,
    };
    let demo_host = std::env::args().any(|a| a == "host=demo");
    let rendered = std::env::args().any(|a| a == "render");

    let probe_stage::Staged {
        mut app,
        seat0,
        seat1,
    } = probe_stage::stage(probe_stage::StageRequest {
        cast: ["officer", "officer"],
        demo_host,
        rendered,
    });
    // ⛔⛔ GET THE SPARRING PARTNER OUT OF THE MUZZLE, or the probe cannot
    // measure the thing it exists to measure.
    //
    // Both seats are Officers standing close, and this probe HOLDS the stick to
    // keep the facing — so firing toward seat1 walks into him as well as
    // shooting at him. Once the muzzle moved to the hand (`Muzzle::Hand`) the
    // round was being born close enough to land on the SAME TICK it spawned:
    // 2026-08-30 the `right` run printed *"NO ROUND EVER SPAWNED"* while the
    // sim clock printed `impact_hitstop` — the shot fired, connected and
    // despawned between two samples, and `live_round` polls surviving
    // projectiles.
    //
    // ⚠ THAT IS NOT A BUG IN THE SHOT. A point-blank hit is a hit. It is a bug
    // in the INSTRUMENT: a probe whose answer depends on which way the other
    // fighter happens to be standing reports "no round" for a working move,
    // which is exactly the vacuous reading its own header warns about.
    // ⭐ PLACED BEHIND THE SHOOTER RATHER THAN FAR AWAY. A big absolute teleport
    // can leave the stage and be clamped or culled, which would trade this
    // problem for a less obvious one; standing him a fixed distance BEHIND the
    // muzzle keeps him on the stage and keeps the firing lane empty whichever
    // way the run steers.
    let (here, _) = probe_stage::kin(&app, seat0);
    let there = probe_stage::kin(&app, seat1).0;
    probe_stage::place(&mut app, seat1, Vec2::new(here.x - 220.0 * steer, there.y));

    println!(
        "[officer_probe] host = {}, steering {}",
        if demo_host {
            "ambition_demo_smash_app"
        } else {
            "ambition_app::build_visible_app"
        },
        if steer < 0.0 { "LEFT" } else { "RIGHT" }
    );

    // ⛔⛔ THE INSTRUMENT PROVES ITSELF FIRST. If no round is ever spawned, every
    // sign below is vacuous — and "no round" and "a round going the right way"
    // both print nothing alarming unless the probe says which happened.
    let settle = |app: &mut App, frame: ControlFrame, ticks: usize| {
        for _ in 0..ticks {
            ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
            app.update();
        }
    };

    settle(&mut app, ControlFrame::default(), 120);

    // Turn him first, on its own, so the facing under test is settled BEFORE the
    // special is pressed. Pressing a direction and the button on the same tick
    // conflates "which way is he facing" with "which way was the stick".
    settle(
        &mut app,
        ControlFrame {
            axis_x: steer,
            ..Default::default()
        },
        20,
    );
    let facing_at_press = facing(&app, seat0);

    // The side special: a held direction plus the special edge.
    ambition_platformer2d::sim::drive_control_frame(
        app.world_mut(),
        ControlFrame {
            axis_x: steer,
            special_pressed: true,
            special_held: true,
            ..Default::default()
        },
    );
    app.update();

    println!(
        "[officer_probe] pressed side-special while facing {:+.0} \
         (stick {:+.0})",
        facing_at_press, steer
    );
    // ⭐ THE OFFSET IS THE QUESTION THE VELOCITY CANNOT ANSWER. A round with the
    // right velocity that leaves from BEHIND him still reads as firing
    // backwards — that is what `fb9230363` ("the shot leaves where the flash
    // is") was about, and a probe that only compared signs would call this run
    // correct.
    println!(
        "  tick  facing   move                     officer.x   round.x   offset   vel.x"
    );

    let mut rounds_seen = 0usize;
    // (tick, facing, observed offset, BORN offset with the tick of travel backed
    // out, vel.x)
    let mut first_round: Option<(usize, f32, f32, f32, f32)> = None;
    for tick in 0..WATCH_TICKS {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_x: steer,
                special_held: true,
                ..Default::default()
            },
        );
        app.update();

        let f = facing(&app, seat0);
        let mv = probe_stage::playing_move(&app, seat0).unwrap_or_else(|| "-".into());
        let ox = probe_stage::kin(&app, seat0).0.x;
        if let Some((pos, vel)) = live_round(&mut app) {
            rounds_seen += 1;
            if first_round.is_none() {
                first_round = Some((tick, f, pos.x - ox, (pos.x - vel.x * SIM_DT) - ox, vel.x));
            }
            println!(
                "  {tick:4}  {f:+6.0}   {mv:<22}  {ox:>9.1}  {:>8.1}  {:>7.1}  {:>6.1}",
                pos.x,
                pos.x - ox,
                vel.x
            );
        } else if tick % 10 == 0 {
            println!("  {tick:4}  {f:+6.0}   {mv:<22}  {ox:>9.1}         -        -       -");
        }
    }

    println!();
    match first_round {
        None => {
            println!(
                "⛔ NO ROUND EVER SPAWNED in {WATCH_TICKS} ticks. This run says NOTHING \
                 about direction — the draw did not fire at all, which is a different \
                 bug and has to be settled first."
            );
        }
        Some((tick, f, offset, born, vx)) => {
            let agree = (vx > 0.0 && f > 0.0) || (vx < 0.0 && f < 0.0);
            // ⛔ THE VERDICT IS ON WHERE IT WAS BORN, NOT WHERE IT WAS FIRST
            // SEEN. See `SIM_DT`: the seen offset always leads the born one by a
            // full tick of travel, which is enough to make a shot that starts on
            // his sternum look like it started ahead of him.
            let muzzle_ahead = (born > 0.0 && f > 0.0) || (born < 0.0 && f < 0.0);
            println!(
                "first round at tick {tick}: BORN at {born:+.1} from the officer \
                 (first seen at {offset:+.1}, one tick of travel later), \
                 vel.x {vx:+.1}, facing {f:+.0} ({rounds_seen} round-ticks observed)"
            );
            if born.abs() < 1.0 {
                println!(
                    "⚠ THE ROUND IS BORN ON HIM ({born:+.1}). Its velocity may still be \
                     right, but it leaves from his body rather than from the barrel the \
                     `shoot` clip draws in his hand — which is what reads as firing \
                     backwards. That is `Muzzle::BodyOrigin`; a drawn weapon wants \
                     `Muzzle::Hand`."
                );
            }
            if !muzzle_ahead {
                println!(
                    "⛔ THE ROUND LEAVES FROM BEHIND HIM. it is born at {born:+.1}, on the \
                     opposite side from facing {f:+.0} — the muzzle is drawn on one side \
                     and the shot spawns on the other, which reads as firing backwards \
                     however the velocity is signed."
                );
            } else {
                println!("  muzzle offset {born:+.1} is AHEAD of him, as it should be.");
            }
            if vx == 0.0 {
                println!("⛔ the round has NO horizontal velocity — it is not travelling at all.");
            } else if agree {
                println!(
                    "✓ THE ROUND TRAVELS THE WAY HE FACES. vel.x {vx:+.1} agrees with \
                     facing {f:+.0}."
                );
            } else {
                println!(
                    "⛔ THE ROUND TRAVELS BACKWARDS. vel.x {vx:+.1} against facing {f:+.0} \
                     — he shoots behind himself."
                );
            }
            println!(
                "\n⚠ Run the other direction too (`left` / `right`). A sign error that \
                 reverses the shot is invisible in whichever direction the character \
                 happens to start facing."
            );
        }
    }
}

fn facing(app: &App, body: Entity) -> f32 {
    app.world()
        .get::<BodyKinematics>(body)
        .map(|k| k.facing.signum())
        .unwrap_or(0.0)
}

/// The first live round in flight, as (pos, vel).
fn live_round(app: &mut App) -> Option<(Vec2, Vec2)> {
    let mut q = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<ambition_platformer2d::projectiles::entity::LiveProjectile>>();
    q.iter(app.world())
        .next()
        .map(|k| (Vec2::new(k.pos.x, k.pos.y), Vec2::new(k.vel.x, k.vel.y)))
}
