//! The traversal envelope: where one body can land from a running jump or a
//! walk-off, measured by the real movement kernel.
//!
//! [`TraversalEnvelope::measure`] clones the body and drives
//! [`step_motion`] in an empty probe world, as
//! `ambition_platformer2d_core::movement::recovery::probe_recovery` does. There is
//! no second physics: the arc is what the kernel did, sampled each step.
//!
//! The run-up is on a long floor until the body has its top speed. Then the
//! floor is cut at the body's leading edge, and two rollouts start from that
//! state: one presses jump on the first step and holds it, the other presses
//! nothing (the walk-off drop). Each sample is the position of the body's feet
//! centre relative to that takeoff edge, in the frame's local axes:
//! `lead` is how far the leading edge is past the takeoff edge, and `rise` is
//! the height of the feet above the takeoff surface (up is positive).
//!
//! The envelope answers for full throttle only. A shorter landing by releasing
//! the stick is a seam ([`TraversalEnvelope::reaches`] does not claim it), and
//! so are air jumps, dashes, wall verbs and flight: the jump rollout presses
//! jump one time.
//!
//! HOW HONEST IT IS, measured on two tunings at 60 Hz (the guard in `tests.rs`):
//! the widest gap the kernel really crosses is between 1.5 px less and 3.75 px
//! more than the widest gap [`TraversalEnvelope::reaches`] allows, over level,
//! higher and lower landings and walk-offs to 1200 px down. Most of the spread
//! is where the takeoff falls: the envelope takes off exactly at the edge, and a
//! body that jumps on its last grounded step takes off up to one step of travel
//! (`takeoff_speed * dt`) short of it. A caller that must not miss subtracts that.

use ambition_platformer2d_core as ae;
use ae::movement::{step_motion, ActionEdges, Edge, InputState, MotionStepContext, MovementAction};
use ae::{BodyClusterScratch, LocalAxes, MotionFrame, Vec2, World};

/// One kernel step of a rollout, relative to the takeoff edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcSample {
    /// Kernel steps since the takeoff.
    pub step: u16,
    /// How far the body's leading edge is past the takeoff edge, along the
    /// run direction.
    pub lead: f32,
    /// The height of the body's feet above the takeoff surface; up is positive.
    pub rise: f32,
}

/// What [`TraversalEnvelope::measure`] runs: the timestep and the limits of
/// each rollout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeProbe {
    pub dt: f32,
    /// The most kernel steps the run-up may take to reach its top speed.
    pub max_run_up_steps: usize,
    /// The most kernel steps one rollout records.
    pub max_arc_steps: usize,
    /// A rollout stops when the feet are this far below the takeoff surface.
    pub max_drop: f32,
}

impl Default for EnvelopeProbe {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            max_run_up_steps: 600,
            max_arc_steps: 600,
            max_drop: 1600.0,
        }
    }
}

/// The landing offsets one body can reach at full throttle, from a running
/// jump and from a walk-off.
#[derive(Clone, Debug, PartialEq)]
pub struct TraversalEnvelope {
    pub probe: EnvelopeProbe,
    /// The body's extent along the run direction and against gravity.
    pub body_width: f32,
    pub body_height: f32,
    /// The speed along the run direction at the takeoff.
    pub takeoff_speed: f32,
    /// The jump rollout: jump pressed on the takeoff step and held.
    pub jump: Vec<ArcSample>,
    /// The walk-off rollout: no jump.
    pub drop: Vec<ArcSample>,
}

impl TraversalEnvelope {
    /// Measure `body` under `frame`. The body runs toward the frame's `+side`.
    ///
    /// Returns `None` when the frame is not axis-aligned (the probe world is
    /// built from axis-aligned blocks), or when the run-up never stands on its
    /// floor.
    pub fn measure(body: &BodyClusterScratch, frame: MotionFrame, probe: EnvelopeProbe) -> Option<Self> {
        let side = frame.side();
        let down = frame.down();
        if !is_cardinal(side) || !is_cardinal(down) {
            return None;
        }
        let size = body.kinematics.size;
        let half_along = (size * side).abs().length() * 0.5;
        let half_tall = (size * down).abs().length() * 0.5;
        // Room for any speed the kernel can give a running body in the time
        // the probe allows. The world is a square around `origin`, so every
        // block and the whole arc have positive coordinates.
        let run_up = probe.dt * probe.max_run_up_steps as f32 * 4000.0;
        let reach = probe.dt * probe.max_arc_steps as f32 * 4000.0 + probe.max_drop;
        let room = 2.0 * (run_up + reach);
        let origin = Vec2::splat(room);
        let local = |along: f32, below: f32| origin + side * along + down * below;

        // The run-up floor: its top face is `below == 0`, and it is long enough
        // that the body cannot reach its end.
        let floor = |from: f32, to: f32| {
            let a = local(from, 0.0);
            let b = local(to, 64.0);
            ae::Block::solid("envelope floor", a.min(b), (a - b).abs())
        };
        let world_of = |blocks: Vec<ae::Block>| {
            World::new("traversal envelope", Vec2::splat(room * 2.0), origin, blocks)
                .with_fall_out_margin(room)
        };
        let run_world = world_of(vec![floor(-run_up, run_up)]);

        let mut runner = body.clone();
        {
            let (model, mut clusters) = runner.parts();
            ae::movement::transit_body(
                model,
                &mut clusters,
                local(-run_up + half_along + 16.0, -half_tall),
                ae::movement::TransitVelocity::Zero,
            );
        }
        let mut speed = 0.0_f32;
        let mut settled = 0;
        for _ in 0..probe.max_run_up_steps {
            step(&mut runner, &run_world, frame, probe.dt, run_input(false, false));
            let now = runner.kinematics.vel.dot(side);
            settled = if runner.ground.on_ground && (now - speed).abs() < 1e-3 { settled + 1 } else { 0 };
            speed = now;
            if settled >= 3 {
                break;
            }
        }
        if !runner.ground.on_ground {
            return None;
        }

        // Cut the floor at the leading edge and roll out each verb from there.
        let edge = (runner.kinematics.pos - origin).dot(side) + half_along;
        let takeoff_world = world_of(vec![floor(edge - run_up, edge)]);
        let rollout = |jump: bool| {
            let mut body = runner.clone();
            let mut samples = Vec::new();
            for index in 0..probe.max_arc_steps {
                step(&mut body, &takeoff_world, frame, probe.dt, run_input(jump && index == 0, jump));
                let lead = (body.kinematics.pos - origin).dot(side) + half_along - edge;
                let rise = -((body.kinematics.pos - origin).dot(down) + half_tall);
                samples.push(ArcSample { step: (index + 1) as u16, lead, rise });
                if rise < -probe.max_drop {
                    break;
                }
            }
            samples
        };
        Some(Self {
            probe,
            body_width: half_along * 2.0,
            body_height: half_tall * 2.0,
            takeoff_speed: speed,
            jump: rollout(true),
            drop: rollout(false),
        })
    }

    /// The highest the feet rise above the takeoff surface in the jump.
    pub fn apex_rise(&self) -> f32 {
        self.jump.iter().map(|sample| sample.rise).fold(0.0, f32::max)
    }

    /// Can a running jump land on a surface whose near edge is `gap` past the
    /// takeoff edge and whose top is `rise` above the takeoff surface?
    ///
    /// Yes when the leading edge passes over the near edge with the feet above
    /// the top, and the arc then comes down to the top. The surface is assumed
    /// to extend at least to [`Self::landing_lead`].
    pub fn reaches(&self, gap: f32, rise: f32) -> bool {
        arc_reaches(&self.jump, gap, rise)
    }

    /// [`Self::reaches`] for a walk-off with no jump.
    pub fn drop_reaches(&self, gap: f32, rise: f32) -> bool {
        arc_reaches(&self.drop, gap, rise)
    }

    /// Where the leading edge is when the falling jump arc comes down to
    /// `rise`: a landing surface must reach this far for the body to stay on it.
    pub fn landing_lead(&self, rise: f32) -> Option<f32> {
        descending_crossing(&self.jump, rise)
    }

    /// [`Self::landing_lead`] for the walk-off.
    pub fn drop_landing_lead(&self, rise: f32) -> Option<f32> {
        descending_crossing(&self.drop, rise)
    }
}

/// Full throttle toward `+side`, with the jump edge as given.
fn run_input(jump_pressed: bool, jump_held: bool) -> InputState {
    InputState {
        axes: LocalAxes::new(1.0, 0.0),
        movement: ActionEdges::<MovementAction>::EMPTY.with(
            MovementAction::Jump,
            Edge {
                pressed: jump_pressed,
                held: jump_held,
                released: false,
            },
        ),
        ..Default::default()
    }
}

fn step(body: &mut BodyClusterScratch, world: &World, frame: MotionFrame, dt: f32, input: InputState) {
    let facing_intent = input.local_axis().x;
    let (model, mut clusters) = body.parts();
    step_motion(
        model,
        &mut clusters,
        MotionStepContext {
            world,
            input,
            frame,
            facing_intent,
            dt,
            // A probe of this body alone, not its real motion among others.
            contact: ae::movement::BodyContactField::NONE,
            pose_owned_externally: false,
            recovery_commitment_outstanding: false,
        },
    );
}

fn is_cardinal(axis: Vec2) -> bool {
    (axis.x.abs() < 1e-4 && (axis.y.abs() - 1.0).abs() < 1e-4)
        || (axis.y.abs() < 1e-4 && (axis.x.abs() - 1.0).abs() < 1e-4)
}

/// The feet height where the leading edge first reaches `lead`, by linear
/// interpolation between the two samples that bracket it.
fn rise_at_lead(arc: &[ArcSample], lead: f32) -> Option<f32> {
    let mut previous = ArcSample { step: 0, lead: 0.0, rise: 0.0 };
    for sample in arc {
        if sample.lead >= lead {
            let span = sample.lead - previous.lead;
            let t = if span > 0.0 { ((lead - previous.lead) / span).clamp(0.0, 1.0) } else { 1.0 };
            return Some(previous.rise + (sample.rise - previous.rise) * t);
        }
        previous = *sample;
    }
    None
}

fn arc_reaches(arc: &[ArcSample], gap: f32, rise: f32) -> bool {
    let gap = gap.max(0.0);
    let Some(over_the_edge) = rise_at_lead(arc, gap) else {
        return false;
    };
    over_the_edge >= rise && descending_crossing(arc, rise).is_some()
}

/// The leading edge where the arc, falling, comes down through `rise`.
fn descending_crossing(arc: &[ArcSample], rise: f32) -> Option<f32> {
    let mut previous = ArcSample { step: 0, lead: 0.0, rise: 0.0 };
    for sample in arc {
        if previous.rise >= rise && sample.rise < rise && sample.rise < previous.rise {
            let t = (previous.rise - rise) / (previous.rise - sample.rise);
            return Some(previous.lead + (sample.lead - previous.lead) * t);
        }
        previous = *sample;
    }
    None
}

#[cfg(test)]
mod tests;
