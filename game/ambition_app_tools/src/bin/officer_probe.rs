//! Which way does the officer's round go?
//!
//! `cargo run -p ambition_app_tools --bin officer_probe -- right`
//! `cargo run -p ambition_app_tools --bin officer_probe -- left`
//!
//! The existing test pins when the round leaves (frame 6, 0.348s, where the
//! muzzle flares). Nothing else states which way it leaves.
//!
//! A moveset test cannot catch this. `MoveEventKind::Ranged` carries no
//! direction; `moveset/mod.rs` chooses it later from three sources, in order:
//!
//!   1. `control.fire`: a live fire edge
//!   2. `playback.aim`: the aim captured when the move started
//!   3. `kin.facing.signum()`: the body's facing
//!
//! This drives the production input path on a real host and reads the round
//! that comes out. It reports the round's `vel.x` and the officer's `facing`
//! and says whether they agree. It runs both ways, because a reversed shot
//! looks correct if you only fire the way the character starts facing.

#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use bevy::prelude::*;

/// Frame 6 of a 12-frame clip at 58ms is 0.348s; at 60Hz that is tick ~21.
/// Watch well past it so a round that arrives late is still seen.
const WATCH_TICKS: usize = 90;

/// A round is visible to this probe only on the tick after it spawns, when it
/// has already moved `speed × dt` (560 ÷ 60 = 9.33 px for the Officer). Back
/// that travel out before judging where the round was born.
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
    // Move the sparring partner out of the firing lane. Both seats are
    // Officers standing close, and this probe holds the stick, so firing
    // toward seat 1 walks into him. With the muzzle at the hand
    // (`Muzzle::Hand`), a point-blank round can hit and despawn on the tick
    // it spawns, and `live_round` only sees surviving projectiles. That would
    // report "no round" for a working move.
    // Place him a fixed distance behind the shooter, not far away: a large
    // teleport can leave the stage and be clamped or culled.
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

    // The instrument proves itself first: if no round spawns, every sign
    // below is vacuous, so say which happened.
    let settle = |app: &mut App, frame: ControlFrame, ticks: usize| {
        for _ in 0..ticks {
            ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
            app.update();
        }
    };

    settle(&mut app, ControlFrame::default(), 120);

    // Turn him first, on its own, so the facing is settled before the special
    // is pressed. Pressing both on one tick conflates facing with stick.
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
    // The offset answers what the velocity cannot: a round with the right
    // velocity that leaves from behind him still reads as firing backwards.
    println!(
        "  tick  facing   move                     officer.x   round.x   offset   vel.x"
    );

    let mut rounds_seen = 0usize;
    // (tick, facing, observed offset, born offset with the tick of travel backed
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
            // Judge where the round was born, not where it was first seen. The
            // seen offset leads the born one by a tick of travel (see
            // `SIM_DT`).
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
