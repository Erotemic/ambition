//! The pipe transit — the scripted slide into one tube and out of the other.
//!
//! ## How it is composed, with no engine edits
//!
//! * The motion is [`ambition_platformer2d::engine_core::movement::transit_body`] re-issued
//!   every tick along an eased path, rather than once at the end. That is the
//!   engine's authority for discretely relocating a body (ADR 0024) and it
//!   reconciles the motion model's private attachment/maneuver state each time, so
//!   a player who enters the pipe mid-wall-cling is not still clinging inside it.
//!   The ECS wrapper is scheduled after ordinary movement, so the transit's
//!   authored position wins every frame instead of allowing an unordered
//!   integrator to displace it after the snap.
//! * The occlusion is presentation data, not code: the pipe props are authored
//!   `PropDraw::Structure`, so their art fills the collider a body stands on
//!   exactly AND draws in front of the cast — which is what lets a pipe swallow a
//!   body sliding into it instead of pasting that body on top of it.
//! * The body says that its pose is not its own: it has
//!   [`ae::PoseOwnedExternally`] for each tick of the transit, the fact a
//!   rider on a mount has. The movement step still runs for it (its clocks
//!   go on), and that step gives a held body no pull and reports no landing
//!   for it. Without the fact the step took her as a body that fell: with her
//!   feet 6 to 14 px in the top of the pipe it put her back on the pipe and
//!   reported a landing, one for each tick (MEASURED in the flagship app: 4
//!   landing cues and 4 puffs of dust in one ride down, Jon 2026-10-11).
//! * The lock is the transit's own control hold (`ControlHold::Transit`):
//!   the control gate gives a held body a neutral frame, so you can not steer
//!   or jump out of a pipe. It borrowed `BodyCombat::recoil_lock_timer` and
//!   set it to zero at the end of the ride, which cut short a real recoil.
//!   The pose is the transit's own `PoseHold::Transit`, so a ride that ends
//!   does not take a hold that another holder has.
//!
//! The choreography is a pure function ([`step_pipe_transit`]) with a thin ECS
//! wrapper, mirroring `snake.rs` and `flag.rs`, so its timing is unit-tested even
//! though the thing it produces is a LOOK.

use bevy::prelude::*;

use ambition_platformer2d::characters::control::{claim_control_hold, release_control_hold, ControlHold, ControlHolds};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PlayerEntity;

/// Seconds spent sinking into the near pipe, and rising out of the far one. Half
/// a second each way: slow enough to read as a slide rather than a snap, short
/// enough that a player who takes the pipe often never waits on it.
pub const SWALLOW_S: f32 = 0.5;
pub const EMERGE_S: f32 = 0.5;

/// Seconds spent stepping across the mouth to the pipe's centre line, before
/// the slide. Short: it is a step, and a body that is on the line skips it.
pub const CENTER_S: f32 = 0.14;

/// A body this near the centre line (px) is on it.
const ON_THE_LINE: f32 = 0.5;

/// How far along the tube's axis the body travels in each phase. Two tiles is the
/// pipe's own height, so the slide starts at the lip and ends with the body fully
/// inside the pipe's footprint — which is exactly the span the pipe art covers, so
/// the body is completely swallowed at the moment of the crossing and nothing
/// pops.
pub const TRAVEL_TILES: f32 = 2.0;

/// The authored cue a warp voices, procedurally synthesized from the provider's
/// audio fragment (see `provider.rs`). Named as an id rather than a shared
/// `SoundCueKey` because "went down a pipe" is Mary-O's verb, not the engine's.
pub const PIPE_WARP_SFX: &str = "mary_o.pipe";

/// Rollback-safe rising-edge state for directional pipe entry.
///
/// A system-local `Local<bool>` survives GGRS world restores and can therefore
/// suppress an input edge during resimulation. Keeping the latch on the body
/// makes the edge part of the same authoritative snapshot as the transit it
/// creates.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PipeEntryLatch {
    pub pressed: bool,
}

/// Which half of the transit is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitPhase {
    /// Stepping across the mouth to the pipe's centre line, before the slide.
    Centering,
    /// Sliding INTO the near pipe, before the crossing.
    Swallowing,
    /// Rising OUT of the far pipe, after it.
    Emerging,
    /// In a pipe that has no far pipe: she stays in it until the level goes
    /// (see [`PipeTransit::leaves`]).
    Leaving,
}

/// A pipe transit in flight, carried by the body making it.
///
/// Rollback-registered like the rest of the demo's per-body state, so a rewind
/// puts a half-swallowed player back mid-slide instead of at one end of it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PipeTransit {
    pub phase: TransitPhase,
    /// Seconds elapsed in the CURRENT phase.
    pub elapsed: f32,
    /// Where this phase's slide begins...
    pub from: ae::Vec2,
    /// ...and where it ends.
    pub to: ae::Vec2,
    /// Where the swallow ends: inside the near pipe, on its centre line.
    pub swallowed: ae::Vec2,
    /// The far pipe's throat — where the body reappears when the swallow ends,
    /// and where the emergence slide starts from.
    pub throat: ae::Vec2,
    /// Where the body finally stands when the whole transit is over.
    pub arrival: ae::Vec2,
    /// The pipe has no far pipe: the far end is another level. The swallow
    /// ends in [`TransitPhase::Leaving`], and the room she arrives in ends the
    /// transit.
    pub leaves: bool,
}

impl PipeTransit {
    /// Begin a transit: the body steps across the mouth to the pipe's centre
    /// line, sinks `TRAVEL_TILES` along `axis`, then rises the same distance out
    /// of the far pipe to `arrival`.
    ///
    /// `axis` is the direction of travel INTO the near pipe — screen-down for the
    /// descent tube, screen-up for the ascent one — so both ends of the trip use
    /// the same construction and neither hard-codes a direction. `mouth` is a
    /// point of the near pipe's centre line. A mouth is wider than a body, so
    /// she can press into it off the line; a slide from there would go down
    /// the pipe's wall.
    pub fn begin(at: ae::Vec2, mouth: ae::Vec2, arrival: ae::Vec2, axis: ae::Vec2, tile: f32) -> Self {
        let travel = axis * (TRAVEL_TILES * tile);
        // Across the mouth only: the part of the way to `mouth` that is not
        // along the pipe.
        let to_mouth = mouth - at;
        let centred = at + (to_mouth - axis * to_mouth.dot(axis));
        let on_the_line = centred.distance(at) < ON_THE_LINE;
        Self {
            phase: if on_the_line { TransitPhase::Swallowing } else { TransitPhase::Centering },
            elapsed: 0.0,
            from: at,
            to: if on_the_line { centred + travel } else { centred },
            swallowed: centred + travel,
            // The emergence continues the journey, it does not reverse it. The
            // throat is a pipe-length BEHIND the arrival along the travel axis, so
            // the body keeps moving the same way it entered: down a descent tube it
            // comes DOWN out of the ceiling pipe, up an ascent tube it comes UP out
            // of the ground. Adding `travel` instead put the throat on the far side
            // of the arrival, so every exit ran backwards — you dropped below the
            // mouth and rose into place going down, and overshot and sank going up.
            throat: arrival - travel,
            arrival,
            leaves: false,
        }
    }

    /// Begin the transit into a pipe that goes to another level: the same step
    /// to the centre line and the same slide down, and no far pipe. She stays
    /// where the swallow ends.
    pub fn begin_leaving(at: ae::Vec2, mouth: ae::Vec2, axis: ae::Vec2, tile: f32) -> Self {
        let swallowed = Self::begin(at, mouth, at, axis, tile).swallowed;
        Self {
            throat: swallowed,
            arrival: swallowed,
            leaves: true,
            ..Self::begin(at, mouth, at, axis, tile)
        }
    }
}

/// What the pure step wants the ECS to do for one transiting body this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitEffects {
    /// Where to put the body this tick.
    pub pos: ae::Vec2,
    /// The transit's next state, or `None` when it is over (unlock the body).
    pub next: Option<PipeTransit>,
    /// This tick is the CROSSING — the body left the near pipe's throat and
    /// appeared in the far one, or the swallow of a pipe that leaves the level
    /// ended. Only true for one tick of a transit.
    pub crossed: bool,
}

/// The whole transit, as a pure function. Advance one phase's clock, ease the
/// body along the current slide, and hand over to the next phase (or finish).
///
/// The ease is smoothstep, so the body leaves the lip gently, moves fastest in
/// the middle of the tube, and settles rather than stopping dead. A linear slide
/// reads as a machine; this reads as being pulled through.
pub fn step_pipe_transit(transit: PipeTransit, dt: f32) -> TransitEffects {
    let elapsed = transit.elapsed + dt;
    let duration = match transit.phase {
        TransitPhase::Centering => CENTER_S,
        TransitPhase::Swallowing => SWALLOW_S,
        TransitPhase::Emerging => EMERGE_S,
        // No end of its own: the room she arrives in ends it.
        TransitPhase::Leaving => {
            return TransitEffects {
                pos: transit.to,
                next: Some(PipeTransit { elapsed, ..transit }),
                crossed: false,
            };
        }
    };
    let t = (elapsed / duration).clamp(0.0, 1.0);
    let pos = transit.from.lerp(transit.to, smoothstep(t));
    if t < 1.0 {
        return TransitEffects {
            pos,
            next: Some(PipeTransit { elapsed, ..transit }),
            crossed: false,
        };
    }
    match transit.phase {
        // On the centre line: sink from here.
        TransitPhase::Centering => TransitEffects {
            pos: transit.to,
            next: Some(PipeTransit {
                phase: TransitPhase::Swallowing,
                elapsed: 0.0,
                from: transit.to,
                to: transit.swallowed,
                ..transit
            }),
            crossed: false,
        },
        // Fully swallowed, and no far pipe: she stays here while the level goes.
        TransitPhase::Swallowing if transit.leaves => TransitEffects {
            pos: transit.swallowed,
            next: Some(PipeTransit {
                phase: TransitPhase::Leaving,
                elapsed: 0.0,
                from: transit.swallowed,
                to: transit.swallowed,
                ..transit
            }),
            crossed: true,
        },
        // Fully swallowed: cross to the far pipe's throat and start rising.
        TransitPhase::Swallowing => TransitEffects {
            pos: transit.throat,
            next: Some(PipeTransit {
                phase: TransitPhase::Emerging,
                elapsed: 0.0,
                from: transit.throat,
                to: transit.arrival,
                ..transit
            }),
            crossed: true,
        },
        // Out the far end, standing where the warp promised.
        TransitPhase::Emerging => TransitEffects {
            pos: transit.arrival,
            next: None,
            crossed: false,
        },
        TransitPhase::Leaving => unreachable!("a Leaving phase has no end of its own"),
    }
}

/// `3t² − 2t³` on `[0,1]`: zero slope at both ends, so the slide eases in and out.
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// The transit, as an ECS system. A thin wrapper over [`step_pipe_transit`]:
/// hold the body's input lock, relocate it along the eased path through the
/// engine's transit authority, and drop the component when it surfaces.
pub fn run_pipe_transits(
    mut commands: Commands,
    world_time: Res<ambition_platformer2d::time::WorldTime>,
    mut bodies: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
            &mut PipeTransit,
            Option<&mut ControlHolds>,
            Option<&ae::PoseOwnedExternally>,
        ),
        With<PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    for (entity, clusters, mut model, mut transit, mut control_holds, pose_holds) in &mut bodies {
        let fx = step_pipe_transit(*transit, dt);
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        // Re-issued every tick, not once at the end: this is what keeps the body
        // ON the scripted path while the shared movement phase runs around it.
        ambition_platformer2d::engine_core::movement::transit_body(
            &mut model,
            &mut clusters,
            fx.pos,
            ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
        );
        match fx.next {
            Some(next) => {
                *transit = next;
                // Claimed again on each tick that does not hold them, so a
                // transit that a restore brings back has its holds on its
                // next tick: you can not steer, jump or fall out of a pipe.
                if !control_holds.as_deref().is_some_and(|holds| holds.holds(ControlHold::Transit)) {
                    claim_control_hold(&mut commands, entity, ControlHold::Transit);
                }
                if !pose_holds.is_some_and(|holds| holds.holds(ae::PoseHold::Transit)) {
                    ae::claim_pose_hold(&mut commands, entity, ae::PoseHold::Transit);
                }
            }
            None => {
                commands.entity(entity).remove::<PipeTransit>();
                // The transit's holds only: a recoil, a script or a mount
                // keeps its own.
                release_control_hold(&mut commands, entity, control_holds.as_deref_mut(), ControlHold::Transit);
                ae::release_pose_hold(&mut commands, entity, ae::PoseHold::Transit);
            }
        }
    }
}

#[cfg(test)]
mod tests;
