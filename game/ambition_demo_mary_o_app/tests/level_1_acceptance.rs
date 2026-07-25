//! **The level-1 acceptance run.** She plays it. Nothing is placed.
//!
//! The scripted SEAM run (`scripted_level_run.rs`) proves the seams connect, but
//! sets her position at three beats — the pipe, the vault exit, and the pole —
//! which between them skip every pit, the pipe wall, the bricks, all three
//! ?-blocks and the stair pyramid. So nothing proved the level was traversable,
//! and the only pickups it took were coins, which go through the shared ECONOMY
//! rather than the equipment path.
//!
//! This is that proof. One state-aware controller (read the body, choose this
//! frame's input) drives the whole thing with no positional set-up anywhere:
//!
//! spawn -> bonk ?-block 0 -> mount it and take the milk -> cross pit A ->
//! climb the secret pipe -> vault -> bank all 8 coins -> climb the return pipe
//! -> surface -> re-power at ?-block 1 -> cross pits B and C -> up the stair
//! pyramid -> the pole -> tally -> a real replay back to spawn.
//!
//! It finishes with all three lives, which is the point: a run that spends
//! lives has shown the level is survivable, not that it is traversable.
//!
//! ## What writing it found
//!
//! Three bugs, none of which any existing test could see, because every
//! existing proof either set her position past the terrain or asserted a value
//! the emitter wrote rather than the effect:
//!
//! - **The vault had no working exit.** The return pipe's BLOCK was derived
//!   from its interact BAND rather than from the vault floor, floating it 48px
//!   clear and leaving its top face above its own band. A body standing on the
//!   pipe spanned 544..592 against a band at 624..656, so Interact could never
//!   fire. The demo's own `the_pipe_leads_into_a_sealed_vault_and_back_out`
//!   stayed green because it checked a body at the band's CENTRE — a point
//!   inside solid rock that no player can occupy — and `scripted_level_run`
//!   stayed green because it teleports her to exactly that point. Fixed, and
//!   that test now stands her on the authored block's real top face.
//!
//! - **A body reset redefined the body.** `reset_body_clusters` hardcoded the
//!   default player size into `base_size`, so a grown Mary-O who fell in a pit
//!   came back with a small collider while still wearing the cap and still
//!   presenting the tall sprite. Fixed in `ambition_engine_core`.
//!
//! - **Pit B was not a pit.** It used to open directly into the secret vault:
//!   the vault's ceiling IS the ground slab, and the vault reached under pit B's
//!   gap in it, so falling in was a soft landing in the secret rather than a
//!   death and jumping under it launched you out. FIXED (`2ac119eed`) by
//!   lengthening the level so the whole vault sits under unbroken ground, and
//!   guarded since by `the_vault_ceiling_is_unbroken_no_pit_opens_a_hole`.

#![cfg(not(feature = "input"))]

use ambition::engine_core::{self as ae, AabbExt};
use ambition::input::ControlFrame;
use ambition::platformer::markers::PrimaryPlayer;
use ambition_demo_mary_o_app::build_demo_app;
use bevy::prelude::*;

/// `mary_o_tall`'s id is private to the demo's `powerups` module; the demo's own
/// `power_loop.rs` hardcodes it the same way.
const TALL_ID: &str = "mary_o_tall";

/// The scripted stick, republished every frame in `PreUpdate` because Bevy runs
/// the fixed-timestep loop BEFORE `Update` — intent written any later is not
/// seen by the tick it was meant to drive.
#[derive(Resource, Clone, Copy, Default)]
struct ScriptedStick(ControlFrame);

fn apply_scripted_stick(stick: Res<ScriptedStick>, mut frame: ResMut<ControlFrame>) {
    *frame = stick.0;
}

/// Everything the controller may look at: where she is, how fast, how big, and
/// whether she has footing. This is the information a player reads off the
/// screen — no privileged access to level state.
#[derive(Clone, Copy, Debug)]
struct Body {
    pos: Vec2,
    size: Vec2,
    on_ground: bool,
}

impl Body {
    fn feet(&self) -> f32 {
        self.pos.y + self.size.y * 0.5
    }
    fn right(&self) -> f32 {
        self.pos.x + self.size.x * 0.5
    }
    fn is_tall(&self) -> bool {
        self.size.y > 60.0
    }
}

fn body(app: &mut App) -> Option<Body> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&ae::BodyKinematics, &ae::BodyGroundState), With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|(kin, ground)| Body {
        pos: kin.pos,
        size: kin.size,
        on_ground: ground.on_ground,
    })
}

fn worn_form(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition::characters::actor::WornCharacter, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|w| w.0.clone())
}

fn wears(app: &mut App, id: &str) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition::characters::equipment::WornEquipment, With<PrimaryPlayer>>();
    q.iter(app.world()).next().is_some_and(|w| w.wears(id))
}

fn health(app: &mut App) -> Option<i32> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition::characters::actor::BodyHealth, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|h| h.current())
}

fn level(app: &mut App) -> (u8, i64, f32) {
    let mut q = app
        .world_mut()
        .query::<&ambition_demo_mary_o::MaryOLevelState>();
    let s = q
        .iter(app.world())
        .next()
        .expect("the level owner exists once the demo is playable");
    (s.lives, s.score as i64, s.time_remaining)
}

// ── Input vocabulary ──────────────────────────────────────────────────────

fn idle() -> ControlFrame {
    ControlFrame::default()
}

/// Move horizontally. `run` sustains the modifier slot — she walks at half
/// throttle without it, and pits B and C are NOT walkable, so the open stretches
/// are run-gated while the precision beats are deliberately walked.
fn move_x(dir: f32, run: bool) -> ControlFrame {
    ControlFrame {
        axis_x: dir,
        right_pressed: dir > 0.0,
        left_pressed: dir < 0.0,
        modifier_held: run,
        ..ControlFrame::default()
    }
}

/// Hold jump through the whole ascent. `RunJump` carries `variable_jump`, so
/// releasing while still rising cuts velocity to 0.54x — a released jump loses
/// 46% of the apex and will not clear pit C.
fn with_jump(mut f: ControlFrame) -> ControlFrame {
    f.jump_pressed = true;
    f.jump_held = true;
    f
}

/// Which beat of the flag sequence is live. Mirrored to a local enum so the
/// test does not need the demo's payload fields.
#[derive(Debug, PartialEq)]
enum FlagPhaseKind {
    Idle,
    Running,
    Tallied,
}

fn flag_phase(app: &mut App) -> Option<FlagPhaseKind> {
    use ambition_demo_mary_o::flag::FlagPhase;
    let mut q = app
        .world_mut()
        .query::<&ambition_demo_mary_o::flag::FlagSequence>();
    q.iter(app.world()).next().map(|s| match s.phase {
        FlagPhase::Idle => FlagPhaseKind::Idle,
        FlagPhase::Tallied { .. } => FlagPhaseKind::Tallied,
        _ => FlagPhaseKind::Running,
    })
}

/// Press INTO a pipe. A warp is DIRECTIONAL (Jon bug #8) and its two ends need
/// opposite presses: `+y` is screen-down, so DOWN takes the surface pipe and UP
/// takes the ceiling pipe at the vault's far end. Interact does nothing at
/// either — which is the whole point of the directional verb.
fn press_into_pipe(down: bool) -> ControlFrame {
    ControlFrame {
        axis_y: if down { 1.0 } else { -1.0 },
        ..ControlFrame::default()
    }
}

/// Her banked coin balance, read from the same `PlayerHudFacts` the HUD's COINS
/// readout draws — so this covers placement all the way to the screen.
fn wallet(app: &mut App) -> i32 {
    app.world()
        .resource::<ambition::sim_view::PlayerHudFacts>()
        .balance
}

/// Run right until `target_x`, clearing any pit that comes up on the way and
/// hopping anything that stops her.
///
/// The stall rule is what gets her up the stair pyramid: the kernel has no
/// auto-step, so a 32px riser is a wall until she jumps it. Rather than encode
/// the pyramid's geometry, notice that she stopped making progress and jump —
/// which is what a player does, and it costs nothing on open ground.
fn run_right_to(app: &mut App, target_x: f32, pits: &[(f32, f32)], budget: usize) -> bool {
    let mut last_x = f32::MIN;
    let mut stalled = 0u32;
    drive(app, budget, |b| {
        if b.pos.x >= target_x {
            return None;
        }
        if b.pos.x - last_x < 0.5 {
            stalled += 1;
        } else {
            stalled = 0;
        }
        last_x = b.pos.x;
        if stalled > 4 && b.on_ground {
            stalled = 0;
            return Some(with_jump(move_x(1.0, true)));
        }
        Some(approach_and_clear(b, pits))
    })
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ScriptedStick>().0 = frame;
    app.update();
}

// ── Level landmarks, read from the authored room ──────────────────────────
//
// `T`, the column tables and the surface height are private to the demo, so the
// spans come from the room's own named blocks. If the level is re-authored this
// run follows it rather than walking into a wall.

fn block(name: &str) -> ae::Aabb {
    let room = ambition_demo_mary_o::level_1_1();
    room.world
        .blocks
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("level 1-1 authors a block named {name}"))
        .aabb
}

/// The three bottomless gaps, as (left lip, right lip), derived from the ground
/// slabs rather than restated.
fn pits() -> Vec<(f32, f32)> {
    let slabs: Vec<ae::Aabb> = [
        "ground_open_teach",
        "ground_after_pit_a",
        "ground_after_pit_b",
        "ground_after_pit_c",
    ]
    .iter()
    .map(|n| block(n))
    .collect();
    slabs.windows(2).map(|w| (w[0].max.x, w[1].min.x)).collect()
}

/// Land on a narrow ledge from beside it: jump, then feed rightward input ONLY
/// while airborne above the ledge's top face and still short of the target x.
/// A running jump covers 260px and would sail straight over a 32px block.
fn mount(b: Body, target_x: f32, ledge_top: f32) -> ControlFrame {
    if b.on_ground {
        return with_jump(move_x(0.0, false));
    }
    // Clear of the ledge's face and not yet over the target: drift.
    if b.feet() < ledge_top - 2.0 && b.pos.x < target_x {
        with_jump(move_x(1.0, false))
    } else {
        with_jump(move_x(0.0, false))
    }
}

/// Run right, and commit to a jump when the next pit's lip is close enough that
/// a running arc clears it. `extra_clearance` pulls the launch earlier for pit C,
/// whose left lip carries a ?-block 96px overhead that a lip-edge launch
/// head-bonks — killing the arc and dropping her in.
fn approach_and_clear(b: Body, pits: &[(f32, f32)]) -> ControlFrame {
    for &(left, right) in pits {
        if b.right() > right {
            continue; // already across
        }
        let width = right - left;
        // Pit C is the only one whose lip is roofed; launch a body-length early.
        let overhead = width > 150.0;
        let launch_at = if overhead { left - 60.0 } else { left - 18.0 };
        if b.pos.x >= launch_at && b.on_ground {
            return with_jump(move_x(1.0, true));
        }
        if !b.on_ground {
            return with_jump(move_x(1.0, true));
        }
        break;
    }
    move_x(1.0, true)
}

fn drive(
    app: &mut App,
    frames: usize,
    mut choose: impl FnMut(Body) -> Option<ControlFrame>,
) -> bool {
    for _ in 0..frames {
        let Some(b) = body(app) else {
            app.update();
            continue;
        };
        match choose(b) {
            Some(frame) => step(app, frame),
            None => return true,
        }
    }
    false
}

fn settle_until_playable(app: &mut App) {
    for _ in 0..600 {
        app.update();
        if let Some(b) = body(app) {
            if b.on_ground {
                return;
            }
        }
    }
    panic!("the demo never activated a playable body on the ground");
}

fn boot() -> App {
    let mut app = build_demo_app();
    // A fixed-tick host without a pinned clock runs a machine-speed-dependent
    // number of ticks per update; the same script would then cover a different
    // distance every run.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_resource::<ScriptedStick>();
    app.add_systems(PreUpdate, apply_scripted_stick);
    app
}

/// The crate-level `cfg(not(feature = "input"))` is NOT sufficient: it reads
/// THIS crate's flag, while the thing that erases a scripted write is
/// `ambition/input` in the dependency. Under `cargo test --workspace` cargo
/// unifies features across the graph, so `ambition` builds WITH `input` while
/// this crate's flag stays off. Ask the composition, not the feature flag.
fn scripted_input_reaches_the_sim(app: &mut App) -> bool {
    step(app, move_x(1.0, true));
    app.world().resource::<ControlFrame>().axis_x > 0.5
}

#[test]
fn she_plays_level_one_from_spawn_to_the_pole_and_it_replays() {
    let mut app = boot();
    settle_until_playable(&mut app);

    if !scripted_input_reaches_the_sim(&mut app) {
        eprintln!(
            "SKIP: a participant pipeline owns `ControlFrame` in this build \
             (`ambition/input` is on, likely via workspace feature unification), \
             so scripted input never reaches the sim."
        );
        return;
    }

    let pits = pits();
    let start = body(&mut app).expect("she is in the world");
    let spawn = start.pos;
    let hp_at_spawn = health(&mut app).expect("she has a health pool");
    eprintln!("spawn {spawn:?} size {:?} pits {pits:?}", start.size);

    // ── 1. The ?-block: bonk it, then land on it to take the milk ──────────
    //
    // The reward pops out RESTING ON the block's top face, so collecting it is
    // a second, separate platforming act — she has to get up there.
    let block0 = block("power_block_0");
    let bonk_x = block0.center().x;
    let took_off = drive(&mut app, 400, |b| {
        if b.pos.x < bonk_x - 4.0 {
            return Some(move_x(1.0, false));
        }
        if b.on_ground {
            return Some(with_jump(move_x(0.0, false)));
        }
        Some(with_jump(move_x(0.0, false)))
    });
    assert!(!took_off, "the bonk beat is time-boxed, not terminal");

    eprintln!("after bonk: {:?}", body(&mut app));

    // Back off left so she can mount the block from beside it.
    drive(&mut app, 60, |b| {
        Some(move_x(
            if b.pos.x > block0.min.x - 40.0 {
                -1.0
            } else {
                0.0
            },
            false,
        ))
    });

    let got_cap = drive(&mut app, 300, |b| {
        if b.is_tall() {
            return None;
        }
        Some(mount(b, block0.center().x, block0.min.y))
    });
    eprintln!("after mount attempt: tall={} {:?}", got_cap, body(&mut app));

    assert!(
        got_cap,
        "she must take the milk off the ?-block through the real pickup path; \
         worn form is {:?}, wears grow_cap = {}",
        worn_form(&mut app),
        wears(&mut app, "grow_cap")
    );
    assert!(
        wears(&mut app, "grow_cap"),
        "the milk equips through the shared equipment path"
    );
    assert_eq!(
        worn_form(&mut app).as_deref(),
        Some(TALL_ID),
        "and the worn form follows the equipment"
    );

    eprintln!("POWERED UP at {:?}", body(&mut app));

    // ── 2. Cross pit A and take the secret pipe ───────────────────────────
    //
    // The first terrain nothing had ever proved she could cross. Pit A is 64px
    // wide; a running arc covers ~260px, so the margin is generous — but the
    // launch still has to be committed from the ledge rather than set up.
    let pipe = block("secret_pipe");
    let reached_pipe = run_right_to(&mut app, pipe.min.x - 20.0, &pits, 900);
    assert!(
        reached_pipe,
        "she must reach the secret pipe across pit A under her own input; she \
         stalled at {:?}",
        body(&mut app)
    );
    eprintln!("AT PIPE {:?}", body(&mut app));

    // The pipe is a 64px wall in her path with a 64px-wide top face. A running
    // jump sails clean over it, so this is a walked mount: rise against the
    // face, then feed rightward input only while above the lip.
    let on_pipe = drive(&mut app, 240, |b| {
        if b.on_ground && b.feet() <= pipe.min.y + 2.0 && b.pos.x > pipe.min.x {
            return None;
        }
        Some(mount(b, pipe.center().x, pipe.min.y))
    });
    assert!(
        on_pipe,
        "she must climb onto the pipe to reach its mouth; she is at {:?}",
        body(&mut app)
    );
    eprintln!("ON PIPE {:?}", body(&mut app));

    let vault = ambition_demo_mary_o::vault_bounds();
    let hp_entering_vault = health(&mut app).expect("she has a health pool");
    let tall_entering_vault = body(&mut app).expect("she is in the world").is_tall();
    // DOWN on the mouth starts the transit — a half-second slide in and out, not
    // a teleport — so this drives until she is actually through it.
    let dropped_in = drive(&mut app, 240, |b| {
        if b.pos.y > vault.min.y + 3.0 * 32.0 {
            return None;
        }
        Some(press_into_pipe(true))
    });
    assert!(
        dropped_in,
        "pressing DOWN on the pipe mouth slides her into the vault; she is at {:?}",
        body(&mut app)
    );
    eprintln!("IN VAULT {:?}", body(&mut app));

    // ── 3. Bank the vault and surface through the return pipe ─────────────
    //
    // A plain walk along the vault floor collects the coin row on the way. The
    // vault is sealed now (see the module note), so there is nothing to avoid.
    let coins_before = wallet(&mut app);
    // The return pipe HANGS FROM THE CEILING now — a tube up through the ground
    // slab, not a stump on the floor. So she does not climb it: she walks the
    // vault floor until she is UNDER it, which is where the exit mouth is.
    let return_pipe = block("vault_return_pipe");
    let at_exit = drive(&mut app, 900, |b| {
        let under_pipe = b.pos.x > return_pipe.min.x + 4.0 && b.pos.x < return_pipe.max.x - 4.0;
        if under_pipe && b.on_ground {
            return None;
        }
        Some(move_x(1.0, false))
    });
    assert!(
        at_exit,
        "she must walk the vault floor to stand under its return pipe; she is at {:?}",
        body(&mut app)
    );
    let coins_after = wallet(&mut app);
    assert!(
        coins_after > coins_before,
        "walking the vault banks its coins through the shared economy \
         ({coins_before} -> {coins_after}) — no demo code collects them by hand"
    );
    eprintln!(
        "VAULT BANKED {coins_before}->{coins_after} at {:?}",
        body(&mut app)
    );

    let surfaced = drive(&mut app, 240, |b| {
        if b.pos.y < vault.min.y {
            return None;
        }
        Some(press_into_pipe(false))
    });
    assert!(
        surfaced,
        "pressing UP under the return pipe surfaces her; she is at {:?}",
        body(&mut app)
    );
    eprintln!("SURFACED {:?}", body(&mut app));

    // ── The milk's actual EFFECT ──────────────────────────────────────────
    //
    // `grow_cap` grants no verb; its whole effect is `OnHit::ConsumeAsArmor`.
    // So the way to exercise it is to take a hit and survive one that would
    // otherwise have cost a life. CONDITIONAL: it fires only if something
    // actually hit her on this run, which is why the assertions sit behind the
    // tall→small check rather than being asserted outright.
    let small_now = !body(&mut app).expect("she is in the world").is_tall();
    if tall_entering_vault && small_now {
        assert!(
            !wears(&mut app, "grow_cap"),
            "the armor is consumed off the worn set, not merely visually"
        );
        assert_eq!(
            worn_form(&mut app).as_deref(),
            Some("mary_o"),
            "and she reverts to the small form"
        );
        assert_eq!(
            health(&mut app),
            Some(hp_entering_vault),
            "the cap ABSORBED the hit — armor that still costs health is not armor"
        );
        eprintln!("ARMOR ABSORBED a hit in the vault, hp still {hp_entering_vault}");
    }

    // ── 4. Re-power at the second ?-block ─────────────────────────────────
    //
    // The ladder again, from the other end: a SMALL Mary-O bonking a fresh
    // ?-block gets the milk, which is what makes the power state a function of
    // her equipment rather than a one-shot flag.
    let block1 = block("power_block_1");
    drive(&mut app, 400, |b| {
        if b.pos.x >= block1.center().x - 4.0 && b.on_ground {
            return None;
        }
        Some(move_x(1.0, false))
    });
    drive(&mut app, 90, |b| {
        Some(mount(b, block1.center().x, block1.min.y))
    });
    drive(&mut app, 60, |b| {
        Some(move_x(
            if b.pos.x > block1.min.x - 40.0 {
                -1.0
            } else {
                0.0
            },
            false,
        ))
    });
    let repowered = drive(&mut app, 300, |b| {
        if b.is_tall() {
            return None;
        }
        Some(mount(b, block1.center().x, block1.min.y))
    });
    assert!(
        repowered,
        "a SMALL Mary-O bonking a fresh ?-block gets the milk again — the power \
         state is a function of her equipment, not a one-shot flag. She is {:?}",
        body(&mut app)
    );
    assert!(
        wears(&mut app, "grow_cap"),
        "and the second milk equips through the same shared path as the first"
    );
    eprintln!("REPOWERED at {:?}", body(&mut app));

    // ── 5. Pit C, the pyramid, and the pole ───────────────────────────────
    let pole = ambition_demo_mary_o::goal_pole();
    let reached_pole = run_right_to(&mut app, pole.x - 8.0, &pits, 2400);
    eprintln!(
        "AT POLE reached={reached_pole} {:?} lives={}",
        body(&mut app),
        level(&mut app).0
    );
    assert!(
        reached_pole,
        "she must cross pit C and the stair pyramid to the pole under her own \
         input; she stalled at {:?}",
        body(&mut app)
    );

    // ── 6. The flag, the tally, and a real replay ─────────────────────────
    //
    // Everything up to here was reachability. This is the level ENDING: run
    // into the pole, ride the sequence through its slide/walk-off/tally, and
    // then wait out `LEVEL_CYCLE_DWELL` for the cyclic restart.
    let (_, score_before, _) = level(&mut app);
    let mut tallied = false;
    for _ in 0..900 {
        if flag_phase(&mut app) == Some(FlagPhaseKind::Tallied) {
            tallied = true;
            break;
        }
        step(&mut app, move_x(1.0, false));
    }
    assert!(
        tallied,
        "touching the pole must run the flag sequence through to a settled \
         tally; phase is {:?} and she is at {:?}",
        flag_phase(&mut app),
        body(&mut app)
    );
    eprintln!("TALLIED at {:?}", body(&mut app));

    // The replay clause. Before 2026-07-21 this binary drained
    // `RoomReplayRequested` with nothing, so no run could have asserted it
    // however it was written (tracks 2.5).
    let away_from_spawn = body(&mut app).expect("she is in the world").pos;
    assert!(
        away_from_spawn.distance(spawn) > 1000.0,
        "she finishes the level far from where she started, which is what makes \
         the replay observable ({away_from_spawn:?} vs {spawn:?})"
    );

    let dwell_frames = (ambition_demo_mary_o::LEVEL_CYCLE_DWELL / (1.0 / 60.0)).ceil() as usize;
    let replayed = drive(&mut app, dwell_frames + 240, |b| {
        if b.pos.distance(spawn) < 64.0 {
            return None;
        }
        Some(idle())
    });
    assert!(
        replayed,
        "past the tally dwell the level must actually replay and put her back at \
         spawn; she is at {:?}, spawn is {spawn:?}",
        body(&mut app)
    );

    let (lives_end, score_end, clock_end) = level(&mut app);
    assert!(
        score_end > score_before,
        "the pole grab banks its score before the level cycles \
         ({score_before} -> {score_end})"
    );
    assert!(
        (ambition_demo_mary_o::STARTING_TIME - clock_end) < 5.0,
        "and the fresh lap gets a fresh clock (got {clock_end})"
    );
    assert_eq!(
        lives_end, 3,
        "she completed level 1-1 without dying once — a run that spends lives \
         has not proved the level is traversable, only survivable"
    );
    assert!(
        health(&mut app).is_some_and(|hp| hp > 0),
        "and finishes alive"
    );
    let _ = hp_at_spawn;
    eprintln!(
        "REPLAYED to {:?} score {score_before}->{score_end} clock {clock_end:.0} lives {lives_end}",
        body(&mut app)
    );
}

/// One named Solid Snake's box and phase. Keyed by id, NOT "the first one the
/// query yields" — that is hash order, so it names a different snake from frame
/// to frame and can watch the wrong body entirely.
fn snake_by_id(
    app: &mut App,
    id: &str,
) -> Option<(ae::Aabb, ambition_demo_mary_o::snake::SnakeShell)> {
    let mut q = app.world_mut().query::<(
        &ambition::actors::features::FeatureId,
        &ambition::actors::features::CenteredAabb,
        &ambition_demo_mary_o::snake::SnakeShell,
    )>();
    q.iter(app.world())
        .find(|(feature_id, ..)| feature_id.as_str() == id)
        .map(|(_, aabb, shell)| (aabb.aabb(), *shell))
}

/// **Landing on a Solid Snake is a STOMP, never a hit.**
///
/// Regression proof for a scheduling bug, so it has to run the real schedule:
/// the demo's shell rule and the engine's shared body-contact pass both look at
/// the same contact, and they must look at it at the same MOMENT. The shell rule
/// classifies from body positions; the contact pass is documented to read
/// post-movement positions. Order the shell rule before the movement phase and
/// the two disagree by one frame of falling — on the landing frame the contact
/// pass sees the overlap while the shell rule still saw her in the air, so the
/// snake stays armed and landing on it hurts.
///
/// No scripted input is needed (and so no skip): she is dropped, and gravity is
/// the whole experiment.
#[test]
fn landing_on_a_snake_stomps_it_instead_of_hurting_her() {
    let mut app = boot();
    settle_until_playable(&mut app);

    let id = "mary_o_snake_0";
    let (snake, phase) = snake_by_id(&mut app, id).expect("level 1-1 stages Solid Snakes");
    assert_eq!(
        phase,
        ambition_demo_mary_o::snake::SnakeShell::Walking,
        "the snake starts as a live walker"
    );
    let hp_before = health(&mut app).expect("she has a health pool");

    // Put her directly above it, high enough to be in free fall by the time she
    // arrives — a stationary body resting on the head would prove nothing about
    // the frame the contact first appears.
    {
        let mut kin = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>()
            .single_mut(app.world_mut())
            .expect("the controlled body");
        kin.pos = ae::Vec2::new(snake.center().x, snake.min.y - 160.0);
        kin.vel = ae::Vec2::ZERO;
    }

    let mut stomped = false;
    for _ in 0..240 {
        step(&mut app, idle());
        assert_eq!(
            health(&mut app),
            Some(hp_before),
            "she must not take a scratch for landing on a snake"
        );
        if snake_by_id(&mut app, id).is_some_and(|(_, phase)| {
            !matches!(phase, ambition_demo_mary_o::snake::SnakeShell::Walking)
        }) {
            stomped = true;
            break;
        }
    }
    assert!(
        stomped,
        "falling onto a walking snake must put it into its shell"
    );
}

/// **A small Mary-O dies to one hit, and the level restarts.**
///
/// Jon: *"if mary-o is small and she takes damage she should die and use her
/// death animation, then restart the level."*
///
/// Every part of that was unreachable: her body carried the host's twenty-point
/// pool, so a contact hit cost 5% of a life bar; and her `Death` row was bound
/// to a sheet row the generator named `dead` while the runtime looks for
/// `death`, so nothing could have drawn it even at zero HP.
///
/// Driven through the real app: walk her INTO a snake from the side (a side
/// contact is the one thing that hurts) and watch the whole beat.
#[test]
fn a_small_mary_o_dies_to_one_hit_and_the_level_restarts() {
    let mut app = boot();
    settle_until_playable(&mut app);

    assert_eq!(
        health(&mut app),
        Some(1),
        "she authors a one-hit body: armor absorbs, then the next hit is fatal"
    );

    let id = "mary_o_snake_0";
    let (snake, _) = snake_by_id(&mut app, id).expect("level 1-1 stages Solid Snakes");
    let lives_before = level(&mut app).0;

    // Stand her beside it, on the ground, and let it walk into her. A SIDE
    // contact is the hit; from the top it would be a stomp.
    {
        let mut kin = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>()
            .single_mut(app.world_mut())
            .expect("the controlled body");
        kin.pos = ae::Vec2::new(snake.center().x, snake.max.y - kin.size.y * 0.5);
        kin.vel = ae::Vec2::ZERO;
    }

    let mut saw_death_pose = false;
    let mut died = false;
    for _ in 0..600 {
        step(&mut app, idle());
        let dying = app
            .world_mut()
            .query_filtered::<&ambition::actors::actor::BodyAnimFacts, With<PrimaryPlayer>>()
            .single(app.world())
            .map(|anim| anim.death_anim_timer > 0.0)
            .unwrap_or(false);
        saw_death_pose |= dying;
        if level(&mut app).0 < lives_before {
            died = true;
            break;
        }
    }
    assert!(died, "a side contact with no armor left must kill her");
    assert!(
        saw_death_pose,
        "and it must be VISIBLE — the death row plays before the level restarts"
    );

    // The beat holds, and only then does the level come back.
    let restarted = drive(&mut app, 600, |b| {
        if b.pos.distance(ae::Vec2::new(snake.center().x, snake.max.y)) > 200.0 {
            return None;
        }
        Some(idle())
    });
    assert!(
        restarted,
        "past the death beat the level restarts and puts her back at spawn; \
         she is at {:?}",
        body(&mut app)
    );
}
