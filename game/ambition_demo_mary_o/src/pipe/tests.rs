//! Unit tests for the pipe-transit choreography. The motion is a pure function,
//! so its whole shape is checked here without a running app — what headless
//! cannot check is how the slide LOOKS.

use super::*;

const DT: f32 = 1.0 / 60.0;
const TILE: f32 = 32.0;

/// The descent tube: sink from the surface mouth, come out of the vault pipe.
fn descent() -> PipeTransit {
    PipeTransit::begin(
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(500.0, 900.0),
        // `+y` is DOWN: you sink INTO a surface pipe.
        ae::Vec2::new(0.0, 1.0),
        TILE,
    )
}

/// Run a transit to completion, returning every position it passed through and
/// the tick index at which it crossed.
fn run(mut transit: PipeTransit) -> (Vec<ae::Vec2>, Option<usize>) {
    let mut path = Vec::new();
    let mut crossed_at = None;
    for tick in 0..600 {
        let fx = step_pipe_transit(transit, DT);
        path.push(fx.pos);
        if fx.crossed {
            assert!(crossed_at.is_none(), "a transit crosses exactly ONCE");
            crossed_at = Some(tick);
        }
        match fx.next {
            Some(next) => transit = next,
            None => return (path, crossed_at),
        }
    }
    panic!("the transit never finished");
}

/// A warp is a MOVE, not a teleport. It takes the authored time, it passes
/// through the pipe on the way, and it ends exactly where the warp promised.
#[test]
fn a_transit_slides_for_its_authored_duration_and_lands_on_the_arrival() {
    let (path, crossed_at) = run(descent());

    let ticks = path.len() as f32;
    let total = SWALLOW_S + EMERGE_S;
    assert!(
        (ticks * DT - total).abs() < 4.0 * DT,
        "the whole transit takes about {total}s, not a frame: {}s",
        ticks * DT
    );
    assert_eq!(
        *path.last().unwrap(),
        descent().arrival,
        "and it ends exactly where the warp promised"
    );
    let crossed_at = crossed_at.expect("the transit crosses to the far pipe");
    assert!(
        ((crossed_at as f32 + 1.0) * DT - SWALLOW_S).abs() < 4.0 * DT,
        "the crossing happens once the body is fully swallowed, about {SWALLOW_S}s in"
    );
}

/// The body is FULLY inside the near pipe before it crosses, and starts fully
/// inside the far one — which is what lets the pipe art swallow it, so nothing
/// pops in or out of existence in the open.
#[test]
fn the_body_is_swallowed_before_it_crosses_and_emerges_from_inside_the_far_pipe() {
    let t = descent();
    let travel = TRAVEL_TILES * TILE;
    assert_eq!(
        t.to,
        t.from + ae::Vec2::new(0.0, travel),
        "the swallow sinks a whole pipe-length in"
    );
    assert_eq!(
        t.throat,
        t.arrival - ae::Vec2::new(0.0, travel),
        "and the emergence starts a whole pipe-length BEHIND the arrival, inside \
         the far pipe"
    );
}

/// The exit continues the journey; it never reverses it.
///
/// down and then bounce up, and when going up you push up and then fall down."
/// That was the emergence running BACKWARDS — the throat sat on the far side of
/// the arrival, so going down a pipe dropped you below the ceiling mouth and
/// floated you up into place, and going up overshot and sank you back. Whichever
/// way a tube points, the second half must travel the SAME way as the first.
#[test]
fn the_emergence_travels_the_same_way_as_the_entry() {
    for (axis, what) in [
        (ae::Vec2::new(0.0, 1.0), "down a descent tube"),
        (ae::Vec2::new(0.0, -1.0), "up an ascent tube"),
    ] {
        let t = PipeTransit::begin(
            ae::Vec2::new(100.0, 400.0),
            ae::Vec2::new(500.0, 900.0),
            axis,
            TILE,
        );
        let entering = (t.to - t.from).normalize();
        let leaving = (t.arrival - t.throat).normalize();
        assert!(
            entering.dot(leaving) > 0.99,
            "going {what}, the body must LEAVE the far pipe the same way it entered \
             the near one (entering {entering:?}, leaving {leaving:?})"
        );
    }
}

/// The slide EASES: it starts slow, moves fastest mid-tube, and settles. A linear
/// ramp reads as a machine pulling you; this reads as a pipe.
#[test]
fn the_slide_eases_in_and_out_rather_than_moving_at_a_constant_rate() {
    let (path, _) = run(descent());
    let swallow_ticks = (SWALLOW_S / DT) as usize;
    let step = |i: usize| (path[i + 1] - path[i]).length();
    let first = step(0);
    let middle = step(swallow_ticks / 2);
    let last = step(swallow_ticks.saturating_sub(3));
    assert!(
        middle > first * 1.5 && middle > last * 1.5,
        "mid-slide must be markedly faster than either end (first {first}, middle \
         {middle}, last {last})"
    );
}

/// Both directions are the same machine. An ASCENT (screen-up into a ceiling
/// pipe) is built by pointing the axis the other way — nothing about the step
/// knows which way is down.
#[test]
fn an_ascent_is_the_same_machine_pointed_the_other_way() {
    let up = PipeTransit::begin(
        ae::Vec2::new(500.0, 900.0),
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(0.0, -1.0),
        TILE,
    );
    assert!(
        up.to.y < up.from.y,
        "rising into a ceiling pipe moves the body UP the screen"
    );
    let (path, crossed) = run(up);
    assert!(crossed.is_some());
    assert_eq!(*path.last().unwrap(), up.arrival);
    assert!(
        path.iter().all(|p| p.is_finite()),
        "no NaN anywhere on the path"
    );
}

/// A transit never overshoots: every sampled position stays on the segment it is
/// easing along, so the body cannot appear outside the tube mid-slide.
#[test]
fn the_slide_never_overshoots_either_end_of_its_segment() {
    let t = descent();
    let (path, crossed_at) = run(t);
    let crossed_at = crossed_at.unwrap();
    for (i, p) in path.iter().enumerate() {
        // The crossing tick itself already reports the FAR pipe's throat, so it
        // belongs to the second segment.
        let (a, b) = if i < crossed_at {
            (t.from, t.to)
        } else {
            (t.throat, t.arrival)
        };
        let (lo, hi) = (a.y.min(b.y), a.y.max(b.y));
        assert!(
            p.y >= lo - 0.01 && p.y <= hi + 0.01,
            "tick {i} left its segment: {p:?} not within {lo}..{hi}"
        );
    }
}
