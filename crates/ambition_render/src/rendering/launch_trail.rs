//! Hard-launch smoke trail — the flight-readability layer.
//!
//! A body thrown hard enough leaves a trailing plume behind its velocity
//! vector, so a spectator can still read the launch after the impact spark has
//! left the screen. This is a LAYER over the hit spark and camera shake, not a
//! replacement for either.
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

/// Speed at which a launch starts smoking, in world units per second.
///
/// Above the Smash stage's `tumble_speed` (500) so the trail marks a hit that
/// actually sent someone, not every knockdown.
const TRAIL_ONSET_SPEED: f32 = 650.0;

/// Speed at which the trail reaches full density. Past this it stops getting
/// denser — a near-KO already reads, and more particles only cost fill rate.
const TRAIL_FULL_SPEED: f32 = 1700.0;

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

/// What one launched body's trail asks for this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailPuff {
    /// Emit only on ticks divisible by this. Smaller is denser.
    pub stride: u64,
    pub particles: u32,
    pub alpha: f32,
}

/// The trail this launch asks for — `None` when the launch is not hard enough
/// to be worth reading from across the stage.
///
/// The whole gate lives here so it can be asserted without a renderer: a
/// voluntary sprint and a launch at the same speed must not answer the same,
/// and only the caller's membership in the launched view separates them.
pub fn launch_trail_puff(launched: bool, speed: f32) -> Option<TrailPuff> {
    if !launched || speed < TRAIL_ONSET_SPEED {
        return None;
    }
    let t = ((speed - TRAIL_ONSET_SPEED) / (TRAIL_FULL_SPEED - TRAIL_ONSET_SPEED)).clamp(0.0, 1.0);
    Some(TrailPuff {
        // Rounds toward the denser end as `t` rises, and never to zero.
        stride: lerp(ONSET_STRIDE as f32, FULL_STRIDE as f32, t).round().max(1.0) as u64,
        particles: lerp(ONSET_PARTICLES as f32, FULL_PARTICLES as f32, t).round() as u32,
        alpha: lerp(MIN_SMOKE_ALPHA, MAX_SMOKE_ALPHA, t),
    })
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
        let Some(puff) = launch_trail_puff(true, speed) else {
            continue;
        };
        if tick.0 % puff.stride != 0 {
            continue;
        }
        let behind = body.vel / speed.max(1.0) * (body.size.x.max(body.size.y) * TRAIL_OFFSET_FRACTION);
        vfx.write(VfxMessage::Burst {
            pos: body.pos - behind,
            count: puff.particles,
            speed: PUFF_SPREAD_SPEED,
            color: [SMOKE_RGB[0], SMOKE_RGB[1], SMOKE_RGB[2], puff.alpha],
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
        assert_eq!(full, launch_trail_puff(true, TRAIL_FULL_SPEED * 10.0).unwrap());
        assert!(full.stride >= 1, "a stride of zero would divide by zero");
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

    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<SimTick>();
        app.init_resource::<LaunchedBodiesView>();
        app.add_message::<VfxMessage>();
        app.add_systems(Update, emit_launch_trails);
        app
    }

    fn set_launched(app: &mut App, vel: Option<Vec2>) {
        let mut view = app.world_mut().resource_mut::<LaunchedBodiesView>();
        view.0.clear();
        if let Some(vel) = vel {
            view.0.push(LaunchedBodyFact {
                pos: Vec2::ZERO,
                vel,
                size: Vec2::new(30.0, 48.0),
            });
        }
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
