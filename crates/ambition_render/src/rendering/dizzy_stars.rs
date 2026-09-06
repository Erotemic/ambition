//! Dizzy stars orbiting a body whose guard has shattered.
//!
//! A shield break is the single most punishing thing that happens to a
//! defender, and until the stars it looked like an ordinary stumble: the
//! `dizzy` pose plays, which is right, but nothing said *why* the fighter had
//! stopped answering. The stars are the genre's answer and they read from
//! across the stage.
//!
//! The stars orbit the body's OWN up, taken from the resolved frame published
//! on [`GuardBreakFact`]. Screen `-Y` would be wrong for the same reason it is
//! wrong everywhere else in this engine: a wall-walker's stars would orbit its
//! shoulder.
//!
//! Emission is keyed to the SIM TICK, exactly as the launch trail is, so the
//! ring turns at one rate whatever the display does, a capture shows what the
//! screen showed, and there is no per-body presentation state for a rollback
//! resimulation to multiply. The `dizzy` pose stays the base animation —
//! nothing here touches what the body is drawn as.

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;
use ambition_sim_view::{GuardBreakFact, GuardBreaksView};
use ambition_time::SimTick;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

/// Stars in the ring. Three is the genre's count and it is also the smallest
/// number that reads as a circle rather than as a wobble.
const STAR_COUNT: u32 = 3;

/// Sim ticks between emissions. The stars are short-lived particles refreshed
/// on this cadence, so this is the ring's frame rate as much as its density.
const EMIT_STRIDE: u64 = 3;

/// Ticks for one full turn of the ring.
const ORBIT_TICKS: f32 = 48.0;

/// Ring radius as a fraction of the body's larger extent, and how far above
/// the body's centre — along the body's own up — the ring is centred.
const ORBIT_RADIUS_FRACTION: f32 = 0.42;
const ORBIT_HEIGHT_FRACTION: f32 = 0.72;

/// The ring flattens toward the viewer instead of being a true circle: a
/// head-on circle reads as a halo, an ellipse reads as orbit.
const ORBIT_FLATTEN: f32 = 0.42;

/// Star colour, and how the ring fades out as the body recovers. The stars
/// thin out toward the end of the break so the beat has a visible finish
/// rather than simply stopping.
const STAR_RGB: [f32; 3] = [1.0, 0.90, 0.42];
const STAR_ALPHA: f32 = 0.95;

/// How much of the break's tail is spent fading. `phase` is `0.0` at the
/// shatter and approaches `1.0` at recovery.
const FADE_FROM_PHASE: f32 = 0.72;

/// The ring's alpha at this point in the break — full until the tail, then
/// fading to nothing as the fighter comes back.
///
/// A pure function of the published phase so the fade is asserted without a
/// renderer, and so nothing here has to know how long a break lasts.
pub fn star_alpha(phase: f32) -> f32 {
    let phase = phase.clamp(0.0, 1.0);
    if phase <= FADE_FROM_PHASE {
        return STAR_ALPHA;
    }
    let through_tail = (phase - FADE_FROM_PHASE) / (1.0 - FADE_FROM_PHASE);
    STAR_ALPHA * (1.0 - through_tail).clamp(0.0, 1.0)
}

/// Where one star sits this tick, in world space.
///
/// `up` is the body's own up (the negated toward-feet direction), so the ring
/// is built in the body's frame and only then expressed in world coordinates.
/// The `right` axis is `up` turned a quarter turn, which keeps the ellipse
/// square to the body under any gravity.
pub fn star_offset(index: u32, tick: u64, up: Vec2, size: Vec2) -> Vec2 {
    let right = Vec2::new(-up.y, up.x);
    let radius = size.x.max(size.y) * ORBIT_RADIUS_FRACTION;
    let height = size.x.max(size.y) * ORBIT_HEIGHT_FRACTION;
    let turn = (tick as f32 / ORBIT_TICKS) + (index as f32 / STAR_COUNT as f32);
    let angle = turn * std::f32::consts::TAU;
    up * height + right * (angle.cos() * radius) + up * (angle.sin() * radius * ORBIT_FLATTEN)
}

/// Turn the ring for every body still paying for a broken guard.
pub fn emit_dizzy_stars(
    tick: Res<SimTick>,
    mut last_sampled: Local<Option<u64>>,
    breaks: Res<GuardBreaksView>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    if *last_sampled == Some(tick.0) {
        return;
    }
    *last_sampled = Some(tick.0);
    if tick.0 % EMIT_STRIDE != 0 {
        return;
    }
    for body in &breaks.0 {
        let alpha = star_alpha(body.phase);
        if alpha <= 0.0 {
            continue;
        }
        for index in 0..STAR_COUNT {
            vfx.write(star_message(body, index, tick.0, alpha));
        }
    }
}

fn star_message(body: &GuardBreakFact, index: u32, tick: u64, alpha: f32) -> VfxMessage {
    // The body's up is the opposite of the direction its feet point.
    let up = -body.gravity_dir;
    VfxMessage::Burst {
        pos: body.pos + star_offset(index, tick, up, body.size),
        // ONE particle per star: the ring's shape is the emission POSITIONS,
        // not a spray. A burst of several would blur the three points into a
        // cloud and lose the orbit.
        count: 1,
        // Nearly stationary, so each star sits where it was placed and the
        // ring is legible as a ring.
        speed: 6.0,
        color: [STAR_RGB[0], STAR_RGB[1], STAR_RGB[2], alpha],
        // Spark: bright, short-lived, and it SHRINKS as it ages, so a star
        // winks out instead of swelling into a puff the way Dust would.
        kind: ParticleKind::Spark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broken(phase: f32, down: Vec2) -> GuardBreakFact {
        GuardBreakFact {
            pos: Vec2::ZERO,
            size: Vec2::new(30.0, 48.0),
            phase,
            gravity_dir: down,
        }
    }

    /// THE RELATIVITY RULE: the ring is built in the body's frame. Under
    /// flipped gravity the stars must be on the other side of the body, not
    /// stubbornly at the top of the screen.
    #[test]
    fn the_ring_orbits_the_bodys_own_up_not_the_screens() {
        let size = Vec2::new(30.0, 48.0);
        // Engine coords are y-down, so ordinary gravity points +Y and up is -Y.
        let ordinary_up = Vec2::new(0.0, -1.0);
        let flipped_up = Vec2::new(0.0, 1.0);
        let sideways_up = Vec2::new(1.0, 0.0);

        for tick in 0..ORBIT_TICKS as u64 {
            for index in 0..STAR_COUNT {
                let ordinary = star_offset(index, tick, ordinary_up, size);
                let flipped = star_offset(index, tick, flipped_up, size);
                let sideways = star_offset(index, tick, sideways_up, size);
                // Ordinary gravity puts the ring ABOVE the body (negative y).
                assert!(ordinary.y < 0.0, "tick {tick} star {index}: {ordinary:?}");
                // Flipped gravity puts it below, and by a mirrored amount.
                assert!(flipped.y > 0.0, "tick {tick} star {index}: {flipped:?}");
                assert!((ordinary.y + flipped.y).abs() < 1e-3);
                // A wall-walker's ring is out to its side, not over its head.
                assert!(sideways.x > 0.0, "tick {tick} star {index}: {sideways:?}");
            }
        }
    }

    /// The stars are a RING: they are spread around it, and it turns.
    #[test]
    fn the_stars_are_spread_around_a_ring_that_turns() {
        let size = Vec2::new(30.0, 48.0);
        let up = Vec2::new(0.0, -1.0);

        // At any tick the three stars occupy three different places.
        let places: Vec<Vec2> = (0..STAR_COUNT)
            .map(|index| star_offset(index, 0, up, size))
            .collect();
        for (a, index) in places.iter().zip(0..) {
            for (b, other) in places.iter().zip(0..) {
                if index != other {
                    assert!(
                        a.distance(*b) > 1.0,
                        "stars {index} and {other} are stacked: {a:?} {b:?}"
                    );
                }
            }
        }

        // And a quarter turn later star 0 has moved.
        let later = star_offset(0, (ORBIT_TICKS / 4.0) as u64, up, size);
        assert!(places[0].distance(later) > 1.0, "the ring must turn");

        // A full turn returns it to where it started.
        let round = star_offset(0, ORBIT_TICKS as u64, up, size);
        assert!(
            places[0].distance(round) < 1e-2,
            "{:?} {round:?}",
            places[0]
        );
    }

    /// The beat finishes rather than stopping: full through the break, fading
    /// through the tail, gone at recovery.
    #[test]
    fn the_ring_fades_out_as_the_fighter_comes_back() {
        assert_eq!(star_alpha(0.0), STAR_ALPHA, "a fresh break is at full");
        assert_eq!(star_alpha(FADE_FROM_PHASE), STAR_ALPHA);
        let mid_tail = star_alpha((FADE_FROM_PHASE + 1.0) * 0.5);
        assert!(mid_tail > 0.0 && mid_tail < STAR_ALPHA, "{mid_tail}");
        assert_eq!(star_alpha(1.0), 0.0, "recovered means gone");
        // Out-of-range phases cannot revive the ring or overdrive it.
        assert_eq!(star_alpha(4.0), 0.0);
        assert_eq!(star_alpha(-4.0), STAR_ALPHA);
    }

    /// A body with no broken guard asks for nothing, and a recovered one stops.
    #[test]
    fn only_a_broken_guard_spins_stars() {
        let mut app = harness();
        assert!(run_ticks(&mut app, 9).is_empty(), "no break, no stars");

        set_breaks(&mut app, &[broken(0.1, Vec2::new(0.0, 1.0))]);
        let asked = run_ticks(&mut app, 9);
        assert!(!asked.is_empty(), "a shattered guard spins stars");
        assert_eq!(
            asked.len() % STAR_COUNT as usize,
            0,
            "stars are emitted a whole ring at a time"
        );

        // Fully recovered: the row is gone the instant the timer is.
        set_breaks(&mut app, &[]);
        assert!(run_ticks(&mut app, 9).is_empty());
    }

    /// A frame that advanced no sim tick emits nothing, so a fast display does
    /// not spin the ring faster or thicken it.
    #[test]
    fn a_frame_without_a_sim_tick_emits_nothing() {
        let mut app = harness();
        set_breaks(&mut app, &[broken(0.1, Vec2::new(0.0, 1.0))]);
        // Land on an emitting tick, then redraw without advancing the sim.
        run_ticks(&mut app, 1);
        drain(&mut app);
        for _ in 0..4 {
            app.update();
        }
        assert!(drain(&mut app).is_empty());
    }

    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<SimTick>();
        app.init_resource::<GuardBreaksView>();
        app.add_message::<VfxMessage>();
        app.add_systems(Update, emit_dizzy_stars);
        app
    }

    fn set_breaks(app: &mut App, rows: &[GuardBreakFact]) {
        let mut view = app.world_mut().resource_mut::<GuardBreaksView>();
        view.0.clear();
        view.0.extend_from_slice(rows);
    }

    fn run_ticks(app: &mut App, n: u64) -> Vec<VfxMessage> {
        for _ in 0..n {
            app.world_mut().resource_mut::<SimTick>().0 += 1;
            app.update();
        }
        drain(app)
    }

    fn drain(app: &mut App) -> Vec<VfxMessage> {
        app.world_mut()
            .resource_mut::<Messages<VfxMessage>>()
            .drain()
            .collect()
    }
}
