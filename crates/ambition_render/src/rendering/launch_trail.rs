//! Hard-launch readability — the flight layer, in TWO beats.
//!
//! 1. **The blast.** A body thrown hard tears out of the hit in a bright spark
//!    flare that lasts exactly as long as the launch's hard control lock. This
//!    is the moment of being launched.
//! 2. **The plume.** For the rest of the flight it trails smoke behind its
//!    velocity vector, so a spectator can still read the launch after the
//!    impact spark has left the screen.
//!
//! Both are a LAYER over the hit spark and camera shake, not a replacement for
//! either.
//!
//! ⭐ THE TWO BEATS ARE THE POINT. A body launched this instant and one that
//! has been tumbling for a second are the same row in the view, at the same
//! speed, and looked identical until the blast existed: the plume alone says
//! *"this body is flying"* and never *"this body was just hit that hard"*.
//! The one fact that separates them is
//! [`ambition_sim_view::LaunchedBodyFact::launch_beat_secs`], and it is the
//! sim's own control-lock window rather than anything reconstructed here.
//!
//! The gate is membership in [`LaunchedBodiesView`] — the sim's resolved
//! "this motion is involuntary" fact — plus speed. Velocity alone is not the
//! question: a fighter can run, fast-fall and recover at any speed under its
//! own power, and none of that is a launch.
//!
//! Everything here is cosmetic and non-rollback. Emission is keyed to the
//! SIM TICK rather than to the frame or to a per-body accumulator, which buys
//! three things at once: a plume of the same density at any refresh rate, the
//! same plume in a capture as on screen, and no presentation state for a
//! rollback resimulation to multiply.

use bevy::prelude::*;

use ambition_sim_view::LaunchedBodiesView;
use ambition_time::SimTick;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

// THE THREE SPEEDS, measured rather than picked.
//
// `match_report -- 30 --runs 5` on the merged tree reports per-run PEAK launch
// speed at 332 / 442 / 770 (min / median / max across runs). Every number below
// is a stated percentile of that distribution, and the first version of this
// file had none of them: `TRAIL_ONSET_SPEED` was 650, chosen off the stage's
// `tumble_speed` as a proxy for "a hit that sent someone", and the trail
// therefore never fired in a match at all.
//
// ⛔ do not re-pick these off a single run. One thirty-second sample is how the
// 650 happened. Measure across runs, and ideally across matchups — these five
// are all George versus George, which is the honest limit of this pick.

/// Speed at which a launch starts smoking, in world units per second.
///
/// The MINIMUM observed per-run peak (0th percentile): the weakest match's best
/// launch still trails, so the effect is part of the match's language rather
/// than a once-a-game event. Set at the median peak instead and a typical match
/// would show the trail exactly once.
const TRAIL_ONSET_SPEED: f32 = 330.0;

/// Speed at which the trail reaches full density. Past this it stops getting
/// denser — more particles only cost fill rate.
///
/// Roughly the 75th percentile of observed per-run peaks, between the median
/// (442) and the max (770).
const TRAIL_FULL_SPEED: f32 = 550.0;

/// Sim ticks between puffs at onset density, and at full density. A stride of
/// one is a puff every tick.
const ONSET_STRIDE: u64 = 3;
const FULL_STRIDE: u64 = 1;

/// Particles per puff at onset and at full density. Slow-moving: the plume is
/// meant to hang where the body WAS, not to spray.
const ONSET_PARTICLES: u32 = 2;
const FULL_PARTICLES: u32 = 3;
const PUFF_SPREAD_SPEED: f32 = 34.0;

/// Fraction of the body's larger extent that the puff sits behind its centre.
const TRAIL_OFFSET_FRACTION: f32 = 0.4;

/// Puff colour: faint at onset, solid at full density.
const SMOKE_RGB: [f32; 3] = [0.82, 0.84, 0.90];
const MIN_SMOKE_ALPHA: f32 = 0.35;
const MAX_SMOKE_ALPHA: f32 = 0.78;

/// Speed at which a launch stops being hard and starts being a KILL.
///
/// The MAXIMUM observed per-run peak (100th percentile), so the ember is
/// reserved for a launch at the very top of what the fight produces and
/// saturates only above anything measured. It was 2600 — four times anything
/// observed, and unreachable.
const TRAIL_NEAR_KO_SPEED: f32 = 770.0;

/// The near-KO plume's colour: an ember, not smoke. A hue change rather than
/// more of the same particles, because more grey at a speed where the plume is
/// already saturated reads as nothing.
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
    /// plume's shift from smoke toward ember and the extra particles that come
    /// with it; `0.0` for every launch below [`TRAIL_NEAR_KO_SPEED`].
    pub ember: f32,
}

/// How hard this launch is, or `None` when it is not a launch worth reading.
///
/// `.0` is the ordinary hard-launch ramp, `0` at [`TRAIL_ONSET_SPEED`] and `1`
/// at [`TRAIL_FULL_SPEED`]; `.1` is how far into the near-KO band it goes.
/// ONE onset for both beats on purpose — the blast and the plume must agree
/// about what counts as a launch, or a hit reads as a flare with no smoke.
fn launch_bands(launched: bool, speed: f32) -> Option<(f32, f32)> {
    if !launched || speed < TRAIL_ONSET_SPEED {
        return None;
    }
    let t = ((speed - TRAIL_ONSET_SPEED) / (TRAIL_FULL_SPEED - TRAIL_ONSET_SPEED)).clamp(0.0, 1.0);
    // The near-KO band runs from where density saturates to twice as far
    // again, so the ember arrives gradually rather than switching on.
    let ember =
        ((speed - TRAIL_NEAR_KO_SPEED) / (TRAIL_NEAR_KO_SPEED - TRAIL_FULL_SPEED)).clamp(0.0, 1.0);
    Some((t, ember))
}

/// The trail this launch asks for — `None` when the launch is not hard enough
/// to be worth reading from across the stage.
///
/// The whole gate lives here so it can be asserted without a renderer: a
/// voluntary sprint and a launch at the same speed must not answer the same,
/// and only the caller's membership in the launched view separates them.
pub fn launch_trail_puff(launched: bool, speed: f32) -> Option<TrailPuff> {
    let (t, ember) = launch_bands(launched, speed)?;
    Some(TrailPuff {
        // Rounds toward the denser end as `t` rises, and never to zero.
        stride: lerp(ONSET_STRIDE as f32, FULL_STRIDE as f32, t)
            .round()
            .max(1.0) as u64,
        particles: lerp(ONSET_PARTICLES as f32, FULL_PARTICLES as f32, t).round() as u32
            + (EMBER_EXTRA_PARTICLES as f32 * ember).round() as u32,
        alpha: lerp(MIN_SMOKE_ALPHA, MAX_SMOKE_ALPHA, t),
        ember,
    })
}

/// Sparks in the flare at onset and at full launch strength, PER TICK of the
/// beat. The beat is short — the engine's ordinary knockback control lock is
/// `0.12s`, seven ticks — so these are per-tick counts of a flare, not of a
/// one-shot burst.
const BLAST_PARTICLES_ONSET: u32 = 3;
const BLAST_PARTICLES_FULL: u32 = 6;

/// How fast the flare throws its sparks. An order of magnitude above
/// [`PUFF_SPREAD_SPEED`], and that gap IS the read: the plume hangs where the
/// body was, the flare leaves.
const BLAST_SPREAD_SPEED: f32 = 300.0;

/// The flare's colour: hot white, shifting to the plume's ember at the near-KO
/// end so the two beats of one launch are the same launch.
const BLAST_RGB: [f32; 3] = [1.0, 0.97, 0.86];
const BLAST_MIN_ALPHA: f32 = 0.70;
const BLAST_MAX_ALPHA: f32 = 0.95;

/// What the FRONT of a launch asks for, on top of the plume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchBlast {
    pub particles: u32,
    pub speed: f32,
    pub alpha: f32,
    /// Shared with [`TrailPuff::ember`] so the flare and the smoke of one
    /// launch shift colour together.
    pub ember: f32,
}

/// The flare this launch asks for THIS TICK — `None` once the launch stops
/// being new, which is the whole distinction this function exists to draw.
///
/// `beat` is [`ambition_sim_view::LaunchedBodyFact::launch_beat_secs`] being
/// live: the sim's hard control lock at the front of a knockback, the window in
/// which the body has been thrown and cannot yet answer for it. ⛔ it is NOT
/// reconstructed from speed, hitstun or the plume — a body decelerating into
/// the same speed twice would flare twice.
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
/// Runs on the render clock but samples on the SIM clock: a frame that
/// advanced no tick emits nothing, so the trail's density is a property of the
/// flight rather than of the machine drawing it.
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
        // THE BLAST, first: it is on top of the plume, not instead of it, so a
        // launch starts smoking from the tick it starts flaring. Sparks at the
        // body rather than behind it — the flare is the body tearing out of the
        // hit, and it shrinks and falls away where the plume grows and hangs.
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
            // Dust already grows and drags as it ages, which is what a plume
            // does. A `Smoke` kind would be the same recipe under a new name.
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
        // Launched but barely moving — a body caught at the top of its arc, or
        // one whose hitstun outlives the speed that earned it.
        assert!(launch_trail_puff(true, TRAIL_ONSET_SPEED - 1.0).is_none());
        // Voluntary motion at a speed no launch could beat. This is exactly the
        // case a velocity-only gate gets wrong.
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

        // The SMOKE saturates at full density: past here a faster launch buys
        // no more grey. What it does buy is the ember — a different band with
        // its own test, and the reason this is about stride and alpha rather
        // than the whole puff.
        let beyond = launch_trail_puff(true, TRAIL_FULL_SPEED * 10.0).unwrap();
        assert_eq!(beyond.stride, full.stride);
        assert_eq!(beyond.alpha, full.alpha);
    }

    /// A NEAR-KO launch stops being smoke and starts being an ember, and it
    /// does it gradually — an ordinary hard launch is untouched by this tier.
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

        // And it is still a launch trail: the gate above it is unchanged.
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

    /// A body launched THIS INSTANT and one that has been tumbling for a second
    /// must not look the same. This is the slice.
    ///
    /// Same view, same speed, same everything except the control lock the sim
    /// published — and only the first one flares.
    #[test]
    fn the_front_of_a_launch_flares_and_a_sustained_tumble_does_not() {
        let hard = Vec2::new(1500.0, 0.0);

        let mut app = harness();
        set_launched_beat(&mut app, Some(hard), 0.09);
        let front = run_ticks(&mut app, 4);
        assert!(!sparks(&front).is_empty(), "the front of a launch flares");
        assert!(!dust(&front).is_empty(), "and it smokes from the same tick");

        // The lock has run out. Still launched, still travelling exactly as
        // fast, still trailing — and no longer flaring.
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

    /// The flare answers to the LAUNCH, not to the lock alone: a body nudged
    /// into a control lock at walking pace has nothing to announce.
    #[test]
    fn a_beat_below_the_launch_onset_does_not_flare() {
        assert!(launch_blast(true, TRAIL_ONSET_SPEED - 1.0).is_none());
        assert!(launch_blast(true, TRAIL_ONSET_SPEED).is_some());
        // And no lock is no flare at any speed a fight can produce.
        assert!(launch_blast(false, TRAIL_ONSET_SPEED * 10.0).is_none());
    }

    /// The flare is the plume's opposite number, and the contrast is what makes
    /// the two beats legible rather than one effect at two densities.
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
        // Hot against cold: the plume is a blue-grey smoke (its blue channel is
        // its highest) and the flare is warm white. Stated as the warmth
        // DIFFERENCE rather than a channel value, so retuning either colour
        // keeps the claim honest.
        let warmth = |c: [f32; 3]| c[0] - c[2];
        assert!(
            warmth(blast_rgb(0.0)) > warmth(SMOKE_RGB),
            "{:?} vs {SMOKE_RGB:?}",
            blast_rgb(0.0)
        );

        // Strength rises with the launch, exactly as the plume's does.
        let onset = launch_blast(true, TRAIL_ONSET_SPEED).unwrap();
        assert!(onset.particles < blast.particles);
        assert!(onset.alpha < blast.alpha);

        // And a near-KO launch flares in the same colour its plume burns.
        let deep = launch_blast(true, TRAIL_NEAR_KO_SPEED * 2.0).unwrap();
        assert_eq!(deep.ember, 1.0);
        assert_eq!(blast_rgb(deep.ember), plume_rgb(deep.ember));
    }

    /// The flare is per SIM TICK like the plume, so a fast display does not
    /// thicken it either.
    #[test]
    fn the_flare_does_not_thicken_with_the_frame_rate() {
        let mut app = harness();
        set_launched_beat(&mut app, Some(Vec2::new(1500.0, 0.0)), 0.09);
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
