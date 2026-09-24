//! The tether reel: she throws a line at a ledge and it pulls her to it.
//!
//! This module catches no ledge. The movement kernel's ledge authority
//! (`ledge_grab::try_start_ledge_grab_clusters_in_frame`) runs every frame, and
//! `LedgeContact::anchor` is where the body hangs. The reel delivers her to the
//! anchor and lets go; the authority catches her with its own cooldown and
//! eligibility rules.
//!
//! A complex move may coordinate authorities but must not become one. Putting
//! her into `LedgeHang` here would duplicate ledge state (trumping, getup, the
//! cooldown).
//!
//! On arrival the reel commands zero velocity, not a coast. The authority
//! catches an airborne body by a stick wall normal or by a Smash-style
//! auto-snap that needs falling at `FALL_SNAP_MIN_VY` (45px/s). Still moving
//! upward, she would hang in the air beside the ledge. From zero, gravity
//! crosses 45px/s in about two frames, about a pixel of drop.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_tether::{TetherPullParams, TETHER_PULL};
use ambition_platformer2d::engine_core as ae;

/// How far apart the virtual probe positions sit along the line, in world px.
///
/// A constant, not a parameter: a sampling step that must not skip a ledge.
/// A ledge's grab band is 28px up and 30px down (`LEDGE_REACH_UP` /
/// `LEDGE_REACH_DOWN`), so 16px cannot step over one.
const LINE_SAMPLE_PX: f32 = 16.0;

/// How close counts as arrived, in world px.
const ARRIVED_PX: f32 = 0.5;

/// A fighter currently being reeled to a ledge she latched.
///
/// Rollback state: the clock and anchor decide where a fighter is over several
/// frames.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct TetherReel {
    /// Seconds before the reel gives up.
    pub remaining_s: f32,
    /// How fast it pulls, in world px per second.
    pub speed: f32,
    /// Where the line bit.
    ///
    /// Latched, not re-asked (unlike `HomingDash`, which re-targets each tick).
    /// A ledge does not move, and a line that re-aimed would not be a line.
    pub anchor: ae::Vec2,
}

/// Checksum probe: the clock and the anchor. `speed` is a constant copied from
/// the move and cannot diverge.
pub fn tether_reel_probe(reel: &TetherReel) -> u64 {
    (reel.remaining_s.to_bits() as u64).rotate_left(23)
        ^ (reel.anchor.x.to_bits() as u64)
        ^ (reel.anchor.y.to_bits() as u64).rotate_left(11)
}

/// Throw the line where a move asked for one, and latch what it bit.
pub fn begin_authored_tether_pulls(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    // The composed collision read-API, as the pogo strike uses.
    collision: ambition_platformer2d::world::collision::CollisionWorld,
    bodies: Query<(
        &ae::BodyKinematics,
        &ae::BodyGroundState,
        &ambition_platformer2d::world::ResolvedMotionFrame,
    )>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != TETHER_PULL {
            continue;
        }
        let params: TetherPullParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("tether-pull params did not hydrate: {err}");
                continue;
            }
        };
        let Ok((kin, ground, frame)) = bodies.get(message.actor) else {
            continue;
        };
        // A tether recovery is an aerial move. On the ground the same fiction
        // is her grab; a grounded line would be a free horizontal dash.
        if ground.on_ground {
            continue;
        }
        let Some(solids) = collision.solids() else {
            continue;
        };
        // The line goes where she faces, and the wall she wants faces back at
        // her (as `requested_wall_normal_clusters` answers `-stick.x.signum()`).
        //
        // "Where she faces" is her frame's side axis: the probe's
        // `wall_normal_x` is in the body's local side axis, so the walk must
        // use the same frame. `to_world` is the transform authored volumes and
        // `spawn_body_strike` use.
        let reach_dir = frame
            .basis()
            .to_world(ae::Vec2::new(kin.facing.signum(), 0.0));
        // Frame-relative already, so it needs no rotation: a sign in the local
        // side axis, not a world direction.
        let wall_normal_x = -kin.facing.signum();
        let steps = (params.reach / LINE_SAMPLE_PX).ceil().max(1.0) as i32;
        let mut bite = None;
        for step in 0..=steps {
            // Deterministic: a fixed sample count from the authored reach.
            let along = (step as f32 / steps as f32) * params.reach;
            let probe_pos = kin.pos + reach_dir * along;
            if let Some(contact) = ae::ledge_grab::probe_ledge_grab_in_frame(
                probe_pos,
                kin.size,
                wall_normal_x,
                &solids,
                frame.down(),
            ) {
                bite = Some(contact);
                break;
            }
        }
        let Some(contact) = bite else {
            // A whiffed tether is not an error: she keeps falling, which is
            // the punish.
            info!(
                target: "ambition::moves",
                "tether: no ledge within {}px of {:?}", params.reach, kin.pos,
            );
            continue;
        };
        info!(
            target: "ambition::moves",
            "tether: bit a ledge at {:?}, reeling from {:?}", contact.anchor, kin.pos,
        );
        commands.entity(message.actor).try_insert((
            TetherReel {
                remaining_s: params.timeout_s,
                speed: params.speed,
                anchor: contact.anchor,
            },
            // The line the player sees, published as the engine's generic
            // `BodyLineAnchor` so presentation need not know `TetherReel`. The
            // line road that draws grabs draws this too.
            ambition_platformer2d::engine_core::BodyLineAnchor(contact.anchor),
        ));
    }
}

/// Reel each tethered fighter toward her anchor, and let go when she arrives.
pub fn reel_tethered_fighters(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    collision: ambition_platformer2d::world::collision::CollisionWorld,
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        &mut TetherReel,
        &ambition_platformer2d::world::ResolvedMotionFrame,
    )>,
) {
    let dt = time.sim_dt();
    let solids = collision.solids();
    for (entity, mut kin, mut reel, frame) in &mut bodies {
        // Test the budget before spending it, so an N-tick reel gets N pulls.
        // `author_tether_pull` asserts `speed * timeout_s >= reach`, a promise
        // measured in exactly those ticks.
        let has_budget = reel.remaining_s > 0.0;
        // Spent after the test, so this tick's pull is paid for by the budget
        // that authorised it.
        reel.remaining_s -= dt;
        let to_anchor = reel.anchor - kin.pos;
        let distance = to_anchor.length();
        // Two different exits. Giving up leaves her momentum alone; stopping
        // her dead would delete her remaining recovery.
        if !has_budget {
            info!(target: "ambition::moves", "tether: the reel gave up short of {:?}", reel.anchor);
            // The line goes with the reel; a line to a ledge she is no longer
            // attached to reads as a live threat.
            commands
                .entity(entity)
                .try_remove::<TetherReel>()
                .try_remove::<ambition_platformer2d::engine_core::BodyLineAnchor>();
            continue;
        }
        // Ask the authority whether it would catch her here, not whether she
        // reached the anchor. The anchor is a hang position that overlaps the
        // wall slightly, so the swept resolve stops her about 1px short and
        // `distance <= ARRIVED_PX` may never hold. Chasing the anchor only
        // delays the catch until the timeout (tick 22 against tick 6 on the
        // live stage). With the authority's own probe there is no tolerance to
        // tune.
        let caught_here = solids.as_ref().is_some_and(|world| {
            ae::ledge_grab::probe_ledge_grab_in_frame(
                kin.pos,
                kin.size,
                -kin.facing.signum(),
                world,
                frame.down(),
            )
            .is_some()
        });
        if caught_here || distance <= ARRIVED_PX {
            // Release to gravity: the ledge authority catches a falling body
            // (see the module header).
            crate::motion::command_body_velocity(&mut kin, ae::Vec2::ZERO, "tether arrived");
            // The line goes with the reel.
            commands
                .entity(entity)
                .try_remove::<TetherReel>()
                .try_remove::<ambition_platformer2d::engine_core::BodyLineAnchor>();
            continue;
        }
        // Shorten the last step so she lands on the anchor: a flat `speed`
        // overshoots by up to one tick (15px at 900px/s).
        let step_speed = reel.speed.min(distance / dt);
        crate::motion::command_body_velocity(
            &mut kin,
            to_anchor.normalize_or_zero() * step_speed,
            "tether reel",
        );
    }
}

#[cfg(test)]
mod tests;
