//! A body on a WIRE: the winch that lifts her, the pendulum she steers, and the
//! one release that writes her exit velocity.
//!
//! ⭐⭐ THESE ARE THE SIX CLAUSES OF JON'S ASK, 2026-08-29, EACH AS A NUMBER:
//! *"It is not a teleport… she gets lifted up by the wire, a fairly large
//! vertical distance, and while she is being lifted by the wire her motion
//! controls should let her swing like a pendulum so she has a bit of horizontal
//! recovery with it too."*
//!
//! ⛔⛔ WHAT THIS FILE CANNOT PROVE is that the MOVE runs any of it. A kernel
//! test drives `catch_the_wire` directly, so it is green for a move that never
//! authors the beat, roots her through it, or ends before the lift does — the
//! exact shape that let the trapdoor be declared done twice while visibly
//! broken. Those guards are in `performer_moveset.rs` and in `trap_probe`.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, LocalAxes, Vec2, World};

/// The rope, in the shape the Performer authors it.
const ROPE: f32 = 720.0;
const RISE: f32 = 420.0;
const LIFT_S: f32 = 0.55;
const MAX_SWING_DEG: f32 = 18.0;
const SWING_ACCEL: f32 = 3.4;
const RELEASE_RISE: f32 = 90.0;

/// The winch's cruise rate, ASKED OF THE KERNEL rather than restated.
///
/// ⛔⛔ THIS FUNCTION USED TO CARRY THE FORMULA, and it went stale the first time
/// the profile changed: the ease-out gained a cruise phase, the executor's solve
/// moved with it, and this copy did not — so five arms failed against a wire the
/// game no longer builds. A test that hard-codes the number its subject derives
/// is measuring a different subject.
fn winch_v0() -> f32 {
    crate::movement::winch_rate_for(RISE, LIFT_S, RELEASE_RISE)
}

fn stick(x: f32) -> InputState {
    InputState {
        axes: LocalAxes::new(x, 0.0),
        ..Default::default()
    }
}

/// A body in open air, already on the wire.
fn on_the_wire(_world: &World, at: Vec2) -> BodyClusterScratch {
    let mut scratch = BodyClusterScratch::new_with_abilities(at, AbilitySet::sandbox_all());
    scratch.kinematics.pos = at;
    let frame = crate::MotionFrame::from_direction(crate::DEFAULT_GRAVITY_DIR, TEST_TUNING.gravity);
    assert!(
        crate::movement::catch_the_wire(
            &mut scratch.model,
            at,
            frame,
            ROPE,
            LIFT_S,
            winch_v0(),
            MAX_SWING_DEG.to_radians(),
            SWING_ACCEL,
            RELEASE_RISE,
        ),
        "an axis-swept body takes the wire"
    );
    scratch
}

/// Somewhere with a lot of air above it — she is going up 420px.
fn open_air() -> (World, Vec2) {
    let world = test_world();
    let at = Vec2::new(800.0, 700.0);
    (world, at)
}

fn wire_of(scratch: &BodyClusterScratch) -> Option<crate::WireState> {
    scratch.axis().wire
}

/// ⛔⛔ CLAUSE ONE, AND IT IS THE WHOLE COMPLAINT: **SHE TRAVELS THROUGH THE
/// SPACE.** Jon: *"It is not a teleport… she doesn't teleport up, she gets
/// lifted up by the wire."*
///
/// ⭐ THE ASSERTION IS ON THE BIGGEST SINGLE TICK, not on the total. A move that
/// covered the whole distance in one frame and then sat still for the rest of
/// the beat has the same start and end as this one and is exactly the move being
/// replaced — so a test on the endpoints agrees with the bug. What separates a
/// lift from a placement is that NO tick is large.
///
/// ⭐ AND THE SECOND ARM IS THE ONE THAT MAKES IT MEAN SOMETHING: the rise is
/// MONOTONIC. A body that oscillated up and down could satisfy a per-tick bound
/// and still not be a lift.
#[test]
fn she_climbs_across_many_ticks_and_no_tick_is_a_teleport() {
    let (world, at) = open_air();
    let mut scratch = on_the_wire(&world, at);
    let mut heights = vec![scratch.kinematics.pos.y];
    for _ in 0..40 {
        step_scratch(&world, &mut scratch, stick(0.0));
        heights.push(scratch.kinematics.pos.y);
        if wire_of(&scratch).is_none() {
            break;
        }
    }
    let biggest = heights
        .windows(2)
        .map(|w| (w[0] - w[1]).abs())
        .fold(0.0_f32, f32::max);
    let total = heights[0] - heights[heights.len() - 1];
    // One tick of the authored winch is `rise / lift_s / 60` ≈ 12.7px. Twice
    // that is generous and still an order of magnitude short of the 215px the
    // teleport it replaces covered in ONE frame.
    assert!(
        biggest < 26.0,
        "a tick moved her {biggest}px — that is a placement, not a lift"
    );
    assert!(
        heights.windows(2).all(|w| w[1] <= w[0] + 0.01),
        "the rise is not monotonic: {heights:?}"
    );
    assert!(
        total > 300.0,
        "she only rose {total}px across the whole lift"
    );
}

/// ⛔⛔ CLAUSE FIVE: **A FAIRLY LARGE VERTICAL DISTANCE**, and the only honest
/// form of that claim is a number.
///
/// ⭐ AGAINST THE MOVE IT REPLACES: the teleport covered 215px. Against the
/// stage: the smash platform's surface sits 420px above the fall blast line, so
/// this lift is exactly the depth a fighter can be knocked to and still be
/// brought back to the boards. That is a number somebody can argue with, which
/// is the point — `MAX_UNDER_S` is the trapdoor's equivalent knob.
#[test]
fn the_lift_covers_the_authored_rise() {
    let (world, at) = open_air();
    let mut scratch = on_the_wire(&world, at);
    for _ in 0..60 {
        step_scratch(&world, &mut scratch, stick(0.0));
        if wire_of(&scratch).is_none() {
            break;
        }
    }
    let rose = at.y - scratch.kinematics.pos.y;
    assert!(
        (rose - RISE).abs() < 30.0,
        "the winch was authored for {RISE}px and delivered {rose}px"
    );
    assert!(
        rose > 215.0,
        "the wire must beat the 215px teleport it replaces; it rose {rose}px"
    );
}

/// ⛔⛔ CLAUSE SIX: **SHE SWINGS, AND THE SWING IS HERS.** Jon: *"her motion
/// controls should let her swing like a pendulum so she has a bit of horizontal
/// recovery with it too."*
///
/// ⛔⛔ THREE ARMS, AND THE NEUTRAL ONE IS WHAT MAKES THE OTHER TWO MEAN
/// ANYTHING. A rule that drifted every body sideways would pass a "held right
/// moves right" arm on its own. The neutral arm says the horizontal travel is
/// the PLAYER's; the mirrored arm says it is the STICK's and not the facing's —
/// the ledge-vs-cap question the trapdoor settled by running it both ways.
#[test]
fn a_held_stick_buys_horizontal_recovery_and_it_mirrors() {
    let (world, at) = open_air();

    // ⛔⛔ THE DISPLACEMENT IS READ WHERE THE WIRE LETS GO. Running on past it
    // measures ordinary air control and credits it to the swing — the mistake
    // three arms in this file made, and the one a "held right moved right"
    // assertion cannot feel.
    let hung_displacement = |sx: f32| {
        let mut s = on_the_wire(&world, at);
        for _ in 0..60 {
            step_scratch(&world, &mut s, stick(sx));
            if wire_of(&s).is_none() {
                return s.kinematics.pos.x - at.x;
            }
        }
        panic!("the wire never let go");
    };
    let neutral = hung_displacement(0.0);
    let right = hung_displacement(1.0);
    let left = hung_displacement(-1.0);

    assert!(
        neutral.abs() < 2.0,
        "a neutral stick drifted her {neutral}px sideways"
    );
    assert!(
        right > 40.0,
        "holding right bought only {right}px of recovery off the rope"
    );
    assert!(
        left < -40.0,
        "holding left bought only {left}px of recovery off the rope"
    );
    assert!(
        (right + left).abs() < 1.0,
        "the swing is not symmetric: right {right} vs left {left}"
    );
}

/// ⛔ AND IT IS *"A BIT"* OF RECOVERY, NOT A TRAVERSAL. The cap is what makes
/// that a bounded claim: an uncapped pendulum on a SHORTENING rope gains angle
/// every tick — the skater pulling her arms in — and would end the lift halfway
/// across the stage.
///
/// ⭐ MEASURED AGAINST THE STAGE it is authored for: 480px of platform, so a
/// held stick may buy well under half of it.
#[test]
fn the_swing_is_capped_so_the_recovery_stays_a_bit_of_one() {
    let (world, at) = open_air();
    let mut scratch = on_the_wire(&world, at);
    let mut furthest = 0.0_f32;
    for _ in 0..60 {
        step_scratch(&world, &mut scratch, stick(1.0));
        // ⛔⛔ MEASURED ONLY WHILE SHE IS ON THE ROPE. Running the loop past the
        // release put 220px on this number — ordinary air control, twenty-seven
        // ticks after the wire had let go, attributed to the swing. That is the
        // THIRD arm in this file to sample the wrong moment; the tell is always
        // a loop whose bound is a tick count rather than the state it is about.
        let Some(w) = wire_of(&scratch) else {
            break;
        };
        furthest = furthest.max((scratch.kinematics.pos.x - at.x).abs());
        assert!(
            w.angle.abs() <= MAX_SWING_DEG.to_radians() + 1e-4,
            "the swing passed its stop at {} rad",
            w.angle
        );
    }
    assert!(
        furthest < 150.0,
        "the swing reached {furthest}px — the smash platform is 480px wide, and \
         a third of it is not 'a bit'"
    );
}

/// ⛔⛔ THE WIRE LETS GO, AND WHEN IT DOES SHE IS STILL MOVING.
///
/// ⭐ THE RELEASE IS THE ONE WRITER OF EXIT VELOCITY, which is the lesson the
/// trapdoor paid for: `LEAP_OUT_SPEED` was authored twice, the later system
/// overwrote the earlier every time, and the move launched nobody for as long as
/// the constant existed. Here the carry is asserted as a RISE — she leaves the
/// wire still going up — because a body released dead-stop at the apex is the
/// teleport's feel with extra steps.
#[test]
fn the_wire_lets_go_and_the_release_is_the_one_write_of_her_exit_velocity() {
    let (world, at) = open_air();
    let mut scratch = on_the_wire(&world, at);
    let mut released_at = None;
    for tick in 0..60 {
        step_scratch(&world, &mut scratch, stick(0.0));
        if wire_of(&scratch).is_none() {
            released_at = Some(tick);
            break;
        }
    }
    let tick = released_at.expect("the wire lets go inside a second");
    assert!(
        tick >= 30,
        "the wire let go on tick {tick}; the authored lift is {LIFT_S}s"
    );
    // +y is DOWN, so rising is negative. The release carry survives the tick it
    // was written on — gravity has had one frame at most to eat into it.
    assert!(
        scratch.kinematics.vel.y < -RELEASE_RISE * 0.5,
        "she left the wire at vy={}, which is not a rise",
        scratch.kinematics.vel.y
    );
}

/// ⛔⛔ AND THE HANDOVER HAS NO STEP IN IT. The winch decelerates to
/// `release_rise`, so the last tick on the rope and the first tick off it are
/// travelling at nearly the same speed.
///
/// ⭐ THIS IS THE ARM THE FIRST DRAFT FAILED. A constant-rate winch lifted her
/// at 764 px/s and the release handed her back doing 90 — an eightfold stop at
/// the apex, which is the teleport's own feel arriving through a different
/// mechanic. The rise was right, the numbers were right, and it read wrong.
#[test]
fn the_lift_decelerates_into_the_release_instead_of_stopping_dead() {
    let (world, at) = open_air();
    let mut scratch = on_the_wire(&world, at);
    let mut last_on_the_rope = 0.0;
    for _ in 0..60 {
        let before = scratch.kinematics.vel.y;
        step_scratch(&world, &mut scratch, stick(0.0));
        if wire_of(&scratch).is_none() {
            let after = scratch.kinematics.vel.y;
            assert!(
                (after - last_on_the_rope).abs() < 60.0,
                "the wire was lifting her at {last_on_the_rope} and let go at \
                 {after} — that is a stop, not a handover"
            );
            return;
        }
        let _ = before;
        last_on_the_rope = scratch.kinematics.vel.y;
    }
    panic!("the wire never let go");
}

/// ⛔⛔ THE SWING HANDS ITS OWN SPEED OVER AT THE CUT — the tangent at that
/// angle, times the rope's length. That is the half of the horizontal recovery
/// a player can aim; the 99px she is displaced while hanging is the half she
/// gets for free.
///
/// ⛔⛔⛔ AND IT IS SAMPLED ON THE RELEASE TICK, WHICH IS THE WHOLE POINT OF THIS
/// COMMENT. The first version ran sixty ticks with the stick held and asserted
/// on the velocity at the end — so it passed on ORDINARY AIR CONTROL
/// twenty-seven ticks after the wire had let go, and would have passed just as
/// well for a release that wrote nothing at all. A trajectory dump is what
/// caught it: the exit velocity was `(0.0, -90.0)` in every arm.
///
/// ⭐⭐ THE CONTRACT IS "LEAVE THE WAY YOU LEANED", and the mirror is exact —
/// ±232 px/s, which is a shade over her own run speed and decays under air
/// friction into roughly another platform-eighth of travel.
///
/// ⭐ THE THIRD ARM IS WHAT MAKES IT A PENDULUM RATHER THAN A FACING BONUS: a
/// stick reversed mid-lift leaves her crossing the arc the OTHER way, and the
/// handover changes SIGN. Nothing that merely rewarded the held direction could
/// do that, and it is the clause Jon asked for — *"her motion controls should
/// let her swing like a pendulum."*
#[test]
fn the_release_carries_the_swings_own_tangential_speed() {
    let (world, at) = open_air();

    // Run until the tick the wire lets go, and read the velocity THERE.
    let exit_velocity = |steer: &dyn Fn(usize) -> f32| {
        let mut s = on_the_wire(&world, at);
        for tick in 0..60 {
            step_scratch(&world, &mut s, stick(steer(tick)));
            if wire_of(&s).is_none() {
                return s.kinematics.vel;
            }
        }
        panic!("the wire never let go");
    };

    let neutral = exit_velocity(&|_| 0.0);
    let right = exit_velocity(&|_| 1.0);
    let left = exit_velocity(&|_| -1.0);
    // Swung out, then reversed early enough to be crossing back through the
    // bottom of the arc when the rope lets go.
    let reversed = exit_velocity(&|tick| if tick < 16 { 1.0 } else { -1.0 });

    assert!(
        neutral.x.abs() < 5.0,
        "an unswung release fired her sideways at {}",
        neutral.x
    );
    // ⛔⛔ A BAND, NOT A FLOOR, AND THE BAND IS THE FINDING. With a HARD stop at
    // the cap this number was a coin flip: the kernel measured +229 px/s and
    // `wire_probe` measured 0 for the same authored wire, because zeroing
    // `ang_vel` at the stop makes the handover depend on whether she clipped it
    // in the last tick or two. A one-sided `> 150` was asserting the lucky side.
    // The stop is soft now, `ang_vel` decays into the release, and both
    // instruments land in this band.
    assert!(
        (100.0..220.0).contains(&right.x),
        "leaning right handed over {} px/s, outside the band both the kernel and \
         the probe should land in — if these two disagree again, the stop has \
         gone hard",
        right.x
    );
    assert!(
        (right.x + left.x).abs() < 1.0,
        "the handover does not mirror: right {} vs left {}",
        right.x,
        left.x
    );
    assert!(
        reversed.x < -60.0,
        "a stick reversed mid-lift left her going {} px/s — the swing has no \
         phase, so it is not a pendulum",
        reversed.x
    );
}

/// ⛔⛔ AND A CUT IS NOT A RELEASE. `cut_the_wire` is what a HIT calls, and it
/// must write no velocity at all: whatever interrupted the wire owns her motion
/// now, and a cut that also launched her would be the second authority that
/// deleted the trapdoor's leap.
#[test]
fn cutting_the_wire_writes_no_velocity_of_its_own() {
    let (world, at) = open_air();
    let mut scratch = on_the_wire(&world, at);
    for _ in 0..10 {
        step_scratch(&world, &mut scratch, stick(1.0));
    }
    // A launch lands on her while she hangs, the way a hit's would.
    let launch = Vec2::new(-300.0, -420.0);
    scratch.kinematics.vel = launch;
    assert!(
        crate::movement::cut_the_wire(&mut scratch.model),
        "she was on a wire"
    );
    assert_eq!(
        scratch.kinematics.vel, launch,
        "the cut overwrote the launch that caused it"
    );
    assert!(wire_of(&scratch).is_none());
    // And the next tick is ordinary physics: gravity is acting on her again.
    step_scratch(&world, &mut scratch, stick(0.0));
    assert!(
        scratch.kinematics.vel.y > launch.y,
        "she is not falling under gravity after the cut"
    );
}

/// ⛔ THE WIRE OUTRANKS THE MANEUVERS SHE WAS MID-WAY THROUGH. A dash timer that
/// survived the catch resumes on the frame the rope lets go and fires her
/// sideways out of her own recovery.
#[test]
fn catching_the_wire_ends_the_maneuvers_underneath_it() {
    let (_world, at) = open_air();
    let mut scratch = BodyClusterScratch::new_with_abilities(at, AbilitySet::sandbox_all());
    scratch.axis_mut().dash_timer = 0.4;
    scratch.axis_mut().jump_squat_timer = 0.2;
    let frame = crate::MotionFrame::from_direction(crate::DEFAULT_GRAVITY_DIR, TEST_TUNING.gravity);
    assert!(crate::movement::catch_the_wire(
        &mut scratch.model,
        at,
        frame,
        ROPE,
        LIFT_S,
        winch_v0(),
        MAX_SWING_DEG.to_radians(),
        SWING_ACCEL,
        RELEASE_RISE,
    ));
    assert_eq!(scratch.axis().dash_timer, 0.0);
    assert_eq!(scratch.axis().jump_squat_timer, 0.0);
}

/// ⛔ A DEGENERATE ROPE IS REFUSED RATHER THAN HUNG. A zero-length wire has no
/// angle — every direction is the same point — and the tangent it would release
/// along is undefined.
#[test]
fn a_degenerate_rope_is_refused() {
    let at = Vec2::new(800.0, 700.0);
    let frame = crate::MotionFrame::from_direction(crate::DEFAULT_GRAVITY_DIR, TEST_TUNING.gravity);
    let mut scratch = BodyClusterScratch::new_with_abilities(at, AbilitySet::sandbox_all());
    assert!(
        !crate::movement::catch_the_wire(
            &mut scratch.model,
            at,
            frame,
            0.0,
            LIFT_S,
            winch_v0(),
            MAX_SWING_DEG.to_radians(),
            SWING_ACCEL,
            RELEASE_RISE,
        ),
        "a zero-length rope must be refused"
    );
    assert!(
        !crate::movement::catch_the_wire(
            &mut scratch.model,
            at,
            frame,
            ROPE,
            0.0,
            winch_v0(),
            MAX_SWING_DEG.to_radians(),
            SWING_ACCEL,
            RELEASE_RISE,
        ),
        "a wire with no lift time must be refused"
    );
    assert!(scratch.axis().wire.is_none());
}
