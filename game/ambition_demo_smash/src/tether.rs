//! The tether reel: she throws a line at a ledge and it pulls her to it.
//!
//! ⭐⭐ THIS MODULE CATCHES NO LEDGE. Ledge grabbing is an engine authority the
//! movement kernel already runs every frame
//! (`ledge_grab::try_start_ledge_grab_clusters_in_frame`, called from
//! `movement/mod.rs`), and a `LedgeContact::anchor` is documented as *"world
//! position the player should snap to (their center while hanging)"*. ⇒ The reel
//! DELIVERS HER TO THE ANCHOR and then lets go. The authority catches her from
//! her real position, on its own terms, with its own release cooldown and its
//! own eligibility rules — none of which this module knows or may know.
//!
//! ⛔ THAT IS THE RULE THIS ROW EXISTS TO DEMONSTRATE: a complex move may
//! coordinate many authorities but must not become the authority for their
//! state. Putting her into `LedgeHang` here would be a second implementation of
//! ledge state, and every rule written since — trumping, getup, the cooldown —
//! would have a copy nobody maintains.
//!
//! ⚠ AND THE RELEASE IS WHY THE ARRIVAL COMMANDS ZERO RATHER THAN COASTING. The
//! authority catches an airborne body two ways: a requested wall normal from the
//! STICK, or a Smash-style auto-snap that requires falling at `FALL_SNAP_MIN_VY`
//! (45px/s). A reel that ended while still carrying her UPWARD satisfies
//! neither, so a tether that arrived would hang in the air beside the ledge it
//! just caught. Zeroing hands her to gravity, which crosses 45px/s in about two
//! frames — roughly a pixel of drop, well inside the probe's own band.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_tether::{TetherPullParams, TETHER_PULL};
use ambition_platformer2d::engine_core as ae;

/// How far apart the virtual probe positions sit along the line, in world px.
///
/// ⭐ A CONSTANT AND NOT A PARAMETER, because it is a sampling artefact rather
/// than a design knob: it must be small enough that no ledge hides between two
/// samples. A ledge's own grab band is 28px up and 30px down
/// (`LEDGE_REACH_UP` / `LEDGE_REACH_DOWN`), so 16px cannot step over one.
const LINE_SAMPLE_PX: f32 = 16.0;

/// How close counts as arrived, in world px.
const ARRIVED_PX: f32 = 0.5;

/// A fighter currently being reeled to a ledge she latched.
///
/// ⛔ ROLLBACK STATE. The clock and the anchor decide where a fighter IS over
/// several frames, so a rewind that restored the reel without them puts the two
/// peers' fighters in different places.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct TetherReel {
    /// Seconds before the reel gives up.
    pub remaining_s: f32,
    /// How fast it pulls, in world px per second.
    pub speed: f32,
    /// Where the line bit.
    ///
    /// ⭐ LATCHED, NOT RE-ASKED — the deliberate contrast with `HomingDash`,
    /// which re-asks its target every tick so a foe can leave the cone. A ledge
    /// does not move, and a line that re-aimed itself mid-reel would not be a
    /// line.
    pub anchor: ae::Vec2,
}

/// Checksum probe: the clock and the anchor — the facts a peer can disagree
/// about. ⛔ `speed` is a constant copied off the move and cannot diverge.
pub fn tether_reel_probe(reel: &TetherReel) -> u64 {
    (reel.remaining_s.to_bits() as u64).rotate_left(23)
        ^ (reel.anchor.x.to_bits() as u64)
        ^ (reel.anchor.y.to_bits() as u64).rotate_left(11)
}

/// Throw the line where a move asked for one, and latch what it bit.
pub fn begin_authored_tether_pulls(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    // The composed collision read-API, the same one the pogo strike asks.
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
        // ⚠ A TETHER RECOVERY IS AN AERIAL MOVE. On the ground the same fiction
        // is her GRAB, which is a different verb with its own reach and its own
        // recovery, and letting the line fire while standing would give her a
        // free horizontal dash along the stage.
        if ground.on_ground {
            continue;
        }
        let Some(solids) = collision.solids() else {
            continue;
        };
        // ⭐ THE LINE GOES WHERE SHE FACES, and the wall she wants is the one
        // whose face points BACK at her — the same reading the kernel uses for
        // an airborne grab request (`requested_wall_normal_clusters` answers
        // `-stick.x.signum()`).
        let reach_dir = ae::Vec2::new(kin.facing.signum(), 0.0);
        let wall_normal_x = -kin.facing.signum();
        let steps = (params.reach / LINE_SAMPLE_PX).ceil().max(1.0) as i32;
        let mut bite = None;
        for step in 0..=steps {
            // Deterministic sampling: a fixed count derived from the authored
            // reach, never from elapsed time or a float accumulator.
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
            // ⚠ A WHIFFED TETHER IS NOT AN ERROR. She threw a line at nothing
            // and keeps falling, which is the punish the move is priced for.
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
        commands.entity(message.actor).try_insert(TetherReel {
            remaining_s: params.timeout_s,
            speed: params.speed,
            anchor: contact.anchor,
        });
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
        reel.remaining_s -= dt;
        let to_anchor = reel.anchor - kin.pos;
        let distance = to_anchor.length();
        // ⛔ THE TWO EXITS ARE NOT THE SAME EXIT, and collapsing them was the
        // first draft's bug. GIVING UP must leave her momentum alone: a reel
        // that expires mid-flight and also stops her dead would delete the
        // recovery she had left and read as the game freezing her in the air.
        if reel.remaining_s <= 0.0 {
            info!(target: "ambition::moves", "tether: the reel gave up short of {:?}", reel.anchor);
            commands.entity(entity).try_remove::<TetherReel>();
            continue;
        }
        // ⭐⭐ ASK THE AUTHORITY WHETHER IT WOULD CATCH HER HERE, rather than
        // wait for her to stand on a point. That is the reel's actual job, and
        // chasing the anchor cannot finish it:
        //
        // ⛔ THE ANCHOR IS A HANG POSITION, AND A HANGING BODY OVERLAPS THE WALL.
        // Measured in a live match: her body is 34.4px wide, the anchor sat at
        // x=63.8 and the platform's face at x=80, so the anchor puts her right
        // edge 1px INSIDE the solid. The swept resolve correctly refuses to move
        // her there, so she pins ~1px short and `distance <= ARRIVED_PX` never
        // becomes true.
        //
        // ⚠ AND THE FIRST VERSION OF THIS COMMENT CALLED THAT A LIVELOCK, WHICH
        // IS FALSE — the poison that was supposed to prove it PASSED. Chasing
        // the anchor does not stop the catch, it DELAYS it: the reel runs out
        // its whole timeout pinned against the wall, releases on the clock, and
        // the authority then catches her anyway. Measured on the live stage:
        // tick 22 chasing the anchor against tick 6 asking the authority. ⇒ The
        // cost is a quarter-second of a fighter stuck to a wall doing nothing,
        // which reads as the move failing and then working for no reason.
        //
        // ⇒ The moment the authority's own probe accepts where she IS, the reel
        // is done. No tolerance to tune, and it cannot disagree with the thing
        // it is handing her to.
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
            // ⭐ RELEASING HANDS HER TO GRAVITY AND GETS OUT OF THE WAY. See the
            // module header: the ledge authority catches a FALLING body, so
            // releasing her with upward velocity would leave her hanging in the
            // air beside the ledge she just reached.
            crate::motion::command_body_velocity(&mut kin, ae::Vec2::ZERO, "tether arrived");
            commands.entity(entity).try_remove::<TetherReel>();
            continue;
        }
        // ⛔ THE LAST STEP IS SHORTENED SO SHE LANDS ON THE ANCHOR RATHER THAN
        // PAST IT. Reeling at a flat `speed` overshoots by up to one tick of
        // travel — 15px at 900px/s — and 15px past a ledge lip is a fighter
        // beside the ledge rather than on it.
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
