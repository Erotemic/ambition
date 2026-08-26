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
//! * The lock is [`BodyCombat::recoil_lock_timer`], the engine's existing
//!   "carried, can't steer" gate, re-held every tick of the transit — the same
//!   lever the snake's shell uses to be frozen. You cannot jump out of a pipe.
//!
//! The choreography is a pure function ([`step_pipe_transit`]) with a thin ECS
//! wrapper, mirroring `snake.rs` and `flag.rs`, so its timing is unit-tested even
//! though the thing it produces is a LOOK.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::PlayerEntity;
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::engine_core as ae;

/// Seconds spent sinking into the near pipe, and rising out of the far one. Half
/// a second each way: slow enough to read as a slide rather than a snap, short
/// enough that a player who takes the pipe often never waits on it.
pub const SWALLOW_S: f32 = 0.5;
pub const EMERGE_S: f32 = 0.5;

/// How far along the tube's axis the body travels in each phase. Two tiles is the
/// pipe's own height, so the slide starts at the lip and ends with the body fully
/// inside the pipe's footprint — which is exactly the span the pipe art covers, so
/// the body is completely swallowed at the moment of the crossing and nothing
/// pops.
pub const TRAVEL_TILES: f32 = 2.0;

/// The freeze-lock re-stamped each tick of a transit (the engine's
/// `recoil_lock_timer`, which hard-zeros movement input). Any value above one
/// frame works — it is refreshed every tick and cleared when the transit ends.
const TRANSIT_LOCK: f32 = 1.0;

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

/// Attach the per-body pipe-entry latch before the entry reader runs.
pub fn ensure_pipe_entry_latch(
    mut commands: Commands,
    bodies: Query<Entity, (With<PlayerEntity>, Without<PipeEntryLatch>)>,
) {
    for entity in &bodies {
        commands
            .entity(entity)
            .try_insert(PipeEntryLatch::default());
    }
}

/// Which half of the transit is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitPhase {
    /// Sliding INTO the near pipe, before the crossing.
    Swallowing,
    /// Rising OUT of the far pipe, after it.
    Emerging,
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
    /// The far pipe's throat — where the body reappears when the swallow ends,
    /// and where the emergence slide starts from.
    pub throat: ae::Vec2,
    /// Where the body finally stands when the whole transit is over.
    pub arrival: ae::Vec2,
}

impl PipeTransit {
    /// Begin a transit: the body sinks `TRAVEL_TILES` along `axis` from where it
    /// stands, then rises the same distance out of the far pipe to `arrival`.
    ///
    /// `axis` is the direction of travel INTO the near pipe — screen-down for the
    /// descent tube, screen-up for the ascent one — so both ends of the trip use
    /// the same construction and neither hard-codes a direction.
    pub fn begin(at: ae::Vec2, arrival: ae::Vec2, axis: ae::Vec2, tile: f32) -> Self {
        let travel = axis * (TRAVEL_TILES * tile);
        Self {
            phase: TransitPhase::Swallowing,
            elapsed: 0.0,
            from: at,
            to: at + travel,
            // The emergence continues the journey, it does not reverse it. The
            // throat is a pipe-length BEHIND the arrival along the travel axis, so
            // the body keeps moving the same way it entered: down a descent tube it
            // comes DOWN out of the ceiling pipe, up an ascent tube it comes UP out
            // of the ground. Adding `travel` instead put the throat on the far side
            // of the arrival, so every exit ran backwards — you dropped below the
            // mouth and rose into place going down, and overshot and sank going up.
            throat: arrival - travel,
            arrival,
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
    /// appeared in the far one. Only true for one tick of a transit.
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
        TransitPhase::Swallowing => SWALLOW_S,
        TransitPhase::Emerging => EMERGE_S,
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
            &mut BodyCombat,
            &mut PipeTransit,
        ),
        With<PlayerEntity>,
    >,
) {
    let dt = world_time.scaled_dt;
    for (entity, clusters, mut model, mut combat, mut transit) in &mut bodies {
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
                // Held every tick: you cannot steer, jump, or fall out of a pipe.
                combat.recoil_lock_timer = TRANSIT_LOCK;
                *transit = next;
            }
            None => {
                combat.recoil_lock_timer = 0.0;
                commands.entity(entity).remove::<PipeTransit>();
            }
        }
    }
}

#[cfg(test)]
mod tests;
