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

use ambition_platformer2d::actor::MatchSeat;
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
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster(["performer", "performer"]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // ⛔⛔ WAIT FOR THE ROUND, NOT FOR A NUMBER — the opening ceremony's length
    // is a moving target and four fixture families have broken on encoding it.
    let mut live = false;
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 0 && held == 0 {
            live = true;
            break;
        }
    }
    assert!(live, "the opening ceremony never released the cast");

    let (seat0, seat1) = seats(&mut app);

    // ⛔⛔ THE INSTRUMENT PROVES ITSELF FIRST. `door_count` below queries a
    // PRESENTATION component, and a presentation layer that was never installed
    // answers zero for the same reason a missing door does. So say how many
    // body visuals exist: if that is zero too, every presentation number in this
    // run is uninformative rather than a finding.
    println!(
        "[trap_probe] presentation: {} body visuals live (0 means the door \
         numbers below say NOTHING about the door)",
        player_visuals(&mut app)
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

    let start = kin(&app, seat0).0;
    println!(
        "[trap_probe] standing at ({:.1}, {:.1}), steering {}",
        start.x,
        start.y,
        if steer > 0.0 { "RIGHT" } else { "LEFT" }
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
    let rival_hp_before = health(&app, seat1);

    for tick in 0..WATCH_TICKS {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_x: steer,
                ..Default::default()
            },
        );
        app.update();

        let (pos, vel) = kin(&app, seat0);
        let under = matches!(mode(&app, seat0), Some(BodyMode::Submerged));
        let doors = door_count(&mut app);
        let boxes = hitbox_count(&mut app, seat0);
        let playing = playing_move(&app, seat0);

        // ⭐⭐ STAGE THE ONE CASE THE MOVE IS ABOUT. Jon: *"damages whoever is
        // on top or above the trap door when she emerges."* A rival left where
        // the match put him is never above her, so the emergence window can be
        // live and hit nothing and the run reads clean. Park him ON the door for
        // the frames she is coming up.
        //
        // ⛔ PROBE-SIDE PLACEMENT, and it is honest for an instrument: it moves
        // WHO is standing there, not what the move does to them.
        if under_ticks_seen > 150 {
            let hers = kin(&app, seat0).0;
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
                 under={under:<5} doors={doors} boxes={boxes} move={}",
                pos.x,
                pos.y,
                vel.x,
                vel.y,
                playing.unwrap_or_else(|| "-".to_string()),
            );
        }
    }

    let rival_hp_after = health(&app, seat1);
    let end = kin(&app, seat0).0;

    println!("[trap_probe] ── the five stages, measured ──");
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
    println!(
        "[trap_probe] ⇒ compare against the five stages in `performer_moveset.rs`: \
         door opens, she sinks, she STEERS under, the exit door opens, she leaps out \
         into a firework that hits above the door."
    );
}

/// How many bodies the presentation layer has built. The self-check for
/// `door_count`.
fn player_visuals(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world
        .query::<&ambition_platformer2d::platformer::lifecycle::PlayerVisual>();
    q.iter(world).count()
}

fn seats(app: &mut App) -> (Entity, Entity) {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &MatchSeat)>();
    let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
    rows.sort_by_key(|(seat, _)| *seat);
    (
        rows.first().expect("the match seats a first fighter").1,
        rows.get(1).expect("the match seats a second fighter").1,
    )
}

fn kin(app: &App, body: Entity) -> (Vec2, Vec2) {
    let k = app
        .world()
        .get::<BodyKinematics>(body)
        .expect("the fighter still has a body");
    (
        Vec2::new(k.pos.x, k.pos.y),
        Vec2::new(k.vel.x, k.vel.y),
    )
}

fn mode(app: &App, body: Entity) -> Option<BodyMode> {
    app.world()
        .get::<BodyModeState>(body)
        .map(|state| state.body_mode)
}

fn playing_move(app: &App, body: Entity) -> Option<String> {
    app.world()
        .get::<ambition_platformer2d::combat::moveset::MovePlayback>(body)
        .map(|p| format!("{}@{:.2}", p.spec.id, p.t))
}

/// Live trapdoor visuals in the world — the thing that tells an opponent where
/// she is, and the half of Jon's ask that hiding her body does not answer.
fn door_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::render::rendering::submerged::TrapdoorVisual>();
    q.iter(world).count()
}

/// Live hitboxes SHE owns. The emergence is the only one this move authors.
fn hitbox_count(app: &mut App, owner: Entity) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::combat::strike::Hitbox>();
    q.iter(world).filter(|hb| hb.owner == owner).count()
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
