//! What does the performer's down-B do, tick by tick?
//!
//! `cargo run -p ambition_app_tools --bin trap_probe`
//!
//! The authoring test proves the spec carries a policy, and the integration
//! test proves she is `Submerged` long enough and that a press cuts it short.
//! Neither shows how far she travelled, whether a door was drawn, whether the
//! emergence hit anybody, or what happens at a ledge. This probe does.
//!
//! It is observational: no thresholds, no pass/fail. It prints the lifecycle,
//! and the reader compares it with the five stages `performer_moveset.rs`
//! names.
//!
//! It drives the production input path (`drive_control_frame`); setting
//! `BodyMode::Submerged` directly would measure the line it wrote. One App in
//! one process, so the global tracing subscriber is safe (as in
//! `roll_probe` and `shark_ride_probe`).
//!
//! One press frame, then the button comes up by default: `special_pressed`
//! is a rising edge, and the beat is a duration, not a hold.

#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_platformer2d::characters::control::DrivingParticipant;
use ambition_platformer2d::engine_core as ae_vec;
use ambition_platformer2d::engine_core::{BodyKinematics, BodyMode, BodyModeState, ControlFrame};
use bevy::prelude::*;

/// How long to watch after the press. 3s of hold + the exit beats, with room.
const WATCH_TICKS: usize = 260;

fn main() {
    // Which way she steers. She stops partway through the beat, and only a
    // mirrored run separates a ledge from a distance cap.
    let steer: f32 = match std::env::args().nth(1).as_deref() {
        Some("left") => -1.0,
        _ => 1.0,
    };
    // How long B stays down. A person does not tap a special for one frame,
    // and `ChargeSustain::UntilPressedAgain` guards its starting press with
    // `charge.held_s > 0.0`, which is worth one tick. So the hold is a
    // variable.
    // Whether the stick stays down: down-B is input by holding down and
    // pressing B, and a thumb does not snap back to neutral at once.
    let down_held = std::env::args().any(|a| a == "downheld");
    let hold_frames: usize = std::env::args()
        .find_map(|a| a.strip_prefix("hold=").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    // Which host. `run_game.sh smash` launches `ambition_demo_smash_app`, not
    // `ambition_app`, and that shell composes its own catalogs. A defect can
    // live in the shell people play.
    let demo_host = std::env::args().any(|a| a == "host=demo");
    // The other posture, which is a different move. In the air the trap door
    // cannot open, so the move cancels into a smoke poof. An `air` run should
    // show no submerged tick, no door, and a much shorter move. Compare it with
    // a grounded run, so "no door" is not read as success when the move is
    // broken.
    let airborne = std::env::args().any(|a| a == "air");
    // `render` gives this probe a presentation layer. `NoWindow` omits the
    // render app (0 body visuals). `OffscreenGpu` is a real wgpu backend with
    // no window, so visibility is observable while the press path still
    // works.
    let rendered = std::env::args().any(|a| a == "render");
    // Host setup, plugin pump, per-host round detection, and removing the
    // brain live in `probe_stage`, shared with `wire_probe`.
    let probe_stage::Staged {
        mut app,
        seat0,
        seat1,
    } = probe_stage::stage(probe_stage::StageRequest {
        cast: ["performer", "performer"],
        demo_host,
        rendered,
    });
    println!(
        "[trap_probe] host = {}",
        if demo_host {
            "ambition_demo_smash_app (the one run_game.sh smash launches)"
        } else {
            "ambition_app::build_visible_app"
        }
    );

    // The instrument proves itself first. `door_count` queries a
    // presentation component, and a missing presentation layer also answers
    // zero. If body visuals are zero, presentation numbers mean nothing.
    println!(
        "[trap_probe] presentation: {} body visuals live (0 means the door \
         numbers below say NOTHING about the door)",
        probe_stage::player_visuals(&mut app)
    );

    // Confirm who drives this body: `probe_stage` removed the brain, and
    // `brain=false` means the delivered `ControlFrame`s are the ones read.
    println!(
        "[trap_probe] seat 0: participant={} brain={}",
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        app.world()
            .get::<ambition_platformer2d::characters::brain::Brain>(seat0)
            .is_some(),
    );

    // She must be standing when the press lands: down-Special in the air is
    // `special_air_down`, a different verb on the same table.
    for _ in 0..120 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame::default(),
        );
        app.update();
    }
    if airborne {
        // Probe-side placement: it lifts her off the boards; it does not
        // change what the move does. High enough to stay airborne for the
        // whole press, and moving, so it cannot be read as resting on
        // invisible ground.
        if let Some(mut k) = app.world_mut().get_mut::<BodyKinematics>(seat0) {
            k.pos.y -= 180.0;
            k.vel.y = -60.0;
        }
        app.update();
    }

    let start = probe_stage::kin(&app, seat0).0;
    println!(
        "[trap_probe] posture = {}",
        if airborne {
            "AIRBORNE — expect smoke and nothing else"
        } else {
            "grounded"
        }
    );
    println!(
        "[trap_probe] standing at ({:.1}, {:.1}), steering {}, B held {hold_frames} frames, down {}",
        start.x,
        start.y,
        if steer > 0.0 { "RIGHT" } else { "LEFT" },
        if down_held { "HELD" } else { "released" }
    );

    ambition_platformer2d::sim::drive_control_frame(
        app.world_mut(),
        ControlFrame {
            // +y is down.
            axis_y: 1.0,
            special_pressed: true,
            special_held: true,
            ..Default::default()
        },
    );
    app.update();
    // The button stays down for `hold_frames` after the press edge, which is
    // what a thumb does.
    for _ in 0..hold_frames {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_y: 1.0,
                special_held: true,
                ..Default::default()
            },
        );
        app.update();
    }

    // ── the watch ────────────────────────────────────────────────────────────
    //
    // She steers right the whole time, because steering is what the
    // subterranean beat is for.
    let mut submerged_ticks = 0usize;
    let mut first_under: Option<usize> = None;
    let mut last_under: Option<usize> = None;
    let mut doors_seen = 0usize;
    let mut peak_hitboxes = 0usize;
    let mut hitbox_ticks = 0usize;
    let mut visible_while_under = 0usize;
    let mut under_start_x = 0.0f32;
    let mut under_end_x = 0.0f32;
    let mut move_ended_at: Option<usize> = None;
    let mut under_ticks_seen = 0usize;
    // Count the smoke, which is the one thing the airborne form produces. A
    // moveset test proves it is authored; this proves it is emitted.
    let mut smoke_bursts = 0usize;
    let mut vfx_cursor = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::vfx::VfxMessage>>()
        .get_cursor();
    let rival_hp_before = health(&app, seat1);

    for tick in 0..WATCH_TICKS {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_x: steer,
                axis_y: if down_held { 1.0 } else { 0.0 },
                ..Default::default()
            },
        );
        app.update();

        let hers = probe_stage::kin(&app, seat0).0;
        smoke_bursts += drain_smoke(&mut app, &mut vfx_cursor, ae_vec::Vec2::new(hers.x, hers.y));
        let (pos, vel) = probe_stage::kin(&app, seat0);
        let under = matches!(mode(&app, seat0), Some(BodyMode::Submerged));
        let doors = door_count(&mut app);
        let boxes = probe_stage::hitbox_count(&mut app, seat0);
        let playing = probe_stage::playing_move(&app, seat0);
        let gest = gesture(&app, seat0);
        let vis = visibility_chain(&mut app);

        // Stage the case the move is about: it damages whoever is on or above
        // the trap door when she emerges. Park the rival on the door while she
        // comes up. This moves who stands there, not what the move does.
        if under_ticks_seen > 150 {
            let hers = probe_stage::kin(&app, seat0).0;
            if let Some(mut k) = app.world_mut().get_mut::<BodyKinematics>(seat1) {
                k.pos.x = hers.x;
                k.pos.y = hers.y - 24.0;
                k.vel.x = 0.0;
            }
        }

        if under {
            under_ticks_seen += 1;
            if first_under.is_none() {
                first_under = Some(tick);
                under_start_x = pos.x;
            }
            last_under = Some(tick);
            under_end_x = pos.x;
            submerged_ticks += 1;
            if doors == 0 {
                // The door is the only thing on stage that shows where she is.
                visible_while_under += 1;
            }
        }
        doors_seen = doors_seen.max(doors);
        peak_hitboxes = peak_hitboxes.max(boxes);
        if boxes > 0 {
            hitbox_ticks += 1;
        }
        if playing.is_none() && move_ended_at.is_none() && tick > 4 {
            move_ended_at = Some(tick);
        }

        // Print the interesting frames rather than all 260: every transition,
        // plus a sample through the long hold.
        let interesting = tick < 24 || tick % 20 == 0 || boxes > 0 || move_ended_at == Some(tick)
            || (180..216).contains(&tick);
        if interesting {
            println!(
                "[trap_probe] t{tick:>3} pos=({:>7.1},{:>7.1}) vel=({:>7.1},{:>7.1}) \
                 under={under:<5} doors={doors} {vis} boxes={boxes} {gest} move={}",
                pos.x,
                pos.y,
                vel.x,
                vel.y,
                playing.unwrap_or_else(|| "-".to_string()),
            );
        }
    }

    let rival_hp_after = health(&app, seat1);
    let end = probe_stage::kin(&app, seat0).0;

    println!("[trap_probe] ── the five stages, measured ──");
    println!(
        "[trap_probe] SMOKE emitted {smoke_bursts} time(s) — the misdirection \
         the whole trick hides behind, and the ONLY thing the airborne form does"
    );
    println!(
        "[trap_probe] SUBMERGED for {submerged_ticks} ticks \
         (first t{}, last t{})",
        first_under.map(|t| t.to_string()).unwrap_or("never".into()),
        last_under.map(|t| t.to_string()).unwrap_or("never".into()),
    );
    println!(
        "[trap_probe] TRAVELLED UNDER {:.1}px while steering right \
         (x {under_start_x:.1} -> {under_end_x:.1})",
        (under_end_x - under_start_x).abs(),
    );
    println!(
        "[trap_probe] NET DISPLACEMENT {:.1}px (x {:.1} -> {:.1})",
        (end.x - start.x).abs(),
        start.x,
        end.x
    );
    println!(
        "[trap_probe] TRAPDOOR VISUALS peaked at {doors_seen}; \
         {visible_while_under} of {submerged_ticks} submerged ticks had NO door on stage"
    );
    println!(
        "[trap_probe] EMERGENCE HITBOX: peak {peak_hitboxes} live, on {hitbox_ticks} ticks; \
         rival DAMAGE TAKEN {rival_hp_before:?} -> {rival_hp_after:?}"
    );
    println!(
        "[trap_probe] MOVE ENDED at t{}",
        move_ended_at
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!(">{WATCH_TICKS}"))
    );
    if airborne {
        println!(
            "[trap_probe] ⇒ AIRBORNE VERDICT: {} submerged ticks (want 0), \
             {doors_seen} doors (want 0), move ended at t{} (the grounded form \
             runs past t200). A poof of smoke and nothing else is the design.",
            submerged_ticks,
            move_ended_at
                .map(|t| t.to_string())
                .unwrap_or_else(|| format!(">{WATCH_TICKS}")),
        );
    }
    println!(
        "[trap_probe] ⇒ compare against the five stages in `performer_moveset.rs`: \
         door opens, she sinks, she STEERS under, the exit door opens, she leaps out \
         into a firework that hits above the door."
    );
}

/// How many `smoke_burst` effects were requested this tick.
///
/// Counted from the emitted message, not the timeline: this shows the move
/// reached the effect system.
fn drain_smoke(
    app: &mut App,
    cursor: &mut bevy::ecs::message::MessageCursor<ambition_platformer2d::vfx::vfx::VfxMessage>,
    near: ae_vec::Vec2,
) -> usize {
    // Near her, because seat 1 is a performer too and presses its own down-B.
    // `VfxMessage` has a position and no owner, so proximity is the join
    // (as in `wire_probe`).
    const NEARBY_PX: f32 = 120.0;
    let smoke = ambition_platformer2d::vfx::fx::FxId::new("smoke_puff");
    let messages = app
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::vfx::VfxMessage>>();
    cursor
        .read(messages)
        .filter(|m| match m {
            ambition_platformer2d::vfx::vfx::VfxMessage::Effect { fx, pos, .. } => {
                *fx == smoke
                    && (pos.x - near.x).abs() < NEARBY_PX
                    && (pos.y - near.y).abs() < NEARBY_PX
            }
            _ => false,
        })
        .count()
}

/// The whole visibility chain in one string. The sim half can work while the
/// sprite still draws.
///
/// Three links:
///   `BodyMode::Submerged` -> `BodyPoseView.submerged` -> `Visibility::Hidden`
///
/// Over every body the presentation layer built, print how many views say
/// submerged and how many of those are hidden. Submerged in view but not
/// hidden means `sync_submerged_visibility` fails or is overwritten; submerged
/// in the sim but not in the view means the projection.
fn visibility_chain(app: &mut App) -> String {
    // The player path: one body, `PlayerVisual` + `BodyPoseView`.
    let (mut pviews, mut psub, mut psub_hidden) = (0usize, 0usize, 0usize);
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::sim_view::BodyPoseView,
            &bevy::prelude::Visibility,
        )>();
        for (pose, vis) in q.iter(world) {
            pviews += 1;
            if pose.submerged {
                psub += 1;
                if matches!(vis, bevy::prelude::Visibility::Hidden) {
                    psub_hidden += 1;
                }
            }
        }
    }
    // The actor path, which a match fighter takes. `BodyPoseView` is
    // player-only (see `debug_viz.rs`). Every fighter is a `FeatureVisual`
    // whose visibility comes from `FeatureViewIndex`.
    let index = app
        .world()
        .get_resource::<ambition_platformer2d::sim_view::FeatureViewIndex>()
        .cloned();
    let (mut aviews, mut asub, mut asub_hidden) = (0usize, 0usize, 0usize);
    if let Some(index) = index {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::render::rendering::FeatureVisual,
            &bevy::prelude::Visibility,
        )>();
        for (visual, vis) in q.iter(world) {
            let Some(view) = index.get(&visual.id) else {
                continue;
            };
            aviews += 1;
            if view.submerged {
                asub += 1;
                if matches!(vis, bevy::prelude::Visibility::Hidden) {
                    asub_hidden += 1;
                }
            }
        }
    }
    format!(
        "player[{pviews}/{psub}/{psub_hidden}] actor[{aviews}/{asub}/{asub_hidden}]"
    )
}

/// The release condition. `ChargeSustain::UntilPressedAgain` ends the freeze
/// when the body's frame has
/// `ActorControlFrame::action_press_that_is_not_movement` and
/// `charge.held_s > 0.0`.
///
/// This prints only the attack gesture. Attack and Special are two of the six
/// verbs the condition reads; a grab, taunt, projectile, or Interact also ends
/// the freeze. A surviving freeze here means "no Attack or Special press".
///
/// The one-tick guard: `special` includes a press replayed from the buffer,
/// so the move's own starting press can end the freeze a tick later.
/// `held_s > 0.0` guards one tick only. Print it; do not reason about it.
fn gesture(app: &App, body: Entity) -> String {
    let g = app
        .world()
        .get::<ambition_platformer2d::characters::actor::attack_gesture::ResolvedAttackGesture>(
            body,
        );
    let charge = app
        .world()
        .get::<ambition_platformer2d::combat::moveset::MovePlayback>(body)
        .and_then(|p| p.charge.as_ref().map(|c| c.held_s));
    match g {
        None => "gest=NONE".to_string(),
        Some(g) => format!(
            "press={} spec={} sheld={} held_s={}",
            g.pressed.is_some() as u8,
            g.special.is_some() as u8,
            g.special_held as u8,
            charge.map(|h| format!("{h:.2}")).unwrap_or("-".into()),
        ),
    }
}

fn mode(app: &App, body: Entity) -> Option<BodyMode> {
    app.world()
        .get::<BodyModeState>(body)
        .map(|state| state.body_mode)
}
/// Live trapdoor visuals: what tells an opponent where she is. Hiding her body
/// does not answer that.
fn door_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::render::rendering::submerged::TrapdoorVisual>();
    q.iter(world).count()
}

/// `damage_taken()`, not `current()`. Under smash rules health stays at its
/// maximum and accumulated damage is what launches scale from.
fn health(app: &App, body: Entity) -> Option<i32> {
    app.world()
        .get::<ambition_platformer2d::characters::actor::BodyHealth>(body)
        .map(|h| h.damage_taken())
}
