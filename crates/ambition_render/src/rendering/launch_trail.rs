//! Hard-launch readability: the flight layer, in two beats.
//!
//! 1. **The blast.** A body thrown hard tears out of the hit in a bright spark
//!    flare that lasts exactly as long as the launch's hard control lock.
//! 2. **The plume.** For the rest of the flight it trails smoke behind its
//!    velocity, so a spectator can still read the launch after the hit spark
//!    is gone.
//!
//! Both are a layer over the hit spark and camera shake, not a replacement.
//!
//! The layering stops where the flight resolves. These cues predict danger;
//! the knockout beat answers it. A body out of play leaves
//! [`LaunchedBodiesView`], so the trail stops at that instant and the knockout
//! owns the beat alone.
//!
//! The two beats separate a body launched this instant from one that has
//! tumbled for a second at the same speed. The separating fact is
//! [`ambition_sim_view::LaunchedBodyFact::launch_beat_secs`], the sim's own
//! control-lock window.
//!
//! The gate is membership in [`LaunchedBodiesView`] (the sim's "this motion is
//! involuntary" fact) plus speed. Speed alone is not enough: a fighter can run,
//! fast-fall, and recover at any speed under its own power.
//!
//! Everything here is cosmetic and non-rollback. Emission is keyed to the sim
//! tick, not the frame or a per-body accumulator. So the plume has the same
//! density at any refresh rate, is the same in a capture, and has no state for
//! a rollback resimulation to multiply.

use bevy::prelude::*;

use ambition_sim_view::LaunchedBodiesView;
use ambition_time::SimTick;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

// The speed thresholds are percentiles of the speed a body flies at while
// launched, one sample per tick of involuntary flight. Source:
// `match_report -- 90 --runs 5`, `smash_george_booul` against itself
// (n = 8002):
//
//     p25 49   p50 213   p75 494   p90 713   p99 1183   max 1901
//
// The distribution is bimodal. The 100-px histogram:
//
//     0:3039  100:867  200:923  300:695  400:505  500:522  600:575  700:454
//     800:191  900:121  1000:22  1100:10   [1200-1499: NOTHING]
//     1500:64  1600:4  1700:4  1800:4  1900:2
//
// The cluster above the gap is a body in free fall off the bottom of the
// stage: the near-KO population.
//
// Do not fit these to peak launch speed (the speed when the launch is
// written). Gravity keeps acting, so flight speed is a different
// distribution. Do not fit them to one run. The sample is one character
// against itself; weight and fall speed change it, so state the matchup
// wherever these numbers are restated.

/// Speed at which a launch starts smoking, in world units per second.
///
/// The median tick of involuntary flight (p50 = 213). Below it the body is
/// drifting, at the top of its arc, or still helpless after its speed is
/// gone. The blast uses the same onset, so the two beats agree on what a
/// launch is.
const TRAIL_ONSET_SPEED: f32 = 210.0;

/// Speed at which the trail reaches full density. Past this, more particles
/// only cost fill rate.
///
/// The 90th percentile of flight (713). The ramp spans p50 to p90, the band
/// of an ordinary launch.
const TRAIL_FULL_SPEED: f32 = 710.0;

/// Sim ticks between puffs at onset density, and at full density. A stride of
/// one is a puff every tick.
const ONSET_STRIDE: u64 = 3;
const FULL_STRIDE: u64 = 1;

/// Particles per puff at onset and at full density. Slow-moving: the plume
/// hangs where the body was; it does not spray.
const ONSET_PARTICLES: u32 = 2;
const FULL_PARTICLES: u32 = 3;
const PUFF_SPREAD_SPEED: f32 = 34.0;

/// Fraction of the body's larger extent that the puff sits behind its centre.
const TRAIL_OFFSET_FRACTION: f32 = 0.4;

/// Puff colour: faint at onset, solid at full density.
const SMOKE_RGB: [f32; 3] = [0.82, 0.84, 0.90];
const MIN_SMOKE_ALPHA: f32 = 0.35;
const MAX_SMOKE_ALPHA: f32 = 0.78;

/// Speed at which a launch becomes a kill.
///
/// This sits in the middle of the empty band (1183 to 1500) in the histogram
/// above. Every free-fall tick burns, and no ordinary launch reaches it. A
/// percentile is wrong here: p99 (1183) is inside the main mass, so the ember
/// would fire on ordinary launches. The wall splat band uses the same
/// gap-based placement.
const TRAIL_NEAR_KO_SPEED: f32 = 1350.0;

/// The near-KO plume colour: an ember, not smoke. A hue change, because more
/// grey on an already saturated plume reads as nothing.
const EMBER_RGB: [f32; 3] = [1.0, 0.62, 0.30];

/// Extra particles per puff at the far end of the near-KO band.
const EMBER_EXTRA_PARTICLES: u32 = 2;

/// What one launched body's trail asks for this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailPuff {
    /// Emit only on ticks divisible by this. Smaller is denser.
    pub stride: u64,
    pub particles: u32,
    pub alpha: f32,
    /// How far into the near-KO band this launch is, `0..=1`. Drives the
    /// shift from smoke to ember and the extra particles; `0.0` below
    /// [`TRAIL_NEAR_KO_SPEED`].
    pub ember: f32,
}

/// How hard this flight reads, `0..=1`, on the trail's band: `0.0` below the
/// onset, `1.0` at full density.
///
/// Shared with the knockout beat, the end of a flight, so the two agree on
/// how hard the same launch was.
pub(crate) fn flight_intensity(speed: f32) -> f32 {
    launch_bands(true, speed).map_or(0.0, |(t, _)| t)
}

/// How hard this launch is, or `None` when it is not a launch worth reading.
///
/// `.0` is the hard-launch ramp: `0` at [`TRAIL_ONSET_SPEED`], `1` at
/// [`TRAIL_FULL_SPEED`]. `.1` is how far into the near-KO band it goes. The
/// blast and the plume share one onset, so a hit never shows a flare with no
/// smoke.
fn launch_bands(launched: bool, speed: f32) -> Option<(f32, f32)> {
    if !launched || speed < TRAIL_ONSET_SPEED {
        return None;
    }
    let t = ((speed - TRAIL_ONSET_SPEED) / (TRAIL_FULL_SPEED - TRAIL_ONSET_SPEED)).clamp(0.0, 1.0);
    // The near-KO band runs from its threshold for as far again as the
    // density ramp, so the ember arrives gradually.
    let ember =
        ((speed - TRAIL_NEAR_KO_SPEED) / (TRAIL_NEAR_KO_SPEED - TRAIL_FULL_SPEED)).clamp(0.0, 1.0);
    Some((t, ember))
}

/// The trail this launch asks for, or `None` when the launch is not hard
/// enough to read from across the stage.
///
/// The gate is here so tests can check it without a renderer. A voluntary
/// sprint and a launch at the same speed differ only by `launched`.
pub fn launch_trail_puff(launched: bool, speed: f32) -> Option<TrailPuff> {
    let (t, ember) = launch_bands(launched, speed)?;
    Some(TrailPuff {
        // Rounds toward the denser end as `t` rises, and never reaches zero.
        stride: lerp(ONSET_STRIDE as f32, FULL_STRIDE as f32, t)
            .round()
            .max(1.0) as u64,
        particles: lerp(ONSET_PARTICLES as f32, FULL_PARTICLES as f32, t).round() as u32
            + (EMBER_EXTRA_PARTICLES as f32 * ember).round() as u32,
        alpha: lerp(MIN_SMOKE_ALPHA, MAX_SMOKE_ALPHA, t),
        ember,
    })
}

/// Sparks in the flare per tick of the beat, at onset and at full strength.
/// The beat is short (the ordinary knockback control lock is `0.12s`, seven
/// ticks), so these are per-tick counts.
const BLAST_PARTICLES_ONSET: u32 = 3;
const BLAST_PARTICLES_FULL: u32 = 6;

/// How fast the flare throws its sparks. Ten times [`PUFF_SPREAD_SPEED`]: the
/// plume hangs where the body was, the flare flies away.
const BLAST_SPREAD_SPEED: f32 = 300.0;

/// The flare colour: hot white, shifting to the plume's ember at the near-KO
/// end, so both beats of one launch match.
const BLAST_RGB: [f32; 3] = [1.0, 0.97, 0.86];
const BLAST_MIN_ALPHA: f32 = 0.70;
const BLAST_MAX_ALPHA: f32 = 0.95;

/// What the front of a launch asks for, on top of the plume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchBlast {
    pub particles: u32,
    pub speed: f32,
    pub alpha: f32,
    /// Shared with [`TrailPuff::ember`] so the flare and the smoke of one
    /// launch shift colour together.
    pub ember: f32,
}

/// The flare this launch asks for this tick, or `None` once the launch is no
/// longer new.
///
/// `beat` is whether [`ambition_sim_view::LaunchedBodyFact::launch_beat_secs`]
/// is live: the sim's hard control lock at the start of a knockback. Do not
/// derive it from speed, hitstun, or the plume; a body that slows through the
/// same speed twice would flare twice.
pub fn launch_blast(beat: bool, speed: f32) -> Option<LaunchBlast> {
    if !beat {
        return None;
    }
    let (t, ember) = launch_bands(true, speed)?;
    Some(LaunchBlast {
        particles: lerp(BLAST_PARTICLES_ONSET as f32, BLAST_PARTICLES_FULL as f32, t).round()
            as u32
            + (EMBER_EXTRA_PARTICLES as f32 * ember).round() as u32,
        speed: BLAST_SPREAD_SPEED,
        alpha: lerp(BLAST_MIN_ALPHA, BLAST_MAX_ALPHA, t),
        ember,
    })
}

/// The flare's colour at this point in the near-KO band: white-hot at `0.0`,
/// ember at `1.0`.
fn blast_rgb(ember: f32) -> [f32; 3] {
    let ember = ember.clamp(0.0, 1.0);
    [
        lerp(BLAST_RGB[0], EMBER_RGB[0], ember),
        lerp(BLAST_RGB[1], EMBER_RGB[1], ember),
        lerp(BLAST_RGB[2], EMBER_RGB[2], ember),
    ]
}

/// The plume's colour at this point in the near-KO band: smoke at `0.0`,
/// ember at `1.0`.
fn plume_rgb(ember: f32) -> [f32; 3] {
    let ember = ember.clamp(0.0, 1.0);
    [
        lerp(SMOKE_RGB[0], EMBER_RGB[0], ember),
        lerp(SMOKE_RGB[1], EMBER_RGB[1], ember),
        lerp(SMOKE_RGB[2], EMBER_RGB[2], ember),
    ]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Emit the trailing plume behind every hard-launched body.
///
/// Runs on the render clock but samples on the sim clock. A frame with no new
/// tick emits nothing, so density depends on the flight, not the machine.
pub fn emit_launch_trails(
    tick: Res<SimTick>,
    mut last_sampled: Local<Option<u64>>,
    launched: Res<LaunchedBodiesView>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    if *last_sampled == Some(tick.0) {
        return;
    }
    *last_sampled = Some(tick.0);
    for body in &launched.0 {
        let speed = body.vel.length();
        // The blast first. It adds to the plume, so a launch smokes from the
        // tick it starts to flare. Sparks sit at the body, not behind it.
        if let Some(blast) = launch_blast(body.launch_beat_secs > 0.0, speed) {
            let rgb = blast_rgb(blast.ember);
            vfx.write(VfxMessage::Burst {
                pos: body.pos,
                count: blast.particles,
                speed: blast.speed,
                color: [rgb[0], rgb[1], rgb[2], blast.alpha],
                kind: ParticleKind::Spark,
            });
        }
        let Some(puff) = launch_trail_puff(true, speed) else {
            continue;
        };
        if tick.0 % puff.stride != 0 {
            continue;
        }
        let behind =
            body.vel / speed.max(1.0) * (body.size.x.max(body.size.y) * TRAIL_OFFSET_FRACTION);
        let rgb = plume_rgb(puff.ember);
        vfx.write(VfxMessage::Burst {
            pos: body.pos - behind,
            count: puff.particles,
            speed: PUFF_SPREAD_SPEED,
            color: [rgb[0], rgb[1], rgb[2], puff.alpha],
            // Dust already grows and drags as it ages, like a plume.
            kind: ParticleKind::Dust,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::Vec2;
    use ambition_sim_view::LaunchedBodyFact;

    /// The gate is the launch fact, not the speed. Both directions.
    #[test]
    fn only_a_launched_body_trails() {
        // Launched and fast: a trail.
        assert!(launch_trail_puff(true, TRAIL_ONSET_SPEED + 1.0).is_some());
        // Launched but barely moving: at the top of its arc, or hitstun that
        // outlives its speed.
        assert!(launch_trail_puff(true, TRAIL_ONSET_SPEED - 1.0).is_none());
        // Voluntary motion faster than any launch. A velocity-only gate fails
        // here.
        assert!(launch_trail_puff(false, TRAIL_FULL_SPEED * 4.0).is_none());
        // Back under normal control: no requests at any speed.
        assert!(launch_trail_puff(false, 0.0).is_none());
    }

    /// Density rises with the launch and then stops rising.
    #[test]
    fn density_rises_with_the_launch_and_saturates() {
        let onset = launch_trail_puff(true, TRAIL_ONSET_SPEED).unwrap();
        let full = launch_trail_puff(true, TRAIL_FULL_SPEED).unwrap();
        assert!(onset.stride > full.stride, "{onset:?} {full:?}");
        assert!(onset.particles <= full.particles);
        assert!(onset.alpha < full.alpha);
        assert!(full.stride >= 1, "a stride of zero would divide by zero");

        // The smoke saturates at full density. A faster launch adds the ember
        // instead (tested separately), so this checks stride and alpha only.
        let beyond = launch_trail_puff(true, TRAIL_FULL_SPEED * 10.0).unwrap();
        assert_eq!(beyond.stride, full.stride);
        assert_eq!(beyond.alpha, full.alpha);
    }

    /// A near-KO launch turns from smoke to ember, gradually. An ordinary hard
    /// launch is unchanged by this tier.
    #[test]
    fn a_near_ko_launch_burns_where_an_ordinary_one_smokes() {
        let ordinary = launch_trail_puff(true, TRAIL_FULL_SPEED).unwrap();
        assert_eq!(ordinary.ember, 0.0, "the band starts above full density");
        assert_eq!(plume_rgb(ordinary.ember), SMOKE_RGB);

        // The band opens at its threshold, not before it.
        assert_eq!(
            launch_trail_puff(true, TRAIL_NEAR_KO_SPEED - 1.0)
                .unwrap()
                .ember,
            0.0
        );

        let entering = launch_trail_puff(true, TRAIL_NEAR_KO_SPEED).unwrap();
        let deep = launch_trail_puff(true, TRAIL_NEAR_KO_SPEED * 2.0).unwrap();
        assert!(entering.ember < deep.ember, "the ember arrives gradually");
        assert_eq!(deep.ember, 1.0, "and it saturates");

        // The kill plume is hotter and thicker than the hard-launch one.
        let hot = plume_rgb(deep.ember);
        assert!(hot[0] > SMOKE_RGB[0] && hot[2] < SMOKE_RGB[2], "{hot:?}");
        assert!(
            deep.particles > ordinary.particles,
            "{} vs {}",
            deep.particles,
            ordinary.particles
        );

        // The launched gate still applies.
        assert!(launch_trail_puff(false, TRAIL_NEAR_KO_SPEED * 4.0).is_none());
    }

    /// The system half, over a run of ticks: a hard launch asks for smoke, and
    /// leaving the launched view stops the requests.
    #[test]
    fn leaving_the_launched_view_stops_the_requests() {
        let mut app = harness();
        set_launched(&mut app, Some(Vec2::new(1500.0, 0.0)));
        assert!(!run_ticks(&mut app, 6).is_empty(), "a hard launch smokes");

        set_launched(&mut app, None);
        assert!(
            run_ticks(&mut app, 6).is_empty(),
            "a body back under its own control must not trail"
        );
    }

    /// A slow launch is still a launch, and it still must not smoke.
    #[test]
    fn a_soft_launch_asks_for_nothing() {
        let mut app = harness();
        set_launched(&mut app, Some(Vec2::new(120.0, 0.0)));
        assert!(run_ticks(&mut app, 6).is_empty());
    }

    /// A frame that advanced no sim tick emits nothing, so a fast display does
    /// not thicken the plume.
    #[test]
    fn a_frame_without_a_sim_tick_emits_nothing() {
        let mut app = harness();
        set_launched(&mut app, Some(Vec2::new(1500.0, 0.0)));
        app.update();
        drain(&mut app);
        // Same tick, three more frames.
        for _ in 0..3 {
            app.update();
        }
        assert!(drain(&mut app).is_empty());
    }

    /// A body launched this instant and one that has tumbled for a second must
    /// look different. Same view and speed; only the published control lock
    /// differs, and only the first flares.
    #[test]
    fn the_front_of_a_launch_flares_and_a_sustained_tumble_does_not() {
        let hard = Vec2::new(TRAIL_ONSET_SPEED * 2.0, 0.0);

        let mut app = harness();
        set_launched_beat(&mut app, Some(hard), 0.09);
        let front = run_ticks(&mut app, 4);
        assert!(!sparks(&front).is_empty(), "the front of a launch flares");
        assert!(!dust(&front).is_empty(), "and it smokes from the same tick");

        // The lock ran out. Still launched, same speed, still trailing, and
        // no longer flaring.
        set_launched_beat(&mut app, Some(hard), 0.0);
        let sustained = run_ticks(&mut app, 4);
        assert!(
            sparks(&sustained).is_empty(),
            "a sustained tumble must not keep flaring: {sustained:?}"
        );
        assert!(
            !dust(&sustained).is_empty(),
            "but the plume is the rest of the flight and continues"
        );
    }

    /// The flare follows the launch, not the lock alone: a body in a control
    /// lock at walking pace does not flare.
    #[test]
    fn a_beat_below_the_launch_onset_does_not_flare() {
        assert!(launch_blast(true, TRAIL_ONSET_SPEED - 1.0).is_none());
        assert!(launch_blast(true, TRAIL_ONSET_SPEED).is_some());
        // No lock, no flare, at any speed.
        assert!(launch_blast(false, TRAIL_ONSET_SPEED * 10.0).is_none());
    }

    /// The flare contrasts with the plume, so the two beats read as two effects.
    #[test]
    fn the_flare_reads_against_the_plume_it_sits_on() {
        let hard = TRAIL_FULL_SPEED;
        let blast = launch_blast(true, hard).unwrap();
        let puff = launch_trail_puff(true, hard).unwrap();
        assert!(
            blast.speed > PUFF_SPREAD_SPEED * 4.0,
            "the flare leaves, the plume hangs"
        );
        assert!(
            blast.alpha > puff.alpha,
            "and it is the brighter of the two"
        );
        // Hot against cold: the plume is blue-grey (blue is its highest
        // channel) and the flare is warm white. Compared as a warmth difference
        // so retuning either colour keeps the test meaningful.
        let warmth = |c: [f32; 3]| c[0] - c[2];
        assert!(
            warmth(blast_rgb(0.0)) > warmth(SMOKE_RGB),
            "{:?} vs {SMOKE_RGB:?}",
            blast_rgb(0.0)
        );

        // Strength rises with the launch, like the plume.
        let onset = launch_blast(true, TRAIL_ONSET_SPEED).unwrap();
        assert!(onset.particles < blast.particles);
        assert!(onset.alpha < blast.alpha);

        // A near-KO launch flares in the same colour its plume burns.
        let deep = launch_blast(true, TRAIL_NEAR_KO_SPEED * 2.0).unwrap();
        assert_eq!(deep.ember, 1.0);
        assert_eq!(blast_rgb(deep.ember), plume_rgb(deep.ember));
    }

    /// The flare is per sim tick like the plume, so a fast display does not
    /// thicken it.
    #[test]
    fn the_flare_does_not_thicken_with_the_frame_rate() {
        let mut app = harness();
        set_launched_beat(
            &mut app,
            Some(Vec2::new(TRAIL_ONSET_SPEED * 2.0, 0.0)),
            0.09,
        );
        app.update();
        assert_eq!(sparks(&drain(&mut app)).len(), 1);
        for _ in 0..3 {
            app.update();
        }
        assert!(sparks(&drain(&mut app)).is_empty());
    }

    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<SimTick>();
        app.init_resource::<LaunchedBodiesView>();
        app.add_message::<VfxMessage>();
        app.add_systems(Update, emit_launch_trails);
        app
    }

    fn set_launched(app: &mut App, vel: Option<Vec2>) {
        set_launched_beat(app, vel, 0.0);
    }

    /// A launched body with `beat` seconds left in its hard control lock.
    fn set_launched_beat(app: &mut App, vel: Option<Vec2>, beat: f32) {
        let mut view = app.world_mut().resource_mut::<LaunchedBodiesView>();
        view.0.clear();
        if let Some(vel) = vel {
            view.0.push(LaunchedBodyFact {
                pos: Vec2::ZERO,
                vel,
                size: Vec2::new(30.0, 48.0),
                launch_beat_secs: beat,
                // The fighter kit's authored threshold. Not under test here.
                tumble_speed: 500.0,
            });
        }
    }

    /// Every spark burst in `msgs` — the flare. The plume is dust.
    fn sparks(msgs: &[VfxMessage]) -> Vec<&VfxMessage> {
        msgs.iter()
            .filter(|m| {
                matches!(
                    m,
                    VfxMessage::Burst {
                        kind: ParticleKind::Spark,
                        ..
                    }
                )
            })
            .collect()
    }

    fn dust(msgs: &[VfxMessage]) -> Vec<&VfxMessage> {
        msgs.iter()
            .filter(|m| {
                matches!(
                    m,
                    VfxMessage::Burst {
                        kind: ParticleKind::Dust,
                        ..
                    }
                )
            })
            .collect()
    }

    /// Advance `n` sim ticks, one frame each, and return everything requested.
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
