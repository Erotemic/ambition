//! WHAT DOES THE PERFORMER'S DOWN-B ACTUALLY DO, TICK BY TICK?
//!
//! `cargo run -p ambition_app_tools --bin trap_probe`
//!
//! ⭐⭐ THIS EXISTS BECAUSE THE MOVE HAS BEEN DECLARED FINISHED MORE THAN ONCE
//! AND KEEPS NOT BEING. Its authoring test proves the SPEC carries a policy; the
//! integration test proves she is `Submerged` for enough ticks and that a press
//! cuts it short. Neither of them can say how far she travelled, whether a door
//! was ever drawn, whether the emergence hit anybody, or what happens at a
//! ledge — which is every clause of the design except the two already guarded.
//!
//! It is OBSERVATIONAL. No thresholds, no pass/fail: it prints the lifecycle and
//! the reader compares it to the five stages `performer_moveset.rs` names.
//!
//! ⛔ IT DRIVES THE PRODUCTION INPUT ROAD. `drive_control_frame` is the only
//! driver that works on this host, and a probe that set `BodyMode::Submerged`
//! directly would measure the line it just wrote. Same reasoning as
//! `roll_probe`, and the same reasoning as `shark_ride_probe` for using ONE App
//! in ONE process so the global tracing subscriber is safe.
//!
//! ⛔ ONE press FRAME, THEN THE BUTTON COMES UP. `special_pressed` is a rising
//! edge, and *nobody holds B while steering* — the beat is a DURATION. Holding
//! it would measure a different move from the one a player performs.

#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_platformer2d::characters::control::DrivingParticipant;
use ambition_platformer2d::engine_core as ae_vec;
use ambition_platformer2d::engine_core::{BodyKinematics, BodyMode, BodyModeState, ControlFrame};
use bevy::prelude::*;

/// How long to watch after the press. 3s of hold + the exit beats, with room.
const WATCH_TICKS: usize = 260;

fn main() {
    // ⭐ WHICH WAY SHE STEERS. She stops dead partway through the beat, and a run
    // in one direction cannot tell a LEDGE from a distance cap: mirror it.
    let steer: f32 = match std::env::args().nth(1).as_deref() {
        Some("left") => -1.0,
        _ => 1.0,
    };
    // ⭐⭐ HOW LONG THE FINGER STAYS ON B. The first version of this probe
    // released the button after ONE frame and measured a healthy three-second
    // beat; Jon, playing, reported *"she just pops right back up"*. A person
    // does not tap a special for one frame, and `ChargeSustain::UntilPressedAgain`
    // guards its own starting press with `charge.held_s > 0.0` — a guard worth
    // exactly one tick. So the hold is the variable.
    // ⭐⭐ IS THE STICK STILL DOWN? You input down-B by HOLDING DOWN and pressing
    // B, and a thumb does not snap back to neutral the instant the move starts.
    // The probe's first runs steered with `axis_y = 0`, which is the one posture
    // a player is NOT in when this move begins.
    let down_held = std::env::args().any(|a| a == "downheld");
    let hold_frames: usize = std::env::args()
        .find_map(|a| a.strip_prefix("hold=").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    // ⭐⭐ WHICH HOST. `run_game.sh smash` launches `ambition_demo_smash_app`,
    // NOT `ambition_app` — a different shell composing its own catalogs. A probe
    // that only ever measures the app host cannot see a defect that lives in the
    // shell somebody plays, and this move has been reported broken in play while
    // measuring healthy here.
    let demo_host = std::env::args().any(|a| a == "host=demo");
    // ⭐⭐ THE OTHER POSTURE, AND IT IS A DIFFERENT MOVE. Jon, 2026-08-29: *"if
    // she isn't on the ground the trap door can't open and she can't go
    // subterranian, so the move cancels. So it doesn't do much in the air, just
    // a poof of smoke."* An `air` run should show NO submerged tick, NO door,
    // and a move that ends in a fraction of the grounded one's time — and the
    // grounded run beside it is what stops "no door" reading as success when the
    // move is simply broken.
    let airborne = std::env::args().any(|a| a == "air");
    // ⭐⭐ `render` GIVES THIS PROBE A PRESENTATION LAYER. `NoWindow` omits the
    // render app entirely — 0 body visuals — so every presentation question this
    // probe asked was unanswerable. `OffscreenGpu` is a REAL wgpu backend with no
    // window, which is the mode that makes the visibility chain observable while
    // the press road still works. This is the overlap `shark_ride_probe`'s doc
    // says did not exist: the suite could reach the behaviour and not see it,
    // the capture could see it and not reach it.
    let rendered = std::env::args().any(|a| a == "render");
    // ⛔⛔ THE HOST, THE PLUGIN PUMP, THE TWO HOSTS' DIFFERENT ROUND
    // ANNOUNCEMENTS AND TAKING THE BRAIN OFF ARE ALL `probe_stage`'S NOW. Four
    // findings live in that file and every one of them was a run that measured
    // nothing; `wire_probe` needed the same four, and two copies of them would
    // have drifted the first time either was corrected.
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

    // ⛔⛔ THE INSTRUMENT PROVES ITSELF FIRST. `door_count` below queries a
    // PRESENTATION component, and a presentation layer that was never installed
    // answers zero for the same reason a missing door does. So say how many
    // body visuals exist: if that is zero too, every presentation number in this
    // run is uninformative rather than a finding.
    println!(
        "[trap_probe] presentation: {} body visuals live (0 means the door \
         numbers below say NOTHING about the door)",
        probe_stage::player_visuals(&mut app)
    );

    // ⛔ WHO IS DRIVING THIS BODY, said out loud. `probe_stage` has already
    // taken the brain off; this is the confirmation, and `brain=false` is what
    // makes every delivered `ControlFrame` below mean anything.
    println!(
        "[trap_probe] seat 0: participant={} brain={}",
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        app.world()
            .get::<ambition_platformer2d::characters::brain::Brain>(seat0)
            .is_some(),
    );

    // She must be STANDING when the press lands: down-Special in the air is
    // `special_air_down`, a different verb on the same table.
    for _ in 0..120 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame::default(),
        );
        app.update();
    }
    if airborne {
        // ⛔ PROBE-SIDE PLACEMENT, honest for an instrument: it lifts her OFF the
        // boards, it does not change what the move does from there. High enough
        // that she is unambiguously airborne for the whole press, and moving, so
        // the run cannot be read as a body resting on invisible ground.
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
            // +y is DOWN.
            axis_y: 1.0,
            special_pressed: true,
            special_held: true,
            ..Default::default()
        },
    );
    app.update();
    // The button STAYS DOWN for `hold_frames` after the press edge, which is
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
    // She steers RIGHT the whole time. Steering is what the subterranean beat is
    // FOR, so a run that leaves the stick centred cannot see the clause the
    // design spends the most words on.
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
    // ⛔⛔ THE SMOKE IS COUNTED, NOT ASSUMED. It is the one thing the airborne
    // form produces, so "the move played" and "the player saw anything" are the
    // same question here — and a moveset test can only prove the effect is
    // AUTHORED. What matters is that it is EMITTED.
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

        // ⭐⭐ STAGE THE ONE CASE THE MOVE IS ABOUT. Jon: *"damages whoever is
        // on top or above the trap door when she emerges."* A rival left where
        // the match put him is never above her, so the emergence window can be
        // live and hit nothing and the run reads clean. Park him ON the door for
        // the frames she is coming up.
        //
        // ⛔ PROBE-SIDE PLACEMENT, and it is honest for an instrument: it moves
        // WHO is standing there, not what the move does to them.
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
                // The door is the ONLY thing on stage that says where she is.
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
                 under={under:<5} doors={doors} {vis} boxes={boxes} move={}",
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

/// How many `smoke_burst` effects were REQUESTED this tick.
///
/// ⛔ COUNTED OFF THE EMITTED MESSAGE, not off the timeline. A moveset test can
/// prove the event is authored; only this can say the move reached the effect
/// system, which is the half the Trap's presentation was missing for weeks.
fn drain_smoke(
    app: &mut App,
    cursor: &mut bevy::ecs::message::MessageCursor<ambition_platformer2d::vfx::vfx::VfxMessage>,
    near: ae_vec::Vec2,
) -> usize {
    // ⛔⛔ NEAR HER, BECAUSE SEAT 1 IS A PERFORMER TOO. It keeps its brain and
    // presses its own down-B on its own schedule, so a stage-wide count reports
    // TWO bursts for one press and measures the CPU. `VfxMessage` carries a
    // position and no owner, so proximity is the join available — the same
    // correction `wire_probe` had to make about counting ropes.
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

/// ⛔⛔ THE WHOLE VISIBILITY CHAIN, IN ONE STRING — because Jon's report is that
/// the SIM half works and the sprite draws anyway: *"she can move around while in
/// the submerged state, but her sprite still draws on the stage and with blinking
/// invincibility."*
///
/// Three links, and naming which one is broken is the entire question:
///   `BodyMode::Submerged` -> `BodyPoseView.submerged` -> `Visibility::Hidden`
///
/// So print, over every body the presentation layer built: how many views say
/// submerged, and how many of those are actually hidden. A body that is
/// submerged-in-view and NOT hidden is `sync_submerged_visibility` failing or
/// being overwritten; a body submerged in the sim whose VIEW says otherwise is
/// the projection.
fn visibility_chain(app: &mut App) -> String {
    // The PLAYER road: one body, `PlayerVisual` + `BodyPoseView`.
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
    // ⛔⛔ THE ACTOR ROAD, WHICH IS THE ONE A MATCH FIGHTER TAKES and the one
    // this probe could not see. `BodyPoseView` is player-bodied only —
    // `debug_viz.rs` says so in as many words — so a probe that only counted
    // those reported `views=0` in a live match and learned nothing. Every
    // fighter is a `FeatureVisual` whose visibility comes from
    // `FeatureViewIndex`.
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

/// ⛔⛔ THE RELEASE CONDITION, READ OUT LOUD. `ChargeSustain::UntilPressedAgain`
/// ends the freeze when the body's frame carries
/// `ActorControlFrame::action_press_that_is_not_movement` and
/// `charge.held_s > 0.0`.
///
/// ⚠ THIS PROBE STILL PRINTS THE ATTACK GESTURE, which is now a SUBSET of what
/// the condition reads: Attack and Special are two of the six verbs in that
/// set, and a grab, taunt, projectile or Interact will end the freeze without
/// changing a single number below. Read a surviving freeze here as "no ATTACK
/// or SPECIAL press", not as "no press at all".
///
/// ⭐ THE ONE-TICK GUARD IS STILL THE SUBTLE PART. `special` is *"the SPECIAL
/// press, live or REPLAYED FROM THE BUFFER"*, so the move's own starting press
/// can end the freeze one tick later if the buffer is still replaying it, and
/// `held_s > 0.0` is worth exactly one tick against that. Whether it fires is
/// not a thing to reason about; it is a thing to print.
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
/// Live trapdoor visuals in the world — the thing that tells an opponent where
/// she is, and the half of Jon's ask that hiding her body does not answer.
fn door_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::render::rendering::submerged::TrapdoorVisual>();
    q.iter(world).count()
}

/// ⛔ `damage_taken()`, NOT `current()`. Under smash rules a fighter's health
/// stays at its maximum and the accumulated damage is what a launch scales off,
/// so `current()` reads 100 -> 100 through a connection that landed. The first
/// run of this probe reported exactly that and it meant nothing.
fn health(app: &App, body: Entity) -> Option<i32> {
    app.world()
        .get::<ambition_platformer2d::characters::actor::BodyHealth>(body)
        .map(|h| h.damage_taken())
}
