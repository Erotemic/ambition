//! End-to-end Mary-O level-1 acceptance run.
//!
//! A state-aware controller traverses the real level without positional setup,
//! exercises a question-block powerup through the shared pickup/equipment path,
//! visits the pipe secret, banks the vault coins, reaches the goal, completes
//! tally, and verifies the resulting room replay. Direct body observations are
//! limited to information a player could infer from the rendered state.

//! The scripted stick goes through `scripted_input` now, which writes after `InputSet::Route` and
//! is therefore the last writer under a composed participant pipeline as well as the only one
//! without it, so the acceptance run means the same thing in both compositions.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core::{self as ae, AabbExt};
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// `mary_o_tall`'s id is private to the demo's `powerups` module; the demo's own
/// `power_loop.rs` hardcodes it the same way.
const TALL_ID: &str = "mary_o_tall";
/// The top of the ladder: the sheet the cinder beacon puts her in. Private to
/// `powerups` for the same reason, and hardcoded in `power_loop.rs` too.
const FIRE_ID: &str = "mary_o_fire";

/// The scripted stick, republished every frame in `PreUpdate` because Bevy runs
/// the fixed-timestep loop BEFORE `Update` — intent written any later is not
/// seen by the tick it was meant to drive.

/// Everything the controller may look at: where she is, how fast, how big,
/// whether she has footing, and what is coming at her. This is the information a
/// player reads off the screen — no privileged access to level state.
#[derive(Clone, Copy, Debug)]
struct Body {
    pos: Vec2,
    size: Vec2,
    on_ground: bool,
    /// Gap to the nearest live hostile ahead of her, if one is in view.
    ///
    /// She is a ONE-HIT body now, so an enemy in the path is not a tax on a
    /// health bar — it is the run. A controller that cannot see enemies cannot
    /// play this level, and pretending otherwise would make the acceptance run
    /// a proof about a level with nothing in it.
    threat_ahead: Option<f32>,
}

impl Body {
    fn feet(&self) -> f32 {
        self.pos.y + self.size.y * 0.5
    }
    /// The top of her head.  derived from her SIZE, never from the small
    /// form's half-height: a grown Mary-O's head is 8px higher than a small
    /// one's, and a bonk beat that steers off a hardcoded `pos.y - 24` keeps the
    /// jump button down for 8px after her head has already reached the block.
    fn head(&self) -> f32 {
        self.pos.y - self.size.y * 0.5
    }
    fn right(&self) -> f32 {
        self.pos.x + self.size.x * 0.5
    }
    fn is_tall(&self) -> bool {
        self.size.y > 60.0
    }
    /// Leave enough gap for input-to-rise latency plus both bodies' closing speed.
    fn should_stomp(&self) -> bool {
        self.on_ground && self.threat_ahead.is_some_and(|gap| gap < STOMP_REACH_PX)
    }
}

/// How far ahead a threat has to be before the chase commits to stomping it.
const STOMP_REACH_PX: f32 = 176.0;

/// Where every collectible in the room IS, which is the question the wand row
/// needs answered before anybody touches the pickup path.
///
/// The mount loop below patrols a stretch and waits for the wand to come to her.
fn world_items(app: &mut App) -> Vec<(String, ae::Vec2)> {
    let mut query = app
        .world_mut()
        .query::<&ambition_platformer2d::actors::items::WorldItem>();
    let world = app.world();
    query
        .iter(world)
        .map(|item| {
            (
                item.sprite.clone().unwrap_or_else(|| "?".to_string()),
                item.pos,
            )
        })
        .collect()
}

fn body(app: &mut App) -> Option<Body> {
    let mut hostiles = app.world_mut().query_filtered::<(
        &ambition_platformer2d::actors::features::CenteredAabb,
        &ambition_platformer2d::characters::actor::BodyHealth,
    ), (
        With<ambition_platformer2d::actors::features::ActorDisposition>,
        Without<PrimaryPlayer>,
    )>();
    let threats: Vec<ae::Aabb> = hostiles
        .iter(app.world())
        .filter(|(_, health)| health.alive())
        .map(|(aabb, _)| aabb.aabb())
        .collect();
    let mut q = app
        .world_mut()
        .query_filtered::<(&ae::BodyKinematics, &ae::BodyGroundState), With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|(kin, ground)| {
        let right = kin.pos.x + kin.size.x * 0.5;
        let feet = kin.pos.y + kin.size.y * 0.5;
        let threat_ahead = threats
            .iter()
            .filter(|t| t.max.x > right && (t.max.y - feet).abs() < 96.0)
            .map(|t| t.min.x - right)
            .filter(|gap| *gap < 220.0)
            .fold(None, |best: Option<f32>, gap| {
                Some(best.map_or(gap, |b| b.min(gap)))
            });
        Body {
            pos: kin.pos,
            size: kin.size,
            on_ground: ground.on_ground,
            threat_ahead,
        }
    })
}

fn worn_form(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::WornCharacter, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|w| w.id().to_string())
}

fn wears(app: &mut App, id: &str) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::equipment::WornEquipment, With<PrimaryPlayer>>();
    q.iter(app.world()).next().is_some_and(|w| w.wears(id))
}

fn health(app: &mut App) -> Option<i32> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::BodyHealth, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|h| h.current())
}

fn level(app: &mut App) -> (i8, i64, f32) {
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

fn press_into_pipe(down: bool) -> ControlFrame {
    ControlFrame {
        axis_y: if down { 1.0 } else { -1.0 },
        ..ControlFrame::default()
    }
}

/// Press into the pipe while STAYING on its mouth.
///
/// She arrives on the lip still carrying the momentum that got her up there, and
/// her classic friction coasts her clean off the far edge before the transit
/// arms. A player holds back against that; so does this. The horizontal input is
/// a correction toward the mouth, never a run.
fn press_into_pipe_at(b: Body, mouth_x: f32, down: bool) -> ControlFrame {
    ControlFrame {
        axis_x: ((mouth_x - b.pos.x) / 16.0).clamp(-1.0, 1.0),
        ..press_into_pipe(down)
    }
}

/// Her banked coin balance, read from the same `PlayerHudFacts` the HUD's COINS
/// readout draws — so this covers placement all the way to the screen.
fn wallet(app: &mut App) -> i32 {
    app.world()
        .resource::<ambition_platformer2d::sim_view::PlayerHudFacts>()
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
        // An enemy in the path is dealt with the only way a one-hit body can:
        // from ABOVE. Walking into it is the end of the attempt.
        if b.should_stomp() {
            return Some(with_jump(move_x(1.0, false)));
        }
        Some(approach_and_clear(b, pits))
    })
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

// ── Level landmarks, read from the authored room ──────────────────────────
//
// `T`, the column tables and the surface height are private to the demo, so the
// spans come from the room's own named blocks. If the level is re-authored this
// run follows it rather than walking into a wall.

fn block(name: &str) -> ae::Aabb {
    block_of(name).aabb
}

/// The whole authored block, so a probe can ask about its IDENTITY and not only
/// its rectangle — "was this one struck" is a question about its `GeoId`.
/// The first ?-block the LEVEL authors.
fn first_power_block() -> ae::world::Block {
    let room = ambition_demo_mary_o::level_1_1();
    room.world
        .blocks
        .iter()
        .find(|b| {
            ambition_demo_mary_o::ldtk_vocabulary::block_look_of(&b.name)
                == Some(ambition_demo_mary_o::ldtk_vocabulary::MaryOBlockLook::Question)
        })
        .expect("level 1-1 authors a ?-block")
        .clone()
}

fn block_of(name: &str) -> ae::world::Block {
    let room = ambition_demo_mary_o::level_1_1();
    room.world
        .blocks
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("level 1-1 authors a block named {name}"))
        .clone()
}

/// The three bottomless gaps, as (left lip, right lip), derived from the ground
/// slabs rather than restated.
fn pits() -> Vec<(f32, f32)> {
    //  the ground runs are painted into an IntGrid and carry no authored
    // name. This named `ground_open_teach` and its three siblings — how the
    // level was BUILT. A pit is a gap in the slab she walks on, which is what it
    // always meant and what survives repainting.
    //
    // The ground run is by far the widest block in the level, and a point inside
    // it is unambiguous — no falling, no stacking, nothing to infer.
    let room = ambition_demo_mary_o::level_1_1();
    let widest = room
        .world
        .blocks
        .iter()
        .filter(|b| !matches!(b.kind, ae::world::BlockKind::Hazard))
        .max_by(|a, b| {
            (a.aabb.max.x - a.aabb.min.x)
                .partial_cmp(&(b.aabb.max.x - b.aabb.min.x))
                .expect("finite")
        })
        .expect("the level has ground");
    let row = (widest.aabb.min.y + widest.aabb.max.y) * 0.5;
    //  exact spans, not samples. A 32px sampler reports the first EMPTY
    // sample as the lip, so every pit's left edge came back 32px late — and this
    // list is what the scripted run uses to time its jumps, so she jumped late
    // and fell in. Merge the solid spans at `row` and the gaps between them are
    // the pits, to the pixel.
    let mut spans: Vec<(f32, f32)> = room
        .world
        .blocks
        .iter()
        .filter(|b| {
            !matches!(b.kind, ae::world::BlockKind::Hazard)
                && b.aabb.min.y <= row
                && b.aabb.max.y >= row
        })
        .map(|b| (b.aabb.min.x, b.aabb.max.x))
        .collect();
    spans.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite"));
    let mut merged: Vec<(f32, f32)> = Vec::new();
    for (lo, hi) in spans {
        match merged.last_mut() {
            Some(last) if lo <= last.1 + 0.5 => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    let pits: Vec<(f32, f32)> = merged.windows(2).map(|w| (w[0].1, w[1].0)).collect();
    pits
}

/// Land on a narrow ledge from beside it: jump, then feed rightward input ONLY
/// while airborne above the ledge's top face and still short of the target x.
/// A running jump covers 260px and would sail straight over a 32px block.
fn mount(b: Body, target_x: f32, ledge_top: f32) -> ControlFrame {
    if b.on_ground {
        return with_jump(move_x(0.0, false));
    }
    if b.feet() < ledge_top - 2.0 {
        with_jump(move_x(
            ((target_x - b.pos.x) / 24.0).clamp(-1.0, 1.0),
            false,
        ))
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
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    app
}

/// Assert that scripted input survives the composed participant pipeline.
/// Probe `ControlFrame` immediately after `Update`: the scripted delivery counter
/// observes the slot table from `FixedUpdate` and can lag the first press. The
/// scripted writer is ordered after normal input routing so a composed input
/// feature cannot erase the test press.
#[track_caller]
fn assert_scripted_input_reaches_the_sim(app: &mut App) {
    let probe = ControlFrame {
        aim_x: 1.0,
        ..ControlFrame::default()
    };
    for _ in 0..20 {
        step(app, probe);
        if app.world().resource::<ControlFrame>().aim_x > 0.5 {
            step(app, ControlFrame::default());
            return;
        }
    }
    panic!(
        "a scripted press did not survive into the simulation, so every assertion \
         after this point would pass on a body nobody was driving"
    );
}

///  what it uniquely covered is real and is NOT covered elsewhere: a whole
/// playthrough, spawn to flagpole, on the production schedule. The mechanics it
/// touches are covered against the authored level by unit probes (the bonk, the
/// stomp, the brick break, the warp), and 1-1's SHAPE is covered by invariants
/// (`the pit rhythm must widen`, `every authored enemy has ground under it`).
/// The gap is the end-to-end run, and it stays a gap until the fixture lands.
///
/// Queue row `G1 PICK 11`.
#[ignore = "route tuned to 1-1's old arrangement; replaced by a fixture course (queue G1 PICK 11)"]
#[test]
fn she_plays_level_one_from_spawn_to_the_pole_and_it_replays() {
    let mut app = boot();
    settle_until_playable(&mut app);

    assert_scripted_input_reaches_the_sim(&mut app);

    let pits = pits();
    let start = body(&mut app).expect("she is in the world");
    let spawn = start.pos;
    let hp_at_spawn = health(&mut app).expect("she has a health pool");
    eprintln!("spawn {spawn:?} size {:?} pits {pits:?}", start.size);

    // ── 1. The ?-block: bonk it, then land on it to take the wand ──────────
    //
    // The reward pops out RESTING ON the block's top face, so collecting it is
    // a second, separate platforming act — she has to get up there.
    let block0 = first_power_block().aabb;
    /// Stay this side of the first pit while chasing the wand — the pit opens at
    /// x = 640 and walking into it is not what this step is testing.
    const FIRST_PIT_SAFE_X: f32 = 560.0;
    // Three things this beat has to respect, all of which a player learns in the
    // first ten seconds of the real game:
    //
    // * She is CLASSIC-slippery. Her ground friction is the faithful conversion,
    //   so letting go of the stick coasts a long way — run at the block and
    //   release and she sails past it. Steer toward the column the whole time,
    //   airborne included.
    // * Jump is an EDGE. Holding it from the approach spends the one press early
    //   and then she walks under the block with the button already down.
    // * There is a Solid Snake just past the block. With no armor yet, touching
    //   its side ends the run, so stomp it when it closes.
    let bonk_x = block0.center().x;
    // How high does she actually get? The bonk beat asserts only that it
    // did not end early, so "she jumped and missed" and "she never jumped" look
    // identical from outside.
    let mut apex_y = f32::MAX;
    let mut head_under_block = f32::MAX;
    let mut frames_under = 0usize;
    let took_off = drive(&mut app, 400, |b| {
        apex_y = apex_y.min(b.pos.y);
        //  apex alone is NOT a jump-height measurement — over 400 frames she
        // can jump from elevated ground. The question is whether her HEAD ever
        // reached the block's underside WHILE HORIZONTALLY UNDER IT.
        {
            let bmin = first_power_block().aabb.min;
            let under = b.pos.x + 12.8 > bmin.x && b.pos.x - 12.8 < bmin.x + 32.0;
            if under {
                frames_under += 1;
                head_under_block = head_under_block.min(b.pos.y - 24.0);
            }
        }
        let toward = (bonk_x - b.pos.x).clamp(-1.0, 1.0);
        let under_it = (b.pos.x - bonk_x).abs() < 8.0;
        // The snake comes FIRST. She is a one-hit body here — no wand yet — so a
        // live walker in the same stretch is the run, and the block will still be
        // there once it is a shell. Jump for it with room to spare: the arc has
        // to land on its head, and a stomp attempted at touching distance is a
        // side contact.
        if b.should_stomp() {
            return Some(with_jump(move_x(1.0, false)));
        }
        // Standing under the block: go STRAIGHT up. Steering while airborne is
        // what carries her over it instead of into its underside.
        if b.on_ground && under_it {
            return Some(with_jump(move_x(0.0, false)));
        }
        if b.on_ground {
            return Some(move_x(toward, false));
        }
        // RELEASE while airborne — a bonk is a SHORT HOP. Holding jump here is what made this
        // beat unable to bonk anything: her classic jump is `jump_speed 450` under `gravity 2250`
        // with `held_rise_gravity_scale 0.2`, so a HELD press rises ~145 px while the ?-block's
        // underside is only 48 px above her head. Tapping is how the game this converges on hits a
        // row-4 block, and the variable jump is what makes the difference expressible at all.
        //
        //  hold only until her HEAD reaches the underside, then release. A
        // bare tap rises 32.6 px and the underside is 48 px up — 15 px short — so
        // "tap" and "hold" are both wrong and the right answer is the one the
        // variable jump exists to express. Steering off the measurement rather
        // than off a frame count means this stays correct if her tuning or the
        // block row moves, which a hand-tuned "hold for 4 frames" would not.
        let underside = first_power_block().aabb.max.y;
        let head_below_underside = b.pos.y - 24.0 > underside;
        if head_below_underside {
            return Some(with_jump(move_x(0.0, false)));
        }
        Some(move_x(0.0, false))
    });
    assert!(!took_off, "the bonk beat is time-boxed, not terminal");

    // `SpentPowerBlocks` is the game's own record that a block gave up its reward.
    assert!(
        app.world()
            .get_resource::<ambition_demo_mary_o::powerups::SpentPowerBlocks>()
            .is_some_and(|spent| spent.is_spent(&first_power_block().id)),
        "she never struck the first ?-block, so no reward was ever spawned — the \
         wand she is later asked to wear was never made. Her head reached \
         {head_under_block:.1} while under it, and the underside is at {:.1}",
        first_power_block().aabb.max.y,
    );

    {
        {
            let mut q = app
                .world_mut()
                .query_filtered::<&ambition_platformer2d::actor::MotionModel, With<PrimaryPlayer>>(
                );
            let model = q.iter(app.world()).next().map(|m| format!("{m:?}"));
            let shown: String = model.unwrap_or_else(|| "<none>".to_string());
            eprintln!("motion model: {}", &shown[..shown.len().min(600)]);
        }
        let block_underside = first_power_block().aabb.max.y;
        let needed_centre = block_underside + 48.0 * 0.5;
        eprintln!(
            "after bonk: apex_y={apex_y:.1} (lower is higher); block underside y={block_underside:.1}; \
             she needs centre y<={needed_centre:.1} — short by {:.1}px",
            apex_y - needed_centre
        );
        eprintln!(
            "under the block for {frames_under} frame(s); best HEAD y there = {head_under_block:.1} \
             (needs <= {block_underside:.1})"
        );
    }
    eprintln!("after bonk: {:?}", body(&mut app));
    eprintln!("after bonk, items: {:?}", world_items(&mut app));
    eprintln!(
        "after bonk, block0 SPENT = {:?}",
        app.world()
            .get_resource::<ambition_demo_mary_o::powerups::SpentPowerBlocks>()
            .map(|spent| spent.is_spent(&first_power_block().id))
    );

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

    // WALK it down. The wand does not wait on the block any more — it rises out and travels,
    // turning at walls, so mounting the block reaches an empty roof. She patrols the safe stretch
    // between the block and the first pit until it comes to her; she is faster than it, so this
    // always converges. Chunked so the WAND's travel is visible, not only hers.
    let mut got_cap = false;
    for chunk in 0..6 {
        // Measured: it leaves the block at x=483 and runs at ~55 px/s while she stood on
        // `FIRST_PIT_SAFE_X` watching it pass, then fall (y 400 → 488 → 1132 → 4294).
        //
        // Walking at the wand (clamped short of the pit, which is what `FIRST_PIT_SAFE_X` is
        // for) converges with time to spare: from x=226 she covers the 257 px gap in under a
        // second, and the wand needs ~2.9 s to reach the pit.
        let wand_x = world_items(&mut app)
            .first()
            .map(|(_, pos)| pos.x)
            .unwrap_or(FIRST_PIT_SAFE_X);
        let target = wand_x.min(FIRST_PIT_SAFE_X);
        got_cap = drive(&mut app, 50, |b| {
            if b.is_tall() {
                return None;
            }
            let toward = if b.pos.x > target { -1.0 } else { 1.0 };
            Some(move_x(toward, false))
        });
        eprintln!(
            "mount chunk {chunk}: tall={got_cap} her={:?} items={:?}",
            body(&mut app).map(|b| b.pos),
            world_items(&mut app)
        );
        if got_cap {
            break;
        }
    }
    eprintln!("after mount attempt: tall={} {:?}", got_cap, body(&mut app));

    assert!(
        got_cap,
        "she must take the wand off the ?-block through the real pickup path; \
         worn form is {:?}, wears star_wand = {}",
        worn_form(&mut app),
        wears(&mut app, "star_wand")
    );
    assert!(
        wears(&mut app, "star_wand"),
        "the wand equips through the shared equipment path"
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
    // "On the pipe" has to mean ON THE MOUTH, not balanced on its corner. She
    // arrives carrying the run that got her up there and coasts — accept her at
    // the lip and she slides off the far edge before the transit can arm, which
    // is exactly what a player feels as "I keep missing the pipe".
    let on_pipe = drive(&mut app, 240, |b| {
        let over_mouth = b.pos.x > pipe.min.x + 24.0 && b.pos.x < pipe.max.x - 24.0;
        if b.on_ground && b.feet() <= pipe.min.y + 2.0 && over_mouth {
            return None;
        }
        // Once she is up, stop running and steer for the middle.
        if b.on_ground && b.feet() <= pipe.min.y + 2.0 {
            return Some(move_x(
                ((pipe.center().x - b.pos.x) / 16.0).clamp(-1.0, 1.0),
                false,
            ));
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
    let mouth_x = pipe.center().x;
    let dropped_in = drive(&mut app, 240, |b| {
        if b.pos.y > vault.min.y + 3.0 * 32.0 {
            return None;
        }
        Some(press_into_pipe_at(b, mouth_x, true))
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

    // ── The wand's actual EFFECT ──────────────────────────────────────────
    //
    // `star_wand` grants no verb; its whole effect is `OnHit::ConsumeAsArmor`.
    // So the way to exercise it is to take a hit and survive one that would
    // otherwise have cost a life. CONDITIONAL: it fires only if something
    // actually hit her on this run, which is why the assertions sit behind the
    // tall→small check rather than being asserted outright.
    let small_now = !body(&mut app).expect("she is in the world").is_tall();
    if tall_entering_vault && small_now {
        assert!(
            !wears(&mut app, "star_wand"),
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
            "the wand ABSORBED the hit — armor that still costs health is not armor"
        );
        eprintln!("ARMOR ABSORBED a hit in the vault, hp still {hp_entering_vault}");
    }

    // ── 4. Re-power at the second ?-block ─────────────────────────────────
    //
    // The ladder again, from the other end: a SMALL Mary-O bonking a fresh
    // ?-block gets the wand, which is what makes the power state a function of
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
        "a SMALL Mary-O bonking a fresh ?-block gets the wand again — the power \
         state is a function of her equipment, not a one-shot flag. She is {:?}",
        body(&mut app)
    );
    assert!(
        wears(&mut app, "star_wand"),
        "and the second wand equips through the same shared path as the first"
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
        &ambition_platformer2d::actors::features::FeatureId,
        &ambition_platformer2d::actors::features::CenteredAabb,
        &ambition_demo_mary_o::snake::SnakeShell,
    )>();
    q.iter(app.world())
        .find(|(feature_id, ..)| feature_id.as_str() == id)
        .map(|(_, aabb, shell)| (aabb.aabb(), *shell))
}

/// The id of SOME staged snake, and its state.
///
/// A snake's placement is authored now, so its id carries the LDtk iid — and there is no
/// numbering to guess. What the tests actually need is *a* snake, and `SnakeShell` is that:
/// only the tag pass attaches it, and it does so off the actor's brain.
fn some_snake(
    app: &mut App,
) -> Option<(String, ae::Aabb, ambition_demo_mary_o::snake::SnakeShell)> {
    let mut q = app.world_mut().query::<(
        &ambition_platformer2d::actors::features::FeatureId,
        &ambition_platformer2d::actors::features::CenteredAabb,
        &ambition_demo_mary_o::snake::SnakeShell,
    )>();
    let mut found: Vec<_> = q
        .iter(app.world())
        .map(|(feature_id, aabb, shell)| (feature_id.as_str().to_string(), aabb.aabb(), *shell))
        .collect();
    // Sorted so a test that picks "the first" picks the same one every run.
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.into_iter().next()
}

/// Landing on a Solid Snake is a STOMP, never a hit.
///
/// The shell rule classifies from body positions; the contact pass is documented to read
/// post-movement positions. Order the shell rule before the movement phase and the two disagree by
/// one frame of falling — on the landing frame the contact pass sees the overlap while the shell
/// rule still saw her in the air, so the snake stays armed and landing on it hurts.
///
/// No scripted input is needed (and so no skip): she is dropped, and gravity is
/// the whole experiment.
#[test]
fn landing_on_a_snake_stomps_it_instead_of_hurting_her() {
    let mut app = boot();
    settle_until_playable(&mut app);

    let (id, snake, phase) = some_snake(&mut app).expect("level 1-1 stages Solid Snakes");
    let id = id.as_str();
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

/// A small Mary-O dies to one hit, and the level restarts.
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

    let (_, snake, _) = some_snake(&mut app).expect("level 1-1 stages Solid Snakes");
    let lives_before = level(&mut app).0;

    // She goes at the snake's own centre: level with it, overlapping it, feet BELOW its top, which
    // is a side contact and cannot be read as landing on it.
    {
        let mut kin = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>()
            .single_mut(app.world_mut())
            .expect("the controlled body");
        kin.pos = snake.center();
        kin.vel = ae::Vec2::ZERO;
    }

    let mut saw_death_pose = false;
    let mut died = false;
    for _ in 0..600 {
        step(&mut app, idle());
        let dying = app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::characters::actor::BodyAnimFacts, With<PrimaryPlayer>>()
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

// ── "No way to get the fire flower" ───────────────────────────────────────

/// The ?-blocks that level TOWARD the lantern, left to right.
///
///  asked of the CONTENTS, not the look. 1-1 authors five blocks wearing
/// the ?-look and two of them hold `AlwaysQuasar`, which is not on the ladder at
/// all (any form takes a quasar). [`first_power_block`] matches the LOOK, so
/// which block it names depends on the order the converter emits entities in;
/// this asks the question the ladder actually cares about.
fn ladder_blocks() -> Vec<ae::world::Block> {
    use ambition_demo_mary_o::ldtk_vocabulary::{MaryOBlockContents, MaryOPickup};
    let room = ambition_demo_mary_o::level_1_1();
    let mut blocks: Vec<ae::world::Block> = room
        .world
        .blocks
        .iter()
        .filter(|b| {
            ambition_demo_mary_o::ldtk_vocabulary::block_of(&b.name).is_some_and(|authored| {
                authored.contents == MaryOBlockContents::Toward(MaryOPickup::Lantern)
            })
        })
        .cloned()
        .collect();
    blocks.sort_by(|a, b| a.aabb.min.x.partial_cmp(&b.aabb.min.x).expect("finite"));
    blocks
}

/// Has this block given up its reward? The game's own record, same as the run
/// above uses — a block is spent by being struck, so this is the honest "did the
/// bonk land" and not a proxy for it.
fn is_spent(app: &App, block: &ae::world::Block) -> bool {
    app.world()
        .get_resource::<ambition_demo_mary_o::powerups::SpentPowerBlocks>()
        .is_some_and(|spent| spent.is_spent(&block.id))
}

/// Where the item wearing this sprite is, if one is in the world.
fn item_at(app: &mut App, sprite: &str) -> Option<ae::Vec2> {
    world_items(app)
        .into_iter()
        .find(|(id, _)| id.as_str() == sprite)
        .map(|(_, pos)| pos)
}

/// Stand under a block and head-bonk it.
///
/// She jumps only from directly beneath the column and steers nothing while
/// airborne, which is what makes the strike inevitable rather than tuned: the
/// block is solid, so a body rising under it stops against its underside
/// whatever the jump was worth. The hold is released off the MEASUREMENT (her
/// head against the underside) rather than a frame count, so the beat stays
/// correct for a body whose height changes — the whole point here, since a grown
/// Mary-O's head is 8px higher than a small one's.
///
/// Returns as soon as the block records the strike, so a failure is a real
/// failure and not a budget that ran out one frame early.
fn bonk_from_beneath(app: &mut App, block: &ae::world::Block, frames: usize) -> bool {
    let centre = block.aabb.center().x;
    let underside = block.aabb.max.y;
    for _ in 0..frames {
        if is_spent(app, block) {
            return true;
        }
        let Some(b) = body(app) else {
            app.update();
            continue;
        };
        // A live walker in the same stretch comes first, exactly as it does in
        // the run above: from the side it costs her the form this beat is about.
        let frame = if b.should_stomp() {
            with_jump(move_x(1.0, false))
        } else if b.on_ground && (b.pos.x - centre).abs() < 8.0 {
            // Under it: straight up. Steering while airborne is what carries her
            // over the block instead of into its underside.
            with_jump(move_x(0.0, false))
        } else if b.on_ground {
            move_x((centre - b.pos.x).clamp(-1.0, 1.0), false)
        } else if b.head() > underside {
            with_jump(move_x(0.0, false))
        } else {
            move_x(0.0, false)
        };
        step(app, frame);
    }
    is_spent(app, block)
}

/// Walk into a travelling pickup until it equips. The wand walks and turns
/// at walls, so this steers at wherever it has got to — clamped short of
/// `limit_x`, because the pickup will happily walk into a pit and following it
/// there is not what is being tested.
fn chase_until_worn(
    app: &mut App,
    sprite: &str,
    row_id: &str,
    limit_x: f32,
    frames: usize,
) -> bool {
    for _ in 0..frames {
        if wears(app, row_id) {
            return true;
        }
        let target = item_at(app, sprite).map(|pos| pos.x).unwrap_or(limit_x);
        let Some(b) = body(app) else {
            app.update();
            continue;
        };
        let frame = if b.should_stomp() {
            with_jump(move_x(1.0, false))
        } else {
            let target = target.min(limit_x);
            move_x(((target - b.pos.x) / 8.0).clamp(-1.0, 1.0), false)
        };
        step(app, frame);
    }
    wears(app, row_id)
}

/// A moving jump onto the block itself.
///
/// `mount` above launches from a STANDSTILL and steers in the air, which is
/// right for the pipe (a 64px rise onto a 64px-wide mouth) and cannot work here,
/// and the arithmetic says why. A standing jump leaves the ground at 435 px/s
/// (band 1 of her phased-gravity law), runs weak gravity until it decays to
/// `held_phase_min_upward_speed`, and apexes about 145px up — while a ?-block's
/// top face is 128px over the run she walks. That is 17px of margin: her feet
/// are above the face for roughly a quarter-second, and neutral air preserves
/// momentum exactly (`air_coast_decel: 0`), so a jump that started still has
/// only `air_accel` to cross the gap with and covers about 12px of it.
///
/// So she LEAVES the ground moving, the way a player does it, and steers only
/// once she is clear of the face — the reversal on the far side of the target is
/// what stops her over a 32-pixel platform rather than past it.
///
/// `side` is which side of the block she attempts it from: `-1` left, `+1`
/// right. `backing` is the first stretch of an attempt, walking out to the
/// run-up.
fn hop_onto(b: Body, block: ae::Aabb, launch_gap: f32, side: f32, backing: bool) -> ControlFrame {
    let centre = block.center().x;
    // Where she leaves the ground, and where the run-up starts.
    let mark = centre + side * launch_gap;
    let start = centre + side * (launch_gap + 48.0);
    if b.on_ground {
        if backing {
            return move_x(((start - b.pos.x) / 8.0).clamp(-1.0, 1.0), false);
        }
        // Charging at the block. She leaves the ground the moment she crosses
        // the mark, so the launch carries her walking speed into the band that
        // decides how high the jump goes.
        let past_mark = (b.pos.x - mark) * side < 0.0;
        let charge = move_x(-side, false);
        return if past_mark { with_jump(charge) } else { charge };
    }
    if b.feet() < block.min.y - 2.0 {
        // Clear of the face: steer FOR the block. Past it the input flips and
        // `air_reverse_accel` brings her back over it.
        with_jump(move_x(((centre - b.pos.x) / 24.0).clamp(-1.0, 1.0), false))
    } else {
        // Still beside it: keep the launch momentum, add nothing to it.
        with_jump(move_x(0.0, false))
    }
}

/// Climb onto the block to take what is sitting on it. The beacon is
/// `ItemMotionPlan::still()` — it waits on its block like the classic flower —
/// so unlike the wand it never comes to her and there is nothing to chase.
///
/// Returns `false` only after every run-up on both sides has been tried, which
/// is what makes a failure here a statement about the LEVEL rather than about
/// one hand-picked number.
fn mount_until_worn(app: &mut App, block: &ae::world::Block, row_id: &str, frames: usize) -> bool {
    const ATTEMPTS: [(f32, f32); 8] = [
        (-1.0, 56.0),
        (1.0, 56.0),
        (-1.0, 72.0),
        (1.0, 72.0),
        (-1.0, 88.0),
        (1.0, 88.0),
        (-1.0, 104.0),
        (1.0, 104.0),
    ];
    const FRAMES_PER_ATTEMPT: usize = 120;
    // The first stretch of each attempt walks out to the run-up start.
    const BACKING_FRAMES: usize = 40;
    for frame_index in 0..frames {
        if wears(app, row_id) {
            return true;
        }
        let Some(b) = body(app) else {
            app.update();
            continue;
        };
        let attempt = frame_index / FRAMES_PER_ATTEMPT;
        let (side, launch_gap) = ATTEMPTS[attempt % ATTEMPTS.len()];
        let backing = frame_index % FRAMES_PER_ATTEMPT < BACKING_FRAMES;
        step(app, hop_onto(b, block.aabb, launch_gap, side, backing));
    }
    wears(app, row_id)
}

/// A GROWN Mary-O bonks a ?-block and ends up wearing the fire flower.
///
/// This is the decision procedure for that report, and either result closes it: if it passes, the
/// level CAN hand it over and nothing in the game says so; if it fails, the level cannot.
///
/// The three claims it makes, in the order they can fail:
///
/// 1. 1-1 authors more than one ladder block. Nothing pays the beacon to a
///    small Mary-O, so a level with one ?-block can never produce one however
///    well it is played.
/// 2. A block struck by a GROWN body pays the BEACON, not another wand. That
///    is `next_rung_toward` seen from outside — the rung is chosen from what she
///    wears, and nothing had ever asked it that question through the real
///    movement kernel.
/// 3. She can reach what it paid. The beacon does not travel; it waits on
///    the block that produced it, so collecting it is a second platforming act
///    and the one the report is most likely to have been about.
#[test]
fn a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower() {
    use ambition_demo_mary_o::powerups::{
        CINDER_BEACON_ID, CINDER_BEACON_SPRITE, STAR_WAND_ID, STAR_WAND_SPRITE,
    };

    let mut app = boot();
    settle_until_playable(&mut app);
    assert_scripted_input_reaches_the_sim(&mut app);

    let ladder = ladder_blocks();
    assert!(
        ladder.len() >= 2,
        "1-1 must author at least TWO blocks that level toward the lantern, or \
         the fire form is unreachable in it by construction: a small Mary-O is \
         paid the wand by the first one, so the beacon needs a second. Authored: \
         {:?}",
        ladder.iter().map(|b| b.name.clone()).collect::<Vec<_>>()
    );
    let first = ladder[0].clone();
    let second = ladder[1].clone();
    let pits = pits();
    // Chasing the wand stops at the first pit's lip: the pickup walks, and the
    // pit is not what this run is testing.
    let safe_x = pits
        .first()
        .map(|&(left, _)| left - 24.0)
        .unwrap_or(f32::MAX);
    eprintln!(
        "ladder blocks {:?}; pits {pits:?}",
        ladder.iter().map(|b| b.aabb).collect::<Vec<_>>()
    );

    // ── HARNESS: get grown, through the real ladder ───────────────────────
    let struck_first = bonk_from_beneath(&mut app, &first, 400);
    assert!(
        struck_first,
        "HARNESS: she never struck the first ladder block, so this run has said \
         nothing about the flower. She is at {:?} and the underside is {:.1}",
        body(&mut app),
        first.aabb.max.y
    );
    let took_the_wand = chase_until_worn(&mut app, STAR_WAND_SPRITE, STAR_WAND_ID, safe_x, 600);
    assert!(
        took_the_wand,
        "HARNESS: she never caught the wand the first block paid, so she cannot \
         arrive at the second one grown. She is at {:?}, items are {:?}",
        body(&mut app),
        world_items(&mut app)
    );
    assert_eq!(
        worn_form(&mut app).as_deref(),
        Some(TALL_ID),
        "HARNESS: the wand is what makes her grown"
    );

    // ── HARNESS: carry the form to the second block ───────────────────────
    let reached = run_right_to(&mut app, second.aabb.center().x, &pits, 1200);
    assert!(
        reached,
        "HARNESS: she stalled on the way to the second ladder block at {:?}",
        body(&mut app)
    );
    let still_grown = body(&mut app).expect("she is in the world").is_tall();
    assert!(
        still_grown,
        "HARNESS: something took the wand off her between the two blocks, so the \
         block she is about to hit will pay a WAND and this run cannot answer the \
         question. Worn form is {:?}",
        worn_form(&mut app)
    );

    // ── THE MEASUREMENT ───────────────────────────────────────────────────
    let struck_while_grown = bonk_from_beneath(&mut app, &second, 400);
    assert!(
        struck_while_grown,
        "a GROWN Mary-O must be able to strike a ?-block at all — the blocks sit \
         four tiles over the run she walks, and her head clears the underside by \
         more when she is tall than when she is small. She is at {:?}",
        body(&mut app)
    );
    let paid = item_at(&mut app, CINDER_BEACON_SPRITE);
    assert!(
        paid.is_some(),
        "the block a GROWN Mary-O struck must pay the BEACON — the rung is chosen \
         from what she wears, and a second wand here would mean the fire form is \
         unreachable however well the level is played. Items in the world: {:?}",
        world_items(&mut app)
    );
    eprintln!("the grown bonk paid a beacon at {paid:?}");

    let wore_it = mount_until_worn(&mut app, &second, CINDER_BEACON_ID, 1000);
    assert!(
        wore_it,
        "and she must be able to REACH it: the beacon waits on the block that \
         paid it, so getting it means climbing onto that block. She is at {:?} \
         and it is at {paid:?}",
        body(&mut app)
    );
    assert_eq!(
        worn_form(&mut app).as_deref(),
        Some(FIRE_ID),
        "wearing the beacon puts her in the fire form"
    );
    assert!(
        body(&mut app).expect("she is in the world").is_tall(),
        "and the fire form is the same height as the grown one — the beacon is a \
         step up the ladder, never a step off it"
    );
}
