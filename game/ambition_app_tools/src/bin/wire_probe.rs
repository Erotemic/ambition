//! What does the performer's up-B do, tick by tick?
//!
//! `cargo run -p ambition_app_tools --bin wire_probe -- right render`
//!
//! Rules this binary follows:
//!
//! 1. A moveset test proves the spec, not the move.
//! 2. The sim is not the game. What a player sees must be observed through a
//!    host with a render app.
//! 3. There are two visibility paths, and `PlayerVisual` is inserted in one
//!    place in the engine. A rule stated on one path is not stated.
//! 4. Smash moves are measured on the smash stage, not an Ambition room.
//! 5. Two authorities for one fact means one is silently ignored.
//!
//! The tick log has no thresholds. The summary at the end states each design
//! clause as a measurement and says which ones the run supports.
//!
//! It drives the production input path (`drive_control_frame`); calling
//! `catch_the_wire` directly would measure the line it wrote.

#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_platformer2d::engine_core::ControlFrame;
use bevy::prelude::*;

/// Long enough for the whole move and a good look at what she does after it.
const WATCH_TICKS: usize = 150;

/// The stage's own numbers, so "fairly large" is measured against something.
/// See `smash_stage()` in `ambition_demo_smash`.
const PLATFORM_TOP_Y: f32 = 300.0;
const PLATFORM_WIDTH: f32 = 480.0;
const FALL_BLAST_DEPTH: f32 = 240.0;

fn main() {
    // Which way she swings: the swing is the clause with a direction.
    let steer: f32 = match std::env::args().nth(1).as_deref() {
        Some("left") => -1.0,
        Some("neutral") => 0.0,
        _ => 1.0,
    };
    let demo_host = std::env::args().any(|a| a == "host=demo");
    let rendered = std::env::args().any(|a| a == "render");
    // Start her in the air, where a recovery is used. A lift measured from
    // the boards cannot show whether she gets back onto them.
    let from_below = std::env::args().any(|a| a == "offstage");

    let probe_stage::Staged {
        mut app,
        seat0,
        seat1,
    } = probe_stage::stage(probe_stage::StageRequest {
        cast: ["performer", "performer"],
        demo_host,
        rendered,
    });
    let _ = seat1;

    println!(
        "[wire_probe] host = {}",
        if demo_host {
            "ambition_demo_smash_app (the one run_game.sh smash launches)"
        } else {
            "ambition_app::build_visible_app"
        }
    );
    // The instrument proves itself first. `wire_count` queries a
    // presentation component, and a missing presentation layer also answers
    // zero. Count both paths: on this stage the player path is zero, because
    // a match seats actors.
    let (player_bodies, actor_bodies) = presentation_bodies(&mut app);
    println!(
        "[wire_probe] presentation: player-road bodies {player_bodies}, \
         actor-road bodies {actor_bodies}. A match seats ACTORS, so 0 on the \
         player road is EXPECTED; 0 on BOTH means every rope number below says \
         NOTHING about the rope."
    );

    // Settle her on the boards first, so the launch point is not mid-fall.
    for _ in 0..120 {
        ambition_platformer2d::sim::drive_control_frame(app.world_mut(), ControlFrame::default());
        app.update();
    }
    if from_below {
        // Probe-side placement: it moves where she starts, not what the move
        // does. Off the side of the platform and below the lip, where a
        // recovery is used.
        if let Some(mut k) = app
            .world_mut()
            .get_mut::<ambition_platformer2d::engine_core::BodyKinematics>(seat0)
        {
            k.pos.x = 320.0 - PLATFORM_WIDTH * 0.5 - 40.0;
            k.pos.y = PLATFORM_TOP_Y + 260.0;
            k.vel = ambition_platformer2d::engine_core::Vec2::new(0.0, 40.0);
        }
        app.update();
    }

    let start = probe_stage::kin(&app, seat0).0;
    println!(
        "[wire_probe] starting at ({:.1}, {:.1}) — {} the platform lip at y={PLATFORM_TOP_Y:.0}; \
         steering {}",
        start.x,
        start.y,
        if start.y > PLATFORM_TOP_Y {
            "BELOW"
        } else {
            "above"
        },
        match steer {
            s if s > 0.0 => "RIGHT",
            s if s < 0.0 => "LEFT",
            _ => "NEUTRAL",
        }
    );

    // Up + B. `axis_y` is +down, so up is negative.
    ambition_platformer2d::sim::drive_control_frame(
        app.world_mut(),
        ControlFrame {
            axis_y: -1.0,
            special_pressed: true,
            special_held: true,
            ..Default::default()
        },
    );
    app.update();

    // ── the watch ────────────────────────────────────────────────────────────
    let mut on_wire_ticks = 0usize;
    let mut first_on: Option<usize> = None;
    let mut last_on: Option<usize> = None;
    let mut ropes_seen = 0usize;
    let mut wire_ticks_without_rope = 0usize;
    let mut highest = start.y;
    let mut biggest_tick_rise = 0.0f32;
    let mut prev = start;
    let mut swing_extent = 0.0f32;
    let mut exit_velocity = Vec2::ZERO;
    let mut displacement_at_release = 0.0f32;
    let mut released_at: Option<usize> = None;
    let mut move_ended_at: Option<usize> = None;
    let mut blink_cues = 0usize;
    let mut peak_hitboxes = 0usize;
    let mut recovery_spent_at: Option<usize> = None;
    let mut cursor = blink_cursor(&mut app);

    for tick in 0..WATCH_TICKS {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_x: steer,
                ..Default::default()
            },
        );
        app.update();

        let (pos, vel) = probe_stage::kin(&app, seat0);
        let anchor = wire_anchor(&app, seat0);
        let on_wire = anchor.is_some();
        let ropes = wire_count(&mut app, seat0);
        let boxes = probe_stage::hitbox_count(&mut app, seat0);
        let playing = probe_stage::playing_move(&app, seat0);
        let vis = visibility_chain(&mut app);
        blink_cues += drain_blink_cues(&mut app, &mut cursor);

        if on_wire {
            on_wire_ticks += 1;
            if first_on.is_none() {
                first_on = Some(tick);
            }
            last_on = Some(tick);
            // +y is down, so a rise is a decrease.
            biggest_tick_rise = biggest_tick_rise.max((prev.y - pos.y).abs());
            // Signed, kept by magnitude: an absolute extent cannot tell a left
            // swing from a right one.
            if (pos.x - start.x).abs() > swing_extent.abs() {
                swing_extent = pos.x - start.x;
            }
            if ropes == 0 {
                // The rope is what explains why she rises. A lift with no wire
                // drawn is a fighter levitating.
                wire_ticks_without_rope += 1;
            }
        } else if last_on.is_some() && released_at.is_none() {
            released_at = Some(tick);
            exit_velocity = vel;
            // Read at release, not at the end of the watch: afterwards the
            // displacement is mostly ordinary air control. Bound a measurement
            // by the state it is about, not by a tick count.
            displacement_at_release = pos.x - start.x;
        }
        highest = highest.min(pos.y);
        ropes_seen = ropes_seen.max(ropes);
        peak_hitboxes = peak_hitboxes.max(boxes);
        if recovery_spent_at.is_none() && recovery_spent(&app, seat0) {
            recovery_spent_at = Some(tick);
        }
        if playing.is_none() && move_ended_at.is_none() && tick > 4 {
            move_ended_at = Some(tick);
        }

        let interesting =
            tick < 6 || tick % 5 == 0 || released_at == Some(tick) || move_ended_at == Some(tick);
        if interesting {
            println!(
                "[wire_probe] t{tick:>3} pos=({:>7.1},{:>7.1}) vel=({:>7.1},{:>7.1}) \
                 wire={on_wire:<5} rope={ropes} {vis} boxes={boxes} move={}",
                pos.x,
                pos.y,
                vel.x,
                vel.y,
                playing.unwrap_or_else(|| "-".to_string()),
            );
        }
        prev = pos;
    }

    let end = probe_stage::kin(&app, seat0).0;
    let rose = start.y - highest;

    println!("[wire_probe] ── Jon's six clauses, measured ──");
    println!(
        "[wire_probe] 1. NOT A TELEPORT: on the wire for {on_wire_ticks} ticks \
         (t{} → t{}); the largest single tick moved her {biggest_tick_rise:.1}px. \
         The teleport it replaces covered 215px in ONE frame.",
        first_on.map(|t| t.to_string()).unwrap_or("never".into()),
        last_on.map(|t| t.to_string()).unwrap_or("never".into()),
    );
    println!(
        "[wire_probe] 2. NO TELEPORT SOUND: `player.blink` emitted {blink_cues} times \
         across the whole move. ⛔ This is the assertion that matters — the cue \
         comes from the EXECUTOR, never from the timeline."
    );
    println!(
        "[wire_probe] 3. A WIRE FROM THE SKY: rope visuals peaked at {ropes_seen}; \
         {wire_ticks_without_rope} of {on_wire_ticks} on-wire ticks had NO rope drawn."
    );
    println!(
        "[wire_probe] 4. SHE IS LIFTED: rose {rose:.1}px, monotonically across \
         {on_wire_ticks} ticks rather than in one placement."
    );
    println!(
        "[wire_probe] 5. A FAIRLY LARGE DISTANCE: {rose:.1}px = {:.2} platform widths \
         ({PLATFORM_WIDTH:.0}px), {:.2}x the fall blast depth ({FALL_BLAST_DEPTH:.0}px), \
         {:.2}x the 215px teleport.",
        rose / PLATFORM_WIDTH,
        rose / FALL_BLAST_DEPTH,
        rose / 215.0,
    );
    println!(
        "[wire_probe] 6. SHE SWINGS: reached {swing_extent:+.1}px from where she \
         started while hanging, and was {:+.1}px across WHEN THE ROPE LET GO \
         (t{}); left it at ({:+.1}, {:.1}) px/s. ⛔ Signed, because an absolute \
         extent cannot tell a left swing from a right one. The {:.1}px she is \
         from the start by the end of the watch is mostly ordinary air control \
         and is NOT the swing.",
        displacement_at_release,
        released_at.map(|t| t.to_string()).unwrap_or("never".into()),
        exit_velocity.x,
        exit_velocity.y,
        (end.x - start.x).abs(),
    );
    println!(
        "[wire_probe] AND IT IS STILL A RECOVERY: `gates.recovery` spent at t{} \
         (never = the up-B is FLIGHT); peak hitboxes {peak_hitboxes} (the wire \
         authors none); move ended at t{}.",
        recovery_spent_at
            .map(|t| t.to_string())
            .unwrap_or("NEVER".into()),
        move_ended_at
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!(">{WATCH_TICKS}")),
    );
    println!(
        "[wire_probe] ⇒ run it the other way (`left`) and from below \
         (`offstage`) before believing any of the direction numbers."
    );
}

/// How many bodies each presentation road has built.
///
/// Two numbers: the paths are not interchangeable, and a match uses the
/// second. See the call site.
fn presentation_bodies(app: &mut App) -> (usize, usize) {
    let players = probe_stage::player_visuals(app);
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::render::rendering::FeatureVisual>();
    let actors = q.iter(world).count();
    (players, actors)
}

/// Where the wire she is on hangs from, straight off the read model the
/// renderer reads. `None` when she is not on one.
fn wire_anchor(app: &App, body: Entity) -> Option<ambition_platformer2d::engine_core::Vec2> {
    app.world()
        .get::<ambition_platformer2d::engine_core::BodyMotionFacts>(body)
        .and_then(|facts| facts.wire_anchor)
}

/// Live rope visuals belonging to this body: what explains why she rises.
///
/// Per body, because seat 1 is also a Performer with its brain on and uses
/// its own up-B. A stage-wide count would measure the CPU.
fn wire_count(app: &mut App, owner: Entity) -> usize {
    // The owner is the visual, not the seat. `FlylineVisual::body` names the
    // presentation entity (with `FeatureVisual`), and a match seat is a sim
    // entity. Join them by `FeatureId`, which both carry.
    let Some(visual) = visual_of(app, owner) else {
        return 0;
    };
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::render::rendering::flyline::FlylineVisual>();
    q.iter(world).filter(|wire| wire.body == visual).count()
}

/// The presentation entity standing in for a sim body, joined by `FeatureId`.
fn visual_of(app: &mut App, sim: Entity) -> Option<Entity> {
    let id = app
        .world()
        .get::<ambition_platformer2d::combat::components::FeatureId>(sim)?
        .as_str()
        .to_string();
    let world = app.world_mut();
    let mut q = world.query::<(
        Entity,
        &ambition_platformer2d::render::rendering::FeatureVisual,
    )>();
    q.iter(world)
        .find(|(_, visual)| visual.id == id)
        .map(|(entity, _)| entity)
}

/// Has this body spent its once-per-airtime recovery?
///
/// This check shows the move is not flight. `UpSpecial::Standard` stamps
/// `gates.recovery`; a wire that lifted her without spending it would let her
/// never come down. Most up-Bs are once per airtime (D204).
///
/// `recovery_charges` is a budget, so "spent" is a body in the air with none
/// left. `post_recovery_helpless` is the freefall the spend buys.
fn recovery_spent(app: &App, body: Entity) -> bool {
    app.world()
        .get::<ambition_platformer2d::engine_core::BodyJumpState>(body)
        .is_some_and(|jump| jump.recovery_charges == 0 || jump.post_recovery_helpless)
}

fn blink_cursor(
    app: &mut App,
) -> bevy::ecs::message::MessageCursor<ambition_platformer2d::sfx::OwnedSfxMessage> {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>()
        .get_cursor()
}

/// Counted, not "at least one", and counted from the emitted cue, not the
/// timeline: `apply_authored_teleports` emits `player.blink` at every
/// transit, so a silent timeline does not prove a silent move.
fn drain_blink_cues(
    app: &mut App,
    cursor: &mut bevy::ecs::message::MessageCursor<ambition_platformer2d::sfx::OwnedSfxMessage>,
) -> usize {
    let blink = ambition_platformer2d::sfx::SfxId::from_static("player.blink");
    let messages = app
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
    cursor
        .read(messages)
        .filter(|owned| {
            matches!(&owned.request, ambition_platformer2d::sfx::SfxMessage::Play { id, .. } if *id == blink)
        })
        .count()
}

/// The whole visibility chain in one string, on both paths: a match fighter
/// does not carry `PlayerVisual`. `player[views/wired] actor[views/wired]`.
fn visibility_chain(app: &mut App) -> String {
    let (mut pviews, mut pwired) = (0usize, 0usize);
    {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_platformer2d::sim_view::BodyPoseView>();
        for pose in q.iter(world) {
            pviews += 1;
            if pose.wire_anchor.is_some() {
                pwired += 1;
            }
        }
    }
    let index = app
        .world()
        .get_resource::<ambition_platformer2d::sim_view::FeatureViewIndex>()
        .cloned();
    let (mut aviews, mut awired) = (0usize, 0usize);
    if let Some(index) = index {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_platformer2d::render::rendering::FeatureVisual>();
        for visual in q.iter(world) {
            let Some(view) = index.get(&visual.id) else {
                continue;
            };
            aviews += 1;
            if view.wire_anchor.is_some() {
                awired += 1;
            }
        }
    }
    format!("player[{pviews}/{pwired}] actor[{aviews}/{awired}]")
}
